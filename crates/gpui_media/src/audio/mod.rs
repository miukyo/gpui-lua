pub mod capture;
pub mod decoder;
pub mod player;

pub use capture::{list_microphone_devices, AudioSamplesCallback, MicrophoneCapture, MicrophoneDeviceInfo, MicrophoneOptions};
pub use decoder::{FfmpegAudioDecoder, FfmpegPacketAudioDecoder};
pub use player::AudioPlayer;

use parking_lot::RwLock;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

#[derive(Clone, Default)]
pub struct AudioManager {
    players: Arc<RwLock<HashMap<PathBuf, Arc<AudioPlayer>>>>,
}

impl AudioManager {
    pub fn new() -> Self {
        #[cfg(target_os = "windows")]
        crate::embedded_ffmpeg::ensure_ffmpeg_dlls();

        Self {
            players: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn load(&self, path: &Path) -> Arc<AudioPlayer> {
        let mut players = self.players.write();
        let path_buf = path.to_path_buf();
        if let Some(player) = players.get(&path_buf) {
            return Arc::clone(player);
        }
        let player = AudioPlayer::new(path_buf.clone());
        players.insert(path_buf, Arc::clone(&player));
        player
    }

    pub fn play(&self, path: &Path) -> Arc<AudioPlayer> {
        let player = self.load(path);
        player.play();
        player
    }

    pub fn get_player(&self, path: &Path) -> Option<Arc<AudioPlayer>> {
        self.players.read().get(path).cloned()
    }

    pub fn register_stream_player(&self, uri: &str) -> Arc<AudioPlayer> {
        let mut players = self.players.write();
        let path_buf = PathBuf::from(uri);
        if let Some(player) = players.get(&path_buf) {
            return Arc::clone(player);
        }
        let player = AudioPlayer::new_stream(path_buf.clone());
        players.insert(path_buf, Arc::clone(&player));
        player
    }
    pub fn collect_idle(&self) {
        let mut players = self.players.write();
        players.retain(|_, player| {
            Arc::strong_count(player) > 1 || player.is_playing()
        });
    }
}
