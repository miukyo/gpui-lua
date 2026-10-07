# Rust Backend Integration

GPUI.lua enables an architecture where **Rust serves as the high-performance system backend** and **Lua defines the dynamic UI frontend**.

## Architecture Overview

```
┌──────────────────────────────────────────────┐
│                  Rust Backend                │
│   • Database / SQLite queries                │
│   • Heavy compute / FFI / Native hardware    │
│   • Background network services              │
└──────────────────────▲───────────────────────┘
                       │ Channels / Signals
┌──────────────────────▼───────────────────────┐
│                  Lua Frontend                │
│   • Reactive UI tree rendering               │
│   • Event handling & Animations              │
│   • Instant hot-reload iterations            │
└──────────────────────────────────────────────┘
```

## Embedding GPUI.lua in a Rust Application

Add `gpui_lua` to your `Cargo.toml`:

```toml
[dependencies]
gpui_lua = { git = "https://github.com/miukyo/gpui-ce" }
gpui = { git = "https://github.com/miukyo/gpui-ce" }

In your `main.rs`:

```rust
use gpui_lua::app::LuaApp;
use gpui::size;

fn main() -> anyhow::Result<()> {
    let app = LuaApp::new("frontend.lua")
        .title("Rust + GPUI.lua Hybrid")
        .size(900.0, 600.0)
        .windows_background(gpui::WindowsWindowBackground::Acrylic)
        .hot_reload(true);

    // Register custom Rust functions into Lua global scope
    let lua = app.runtime().lua();
    let globals = lua.globals();

    globals.set(
        "rust_compute_fibonacci",
        lua.create_function(|_, n: u64| {
            fn fib(x: u64) -> u64 {
                if x <= 1 { x } else { fib(x - 1) + fib(x - 2) }
            }
            Ok(fib(n))
        })?,
    )?;

    app.run()
}
```

## Invoking Rust Methods from Lua

In your `frontend.lua`:

```lua
local result, set_result = signal(0, "fib_val")

function App()
  return ui.Card({
    title = "Rust Native Compute Bridge",
    subtitle = "Calculates n-th Fibonacci number in compiled Rust",
    children = {
      ui.text("Result: " .. tostring(result())):size(24):bold(),
      ui.Button({
        label = "Compute fib(30)",
        variant = "primary",
        on_click = function()
          local val = rust_compute_fibonacci(30)
          set_result(val)
        end
      })
    }
  })
end
```
## Registering Custom GPUI Elements from Rust

You can register any native GPUI component into Lua with a single method call:

```rust
use gpui_lua::{CustomElementContext, LuaApp};
use gpui::{div, rgb, px, ParentElement, Styled};

fn main() -> anyhow::Result<()> {
    let app = LuaApp::new("app.lua")
        // Registers `ui.my_badge(props)` in Lua
        .register_element("my_badge", |cx: CustomElementContext| {
            let label = cx.get_str("label").unwrap_or("Default");
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

In your Lua script:

```lua
function App()
  return ui.div({
    children = {
      ui.my_badge({ label = "Native Rust Widget" }),
      ui.my_badge():prop("label", "Chained Syntax"):child(ui.text("Inner child"))
    }
  })
end
```

## Rust Backend Bridge (`BackendBridge`)

Use `BackendBridge` to dispatch asynchronous tasks from background threads and trigger re-renders:

```rust
use gpui_lua::backend::BackendBridge;

// Spawn background thread to query DB or network
std::thread::spawn(move || {
    let data = fetch_heavy_dataset();
    // Dispatch to Lua state
});
```
