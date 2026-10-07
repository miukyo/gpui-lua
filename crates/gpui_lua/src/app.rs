use crate::backend::BackendBridge;
use crate::reactive::store::StoredValue;
#[cfg(feature = "net")]
use crate::runtime::ReqwestHttpClient;
use crate::runtime::{resolve_script_path, CsdOptions, LuaRuntime, LuaView};
use gpui::{
    App, AppContext, Bounds, DisplayId, Pixels, SharedString, Size, TitlebarOptions,
    WindowBounds, WindowDecorations, WindowKind, WindowOptions, px, size,
};
use mlua::{FromLuaMulti, IntoLua};
use std::future::Future;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

/// High-level application runner establishing Rust as backend and Lua as frontend.
pub struct LuaApp {
    script_path: PathBuf,
    pub window_options: WindowOptions,
    pub csd_options: CsdOptions,
    default_size: Option<Size<Pixels>>,
    hot_reload: bool,
    runtime: Arc<LuaRuntime>,
}

impl LuaApp {
    /// Initialize a new application with the given Lua script entrypoint.
    pub fn new(script_path: impl AsRef<Path>) -> Self {
        Self::try_new(script_path).expect("Failed to initialize Lua runtime")
    }

    /// Initialize a new application whose frontend scripts and assets are embedded in binary.
    pub fn embedded<E: rust_embed::RustEmbed + 'static>(entry_path: impl Into<String>) -> Self {
        Self::try_embedded::<E>(entry_path).expect("Failed to initialize Lua runtime")
    }

    /// Try to initialize a new application with the given Lua script entrypoint.
    pub fn try_new(script_path: impl AsRef<Path>) -> anyhow::Result<Self> {
        let runtime = LuaRuntime::new()
            .map_err(|e| anyhow::anyhow!("Failed to initialize Lua runtime: {e}"))?;

        let mut window_options = WindowOptions::default();
        window_options.titlebar = Some(TitlebarOptions {
            title: Some(SharedString::new_static("GPUI-CE Lua")),
            ..Default::default()
        });
        runtime.set_title("GPUI-CE Lua");

        let csd_options = CsdOptions::default();

        Ok(Self {
            script_path: script_path.as_ref().to_path_buf(),
            window_options,
            csd_options,
            default_size: Some(size(px(800.0), px(600.0))),
            hot_reload: cfg!(debug_assertions),
            runtime,
        })
    }

    /// Try to initialize an application whose frontend scripts and assets are embedded in binary.
    pub fn try_embedded<E: rust_embed::RustEmbed + 'static>(entry_path: impl Into<String>) -> anyhow::Result<Self> {
        let entry = entry_path.into();
        let app = Self::try_new(&entry)?;
        app.runtime.register_embedded_assets::<E>();
        Ok(app)
    }

    /// Initialize application from a standalone bundled package manifest.
    pub fn from_bundled_manifest(manifest: crate::bundle::AppBundleManifest) -> anyhow::Result<Self> {
        let entry = manifest.entrypoint.clone();
        let app = Self::try_new(&entry)?;
        app.runtime.assets().register_file_map(manifest.files);

        let mut app = app;
        if let Some(title) = manifest.title {
            app = app.title(title);
        }
        if let (Some(w), Some(h)) = (manifest.width, manifest.height) {
            app = app.size(w, h);
        }
        if let (Some(mw), Some(mh)) = (manifest.min_width, manifest.min_height) {
            app = app.min_size(mw, mh);
        }
        if let Some(r) = manifest.resizable {
            app = app.resizable(r);
        }
        if let Some(m) = manifest.minimizable {
            app = app.minimizable(m);
        }
        if manifest.csd.unwrap_or(true) {
            app = app.csd();
        }
        if let Some(h) = manifest.csd_height {
            let mut opts = app.csd_options;
            opts.height = h;
            app = app.csd_options(opts);
        }
        #[cfg(target_os = "windows")]
        {
            let bg_str = manifest.windows_background.as_deref().or(manifest.background.as_deref());
            if let Some(bg) = bg_str {
                match bg.to_lowercase().trim() {
                    "acrylic" => { app = app.windows_background(gpui::WindowsWindowBackground::Acrylic); }
                    "mica" => { app = app.windows_background(gpui::WindowsWindowBackground::MicaBackdrop); }
                    "mica_alt" | "mica-alt" | "tabbed" => { app = app.windows_background(gpui::WindowsWindowBackground::MicaAltBackdrop); }
                    "opaque" => { app = app.windows_background(gpui::WindowsWindowBackground::Opaque); }
                    "transparent" => { app = app.windows_background(gpui::WindowsWindowBackground::Transparent); }
                    "blurred" => { app = app.windows_background(gpui::WindowsWindowBackground::Blurred); }
                    _ => {}
                }
            }
        }
        #[cfg(target_os = "macos")]
        {
            let bg_str = manifest.macos_background.as_deref().or(manifest.background.as_deref());
            if let Some(bg) = bg_str {
                app = app.set_macos_background_str(bg);
            }
        }
        #[cfg(any(target_os = "linux", target_os = "freebsd"))]
        {
            let bg_str = manifest.linux_background.as_deref().or(manifest.background.as_deref());
            if let Some(bg) = bg_str {
                match bg.to_lowercase().trim() {
                    "transparent" => { app = app.linux_background(gpui::LinuxWindowBackground::Transparent); }
                    "opaque" => { app = app.linux_background(gpui::LinuxWindowBackground::Opaque); }
                    "blurred" => { app = app.linux_background(gpui::LinuxWindowBackground::Blurred); }
                    _ => {}
                }
            }
        }
        app.hot_reload = false;
        Ok(app)
    }
    /// Register a `RustEmbed` asset bundle with this application.
    pub fn with_embedded_assets<E: rust_embed::RustEmbed + 'static>(self) -> Self {
        self.runtime.register_embedded_assets::<E>();
        self
    }

    /// Replace the window options with a custom configuration.
    pub fn window_options(mut self, options: WindowOptions) -> Self {
        self.window_options = options;
        self
    }

    /// Mutate the current window options in place.
    pub fn with_window_options<F: FnOnce(&mut WindowOptions)>(mut self, f: F) -> Self {
        f(&mut self.window_options);
        self
    }

    /// Set window title.
    pub fn title(mut self, title: impl Into<SharedString>) -> Self {
        let title = title.into();
        self.runtime.set_title(title.to_string());
        if let Some(titlebar) = &mut self.window_options.titlebar {
            titlebar.title = Some(title);
        } else {
            self.window_options.titlebar = Some(TitlebarOptions {
                title: Some(title),
                ..Default::default()
            });
        }
        self
    }

    /// Set default window width and height in pixels.
    pub fn size(mut self, width: f32, height: f32) -> Self {
        self.default_size = Some(size(px(width), px(height)));
        self
    }

    /// Set explicit window bounds in pixels.
    pub fn bounds(mut self, bounds: Bounds<Pixels>) -> Self {
        self.window_options.window_bounds = Some(WindowBounds::Windowed(bounds));
        self
    }

    /// Set explicit window bounds (Windowed, Maximize, or Fullscreen).
    pub fn window_bounds(mut self, bounds: WindowBounds) -> Self {
        self.window_options.window_bounds = Some(bounds);
        self
    }

    /// Set minimum window width and height.
    pub fn min_size(mut self, width: f32, height: f32) -> Self {
        self.window_options.window_min_size = Some(size(px(width), px(height)));
        self
    }

    /// Set whether the window is resizable by the user.
    pub fn resizable(mut self, resizable: bool) -> Self {
        self.window_options.is_resizable = resizable;
        self
    }

    /// Set whether the window can be minimized by the user.
    pub fn minimizable(mut self, minimizable: bool) -> Self {
        self.window_options.is_minimizable = minimizable;
        self
    }

    /// Set whether the window is movable by the user.
    pub fn movable(mut self, movable: bool) -> Self {
        self.window_options.is_movable = movable;
        self
    }

    /// Set whether the window receives focus when created.
    pub fn focus(mut self, focus: bool) -> Self {
        self.window_options.focus = focus;
        self
    }

    /// Set whether the window is shown immediately when created.
    pub fn show(mut self, show: bool) -> Self {
        self.window_options.show = show;
        self
    }

    /// Set window kind (Normal, PopUp, Floating, etc.).
    pub fn kind(mut self, kind: WindowKind) -> Self {
        self.window_options.kind = kind;
        self
    }

    /// Set complete titlebar options.
    pub fn titlebar(mut self, titlebar: TitlebarOptions) -> Self {
        self.window_options.titlebar = Some(titlebar);
        self
    }

    /// Remove the titlebar entirely (chromeless/borderless window).
    pub fn without_titlebar(mut self) -> Self {
        self.window_options.titlebar = None;
        self
    }

    /// Set whether the titlebar appears transparent (for custom-drawn titlebars).
    pub fn transparent_titlebar(mut self, appears_transparent: bool) -> Self {
        if let Some(titlebar) = &mut self.window_options.titlebar {
            titlebar.appears_transparent = appears_transparent;
        } else {
            self.window_options.titlebar = Some(TitlebarOptions {
                appears_transparent,
                ..Default::default()
            });
        }
        self
    }

    /// Set window decorations on Linux (Client or Server).
    pub fn decorations(mut self, decorations: WindowDecorations) -> Self {
        self.window_options.window_decorations = Some(decorations);
        self
    }

    /// Enable Client-Side Decorations (CSD). The application renders the titlebar itself inside the window canvas.
    ///
    /// - Windows: Sets `WindowDecorations::Client` and removes OS titlebar via `WM_NCCALCSIZE` while preserving native snap/resize/shadows.
    /// - macOS: Content extends under transparent titlebar.
    /// - Linux (Wayland/X11): Disables server-side decorations.
    pub fn client_decorations(mut self, enabled: bool) -> Self {
        self.runtime.set_csd(enabled);
        if enabled {
            self.window_options.window_decorations = Some(WindowDecorations::Client);
            self.transparent_titlebar(true)
        } else {
            self.window_options.window_decorations = Some(WindowDecorations::Server);
            self.transparent_titlebar(false)
        }
    }

    /// Enable Client-Side Decorations (CSD).
    pub fn csd(self) -> Self {
        self.client_decorations(true)
    }

    /// Replace the CSD options configuration.
    pub fn csd_options(mut self, options: CsdOptions) -> Self {
        self.csd_options = options;
        self.runtime.set_csd_options(options);
        self
    }

    /// Mutate the CSD options configuration in place.
    pub fn with_csd_options<F: FnOnce(&mut CsdOptions)>(mut self, f: F) -> Self {
        f(&mut self.csd_options);
        self.runtime.set_csd_options(self.csd_options);
        self
    }

    /// Set application ID (used for desktop environment grouping).
    pub fn app_id(mut self, id: impl Into<String>) -> Self {
        self.window_options.app_id = Some(id.into());
        self
    }

    /// Set the display monitor on which the window opens.
    pub fn display(mut self, display_id: DisplayId) -> Self {
        self.window_options.display_id = Some(display_id);
        self
    }

    /// Set whether the application owns dragging the titlebar (macOS).
    pub fn app_owns_titlebar_drag(mut self, owns: bool) -> Self {
        self.window_options.app_owns_titlebar_drag = owns;
        self
    }

    /// Set inactive window animation frame interval.
    pub fn inactive_frame_interval(mut self, interval: Option<Duration>) -> Self {
        self.window_options.inactive_frame_interval = interval;
        self
    }

    /// Register a custom GPUI element available in Lua under `ui.<name>(props_or_children)`.
    pub fn register_element<F, E>(self, name: impl Into<String>, renderer: F) -> Self
    where
        F: Fn(crate::dsl::CustomElementContext) -> E + Send + Sync + 'static,
        E: gpui::IntoElement,
    {
        self.runtime.register_element(name, renderer);
        self
    }

    #[cfg(target_os = "windows")]
    /// Set Windows background appearance (Opaque, Transparent, Blurred/Acrylic, Mica, MicaAlt).
    pub fn windows_background(mut self, bg: gpui::WindowsWindowBackground) -> Self {
        self.window_options.windows_window_background = bg;
        self
    }


    #[cfg(target_os = "macos")]
    /// Set macOS window background appearance.
    pub fn macos_background(mut self, bg: gpui::MacosWindowBackground) -> Self {
        self.window_options.macos_window_background = bg;
        self
    }

    #[cfg(target_os = "macos")]
    /// Set macOS specific `NSVisualEffectMaterial` window background.
    pub fn macos_material(mut self, material: gpui::MacosVisualEffectMaterial) -> Self {
        self.window_options.macos_window_background = gpui::MacosWindowBackground::Material(material);
        self
    }

    #[cfg(target_os = "macos")]
    /// Parse and apply macOS window background appearance or `NSVisualEffectMaterial`.
    pub fn set_macos_background_str(mut self, bg: &str) -> Self {
        let normalized = bg.to_lowercase().trim().replace(['-', ' '], "_");
        match normalized.as_str() {
            "opaque" => self.macos_background(gpui::MacosWindowBackground::Opaque),
            "transparent" => self.macos_background(gpui::MacosWindowBackground::Transparent),
            "blurred" => self.macos_background(gpui::MacosWindowBackground::Blurred),
            "titlebar" => self.macos_material(gpui::MacosVisualEffectMaterial::Titlebar),
            "selection" => self.macos_material(gpui::MacosVisualEffectMaterial::Selection),
            "menu" => self.macos_material(gpui::MacosVisualEffectMaterial::Menu),
            "popover" => self.macos_material(gpui::MacosVisualEffectMaterial::Popover),
            "sidebar" => self.macos_material(gpui::MacosVisualEffectMaterial::Sidebar),
            "header_view" | "headerview" => self.macos_material(gpui::MacosVisualEffectMaterial::HeaderView),
            "sheet" => self.macos_material(gpui::MacosVisualEffectMaterial::Sheet),
            "window_background" | "windowbackground" => self.macos_material(gpui::MacosVisualEffectMaterial::WindowBackground),
            "hud" | "hud_window" | "hudwindow" => self.macos_material(gpui::MacosVisualEffectMaterial::HudWindow),
            "full_screen_ui" | "fullscreenui" => self.macos_material(gpui::MacosVisualEffectMaterial::FullScreenUI),
            "tooltip" => self.macos_material(gpui::MacosVisualEffectMaterial::ToolTip),
            "content_background" | "contentbackground" => self.macos_material(gpui::MacosVisualEffectMaterial::ContentBackground),
            "under_window_background" | "underwindowbackground" | "under_window" => self.macos_material(gpui::MacosVisualEffectMaterial::UnderWindowBackground),
            "under_page_background" | "underpagebackground" | "under_page" => self.macos_material(gpui::MacosVisualEffectMaterial::UnderPageBackground),
            "appearance_based" | "appearancebased" => self.macos_material(gpui::MacosVisualEffectMaterial::AppearanceBased),
            "light" => self.macos_material(gpui::MacosVisualEffectMaterial::Light),
            "dark" => self.macos_material(gpui::MacosVisualEffectMaterial::Dark),
            "medium_light" | "mediumlight" => self.macos_material(gpui::MacosVisualEffectMaterial::MediumLight),
            "ultra_dark" | "ultradark" => self.macos_material(gpui::MacosVisualEffectMaterial::UltraDark),
            _ => self,
        }
    }

    #[cfg(any(target_os = "linux", target_os = "freebsd"))]
    /// Set Linux window background appearance.
    pub fn linux_background(mut self, bg: gpui::LinuxWindowBackground) -> Self {
        self.window_options.linux_window_background = bg;
        self
    }

    /// Enable or disable hot reload on script changes (enabled by default).
    pub fn hot_reload(mut self, enabled: bool) -> Self {
        self.hot_reload = enabled;
        self
    }

    /// Access the underlying `LuaRuntime`.
    pub fn runtime(&self) -> &Arc<LuaRuntime> {
        &self.runtime
    }

    /// Access the underlying `BackendBridge`.
    pub fn backend(&self) -> &BackendBridge {
        self.runtime.backend()
    }

    /// Register a synchronous Rust function callable from Lua via `backend.<name>(...)`.
    pub fn register_fn<F, A, R>(&mut self, name: impl Into<String>, func: F) -> &mut Self
    where
        F: Fn(A) -> mlua::Result<R> + Send + Sync + 'static,
        A: FromLuaMulti + 'static,
        R: IntoLua + 'static,
    {
        self.runtime.register_fn(name, func);
        self
    }

    /// Register a synchronous Rust function accepting any deserializable argument and returning any serializable type.
    pub fn register_json_fn<F, A, R>(&mut self, name: impl Into<String>, func: F) -> &mut Self
    where
        F: Fn(A) -> std::result::Result<R, String> + Send + Sync + 'static,
        A: serde::de::DeserializeOwned + 'static,
        R: serde::Serialize + 'static,
    {
        self.runtime.register_json_fn(name, func);
        self
    }

    /// Register an asynchronous Rust function callable from Lua via `backend.<name>(..., callback)`.
    pub fn register_async_fn<F, Fut, A, R>(&mut self, name: impl Into<String>, func: F) -> &mut Self
    where
        F: Fn(A) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = std::result::Result<R, String>> + Send + 'static,
        A: serde::de::DeserializeOwned + Send + 'static,
        R: serde::Serialize + Send + 'static,
    {
        self.runtime.register_async_fn(name, func);
        self
    }

    /// Emit an event from the Rust backend to Lua frontend listeners (`backend.on(event, handler)`).
    pub fn emit(&self, event: &str, data: impl serde::Serialize) -> std::result::Result<(), String> {
        self.runtime.emit(event, data)
    }

    /// Register a Rust listener for events emitted from Lua (`backend.emit(event, data)`).
    pub fn on<F>(&mut self, event: impl Into<String>, handler: F) -> &mut Self
    where
        F: Fn(&serde_json::Value) + Send + Sync + 'static,
    {
        self.runtime.on(event, handler);
        self
    }

    /// Set a reactive signal value from Rust.
    pub fn set_signal(&self, key: impl Into<String>, val: StoredValue) {
        self.runtime.store().set(key.into(), val);
        self.runtime.bridge().notify();
    }

    /// Read a reactive signal value from Rust.
    pub fn get_signal(&self, key: &str) -> Option<StoredValue> {
        self.runtime.store().get(key)
    }

    /// Launch the GPUI application window and execute the Lua frontend.
    pub fn run(self) -> anyhow::Result<()> {
        let resolved_path = resolve_script_path(&self.script_path);
        let _ = self.runtime.load_script(&resolved_path);

        if self.hot_reload && resolved_path.exists() {
            let _ = self.runtime.enable_hot_reload();
        }

        #[cfg(feature = "net")]
        let app = {
            let http_client = Arc::new(ReqwestHttpClient::new());
            gpui_platform::application()
                .with_assets(self.runtime.assets().clone())
                .with_http_client(http_client)
        };
        #[cfg(not(feature = "net"))]
        let app = gpui_platform::application()
            .with_assets(self.runtime.assets().clone());
        let is_csd = self.window_options.window_decorations == Some(WindowDecorations::Client)
            || self.window_options.titlebar.as_ref().is_some_and(|t| t.appears_transparent);
        if is_csd {
            self.runtime.set_csd(true);
        }
        if let Some(tb) = &self.window_options.titlebar {
            if let Some(t) = &tb.title {
                self.runtime.set_title(t.to_string());
            }
        }
        self.runtime.set_csd_options(self.csd_options);
        let runtime_clone = self.runtime.clone();
        let default_size = self.default_size;
        let mut window_options = self.window_options;
        app.run(move |cx: &mut App| {
            let _tokio_guard = crate::tokio_runtime().enter();
            cx.activate(true);
            cx.on_window_closed(|cx, _window_id| {
                if cx.windows().is_empty() {
                    cx.quit();
                }
            })
            .detach();

            if window_options.window_bounds.is_none() {
                let s = default_size.unwrap_or_else(|| size(px(800.0), px(600.0)));
                let bounds = Bounds::centered(None, s, cx);
                window_options.window_bounds = Some(WindowBounds::Windowed(bounds));
            }

            cx.open_window(
                window_options,
                |_window, cx| cx.new(|cx| LuaView::new(runtime_clone, cx)),
            )
            .expect("Failed to open GPUI window");
        });

        Ok(())
    }
}
