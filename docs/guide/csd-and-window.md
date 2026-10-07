# Client-Side Decorations & Native Window Backdrops

GPUI.lua provides native desktop window management with hardware-accelerated **Client-Side Decorations (CSD)** and OS-specific visual backdrops (Windows Mica/Acrylic, macOS NSVisualEffectMaterial vibrancy, and Linux transparent compositing).

## Client-Side Decorations (CSD)

When CSD is enabled, GPUI renders a custom titlebar with minimize, maximize, and close controls directly on the GPU render surface.

### Configuring CSD in `gpui.toml`

```toml
[window]
title = "My App"
width = 960
height = 680
csd = true
csd_height = 38.0
```

### Configuring CSD via Rust API

```rust
use gpui_lua::app::LuaApp;
use gpui_lua::runtime::CsdOptions;
use gpui::rgb;

let app = LuaApp::new("main.lua")
    .csd()
    .csd_options(CsdOptions {
        height: 38.0,
        button_width: 46.0,
        icon_color: rgb(0xcdd6f4).into(),
        hover_bg: rgb(0x45475a).into(),
        active_bg: rgb(0x585b70).into(),
        close_hover_bg: rgb(0xe81123).into(),
        close_active_bg: rgb(0xdc0a1c).into(),
        close_hover_color: gpui::white().into(),
    });
```

---

## Native Window Backdrops

GPUI.lua supports OS-specific frosted, blurred, and transparent materials:

### Windows 10 & 11

| Backdrop | Supported OS | Implementation | Description |
| :--- | :--- | :--- | :--- |
| `acrylic` | Windows 10 & 11 | `SetWindowCompositionAttribute` (accent state 4) / `DWMWA_SYSTEMBACKDROP_TYPE = 3` | Frosted glass texture with noise and blur |
| `mica` | Windows 11 (22000+) | `DWMWA_SYSTEMBACKDROP_TYPE = 2` | Dynamic theme-tinted backdrop sampling desktop wallpaper |
| `mica-alt` / `tabbed` | Windows 11 (22621+) | `DWMWA_SYSTEMBACKDROP_TYPE = 4` | Layered high-contrast Mica variant for tabbed surfaces |
| `transparent` | Windows 10 & 11 | Alpha compositor pass | Fully transparent window canvas |
| `opaque` | All Windows versions | Solid clear color | Standard solid GPU clear color |

::: tip Automatic Windows 10 Fallback
On Windows 10 where Mica is unsupported, GPUI.lua automatically falls back to Acrylic blur rather than failing or displaying an opaque surface.
:::

### macOS Vibrancy (`NSVisualEffectMaterial`)

On macOS, windows can use any native `NSVisualEffectMaterial` blur:

| Material Name | Visual Effect |
| :--- | :--- |
| `sidebar` | Standard sidebar material |
| `titlebar` | Window titlebar vibrancy |
| `menu` | Menu bar and context menu vibrancy |
| `popover` | Popover translucent background |
| `hud` / `hud_window` | Heads-up display translucent dark blur |
| `sheet` | Modal sheet background |
| `selection` | In-window selection highlight material |
| `tooltip` | Tooltip translucent background |
| `content_background` | In-window content material |
| `under_window_background`| Translucent under-window surface |
| `under_page_background` | Translucent under-page surface |
| `appearance_based` | Dynamically adapts to Light/Dark system mode |
| `light` | Light vibrancy material |
| `dark` | Dark vibrancy material |
| `medium_light` | Medium light vibrancy |
| `ultra_dark` | Ultra dark vibrancy |
| `transparent` | Fully transparent NSWindow |
| `opaque` | Standard solid opaque window |

### Linux (Wayland / X11)

| Backdrop | Description |
| :--- | :--- |
| `transparent` | Fully transparent window canvas for compositor transparency |
| `opaque` | Solid standard clear color |
| `blurred` | Requests blur backdrop if supported by Wayland/X11 compositor |

---

## Configuration via `gpui.toml`

GPUI.lua provides multiple flexible ways to configure window backdrops in `gpui.toml`:

### 1. Per-Platform Table (`[window.background]`)

Target each operating system with its native material:

```toml
[window]
title = "Cross-Platform Dashboard"
width = 1000
height = 700
csd = true

# Per-platform window backdrop configuration
[window.background]
windows = "acrylic"       # transparent, mica, mica-alt, acrylic, opaque
macos = "sidebar"         # any NSVisualEffectMaterial (sidebar, hud, etc.)
linux = "transparent"     # transparent, opaque
```

### 2. Direct Platform Keys

```toml
[window]
title = "Cross-Platform Dashboard"
windows_background = "mica"
macos_background = "hud"
linux_background = "transparent"
```

### 3. Universal Shorthand

Apply a single string across platforms:

```toml
[window]
background = "acrylic"
```

---

## CLI Options & Flags

Override window background settings on the command line:

### General & Platform-Specific Flags

```bash
# Universal background flag
gpui-lua run main.lua --background acrylic

# Target individual platforms
gpui-lua run main.lua --windows-background mica-alt --macos-background sidebar --linux-background transparent
```

---

## Rust API Methods

When configuring `LuaApp` in Rust:

```rust
use gpui_lua::app::LuaApp;
use gpui::MacosVisualEffectMaterial;

let mut app = LuaApp::new("main.lua");

// Windows-specific backdrop setters
#[cfg(target_os = "windows")]
{
    // Acrylic, MicaBackdrop, MicaAltBackdrop, Transparent, or Opaque
    app = app.windows_background(gpui::WindowsWindowBackground::Acrylic);
}

// macOS-specific backdrop setters
#[cfg(target_os = "macos")]
{
    app = app.set_macos_background_str("sidebar");
    app = app.macos_material(MacosVisualEffectMaterial::HudWindow);
}

// Linux-specific backdrop setters
#[cfg(any(target_os = "linux", target_os = "freebsd"))]
{
    app = app.linux_background(gpui::LinuxWindowBackground::Transparent);
}
```
