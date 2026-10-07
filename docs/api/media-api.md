# Hardware Media API Reference (`media`)

Hardware camera and microphone capture methods.

---

## Device Discovery

### `media.list_cameras()`
Returns array of detected hardware camera devices:
`{ id: integer, name: string, default: boolean }`

```lua
local cams = media.list_cameras()
for _, cam in ipairs(cams) do
    print(cam.id, cam.name, cam.default)
end
```

### `media.list_microphones()`
Returns array of detected audio input devices:
`{ id: integer, name: string, default: boolean }`

```lua
local mics = media.list_microphones()
for _, mic in ipairs(mics) do
    print(mic.id, mic.name, mic.default)
end
```

---

## Capture Handles

### `media.open_camera(options)`
Opens webcam capture. Index 0 binds the OS default webcam.

```lua
local cam = media.open_camera({
    device = 0,
    width = 1280,
    height = 720,
    fps = 30
})

-- Methods
print(cam:name())      -- Hardware device name
print(cam:src())       -- Stream URI (e.g. "camera://0")
print(cam:width())     -- Frame width
print(cam:height())    -- Frame height
print(cam:fps())       -- Target framerate
print(cam:is_muted())  -- Boolean mute status

cam:mute(true)         -- Mute video feed
cam:mute(false)
cam:stop()             -- Release hardware camera
```

### `media.open_microphone(options)`
Opens audio input capture. Index 0 binds the OS default microphone.

```lua
local mic = media.open_microphone({
    device = 0,
    sample_rate = 48000,
    channels = 2
})

-- Methods
print(mic:name())      -- Audio device name
print(mic:src())       -- Stream URI (e.g. "microphone://0")
print(mic:is_muted())  -- Boolean mute status

mic:mute(true)
mic:mute(false)
mic:stop()             -- Release hardware microphone
```

---

## Stream Binding

### `media.bind(source)`
Extracts URI string from a camera or microphone handle for element binding.

```lua
local uri = media.bind(cam)
```
