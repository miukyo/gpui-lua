# Automatic Asset & Script Bundling

`gpui.lua` embeds Lua scripts, images, audio, video, and fonts directly into a single self-contained executable. No external files, DLLs, or asset folders required.

---

## Automatic Bundling (No Rust Code Needed)

Run one CLI command:

```bash
gpui-lua build main.lua --assets ./assets -o dist/my_app.exe --title "My App"
```

All files inside `./assets` and `main.lua` are packaged into `my_app.exe`.

### Resolving Embedded Files in Lua

Paths resolve automatically from the embedded bundle in memory:

```lua
-- Required modules load from memory
local config = require("config")

function App()
    return ui.div({
        children = {
            -- Images & icons resolve directly from bundled assets
            ui.img({ src = "assets/logo.png", w = 64, h = 64 }),
            ui.svg({ path = "assets/icon.svg", w = 24, h = 24 }),
            -- Audio & video play from bundled assets
            ui.video({ src = "assets/intro.mp4", autoplay = true })
        }
    })
end
```
---

## Packaging Executable Resources & Icons

`gpui-lua build` automatically sets OS-level binary metadata and application icons:

```bash
gpui-lua build main.lua \
  --assets ./assets \
  --icon ./assets/icon.ico \
  --product-name "My Application" \
  --file-description "High Performance Desktop Tool" \
  --company "Acme Corp" \
  --copyright "Copyright (c) 2026" \
  --app-version "1.0.0" \
  --identifier "com.acme.myapp" \
  -o dist/my_app.exe
```

When run:
- **Windows**: Embeds binary `VS_VERSION_INFO` and multi-resolution icon resources into `.exe`.
- **macOS**: Generates `.app` bundle with `Contents/Info.plist` and `Resources/{icon}.icns`.
- **Linux**: Creates an XDG `{app}.desktop` launcher with icon associations.

## Optional: Compile-Time Embedding in Rust

If using the Rust backend workflow, embed assets using `RustEmbed`:

```rust
use gpui_lua::LuaApp;
use rust_embed::RustEmbed;

#[derive(RustEmbed)]
#[folder = "assets/"]
struct Assets;

fn main() -> anyhow::Result<()> {
    LuaApp::embedded::<Assets>("main.lua")
        .title("My App")
        .run()
}
```
