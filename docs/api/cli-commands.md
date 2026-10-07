# CLI Commands Reference

Command-line reference for the `gpui-lua` binary.

---

## Commands

```bash
# Run Lua script directly
gpui-lua run <script.lua> [options]

# Run with live hot-reloading file watcher
gpui-lua dev <script.lua> [options]

# Package script and assets into single standalone .exe
gpui-lua build <entry.lua> [options]

# Display engine version
gpui-lua --version

# Display usage help
gpui-lua --help
```

---

## Options & Flags

| Flag | Parameter | Description |
| :--- | :--- | :--- |
| `-o, --output` | `<path>` | Output destination for `build` (default: `<entry_stem>.exe`) |
| `-a, --assets` | `<dir>` | Static assets folder to embed alongside script |
| `-t, --title` | `<string>` | Initial window title |
| `-w, --width` | `<number>` | Initial window width in pixels (default: `900`) |
| `-h, --height` | `<number>` | Initial window height in pixels (default: `650`) |
| `--no-csd` | - | Disable Client-Side Decorations (use OS native borders) |
| `--csd-height` | `<number>` | Height of CSD titlebar in pixels (default: `38`) |
| `--background` | `<value>` | Universal window backdrop effect |
| `--windows-background` | `<value>` | Windows background (`acrylic`, `mica`, `mica-alt`, `opaque`, `transparent`) |
| `--macos-background` | `<value>` | macOS background (`sidebar`, `hud`, `titlebar`, `transparent`, etc.) |
| `--linux-background` | `<value>` | Linux background (`transparent`, `opaque`) |
| `--icon` | `<path>` | Application icon (`.ico`, `.icns`, or `.png`) |
| `--product-name` | `<string>` | Product name embedded in OS resources |
| `--file-description` | `<string>` | File description resource |
| `--company` | `<string>` | Company / organization resource |
| `--copyright` | `<string>` | Legal copyright resource |
| `--app-version` | `<version>` | Application semantic version string |
| `--identifier` | `<id>` | App bundle / desktop identifier (e.g. `com.example.app`) |
---

## Examples

```bash
# Rapid development with hot reloading
gpui-lua dev counter.lua

# Build self-contained release executable
gpui-lua build counter.lua -o dist/counter.exe --title "Counter Demo"

# Build full app with images and fonts
gpui-lua build main.lua --assets ./assets -o dist/my_app.exe
```
