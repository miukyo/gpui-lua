<p align="center">
  <img src="docs/public/logo.svg" alt="GPUI.lua logo" width="240">
</p>

# GPUI.lua (GPUI-CE Fork)

High-performance GPU-accelerated Lua desktop runtime and single-binary packaging system powered by a dedicated fork of **GPUI-CE** and **LuaJIT 2.1**.

This repository is **not** the upstream GPUI-CE community edition. It is a personal fork maintained as the engine behind **[gpui.lua](crates/gpui_lua)** — adding native capabilities required for desktop applications: hardware-accelerated media playback, client-side decorations (CSD), and OS-native window vibrancy.

---

## Two Workflows

### 1. Standalone Lua CLI (`gpui-lua`)
- **Zero compile toolchain needed**: No Rust toolchain or C/C++ compiler required.
- **Download prebuilt binaries**: Directly from **[GitHub Releases](https://github.com/miukyo/gpui-lua/releases)**.
- **Commands**:
  - `gpui-lua init [name]` — Scaffolds project with `gpui.toml`, `main.lua`, and `gpui.d.lua` type definitions.
  - `gpui-lua dev [script]` — Runs live with filesystem watcher and hot reloading.
  - `gpui-lua build [script]` — Packages script, assets, and OS metadata into a single standalone binary.

### 2. Rust Backend + Lua Frontend (Hybrid)
- For Rust applications embedding GPUI-CE and Lua.
- Add as a Git dependency pointing to this repository:
  ```toml
  [dependencies]
  gpui_lua = { git = "https://github.com/miukyo/gpui-lua" }
  gpui = { git = "https://github.com/miukyo/gpui-lua" }
  ```

---

## Window Backgrounds & Backdrops

GPUI.lua supports hardware-accelerated and OS-native window backdrops across Windows, macOS, and Linux.

### Supported Materials

- **Windows 10 & 11**:
  - `acrylic`: Frosted glass translucent blur via DWM / `SetWindowCompositionAttribute`
  - `mica`: Dynamic wallpaper-tinted material (Windows 11 build 22000+)
  - `mica-alt` / `tabbed`: High-contrast tabbed Mica variant (Windows 11 build 22621+)
  - `transparent`: Full transparent window compositing pass
  - `opaque`: Standard solid window background
  - *Fallback*: Windows 10 automatically falls back to Acrylic blur when Mica is configured.
- **macOS**:
  - Full `NSVisualEffectMaterial` vibrancy support: `sidebar`, `titlebar`, `hud`, `menu`, `popover`, `sheet`, `selection`, `tooltip`, `light`, `dark`, `ultra_dark`
  - `transparent` and `opaque`
- **Linux (Wayland / X11)**:
  - `transparent`: Full compositor transparency
  - `opaque`: Solid background
  - `blurred`: Compositor blur where supported

### Configuration (`gpui.toml`)

```toml
[window]
title = "My App"
width = 960
height = 650
csd = true

# Per-platform backdrop settings
[window.background]
windows = "acrylic"       # transparent, mica, mica-alt, acrylic, opaque
macos = "sidebar"         # any NSVisualEffectMaterial (sidebar, hud, etc.)
linux = "transparent"     # transparent, opaque
```

### CLI Overrides

```bash
gpui-lua run main.lua --background acrylic
gpui-lua run main.lua --windows-background mica-alt --macos-background sidebar
gpui-lua run main.lua --windows-background transparent
```

---

## Rust API & Custom GPUI Elements

Embed `gpui_lua` into Rust and expose native GPUI components callable directly from Lua:

```rust
use gpui_lua::{CustomElementContext, LuaApp};
use gpui::{div, rgb, px, ParentElement, Styled, WindowsWindowBackground};

fn main() -> anyhow::Result<()> {
    let app = LuaApp::new("main.lua")
        .title("Rust + GPUI.lua App")
        .windows_background(WindowsWindowBackground::Acrylic)
        // Register custom native GPUI component callable in Lua via ui.my_widget(props)
        .register_element("my_widget", |cx: CustomElementContext| {
            let label = cx.get_str("label").unwrap_or("Default");
            div()
                .bg(rgb(0x313244))
                .p(px(8.0))
                .rounded(px(6.0))
                .child(label.to_string())
                .children(cx.children)
        });

    app.run()
}
```

In Lua:

```lua
function App()
  return ui.div({
    children = {
      ui.my_widget({ label = "Native Rust Widget" }),
      ui.my_widget():prop("label", "Chained syntax"):child(ui.text("Child"))
    }
  })
end
```

---

## Executable Resources & OS Metadata

Configure binary metadata and application icons in `gpui.toml`:

```toml
[app]
name = "My App"
version = "1.0.0"
product_name = "My Product"
file_description = "High-performance desktop tool"
company_name = "Acme Corp"
copyright = "Copyright (c) 2026"
identifier = "com.acme.myapp"
icon = "assets/icon.ico"
```

Running `gpui-lua build` automatically embeds:
- **Windows**: `VS_VERSION_INFO` resource table & multi-resolution `.ico` icon into the `.exe`.
- **macOS**: Standalone `.app` bundle with `Contents/Info.plist` & `Resources/{icon}.icns`.
- **Linux**: XDG `.desktop` launcher & application icon.

---

## Why this GPUI-CE Fork Exists

| Feature | Why |
| :--- | :--- |
| **Video & Audio via FFmpeg** | Native `video` / `audio` elements with hardware-accelerated decoding without shelling out to a browser engine. |
| **Backdrop Blur + Overflow Fade** | Composes container blur with overflow fading without dropping visual effects. |
| **macOS WGPU Rendering** | Unifies macOS rendering with Linux and WebGPU pipelines for cross-platform consistency. Windows uses DirectX 11/12. |
| **Client-Side Decorations (CSD)** | Custom titlebars and window control buttons matching application UI. |
| **LuaJIT Dynamic Frontend** | Lua DSL with reactive state signals (`signal()`) and hot reloading with state preservation. |

*Note for pure Rust developers*: All underlying GPUI-CE crates remain standard Rust UI crates. You can write pure Rust GUI applications using the examples in `crates/gpui/examples/learn`.

---

## Documentation & Links

- **Documentation & Playground**: [`docs`](docs)
- **GPUI Learn Examples**: [`crates/gpui/examples/learn`](crates/gpui/examples/learn)
- **Releases**: [github.com/miukyo/gpui-lua/releases](https://github.com/miukyo/gpui-lua/releases)
- **Upstream GPUI-CE**: [github.com/gpui-ce/gpui-ce](https://github.com/gpui-ce/gpui-ce)
