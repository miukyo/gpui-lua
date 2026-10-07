use crate::dsl::node::SyncRegistryKey;
use crate::reactive::bridge::ReactiveBridge;
use futures::{SinkExt, StreamExt};
use mlua::{Function, Lua, Result, Table, Value};
use parking_lot::Mutex;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::Message;

static NEXT_WS_ID: AtomicU64 = AtomicU64::new(1);
static NEXT_SSE_ID: AtomicU64 = AtomicU64::new(1);

pub struct LuaWebSocket {
    pub id: u64,
    pub tx: mpsc::UnboundedSender<String>,
    pub closed: Arc<AtomicBool>,
    pub on_message: Arc<Mutex<Option<SyncRegistryKey>>>,
    pub on_open: Arc<Mutex<Option<SyncRegistryKey>>>,
    pub on_close: Arc<Mutex<Option<SyncRegistryKey>>>,
    pub on_error: Arc<Mutex<Option<SyncRegistryKey>>>,
}

impl mlua::UserData for LuaWebSocket {
    fn add_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("send", |_lua, this, data: Value| {
            if this.closed.load(Ordering::SeqCst) {
                return Err(mlua::Error::RuntimeError("WebSocket is closed".to_string()));
            }
            let text = match data {
                Value::String(s) => s.to_str().unwrap_or_default().to_string(),
                Value::Table(t) => {
                    let json_val = crate::stdlib::json::lua_value_to_json(Value::Table(t))?;
                    serde_json::to_string(&json_val).unwrap_or_default()
                }
                _ => format!("{data:?}"),
            };
            let _ = this.tx.send(text);
            Ok(())
        });

        methods.add_method("close", |_lua, this, ()| {
            this.closed.store(true, Ordering::SeqCst);
            Ok(())
        });

        methods.add_method("on_message", |lua, this, handler: Function| {
            let key = SyncRegistryKey::new(lua.create_registry_value(handler)?);
            *this.on_message.lock() = Some(key);
            Ok(())
        });

        methods.add_method("on_open", |lua, this, handler: Function| {
            let key = SyncRegistryKey::new(lua.create_registry_value(handler)?);
            *this.on_open.lock() = Some(key);
            Ok(())
        });

        methods.add_method("on_close", |lua, this, handler: Function| {
            let key = SyncRegistryKey::new(lua.create_registry_value(handler)?);
            *this.on_close.lock() = Some(key);
            Ok(())
        });

        methods.add_method("on_error", |lua, this, handler: Function| {
            let key = SyncRegistryKey::new(lua.create_registry_value(handler)?);
            *this.on_error.lock() = Some(key);
            Ok(())
        });
    }
}

pub struct LuaEventSource {
    pub id: u64,
    pub closed: Arc<AtomicBool>,
    pub on_message: Arc<Mutex<Option<SyncRegistryKey>>>,
    pub on_event: Arc<Mutex<HashMap<String, SyncRegistryKey>>>,
}

impl mlua::UserData for LuaEventSource {
    fn add_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("close", |_lua, this, ()| {
            this.closed.store(true, Ordering::SeqCst);
            Ok(())
        });

        methods.add_method("on_message", |lua, this, handler: Function| {
            let key = SyncRegistryKey::new(lua.create_registry_value(handler)?);
            *this.on_message.lock() = Some(key);
            Ok(())
        });

        methods.add_method("on_event", |lua, this, (event_name, handler): (String, Function)| {
            let key = SyncRegistryKey::new(lua.create_registry_value(handler)?);
            this.on_event.lock().insert(event_name, key);
            Ok(())
        });
    }
}

pub fn register(
    lua: &Lua,
    lua_ref: Arc<parking_lot::RwLock<Option<Arc<Mutex<Lua>>>>>,
    bridge: ReactiveBridge,
) -> Result<()> {
    let ws_lua = lua_ref.clone();
    let ws_bridge = bridge.clone();

    // net.websocket(url, [options])
    let websocket_fn = lua.create_function(move |_lua, (url_str, _opts): (String, Option<Table>)| {
        let ws_id = NEXT_WS_ID.fetch_add(1, Ordering::SeqCst);
        let (tx, mut rx) = mpsc::unbounded_channel::<String>();
        let closed = Arc::new(AtomicBool::new(false));

        let on_message: Arc<Mutex<Option<SyncRegistryKey>>> = Arc::new(Mutex::new(None));
        let on_open: Arc<Mutex<Option<SyncRegistryKey>>> = Arc::new(Mutex::new(None));
        let on_close: Arc<Mutex<Option<SyncRegistryKey>>> = Arc::new(Mutex::new(None));
        let on_error: Arc<Mutex<Option<SyncRegistryKey>>> = Arc::new(Mutex::new(None));

        let t_closed = closed.clone();
        let t_on_msg = on_message.clone();
        let t_on_open = on_open.clone();
        let t_on_close = on_close.clone();
        let t_on_err = on_error.clone();
        let t_lua = ws_lua.clone();
        let t_bridge = ws_bridge.clone();

        crate::tokio_runtime().spawn(async move {
            match connect_async(&url_str).await {
                Ok((ws_stream, _)) => {
                    // Trigger on_open
                    if let Some(open_key) = t_on_open.lock().clone() {
                        if let Some(lua_arc) = t_lua.read().clone() {
                            let lua = lua_arc.lock();
                            if let Ok(func) = open_key.with(|k| lua.registry_value::<Function>(k)) {
                                let _ = func.call::<()>(());
                            }
                        }
                        t_bridge.notify();
                    }

                    let (mut write, mut read) = ws_stream.split();

                    let write_closed = t_closed.clone();
                    let write_task = tokio::spawn(async move {
                        while let Some(msg_text) = rx.recv().await {
                            if write_closed.load(Ordering::SeqCst) {
                                break;
                            }
                            if write.send(Message::Text(msg_text.into())).await.is_err() {
                                break;
                            }
                        }
                        let _ = write.close().await;
                    });

                    let read_closed = t_closed.clone();
                    let read_lua = t_lua.clone();
                    let read_bridge = t_bridge.clone();
                    let read_on_msg = t_on_msg.clone();
                    let read_on_err = t_on_err.clone();
                    let read_on_close = t_on_close.clone();

                    while let Some(msg_res) = read.next().await {
                        if read_closed.load(Ordering::SeqCst) {
                            break;
                        }
                        match msg_res {
                            Ok(Message::Text(txt)) => {
                                if let Some(msg_key) = read_on_msg.lock().clone() {
                                    if let Some(lua_arc) = read_lua.read().clone() {
                                        let lua = lua_arc.lock();
                                        if let Ok(func) = msg_key.with(|k| lua.registry_value::<Function>(k)) {
                                            let _ = func.call::<()>(Value::String(lua.create_string(&txt)));
                                        }
                                    }
                                    read_bridge.notify();
                                }
                            }
                            Ok(Message::Close(frame)) => {
                                let code = frame.as_ref().map(|f| f.code.into()).unwrap_or(1000u16);
                                let reason = frame.as_ref().map(|f| f.reason.to_string()).unwrap_or_default();
                                if let Some(close_key) = read_on_close.lock().clone() {
                                    if let Some(lua_arc) = read_lua.read().clone() {
                                        let lua = lua_arc.lock();
                                        if let Ok(func) = close_key.with(|k| lua.registry_value::<Function>(k)) {
                                            let _ = func.call::<()>((code, Value::String(lua.create_string(&reason))));
                                        }
                                    }
                                    read_bridge.notify();
                                }
                                break;
                            }
                            Err(e) => {
                                if let Some(err_key) = read_on_err.lock().clone() {
                                    if let Some(lua_arc) = read_lua.read().clone() {
                                        let lua = lua_arc.lock();
                                        if let Ok(func) = err_key.with(|k| lua.registry_value::<Function>(k)) {
                                            let err_str = e.to_string();
                                            let _ = func.call::<()>(Value::String(lua.create_string(&err_str)));
                                        }
                                    }
                                    read_bridge.notify();
                                }
                                break;
                            }
                            _ => {}
                        }
                    }

                    read_closed.store(true, Ordering::SeqCst);
                    let _ = write_task.await;
                }
                Err(e) => {
                    if let Some(err_key) = t_on_err.lock().clone() {
                        if let Some(lua_arc) = t_lua.read().clone() {
                            let lua = lua_arc.lock();
                            if let Ok(func) = err_key.with(|k| lua.registry_value::<Function>(k)) {
                                let err_str = e.to_string();
                                let _ = func.call::<()>(Value::String(lua.create_string(&err_str)));
                            }
                        }
                        t_bridge.notify();
                    }
                }
            }
        });

        Ok(LuaWebSocket {
            id: ws_id,
            tx,
            closed,
            on_message,
            on_open,
            on_close,
            on_error,
        })
    })?;

    // net.sse(url, [options])
    let sse_lua = lua_ref.clone();
    let sse_bridge = bridge.clone();

    let sse_fn = lua.create_function(move |_lua, (url_str, _opts): (String, Option<Table>)| {
        let sse_id = NEXT_SSE_ID.fetch_add(1, Ordering::SeqCst);
        let closed = Arc::new(AtomicBool::new(false));
        let on_message: Arc<Mutex<Option<SyncRegistryKey>>> = Arc::new(Mutex::new(None));
        let on_event: Arc<Mutex<HashMap<String, SyncRegistryKey>>> = Arc::new(Mutex::new(HashMap::new()));

        let t_closed = closed.clone();
        let t_on_msg = on_message.clone();
        let t_on_event = on_event.clone();
        let t_lua = sse_lua.clone();
        let t_bridge = sse_bridge.clone();

        crate::tokio_runtime().spawn(async move {
            let client = reqwest::Client::new();
            let resp = client
                .get(&url_str)
                .header("Accept", "text/event-stream")
                .send()
                .await;

            if let Ok(mut response) = resp {
                let mut buffer = String::new();
                let mut current_event = "message".to_string();
                let mut current_data = String::new();

                while let Ok(Some(chunk)) = response.chunk().await {
                    if t_closed.load(Ordering::SeqCst) {
                        break;
                    }
                    buffer.push_str(&String::from_utf8_lossy(&chunk));

                    while let Some(idx) = buffer.find('\n') {
                        let line = buffer[..idx].trim_end_matches('\r').to_string();
                        buffer.drain(..=idx);

                        if line.is_empty() {
                            // Dispatch event
                            if !current_data.is_empty() {
                                if let Some(lua_arc) = t_lua.read().clone() {
                                    let lua = lua_arc.lock();
                                    if current_event == "message" {
                                        if let Some(msg_key) = t_on_msg.lock().clone() {
                                            if let Ok(func) = msg_key.with(|k| lua.registry_value::<Function>(k)) {
                                                let _ = func.call::<()>(Value::String(lua.create_string(&current_data)));
                                            }
                                        }
                                    } else {
                                        let events = t_on_event.lock();
                                        if let Some(ev_key) = events.get(&current_event) {
                                            if let Ok(func) = ev_key.with(|k| lua.registry_value::<Function>(k)) {
                                                let _ = func.call::<()>(Value::String(lua.create_string(&current_data)));
                                            }
                                        }
                                    }
                                }
                                t_bridge.notify();
                                current_data.clear();
                                current_event = "message".to_string();
                            }
                        } else if line.starts_with("event:") {
                            current_event = line["event:".len()..].trim().to_string();
                        } else if line.starts_with("data:") {
                            let piece = line["data:".len()..].trim();
                            if !current_data.is_empty() {
                                current_data.push('\n');
                            }
                            current_data.push_str(piece);
                        }
                    }
                }
            }
        });

        Ok(LuaEventSource {
            id: sse_id,
            closed,
            on_message,
            on_event,
        })
    })?;

    if let Ok(net_tbl) = lua.globals().get::<Table>("net") {
        net_tbl.set("websocket", websocket_fn.clone())?;
        net_tbl.set("sse", sse_fn.clone())?;
    } else {
        let net_tbl = lua.create_table();
        net_tbl.set("websocket", websocket_fn.clone())?;
        net_tbl.set("sse", sse_fn.clone())?;
        lua.globals().set("net", net_tbl)?;
    }

    Ok(())
}
