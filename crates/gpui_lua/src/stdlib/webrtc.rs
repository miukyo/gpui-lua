use crate::dsl::node::SyncRegistryKey;
use crate::reactive::bridge::ReactiveBridge;
use mlua::{Function, Lua, Result, Table, UserData, UserDataMethods, Value};
use parking_lot::Mutex;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use webrtc::data_channel::{DataChannel, DataChannelEvent};
use webrtc::media_stream::track_local::static_rtp::TrackLocalStaticRTP;
use webrtc::media_stream::track_local::TrackLocal;
use webrtc::media_stream::track_remote::{TrackRemote, TrackRemoteEvent};
use webrtc::peer_connection::{
    PeerConnection, PeerConnectionBuilder, PeerConnectionEventHandler,
    RTCConfigurationBuilder, RTCIceCandidateInit, RTCIceServer,
    RTCPeerConnectionIceEvent, RTCPeerConnectionState, RTCSessionDescription,
};
use rtc::media_stream::MediaStreamTrack;
use rtc::rtp_transceiver::rtp_sender::RtpCodecKind;

#[cfg(feature = "media")]
use super::media::{LuaCameraCapture, LuaMicrophoneCapture};

static NEXT_PEER_ID: AtomicU64 = AtomicU64::new(1);
static NEXT_CHANNEL_ID: AtomicU64 = AtomicU64::new(1);
static NEXT_TRACK_ID: AtomicU64 = AtomicU64::new(1);

#[cfg(feature = "media")]
static MEDIA_MANAGERS: parking_lot::RwLock<Option<(media::VideoManager, media::AudioManager)>> =
    parking_lot::RwLock::new(None);

#[cfg(feature = "media")]
pub fn set_media_managers(video: media::VideoManager, audio: media::AudioManager) {
    *MEDIA_MANAGERS.write() = Some((video, audio));
}

#[cfg(feature = "media")]
pub fn get_media_managers() -> Option<(media::VideoManager, media::AudioManager)> {
    MEDIA_MANAGERS.read().clone()
}

#[derive(Clone)]
pub struct LuaDataChannel {
    pub id: u64,
    pub channel: Arc<dyn DataChannel>,
    pub on_message: Arc<Mutex<Option<SyncRegistryKey>>>,
    pub on_open: Arc<Mutex<Option<SyncRegistryKey>>>,
    pub on_close: Arc<Mutex<Option<SyncRegistryKey>>>,
}

impl UserData for LuaDataChannel {
    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("send", |_lua, this, data: Value| {
            let text = match data {
                Value::String(s) => s.to_str().unwrap_or_default().to_string(),
                Value::Table(t) => {
                    let json_val = crate::stdlib::json::lua_value_to_json(Value::Table(t))?;
                    serde_json::to_string(&json_val).unwrap_or_default()
                }
                _ => format!("{data:?}"),
            };

            let ch = this.channel.clone();
            crate::tokio_runtime().spawn(async move {
                let _ = ch.send_text(&text).await;
            });
            Ok(())
        });

        methods.add_method("close", |_lua, this, ()| {
            let ch = this.channel.clone();
            crate::tokio_runtime().spawn(async move {
                let _ = ch.close().await;
            });
            Ok(())
        });

        methods.add_method("on_message", |lua, this, handler: Function| {
            let key = SyncRegistryKey::new(lua.create_registry_value(handler)?);
            *this.on_message.lock() = Some(key);
            Ok(())
        });

        methods.add_method("on_open", |lua, this, handler: Function| {
            let key = SyncRegistryKey::new(lua.create_registry_value(handler)?);
            *this.on_open.lock() = Some(key);
            Ok(())
        });

        methods.add_method("on_close", |lua, this, handler: Function| {
            let key = SyncRegistryKey::new(lua.create_registry_value(handler)?);
            *this.on_close.lock() = Some(key);
            Ok(())
        });
    }
}

fn spawn_data_channel_poller(
    channel: Arc<dyn DataChannel>,
    on_message: Arc<Mutex<Option<SyncRegistryKey>>>,
    on_open: Arc<Mutex<Option<SyncRegistryKey>>>,
    on_close: Arc<Mutex<Option<SyncRegistryKey>>>,
    lua_ref: Arc<parking_lot::RwLock<Option<Arc<Mutex<Lua>>>>>,
    bridge: ReactiveBridge,
) {
    crate::tokio_runtime().spawn(async move {
        while let Some(evt) = channel.poll().await {
            match evt {
                DataChannelEvent::OnOpen => {
                    let k = on_open.lock().clone();
                    if let Some(key) = k {
                        if let Some(lua_arc) = lua_ref.read().clone() {
                            let lua = lua_arc.lock();
                            if let Ok(func) = key.with(|k_ref| lua.registry_value::<Function>(k_ref)) {
                                let _ = func.call::<()>(());
                            }
                        }
                        bridge.notify();
                    }
                }
                DataChannelEvent::OnMessage(msg) => {
                    let text = String::from_utf8_lossy(&msg.data).to_string();
                    let k = on_message.lock().clone();
                    if let Some(key) = k {
                        if let Some(lua_arc) = lua_ref.read().clone() {
                            let lua = lua_arc.lock();
                            if let Ok(func) = key.with(|k_ref| lua.registry_value::<Function>(k_ref)) {
                                let _ = func.call::<()>(Value::String(lua.create_string(&text)));
                            }
                        }
                        bridge.notify();
                    }
                }
                DataChannelEvent::OnClose => {
                    let k = on_close.lock().clone();
                    if let Some(key) = k {
                        if let Some(lua_arc) = lua_ref.read().clone() {
                            let lua = lua_arc.lock();
                            if let Ok(func) = key.with(|k_ref| lua.registry_value::<Function>(k_ref)) {
                                let _ = func.call::<()>(());
                            }
                        }
                        bridge.notify();
                    }
                    break;
                }
                _ => {}
            }
        }
    });
}

/// Remote WebRTC track (Audio or Video) received from a peer.
#[derive(Clone)]
pub struct LuaRemoteTrack {
    pub id: String,
    pub kind: String,
    pub uri: String,
    pub track: Arc<dyn TrackRemote>,
}

impl UserData for LuaRemoteTrack {
    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("id", |_lua, this, ()| Ok(this.id.clone()));
        methods.add_method("kind", |_lua, this, ()| Ok(this.kind.clone()));
        methods.add_method("src", |_lua, this, ()| Ok(this.uri.clone()));
        methods.add_method("uri", |_lua, this, ()| Ok(this.uri.clone()));

        // track:play() returns the stream URI
        methods.add_method("play", |_lua, this, ()| Ok(this.uri.clone()));
    }
}

/// Local WebRTC track (Camera or Microphone) created to send media to a peer.
#[derive(Clone)]
pub struct LuaLocalTrack {
    pub id: String,
    pub kind: String,
    pub uri: String,
    pub track: Arc<TrackLocalStaticRTP>,
    pub running: Arc<AtomicBool>,
}

impl UserData for LuaLocalTrack {
    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("id", |_lua, this, ()| Ok(this.id.clone()));
        methods.add_method("kind", |_lua, this, ()| Ok(this.kind.clone()));
        methods.add_method("src", |_lua, this, ()| Ok(this.uri.clone()));
        methods.add_method("uri", |_lua, this, ()| Ok(this.uri.clone()));
        methods.add_method("stop", |_lua, this, ()| {
            this.running.store(false, Ordering::Relaxed);
            Ok(())
        });
    }
}

struct LuaPeerConnectionHandler {
    on_ice_candidate: Arc<Mutex<Option<SyncRegistryKey>>>,
    on_connection_state: Arc<Mutex<Option<SyncRegistryKey>>>,
    on_data_channel: Arc<Mutex<Option<SyncRegistryKey>>>,
    on_track: Arc<Mutex<Option<SyncRegistryKey>>>,
    channels: Arc<Mutex<HashMap<u64, Arc<LuaDataChannel>>>>,
    tracks: Arc<Mutex<HashMap<String, Arc<LuaRemoteTrack>>>>,
    lua_ref: Arc<parking_lot::RwLock<Option<Arc<Mutex<Lua>>>>>,
    bridge: ReactiveBridge,
    #[cfg(feature = "media")]
    video: Option<media::VideoManager>,
    #[cfg(feature = "media")]
    audio: Option<media::AudioManager>,
}

#[async_trait::async_trait]
impl PeerConnectionEventHandler for LuaPeerConnectionHandler {
    async fn on_ice_candidate(&self, event: RTCPeerConnectionIceEvent) {
        if let Ok(init) = event.candidate.to_json() {
            let key_opt = self.on_ice_candidate.lock().clone();
            if let Some(key) = key_opt {
                if let Some(lua_arc) = self.lua_ref.read().clone() {
                    let lua = lua_arc.lock();
                    if let Ok(func) = key.with(|k| lua.registry_value::<Function>(k)) {
                        let tbl = lua.create_table();
                        let _ = tbl.set("candidate", init.candidate);
                        let _ = tbl.set("sdpMid", init.sdp_mid);
                        let _ = tbl.set("sdpMLineIndex", init.sdp_mline_index);
                        let _ = func.call::<()>(Value::Table(tbl));
                    }
                }
                self.bridge.notify();
            }
        }
    }

    async fn on_connection_state_change(&self, state: RTCPeerConnectionState) {
        let state_str = match state {
            RTCPeerConnectionState::New => "new",
            RTCPeerConnectionState::Connecting => "connecting",
            RTCPeerConnectionState::Connected => "connected",
            RTCPeerConnectionState::Disconnected => "disconnected",
            RTCPeerConnectionState::Failed => "failed",
            RTCPeerConnectionState::Closed => "closed",
            _ => "unknown",
        };
        let key_opt = self.on_connection_state.lock().clone();
        if let Some(key) = key_opt {
            if let Some(lua_arc) = self.lua_ref.read().clone() {
                let lua = lua_arc.lock();
                if let Ok(func) = key.with(|k| lua.registry_value::<Function>(k)) {
                    let _ = func.call::<()>(state_str);
                }
            }
            self.bridge.notify();
        }
    }

    async fn on_data_channel(&self, data_channel: Arc<dyn DataChannel>) {
        let ch_id = NEXT_CHANNEL_ID.fetch_add(1, Ordering::SeqCst);
        let on_message = Arc::new(Mutex::new(None));
        let on_open = Arc::new(Mutex::new(None));
        let on_close = Arc::new(Mutex::new(None));

        let lua_dc = Arc::new(LuaDataChannel {
            id: ch_id,
            channel: data_channel.clone(),
            on_message: on_message.clone(),
            on_open: on_open.clone(),
            on_close: on_close.clone(),
        });

        spawn_data_channel_poller(
            data_channel,
            on_message.clone(),
            on_open.clone(),
            on_close.clone(),
            self.lua_ref.clone(),
            self.bridge.clone(),
        );

        self.channels.lock().insert(ch_id, lua_dc.clone());

        let key_opt = self.on_data_channel.lock().clone();
        if let Some(key) = key_opt {
            if let Some(lua_arc) = self.lua_ref.read().clone() {
                let lua = lua_arc.lock();
                if let Ok(func) = key.with(|k| lua.registry_value::<Function>(k)) {
                    let _ = func.call::<()>(lua_dc.as_ref().clone());
                }
            }
            self.bridge.notify();
        }
    }

    async fn on_track(&self, track: Arc<dyn TrackRemote>) {
        let track_id = track.track_id().await.to_string();
        let is_video = track.kind().await == RtpCodecKind::Video;
        let kind = if is_video { "video" } else { "audio" };
        let uri = format!("webrtc://track/{track_id}");

        #[cfg(feature = "media")]
        {
            if is_video {
                if let Some(v_mgr) = &self.video {
                    let player = v_mgr.register_stream_player(&uri);
                    let track_clone = track.clone();
                    let bridge_clone = self.bridge.clone();
                    let player_clone = player.clone();

                    crate::tokio_runtime().spawn(async move {
                        let mut decoder = media::FfmpegPacketVideoDecoder::new_h264().ok();
                        let mut frames_out = Vec::new();
                        while let Some(event) = track_clone.poll().await {
                            match event {
                                TrackRemoteEvent::OnRtpPacket(pkt) => {
                                    let payload = pkt.payload;
                                    if payload.is_empty() {
                                        continue;
                                    }
                                    if let Some(dec) = decoder.as_mut() {
                                        frames_out.clear();
                                        if dec.decode_packet(&payload, &mut frames_out).is_ok() {
                                            for (w, h, data) in frames_out.drain(..) {
                                                player_clone.push_bgra_frame(w, h, data);
                                                bridge_clone.notify();
                                            }
                                        }
                                    }
                                }
                                TrackRemoteEvent::OnEnded => break,
                                _ => {}
                            }
                        }
                    });
                }
            } else {
                if let Some(a_mgr) = &self.audio {
                    let player = a_mgr.register_stream_player(&uri);
                    let track_clone = track.clone();
                    let player_clone = player.clone();

                    crate::tokio_runtime().spawn(async move {
                        let mut decoder = media::FfmpegPacketAudioDecoder::new_opus().ok();
                        let mut samples_out = Vec::new();
                        while let Some(event) = track_clone.poll().await {
                            match event {
                                TrackRemoteEvent::OnRtpPacket(pkt) => {
                                    let payload = pkt.payload;
                                    if payload.is_empty() {
                                        continue;
                                    }
                                    if let Some(dec) = decoder.as_mut() {
                                        samples_out.clear();
                                        if dec.decode_packet(&payload, &mut samples_out).is_ok() {
                                            player_clone.push_samples(&samples_out);
                                        }
                                    }
                                }
                                TrackRemoteEvent::OnEnded => break,
                                _ => {}
                            }
                        }
                    });
                }
            }
        }

        let remote_track = Arc::new(LuaRemoteTrack {
            id: track_id.clone(),
            kind: kind.to_string(),
            uri: uri.clone(),
            track: track.clone(),
        });

        self.tracks.lock().insert(track_id, remote_track.clone());

        let key_opt = self.on_track.lock().clone();
        if let Some(key) = key_opt {
            if let Some(lua_arc) = self.lua_ref.read().clone() {
                let lua = lua_arc.lock();
                if let Ok(func) = key.with(|k| lua.registry_value::<Function>(k)) {
                    let _ = func.call::<()>(remote_track.as_ref().clone());
                }
            }
            self.bridge.notify();
        }
    }
}

#[derive(Clone)]
pub struct LuaPeerConnection {
    pub id: u64,
    pub pc: Arc<dyn PeerConnection>,
    pub on_ice_candidate: Arc<Mutex<Option<SyncRegistryKey>>>,
    pub on_connection_state: Arc<Mutex<Option<SyncRegistryKey>>>,
    pub on_data_channel: Arc<Mutex<Option<SyncRegistryKey>>>,
    pub on_track: Arc<Mutex<Option<SyncRegistryKey>>>,
    pub channels: Arc<Mutex<HashMap<u64, Arc<LuaDataChannel>>>>,
    pub tracks: Arc<Mutex<HashMap<String, Arc<LuaRemoteTrack>>>>,
    pub lua_ref: Arc<parking_lot::RwLock<Option<Arc<Mutex<Lua>>>>>,
    pub bridge: ReactiveBridge,
}

impl UserData for LuaPeerConnection {
    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        // peer:create_offer([options]) -> promise
        methods.add_method("create_offer", |lua, this, _opts: Option<Table>| {
            let pc = this.pc.clone();
            let lua_ref = this.lua_ref.clone();
            let bridge = this.bridge.clone();

            let create_promise: Function = lua.globals().get("__create_promise")?;
            let promise_tuple: (Table, Function, Function) = create_promise.call(())?;
            let (promise, resolve, reject) = promise_tuple;

            let res_key = SyncRegistryKey::new(lua.create_registry_value(resolve)?);
            let rej_key = SyncRegistryKey::new(lua.create_registry_value(reject)?);

            crate::tokio_runtime().spawn(async move {
                match pc.create_offer(None).await {
                    Ok(offer) => {
                        let _ = pc.set_local_description(offer.clone()).await;
                        if let Some(lua_arc) = lua_ref.read().clone() {
                            let lua = lua_arc.lock();
                            if let Ok(func) = res_key.with(|k| lua.registry_value::<Function>(k)) {
                                let desc_tbl = lua.create_table();
                                let _ = desc_tbl.set("type", offer.sdp_type.to_string());
                                let _ = desc_tbl.set("sdp", offer.sdp);
                                let _ = func.call::<()>(Value::Table(desc_tbl));
                            }
                        }
                    }
                    Err(e) => {
                        if let Some(lua_arc) = lua_ref.read().clone() {
                            let lua = lua_arc.lock();
                            if let Ok(func) = rej_key.with(|k| lua.registry_value::<Function>(k)) {
                                let _ = func.call::<()>(Value::String(lua.create_string(&e.to_string())));
                            }
                        }
                    }
                }
                bridge.notify();
            });

            Ok(promise)
        });

        // peer:create_answer() -> promise
        methods.add_method("create_answer", |lua, this, ()| {
            let pc = this.pc.clone();
            let lua_ref = this.lua_ref.clone();
            let bridge = this.bridge.clone();

            let create_promise: Function = lua.globals().get("__create_promise")?;
            let promise_tuple: (Table, Function, Function) = create_promise.call(())?;
            let (promise, resolve, reject) = promise_tuple;

            let res_key = SyncRegistryKey::new(lua.create_registry_value(resolve)?);
            let rej_key = SyncRegistryKey::new(lua.create_registry_value(reject)?);

            crate::tokio_runtime().spawn(async move {
                match pc.create_answer(None).await {
                    Ok(answer) => {
                        let _ = pc.set_local_description(answer.clone()).await;
                        if let Some(lua_arc) = lua_ref.read().clone() {
                            let lua = lua_arc.lock();
                            if let Ok(func) = res_key.with(|k| lua.registry_value::<Function>(k)) {
                                let desc_tbl = lua.create_table();
                                let _ = desc_tbl.set("type", answer.sdp_type.to_string());
                                let _ = desc_tbl.set("sdp", answer.sdp);
                                let _ = func.call::<()>(Value::Table(desc_tbl));
                            }
                        }
                    }
                    Err(e) => {
                        if let Some(lua_arc) = lua_ref.read().clone() {
                            let lua = lua_arc.lock();
                            if let Ok(func) = rej_key.with(|k| lua.registry_value::<Function>(k)) {
                                let _ = func.call::<()>(Value::String(lua.create_string(&e.to_string())));
                            }
                        }
                    }
                }
                bridge.notify();
            });

            Ok(promise)
        });

        // peer:set_remote_description(desc_tbl) -> promise
        methods.add_method("set_remote_description", |lua, this, desc: Table| {
            let sdp: String = desc.get("sdp").unwrap_or_default();
            let typ: String = desc.get("type").unwrap_or_else(|_| "offer".to_string());
            let pc = this.pc.clone();
            let lua_ref = this.lua_ref.clone();
            let bridge = this.bridge.clone();

            let create_promise: Function = lua.globals().get("__create_promise")?;
            let promise_tuple: (Table, Function, Function) = create_promise.call(())?;
            let (promise, resolve, reject) = promise_tuple;

            let res_key = SyncRegistryKey::new(lua.create_registry_value(resolve)?);
            let rej_key = SyncRegistryKey::new(lua.create_registry_value(reject)?);

            crate::tokio_runtime().spawn(async move {
                let desc_init = match typ.as_str() {
                    "answer" => RTCSessionDescription::answer(sdp),
                    _ => RTCSessionDescription::offer(sdp),
                };
                match desc_init {
                    Ok(session_desc) => match pc.set_remote_description(session_desc).await {
                        Ok(()) => {
                            if let Some(lua_arc) = lua_ref.read().clone() {
                                let lua = lua_arc.lock();
                                if let Ok(func) = res_key.with(|k| lua.registry_value::<Function>(k)) {
                                    let _ = func.call::<()>(Value::Boolean(true));
                                }
                            }
                        }
                        Err(e) => {
                            if let Some(lua_arc) = lua_ref.read().clone() {
                                let lua = lua_arc.lock();
                                if let Ok(func) = rej_key.with(|k| lua.registry_value::<Function>(k)) {
                                    let _ = func.call::<()>(Value::String(lua.create_string(&e.to_string())));
                                }
                            }
                        }
                    },
                    Err(e) => {
                        if let Some(lua_arc) = lua_ref.read().clone() {
                            let lua = lua_arc.lock();
                            if let Ok(func) = rej_key.with(|k| lua.registry_value::<Function>(k)) {
                                let _ = func.call::<()>(Value::String(lua.create_string(&e.to_string())));
                            }
                        }
                    }
                }
                bridge.notify();
            });

            Ok(promise)
        });

        // peer:add_ice_candidate(candidate_tbl)
        methods.add_method("add_ice_candidate", |_lua, this, candidate_tbl: Table| {
            let candidate: String = candidate_tbl.get("candidate").unwrap_or_default();
            let sdp_mid: Option<String> = candidate_tbl.get("sdpMid").ok();
            let sdp_mline_index: Option<u16> = candidate_tbl.get("sdpMLineIndex").ok();

            let pc = this.pc.clone();
            crate::tokio_runtime().spawn(async move {
                let init = RTCIceCandidateInit {
                    candidate,
                    sdp_mid,
                    sdp_mline_index,
                    ..Default::default()
                };
                let _ = pc.add_ice_candidate(init).await;
            });

            Ok(())
        });

        // peer:create_data_channel(label, [opts]) -> LuaDataChannel
        methods.add_method(
            "create_data_channel",
            |_lua, this, (label, _opts): (String, Option<Table>)| {
                let pc = this.pc.clone();
                let ch_id = NEXT_CHANNEL_ID.fetch_add(1, Ordering::SeqCst);
                let on_message = Arc::new(Mutex::new(None));
                let on_open = Arc::new(Mutex::new(None));
                let on_close = Arc::new(Mutex::new(None));

                let (tx, rx) = tokio::sync::oneshot::channel();
                let label_clone = label.clone();
                crate::tokio_runtime().spawn(async move {
                    let ch = pc.create_data_channel(&label_clone, None).await;
                    let _ = tx.send(ch);
                });

                let channel = futures::executor::block_on(rx)
                    .map_err(|e| mlua::Error::RuntimeError(format!("Channel receive error: {e}")))?
                    .map_err(|e| mlua::Error::RuntimeError(format!("DataChannel error: {e}")))?;

                spawn_data_channel_poller(
                    channel.clone(),
                    on_message.clone(),
                    on_open.clone(),
                    on_close.clone(),
                    this.lua_ref.clone(),
                    this.bridge.clone(),
                );

                let dc = LuaDataChannel {
                    id: ch_id,
                    channel,
                    on_message,
                    on_open,
                    on_close,
                };

                this.channels.lock().insert(ch_id, Arc::new(dc.clone()));

                Ok(dc)
            },
        );

        // peer:add_track(source) -> adds camera or mic stream to WebRTC
        methods.add_method("add_track", |_lua, this, source: Value| {
            this.add_track_internal(source)
        });

        // peer:on_track(fn)
        methods.add_method("on_track", |lua, this, handler: Function| {
            let key = SyncRegistryKey::new(lua.create_registry_value(handler)?);
            *this.on_track.lock() = Some(key);
            Ok(())
        });

        // peer:on_ice_candidate(fn)
        methods.add_method("on_ice_candidate", |lua, this, handler: Function| {
            let key = SyncRegistryKey::new(lua.create_registry_value(handler)?);
            *this.on_ice_candidate.lock() = Some(key);
            Ok(())
        });

        // peer:on_connection_state_change(fn)
        methods.add_method(
            "on_connection_state_change",
            |lua, this, handler: Function| {
                let key = SyncRegistryKey::new(lua.create_registry_value(handler)?);
                *this.on_connection_state.lock() = Some(key);
                Ok(())
            },
        );

        // peer:on_data_channel(fn)
        methods.add_method("on_data_channel", |lua, this, handler: Function| {
            let key = SyncRegistryKey::new(lua.create_registry_value(handler)?);
            *this.on_data_channel.lock() = Some(key);
            Ok(())
        });

        // peer:close()
        methods.add_method("close", |_lua, this, ()| {
            let pc = this.pc.clone();
            crate::tokio_runtime().spawn(async move {
                let _ = pc.close().await;
            });
            Ok(())
        });
    }
}

impl LuaPeerConnection {
    fn add_track_internal(&self, source: Value) -> Result<LuaLocalTrack> {
        let pc = self.pc.clone();
        let track_id = format!("track_{}", NEXT_TRACK_ID.fetch_add(1, Ordering::SeqCst));

        #[cfg(feature = "media")]
        if let Value::UserData(ud) = &source {
            if let Ok(cam) = ud.borrow::<LuaCameraCapture>() {
                let track_meta = MediaStreamTrack::new(
                    "webrtc-stream".to_string(),
                    track_id.clone(),
                    "video".to_string(),
                    RtpCodecKind::Video,
                    vec![],
                );
                let track = Arc::new(TrackLocalStaticRTP::new(track_meta));
                let track_clone = track.clone();
                let pc_clone = pc.clone();

                crate::tokio_runtime().spawn(async move {
                    let _ = pc_clone.add_track(track_clone as Arc<dyn TrackLocal>).await;
                });

                let track_writer = track.clone();
                let running = Arc::new(AtomicBool::new(true));
                let running_cb = Arc::clone(&running);
                let seq_counter = Arc::new(AtomicU64::new(0));
                let seq_cb = Arc::clone(&seq_counter);

                cam.inner.add_frame_listener(Arc::new(move |_w, _h, rgba_data| {
                    if !running_cb.load(Ordering::Relaxed) {
                        return;
                    }
                    let track_w = track_writer.clone();
                    let seq_num = seq_cb.fetch_add(1, Ordering::Relaxed) as u16;
                    let payload = bytes::Bytes::copy_from_slice(rgba_data);
                    let packet = rtc::rtp::Packet {
                        header: rtc::rtp::Header {
                            payload_type: 96,
                            sequence_number: seq_num,
                            timestamp: (std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_millis() as u32).wrapping_mul(90),
                            ssrc: 12345,
                            marker: true,
                            ..Default::default()
                        },
                        payload,
                    };
                    crate::tokio_runtime().spawn(async move {
                        let _ = track_w.write_rtp(packet).await;
                    });
                }));

                return Ok(LuaLocalTrack {
                    id: track_id.clone(),
                    kind: "video".to_string(),
                    uri: format!("webrtc://track/{track_id}"),
                    track,
                    running,
                });
            } else if let Ok(mic) = ud.borrow::<LuaMicrophoneCapture>() {
                let track_meta = MediaStreamTrack::new(
                    "webrtc-stream".to_string(),
                    track_id.clone(),
                    "audio".to_string(),
                    RtpCodecKind::Audio,
                    vec![],
                );
                let track = Arc::new(TrackLocalStaticRTP::new(track_meta));
                let track_clone = track.clone();
                let pc_clone = pc.clone();

                crate::tokio_runtime().spawn(async move {
                    let _ = pc_clone.add_track(track_clone as Arc<dyn TrackLocal>).await;
                });

                let track_writer = track.clone();
                let running = Arc::new(AtomicBool::new(true));
                let running_cb = Arc::clone(&running);
                let seq_counter = Arc::new(AtomicU64::new(0));
                let seq_cb = Arc::clone(&seq_counter);

                mic.inner.add_samples_listener(Arc::new(move |samples| {
                    if !running_cb.load(Ordering::Relaxed) {
                        return;
                    }
                    let track_w = track_writer.clone();
                    let seq_num = seq_cb.fetch_add(1, Ordering::Relaxed) as u16;
                    let mut byte_vec = Vec::with_capacity(samples.len() * 4);
                    for &s in samples {
                        byte_vec.extend_from_slice(&s.to_le_bytes());
                    }
                    let payload = bytes::Bytes::from(byte_vec);
                    let packet = rtc::rtp::Packet {
                        header: rtc::rtp::Header {
                            payload_type: 111,
                            sequence_number: seq_num,
                            timestamp: (std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_millis() as u32).wrapping_mul(48),
                            ssrc: 54321,
                            marker: true,
                            ..Default::default()
                        },
                        payload,
                    };
                    crate::tokio_runtime().spawn(async move {
                        let _ = track_w.write_rtp(packet).await;
                    });
                }));

                return Ok(LuaLocalTrack {
                    id: track_id.clone(),
                    kind: "audio".to_string(),
                    uri: format!("webrtc://track/{track_id}"),
                    track,
                    running,
                });
            }
        }

        Err(mlua::Error::RuntimeError("Unsupported media source for add_track. Pass camera or microphone.".to_string()))
    }
}

pub fn register(
    lua: &Lua,
    lua_ref: Arc<parking_lot::RwLock<Option<Arc<Mutex<Lua>>>>>,
    bridge: ReactiveBridge,
    #[cfg(feature = "media")] video: Option<media::VideoManager>,
    #[cfg(feature = "media")] audio: Option<media::AudioManager>,
) -> Result<()> {
    let webrtc_tbl = lua.create_table();

    let p_lua = lua_ref.clone();
    let p_bridge = bridge.clone();

    #[cfg(feature = "media")]
    let v_mgr = video.or_else(|| get_media_managers().map(|(v, _)| v));
    #[cfg(feature = "media")]
    let a_mgr = audio.or_else(|| get_media_managers().map(|(_, a)| a));
    // webrtc.create_peer_connection([config])
    webrtc_tbl.set(
        "create_peer_connection",
        lua.create_function(move |_lua, config_tbl: Option<Table>| {
            let peer_id = NEXT_PEER_ID.fetch_add(1, Ordering::SeqCst);
            let on_ice_candidate: Arc<Mutex<Option<SyncRegistryKey>>> = Arc::new(Mutex::new(None));
            let on_connection_state: Arc<Mutex<Option<SyncRegistryKey>>> = Arc::new(Mutex::new(None));
            let on_data_channel: Arc<Mutex<Option<SyncRegistryKey>>> = Arc::new(Mutex::new(None));
            let on_track: Arc<Mutex<Option<SyncRegistryKey>>> = Arc::new(Mutex::new(None));
            let channels = Arc::new(Mutex::new(HashMap::new()));
            let tracks = Arc::new(Mutex::new(HashMap::new()));

            let mut ice_servers = Vec::new();
            if let Some(cfg) = config_tbl {
                if let Ok(servers) = cfg.get::<Table>("ice_servers") {
                    for i in 1..=servers.raw_len() {
                        if let Ok(server_tbl) = servers.raw_get::<Table>(i) {
                            let mut urls = Vec::new();
                            if let Ok(u) = server_tbl.get::<String>("urls") {
                                urls.push(u);
                            } else if let Ok(u_tbl) = server_tbl.get::<Table>("urls") {
                                for j in 1..=u_tbl.raw_len() {
                                    if let Ok(u) = u_tbl.raw_get::<String>(j) {
                                        urls.push(u);
                                    }
                                }
                            }
                            ice_servers.push(RTCIceServer {
                                urls,
                                username: server_tbl.get("username").unwrap_or_default(),
                                credential: server_tbl.get("credential").unwrap_or_default(),
                            });
                        }
                    }
                }
            }

            if ice_servers.is_empty() {
                ice_servers.push(RTCIceServer {
                    urls: vec!["stun:stun.l.google.com:19302".to_string()],
                    username: String::new(),
                    credential: String::new(),
                });
            }

            let rtc_config = RTCConfigurationBuilder::default()
                .with_ice_servers(ice_servers)
                .build();

            #[cfg(feature = "media")]
            let h_video = v_mgr.clone();
            #[cfg(feature = "media")]
            let h_audio = a_mgr.clone();

            let handler = Arc::new(LuaPeerConnectionHandler {
                on_ice_candidate: on_ice_candidate.clone(),
                on_connection_state: on_connection_state.clone(),
                on_data_channel: on_data_channel.clone(),
                on_track: on_track.clone(),
                channels: channels.clone(),
                tracks: tracks.clone(),
                lua_ref: p_lua.clone(),
                bridge: p_bridge.clone(),
                #[cfg(feature = "media")]
                video: h_video,
                #[cfg(feature = "media")]
                audio: h_audio,
            });

            let (tx, rx) = tokio::sync::oneshot::channel();
            crate::tokio_runtime().spawn(async move {
                let pc_res = PeerConnectionBuilder::new()
                    .with_configuration(rtc_config)
                    .with_handler(handler)
                    .with_udp_addrs(vec!["0.0.0.0:0"])
                    .build()
                    .await;
                let _ = tx.send(pc_res);
            });

            let pc_raw = futures::executor::block_on(rx).map_err(|e| {
                mlua::Error::RuntimeError(format!("Failed to build PeerConnection: {e}"))
            })?;

            let pc = Arc::new(pc_raw.map_err(|e| {
                mlua::Error::RuntimeError(format!("PeerConnection error: {e}"))
            })?);

            Ok(LuaPeerConnection {
                id: peer_id,
                pc,
                on_ice_candidate,
                on_connection_state,
                on_data_channel,
                on_track,
                channels,
                tracks,
                lua_ref: p_lua.clone(),
                bridge: p_bridge.clone(),
            })
        })?,
    )?;

    lua.globals().set("webrtc", webrtc_tbl.clone())?;

    if let Ok(pkg) = lua.globals().get::<Table>("package") {
        if let Ok(loaded) = pkg.get::<Table>("loaded") {
            let _ = loaded.set("webrtc", webrtc_tbl);
        }
    }

    Ok(())
}
