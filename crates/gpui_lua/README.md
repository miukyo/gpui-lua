# GPUI.lua

High-performance GPU-accelerated Lua desktop runtime and single-binary packaging system powered by GPUI-CE and LuaJIT.

## Two Workflows

1. **Standalone Lua CLI (`gpui-lua`)**:
   - No Rust toolchain or C/C++ compiler needed.
   - Download prebuilt binaries directly from [GitHub Releases](https://github.com/miukyo/gpui-lua/releases).
   - Commands: `gpui-lua init [name]`, `gpui-lua dev`, `gpui-lua build`.

2. **Rust Backend + Lua Frontend (Hybrid)**:
   - For Rust applications embedding GPUI-CE and Lua.
   - Add as Git dependency from `https://github.com/miukyo/gpui-lua`:
     ```toml
     [dependencies]
     gpui_lua = { git = "https://github.com/miukyo/gpui-lua" }
     gpui = { git = "https://github.com/miukyo/gpui-lua" }
     ```
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

### Rust API

```rust
use gpui_lua::{CustomElementContext, LuaApp};
use gpui::{div, rgb, px, ParentElement, Styled, WindowsWindowBackground};

let app = LuaApp::new("main.lua")
    .windows_background(WindowsWindowBackground::Acrylic)
    // Register custom native GPUI component callable in Lua via ui.my_widget(props)
    .register_element("my_widget", |cx: CustomElementContext| {
        let label = cx.get_str("label").unwrap_or("Default");
        div()
            .bg(rgb(0x313244))
            .p(px(8.0))
            .child(label.to_string())
            .children(cx.children)
    });
```

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

Packaging automatically embeds:
- **Windows**: `VS_VERSION_INFO` resource & multi-resolution `.ico` icon in `.exe`.
- **macOS**: `.app` bundle with `Contents/Info.plist` & `Resources/{icon}.icns`.
- **Linux**: XDG `.desktop` launcher & application icon.
