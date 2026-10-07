# Rust API Reference

Rust API reference for embedding and customizing `gpui_lua`.

## `LuaApp`

The main application builder and runner.

```rust
use gpui_lua::app::LuaApp;
```

### Constructors
- `LuaApp::new(script_path: impl AsRef<Path>) -> Self`
  Initializes application using script entrypoint. Panics on runtime init failure.
- `LuaApp::try_new(script_path: impl AsRef<Path>) -> anyhow::Result<Self>`
  Fallible initialization.
- `LuaApp::from_bundled_manifest(manifest: AppBundleManifest) -> anyhow::Result<Self>`
  Initializes embedded application from packaged single-binary manifest.

### Window Styling & Configuration Methods
- `.title(title: impl Into<SharedString>) -> Self`
- `.size(width: f32, height: f32) -> Self`
- `.min_size(width: f32, height: f32) -> Self`
- `.resizable(resizable: bool) -> Self`
- `.minimizable(minimizable: bool) -> Self`
- `.csd() -> Self`: Enables Client-Side Decorations.
- `.no_csd() -> Self`: Disables Client-Side Decorations.
- `.csd_options(options: CsdOptions) -> Self`
- `.hot_reload(enabled: bool) -> Self`: Controls filesystem watcher.

### OS-Specific Window Backdrops
- Windows:
  - `.windows_background(bg: gpui::WindowsWindowBackground) -> Self`
- macOS:
  - `.macos_background(bg: gpui::MacosWindowBackground) -> Self`
  - `.set_macos_background_str(bg: &str) -> Self`
  - `.macos_material(mat: gpui::MacosVisualEffectMaterial) -> Self`
- Linux:
  - `.linux_background(bg: gpui::LinuxWindowBackground) -> Self`
### Font Management
- `.with_font(self, path: impl AsRef<Path>) -> Self`: Load TrueType / OpenType font from file or asset.
- `.load_font(self, path: impl AsRef<Path>) -> Self`: Alias for `with_font`.
- `.with_font_bytes(self, bytes: impl Into<Cow<'static, [u8]>>) -> Self`: Load font from in-memory bytes.

- `.register_element<F, E>(self, name: impl Into<String>, renderer: F) -> Self`
  Registers a custom GPUI element available in Lua under `ui.<name>(props_or_children)`.

### Execution
- `.run(self) -> anyhow::Result<()>`: Launches the GPUI application loop on the main thread.
- `.runtime(&self) -> &Arc<LuaRuntime>`: Borrow access to underlying Lua runtime and MLua engine.

---

## `CustomElementContext`

Context passed to custom element renderers registered via `.register_element`:

```rust
pub struct CustomElementContext {
    pub tag: String,
    pub props: serde_json::Value,
    pub children: Vec<AnyElement>,
}

impl CustomElementContext {
    pub fn get_str(&self, key: &str) -> Option<&str>;
    pub fn get_bool(&self, key: &str) -> Option<bool>;
    pub fn get_f64(&self, key: &str) -> Option<f64>;
    pub fn get_i64(&self, key: &str) -> Option<i64>;
    pub fn get_prop<T: DeserializeOwned>(&self, key: &str) -> Option<T>;
    pub fn deserialize<T: DeserializeOwned>(&self) -> Result<T, serde_json::Error>;
}
```
---

## `CsdOptions`

Titlebar dimensions and color options for Client-Side Decorations:

```rust
pub struct CsdOptions {
    pub height: f32,             // Default: 38.0
    pub button_width: f32,       // Default: 46.0
    pub icon_color: Hsla,
    pub hover_bg: Hsla,
    pub active_bg: Hsla,
    pub close_hover_bg: Hsla,
    pub close_active_bg: Hsla,
    pub close_hover_color: Hsla,
}
```

---

## `AppConfig` (`gpui.toml`)

Declarative configuration struct for projects:

```rust
use gpui_lua::config::AppConfig;

let config = AppConfig::load_file("gpui.toml")?;
let app = config.apply_to_app(LuaApp::new("main.lua"));
```
