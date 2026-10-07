use crate::reactive::bridge::ReactiveBridge;
use mlua::{Lua, Result, Value};
use parking_lot::RwLock;
use std::sync::Arc;

#[derive(Clone, Debug, PartialEq)]
pub enum OsAction {
    WriteClipboard(String),
    OpenUrl(String),
    SetWindowTitle(String),
    SetWindowSize(f32, f32),
    SetWindowPosition(f32, f32),
    CenterWindow,
    FocusWindow,
    MinimizeWindow,
    ZoomWindow,
    MaximizeWindow,
    RestoreWindow,
    ToggleFullscreen,
    CloseWindow,
    StartWindowMove,
    SetWindowAlwaysOnTop(bool),
    SetWindowResizable(bool),
    SetWindowDecorations(bool),
    QuitApp,
    OpenFilePrompt {
        files: bool,
        directories: bool,
        multiple: bool,
        prompt: Option<String>,
        callback_id: u64,
    },
    SaveFilePrompt {
        directory: std::path::PathBuf,
        suggested_name: Option<String>,
        callback_id: u64,
    },
    MessagePrompt {
        level: String,
        message: String,
        detail: Option<String>,
        callback_id: u64,
    },
}

#[derive(Clone, Debug, Default)]
pub struct WindowInfo {
    pub is_fullscreen: bool,
    pub is_maximized: bool,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub title: String,
    pub is_focused: bool,
}

#[derive(Clone, Default)]
pub struct OsBridge {
    actions: Arc<RwLock<Vec<OsAction>>>,
    window_info: Arc<RwLock<WindowInfo>>,
    clipboard_text: Arc<RwLock<Option<String>>>,
    bridge: Option<ReactiveBridge>,
}

impl OsBridge {
    pub fn new(bridge: ReactiveBridge) -> Self {
        Self {
            actions: Arc::new(RwLock::new(Vec::new())),
            window_info: Arc::new(RwLock::new(WindowInfo::default())),
            clipboard_text: Arc::new(RwLock::new(None)),
            bridge: Some(bridge),
        }
    }

    pub fn push_action(&self, action: OsAction) {
        self.actions.write().push(action);
        if let Some(b) = &self.bridge {
            b.notify();
        }
    }

    pub fn drain_actions(&self) -> Vec<OsAction> {
        std::mem::take(&mut *self.actions.write())
    }

    pub fn set_window_info(&self, info: WindowInfo) {
        *self.window_info.write() = info;
    }

    pub fn get_window_info(&self) -> WindowInfo {
        self.window_info.read().clone()
    }

    pub fn set_clipboard_cache(&self, text: String) {
        *self.clipboard_text.write() = Some(text);
    }

    pub fn get_clipboard_cache(&self) -> Option<String> {
        self.clipboard_text.read().clone()
    }
}

pub fn register(lua: &Lua, os_bridge: OsBridge) -> Result<()> {
    let os_tbl = lua.create_table();

    // Platform constants
    #[cfg(target_os = "windows")]
    os_tbl.set("platform", "windows")?;
    #[cfg(target_os = "macos")]
    os_tbl.set("platform", "macos")?;
    #[cfg(target_os = "linux")]
    os_tbl.set("platform", "linux")?;
    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    os_tbl.set("platform", "unknown")?;

    os_tbl.set("arch", std::env::consts::ARCH)?;

    // 1. Clipboard submodule
    let clipboard = lua.create_table();
    let bridge_set = os_bridge.clone();
    clipboard.set(
        "set",
        lua.create_function(move |_lua, text: String| {
            bridge_set.set_clipboard_cache(text.clone());
            bridge_set.push_action(OsAction::WriteClipboard(text));
            Ok(())
        })?,
    )?;

    let bridge_get = os_bridge.clone();
    clipboard.set(
        "get",
        lua.create_function(move |lua, ()| match bridge_get.get_clipboard_cache() {
            Some(s) => Ok(Value::String(lua.create_string(&s))),
            None => Ok(Value::Nil),
        })?,
    )?;

    let bridge_clear = os_bridge.clone();
    clipboard.set(
        "clear",
        lua.create_function(move |_lua, ()| {
            bridge_clear.set_clipboard_cache(String::new());
            bridge_clear.push_action(OsAction::WriteClipboard(String::new()));
            Ok(())
        })?,
    )?;
    os_tbl.set("clipboard", clipboard)?;

    // 2. Browser / URLs
    let bridge_url = os_bridge.clone();
    os_tbl.set(
        "open_url",
        lua.create_function(move |_lua, url: String| {
            bridge_url.push_action(OsAction::OpenUrl(url));
            Ok(())
        })?,
    )?;

    os_tbl.set(
        "open_path",
        lua.create_function(|_lua, path: String| {
            #[cfg(target_os = "windows")]
            let _ = std::process::Command::new("explorer").arg(&path).spawn();
            #[cfg(target_os = "macos")]
            let _ = std::process::Command::new("open").arg(&path).spawn();
            #[cfg(target_os = "linux")]
            let _ = std::process::Command::new("xdg-open").arg(&path).spawn();
            Ok(())
        })?,
    )?;

    // 3. Window submodule
    let window_tbl = lua.create_table();

    let bridge_title = os_bridge.clone();
    window_tbl.set(
        "set_title",
        lua.create_function(move |_lua, title: String| {
            bridge_title.push_action(OsAction::SetWindowTitle(title));
            Ok(())
        })?,
    )?;

    let bridge_min = os_bridge.clone();
    window_tbl.set(
        "minimize",
        lua.create_function(move |_lua, ()| {
            bridge_min.push_action(OsAction::MinimizeWindow);
            Ok(())
        })?,
    )?;

    let bridge_zoom = os_bridge.clone();
    window_tbl.set(
        "maximize",
        lua.create_function(move |_lua, ()| {
            bridge_zoom.push_action(OsAction::ZoomWindow);
            Ok(())
        })?,
    )?;

    let bridge_fs = os_bridge.clone();
    window_tbl.set(
        "toggle_fullscreen",
        lua.create_function(move |_lua, ()| {
            bridge_fs.push_action(OsAction::ToggleFullscreen);
            Ok(())
        })?,
    )?;

    let bridge_close = os_bridge.clone();
    window_tbl.set(
        "close",
        lua.create_function(move |_lua, ()| {
            bridge_close.push_action(OsAction::CloseWindow);
            Ok(())
        })?,
    )?;

    let bridge_move = os_bridge.clone();
    window_tbl.set(
        "start_move",
        lua.create_function(move |_lua, ()| {
            bridge_move.push_action(OsAction::StartWindowMove);
            Ok(())
        })?,
    )?;

    let bridge_state_fs = os_bridge.clone();
    window_tbl.set(
        "is_fullscreen",
        lua.create_function(move |_lua, ()| {
            Ok(bridge_state_fs.get_window_info().is_fullscreen)
        })?,
    )?;

    let bridge_state_max = os_bridge.clone();
    window_tbl.set(
        "is_maximized",
        lua.create_function(move |_lua, ()| {
            Ok(bridge_state_max.get_window_info().is_maximized)
        })?,
    )?;

    let bridge_size = os_bridge.clone();
    window_tbl.set(
        "get_size",
        lua.create_function(move |lua, ()| {
            let info = bridge_size.get_window_info();
            let size_tbl = lua.create_table();
            size_tbl.set("width", info.width)?;
            size_tbl.set("height", info.height)?;
            Ok(size_tbl)
        })?,
    )?;

    let bridge_set_size = os_bridge.clone();
    window_tbl.set(
        "set_size",
        lua.create_function(move |_lua, (w, h): (f32, f32)| {
            bridge_set_size.push_action(OsAction::SetWindowSize(w, h));
            Ok(())
        })?,
    )?;

    let bridge_set_pos = os_bridge.clone();
    window_tbl.set(
        "set_position",
        lua.create_function(move |_lua, (x, y): (f32, f32)| {
            bridge_set_pos.push_action(OsAction::SetWindowPosition(x, y));
            Ok(())
        })?,
    )?;

    let bridge_center = os_bridge.clone();
    window_tbl.set(
        "center",
        lua.create_function(move |_lua, ()| {
            bridge_center.push_action(OsAction::CenterWindow);
            Ok(())
        })?,
    )?;

    let bridge_focus = os_bridge.clone();
    window_tbl.set(
        "focus",
        lua.create_function(move |_lua, ()| {
            bridge_focus.push_action(OsAction::FocusWindow);
            Ok(())
        })?,
    )?;

    let bridge_ontop = os_bridge.clone();
    window_tbl.set(
        "set_always_on_top",
        lua.create_function(move |_lua, on_top: bool| {
            bridge_ontop.push_action(OsAction::SetWindowAlwaysOnTop(on_top));
            Ok(())
        })?,
    )?;

    let bridge_resiz = os_bridge.clone();
    window_tbl.set(
        "set_resizable",
        lua.create_function(move |_lua, resizable: bool| {
            bridge_resiz.push_action(OsAction::SetWindowResizable(resizable));
            Ok(())
        })?,
    )?;

    let bridge_dec = os_bridge.clone();
    window_tbl.set(
        "set_decorations",
        lua.create_function(move |_lua, dec: bool| {
            bridge_dec.push_action(OsAction::SetWindowDecorations(dec));
            Ok(())
        })?,
    )?;

    os_tbl.set("window", window_tbl.clone())?;
    lua.globals().set("window", window_tbl)?;

    // 4. App submodule
    let app_tbl = lua.create_table();
    let bridge_quit = os_bridge.clone();
    app_tbl.set(
        "quit",
        lua.create_function(move |_lua, ()| {
            bridge_quit.push_action(OsAction::QuitApp);
            Ok(())
        })?,
    )?;
    os_tbl.set("app", app_tbl)?;

    lua.globals().set("os_platform", os_tbl.clone())?;
    // Also extend standard Lua os table or expose as `system` / `os_platform`
    if let Ok(std_os) = lua.globals().get::<mlua::Table>("os") {
        for pair in os_tbl.pairs::<String, Value>() {
            if let Ok((k, v)) = pair {
                let _ = std_os.set(k.as_str(), v);
            }
        }
    }

    Ok(())
}
