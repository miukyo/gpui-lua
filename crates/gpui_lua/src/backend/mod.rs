use crate::dsl::node::SyncRegistryKey;
use crate::net::json_to_lua_value;
use crate::reactive::bridge::ReactiveBridge;
use futures::future::BoxFuture;
use mlua::{FromLuaMulti, Function, IntoLua, Lua, MultiValue, Result, Table, Value};
use parking_lot::RwLock;
use std::collections::HashMap;
use std::future::Future;
use std::sync::Arc;

pub type SyncHandler = Box<dyn Fn(&Lua, MultiValue) -> Result<Value> + Send + Sync>;
pub type AsyncHandler = Box<dyn Fn(MultiValue) -> BoxFuture<'static, std::result::Result<serde_json::Value, String>> + Send + Sync>;
pub type RustEventHandler = Box<dyn Fn(&serde_json::Value) + Send + Sync>;

#[derive(Clone, Default)]
pub struct BackendBridge {
    sync_handlers: Arc<RwLock<HashMap<String, Arc<SyncHandler>>>>,
    async_handlers: Arc<RwLock<HashMap<String, Arc<AsyncHandler>>>>,
    event_listeners: Arc<parking_lot::Mutex<HashMap<String, Vec<SyncRegistryKey>>>>,
    rust_listeners: Arc<RwLock<HashMap<String, Vec<Arc<RustEventHandler>>>>>,
    bridge: Option<ReactiveBridge>,
    lua_ref: Arc<RwLock<Option<Arc<parking_lot::Mutex<Lua>>>>>,
}

impl BackendBridge {
    pub fn new(bridge: ReactiveBridge) -> Self {
        Self {
            sync_handlers: Arc::new(RwLock::new(HashMap::new())),
            async_handlers: Arc::new(RwLock::new(HashMap::new())),
            event_listeners: Arc::new(parking_lot::Mutex::new(HashMap::new())),
            rust_listeners: Arc::new(RwLock::new(HashMap::new())),
            bridge: Some(bridge),
            lua_ref: Arc::new(RwLock::new(None)),
        }
    }

    pub fn set_lua(&self, lua: Arc<parking_lot::Mutex<Lua>>) {
        *self.lua_ref.write() = Some(lua);
    }

    /// Register a synchronous Rust function callable from Lua via `backend.<name>(...)`.
    pub fn register_fn<F, A, R>(&self, name: impl Into<String>, func: F)
    where
        F: Fn(A) -> Result<R> + Send + Sync + 'static,
        A: FromLuaMulti + 'static,
        R: IntoLua + 'static,
    {
        let handler = Box::new(move |lua: &Lua, args: MultiValue| {
            let parsed_args = A::from_lua_multi(args, lua)?;
            let result = func(parsed_args)?;
            result.into_lua(lua)
        });
        self.sync_handlers
            .write()
            .insert(name.into(), Arc::new(handler));
    }

    /// Register a synchronous Rust function accepting any deserializable argument and returning any serializable type.
    pub fn register_json_fn<F, A, R>(&self, name: impl Into<String>, func: F)
    where
        F: Fn(A) -> std::result::Result<R, String> + Send + Sync + 'static,
        A: serde::de::DeserializeOwned + 'static,
        R: serde::Serialize + 'static,
    {
        let handler = Box::new(move |lua: &Lua, args: MultiValue| {
            let json_arg = lua_multivalue_to_json(&args);
            let parsed = serde_json::from_value::<A>(json_arg)
                .map_err(|e| mlua::Error::RuntimeError(format!("Argument parse error: {e}")))?;
            let result = func(parsed)
                .map_err(mlua::Error::RuntimeError)?;
            let json_res = serde_json::to_value(result)
                .map_err(|e| mlua::Error::RuntimeError(format!("Result serialize error: {e}")))?;
            json_to_lua_value(lua, &json_res)
        });
        self.sync_handlers
            .write()
            .insert(name.into(), Arc::new(handler));
    }

    /// Register an asynchronous Rust function callable from Lua via `backend.<name>(..., callback)`.
    pub fn register_async_fn<F, Fut, A, R>(&self, name: impl Into<String>, func: F)
    where
        F: Fn(A) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = std::result::Result<R, String>> + Send + 'static,
        A: serde::de::DeserializeOwned + Send + 'static,
        R: serde::Serialize + Send + 'static,
    {
        let handler = Box::new(move |args: MultiValue| {
            // Convert Lua args to JSON value
            let json_arg = lua_multivalue_to_json(&args);
            let parsed = match serde_json::from_value::<A>(json_arg) {
                Ok(a) => a,
                Err(e) => {
                    return Box::pin(async move {
                        Err(format!("Failed to deserialize arguments: {e}"))
                    }) as BoxFuture<'static, std::result::Result<serde_json::Value, String>>;
                }
            };
            let fut = func(parsed);
            Box::pin(async move {
                let res = fut.await?;
                serde_json::to_value(res).map_err(|e| format!("Serialization error: {e}"))
            })
        });

        self.async_handlers
            .write()
            .insert(name.into(), Arc::new(handler));
    }

    /// Emit an event from Rust backend to Lua frontend listeners (`backend.on(event, handler)`).
    pub fn emit(&self, event: &str, data: impl serde::Serialize) -> std::result::Result<(), String> {
        let json_val = serde_json::to_value(data).map_err(|e| e.to_string())?;

        // 1. Notify Rust listeners
        if let Some(handlers) = self.rust_listeners.read().get(event) {
            for handler in handlers {
                handler(&json_val);
            }
        }

        // 2. Notify Lua listeners
        if let Some(lua_arc) = self.lua_ref.read().clone() {
            let lua = lua_arc.lock();
            if let Some(keys) = self.event_listeners.lock().get(event).cloned() {
                for key in keys {
                    if let Ok(func) = key.with(|k| lua.registry_value::<Function>(k)) {
                        if let Ok(lua_val) = json_to_lua_value(&lua, &json_val) {
                            let _ = func.call::<()>(lua_val);
                        }
                    }
                }
            }
        }

        if let Some(b) = &self.bridge {
            b.notify();
        }

        Ok(())
    }

    /// Listen to an event emitted from Lua (`backend.emit(event, data)`).
    pub fn on<F>(&self, event: impl Into<String>, handler: F)
    where
        F: Fn(&serde_json::Value) + Send + Sync + 'static,
    {
        self.rust_listeners
            .write()
            .entry(event.into())
            .or_default()
            .push(Arc::new(Box::new(handler)));
    }
}

pub fn register(lua: &Lua, backend: BackendBridge) -> Result<()> {
    let backend_tbl = lua.create_table();

    // backend.call(name, ...args)
    let sync_map = backend.sync_handlers.clone();
    backend_tbl.set(
        "call",
        lua.create_function(move |lua, (name, args): (String, MultiValue)| {
            if let Some(handler) = sync_map.read().get(&name).cloned() {
                handler(lua, args)
            } else {
                Err(mlua::Error::RuntimeError(format!(
                    "Rust backend function '{name}' not found"
                )))
            }
        })?,
    )?;

    // backend.call_async(name, ...args, [callback])
    let async_map = backend.async_handlers.clone();
    let bridge_notify = backend.bridge.clone();
    let lua_holder = backend.lua_ref.clone();
    backend_tbl.set(
        "call_async",
        lua.create_function(move |lua, (name, args): (String, MultiValue)| {
            let mut arg_list: Vec<Value> = args.into_iter().collect();
            let callback = if let Some(last) = arg_list.last() {
                if let Value::Function(_) = last {
                    match arg_list.pop() {
                        Some(Value::Function(f)) => Some(lua.create_registry_value(f)?),
                        _ => None,
                    }
                } else {
                    None
                }
            } else {
                None
            };

            let remaining_args = MultiValue::from_vec(arg_list);
            let handler = match async_map.read().get(&name).cloned() {
                Some(h) => h,
                None => {
                    return Err(mlua::Error::RuntimeError(format!(
                        "Rust async backend function '{name}' not found"
                    )));
                }
            };

            let bridge = bridge_notify.clone();
            let lua_arc_ref = lua_holder.clone();

            crate::tokio_runtime().spawn(async move {
                let result = handler(remaining_args).await;

                if let Some(key) = callback {
                    if let Some(lua_arc) = lua_arc_ref.read().clone() {
                        let lua = lua_arc.lock();
                        if let Ok(func) = lua.registry_value::<Function>(&key) {
                            match result {
                                Ok(json_res) => {
                                    if let Ok(lua_val) = json_to_lua_value(&lua, &json_res) {
                                        let _ = func.call::<()>((Value::Nil, lua_val));
                                    }
                                }
                                Err(err_msg) => {
                                    let _ = func.call::<()>((err_msg, Value::Nil));
                                }
                            }
                        }
                    }
                    if let Some(b) = bridge {
                        b.notify();
                    }
                }
            });

            Ok(())
        })?,
    )?;

    // backend.on(event, handler_fn)
    let listeners_map = backend.event_listeners.clone();
    backend_tbl.set(
        "on",
        lua.create_function(move |lua, (event, handler): (String, Function)| {
            let key = SyncRegistryKey::new(lua.create_registry_value(handler)?);
            listeners_map
                .lock()
                .entry(event)
                .or_default()
                .push(key);
            Ok(())
        })?,
    )?;

    // backend.emit(event, data)
    let rust_event_listeners = backend.rust_listeners.clone();
    backend_tbl.set(
        "emit",
        lua.create_function(move |_lua, (event, data): (String, Value)| {
            let json_val = lua_value_to_json(&data);
            if let Some(listeners) = rust_event_listeners.read().get(&event) {
                for listener in listeners {
                    listener(&json_val);
                }
            }
            Ok(())
        })?,
    )?;

    // Metatable on backend for direct syntax: `backend.my_fn(...)`
    lua.load(r#"
        local b_tbl = ...
        local unpack = table.unpack or unpack
        local mt = {
            __index = function(tbl, key)
                -- First check raw methods
                local raw = rawget(tbl, key)
                if raw ~= nil then return raw end

                -- Return a callable proxy that dispatches to backend.call or call_async
                return function(...)
                    local args = {...}
                    local last_arg = args[#args]
                    if type(last_arg) == "function" then
                        -- Treat as async callback dispatch if last arg is function
                        return tbl.call_async(key, unpack(args))
                    else
                        return tbl.call(key, unpack(args))
                    end
                end
            end
        }
        setmetatable(b_tbl, mt)
    "#).call::<()>(backend_tbl.clone())?;

    lua.globals().set("backend", backend_tbl.clone())?;

    let package: Table = match lua.globals().get("package") {
        Ok(p) => p,
        Err(_) => {
            let p = lua.create_table();
            let loaded = lua.create_table();
            p.set("loaded", loaded)?;
            lua.globals().set("package", p.clone())?;
            p
        }
    };
    let loaded: Table = match package.get("loaded") {
        Ok(l) => l,
        Err(_) => {
            let l = lua.create_table();
            package.set("loaded", l.clone())?;
            l
        }
    };
    loaded.set("backend", backend_tbl)?;

    Ok(())
}

fn lua_value_to_json(val: &Value) -> serde_json::Value {
    match val {
        Value::Nil => serde_json::Value::Null,
        Value::Boolean(b) => serde_json::Value::Bool(*b),
        Value::Integer(i) => serde_json::Value::Number((*i).into()),
        Value::Number(n) => {
            if let Some(num) = serde_json::Number::from_f64(*n) {
                serde_json::Value::Number(num)
            } else {
                serde_json::Value::Null
            }
        }
        Value::String(s) => {
            let str_val = s.to_str().map(|v| v.to_string()).unwrap_or_default();
            serde_json::Value::String(str_val)
        }
        Value::Table(t) => {
            let mut is_array = true;
            let mut count = 0;
            for pair in t.pairs::<Value, Value>() {
                count += 1;
                if let Ok((Value::Integer(i), _)) = pair {
                    if i != count {
                        is_array = false;
                        break;
                    }
                } else {
                    is_array = false;
                    break;
                }
            }

            if is_array && count > 0 {
                let mut arr = Vec::new();
                for i in 1..=count {
                    if let Ok(v) = t.get(i) {
                        arr.push(lua_value_to_json(&v));
                    }
                }
                serde_json::Value::Array(arr)
            } else {
                let mut map = serde_json::Map::new();
                for pair in t.pairs::<Value, Value>() {
                    if let Ok((k, v)) = pair {
                        let key_str = match k {
                            Value::String(s) => s.to_str().map(|v| v.to_string()).unwrap_or_default(),
                            Value::Integer(i) => i.to_string(),
                            Value::Number(n) => n.to_string(),
                            _ => continue,
                        };
                        map.insert(key_str, lua_value_to_json(&v));
                    }
                }
                serde_json::Value::Object(map)
            }
        }
        _ => serde_json::Value::Null,
    }
}

fn lua_multivalue_to_json(args: &MultiValue) -> serde_json::Value {
    if args.len() == 1 {
        lua_value_to_json(&args[0])
    } else {
        let mut list = Vec::new();
        for arg in args.iter() {
            list.push(lua_value_to_json(arg));
        }
        serde_json::Value::Array(list)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dsl;
    use crate::reactive;

    #[test]
    fn test_rust_backend_sync_functions() -> Result<()> {
        let lua = Lua::new();
        let bridge = ReactiveBridge::new();
        let backend = BackendBridge::new(bridge);

        // Register Rust sync functions
        backend.register_fn("add", |(a, b): (i64, i64)| Ok(a + b));
        backend.register_fn("greet", |name: String| Ok(format!("Hello, {name} from Rust!")));

        register(&lua, backend)?;

        lua.load(r#"
            -- Test direct method call syntax via metatable
            local sum = backend.add(10, 25)
            assert(sum == 35, "backend.add failed: " .. tostring(sum))

            local greeting = backend.greet("G-Engine")
            assert(greeting == "Hello, G-Engine from Rust!", "backend.greet failed")

            -- Test explicit backend.call
            local sum2 = backend.call("add", 5, 7)
            assert(sum2 == 12, "backend.call failed")
        "#).exec()?;

        Ok(())
    }

    #[tokio::test]
    async fn test_rust_backend_async_functions() -> Result<()> {
        let lua = Lua::new();
        let bridge = ReactiveBridge::new();
        let backend = BackendBridge::new(bridge.clone());
        let lua_arc = Arc::new(parking_lot::Mutex::new(lua));
        backend.set_lua(lua_arc.clone());

        // Register async Rust backend function
        backend.register_async_fn("fetch_record", |id: u64| async move {
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
            Ok(serde_json::json!({
                "id": id,
                "title": "Rust Backend Data",
                "active": true
            }))
        });

        {
            let lua = lua_arc.lock();
            dsl::register(&lua)?;
            let store = crate::reactive::ReactiveStore::new();
            reactive::register(&lua, store, bridge)?;
            register(&lua, backend)?;
        }

        {
            let lua = lua_arc.lock();
            lua.load(r#"
                local loaded_title, set_title = signal("initial", "record_title")

                -- Call async Rust function with callback
                backend.fetch_record(42, function(err, record)
                    assert(err == nil, "Error in async call: " .. tostring(err))
                    assert(record.id == 42, "Record id mismatch")
                    assert(record.active == true, "Record active mismatch")
                    set_title(record.title)
                end)
            "#).exec()?;
        }

        // Allow background async task to execute
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        {
            let lua = lua_arc.lock();
            let title: String = lua.load("return __gpui_store_get('record_title')").eval()?;
            assert_eq!(title, "Rust Backend Data");
        }

        Ok(())
    }

    #[test]
    fn test_rust_to_lua_event_emission() -> Result<()> {
        let lua = Lua::new();
        let bridge = ReactiveBridge::new();
        let backend = BackendBridge::new(bridge);
        let lua_arc = Arc::new(parking_lot::Mutex::new(lua));
        backend.set_lua(lua_arc.clone());

        {
            let lua = lua_arc.lock();
            register(&lua, backend.clone())?;

            lua.load(r#"
                event_received_data = nil
                backend.on("user_joined", function(data)
                    event_received_data = data
                end)
            "#).exec()?;
        }

        // Emit event from Rust backend
        backend.emit("user_joined", serde_json::json!({
            "username": "ferris",
            "tier": "gold"
        })).expect("Failed to emit event");

        {
            let lua = lua_arc.lock();
            let username: String = lua.load("return event_received_data.username").eval()?;
            let tier: String = lua.load("return event_received_data.tier").eval()?;
            assert_eq!(username, "ferris");
            assert_eq!(tier, "gold");
        }

        Ok(())
    }

    #[test]
    fn test_lua_to_rust_event_emission() -> Result<()> {
        let lua = Lua::new();
        let bridge = ReactiveBridge::new();
        let backend = BackendBridge::new(bridge);

        let received = Arc::new(parking_lot::Mutex::new(None));
        let rec_clone = received.clone();

        backend.on("frontend_action", move |data| {
            *rec_clone.lock() = Some(data.clone());
        });

        register(&lua, backend)?;

        lua.load(r#"
            backend.emit("frontend_action", { action = "navigate", route = "/settings" })
        "#).exec()?;

        let data = received.lock().clone().expect("Expected Rust to receive event");
        assert_eq!(data["action"], "navigate");
        assert_eq!(data["route"], "/settings");

        Ok(())
    }
}
