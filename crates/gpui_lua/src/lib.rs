extern crate ulua_rt as mlua;
pub use ulua_rt;

pub mod app;
pub mod config;
pub mod assets;
pub mod bundle;
pub mod backend;
pub mod dsl;
pub mod hot_reload;
pub mod input;
pub mod net;
pub mod reactive;
pub mod runtime;
pub mod stdlib;

pub use dsl::{CustomElementContext, CustomElementRenderer, register_custom_element, LuaElementBuilder, LuaNode};
pub use hot_reload::{HotReloadError, ScriptWatcher};
pub use reactive::{ReactiveBridge, ReactiveStore};
pub use app::LuaApp;
pub use config::AppConfig;
pub use bundle::{bundle_standalone_binary, check_bundled_app, AppBundleManifest};
pub use gpui::{
    Bounds, DisplayId, Pixels, Point, SharedString, Size, TitlebarOptions, WindowBounds,
    WindowControlArea, WindowDecorations, WindowKind, WindowOptions, px, size,
};
#[cfg(target_os = "windows")]
pub use gpui::WindowsWindowBackground;
#[cfg(target_os = "macos")]
pub use gpui::MacosWindowBackground;
#[cfg(any(target_os = "linux", target_os = "freebsd"))]
pub use gpui::LinuxWindowBackground;
pub use backend::BackendBridge;
pub use assets::{EmbeddedAssetManager, EmbeddedStdlib, EmbeddedTypes};
#[cfg(feature = "media")]
pub use media::{video, audio, VideoPlayer, VideoManager, AudioPlayer, AudioManager};
pub use input::InputState;

use std::sync::LazyLock;
use tokio::runtime::Runtime;

pub static TOKIO_RUNTIME: LazyLock<Runtime> = LazyLock::new(|| {
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("Failed to initialize Tokio runtime")
});

pub fn tokio_runtime() -> &'static Runtime {
    &TOKIO_RUNTIME
}
pub use runtime::{CsdOptions, IntoHsla, LuaRuntime, LuaView};
