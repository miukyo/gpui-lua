pub mod async_rt;
#[cfg(feature = "crypto")]
pub mod crypto;
#[cfg(feature = "ffi")]
pub mod ffi;
pub mod fs;
pub mod json;
pub mod media;
pub mod os;
#[cfg(feature = "sqlite")]
pub mod storage_db;
pub mod timer;
#[cfg(feature = "net")]
pub mod ws_sse;
#[cfg(feature = "webrtc")]
pub mod webrtc;
pub use async_rt::AsyncEngine;
pub use os::{OsAction, OsBridge, WindowInfo};
pub use timer::TimerEngine;
use mlua::{Lua, MultiValue, Result, Table, Value};

const COLORS_LUA: &str = include_str!("lua/colors.lua");
const UI_LUA: &str = include_str!("lua/ui.lua");

pub fn register(lua: &Lua, timer_engine: TimerEngine, os_bridge: OsBridge, async_engine: AsyncEngine) -> Result<()> {
    // 1. Register timer, json, fs, os, async_rt, crypto, storage_db, ws_sse, ffi
    timer::register(lua, timer_engine)?;
    json::register(lua)?;
    fs::register(lua)?;
    os::register(lua, os_bridge)?;
    async_rt::register(lua, async_engine.clone())?;
    #[cfg(feature = "crypto")]
    crypto::register(lua)?;
    #[cfg(feature = "sqlite")]
    storage_db::register(lua)?;
    #[cfg(feature = "net")]
    ws_sse::register(lua, async_engine.lua_ref(), async_engine.bridge())?;
    #[cfg(feature = "ffi")]
    ffi::register(lua)?;
    #[cfg(feature = "webrtc")]
    webrtc::register(lua, async_engine.lua_ref(), async_engine.bridge(), #[cfg(feature = "media")] None, #[cfg(feature = "media")] None)?;
    let log_tbl = lua.create_table();

    log_tbl.set(
        "debug",
        lua.create_function(|_lua, args: MultiValue| {
            let msg = format_multi_values(&args);
            log::debug!("{msg}");
            println!("[DEBUG] {msg}");
            crate::devtools::state::record_global_log(crate::devtools::state::LogLevel::Debug, &msg);
            Ok(())
        })?,
    )?;

    log_tbl.set(
        "info",
        lua.create_function(|_lua, args: MultiValue| {
            let msg = format_multi_values(&args);
            log::info!("{msg}");
            println!("[INFO] {msg}");
            crate::devtools::state::record_global_log(crate::devtools::state::LogLevel::Info, &msg);
            Ok(())
        })?,
    )?;

    log_tbl.set(
        "warn",
        lua.create_function(|_lua, args: MultiValue| {
            let msg = format_multi_values(&args);
            log::warn!("{msg}");
            eprintln!("[WARN] {msg}");
            crate::devtools::state::record_global_log(crate::devtools::state::LogLevel::Warn, &msg);
            Ok(())
        })?,
    )?;

    log_tbl.set(
        "error",
        lua.create_function(|_lua, args: MultiValue| {
            let msg = format_multi_values(&args);
            log::error!("{msg}");
            eprintln!("[ERROR] {msg}");
            crate::devtools::state::record_global_log(crate::devtools::state::LogLevel::Error, &msg);
            Ok(())
        })?,
    )?;

    lua.globals().set("log", log_tbl)?;

    // 3. Register colors.lua
    let colors_table: Table = lua.load(COLORS_LUA).set_name("colors.lua").eval()?;
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
    loaded.set("colors", colors_table.clone())?;
    lua.globals().set("colors", colors_table)?;

    // 4. Register ui.lua components into both package.loaded["ui"] and globals().ui
    let ui_components: Table = lua.load(UI_LUA).set_name("ui.lua").eval()?;
    loaded.set("ui.components", ui_components.clone())?;

    if let Ok(ui_tbl) = lua.globals().get::<Table>("ui") {
        for pair in ui_components.pairs::<String, Value>() {
            if let Ok((k, v)) = pair {
                ui_tbl.set(k.as_str(), v)?;
            }
        }
        loaded.set("ui", ui_tbl)?;
    }

    Ok(())
}
#[cfg(feature = "media")]
pub fn register_media(lua: &Lua, video: ::media::VideoManager, audio: ::media::AudioManager) -> Result<()> {
    media::register(lua, video, audio)?;
    Ok(())
}

pub fn format_lua_value_pretty(val: &Value) -> String {
    match val {
        Value::Nil => "nil".to_string(),
        Value::Boolean(b) => b.to_string(),
        Value::Integer(i) => i.to_string(),
        Value::Number(n) => n.to_string(),
        Value::String(s) => s.to_str().map(|v| v.to_string()).unwrap_or_default(),
        Value::Table(t) => {
            if let Ok(json_val) = crate::stdlib::json::lua_value_to_json(Value::Table(t.clone())) {
                if let Ok(pretty) = serde_json::to_string_pretty(&json_val) {
                    return pretty;
                }
            }
            let mut entries = Vec::new();
            for pair in t.clone().pairs::<Value, Value>() {
                if let Ok((k, v)) = pair {
                    let k_s = match k {
                        Value::String(s) => s.to_str().unwrap_or_default().to_string(),
                        Value::Integer(i) => i.to_string(),
                        other => format!("{other:?}"),
                    };
                    let v_s = format_lua_value_pretty(&v);
                    entries.push(format!("  \"{k_s}\": {v_s}"));
                }
            }
            if entries.is_empty() {
                "{}".to_string()
            } else {
                format!("{{\n{}\n}}", entries.join(",\n"))
            }
        }
        Value::Function(_) => "[function]".to_string(),
        Value::UserData(_) => "[userdata]".to_string(),
        Value::LightUserData(_) => "[lightuserdata]".to_string(),
        Value::Thread(_) => "[thread]".to_string(),
        Value::Error(e) => format!("[error: {e}]"),
        _ => format!("{val:?}"),
    }
}

pub fn format_multi_values(args: &MultiValue) -> String {
    let mut parts = Vec::new();
    for val in args.iter() {
        parts.push(format_lua_value_pretty(val));
    }
    parts.join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dsl;
    use crate::reactive::bridge::ReactiveBridge;

    #[test]
    fn test_stdlib_json_and_fs() -> Result<()> {
        let lua = Lua::new();
        let bridge = ReactiveBridge::new();
        let timer_engine = TimerEngine::new(bridge.clone());
        let os_bridge = OsBridge::new(bridge.clone());
        let async_engine = AsyncEngine::new(bridge.clone(), os_bridge.clone());
        dsl::register(&lua)?;
        register(&lua, timer_engine, os_bridge, async_engine)?;
        let temp_dir = tempfile::tempdir().unwrap();
        let file_path = temp_dir.path().join("test.json");
        let path_str = file_path.to_string_lossy().to_string();

        lua.globals().set("test_path", path_str)?;

        lua.load(r#"
            local obj = { title = "Testing", count = 99 }
            local encoded = json.encode(obj)
            assert(type(encoded) == "string", "json.encode failed")

            local decoded = json.decode(encoded)
            assert(decoded.title == "Testing", "json.decode title failed")
            assert(decoded.count == 99, "json.decode count failed")

            local ok, err = fs.write(test_path, encoded)
            assert(ok, "fs.write failed: " .. tostring(err))
            assert(fs.exists(test_path), "fs.exists failed")

            local read_data, read_err = fs.read(test_path)
            assert(read_data == encoded, "fs.read failed: " .. tostring(read_err))
        "#).exec()?;

        Ok(())
    }

    #[test]
    fn test_stdlib_colors_and_ui_components() -> Result<()> {
        let lua = Lua::new();
        let bridge = ReactiveBridge::new();
        let timer_engine = TimerEngine::new(bridge.clone());
        let os_bridge = OsBridge::new(bridge.clone());
        let async_engine = AsyncEngine::new(bridge.clone(), os_bridge.clone());
        dsl::register(&lua)?;
        register(&lua, timer_engine, os_bridge, async_engine)?;
        lua.load(r##"
            assert(colors.rgb(255, 0, 0) == "#ff0000", "RGB helper failed")
            assert(colors.hex("ff0000") == "#ff0000", "Hex helper failed")

            -- UI Input and Textarea Components
            local inp = ui.input({
                id = "test_inp",
                value = "Initial Text",
                placeholder = "Type here...",
                max_length = 50,
                pattern = "^[%w%s]+$"
            })
            assert(inp ~= nil, "ui.input failed")

            local tarea = ui.textarea({
                id = "test_tarea",
                value = "Line 1\nLine 2",
                rows = 5
            })
            assert(tarea ~= nil, "ui.textarea failed")

            local row = ui.row({ gap = 8, ui.text("A"), ui.text("B") })
            assert(row ~= nil, "ui.row failed")

            local col = ui.column({ gap = 4, ui.text("C") })
            assert(col ~= nil, "ui.column failed")

            local stack = ui.stack({ ui.text("Top") })
            assert(stack ~= nil, "ui.stack failed")
        "##).exec()?;
        Ok(())
    }

    #[test]
    fn test_stdlib_os_library() -> Result<()> {
        let lua = Lua::new();
        let bridge = ReactiveBridge::new();
        let timer_engine = TimerEngine::new(bridge.clone());
        let os_bridge = OsBridge::new(bridge.clone());
        let async_engine = AsyncEngine::new(bridge.clone(), os_bridge.clone());
        register(&lua, timer_engine, os_bridge.clone(), async_engine)?;

        lua.load(r#"
            -- Clipboard
            os.clipboard.set("copied text")
            assert(os.clipboard.get() == "copied text", "Clipboard get failed")
            os.clipboard.clear()
            assert(os.clipboard.get() == "", "Clipboard clear failed")

            -- Window controls
            os.window.set_title("My App")
            os.window.minimize()
            os.window.maximize()
            os.window.toggle_fullscreen()
            assert(os.window.is_fullscreen() == false)
            assert(os.window.is_maximized() == false)
            local size = os.window.get_size()
            assert(size.width == 0.0)

            -- Platform
            assert(os.platform ~= nil, "os.platform is nil")
            assert(os.arch ~= nil, "os.arch is nil")

            -- App
            os.app.quit()
        "#).exec()?;

        let actions = os_bridge.drain_actions();
        assert!(actions.contains(&OsAction::WriteClipboard("copied text".to_string())));
        assert!(actions.contains(&OsAction::SetWindowTitle("My App".to_string())));
        assert!(actions.contains(&OsAction::MinimizeWindow));
        assert!(actions.contains(&OsAction::ZoomWindow));
        assert!(actions.contains(&OsAction::ToggleFullscreen));
        assert!(actions.contains(&OsAction::QuitApp));

        Ok(())
    }

    #[test]
    fn test_async_runtime_and_events() -> Result<()> {
        let lua = Lua::new();
        let lua_arc = std::sync::Arc::new(parking_lot::Mutex::new(lua));
        let bridge = ReactiveBridge::new();
        let timer_engine = TimerEngine::new(bridge.clone());
        let os_bridge = OsBridge::new(bridge.clone());
        let async_engine = AsyncEngine::new(bridge.clone(), os_bridge.clone());
        async_engine.set_lua(lua_arc.clone());
        {
            let lua_guard = lua_arc.lock();
            dsl::register(&lua_guard)?;
            register(&lua_guard, timer_engine, os_bridge.clone(), async_engine.clone())?;
        }
        let lua = lua_arc.lock();

        lua.load(r#"
            assert(async ~= nil, "async module missing")
            assert(http ~= nil, "http module missing")
            assert(fs ~= nil, "fs module missing")
            assert(process ~= nil, "process module missing")
            assert(dialog ~= nil, "dialog module missing")
            assert(hotkey ~= nil, "hotkey module missing")
            assert(window ~= nil, "window module missing")

            local executed = false
            async.spawn(function()
                executed = true
            end)
            assert(executed, "async.spawn failed to run immediately")

            -- Test hotkey registration
            local hotkey_hit = false
            hotkey.bind("ctrl+s", function()
                hotkey_hit = true
            end)

            -- Test window API methods
            window.set_title("Async Window")
            window.set_size(800, 600)
            window.minimize()
            window.maximize()

            -- Test event handler definitions on ui.div
            local root = ui.div({
                on_click = function() end,
                on_mouse_down = function() end,
                on_mouse_up = function() end,
                on_mouse_move = function() end,
                on_key_down = function() end,
                on_key_up = function() end,
                on_scroll_wheel = function() end
            })
            assert(root ~= nil, "ui.div with events failed")

            -- Test HTTP client invocation with callback as second argument
            local p1 = http.get("https://example.com/api", function() end)
            assert(p1 ~= nil, "http.get with callback must return promise")

            local p2 = http.get("https://example.com/api", { headers = { ["Authorization"] = "Bearer token" } }, function() end)
            assert(p2 ~= nil, "http.get with options and callback must return promise")

            local p3 = http.post("https://example.com/api", { name = "test" }, function() end)
            assert(p3 ~= nil, "http.post with callback must return promise")
        "#).exec()?;
        drop(lua);

        // Trigger hotkey from Rust
        assert!(async_engine.trigger_hotkey("ctrl+s"));

        let actions = os_bridge.drain_actions();
        assert!(actions.contains(&OsAction::SetWindowTitle("Async Window".to_string())));
        assert!(actions.contains(&OsAction::SetWindowSize(800.0, 600.0)));
        assert!(actions.contains(&OsAction::MinimizeWindow));
        assert!(actions.contains(&OsAction::ZoomWindow));

        Ok(())
    }

    #[cfg(all(feature = "crypto", feature = "sqlite", feature = "ffi", feature = "net", feature = "webrtc"))]
    #[test]
    fn test_crypto_storage_db_ffi() -> Result<()> {
        let lua = Lua::new();
        let bridge = ReactiveBridge::new();
        let timer_engine = TimerEngine::new(bridge.clone());
        let os_bridge = OsBridge::new(bridge.clone());
        let async_engine = AsyncEngine::new(bridge.clone(), os_bridge.clone());
        dsl::register(&lua)?;
        register(&lua, timer_engine, os_bridge, async_engine)?;

        lua.load(r#"
            -- 1. Crypto
            assert(crypto ~= nil, "crypto module missing")
            local h256 = crypto.sha256("hello world")
            assert(#h256 == 64, "sha256 length mismatch")
            local h512 = crypto.sha512("hello world")
            assert(#h512 == 128, "sha512 length mismatch")
            local hmac = crypto.hmac_sha256("secret_key", "payload")
            assert(#hmac == 64, "hmac length mismatch")
            local u = crypto.uuid()
            assert(#u == 36, "uuid length mismatch")
            local b64 = crypto.base64_encode("gpui.lua")
            assert(crypto.base64_decode(b64) == "gpui.lua", "base64 decode mismatch")

            -- 2. LocalStorage
            assert(storage ~= nil, "storage module missing")
            storage.set("app_version", "1.0.0")
            storage.set("theme", { dark = true, accent = "blue" })
            assert(storage.get("app_version") == "1.0.0", "storage get version failed")
            local theme = storage.get("theme")
            assert(theme.dark == true and theme.accent == "blue", "storage get table failed")

            -- 3. Database (SQLite)
            assert(db ~= nil, "db module missing")
            local conn = db.open_memory()
            assert(conn ~= nil, "open_memory failed")
            conn:exec([[
                CREATE TABLE items (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    name TEXT NOT NULL,
                    qty INTEGER NOT NULL
                );
            ]])
            local res1 = conn:exec("INSERT INTO items (name, qty) VALUES (?, ?)", { "Apple", 10 })
            assert(res1.changes == 1, "insert changes mismatch")
            local res2 = conn:exec("INSERT INTO items (name, qty) VALUES (?, ?)", { "Banana", 25 })
            assert(res2.last_insert_rowid == 2, "last_insert_rowid mismatch")

            local rows = conn:query("SELECT * FROM items ORDER BY id ASC")
            assert(#rows == 2, "query row count mismatch")
            assert(rows[1].name == "Apple" and rows[1].qty == 10, "row 1 data mismatch")
            assert(rows[2].name == "Banana" and rows[2].qty == 25, "row 2 data mismatch")

            local single = conn:query_row("SELECT name FROM items WHERE qty > ?", { 20 })
            assert(single.name == "Banana", "query_row mismatch")
            conn:close()

            -- 4. FFI module presence
            assert(ffi ~= nil, "ffi module missing")
            assert(ffi.os ~= nil, "ffi.os missing")
            assert(ffi.arch ~= nil, "ffi.arch missing")

            -- 5. Networking WebSocket & SSE presence
            assert(net ~= nil, "net module missing")
            assert(net.websocket ~= nil, "net.websocket missing")
            assert(net.sse ~= nil, "net.sse missing")

            -- 6. WebRTC PeerConnection & DataChannel
            assert(webrtc ~= nil, "webrtc module missing")
            assert(webrtc.create_peer_connection ~= nil, "webrtc.create_peer_connection missing")
            local peer = webrtc.create_peer_connection({
                ice_servers = {
                    { urls = "stun:stun.l.google.com:19302" }
                }
            })
            assert(peer ~= nil, "create_peer_connection failed")
            local dc = peer:create_data_channel("chat")
            assert(dc ~= nil, "create_data_channel failed")
            peer:close()
        "#).exec()?;

        Ok(())
    }
}
