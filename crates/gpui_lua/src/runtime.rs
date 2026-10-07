use crate::assets::EmbeddedAssetManager;
use crate::backend::BackendBridge;
use crate::dsl::node::SyncRegistryKey;
use crate::dsl::{self, parse_div_table, LuaElementBuilder, LuaInvoker, LuaNode};
use crate::hot_reload::{self, render_error_view, HotReloadError, ScriptWatcher};
#[cfg(feature = "net")]
use crate::net::{self, HttpEngine};
use crate::reactive::{self, ReactiveBridge, ReactiveStore};
use crate::stdlib::{self, OsBridge, TimerEngine};
#[cfg(feature = "media")]
use media::{AudioManager, AudioPlayer, VideoManager, VideoPlayer};
#[cfg(feature = "net")]
use gpui::http_client::{HttpClient, HttpResponse as GpuiHttpResponse};
#[cfg(feature = "net")]
use std::future::Future;
#[cfg(feature = "net")]
use futures::future::BoxFuture;
use gpui::{
    div, px, rgb, rgba, svg, white, Context,
    Hsla, InteractiveElement, IntoElement, ParentElement, Render,
    StatefulInteractiveElement, Styled, Task, Window, WindowControlArea,
};
use mlua::{Function, Lua, Value};
use parking_lot::{Mutex, RwLock};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
pub struct LuaRuntime {
    lua: Arc<Mutex<Lua>>,
    store: ReactiveStore,
    bridge: ReactiveBridge,
    os_bridge: OsBridge,
    backend_bridge: BackendBridge,
    assets: EmbeddedAssetManager,
#[cfg(feature = "media")]
    video: VideoManager,
#[cfg(feature = "media")]
    audio: AudioManager,
#[cfg(feature = "net")]
    _http_engine: HttpEngine,
    _timer_engine: TimerEngine,
    error_state: Arc<RwLock<Option<HotReloadError>>>,
    script_path: Arc<RwLock<Option<PathBuf>>>,
    watcher: Arc<RwLock<Option<ScriptWatcher>>>,
    csd: Arc<AtomicBool>,
    csd_options: Arc<RwLock<CsdOptions>>,
    title: Arc<RwLock<String>>,
    async_engine: crate::stdlib::AsyncEngine,
    text_system: Arc<RwLock<Option<Arc<gpui::TextSystem>>>>,
    queued_fonts: Arc<RwLock<Vec<std::borrow::Cow<'static, [u8]>>>>,
    loaded_font_names: Arc<RwLock<Vec<String>>>,
    pub devtools: Arc<crate::devtools::DevToolsManager>,
}
fn init_luau_package_system(lua: &Lua) -> mlua::Result<()> {
    let pkg = lua.create_table();
    let loaded = lua.create_table();
    let loaders = lua.create_table();
    pkg.set("loaded", loaded)?;
    pkg.set("loaders", loaders)?;
    pkg.set("searchers", pkg.get::<mlua::Table>("loaders")?)?;
    lua.globals().set("package", pkg)?;

    #[cfg(feature = "jit")]
    {
        let _ = lua.enable_jit(true);
    }
    lua.set_compiler(ulua_rt::Compiler::new().set_optimization_level(2));

    lua.load(r#"
        function require(modname)
            local loaded = package.loaded
            if loaded[modname] ~= nil then
                return loaded[modname]
            end

            local errors = {}
            local loaders = package.loaders or package.searchers or {}
            for _, loader in ipairs(loaders) do
                local fn, err = loader(modname)
                if type(fn) == "function" then
                    local res = fn(modname)
                    if res ~= nil then
                        loaded[modname] = res
                    elseif loaded[modname] == nil then
                        loaded[modname] = true
                    end
                    return loaded[modname]
                elseif type(err) == "string" then
                    table.insert(errors, err)
                end
            end

            error("module '" .. tostring(modname) .. "' not found:" .. table.concat(errors, ""))
        end
    "#).exec()?;

    Ok(())
}

impl LuaRuntime {
    pub fn new() -> mlua::Result<Arc<Self>> {
        let lua = Lua::new();
        let store = ReactiveStore::new();
        let bridge = ReactiveBridge::new();
        let timer_engine = TimerEngine::new(bridge.clone());
        #[cfg(feature = "net")]
        let http_engine = HttpEngine::new(bridge.clone());
        let os_bridge = OsBridge::new(bridge.clone());
        let backend_bridge = BackendBridge::new(bridge.clone());
        let async_engine = crate::stdlib::AsyncEngine::new(bridge.clone(), os_bridge.clone());
        let assets = EmbeddedAssetManager::new();
        #[cfg(feature = "media")]
        let video_cb_bridge = bridge.clone();
        #[cfg(feature = "media")]
        let video = VideoManager::new(Some(Arc::new(move || {
            video_cb_bridge.notify();
        })));
        #[cfg(feature = "media")]
        let audio = AudioManager::new();
        let csd_options = Arc::new(RwLock::new(CsdOptions::default()));
        let csd = Arc::new(AtomicBool::new(false));
        let title = Arc::new(RwLock::new("GPUI-CE".to_string()));
        let text_system: Arc<RwLock<Option<Arc<gpui::TextSystem>>>> = Arc::new(RwLock::new(None));
        let queued_fonts = Arc::new(RwLock::new(Vec::new()));
        let loaded_font_names = Arc::new(RwLock::new(Vec::new()));
        let devtools = crate::devtools::DevToolsManager::new();
        let lua_arc = Arc::new(Mutex::new(lua));
        backend_bridge.set_lua(lua_arc.clone());
        async_engine.set_lua(lua_arc.clone());
        {
            let lua_guard = lua_arc.lock();
            init_luau_package_system(&lua_guard)?;
            dsl::register(&lua_guard)?;
            reactive::register(&lua_guard, store.clone(), bridge.clone())?;
        #[cfg(feature = "net")]
        net::register(
            &lua_guard,
            http_engine.clone().with_lua(lua_arc.clone()),
        )?;
            stdlib::register(
                &lua_guard,
                timer_engine.clone().with_lua(lua_arc.clone()),
                os_bridge.clone(),
                async_engine.clone(),
            )?;
            assets.register_lua_searcher(&lua_guard)?;

            // Register ui.load_font, ui.add_font, font.load
            let ts_for_load = text_system.clone();
            let qf_for_load = queued_fonts.clone();
            let fn_for_load = loaded_font_names.clone();
            let assets_for_load = assets.clone();

            let load_font_fn = lua_guard.create_function(move |_lua, src: String| {
                let bytes: std::borrow::Cow<'static, [u8]> = if let Some(b) = assets_for_load.get(&src) {
                    b
                } else {
                    let resolved = crate::runtime::resolve_script_path(Path::new(&src));
                    if resolved.exists() {
                        match std::fs::read(&resolved) {
                            Ok(b) => std::borrow::Cow::Owned(b),
                            Err(e) => return Err(mlua::Error::RuntimeError(format!("Failed to read font file '{}': {e}", resolved.display()))),
                        }
                    } else if let Ok(b) = std::fs::read(&src) {
                        std::borrow::Cow::Owned(b)
                    } else {
                        return Err(mlua::Error::RuntimeError(format!("Font asset not found: '{src}'")));
                    }
                };

                let family = crate::font::extract_font_family_name(&bytes)
                    .unwrap_or_else(|| {
                        Path::new(&src)
                            .file_stem()
                            .and_then(|s| s.to_str())
                            .unwrap_or("CustomFont")
                            .to_string()
                    });

                fn_for_load.write().push(family.clone());

                if let Some(ref ts) = *ts_for_load.read() {
                    if let Err(e) = ts.add_fonts(vec![bytes]) {
                        return Err(mlua::Error::RuntimeError(format!("Failed to register font: {e}")));
                    }
                } else {
                    qf_for_load.write().push(bytes);
                }

                Ok(family)
            })?;

            if let Ok(ui_tbl) = lua_guard.globals().get::<mlua::Table>("ui") {
                ui_tbl.set("load_font", load_font_fn.clone())?;
                ui_tbl.set("add_font", load_font_fn.clone())?;
            }

            let font_tbl = lua_guard.create_table();
            font_tbl.set("load", load_font_fn)?;
            lua_guard.globals().set("font", font_tbl)?;

            // Register DevTools Lua API
            let dt_open = devtools.clone();
            let dt_toggle = devtools.clone();
            let dt_close = devtools.clone();

            if let Ok(ui_tbl) = lua_guard.globals().get::<mlua::Table>("ui") {
                let dt_open_c = dt_open.clone();
                ui_tbl.set("open_devtools", lua_guard.create_function(move |_lua, ()| {
                    dt_open_c.is_open.store(true, std::sync::atomic::Ordering::SeqCst);
                    Ok(())
                })?)?;

                let dt_toggle_c = dt_toggle.clone();
                ui_tbl.set("toggle_devtools", lua_guard.create_function(move |_lua, ()| {
                    let cur = dt_toggle_c.is_open.load(std::sync::atomic::Ordering::SeqCst);
                    dt_toggle_c.is_open.store(!cur, std::sync::atomic::Ordering::SeqCst);
                    Ok(())
                })?)?;

                let dt_close_c = dt_close.clone();
                ui_tbl.set("close_devtools", lua_guard.create_function(move |_lua, ()| {
                    dt_close_c.is_open.store(false, std::sync::atomic::Ordering::SeqCst);
                    Ok(())
                })?)?;
            }

            let devtools_tbl = lua_guard.create_table();
            let dt_open_c = dt_open.clone();
            devtools_tbl.set("open", lua_guard.create_function(move |_lua, ()| {
                dt_open_c.is_open.store(true, std::sync::atomic::Ordering::SeqCst);
                Ok(())
            })?)?;
            let dt_toggle_c = dt_toggle.clone();
            devtools_tbl.set("toggle", lua_guard.create_function(move |_lua, ()| {
                let cur = dt_toggle_c.is_open.load(std::sync::atomic::Ordering::SeqCst);
                dt_toggle_c.is_open.store(!cur, std::sync::atomic::Ordering::SeqCst);
                Ok(())
            })?)?;
            let dt_close_c = dt_close.clone();
            devtools_tbl.set("close", lua_guard.create_function(move |_lua, ()| {
                dt_close_c.is_open.store(false, std::sync::atomic::Ordering::SeqCst);
                Ok(())
            })?)?;
            lua_guard.globals().set("devtools", devtools_tbl)?;

            // Hook print to DevTools console log
            let dt_log = devtools.clone();
            lua_guard.globals().set("print", lua_guard.create_function(move |_lua, args: mlua::MultiValue| {
                let mut parts = Vec::new();
                for arg in args.iter() {
                    match arg {
                        mlua::Value::String(s) => parts.push(s.to_str().unwrap_or_default().to_string()),
                        mlua::Value::Table(t) => {
                            if let Ok(json) = crate::stdlib::json::lua_value_to_json(mlua::Value::Table(t.clone())) {
                                parts.push(json.to_string());
                            } else {
                                parts.push(format!("{arg:?}"));
                            }
                        }
                        other => parts.push(format!("{other:?}")),
                    }
                }
                let msg = parts.join("\t");
                println!("{msg}");
                dt_log.state.log(crate::devtools::state::LogLevel::Info, &msg);
                Ok(())
            })?)?;
        {
            let _ = stdlib::register_media(&lua_guard, video.clone(), audio.clone());
            let audio_mgr_load = audio.clone();
            let audio_mgr_play = audio.clone();
            let audio_tbl = lua_guard.create_table();
            audio_tbl.set(
                "load",
                lua_guard.create_function(move |_lua, src: String| {
                    let resolved = crate::runtime::resolve_script_path(Path::new(&src));
                    let player = audio_mgr_load.load(&resolved);
                    Ok(LuaAudioPlayer(player))
                })?,
            )?;
            audio_tbl.set(
                "play",
                lua_guard.create_function(move |_lua, src: String| {
                    let resolved = crate::runtime::resolve_script_path(Path::new(&src));
                    let player = audio_mgr_play.play(&resolved);
                    Ok(LuaAudioPlayer(player))
                })?,
            )?;
            lua_guard.globals().set("audio", audio_tbl)?;

            let video_play = video.clone();
            lua_guard.globals().set(
                "__gpui_video_play",
                lua_guard.create_function(move |_lua, src: String| {
                    let resolved = crate::runtime::resolve_script_path(Path::new(&src));
                    video_play.get_or_create(&resolved, true).play();
                    Ok(())
                })?,
            )?;
            let video_pause = video.clone();
            lua_guard.globals().set(
                "__gpui_video_pause",
                lua_guard.create_function(move |_lua, src: String| {
                    let resolved = crate::runtime::resolve_script_path(Path::new(&src));
                    video_pause.get_or_create(&resolved, false).pause();
                    Ok(())
                })?,
            )?;
        }
        }
        Ok(Arc::new(Self {
            lua: lua_arc,
            store,
            bridge,
            os_bridge,
            backend_bridge,
            assets,
            #[cfg(feature = "media")]
            video,
            #[cfg(feature = "media")]
            audio,
            #[cfg(feature = "net")]
            _http_engine: http_engine,
            _timer_engine: timer_engine,
            error_state: Arc::new(RwLock::new(None)),
            script_path: Arc::new(RwLock::new(None)),
            watcher: Arc::new(RwLock::new(None)),
            csd,
            csd_options,
            title,
            async_engine,
            text_system,
            queued_fonts,
            loaded_font_names,
            devtools,
        }))
    }

    pub fn set_csd(&self, enabled: bool) {
        self.csd.store(enabled, Ordering::SeqCst);
    }

    pub fn is_csd(&self) -> bool {
        self.csd.load(Ordering::SeqCst)
    }

    pub fn set_csd_options(&self, options: CsdOptions) {
        *self.csd_options.write() = options;
    }

    pub fn csd_options(&self) -> CsdOptions {
        *self.csd_options.read()
    }

    pub fn update_csd_options<F: FnOnce(&mut CsdOptions)>(&self, f: F) {
        f(&mut self.csd_options.write());
    }

    pub fn set_title(&self, title: impl Into<String>) {
        *self.title.write() = title.into();
    }

    pub fn title(&self) -> String {
        self.title.read().clone()
    }

    pub fn bridge(&self) -> &ReactiveBridge {
        &self.bridge
    }

    pub fn lua(&self) -> Arc<Mutex<Lua>> {
        self.lua.clone()
    }

    /// Register a custom GPUI element available in Lua under `ui.<name>(props)`.
    pub fn register_element<F, E>(&self, name: impl Into<String>, renderer: F)
    where
        F: Fn(crate::dsl::CustomElementContext) -> E + Send + Sync + 'static,
        E: gpui::IntoElement,
    {
        let name = name.into();
        crate::dsl::register_custom_element(&name, Arc::new(move |cx| {
            renderer(cx).into_any_element()
        }));

        let lua = self.lua.lock();
        let _ = crate::dsl::bind_custom_element_in_lua(&lua, &name);
    }

    /// Sets or updates the GPUI text system and flushes any queued fonts immediately.
    pub fn init_text_system(&self, text_system: Arc<gpui::TextSystem>) {
        *self.text_system.write() = Some(text_system.clone());
        let queued = std::mem::take(&mut *self.queued_fonts.write());
        if !queued.is_empty() {
            let _ = text_system.add_fonts(queued);
        }
    }

    /// Returns all registered/loaded font family names.
    pub fn loaded_font_names(&self) -> Vec<String> {
        self.loaded_font_names.read().clone()
    }

    /// Load a font from embedded assets or from the filesystem by path.
    pub fn load_font(&self, src: &str) -> anyhow::Result<String> {
        let bytes: std::borrow::Cow<'static, [u8]> = if let Some(b) = self.assets.get(src) {
            b
        } else {
            let resolved = crate::runtime::resolve_script_path(Path::new(src));
            if resolved.exists() {
                std::borrow::Cow::Owned(std::fs::read(&resolved)?)
            } else if Path::new(src).exists() {
                std::borrow::Cow::Owned(std::fs::read(src)?)
            } else {
                anyhow::bail!("Font asset not found: '{src}'");
            }
        };

        self.load_font_bytes(bytes)
    }

    /// Load a font from a filesystem path.
    pub fn load_font_file(&self, path: impl AsRef<Path>) -> anyhow::Result<String> {
        let path = path.as_ref();
        let bytes = std::fs::read(path)?;
        self.load_font_bytes(std::borrow::Cow::Owned(bytes))
    }

    /// Load raw TrueType / OpenType font bytes.
    pub fn load_font_bytes(&self, bytes: impl Into<std::borrow::Cow<'static, [u8]>>) -> anyhow::Result<String> {
        let bytes = bytes.into();
        let family = crate::font::extract_font_family_name(&bytes)
            .unwrap_or_else(|| "CustomFont".to_string());

        self.loaded_font_names.write().push(family.clone());

        if let Some(ref ts) = *self.text_system.read() {
            ts.add_fonts(vec![bytes])?;
        } else {
            self.queued_fonts.write().push(bytes);
        }

        Ok(family)
    }

    pub fn os_bridge(&self) -> &OsBridge {
        &self.os_bridge
    }

    pub fn backend(&self) -> &BackendBridge {
        &self.backend_bridge
    }

    pub fn assets(&self) -> &EmbeddedAssetManager {
        &self.assets
    }

    #[cfg(feature = "media")]
    pub fn video(&self) -> &VideoManager {
        &self.video
    }

    #[cfg(feature = "media")]
    pub fn audio(&self) -> &AudioManager {
        &self.audio
    }

    pub fn async_engine(&self) -> &crate::stdlib::AsyncEngine {
        &self.async_engine
    }

    pub fn register_embedded_assets<E: rust_embed::RustEmbed + 'static>(&self) {
        self.assets.register_embed::<E>();
    }

    pub fn load_embedded(&self, embedded_path: &str) -> Result<(), HotReloadError> {
        let path = PathBuf::from(embedded_path);
        self.load_script(path)
    }

    pub fn register_fn<F, A, R>(&self, name: impl Into<String>, func: F)
    where
        F: Fn(A) -> mlua::Result<R> + Send + Sync + 'static,
        A: mlua::FromLuaMulti + 'static,
        R: mlua::IntoLua + 'static,
    {
        self.backend_bridge.register_fn(name, func);
    }

    pub fn register_json_fn<F, A, R>(&self, name: impl Into<String>, func: F)
    where
        F: Fn(A) -> std::result::Result<R, String> + Send + Sync + 'static,
        A: serde::de::DeserializeOwned + 'static,
        R: serde::Serialize + 'static,
    {
        self.backend_bridge.register_json_fn(name, func);
    }

    pub fn register_async_fn<F, Fut, A, R>(&self, name: impl Into<String>, func: F)
    where
        F: Fn(A) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = std::result::Result<R, String>> + Send + 'static,
        A: serde::de::DeserializeOwned + Send + 'static,
        R: serde::Serialize + Send + 'static,
    {
        self.backend_bridge.register_async_fn(name, func);
    }

    pub fn emit(&self, event: &str, data: impl serde::Serialize) -> std::result::Result<(), String> {
        self.backend_bridge.emit(event, data)
    }

    pub fn on<F>(&self, event: impl Into<String>, handler: F)
    where
        F: Fn(&serde_json::Value) + Send + Sync + 'static,
    {
        self.backend_bridge.on(event, handler);
    }

    pub fn store(&self) -> &ReactiveStore {
        &self.store
    }

    pub fn current_error(&self) -> Option<HotReloadError> {
        self.error_state.read().clone()
    }

    pub fn set_error(&self, err: HotReloadError) {
        *self.error_state.write() = Some(err);
        self.bridge.notify();
    }

    pub fn clear_error(&self) {
        *self.error_state.write() = None;
        self.bridge.notify();
    }

    pub fn load_script(&self, path: impl AsRef<Path>) -> Result<(), HotReloadError> {
        let p = path.as_ref().to_path_buf();
        *self.script_path.write() = Some(p.clone());

        if p.exists() {
            let res = {
                let lua = self.lua.lock();
                hot_reload::reload_lua_file(&lua, &p)
            };
            return match res {
                Ok(()) => {
                    *self.error_state.write() = None;
                    self.bridge.notify();
                    Ok(())
                }
                Err(e) => {
                    *self.error_state.write() = Some(e.clone());
                    self.bridge.notify();
                    Err(e)
                }
            };
        }

        let path_str = p.to_string_lossy();
        let filename = p.file_name().map(|f| f.to_string_lossy().to_string());
        let embedded_src = self.assets.get_str(&path_str)
            .or_else(|| filename.as_deref().and_then(|f| self.assets.get_str(f)));

        if let Some(src) = embedded_src {
            let lua = self.lua.lock();
            let chunk_name = format!("@embedded:{}", filename.as_deref().unwrap_or(&path_str));
            let res: mlua::Result<()> = lua.load(&src).set_name(&chunk_name).exec();
            return match res {
                Ok(()) => {
                    *self.error_state.write() = None;
                    self.bridge.notify();
                    Ok(())
                }
                Err(e) => {
                    let err = HotReloadError::parse(&e.to_string(), None);
                    *self.error_state.write() = Some(err.clone());
                    self.bridge.notify();
                    Err(err)
                }
            };
        }

        let err = HotReloadError::new(format!("Script not found on disk or embedded: {}", p.display()));
        *self.error_state.write() = Some(err.clone());
        self.bridge.notify();
        Err(err)
    }

    pub fn enable_hot_reload(self: &Arc<Self>) -> notify::Result<()> {
        let path = match self.script_path.read().clone() {
            Some(p) => p,
            None => return Ok(()),
        };

        let runtime_weak = Arc::downgrade(self);
        let watcher = ScriptWatcher::new(
            path,
            self.bridge.clone(),
            Arc::new(move |file_path| {
                if let Some(runtime) = runtime_weak.upgrade() {
                    runtime.load_script(file_path)
                } else {
                    Ok(())
                }
            }),
        )?;

        *self.watcher.write() = Some(watcher);
        Ok(())
    }

    pub fn render_node(&self) -> Result<LuaNode, HotReloadError> {
        let node_res = self.render_node_internal();
        if let Ok(ref node) = node_res {
            *self.devtools.state.element_tree.write() = Some(crate::devtools::state::ElementTreeNode::from_lua_node(node, vec![]));
        }
        node_res
    }

    fn render_node_internal(&self) -> Result<LuaNode, HotReloadError> {
        let lua = self.lua.lock();

        // 1. Try globals.App()
        if let Ok(app_fn) = lua.globals().get::<Function>("App") {
            return call_render_fn(&lua, app_fn);
        }

        // 2. Try globals.render()
        if let Ok(render_fn) = lua.globals().get::<Function>("render") {
            return call_render_fn(&lua, render_fn);
        }

        // 3. Try __gpui_last_result
        if let Ok(last_res) = lua.globals().get::<Value>("__gpui_last_result") {
            match last_res {
                Value::Function(f) => return call_render_fn(&lua, f),
                Value::UserData(ud) => {
                    if let Ok(b) = ud.borrow::<LuaElementBuilder>() {
                        return Ok(b.to_node());
                    }
                }
                Value::Table(t) => {
                    if let Ok(b) = parse_div_table(&lua, t) {
                        return Ok(b.to_node());
                    }
                }
                _ => {}
            }
        }

        Err(HotReloadError::new(
            "No root component found. Define function App() or render() in Lua, or return a ui element."
                .to_string(),
        ))
    }
}

fn call_render_fn(lua: &Lua, func: Function) -> Result<LuaNode, HotReloadError> {
    let res: mlua::Result<Value> = lua.load(r#"
        local fn = ...
        local ok, res_or_err = xpcall(fn, function(e)
            return tostring(e) .. "\n" .. (debug.traceback and debug.traceback() or "")
        end)
        if not ok then
            error(res_or_err)
        end
        return res_or_err
    "#).call(func);
    match res {
        Ok(Value::UserData(ud)) => {
            if let Ok(builder) = ud.borrow::<LuaElementBuilder>() {
                Ok(builder.to_node())
            } else {
                Err(HotReloadError::new(
                    "Root component returned unrecognized userdata",
                ))
            }
        }
        Ok(Value::Table(t)) => {
            let builder = parse_div_table(lua, t)
                .map_err(|e| HotReloadError::new(format!("Failed to parse root table: {e}")))?;
            Ok(builder.to_node())
        }
        Ok(Value::String(s)) => {
            let content = s.to_str().map(|v| v.to_string()).unwrap_or_default();
            Ok(LuaNode::Text(crate::dsl::TextNode {
                content,
                ..Default::default()
            }))
        }
        Ok(val) => Err(HotReloadError::new(format!(
            "Root component returned unexpected type: {}",
            val.type_name()
        ))),
        Err(e) => Err(HotReloadError::new(format!("Error calling render: {e}"))),
    }
}

impl LuaInvoker for LuaRuntime {
    fn call_key(&self, key: &SyncRegistryKey) -> Result<(), mlua::Error> {
        let lua = self.lua.lock();
        let func = key.with(|k| lua.registry_value::<Function>(k))?;
        func.call::<()>(())?;
        self.bridge.notify();
        Ok(())
    }

    fn call_key_bool(&self, key: &SyncRegistryKey, arg: bool) -> Result<(), mlua::Error> {
        let lua = self.lua.lock();
        let func = key.with(|k| lua.registry_value::<Function>(k))?;
        func.call::<()>(arg)?;
        self.bridge.notify();
        Ok(())
    }

    fn call_key_bounds(
        &self,
        key: &SyncRegistryKey,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
    ) -> Result<(), mlua::Error> {
        let lua = self.lua.lock();
        let func = key.with(|k| lua.registry_value::<Function>(k))?;
        let bounds_tbl = lua.create_table();
        bounds_tbl.set("x", x)?;
        bounds_tbl.set("y", y)?;
        bounds_tbl.set("w", w)?;
        bounds_tbl.set("h", h)?;
        func.call::<()>(bounds_tbl)?;
        Ok(())
    }

    fn call_key_paths(
        &self,
        key: &SyncRegistryKey,
        paths: Vec<String>,
    ) -> Result<(), mlua::Error> {
        let lua = self.lua.lock();
        let func = key.with(|k| lua.registry_value::<Function>(k))?;
        let paths_tbl = lua.create_table();
        for (i, p) in paths.into_iter().enumerate() {
            paths_tbl.set(i + 1, p)?;
        }
        func.call::<()>(paths_tbl)?;
        self.bridge.notify();
        Ok(())
    }

    fn call_key_mouse(
        &self,
        key: &SyncRegistryKey,
        x: f32,
        y: f32,
        button: &str,
    ) -> Result<(), mlua::Error> {
        let lua = self.lua.lock();
        let func = key.with(|k| lua.registry_value::<Function>(k))?;
        let event_tbl = lua.create_table();
        let _ = event_tbl.set("x", x);
        let _ = event_tbl.set("y", y);
        let _ = event_tbl.set("button", button);
        func.call::<()>(event_tbl)?;
        self.bridge.notify();
        Ok(())
    }

    fn call_key_key(
        &self,
        key: &SyncRegistryKey,
        key_str: &str,
        key_char: Option<&str>,
        ctrl: bool,
        alt: bool,
        shift: bool,
        meta: bool,
    ) -> Result<(), mlua::Error> {
        let lua = self.lua.lock();
        let func = key.with(|k| lua.registry_value::<Function>(k))?;
        let event_tbl = lua.create_table();
        let _ = event_tbl.set("key", key_str);
        if let Some(ch) = key_char {
            let _ = event_tbl.set("char", ch);
        }
        let _ = event_tbl.set("ctrl", ctrl);
        let _ = event_tbl.set("alt", alt);
        let _ = event_tbl.set("shift", shift);
        let _ = event_tbl.set("meta", meta);
        func.call::<()>(event_tbl)?;
        self.bridge.notify();
        Ok(())
    }

    fn call_key_scroll(
        &self,
        key: &SyncRegistryKey,
        dx: f32,
        dy: f32,
    ) -> Result<(), mlua::Error> {
        let lua = self.lua.lock();
        let func = key.with(|k| lua.registry_value::<Function>(k))?;
        let event_tbl = lua.create_table();
        let _ = event_tbl.set("dx", dx);
        let _ = event_tbl.set("dy", dy);
        func.call::<()>(event_tbl)?;
        self.bridge.notify();
        Ok(())
    }

    fn call_key_input(&self, key: &SyncRegistryKey, val: &str) -> Result<(), mlua::Error> {
        let lua = self.lua.lock();
        let func = key.with(|k| lua.registry_value::<Function>(k))?;
        func.call::<()>(val)?;
        self.bridge.notify();
        Ok(())
    }
    fn get_asset(&self, path: &str) -> Option<std::borrow::Cow<'static, [u8]>> {
        self.assets.get(path)
    }

    #[cfg(feature = "media")]
    fn get_video_player(&self, src: &str, autoplay: bool) -> Option<Arc<VideoPlayer>> {
        if src.starts_with("webrtc://") || src.starts_with("camera://") || src.starts_with("media://") {
            let path = PathBuf::from(src);
            if let Some(p) = self.video.get_player(&path) {
                return Some(p);
            }
            return Some(self.video.register_stream_player(src));
        }
        let resolved = crate::runtime::resolve_script_path(Path::new(src));
        Some(self.video.get_or_create(&resolved, autoplay))
    }

    #[cfg(feature = "media")]
    fn get_audio_player(&self, src: &str) -> Option<Arc<media::AudioPlayer>> {
        if src.starts_with("webrtc://") || src.starts_with("microphone://") || src.starts_with("media://") {
            let path = PathBuf::from(src);
            if let Some(p) = self.audio.get_player(&path) {
                return Some(p);
            }
            return Some(self.audio.register_stream_player(src));
        }
        let resolved = crate::runtime::resolve_script_path(Path::new(src));
        Some(self.audio.load(&resolved))
    }
}
pub struct LuaView {
    pub runtime: Arc<LuaRuntime>,
    _notification_task: Option<Task<()>>,
    focus_handle: gpui::FocusHandle,
}

impl LuaView {
    pub fn new(runtime: Arc<LuaRuntime>, cx: &mut Context<Self>) -> Self {
        runtime.init_text_system(cx.text_system().clone());
        let (tx, rx) = async_channel::unbounded::<()>();
        runtime.bridge().set_sender(tx);
        let task = cx.spawn(async move |weak_view, async_app| {
            while let Ok(()) = rx.recv().await {
                // Coalesce queued frame / signal notifications to prevent lagging
                while let Ok(()) = rx.try_recv() {}
                let _ = weak_view.update(async_app, |_view, cx| {
                    cx.notify();
                });
            }
        });

        let focus_handle = cx.focus_handle();

        Self {
            runtime,
            _notification_task: Some(task),
            focus_handle,
        }
    }
}

const MINUS_SVG: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none"><path d="M20 12L4 12" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5"/></svg>"#;
const SQUARE_SVG: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none"><rect x="4.5" y="4.5" width="15" height="15" rx="1.5" stroke="currentColor" stroke-width="1.5"/></svg>"#;
const CLOSE_SVG: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none"><path d="M18 6L6 18M18 18L6 6" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5"/></svg>"#;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CsdOptions {
    pub height: f32,
    pub button_width: f32,
    pub icon_color: Hsla,
    pub hover_bg: Hsla,
    pub active_bg: Hsla,
    pub close_hover_bg: Hsla,
    pub close_active_bg: Hsla,
    pub close_hover_color: Hsla,
}

pub trait IntoHsla {
    fn into_hsla(self) -> Hsla;
}

impl IntoHsla for Hsla {
    fn into_hsla(self) -> Hsla {
        self
    }
}

impl IntoHsla for gpui::Rgba {
    fn into_hsla(self) -> Hsla {
        gpui::rgb_to_hsla(self)
    }
}

impl Default for CsdOptions {
    fn default() -> Self {
        Self {
            height: 38.0,
            button_width: 46.0,
            icon_color: gpui::rgb_to_hsla(rgb(0xcdd6f4)),
            hover_bg: gpui::rgb_to_hsla(rgba(0xffffff14)),
            active_bg: gpui::rgb_to_hsla(rgba(0xffffff28)),
            close_hover_bg: gpui::rgb_to_hsla(rgb(0xe81123)),
            close_active_bg: gpui::rgb_to_hsla(rgb(0xdc0a1c)),
            close_hover_color: white(),
        }
    }
}

impl CsdOptions {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn height(mut self, height: f32) -> Self {
        self.height = height;
        self
    }

    pub fn button_width(mut self, width: f32) -> Self {
        self.button_width = width;
        self
    }

    pub fn icon_color(mut self, color: impl IntoHsla) -> Self {
        self.icon_color = color.into_hsla();
        self
    }

    pub fn hover_bg(mut self, bg: impl IntoHsla) -> Self {
        self.hover_bg = bg.into_hsla();
        self
    }

    pub fn active_bg(mut self, bg: impl IntoHsla) -> Self {
        self.active_bg = bg.into_hsla();
        self
    }

    pub fn close_hover_bg(mut self, bg: impl IntoHsla) -> Self {
        self.close_hover_bg = bg.into_hsla();
        self
    }

    pub fn close_active_bg(mut self, bg: impl IntoHsla) -> Self {
        self.close_active_bg = bg.into_hsla();
        self
    }

    pub fn close_hover_color(mut self, color: impl IntoHsla) -> Self {
        self.close_hover_color = color.into_hsla();
        self
    }
}

fn has_window_controls(node: &crate::dsl::LuaNode) -> bool {
    match node {
        crate::dsl::LuaNode::Div(div) => {
            if div.window_control.is_some() {
                return true;
            }
            div.children.iter().any(has_window_controls)
        }
        _ => false,
    }
}

fn render_frame_titlebar(options: &CsdOptions) -> gpui::Div {
    let is_macos = cfg!(target_os = "macos");

    let mut drag_area = div()
        .flex()
        .items_center()
        .flex_1()
        .h_full()
        .window_control_area(WindowControlArea::Drag);

    if is_macos {
        drag_area = drag_area.child(div().w(px(68.0)).h_full());
    }
    let mut controls = div()
        .flex()
        .items_center()
        .h_full()
        .on_mouse_down(gpui::MouseButton::Left, |_, _, cx| cx.stop_propagation());

    let icon_color = options.icon_color;
    let hover_bg = options.hover_bg;
    let active_bg = options.active_bg;
    let close_hover_bg = options.close_hover_bg;
    let close_active_bg = options.close_active_bg;
    let close_hover_color = options.close_hover_color;
    let btn_width = options.button_width;

    if !is_macos {
        controls = controls
            .child(
                div()
                    .id("titlebar-windows-minimize")
                    .w(px(btn_width))
                    .h_full()
                    .flex()
                    .items_center()
                    .justify_center()
                    .hover(move |s| s.bg(hover_bg))
                    .active(move |s| s.bg(active_bg))
                    .window_control_area(WindowControlArea::Min)
                    .on_click(|_, window, cx| {
                        cx.stop_propagation();
                        window.minimize_window();
                    })
                    .child(
                        svg()
                            .data(MINUS_SVG.as_bytes())
                            .w(px(14.0))
                            .h(px(14.0))
                            .text_color(icon_color),
                    ),
            )
            .child(
                div()
                    .id("titlebar-windows-maximize")
                    .w(px(btn_width))
                    .h_full()
                    .flex()
                    .items_center()
                    .justify_center()
                    .hover(move |s| s.bg(hover_bg))
                    .active(move |s| s.bg(active_bg))
                    .window_control_area(WindowControlArea::Max)
                    .on_click(|_, window, cx| {
                        cx.stop_propagation();
                        window.zoom_window();
                    })
                    .child(
                        svg()
                            .data(SQUARE_SVG.as_bytes())
                            .w(px(12.0))
                            .h(px(12.0))
                            .text_color(icon_color),
                    ),
            )
            .child(
                div()
                    .id("titlebar-windows-close")
                    .w(px(btn_width))
                    .h_full()
                    .flex()
                    .items_center()
                    .justify_center()
                    .hover(move |s| s.bg(close_hover_bg).text_color(close_hover_color))
                    .active(move |s| s.bg(close_active_bg).text_color(close_hover_color))
                    .window_control_area(WindowControlArea::Close)
                    .on_click(|_, window, cx| {
                        cx.stop_propagation();
                        window.remove_window();
                    })
                    .child(
                        svg()
                            .data(CLOSE_SVG.as_bytes())
                            .w(px(14.0))
                            .h(px(14.0))
                            .text_color(icon_color),
                    ),
            );
    }

    div()
        .absolute()
        .top_0()
        .left_0()
        .right_0()
        .h(px(options.height))
        .flex()
        .items_center()
        .justify_between()
        .window_control_area(WindowControlArea::Drag)
        .child(drag_area)
        .child(controls)
}

impl Render for LuaView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if !self.focus_handle.is_focused(window) && window.focused(cx).is_none() {
            window.focus(&self.focus_handle, cx);
        }

        // 1. Sync window metrics
        let size = window.viewport_size();
        self.runtime.os_bridge().set_window_info(crate::stdlib::WindowInfo {
            is_fullscreen: window.is_fullscreen(),
            is_maximized: window.is_maximized(),
            x: 0.0,
            y: 0.0,
            width: f32::from(size.width),
            height: f32::from(size.height),
            title: String::new(),
            is_focused: true,
        });

        // 2. Read clipboard cache
        if let Some(text) = cx.read_from_clipboard().and_then(|item| item.text()) {
            self.runtime.os_bridge().set_clipboard_cache(text);
        }

        // 3. Drain and execute pending OS actions
        for action in self.runtime.os_bridge().drain_actions() {
            match action {
                crate::stdlib::OsAction::WriteClipboard(text) => {
                    cx.write_to_clipboard(gpui::ClipboardItem::new_string(text));
                }
                crate::stdlib::OsAction::OpenUrl(url) => {
                    cx.open_url(&url);
                }
                crate::stdlib::OsAction::SetWindowTitle(title) => {
                    window.set_window_title(&title);
                }
                crate::stdlib::OsAction::MinimizeWindow => {
                    window.minimize_window();
                }
                crate::stdlib::OsAction::ZoomWindow => {
                    window.zoom_window();
                }
                crate::stdlib::OsAction::ToggleFullscreen => {
                    window.toggle_fullscreen();
                }
                crate::stdlib::OsAction::CloseWindow => {
                    window.remove_window();
                }
                crate::stdlib::OsAction::StartWindowMove => {
                    window.start_window_move();
                }
                crate::stdlib::OsAction::QuitApp => {
                    cx.quit();
                }
                crate::stdlib::OsAction::SetWindowSize(w, h) => {
                    window.resize(gpui::size(gpui::px(w), gpui::px(h)));
                }
                crate::stdlib::OsAction::SetWindowPosition(_x, _y) => {}
                crate::stdlib::OsAction::CenterWindow => {}
                crate::stdlib::OsAction::FocusWindow => {
                    window.activate();
                }
                crate::stdlib::OsAction::MaximizeWindow => {
                    window.zoom_window();
                }
                crate::stdlib::OsAction::RestoreWindow => {
                    window.zoom_window();
                }
                crate::stdlib::OsAction::SetWindowAlwaysOnTop(_) => {}
                crate::stdlib::OsAction::SetWindowResizable(_) => {}
                crate::stdlib::OsAction::SetWindowDecorations(_) => {}
                crate::stdlib::OsAction::OpenFilePrompt {
                    files,
                    directories,
                    multiple,
                    prompt,
                    callback_id,
                } => {
                    let prompt_rx = cx.prompt_for_paths(gpui::PathPromptOptions {
                        files,
                        directories,
                        multiple,
                        prompt: prompt.map(gpui::SharedString::from),
                    });
                    let eng = self.runtime.async_engine().clone();
                    crate::tokio_runtime().spawn(async move {
                        let res = prompt_rx.await;
                        match res {
                            Ok(Ok(Some(paths))) => {
                                let strs = paths.into_iter().map(|p| p.to_string_lossy().to_string()).collect();
                                eng.trigger_dialog_callback(callback_id, None, Some(strs));
                            }
                            Ok(Ok(None)) => {
                                eng.trigger_dialog_callback(callback_id, None, None);
                            }
                            Ok(Err(e)) => {
                                eng.trigger_dialog_callback(callback_id, Some(e.to_string()), None);
                            }
                            Err(e) => {
                                eng.trigger_dialog_callback(callback_id, Some(e.to_string()), None);
                            }
                        }
                    });
                }
                crate::stdlib::OsAction::SaveFilePrompt {
                    directory,
                    suggested_name,
                    callback_id,
                } => {
                    let prompt_rx = cx.prompt_for_new_path(&directory, suggested_name.as_deref());
                    let eng = self.runtime.async_engine().clone();
                    crate::tokio_runtime().spawn(async move {
                        let res = prompt_rx.await;
                        match res {
                            Ok(Ok(Some(path))) => {
                                eng.trigger_dialog_callback(callback_id, None, Some(vec![path.to_string_lossy().to_string()]));
                            }
                            Ok(Ok(None)) => {
                                eng.trigger_dialog_callback(callback_id, None, None);
                            }
                            Ok(Err(e)) => {
                                eng.trigger_dialog_callback(callback_id, Some(e.to_string()), None);
                            }
                            Err(e) => {
                                eng.trigger_dialog_callback(callback_id, Some(e.to_string()), None);
                            }
                        }
                    });
                }
                crate::stdlib::OsAction::MessagePrompt {
                    level,
                    message,
                    detail,
                    callback_id,
                } => {
                    let plevel = match level.to_lowercase().as_str() {
                        "warning" => gpui::PromptLevel::Warning,
                        "critical" | "error" => gpui::PromptLevel::Critical,
                        _ => gpui::PromptLevel::Info,
                    };
                    let prompt_rx = window.prompt(plevel, &message, detail.as_deref(), &["OK", "Cancel"], cx);
                    let eng = self.runtime.async_engine().clone();
                    crate::tokio_runtime().spawn(async move {
                        let res = prompt_rx.await;
                        match res {
                            Ok(idx) => {
                                let ans = if idx == 0 { "ok" } else { "cancel" };
                                eng.trigger_dialog_callback(callback_id, None, Some(vec![ans.to_string()]));
                            }
                            Err(e) => {
                                eng.trigger_dialog_callback(callback_id, Some(e.to_string()), None);
                            }
                        }
                    });
                }
            }
        }

        // Collect idle video players to release resources
        #[cfg(feature = "media")]
        {
            self.runtime.video().collect_idle();
            self.runtime.audio().collect_idle();
        }

        self.runtime.init_text_system(cx.text_system().clone());

        if let Some(err) = self.runtime.current_error() {
            return render_error_view(&err);
        }

        // Automatically open detached DevTools window if requested
        if self.runtime.devtools.is_open.load(std::sync::atomic::Ordering::SeqCst)
            && self.runtime.devtools.window_handle.read().is_none()
        {
            self.runtime.devtools.open_window(self.runtime.clone(), cx);
        }

        match self.runtime.render_node() {
            Ok(node) => {
                let has_controls = has_window_controls(&node);
                let content = dsl::convert_node(node, Some(self.runtime.clone()));

                let eng_hk = self.runtime.async_engine().clone();
                let dt_hk = self.runtime.devtools.clone();
                let rt_hk = self.runtime.clone();

                let is_inspect = self.runtime.devtools.state.inspect_cursor_active.load(std::sync::atomic::Ordering::Relaxed);

                let mut root_container = div()
                    .id("gpui_lua_root")
                    .track_focus(&self.focus_handle)
                    .size_full()
                    .relative()
                    .on_key_down(move |e, _window, cx| {
                        let k = e.keystroke.key.as_str();
                        let m = &e.keystroke.modifiers;

                        // F12 or Ctrl+Shift+I or Cmd+Opt+I: Toggle DevTools
                        if k == "f12"
                            || (m.control && m.shift && k.eq_ignore_ascii_case("i"))
                            || (m.platform && m.alt && k.eq_ignore_ascii_case("i"))
                        {
                            dt_hk.toggle_window(rt_hk.clone(), cx);
                            cx.stop_propagation();
                            return;
                        }

                        // Ctrl+Shift+C or Cmd+Shift+C: Toggle Inspect Cursor
                        if (m.control && m.shift && k.eq_ignore_ascii_case("c"))
                            || (m.platform && m.shift && k.eq_ignore_ascii_case("c"))
                        {
                            dt_hk.toggle_inspect_mode();
                            rt_hk.bridge().notify();
                            cx.stop_propagation();
                            return;
                        }

                        let mut parts = Vec::new();
                        if m.control { parts.push("ctrl"); }
                        if m.alt { parts.push("alt"); }
                        if m.shift { parts.push("shift"); }
                        if m.platform { parts.push("meta"); }
                        parts.push(&e.keystroke.key);
                        let chord = parts.join("+").to_lowercase();
                        if eng_hk.trigger_hotkey(&chord) {
                            cx.stop_propagation();
                        }
                    });

                if is_inspect {
                    root_container = root_container.cursor(gpui::CursorStyle::Crosshair);
                }

                let dt_move = self.runtime.devtools.clone();
                let bridge_move = self.runtime.bridge().clone();
                root_container = root_container.on_mouse_move(move |e, _window, _cx| {
                    if dt_move.state.inspect_cursor_active.load(std::sync::atomic::Ordering::Relaxed) {
                        let hit = dt_move.hit_test_inspect(e.position.x, e.position.y);
                        *dt_move.state.hovered_path.write() = hit;
                        bridge_move.notify();
                    }
                });

                let dt_click = self.runtime.devtools.clone();
                let bridge_click = self.runtime.bridge().clone();
                root_container = root_container.on_mouse_down(gpui::MouseButton::Left, move |_e, _window, cx| {
                    if dt_click.state.inspect_cursor_active.load(std::sync::atomic::Ordering::Relaxed) {
                        let hovered = dt_click.state.hovered_path.read().clone();
                        *dt_click.state.selected_path.write() = hovered;
                        dt_click.state.inspect_cursor_active.store(false, std::sync::atomic::Ordering::Relaxed);
                        cx.stop_propagation();
                        bridge_click.notify();
                    }
                });

                root_container = root_container.child(content);

                // Attach Box Model Overlay on top of root container
                if self.runtime.devtools.is_open.load(std::sync::atomic::Ordering::Relaxed)
                    || self.runtime.devtools.state.inspect_cursor_active.load(std::sync::atomic::Ordering::Relaxed)
                {
                    root_container = root_container.child(crate::devtools::render_devtools_overlay(self.runtime.devtools.state.clone()));
                }
                if self.runtime.is_csd() && !has_controls {
                    div()
                        .size_full()
                        .relative()
                        .child(root_container)
                        .child(render_frame_titlebar(&self.runtime.csd_options()))
                        .into_any_element()
                } else {
                    root_container.into_any_element()
                }
            }
            Err(err) => render_error_view(&err),
        }
    }
}

#[cfg(feature = "media")]
#[derive(Clone)]
pub struct LuaAudioPlayer(pub Arc<AudioPlayer>);

#[cfg(feature = "media")]

impl mlua::UserData for LuaAudioPlayer {
    fn add_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("play", |_, this, ()| {
            this.0.play();
            Ok(())
        });
        methods.add_method("pause", |_, this, ()| {
            this.0.pause();
            Ok(())
        });
        methods.add_method("stop", |_, this, ()| {
            this.0.stop();
            Ok(())
        });
        methods.add_method("volume", |_, this, vol: f32| {
            this.0.set_volume(vol);
            Ok(())
        });
        methods.add_method("loop", |_, this, looping: Option<bool>| {
            this.0.set_loop(looping.unwrap_or(true));
            Ok(())
        });
        methods.add_method("seek", |_, this, secs: f64| {
            let _ = this.0.seek(secs);
            Ok(())
        });
        methods.add_method("duration", |_, this, ()| {
            Ok(this.0.duration_secs())
        });
        methods.add_method("position", |_, this, ()| {
            Ok(this.0.position_secs())
        });
        methods.add_method("is_playing", |_, this, ()| {
            Ok(this.0.is_playing())
        });
    }
}

#[cfg(feature = "net")]
pub struct ReqwestHttpClient {
    client: reqwest::Client,
}
#[cfg(feature = "net")]
impl ReqwestHttpClient {
    pub fn new() -> Self {
        let _guard = crate::tokio_runtime().enter();
        let client = reqwest::Client::builder()
            .pool_idle_timeout(std::time::Duration::from_secs(90))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());
        Self { client }
    }
}

#[cfg(feature = "net")]
impl HttpClient for ReqwestHttpClient {
    fn get(
        &self,
        url: &str,
        _follow_redirects: bool,
    ) -> BoxFuture<'static, anyhow::Result<GpuiHttpResponse>> {
        let client = self.client.clone();
        let url = url.to_string();
        Box::pin(async move {
            let handle = crate::tokio_runtime().spawn(async move {
                let resp = client.get(&url).send().await?;
                let status = http::StatusCode::from_u16(resp.status().as_u16())?;
                let body = resp.bytes().await?.to_vec();
                Ok::<_, anyhow::Error>(GpuiHttpResponse { status, body })
            });
            match handle.await {
                Ok(res) => res,
                Err(e) => Err(anyhow::anyhow!("Tokio task join error: {e}")),
            }
        })
    }
}

pub fn resolve_script_path(script_path: &Path) -> PathBuf {
    let path_str = script_path.to_string_lossy();
    if path_str.contains("://")
        || path_str.starts_with("http:")
        || path_str.starts_with("https:")
        || path_str.starts_with("rtsp:")
        || path_str.starts_with("rtmp:")
        || path_str.starts_with("udp:")
        || path_str.starts_with("tcp:")
    {
        return script_path.to_path_buf();
    }
    if script_path.exists() {
        return script_path.to_path_buf();
    }
    if let Ok(cwd) = std::env::current_dir() {
        let candidates = [
            cwd.join(script_path),
            cwd.join("crates/gpui_lua/examples").join(script_path),
            cwd.join("examples").join(script_path),
            cwd.join("crates/gpui_lua").join(script_path),
        ];
        for c in candidates {
            if c.exists() {
                return c;
            }
        }
        let mut curr = cwd.clone();
        for _ in 0..4 {
            let candidates = [
                curr.join(script_path),
                curr.join("crates/gpui_lua/examples").join(script_path),
                curr.join("examples").join(script_path),
                curr.join("crates/gpui_lua").join(script_path),
            ];
            for c in candidates {
                if c.exists() {
                    return c;
                }
            }
            if !curr.pop() {
                break;
            }
        }
    }
    script_path.to_path_buf()
}

#[cfg(test)]
mod test_conference {
    use super::*;

    #[test]
    fn test_counter_render() {
        let runtime = LuaRuntime::new().unwrap();
        let path = resolve_script_path(Path::new("counter.lua"));
        runtime.load_script(path).unwrap();
        let node = runtime.render_node();
        match node {
            Ok(_) => println!("Successfully rendered counter.lua!"),
            Err(e) => panic!("Error calling render on counter.lua: {e:?}"),
        }
    }
    #[test]
    fn test_input_and_textarea_behavior() {
        let runtime = LuaRuntime::new().unwrap();
        let lua = runtime.lua();
        let lua = lua.lock();
        lua.load(r#"
            -- Test 1: UTF-8 Masking with 3-byte bullet
            local pass_el = ui.input({
                id = "pass_test",
                value = "MySecret123!",
                type = "password",
                mask = "•"
            })
            assert(pass_el ~= nil, "password input failed")

            -- Test 2: Validation regex and error state
            local valid_called = false
            local email_el = ui.input({
                id = "email_test",
                value = "invalid-email",
                pattern = "^[%w_.-]+@[%w_.-]+%.%a+$",
                error_message = "Valid email required",
                on_validate = function(is_valid, err)
                    valid_called = true
                    assert(not is_valid, "should be invalid")
                    assert(err == "Valid email required", "error msg mismatch")
                end
            })
            assert(email_el ~= nil, "email input failed")
            assert(valid_called, "on_validate was not called")

            -- Test 3: Textarea multi-line
            local area_el = ui.textarea({
                id = "area_test",
                value = "Line 1: GPUI-CE\nLine 2: Multi-line text editor\nLine 3: Done",
                rows = 6
            })
            assert(area_el ~= nil, "textarea failed")

            -- Test 4: Textarea max_row and max_rows
            local area_max = ui.textarea({
                id = "area_max",
                rows = 2,
                max_row = 4
            })
            assert(area_max ~= nil, "textarea max_row failed")
        "#).exec().expect("input and textarea behavior test failed");
    }

    #[cfg(all(feature = "media", feature = "webrtc"))]
    #[test]
    fn test_camera_microphone_webrtc_stream_binding() {
        let runtime = LuaRuntime::new().unwrap();
        let lua = runtime.lua();
        let lua = lua.lock();
        lua.load(r#"
            -- 0. Enumerate devices
            local cams = media.list_cameras()
            assert(#cams >= 1, "at least one camera device must be listed")
            assert(cams[1].name ~= nil, "camera must have a name")

            local mics = media.list_microphones()
            assert(#mics >= 1, "at least one microphone device must be listed")
            assert(mics[1].name ~= nil, "microphone must have a name")

            -- 1. Open Camera and Microphone Capture
            assert(media ~= nil, "media module must exist")
            local cam = media.open_camera({ device = 0, width = 1280, height = 720, fps = 30 })
            assert(cam ~= nil, "camera capture must succeed")
            assert(cam:name() ~= nil, "camera must have a name")
            assert(cam:src() == "camera://0", "camera URI must be camera://0")
            assert(cam:width() == 1280, "camera width must be 1280")
            assert(cam:height() == 720, "camera height must be 720")
            assert(cam:fps() == 30, "camera fps must be 30")

            local mic = media.open_microphone({ sample_rate = 48000, channels = 2 })
            assert(mic ~= nil, "mic capture must succeed")
            assert(mic:src() == "microphone://0", "mic URI must be microphone://0")

            -- Test camera and mic muting
            cam:mute(true)
            assert(cam:is_muted() == true, "camera must be muted")
            cam:mute(false)
            assert(cam:is_muted() == false, "camera must be unmuted")

            mic:mute(true)
            assert(mic:is_muted() == true, "mic must be muted")
            mic:mute(false)
            assert(mic:is_muted() == false, "mic must be unmuted")

            -- 2. Bind to WebRTC Peer Connection
            assert(webrtc ~= nil, "webrtc module must exist")
            local peer = webrtc.create_peer_connection({
                ice_servers = {
                    { urls = "stun:stun.l.google.com:19302" }
                }
            })
            assert(peer ~= nil, "peer connection must succeed")

            local cam_track = peer:add_track(cam)
            assert(cam_track ~= nil, "cam track must be created")
            assert(cam_track:kind() == "video", "cam track kind must be video")
            assert(string.sub(cam_track:src(), 1, 9) == "webrtc://", "track src must start with webrtc://")

            local mic_track = peer:add_track(mic)
            assert(mic_track ~= nil, "mic track must be created")
            assert(mic_track:kind() == "audio", "mic track kind must be audio")

            -- 3. Test binding to ui.video and ui.audio elements
            local vid_el = ui.video({ src = cam, autoplay = true })
            assert(vid_el ~= nil, "ui.video with camera object must succeed")

            local vid_el_str = ui.video({ src = cam:src(), autoplay = true })
            assert(vid_el_str ~= nil, "ui.video with camera:src() must succeed")

            local vid_track_el = ui.video({ src = cam_track, autoplay = true })
            assert(vid_track_el ~= nil, "ui.video with local track must succeed")

            -- Clean up
            cam_track:stop()
            mic_track:stop()
            cam:stop()
            mic:stop()
            peer:close()
        "#).exec().expect("camera, microphone and webrtc stream binding test failed");
    }

    #[test]
    fn test_font_loading_lua_api() {
        let runtime = LuaRuntime::new().unwrap();

        let mut mock_ttf = Vec::new();
        mock_ttf.extend_from_slice(&[0x00, 0x01, 0x00, 0x00, 0, 1, 0, 0, 0, 0, 0, 0]);
        mock_ttf.extend_from_slice(b"name");
        mock_ttf.extend_from_slice(&[0, 0, 0, 0, 0, 0, 0, 28, 0, 0, 0, 100]);
        mock_ttf.extend_from_slice(&[0, 0, 0, 1, 0, 18]);
        mock_ttf.extend_from_slice(&[0, 3, 0, 1, 0x04, 0x09, 0, 1, 0, 10, 0, 0]);
        for c in "Inter".encode_utf16() {
            mock_ttf.extend_from_slice(&c.to_be_bytes());
        }

        let family = runtime.load_font_bytes(mock_ttf).unwrap();
        assert_eq!(family, "Inter");
        assert!(runtime.loaded_font_names().contains(&"Inter".to_string()));

        let lua = runtime.lua();
        let lua = lua.lock();
        lua.load(r#"
            assert(type(ui.load_font) == "function", "ui.load_font must exist")
            assert(type(font.load) == "function", "font.load must exist")
        "#).exec().expect("font API test failed");
    }

    #[test]
    fn test_devtools_state_and_lua_bindings() {
        let runtime = LuaRuntime::new().unwrap();

        let lua = runtime.lua();
        let lua = lua.lock();
        lua.load(r#"
            assert(type(ui.open_devtools) == "function", "ui.open_devtools must exist")
            assert(type(ui.toggle_devtools) == "function", "ui.toggle_devtools must exist")
            assert(type(devtools.open) == "function", "devtools.open must exist")
            assert(type(devtools.toggle) == "function", "devtools.toggle must exist")

            ui.open_devtools()
        "#).exec().expect("devtools lua binding failed");

        assert!(runtime.devtools.is_open.load(std::sync::atomic::Ordering::SeqCst));

        let req_id = runtime.devtools.state.record_http_request("GET", "https://api.example.com/test", std::collections::HashMap::new(), None);
        runtime.devtools.state.record_http_response(req_id, 200, "OK", std::collections::HashMap::new(), Some(r#"{"status":"ok"}"#.to_string()), 15);

        let entries = runtime.devtools.state.network_entries.read();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].url, "https://api.example.com/test");
        assert_eq!(entries[0].status, Some(200));

        runtime.devtools.state.log(crate::devtools::state::LogLevel::Info, "DevTools initialized successfully");
        let logs = runtime.devtools.state.console_entries.read();
        assert_eq!(logs.len(), 1);
        assert_eq!(logs[0].message, "DevTools initialized successfully");
    }

    #[test]
    fn test_devtools_edge_distinction_and_text_overlay() {
        use crate::dsl::node::{Edges, Length, LuaNode, DivNode, TextNode, StyleProps};
        use crate::devtools::state::{ElementTreeNode, BoxModelMetrics};
        use gpui::{Bounds, Point, Size, px};

        // 1. Text node tree conversion
        let text_node = LuaNode::Text(TextNode {
            content: "Hello GPUI".to_string(),
            size: Some(16.0),
            color: None,
            bold: true,
            italic: false,
            underline: false,
            line_through: false,
            line_height: None,
            font_family: None,
            letter_spacing: None,
            truncate: false,
            line_clamp: None,
        });
        let tree_text = ElementTreeNode::from_lua_node(&text_node, vec![0, 1]);
        assert_eq!(tree_text.tag, "text");
        assert_eq!(tree_text.style.font_size, Some(16.0));
        assert!(tree_text.attrs.iter().any(|(k, v)| k == "size" && v == "16"));
        assert!(tree_text.attrs.iter().any(|(k, v)| k == "bold" && v == "true"));

        // 2. Div with distinct padding edges: px=8, py=3, and margin edges: mx=5, mt=10, mb=20
        let mut style = StyleProps::default();
        style.padding = Edges {
            top: Some(Length::Px(3.0)),
            bottom: Some(Length::Px(3.0)),
            left: Some(Length::Px(8.0)),
            right: Some(Length::Px(8.0)),
        };
        style.margin = Edges {
            top: Some(Length::Px(10.0)),
            bottom: Some(Length::Px(20.0)),
            left: Some(Length::Px(5.0)),
            right: Some(Length::Px(5.0)),
        };
        let div_node = LuaNode::Div(DivNode {
            id: Some("box_el".to_string()),
            style: style.clone(),
            ..Default::default()
        });
        let tree_div = ElementTreeNode::from_lua_node(&div_node, vec![0]);
        let attr_map: std::collections::HashMap<_, _> = tree_div.attrs.into_iter().collect();
        assert_eq!(attr_map.get("px").map(|s| s.as_str()), Some("8"));
        assert_eq!(attr_map.get("py").map(|s| s.as_str()), Some("3"));
        assert_eq!(attr_map.get("mx").map(|s| s.as_str()), Some("5"));
        assert_eq!(attr_map.get("mt").map(|s| s.as_str()), Some("10"));
        assert_eq!(attr_map.get("mb").map(|s| s.as_str()), Some("20"));

        // 3. BoxModelMetrics
        let bounds = Bounds {
            origin: Point::new(px(10.0), px(10.0)),
            size: Size::new(px(200.0), px(100.0)),
        };
        let bm = BoxModelMetrics::from_style_and_bounds(&style, bounds);
        assert_eq!(bm.padding_top, 3.0);
        assert_eq!(bm.padding_bottom, 3.0);
        assert_eq!(bm.padding_left, 8.0);
        assert_eq!(bm.padding_right, 8.0);
        assert_eq!(bm.margin_top, 10.0);
        assert_eq!(bm.margin_bottom, 20.0);
        assert_eq!(bm.margin_left, 5.0);
        assert_eq!(bm.margin_right, 5.0);
        assert_eq!(bm.content_width, 200.0 - 16.0);
        assert_eq!(bm.content_height, 100.0 - 6.0);
    }
}
