# Hardware Media (`media`)

DirectShow, CPAL, and FFmpeg camera and microphone capture.

---

## Webcams

```lua
-- List detected hardware cameras
local cameras = media.list_cameras()
for _, cam in ipairs(cameras) do
    print(cam.id, cam.name, cam.default)
end

-- Open camera (index 0 is OS default)
local cam = media.open_camera({
    device = 0,
    width = 1280,
    height = 720,
    fps = 30
})

-- Pass directly into video element for instant live preview
ui.video({ src = cam, autoplay = true, w = 640, h = 360 })

-- Mute / Unmute video (emits black frames when muted)
cam:mute(true)
cam:mute(false)

-- Stop camera and release hardware device
cam:stop()
```

---

## Microphones

```lua
-- List detected audio input devices
local mics = media.list_microphones()
for _, mic in ipairs(mics) do
    print(mic.id, mic.name, mic.default)
end

-- Open microphone (index 0 is OS default)
local mic = media.open_microphone({
    device = 0,
    sample_rate = 48000,
    channels = 2
})

-- Mute / Unmute
mic:mute(true)
mic:mute(false)

-- Stop microphone
mic:stop()
```

---

## Stream Binding

```lua
-- Extract stream URI string (e.g. "camera://0", "microphone://0")
local uri = media.bind(cam)
```
