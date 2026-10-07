pub mod capture;
pub mod decoder;
pub mod player;

pub use capture::{list_camera_devices, CameraCapture, CameraDeviceInfo, CameraOptions, VideoFrameCallback};
pub use decoder::{FfmpegPacketVideoDecoder, FfmpegVideoDecoder};
pub use player::VideoPlayer;

use parking_lot::RwLock;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

#[derive(Clone, Default)]
pub struct VideoManager {
    players: Arc<RwLock<HashMap<PathBuf, Arc<VideoPlayer>>>>,
    on_frame: Option<Arc<dyn Fn() + Send + Sync>>,
}

impl VideoManager {
    pub fn new(on_frame: Option<Arc<dyn Fn() + Send + Sync>>) -> Self {
        #[cfg(target_os = "windows")]
        crate::embedded_ffmpeg::ensure_ffmpeg_dlls();

        Self {
            players: Arc::new(RwLock::new(HashMap::new())),
            on_frame,
        }
    }

    pub fn get_or_create(&self, path: &Path, autoplay: bool) -> Arc<VideoPlayer> {
        let mut players = self.players.write();
        let path_buf = path.to_path_buf();
        if let Some(player) = players.get(&path_buf) {
            return Arc::clone(player);
        }
        let player = VideoPlayer::new(path_buf.clone(), autoplay, self.on_frame.clone());
        players.insert(path_buf, Arc::clone(&player));
        player
    }

    pub fn get_player(&self, path: &Path) -> Option<Arc<VideoPlayer>> {
        self.players.read().get(path).cloned()
    }

    pub fn register_stream_player(&self, uri: &str) -> Arc<VideoPlayer> {
        let mut players = self.players.write();
        let path_buf = PathBuf::from(uri);
        if let Some(player) = players.get(&path_buf) {
            return Arc::clone(player);
        }
        let player = VideoPlayer::new_stream(path_buf.clone(), self.on_frame.clone());
        players.insert(path_buf, Arc::clone(&player));
        player
    }
    pub fn collect_idle(&self) {
        let mut players = self.players.write();
        let now = std::time::Instant::now();
        players.retain(|_, player| {
            player.is_playing()
                || now.duration_since(player.last_accessed()) < std::time::Duration::from_secs(5)
        });
    }
}
