# Lua API Reference

Categorized API reference and code patterns for `gpui.lua`.

---

## 1. CLI Commands

| Command | Usage | Description |
| :--- | :--- | :--- |
| `run` | `gpui-lua run <script.lua>` | Run script directly |
| `dev` | `gpui-lua dev <script.lua>` | Run with live hot reload watcher on save |
| `build` | `gpui-lua build <entry.lua> [options]` | Package script and assets into single `.exe` |
| `version` | `gpui-lua --version` | Display version and engine features |

### Build Options
- `-o, --output <path>`: Destination executable path (default: `<name>.exe`).
- `-a, --assets <dir>`: Static assets folder (images, fonts, audio, video).
- `-t, --title <str>`: Window title.
- `-w, --width <px>`: Initial window width (default `900`).
- `-h, --height <px>`: Initial window height (default `650`).
- `--no-csd`: Disable custom client-side decorations (use native titlebar).
- `--background <val>`: Window backdrop material (`acrylic`, `mica`, `sidebar`, `transparent`, etc.).

```bash
gpui-lua build counter.lua --assets ./assets -o dist/counter.exe --title "Counter"
```

---

## 2. UI Elements

### `ui.div(props)`
Base container supporting layout, styling, and event handlers.

```lua
local el = ui.div({
    w = 300, h = 200,
    bg = "#1e1e2e",
    p = 16, rounded = 8,
    border = 1, border_color = "#313244",
    flex_col = true, gap = 8,
    children = { ... }
})
```

### `ui.text(content)`
Typography element for anti-aliased text.

```lua
local el = ui.text("Hello World")
    :size(16)
    :bold()
    :italic()
    :color("#cdd6f4")
    :line_height(24)
```

### `ui.input(props)`
Single-line text input with selection, clipboard, cursor, masking, and validation.

```lua
local val, set_val = signal("", "name")

local el = ui.input({
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

Password masking:
```lua
local pass = ui.input({
    type = "password",
    mask = "•",
    placeholder = "Password"
})
```

### `ui.textarea(props)`
Multi-line text editor with line wrapping, row count, and max rows limit.

```lua
local bio, set_bio = signal("", "bio")

local el = ui.textarea({
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

### `ui.video(props)`
Hardware-accelerated video player (D3D11VA, VAAPI, VideoToolbox). Accepts file paths, camera objects, or WebRTC stream tracks.

```lua
-- File
ui.video({ src = "clip.mp4", w = 640, h = 360, autoplay = true, loop = true })

-- Live camera
local cam = media.open_camera({ width = 1280, height = 720 })
ui.video({ src = cam, w_full = true, h = 360, autoplay = true })

-- WebRTC remote track
peer:on_track(function(track)
    if track:kind() == "video" then
        ui.video({ src = track:src(), autoplay = true })
    end
end)
```

### `ui.img(props)`
Raster image renderer with texture cache.

```lua
ui.img({ src = "assets/logo.png", w = 48, h = 48, fit = "contain" })
```

### `ui.svg(props)`
Vector SVG element.

```lua
ui.svg({ path = "assets/icon.svg", w = 24, h = 24, color = "#89b4fa" })
```

### `ui.canvas(paint_fn)`
Custom 2D canvas drawing element.

```lua
ui.canvas(function(bounds)
    -- Custom 2D painting commands
end)
```

---

## 3. Layout Containers

### `ui.row(props)`
Horizontal flex container (`flex_row = true`).

```lua
ui.row({
    gap = 8,
    align = "center",
    justify = "between",
    w_full = true,
    children = { ui.text("A"), ui.text("B") }
})
```

### `ui.column(props)`
Vertical flex container (`flex_col = true`).

```lua
ui.column({
    gap = 12,
    p = 16,
    children = { ui.text("Header"), ui.text("Body") }
})
```

### `ui.stack(props)`
Overlay container where children stack along the Z-axis.

```lua
ui.stack({
    w_full = true, h = 200,
    children = {
        ui.div({ bg = "#000000", w_full = true, h_full = true }),
        ui.text("Centered Text"):color("#ffffff")
    }
})
```

---

## 4. Builder Style Methods

All methods return `self` for chaining.

### Sizing & Spacing
- `:w(px)` / `:h(px)`: Fixed width/height in pixels.
- `:w_full()` / `:h_full()`: Width/height 100%.
- `:min_w(px)` / `:max_w(px)`: Min/max width constraints.
- `:min_h(px)` / `:max_h(px)`: Min/max height constraints.
- `:p(px)` / `:px(px)` / `:py(px)`: Padding (all, horizontal, vertical).
- `:m(px)` / `:mx(px)` / `:my(px)`: Margin (all, horizontal, vertical).
- `:gap(px)`: Spacing between children.

### Color & Styling
- `:bg(color)`: Background color (hex or `#rgba`).
- `:color(color)`: Text color.
- `:rounded(px)`: Corner radius.
- `:border(px)`: Border width.
- `:border_color(color)`: Border stroke color.
- `:shadow_sm()`, `:shadow_md()`, `:shadow_lg()`: Hardware drop shadow levels.
- `:opacity(0.0 - 1.0)`: Element opacity.
- `:backdrop_blur(px)`: Frosted glass backdrop blur radius.

### Overflow & Masks
- `:overflow_hidden()`: Clip overflowing child content.
- `:overflow_scroll()`: Enable scrollable container viewport.
- `:overflow_fade_top(px)` / `:overflow_fade_bottom(px)`: Progressive mask attenuation distance.
- `:overflow_fade_left(px)` / `:overflow_fade_right(px)`: Progressive mask horizontal attenuation.

### Flexbox & Alignment
- `:flex_row()`: Horizontal layout.
- `:flex_col()`: Vertical layout.
- `:items_center()` / `:items_start()` / `:items_end()`: Cross-axis alignment.
- `:justify_center()` / `:justify_between()` / `:justify_start()` / `:justify_end()`: Main-axis alignment.

### Interactivity & Events
- `:cursor_pointer()`: Change mouse cursor to pointer on hover.
- `:on_click(function())`: Click event listener.
- `:on_mouse_down(function(e))`: Mouse down callback.
- `:on_mouse_up(function(e))`: Mouse up callback.
- `:on_mouse_move(function(e))`: Mouse move callback.
- `:on_key_down(function(e))`: Key down callback.
- `:on_key_up(function(e))`: Key up callback.
- `:on_scroll_wheel(function(e))`: Scroll wheel callback.
- `:child(el)`: Append single child element.
- `:children(list)`: Append list of children.
- `:rows(n)`: Textarea visible rows count.
- `:max_rows(n)`: Textarea maximum rows cap.

---

## 5. Reactive State

### `signal(initial_val, [key])`
Creates reactive state pair `get, set`. Persists across live hot-reloads when `key` provided.

```lua
local count, set_count = signal(0, "counter")

-- Read
print(count())

-- Write
set_count(count() + 1)
```

### `computed(fn)`
Creates read-only derived state. Automatically recalculates when tracked signals update.

```lua
local count, set_count = signal(5, "num")
local double = computed(function()
    return count() * 2
end)

print(double()) -- 10
```

### `effect(fn)`
Runs side effect automatically whenever any read signal changes.

```lua
effect(function()
    print("State changed:", count())
end)
```

---

## 6. Hardware Media & Devices (`media`)

### Camera
```lua
-- List detected webcams
local cameras = media.list_cameras()
for _, c in ipairs(cameras) do
    print(c.id, c.name, c.default)
end

-- Open webcam (index 0 is OS default)
local cam = media.open_camera({ device = 0, width = 1280, height = 720, fps = 30 })

-- Pass handle directly to ui.video
ui.video({ src = cam, autoplay = true })

-- Methods
cam:mute(true)   -- Mute video (renders black frames)
cam:mute(false)
print(cam:name(), cam:src(), cam:width(), cam:height(), cam:fps(), cam:is_muted())
cam:stop()       -- Release hardware device
```

### Microphone
```lua
-- List detected audio inputs
local mics = media.list_microphones()
for _, m in ipairs(mics) do
    print(m.id, m.name, m.default)
end

-- Open microphone (index 0 is OS default)
local mic = media.open_microphone({ device = 0, sample_rate = 48000, channels = 2 })

-- Methods
mic:mute(true)
mic:mute(false)
print(mic:name(), mic:src(), mic:is_muted())
mic:stop()
```

### Stream Binding
```lua
-- Binds camera or microphone handle to URI string
local uri = media.bind(cam)
```

---

## 7. WebRTC P2P (`webrtc`)

```lua
-- 1. Create PeerConnection
local peer = webrtc.create_peer_connection({
    ice_servers = { { urls = "stun:stun.l.google.com:19302" } }
})

-- 2. Add local camera or mic stream
local cam = media.open_camera({ width = 1280, height = 720 })
local cam_track = peer:add_track(cam)

local mic = media.open_microphone({ sample_rate = 48000 })
local mic_track = peer:add_track(mic)

-- 3. Receive remote tracks (FFmpeg H.264 & Opus decoding)
peer:on_track(function(track)
    if track:kind() == "video" then
        ui.video({ src = track:src(), autoplay = true })
    elseif track:kind() == "audio" then
        track:play()
    end
end)

-- 4. P2P DataChannel
local dc = peer:create_data_channel("chat")
dc:on_open(function() dc:send("Hello peer!") end)
dc:on_message(function(msg) print("Peer says:", msg) end)

-- 5. Signaling events
peer:on_ice_candidate(function(cand)
    -- Send cand.candidate, cand.sdpMid, cand.sdpMLineIndex
end)

peer:on_connection_state_change(function(state)
    print("State:", state) -- "connecting", "connected", "closed"
end)

local offer = peer:create_offer():await()
peer:set_remote_description({ type = "answer", sdp = remote_sdp }):await()
```

---

## 8. Audio Player Engine (`audio`)

```lua
-- Load sound effect
local click = audio.load("assets/click.wav")
click:volume(0.8)
click:play()

-- Play music track immediately
local bgm = audio.play("assets/music.mp3")
bgm:loop(true)
bgm:volume(0.5)

-- Playback controls
bgm:pause()
bgm:seek(15.0) -- Seek to 15 seconds
print(bgm:duration(), bgm:position(), bgm:is_playing())
```

---

## 9. Desktop System & OS

### Filesystem (`fs`)
```lua
fs.write("config.json", '{"theme":"dark"}')
local content = fs.read("config.json")
local exists = fs.exists("config.json")
local list = fs.list_dir("./assets")
fs.create_dir("output/logs")
fs.remove("temp.txt")
```

### JSON (`json`)
```lua
local str = json.encode({ score = 100, tags = { "gpu", "ui" } })
local obj = json.decode(str)
```

### Timers (`timer`)
```lua
local t1 = timer.timeout(1000, function() print("1 second elapsed") end)
local t2 = timer.interval(500, function() print("Tick every 500ms") end)
timer.cancel(t2)
```

### Operating System (`os`)
```lua
local plat = os.platform() -- "windows", "macos", "linux"
local home = os.home_dir()
local appdata = os.appdata_dir()
local win = os.window_info() -- { width, height, scale_factor }

-- Clipboard
os.clipboard_set("Text")
local text = os.clipboard_get()

-- Shell process execution
local res = os.exec("git", { "status", "--short" })
print(res.exit_code, res.stdout)
```

### HTTP Client (`http`)
```lua
http.get("https://httpbin.org/get", function(err, resp)
    if not err and resp.status == 200 then
        print(resp.body)
    end
end)

http.post("https://httpbin.org/post", '{"id":1}', { ["Content-Type"] = "application/json" }, function(err, resp)
    print(resp.body)
end)
```
