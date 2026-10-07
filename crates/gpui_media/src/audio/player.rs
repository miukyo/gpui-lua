use super::decoder::{FfmpegAudioDecoder, AUDIO_SAMPLE_RATE};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{SampleFormat, StreamConfig};
use parking_lot::{Mutex, RwLock};
use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

static NEXT_AUDIO_ID: AtomicU64 = AtomicU64::new(1);

pub struct AudioPlayer {
    id: u64,
    pub src: PathBuf,
    is_playing: Arc<AtomicBool>,
    is_looping: Arc<AtomicBool>,
    volume: Arc<RwLock<f32>>,
    worker_cancel: Arc<AtomicBool>,
    duration_secs: Arc<RwLock<f64>>,
    position_secs: Arc<RwLock<f64>>,
    seek_request: Arc<RwLock<Option<f64>>>,
    ring_buffer: Arc<Mutex<VecDeque<f32>>>,
}

impl AudioPlayer {
    pub fn new(src: PathBuf) -> Arc<Self> {
        let id = NEXT_AUDIO_ID.fetch_add(1, Ordering::SeqCst);
        let is_playing = Arc::new(AtomicBool::new(false));
        let is_looping = Arc::new(AtomicBool::new(false));
        let volume = Arc::new(RwLock::new(1.0));
        let worker_cancel = Arc::new(AtomicBool::new(false));
        let duration_secs = Arc::new(RwLock::new(0.0));
        let position_secs = Arc::new(RwLock::new(0.0));
        let seek_request = Arc::new(RwLock::new(None));
        let ring_buffer = Arc::new(Mutex::new(VecDeque::with_capacity(96000))); // ~1s buffer

        let player = Arc::new(Self {
            id,
            src: src.clone(),
            is_playing: Arc::clone(&is_playing),
            is_looping: Arc::clone(&is_looping),
            volume: Arc::clone(&volume),
            worker_cancel: Arc::clone(&worker_cancel),
            duration_secs: Arc::clone(&duration_secs),
            position_secs: Arc::clone(&position_secs),
            seek_request: Arc::clone(&seek_request),
            ring_buffer: Arc::clone(&ring_buffer),
        });

        // Background thread owns cpal::Stream (which is !Send / !Sync)
        let ring_output = Arc::clone(&ring_buffer);
        let playing_output = Arc::clone(&is_playing);
        let volume_output = Arc::clone(&volume);
        let cancel_output = Arc::clone(&worker_cancel);
        std::thread::Builder::new()
            .name(format!("gpui-audio-output-{id}"))
            .spawn(move || {
                let _stream = Self::setup_audio_output(
                    ring_output,
                    playing_output,
                    volume_output,
                );
                while !cancel_output.load(Ordering::Relaxed) {
                    std::thread::sleep(Duration::from_millis(50));
                }
            })
            .ok();

        // Background thread handles audio decoding via FFmpeg
        std::thread::Builder::new()
            .name(format!("gpui-audio-decoder-{id}"))
            .spawn(move || {
                let mut decoder = match FfmpegAudioDecoder::open(&src) {
                    Ok(d) => {
                        *duration_secs.write() = d.duration_secs();
                        d
                    }
                    Err(e) => {
                        log::error!("Failed to open audio file {}: {e}", src.display());
                        return;
                    }
                };

                let mut sample_buf = Vec::new();
                let mut total_samples_decoded = 0u64;

                while !worker_cancel.load(Ordering::Relaxed) {
                    // Check if seek was requested
                    if let Some(target) = seek_request.write().take() {
                        let _ = decoder.seek(target);
                        ring_buffer.lock().clear();
                        total_samples_decoded = (target * AUDIO_SAMPLE_RATE as f64) as u64;
                        *position_secs.write() = target;
                    }

                    if !is_playing.load(Ordering::Relaxed) {
                        std::thread::sleep(Duration::from_millis(20));
                        continue;
                    }

                    // Keep ring buffer fed to ~48000 samples (~0.5s ahead)
                    let current_len = ring_buffer.lock().len();
                    if current_len >= 48000 {
                        std::thread::sleep(Duration::from_millis(10));
                        continue;
                    }

                    match decoder.read_next_samples(&mut sample_buf) {
                        Ok(true) => {
                            let mut queue = ring_buffer.lock();
                            queue.extend(sample_buf.iter().copied());
                            total_samples_decoded += (sample_buf.len() / 2) as u64;
                            *position_secs.write() = total_samples_decoded as f64 / AUDIO_SAMPLE_RATE as f64;
                        }
                        Ok(false) => {
                            if is_looping.load(Ordering::Relaxed) {
                                let _ = decoder.rewind();
                                total_samples_decoded = 0;
                            } else {
                                is_playing.store(false, Ordering::Relaxed);
                            }
                        }
                        Err(e) => {
                            log::warn!("Audio decoding error: {e}");
                            break;
                        }
                    }
                }
            })
            .ok();

        player
    }
    /// Creates a stream-backed audio player for live audio (WebRTC / Microphone).
    pub fn new_stream(src: PathBuf) -> Arc<Self> {
        let id = NEXT_AUDIO_ID.fetch_add(1, Ordering::SeqCst);
        let is_playing = Arc::new(AtomicBool::new(true));
        let is_looping = Arc::new(AtomicBool::new(false));
        let volume = Arc::new(RwLock::new(1.0));
        let worker_cancel = Arc::new(AtomicBool::new(false));
        let duration_secs = Arc::new(RwLock::new(0.0));
        let position_secs = Arc::new(RwLock::new(0.0));
        let seek_request = Arc::new(RwLock::new(None));
        let ring_buffer = Arc::new(Mutex::new(VecDeque::with_capacity(96000)));

        let player = Arc::new(Self {
            id,
            src,
            is_playing: Arc::clone(&is_playing),
            is_looping: Arc::clone(&is_looping),
            volume: Arc::clone(&volume),
            worker_cancel: Arc::clone(&worker_cancel),
            duration_secs,
            position_secs,
            seek_request,
            ring_buffer: Arc::clone(&ring_buffer),
        });

        // Background thread owns cpal::Stream
        let ring_output = Arc::clone(&ring_buffer);
        let playing_output = Arc::clone(&is_playing);
        let volume_output = Arc::clone(&volume);
        let cancel_output = Arc::clone(&worker_cancel);
        std::thread::Builder::new()
            .name(format!("gpui-audio-stream-{id}"))
            .spawn(move || {
                let _stream = Self::setup_audio_output(
                    ring_output,
                    playing_output,
                    volume_output,
                );
                while !cancel_output.load(Ordering::Relaxed) {
                    std::thread::sleep(Duration::from_millis(50));
                }
            })
            .ok();

        player
    }

    /// Pushes incoming audio samples (stereo or mono float samples) to be played out.
    pub fn push_samples(&self, samples: &[f32]) {
        let mut queue = self.ring_buffer.lock();
        if queue.len() > 96000 {
            queue.clear();
        }
        queue.extend(samples.iter().copied());
    }

    fn setup_audio_output(
        ring: Arc<Mutex<VecDeque<f32>>>,
        playing: Arc<AtomicBool>,
        volume: Arc<RwLock<f32>>,
    ) -> Option<cpal::Stream> {
        let host = cpal::default_host();
        let device = host.default_output_device()?;
        let default_config = device.default_output_config().ok()?;
        let sample_format = default_config.sample_format();
        let config: StreamConfig = default_config.into();
        let err_fn = |err| log::error!("Audio output stream error: {err}");

        let stream_res = match sample_format {
            SampleFormat::F32 => device.build_output_stream(
                &config,
                move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                    if !playing.load(Ordering::Relaxed) {
                        data.fill(0.0);
                        return;
                    }
                    let vol = *volume.read();
                    let mut queue = ring.lock();
                    for s in data.iter_mut() {
                        *s = queue.pop_front().unwrap_or(0.0) * vol;
                    }
                },
                err_fn,
                None,
            ),
            SampleFormat::I16 => device.build_output_stream(
                &config,
                move |data: &mut [i16], _: &cpal::OutputCallbackInfo| {
                    if !playing.load(Ordering::Relaxed) {
                        data.fill(0);
                        return;
                    }
                    let vol = *volume.read();
                    let mut queue = ring.lock();
                    for s in data.iter_mut() {
                        let sample = queue.pop_front().unwrap_or(0.0) * vol;
                        *s = (sample * i16::MAX as f32) as i16;
                    }
                },
                err_fn,
                None,
            ),
            _ => return None,
        };

        match stream_res {
            Ok(stream) => {
                let _ = stream.play();
                Some(stream)
            }
            Err(_) => None,
        }
    }

    pub fn id(&self) -> u64 {
        self.id
    }

    pub fn play(&self) {
        self.is_playing.store(true, Ordering::Relaxed);
    }

    pub fn pause(&self) {
        self.is_playing.store(false, Ordering::Relaxed);
    }

    pub fn stop(&self) {
        self.is_playing.store(false, Ordering::Relaxed);
        self.ring_buffer.lock().clear();
        *self.position_secs.write() = 0.0;
    }

    pub fn seek(&self, seconds: f64) {
        *self.seek_request.write() = Some(seconds);
    }

    pub fn is_playing(&self) -> bool {
        self.is_playing.load(Ordering::Relaxed)
    }

    pub fn set_loop(&self, looping: bool) {
        self.is_looping.store(looping, Ordering::Relaxed);
    }

    pub fn set_volume(&self, vol: f32) {
        *self.volume.write() = vol.clamp(0.0, 1.0);
    }

    pub fn duration_secs(&self) -> f64 {
        *self.duration_secs.read()
    }

    pub fn position_secs(&self) -> f64 {
        *self.position_secs.read()
    }
}

impl Drop for AudioPlayer {
    fn drop(&mut self) {
        self.worker_cancel.store(true, Ordering::Relaxed);
    }
}
