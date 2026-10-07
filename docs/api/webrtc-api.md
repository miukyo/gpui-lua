# WebRTC P2P API Reference (`webrtc`)

Peer-to-peer data channels, local track streaming, and remote track FFmpeg decoding.

---

## `webrtc.create_peer_connection(config)`

```lua
local peer = webrtc.create_peer_connection({
    ice_servers = {
        { urls = "stun:stun.l.google.com:19302" }
    }
})
```

---

## Tracks & Streaming

### `peer:add_track(source)`
Attaches camera or microphone handle to the peer connection and returns a local track.

```lua
local cam = media.open_camera({ width = 1280, height = 720 })
local video_track = peer:add_track(cam)

local mic = media.open_microphone({ sample_rate = 48000 })
local audio_track = peer:add_track(mic)

print(video_track:id(), video_track:kind(), video_track:src())
```

### `peer:on_track(handler)`
Receives remote audio or video tracks. Video decodes via FFmpeg H.264; audio decodes via FFmpeg Opus.

```lua
peer:on_track(function(track)
    print("Received remote track:", track:kind(), track:id())
    if track:kind() == "video" then
        ui.video({ src = track:src(), autoplay = true })
    elseif track:kind() == "audio" then
        track:play()
    end
end)
```

---

## Data Channels

### `peer:create_data_channel(label, [options])`

```lua
local dc = peer:create_data_channel("chat")

dc:on_open(function()
    print("Channel opened")
    dc:send("Hello peer!")
end)

dc:on_message(function(text)
    print("Received:", text)
end)

dc:on_close(function()
    print("Channel closed")
end)

dc:close()
```

---

## Signaling & Connection Lifecycle

```lua
-- ICE Candidate gathering
peer:on_ice_candidate(function(cand)
    -- Signal cand.candidate, cand.sdpMid, cand.sdpMLineIndex to peer
end)

-- Connection state change ("connecting", "connected", "disconnected", "failed", "closed")
peer:on_connection_state_change(function(state)
    print("Connection state:", state)
end)

-- Create Offer
local offer = peer:create_offer():await()
-- Send offer.type, offer.sdp to remote peer

-- Set Remote Description
peer:set_remote_description({ type = "answer", sdp = remote_sdp }):await()

-- Close Peer Connection
peer:close()
```
