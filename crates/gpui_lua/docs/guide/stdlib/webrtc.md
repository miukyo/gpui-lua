# WebRTC P2P (`webrtc`)

Peer-to-peer data channels, local media track streaming, and remote track FFmpeg decoding.

```lua
-- 1. Create PeerConnection
local peer = webrtc.create_peer_connection({
    ice_servers = { { urls = "stun:stun.l.google.com:19302" } }
})

-- 2. Stream local camera or microphone to peer
local cam = media.open_camera({ width = 1280, height = 720 })
local cam_track = peer:add_track(cam)

local mic = media.open_microphone({ sample_rate = 48000 })
local mic_track = peer:add_track(mic)

-- 3. Receive incoming remote audio and video tracks
local remote_video_src, set_remote_video_src = signal(nil, "remote_video")

peer:on_track(function(track)
    if track:kind() == "video" then
        set_remote_video_src(track:src())
    elseif track:kind() == "audio" then
        track:play()
    end
end)

-- 4. Bidirectional DataChannel
local dc = peer:create_data_channel("chat")

dc:on_open(function()
    dc:send("Hello from GPUI.lua!")
end)

dc:on_message(function(text)
    print("Received message:", text)
end)

-- 5. Signaling events
peer:on_ice_candidate(function(cand)
    -- Signal cand.candidate, cand.sdpMid, cand.sdpMLineIndex
end)

peer:on_connection_state_change(function(state)
    print("Connection state:", state) -- "connecting", "connected", "closed"
end)

-- 6. Offer / Answer negotiation
local offer = peer:create_offer():await()
peer:set_remote_description({ type = "answer", sdp = remote_sdp }):await()
```
