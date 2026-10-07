# Getting Started

GPUI.lua offers two distinct workflows depending on whether you are writing pure Lua or building a hybrid Rust application:

1. **Standalone Lua CLI Workflow**: Download the precompiled `gpui-lua` binary. No Rust toolchain, C/C++ compiler, or build tools required.
2. **Rust Backend + Lua Frontend Workflow**: Embed `gpui_lua` directly into your Rust application via Git dependency.

---

## Workflow 1: Standalone Lua CLI

If you want to build desktop applications using pure Lua, you do not need Rust or a C/C++ compiler.

### 1. Download the CLI Binary

Download the precompiled `gpui-lua` binary for Windows, macOS, or Linux directly from GitHub Releases:

👉 **[Download `gpui-lua` Releases](https://github.com/miukyo/gpui-lua/releases)**

Place the executable in your `PATH` (or working directory).

*(Optional)* If you already have a Rust toolchain installed, you can build from source:

```bash
cargo install --git https://github.com/miukyo/gpui-lua gpui_lua
```

### 2. Scaffold a Project

Run `init` to create a project with configuration, starter UI, and LuaLS type definitions:

```bash
gpui-lua init my-app
cd my-app
```

This generates:
- `gpui.toml`: Window geometry, backdrops, and packaging configuration.
- `main.lua`: Reactive counter application entrypoint.
- `gpui.d.lua`: Comprehensive EmmyLua / LuaLS type annotations for instant editor autocomplete.

### 3. Run with Live Hot Reloading

Start development mode:

```bash
gpui-lua dev
```

Any edits to `main.lua` are immediately reflected on the GPU canvas while preserving reactive state.

### 4. Package Standalone Binary

Bundle your Lua code, assets, and OS-level executable resources into a single standalone binary:

```bash
gpui-lua build -o dist/my-app.exe
```

The resulting executable in `dist/` runs on any machine with zero external dependencies.

---

## Workflow 2: Rust Backend + Lua Frontend

For hybrid architectures where compiled Rust handles high-performance backend logic, databases, or native hardware, and Lua drives the dynamic UI frontend.

### Requirements

- **Rust toolchain** (1.80+ recommended)
- **C/C++ compiler** (MSVC on Windows, Clang on macOS, GCC/Clang on Linux)

### Adding the Dependency

Because `gpui_lua` and the GPUI-CE fork are not published to crates.io, add the dependency pointing to the GitHub repository:

```toml
[dependencies]
gpui_lua = { git = "https://github.com/miukyo/gpui-lua" }
gpui = { git = "https://github.com/miukyo/gpui-lua" }
```

### Application Entrypoint

In your `src/main.rs`:

```rust
use gpui_lua::{CustomElementContext, LuaApp};
use gpui::{div, rgb, px, ParentElement, Styled, WindowsWindowBackground};

fn main() -> anyhow::Result<()> {
    let app = LuaApp::new("main.lua")
        .title("Rust + GPUI.lua App")
        .size(960.0, 680.0)
        .windows_background(WindowsWindowBackground::Acrylic)
        .hot_reload(true)
        // Register custom native GPUI elements callable in Lua via ui.my_badge(props)
        .register_element("my_badge", |cx: CustomElementContext| {
            let label = cx.get_str("label").unwrap_or("Badge");
            div()
                .bg(rgb(0x313244))
                .p(px(8.0))
                .rounded(px(4.0))
                .child(label.to_string())
                .children(cx.children)
        });

    app.run()
}
```

In your `main.lua`:

```lua
local count, set_count = signal(0, "val")

function App()
  return ui.div({
    w_full = true,
    h_full = true,
    items_center = true,
    justify_center = true,
    p = 24,
    children = {
      ui.my_badge({ label = "Native Element" }),
      ui.Button({
        label = "Increment: " .. tostring(count()),
        on_click = function() set_count(count() + 1) end
      })
    }
  })
end
```

---

## CLI Reference

### Commands

| Command | Description |
| :--- | :--- |
| `gpui-lua init [NAME]` | Scaffold a project with `gpui.toml`, `main.lua`, and `gpui.d.lua` |
| `gpui-lua run [SCRIPT]` | Execute a Lua application script |
| `gpui-lua dev [SCRIPT]` | Run with filesystem watcher and live hot reloading |
| `gpui-lua build [SCRIPT]` | Package the application into a standalone executable |
| `gpui-lua version` | Print engine and runtime version info |

### CLI Options

```bash
OPTIONS:
    -c, --config <PATH>       Path to config file (default: gpui.toml)
    -o, --output <PATH>       Output executable path for build
    -a, --assets <DIR>        Assets directory to bundle alongside script
    -t, --title <STRING>      Initial window title
    -w, --width <NUMBER>      Initial window width in pixels
    -h, --height <NUMBER>     Initial window height in pixels
    --min-width <NUMBER>      Minimum window width constraint
    --min-height <NUMBER>     Minimum window height constraint
    --csd / --no-csd          Enable or disable custom Client-Side Decorations
    --csd-height <NUMBER>     Height of CSD titlebar in pixels (default: 38)
    --background <VALUE>      Universal window backdrop effect
    --windows-background <V>  Windows background (transparent, mica, mica-alt, acrylic, opaque)
    --macos-background <V>    macOS background (NSVisualEffectMaterial, transparent, opaque)
    --linux-background <V>    Linux background (transparent, opaque)
    --icon <PATH>             Application icon (.ico, .icns, or .png)
    --product-name <STRING>   Product name resource
    --file-description <STR>  File description resource
    --company <STRING>        Company or author resource
    --copyright <STRING>      Legal copyright resource
    --app-version <VERSION>   Application semantic version
    --identifier <ID>         App bundle / desktop identifier (e.g. com.example.app)
    --template <PATH>         Base template executable to package onto
```
