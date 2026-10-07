# Rust API Reference

Rust interfaces and structs exported by `gpui_lua` for embedding Lua in Rust.

```rust
use gpui_lua::LuaApp;
```

---

## 1. Application Host (`LuaApp`)

### Constructors

```rust
// Load from file path
let app = LuaApp::new("ui/app.lua");

// Try loading (returns Result instead of panicking)
let app = LuaApp::try_new("ui/app.lua")?;

// Load from embedded RustEmbed asset struct
let app = LuaApp::embedded::<MyAssets>("ui/app.lua");

// Launch from standalone bundled package manifest
let app = LuaApp::from_bundled_manifest(manifest)?;
```

### Configuration Methods

```rust
let app = LuaApp::new("ui/app.lua")
    // Set window title
    .title("My Desktop App")
    // Set initial window dimensions
    .size(900.0, 650.0)
    // Set minimum window bounds
    .min_size(400.0, 300.0)
    // Toggle resizable window
    .resizable(true)
    // Enable client-side titlebar decorations
    .csd()
    // Configure titlebar button and hover colors
    .csd_options(CsdOptions {
        height: 38.0,
        button_width: 46.0,
        ..Default::default()
    })
    // Enable Windows 11 Acrylic frosted glass window background
    .windows_background(gpui::WindowsWindowBackground::Acrylic)
    // Enable filesystem hot reloading
    .hot_reload(true);
```

### Running the Application

```rust
// Blocks current thread and runs GPUI event loop until window is closed
app.run()?;
```

---

## 2. Window & Chrome Configuration (`CsdOptions`)

Custom Client-Side Decoration styling:

```rust
use gpui_lua::CsdOptions;

let options = CsdOptions {
    height: 38.0,
    button_width: 46.0,
    icon_color: gpui::rgb(0xcdd6f4).into(),
    hover_bg: gpui::rgba(0xffffff14).into(),
    active_bg: gpui::rgba(0xffffff28).into(),
    close_hover_bg: gpui::rgb(0xe81123).into(),
    close_active_bg: gpui::rgb(0xdc0a1c).into(),
    close_hover_color: gpui::white().into(),
};
```

---

## 3. Rust Backend RPC Functions

Expose native Rust functions to Lua (callable via `backend.<name>(...)`):

### Synchronous Rust Functions
```rust
// Rust
app.register_fn("add_numbers", |(a, b): (i64, i64)| {
    Ok(a + b)
})?;
```
```lua
-- Lua
local sum = backend.add_numbers(10, 20)
print(sum) -- 30
```

### JSON RPC Functions
```rust
// Rust
app.register_json_fn("get_user", |args: serde_json::Value| {
    let id = args["id"].as_i64().unwrap_or(0);
    Ok(serde_json::json!({
        "id": id,
        "name": "Alice",
        "roles": ["admin", "developer"]
    }))
})?;
```
```lua
-- Lua
local user = backend.get_user({ id = 42 })
print(user.name, user.roles[1])
```

### Asynchronous Tokio Tasks
```rust
// Rust
app.register_async_fn("fetch_remote", |url: String| async move {
    let text = reqwest::get(&url).await?.text().await?;
    Ok(text)
})?;
```

---

## 4. Reactive State Access from Rust

Read and write Lua reactive signals directly from Rust:

```rust
use gpui_lua::reactive::store::StoredValue;

// Set signal value from Rust (triggers Lua UI re-render)
app.set_signal("counter_val", StoredValue::Integer(100));

// Read signal value in Rust
if let Some(val) = app.get_signal("counter_val") {
    println!("Current count: {val:?}");
}
```

---

## 5. Single-Binary Bundler (`bundle`)

Package Lua scripts and assets directly into an executable binary:

```rust
use gpui_lua::{bundle_standalone_binary, check_bundled_app, LuaApp};
use std::path::Path;

// 1. Check if current binary is a packaged standalone app
if let Some(manifest) = check_bundled_app() {
    LuaApp::from_bundled_manifest(manifest)?.run()?;
    return Ok(());
}

// 2. Programmatically bundle a script into an executable
bundle_standalone_binary(
    Path::new("target/release/gpui-lua.exe"), // Base executable template
    Path::new("counter.lua"),                 // Lua entrypoint script
    Some(Path::new("./assets")),              // Static assets directory
    Path::new("dist/app.exe"),                // Destination executable
    Some("My App".to_string()),               // Window title
    Some(900.0),                              // Initial width
    Some(650.0),                              // Initial height
    true,                                     // Enable CSD
)?;
```
