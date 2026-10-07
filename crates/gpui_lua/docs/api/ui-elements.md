# UI Elements Reference

Comprehensive reference for all core elements in `ui.*`.

---

## `ui.div(props)`
Base container element supporting flex layout, styling, border, background, and events.

```lua
ui.div({
    w = 300, h = 200,
    bg = "#1e1e2e",
    p = 16, rounded = 8,
    border = 1, border_color = "#313244",
    flex_col = true, gap = 8,
    children = { ... }
})
```

---

## `ui.text(content)`
Typography element for anti-aliased text.

```lua
ui.text("Hello World")
    :size(16)
    :bold()
    :italic()
    :color("#cdd6f4")
    :line_height(24)
```

---

## `ui.input(props)`
Single-line text input with selection, clipboard, cursor blinking, masking, and validation.

```lua
local val, set_val = signal("", "username")

ui.input({
    id = "user_input",
    value = val(),
    placeholder = "Enter username...",
    placeholder_color = "#6c7086",
    max_length = 32,
    pattern = "^[a-zA-Z0-9_]+$",
    error_message = "Only alphanumeric allowed",
    bg = "#181825",
    focus_bg = "#1e1e2e",
    border = 1,
    border_color = "#45475a",
    focus_border_color = "#89b4fa",
    error_border_color = "#ef4444",
    text_color = "#cdd6f4",
    cursor_color = "#89b4fa",
    selection_bg = "#313244",
    rounded = 6,
    px = 12, py = 8,
    font_size = 14,
    on_change = function(v) set_val(v) end,
    on_submit = function(v) print("Submit:", v) end,
    on_validate = function(ok, err) print(ok, err) end,
})
```

Password input:
```lua
ui.input({
    type = "password",
    mask = "•",
    placeholder = "Password"
})
```

---

## `ui.textarea(props)`
Multi-line text editor with line wrapping, selection, and row caps.

```lua
local bio, set_bio = signal("", "bio")

ui.textarea({
    id = "user_bio",
    value = bio(),
    placeholder = "Write biography...",
    rows = 4,        -- Initial visible lines
    max_rows = 8,    -- Hard ceiling on lines
    bg = "#181825",
    border = 1,
    border_color = "#45475a",
    text_color = "#cdd6f4",
    px = 12, py = 8,
    on_change = function(v) set_bio(v) end,
})
```

---

## `ui.video(props)`
Hardware-accelerated video player (D3D11VA, VAAPI, VideoToolbox). Accepts file paths, camera objects, or WebRTC stream tracks.

```lua
-- File
ui.video({ src = "assets/clip.mp4", w = 640, h = 360, autoplay = true, loop = true })

-- Live camera capture
local cam = media.open_camera({ width = 1280, height = 720 })
ui.video({ src = cam, w_full = true, h = 360, autoplay = true })

-- Remote WebRTC track
peer:on_track(function(track)
    if track:kind() == "video" then
        ui.video({ src = track:src(), autoplay = true })
    end
end)
```

---

## `ui.img(props)`
Raster image renderer with GPU texture caching.

```lua
ui.img({ src = "assets/logo.png", w = 48, h = 48, fit = "contain" })
```

---

## `ui.svg(props)`
Vector SVG element with color tinting.

```lua
ui.svg({ path = "assets/icon.svg", w = 24, h = 24, color = "#89b4fa" })
```

---

## `ui.canvas(paint_fn)`
Custom 2D canvas drawing element.

```lua
ui.canvas(function(bounds)
    -- Custom 2D painting commands
end)
```
