# Audio Engine (`audio`)

Hardware audio playback powered by CPAL and FFmpeg.

```lua
-- Load audio file without playing
local sound = audio.load("assets/click.wav")
sound:volume(0.8)
sound:play()

-- Play background music immediately
local bgm = audio.play("assets/music.mp3")
bgm:loop(true)
bgm:volume(0.5)

-- Playback controls
bgm:pause()
bgm:seek(15.0) -- Seek to 15 seconds

-- Query state
print("Duration:", bgm:duration())
print("Position:", bgm:position())
print("Playing:", bgm:is_playing())
```
