# Configuration File (`gpui.toml`)

`GPUI.lua` supports a centralized configuration file (`gpui.toml`, `gpui.lua.toml`, or `gpui.json`) to declare window geometry, titlebar styles, system backdrops, asset bundling, and build parameters without writing any Rust boilerplate.

---

## Quick Start (`gpui-lua init`)

Generate a starter configuration file and entrypoint script:

```bash
gpui-lua init my-app
```

This creates:
1. `gpui.toml` with default window, build, and development settings.
2. `main.lua` with a working reactive UI.

---

## Configuration Reference

A complete `gpui.toml`:

```toml
[app]
name = "My Desktop App"             # Application internal name
entry = "main.lua"                  # Lua entrypoint script (relative to project root)
version = "1.0.0"                   # Application semantic version string
product_name = "My Desktop App"     # Product name embedded in executable resources
file_description = "Desktop App"    # File description (Windows / macOS / Linux)
company_name = "Acme Corp"          # Company or organization name
copyright = "Copyright (c) 2026"    # Legal copyright statement
identifier = "com.example.myapp"    # Bundle identifier (macOS CFBundleIdentifier, Linux ID)
icon = "assets/icon.ico"            # Application icon (.ico, .icns, or .png)
[window]
title = "My Desktop App"      # Initial window titlebar text
width = 1024                  # Initial window width in pixels (default: 900)
height = 768                  # Initial window height in pixels (default: 650)
min_width = 400               # Minimum width constraint (default: 400)
min_height = 300              # Minimum height constraint (default: 300)
resizable = true              # Allow user window resizing (default: true)
minimizable = true            # Allow user window minimization (default: true)
csd = true                    # Enable custom Client-Side Window Decorations (default: true)
csd_height = 38.0             # Height of custom CSD titlebar in pixels (default: 38)

# OS-dependent window background
[window.background]
windows = "acrylic"           # transparent, mica, mica-alt, acrylic, opaque
macos = "sidebar"             # any NSVisualEffectMaterial (sidebar, titlebar, hud, etc.)
linux = "transparent"         # transparent, opaque

# Alternatively, specify direct platform overrides:
# windows_background = "mica"
# macos_background = "sidebar"
# linux_background = "transparent"
[build]
output = "dist/my_app.exe"    # Output destination for standalone binary
assets = "assets"             # Directory of images, fonts, and audio to embed
template = ""                 # Optional path to custom base executable

[dev]
hot_reload = true             # Enable live file watching on save (default: true)
```

---

## Window Backdrops (`[window.background]`)

You can specify window backdrops universally or per operating system:

```toml
[window.background]
windows = "acrylic"       # Windows 10 & 11 frosted glass
macos = "sidebar"         # macOS vibrancy material
linux = "transparent"     # Linux Wayland/X11 transparency
```

### Supported Values by Platform

| Platform | Values | Description |
| :--- | :--- | :--- |
| **Windows** | `"acrylic"` | Frosted glass translucent blur |
| | `"mica"` | Wallpaper-tinted dynamic backdrop (Windows 11) |
| | `"mica_alt"` / `"mica-alt"` | Tabbed high-contrast Mica variant (Windows 11) |
| | `"transparent"` | Full compositor window transparency |
| | `"opaque"` | Standard solid window background |
| **macOS** | `"sidebar"`, `"titlebar"`, `"hud"`, `"menu"`, `"popover"`, `"sheet"`, `"selection"`, `"tooltip"`, `"dark"`, `"light"`, `"ultra_dark"` | Native `NSVisualEffectMaterial` vibrancy |
| | `"transparent"`, `"opaque"` | Native window transparency / solid |
| **Linux** | `"transparent"`, `"opaque"`, `"blurred"` | Wayland / X11 transparency / blur |
---

## Executable Resources & OS Metadata (`[app]`)

When packaging your application with `gpui-lua build`, GPUI.lua injects OS-native resources directly into the output artifact:

| Field | Windows (`.exe`) | macOS (`.app`) | Linux |
| :--- | :--- | :--- | :--- |
| `product_name` | `ProductName` string in `VS_VERSION_INFO` | `CFBundleName` in `Info.plist` | `Name` in `.desktop` file |
| `file_description` | `FileDescription` string in `VS_VERSION_INFO` | `CFBundleDisplayName` | `Comment` in `.desktop` file |
| `version` | `FileVersion` & `ProductVersion` | `CFBundleVersion` & `CFBundleShortVersionString` | `Version` in `.desktop` file |
| `company_name` | `CompanyName` in `VS_VERSION_INFO` | - | - |
| `copyright` | `LegalCopyright` in `VS_VERSION_INFO` | `NSHumanReadableCopyright` | - |
| `identifier` | App User Model ID / grouping | `CFBundleIdentifier` | Desktop file ID |
| `icon` | Embedded PE Icon (`RT_ICON` / `RT_GROUP_ICON`) | `Contents/Resources/AppIcon.icns` | Desktop launcher icon |

::: tip Cross-Platform Packaging
- On **Windows**: Injects binary `VS_VERSION_INFO` and multi-resolution `.ico` images directly into the Portable Executable (PE) headers using Win32 resource APIs.
- On **macOS**: Automatically creates `{AppName}.app/Contents/` with a valid `Info.plist`, native binary, and `.icns` resource directory.
- On **Linux**: Generates a Freedesktop-compliant `{app_name}.desktop` launcher and sets executable permissions.
:::

## CLI Integration & Overrides

When `gpui.toml` is present in the working directory:

```bash
# 1. Run entrypoint defined in gpui.toml
gpui-lua run

# 2. Live hot reload using window settings from gpui.toml
gpui-lua dev

# 3. Bundle standalone single .exe using [build] settings from gpui.toml
gpui-lua build
```

CLI flags always override configuration file fields:

```bash
# Temporarily override dimensions and backdrop:
gpui-lua run -w 1280 -h 800 --background mica

# Use a custom config file:
gpui-lua run --config custom_config.toml
```
