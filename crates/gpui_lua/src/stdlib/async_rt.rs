use crate::dsl::node::SyncRegistryKey;
use crate::reactive::bridge::ReactiveBridge;
use crate::stdlib::os::{OsAction, OsBridge};
use mlua::{Function, Lua, Result, Table, Value};
use parking_lot::Mutex;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin, Command};

static NEXT_PROC_ID: AtomicU64 = AtomicU64::new(1);
static NEXT_CALLBACK_ID: AtomicU64 = AtomicU64::new(1);

const ASYNC_CORE_LUA: &str = include_str!("lua/async_core.lua");

pub struct ProcessHandle {
    pub stdin: Option<ChildStdin>,
    pub child: Option<Child>,
}

#[derive(Clone)]
pub struct AsyncEngine {
    processes: Arc<Mutex<HashMap<u64, ProcessHandle>>>,
    process_handlers: Arc<Mutex<HashMap<u64, (Option<SyncRegistryKey>, Option<SyncRegistryKey>, Option<SyncRegistryKey>)>>>,
    dialog_callbacks: Arc<Mutex<HashMap<u64, SyncRegistryKey>>>,
    hotkeys: Arc<Mutex<HashMap<String, (SyncRegistryKey, bool)>>>,
    tray_items: Arc<Mutex<HashMap<u64, String>>>,
    lua_ref: Arc<parking_lot::RwLock<Option<Arc<Mutex<Lua>>>>>,
    bridge: ReactiveBridge,
    os_bridge: OsBridge,
}

impl AsyncEngine {
    pub fn new(bridge: ReactiveBridge, os_bridge: OsBridge) -> Self {
        Self {
            processes: Arc::new(Mutex::new(HashMap::new())),
            process_handlers: Arc::new(Mutex::new(HashMap::new())),
            dialog_callbacks: Arc::new(Mutex::new(HashMap::new())),
            hotkeys: Arc::new(Mutex::new(HashMap::new())),
            tray_items: Arc::new(Mutex::new(HashMap::new())),
            lua_ref: Arc::new(parking_lot::RwLock::new(None)),
            bridge,
            os_bridge,
        }
    }

    pub fn set_lua(&self, lua: Arc<Mutex<Lua>>) {
        *self.lua_ref.write() = Some(lua);
    }

    pub fn lua_ref(&self) -> Arc<parking_lot::RwLock<Option<Arc<Mutex<Lua>>>>> {
        self.lua_ref.clone()
    }

    pub fn bridge(&self) -> ReactiveBridge {
        self.bridge.clone()
    }
    pub fn trigger_dialog_callback(&self, id: u64, err: Option<String>, result: Option<Vec<String>>) {
        let key_opt = self.dialog_callbacks.lock().remove(&id);
        if let Some(key) = key_opt {
            if let Some(lua_arc) = self.lua_ref.read().clone() {
                let lua = lua_arc.lock();
                if let Ok(func) = key.with(|k| lua.registry_value::<Function>(k)) {
                    match (err, result) {
                        (Some(e), _) => {
                            let _ = func.call::<()>((Value::String(lua.create_string(&e)), Value::Nil));
                        }
                        (None, Some(paths)) => {
                            let tbl = lua.create_table();
                            for (i, p) in paths.into_iter().enumerate() {
                                let _ = tbl.set(i + 1, p);
                            }
                            let _ = func.call::<()>((Value::Nil, Value::Table(tbl)));
                        }
                        (None, None) => {
                            let _ = func.call::<()>((Value::Nil, Value::Nil));
                        }
                    }
                }
            }
            self.bridge.notify();
        }
    }

    pub fn trigger_hotkey(&self, shortcut: &str) -> bool {
        let hotkeys = self.hotkeys.lock();
        if let Some((key, _)) = hotkeys.get(&shortcut.to_lowercase()) {
            if let Some(lua_arc) = self.lua_ref.read().clone() {
                let lua = lua_arc.lock();
                if let Ok(func) = key.with(|k| lua.registry_value::<Function>(k)) {
                    let _ = func.call::<()>(());
                    self.bridge.notify();
                    return true;
                }
            }
        }
        false
    }
}

pub fn register(lua: &Lua, engine: AsyncEngine) -> Result<()> {
    #[cfg(feature = "net")]
    let eng_http = engine.clone();
    #[cfg(feature = "net")]
    let eng_lua = engine.lua_ref.clone();

    // 1. __async_http_fetch
    #[cfg(feature = "net")]
    lua.globals().set(
        "__async_http_fetch",
        lua.create_function(move |_lua, (opts, callback): (Table, Function)| {
            let method_str: String = opts.get("method").unwrap_or_else(|_| "GET".to_string());
            let url_str: String = opts.get("url").unwrap_or_default();
            let body_str: Option<String> = opts.get("body").ok();
            let timeout_secs: u64 = opts.get("timeout").unwrap_or(30);

            let mut req_headers = reqwest::header::HeaderMap::new();
            let mut devtools_headers = HashMap::new();
            if let Ok(headers_tbl) = opts.get::<Table>("headers") {
                for pair in headers_tbl.pairs::<String, String>() {
                    if let Ok((k, v)) = pair {
                        devtools_headers.insert(k.clone(), v.clone());
                        if let (Ok(hn), Ok(hv)) = (k.parse::<reqwest::header::HeaderName>(), v.parse()) {
                            req_headers.insert(hn, hv);
                        }
                    }
                }
            }

            let req_id = crate::devtools::state::record_global_http_request(
                &method_str,
                &url_str,
                devtools_headers,
                body_str.clone(),
            );

            let cb_key = SyncRegistryKey::new(_lua.create_registry_value(callback)?);
            let bridge = eng_http.bridge.clone();
            let lua_ref = eng_lua.clone();
            crate::tokio_runtime().spawn(async move {
                let client = reqwest::Client::builder()
                    .timeout(std::time::Duration::from_secs(timeout_secs))
                    .build()
                    .unwrap_or_default();

                let method = match method_str.to_uppercase().as_str() {
                    "POST" => reqwest::Method::POST,
                    "PUT" => reqwest::Method::PUT,
                    "DELETE" => reqwest::Method::DELETE,
                    "PATCH" => reqwest::Method::PATCH,
                    "HEAD" => reqwest::Method::HEAD,
                    _ => reqwest::Method::GET,
                };

                let mut req = client.request(method, &url_str).headers(req_headers);
                if let Some(b) = body_str {
                    req = req.body(b);
                }

                let response_result = match req.send().await {
                    Ok(resp) => {
                        let status = resp.status().as_u16();
                        let status_text = resp.status().canonical_reason().unwrap_or("").to_string();
                        let mut headers_map = HashMap::new();
                        for (k, v) in resp.headers() {
                            if let Ok(val) = v.to_str() {
                                headers_map.insert(k.as_str().to_string(), val.to_string());
                            }
                        }
                        let body = resp.text().await.unwrap_or_default();
                        Ok((status, status_text, headers_map, body))
                    }
                    Err(e) => Err(e.to_string()),
                };

                if let Some(id) = req_id {
                    match &response_result {
                        Ok((status, status_text, headers_map, body)) => {
                            crate::devtools::state::record_global_http_response(
                                id,
                                *status,
                                status_text,
                                headers_map.clone(),
                                Some(body.clone()),
                                body.len(),
                            );
                        }
                        Err(err_str) => {
                            crate::devtools::state::record_global_http_response(
                                id,
                                0,
                                err_str,
                                HashMap::new(),
                                None,
                                0,
                            );
                        }
                    }
                }
                let lua_arc_opt = lua_ref.read().clone();
                if let Some(lua_arc) = lua_arc_opt {
                    let lua = lua_arc.lock();
                    if let Ok(func) = cb_key.with(|k| lua.registry_value::<Function>(k)) {
                        match response_result {
                            Ok((status, status_text, headers_map, body)) => {
                                let resp_tbl = lua.create_table();
                                let _ = resp_tbl.set("status", status);
                                let _ = resp_tbl.set("status_text", status_text);
                                let _ = resp_tbl.set("ok", status >= 200 && status < 300);
                                let _ = resp_tbl.set("body", body);

                                let headers_tbl = lua.create_table();
                                for (k, v) in headers_map {
                                    let _ = headers_tbl.set(k.as_str(), v);
                                }
                                let _ = resp_tbl.set("headers", headers_tbl);

                                let _ = func.call::<()>((Value::Nil, Value::Table(resp_tbl)));
                            }
                            Err(err_str) => {
                                let _ = func.call::<()>((Value::String(lua.create_string(&err_str)), Value::Nil));
                            }
                        }
                    }
                }

                bridge.notify();
            });

            Ok(())
        })?,
    )?;
    #[cfg(not(feature = "net"))]
    lua.globals().set(
        "__async_http_fetch",
        lua.create_function(move |_lua, (_opts, callback): (Table, Function)| {
            callback.call::<()>(("net feature is disabled in this build", Value::Nil))?;
            Ok(())
        })?,
    )?;

    // 2. __async_fs_* functions
    let eng_fs_read = engine.clone();
    lua.globals().set(
        "__async_fs_read",
        lua.create_function(move |_lua, (path_str, callback): (String, Function)| {
            let cb_key = SyncRegistryKey::new(_lua.create_registry_value(callback)?);
            let bridge = eng_fs_read.bridge.clone();
            let lua_ref = eng_fs_read.lua_ref.clone();

            crate::tokio_runtime().spawn(async move {
                let res = tokio::fs::read_to_string(&path_str).await;
                if let Some(lua_arc) = lua_ref.read().clone() {
                    let lua = lua_arc.lock();
                    if let Ok(func) = cb_key.with(|k| lua.registry_value::<Function>(k)) {
                        match res {
                            Ok(content) => {
                                let _ = func.call::<()>((Value::Nil, Value::String(lua.create_string(&content))));
                            }
                            Err(e) => {
                                let err_str = e.to_string();
                                let _ = func.call::<()>((Value::String(lua.create_string(&err_str)), Value::Nil));
                            }
                        }
                    }
                }
                bridge.notify();
            });
            Ok(())
        })?,
    )?;

    let eng_fs_write = engine.clone();
    lua.globals().set(
        "__async_fs_write",
        lua.create_function(move |_lua, (path_str, data, callback): (String, Value, Function)| {
            let bytes = match data {
                Value::String(s) => s.as_bytes().to_vec(),
                Value::Integer(i) => i.to_string().into_bytes(),
                Value::Number(n) => n.to_string().into_bytes(),
                _ => Vec::new(),
            };

            let cb_key = SyncRegistryKey::new(_lua.create_registry_value(callback)?);
            let bridge = eng_fs_write.bridge.clone();
            let lua_ref = eng_fs_write.lua_ref.clone();

            crate::tokio_runtime().spawn(async move {
                if let Some(parent) = Path::new(&path_str).parent() {
                    let _ = tokio::fs::create_dir_all(parent).await;
                }
                let res = tokio::fs::write(&path_str, bytes).await;
                if let Some(lua_arc) = lua_ref.read().clone() {
                    let lua = lua_arc.lock();
                    if let Ok(func) = cb_key.with(|k| lua.registry_value::<Function>(k)) {
                        match res {
                            Ok(()) => {
                                let _ = func.call::<()>((Value::Nil, Value::Boolean(true)));
                            }
                            Err(e) => {
                                let err_str = e.to_string();
                                let _ = func.call::<()>((Value::String(lua.create_string(&err_str)), Value::Nil));
                            }
                        }
                    }
                }
                bridge.notify();
            });
            Ok(())
        })?,
    )?;

    let eng_fs_exists = engine.clone();
    lua.globals().set(
        "__async_fs_exists",
        lua.create_function(move |_lua, (path_str, callback): (String, Function)| {
            let cb_key = SyncRegistryKey::new(_lua.create_registry_value(callback)?);
            let bridge = eng_fs_exists.bridge.clone();
            let lua_ref = eng_fs_exists.lua_ref.clone();

            crate::tokio_runtime().spawn(async move {
                let exists = Path::new(&path_str).exists();
                if let Some(lua_arc) = lua_ref.read().clone() {
                    let lua = lua_arc.lock();
                    if let Ok(func) = cb_key.with(|k| lua.registry_value::<Function>(k)) {
                        let _ = func.call::<()>(Value::Boolean(exists));
                    }
                }
                bridge.notify();
            });
            Ok(())
        })?,
    )?;

    let eng_fs_list = engine.clone();
    lua.globals().set(
        "__async_fs_list_dir",
        lua.create_function(move |_lua, (path_str, callback): (String, Function)| {
            let cb_key = SyncRegistryKey::new(_lua.create_registry_value(callback)?);
            let bridge = eng_fs_list.bridge.clone();
            let lua_ref = eng_fs_list.lua_ref.clone();

            crate::tokio_runtime().spawn(async move {
                let mut entries = Vec::new();
                let read_res = tokio::fs::read_dir(&path_str).await;

                if let Ok(mut dir) = read_res {
                    while let Ok(Some(entry)) = dir.next_entry().await {
                        let name = entry.file_name().to_string_lossy().to_string();
                        let is_dir = entry.file_type().await.map(|t| t.is_dir()).unwrap_or(false);
                        let size = entry.metadata().await.map(|m| m.len()).unwrap_or(0);
                        entries.push((name, is_dir, size));
                    }
                }

                if let Some(lua_arc) = lua_ref.read().clone() {
                    let lua = lua_arc.lock();
                    if let Ok(func) = cb_key.with(|k| lua.registry_value::<Function>(k)) {
                        let tbl = lua.create_table();
                        for (i, (name, is_dir, sz)) in entries.into_iter().enumerate() {
                            let item = lua.create_table();
                            let _ = item.set("name", name);
                            let _ = item.set("is_dir", is_dir);
                            let _ = item.set("size", sz);
                            let _ = tbl.set(i + 1, item);
                        }
                        let _ = func.call::<()>((Value::Nil, Value::Table(tbl)));
                    }
                }
                bridge.notify();
            });
            Ok(())
        })?,
    )?;

    let eng_fs_rem = engine.clone();
    lua.globals().set(
        "__async_fs_remove",
        lua.create_function(move |_lua, (path_str, callback): (String, Function)| {
            let cb_key = SyncRegistryKey::new(_lua.create_registry_value(callback)?);
            let bridge = eng_fs_rem.bridge.clone();
            let lua_ref = eng_fs_rem.lua_ref.clone();

            crate::tokio_runtime().spawn(async move {
                let path = Path::new(&path_str);
                let res = if path.is_dir() {
                    tokio::fs::remove_dir_all(path).await
                } else {
                    tokio::fs::remove_file(path).await
                };

                if let Some(lua_arc) = lua_ref.read().clone() {
                    let lua = lua_arc.lock();
                    if let Ok(func) = cb_key.with(|k| lua.registry_value::<Function>(k)) {
                        match res {
                            Ok(()) => {
                                let _ = func.call::<()>((Value::Nil, Value::Boolean(true)));
                            }
                            Err(e) => {
                                let _ = func.call::<()>((Value::String(lua.create_string(&e.to_string())), Value::Nil));
                            }
                        }
                    }
                }
                bridge.notify();
            });
            Ok(())
        })?,
    )?;

    let eng_fs_mkdir = engine.clone();
    lua.globals().set(
        "__async_fs_create_dir",
        lua.create_function(move |_lua, (path_str, callback): (String, Function)| {
            let cb_key = SyncRegistryKey::new(_lua.create_registry_value(callback)?);
            let bridge = eng_fs_mkdir.bridge.clone();
            let lua_ref = eng_fs_mkdir.lua_ref.clone();

            crate::tokio_runtime().spawn(async move {
                let res = tokio::fs::create_dir_all(&path_str).await;
                if let Some(lua_arc) = lua_ref.read().clone() {
                    let lua = lua_arc.lock();
                    if let Ok(func) = cb_key.with(|k| lua.registry_value::<Function>(k)) {
                        match res {
                            Ok(()) => {
                                let _ = func.call::<()>((Value::Nil, Value::Boolean(true)));
                            }
                            Err(e) => {
                                let _ = func.call::<()>((Value::String(lua.create_string(&e.to_string())), Value::Nil));
                            }
                        }
                    }
                }
                bridge.notify();
            });
            Ok(())
        })?,
    )?;

    // 3. __async_process_* functions
    let eng_proc = engine.clone();
    lua.globals().set(
        "__async_process_spawn",
        lua.create_function(move |_lua, (cmd_str, args_tbl, _opts): (String, Table, Table)| {
            let proc_id = NEXT_PROC_ID.fetch_add(1, Ordering::SeqCst);
            let mut args = Vec::new();
            for i in 1..=args_tbl.raw_len() {
                if let Ok(val) = args_tbl.raw_get::<String>(i) {
                    args.push(val);
                }
            }

            let mut command = Command::new(&cmd_str);
            command
                .args(&args)
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .stdin(Stdio::piped());

            let spawn_res = command.spawn();

            match spawn_res {
                Ok(mut child) => {
                    let stdin = child.stdin.take();
                    let stdout = child.stdout.take();
                    let stderr = child.stderr.take();

                    eng_proc.processes.lock().insert(
                        proc_id,
                        ProcessHandle {
                            stdin,
                            child: Some(child),
                        },
                    );

                    let eng_out = eng_proc.clone();
                    if let Some(out) = stdout {
                        crate::tokio_runtime().spawn(async move {
                            let mut reader = BufReader::new(out).lines();
                            while let Ok(Some(line)) = reader.next_line().await {
                                let handlers = eng_out.process_handlers.lock();
                                if let Some((Some(stdout_fn), _, _)) = handlers.get(&proc_id) {
                                    if let Some(lua_arc) = eng_out.lua_ref.read().clone() {
                                        let lua = lua_arc.lock();
                                        if let Ok(func) = stdout_fn.with(|k| lua.registry_value::<Function>(k)) {
                                            let chunk = format!("{line}\n");
                                            let _ = func.call::<()>(Value::String(lua.create_string(&chunk)));
                                        }
                                    }
                                    eng_out.bridge.notify();
                                }
                            }
                        });
                    }

                    let eng_err = eng_proc.clone();
                    if let Some(err) = stderr {
                        crate::tokio_runtime().spawn(async move {
                            let mut reader = BufReader::new(err).lines();
                            while let Ok(Some(line)) = reader.next_line().await {
                                let handlers = eng_err.process_handlers.lock();
                                if let Some((_, Some(stderr_fn), _)) = handlers.get(&proc_id) {
                                    if let Some(lua_arc) = eng_err.lua_ref.read().clone() {
                                        let lua = lua_arc.lock();
                                        if let Ok(func) = stderr_fn.with(|k| lua.registry_value::<Function>(k)) {
                                            let chunk = format!("{line}\n");
                                            let _ = func.call::<()>(Value::String(lua.create_string(&chunk)));
                                        }
                                    }
                                    eng_err.bridge.notify();
                                }
                            }
                        });
                    }

                    let eng_exit = eng_proc.clone();
                    crate::tokio_runtime().spawn(async move {
                        let child_opt = {
                            let mut procs = eng_exit.processes.lock();
                            procs.get_mut(&proc_id).and_then(|p| p.child.take())
                        };

                        if let Some(mut c) = child_opt {
                            let status = c.wait().await;
                            let code = status.map(|s| s.code().unwrap_or(-1)).unwrap_or(-1);

                            let handlers = eng_exit.process_handlers.lock();
                            if let Some((_, _, Some(exit_fn))) = handlers.get(&proc_id) {
                                if let Some(lua_arc) = eng_exit.lua_ref.read().clone() {
                                    let lua = lua_arc.lock();
                                    if let Ok(func) = exit_fn.with(|k| lua.registry_value::<Function>(k)) {
                                        let exit_tbl = lua.create_table();
                                        let _ = exit_tbl.set("code", code);
                                        let _ = exit_tbl.set("success", code == 0);
                                        let _ = func.call::<()>(Value::Table(exit_tbl));
                                    }
                                }
                                eng_exit.bridge.notify();
                            }
                        }
                    });

                    Ok(proc_id)
                }
                Err(e) => Err(mlua::Error::RuntimeError(format!("Failed to spawn process: {e}"))),
            }
        })?,
    )?;

    let eng_reg_h = engine.clone();
    lua.globals().set(
        "__register_process_handlers",
        lua.create_function(move |_lua, (proc_id, handlers_tbl): (u64, Table)| {
            let stdout_key = handlers_tbl
                .get::<Function>("on_stdout")
                .ok()
                .map(|f| SyncRegistryKey::new(_lua.create_registry_value(f).unwrap()));
            let stderr_key = handlers_tbl
                .get::<Function>("on_stderr")
                .ok()
                .map(|f| SyncRegistryKey::new(_lua.create_registry_value(f).unwrap()));
            let exit_key = handlers_tbl
                .get::<Function>("on_exit")
                .ok()
                .map(|f| SyncRegistryKey::new(_lua.create_registry_value(f).unwrap()));

            eng_reg_h
                .process_handlers
                .lock()
                .insert(proc_id, (stdout_key, stderr_key, exit_key));

            Ok(())
        })?,
    )?;

    let eng_stdin = engine.clone();
    lua.globals().set(
        "__async_process_stdin",
        lua.create_function(move |_lua, (proc_id, text): (u64, String)| {
            let mut procs = eng_stdin.processes.lock();
            if let Some(proc) = procs.get_mut(&proc_id) {
                if let Some(stdin) = proc.stdin.as_mut() {
                    let bytes = text.into_bytes();
                    let _ = stdin.write_all(&bytes);
                }
            }
            Ok(())
        })?,
    )?;

    let eng_kill = engine.clone();
    lua.globals().set(
        "__async_process_kill",
        lua.create_function(move |_lua, proc_id: u64| {
            let mut procs = eng_kill.processes.lock();
            if let Some(proc) = procs.get_mut(&proc_id) {
                if let Some(child) = proc.child.as_mut() {
                    let _ = child.start_kill();
                }
            }
            Ok(())
        })?,
    )?;

    // 4. Native Desktop Dialogs
    let eng_open_f = engine.clone();
    lua.globals().set(
        "__async_dialog_open_file",
        lua.create_function(move |_lua, (options, callback): (Table, Function)| {
            let cb_id = NEXT_CALLBACK_ID.fetch_add(1, Ordering::SeqCst);
            let cb_key = SyncRegistryKey::new(_lua.create_registry_value(callback)?);
            eng_open_f.dialog_callbacks.lock().insert(cb_id, cb_key);

            let multiple: bool = options.get("multiple").unwrap_or(false);
            let directories: bool = options.get("directories").unwrap_or(false);
            let prompt: Option<String> = options.get("title").ok();

            eng_open_f.os_bridge.push_action(OsAction::OpenFilePrompt {
                files: !directories,
                directories,
                multiple,
                prompt,
                callback_id: cb_id,
            });

            Ok(())
        })?,
    )?;

    let eng_save_f = engine.clone();
    lua.globals().set(
        "__async_dialog_save_file",
        lua.create_function(move |_lua, (options, callback): (Table, Function)| {
            let cb_id = NEXT_CALLBACK_ID.fetch_add(1, Ordering::SeqCst);
            let cb_key = SyncRegistryKey::new(_lua.create_registry_value(callback)?);
            eng_save_f.dialog_callbacks.lock().insert(cb_id, cb_key);

            let dir_str: String = options.get("directory").unwrap_or_else(|_| ".".to_string());
            let name_str: Option<String> = options.get("default_name").ok();

            eng_save_f.os_bridge.push_action(OsAction::SaveFilePrompt {
                directory: PathBuf::from(dir_str),
                suggested_name: name_str,
                callback_id: cb_id,
            });

            Ok(())
        })?,
    )?;

    let eng_pick_f = engine.clone();
    lua.globals().set(
        "__async_dialog_pick_folder",
        lua.create_function(move |_lua, (options, callback): (Table, Function)| {
            let cb_id = NEXT_CALLBACK_ID.fetch_add(1, Ordering::SeqCst);
            let cb_key = SyncRegistryKey::new(_lua.create_registry_value(callback)?);
            eng_pick_f.dialog_callbacks.lock().insert(cb_id, cb_key);

            let prompt: Option<String> = options.get("title").ok();

            eng_pick_f.os_bridge.push_action(OsAction::OpenFilePrompt {
                files: false,
                directories: true,
                multiple: false,
                prompt,
                callback_id: cb_id,
            });

            Ok(())
        })?,
    )?;

    let eng_msg = engine.clone();
    lua.globals().set(
        "__async_dialog_message",
        lua.create_function(move |_lua, (msg, options, callback): (String, Table, Function)| {
            let cb_id = NEXT_CALLBACK_ID.fetch_add(1, Ordering::SeqCst);
            let cb_key = SyncRegistryKey::new(_lua.create_registry_value(callback)?);
            eng_msg.dialog_callbacks.lock().insert(cb_id, cb_key);

            let level_str: String = options.get("level").unwrap_or_else(|_| "info".to_string());
            let title: Option<String> = options.get("title").ok();

            eng_msg.os_bridge.push_action(OsAction::MessagePrompt {
                level: level_str,
                message: msg,
                detail: title,
                callback_id: cb_id,
            });

            Ok(())
        })?,
    )?;

    // 5. Hotkeys
    let eng_hk = engine.clone();
    lua.globals().set(
        "__hotkey_bind",
        lua.create_function(move |_lua, (shortcut, callback, global): (String, Function, bool)| {
            let key = SyncRegistryKey::new(_lua.create_registry_value(callback)?);
            eng_hk
                .hotkeys
                .lock()
                .insert(shortcut.to_lowercase(), (key, global));
            Ok(())
        })?,
    )?;

    let eng_hk_un = engine.clone();
    lua.globals().set(
        "__hotkey_unbind",
        lua.create_function(move |_lua, shortcut: String| {
            eng_hk_un.hotkeys.lock().remove(&shortcut.to_lowercase());
            Ok(())
        })?,
    )?;

    // 6. System Tray
    let eng_tray = engine.clone();
    lua.globals().set(
        "__tray_create",
        lua.create_function(move |_lua, options: Table| {
            let id = NEXT_CALLBACK_ID.fetch_add(1, Ordering::SeqCst);
            let title: String = options.get("title").unwrap_or_default();
            eng_tray.tray_items.lock().insert(id, title);
            Ok(id)
        })?,
    )?;

    let eng_tray_t = engine.clone();
    lua.globals().set(
        "__tray_set_title",
        lua.create_function(move |_lua, (id, title): (u64, String)| {
            eng_tray_t.tray_items.lock().insert(id, title);
            Ok(())
        })?,
    )?;

    let _eng_tray_tt = engine.clone();
    lua.globals().set(
        "__tray_set_tooltip",
        lua.create_function(move |_lua, (_id, _tooltip): (u64, String)| {
            Ok(())
        })?,
    )?;

    let eng_tray_d = engine.clone();
    lua.globals().set(
        "__tray_destroy",
        lua.create_function(move |_lua, id: u64| {
            eng_tray_d.tray_items.lock().remove(&id);
            Ok(())
        })?,
    )?;

    // 7. Load and mount async_core.lua
    let async_core: Table = lua.load(ASYNC_CORE_LUA).set_name("async_core.lua").eval()?;
    lua.globals().set("async", async_core.get::<Table>("async")?)?;
    lua.globals().set("http", async_core.get::<Table>("http")?)?;
    lua.globals().set("fs", async_core.get::<Table>("fs")?)?;
    lua.globals().set("process", async_core.get::<Table>("process")?)?;
    lua.globals().set("dialog", async_core.get::<Table>("dialog")?)?;
    lua.globals().set("hotkey", async_core.get::<Table>("hotkey")?)?;
    lua.globals().set("tray", async_core.get::<Table>("tray")?)?;

    // Also register in package.loaded
    if let Ok(pkg) = lua.globals().get::<Table>("package") {
        if let Ok(loaded) = pkg.get::<Table>("loaded") {
            let _ = loaded.set("async", async_core.get::<Table>("async")?);
            let _ = loaded.set("http", async_core.get::<Table>("http")?);
            let _ = loaded.set("fs", async_core.get::<Table>("fs")?);
            let _ = loaded.set("process", async_core.get::<Table>("process")?);
            let _ = loaded.set("dialog", async_core.get::<Table>("dialog")?);
            let _ = loaded.set("hotkey", async_core.get::<Table>("hotkey")?);
            let _ = loaded.set("tray", async_core.get::<Table>("tray")?);
        }
    }

    Ok(())
}
