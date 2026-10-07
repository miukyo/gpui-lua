use super::decoder::FfmpegVideoDecoder;
use crate::audio::{AudioPlayer, FfmpegAudioDecoder};
use parking_lot::RwLock;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::{Duration, Instant};

static NEXT_PLAYER_ID: AtomicU64 = AtomicU64::new(1);

pub type FrameCallback = Box<dyn Fn() + Send + Sync>;

pub struct VideoPlayer {
    id: u64,
    pub src: PathBuf,
    is_playing: Arc<AtomicBool>,
    is_looping: Arc<AtomicBool>,
    is_muted: Arc<AtomicBool>,
    volume: Arc<RwLock<f32>>,
    current_image: Arc<RwLock<Option<Arc<gpui::RenderImage>>>>,
    #[cfg(target_os = "windows")]
    current_surface: Arc<RwLock<Option<gpui::SurfaceSource>>>,
    worker_cancel: Arc<AtomicBool>,
    duration_secs: Arc<RwLock<f64>>,
    position_secs: Arc<RwLock<f64>>,
    seek_request: Arc<RwLock<Option<f64>>>,
    prev_images: Arc<parking_lot::Mutex<std::collections::VecDeque<Arc<gpui::RenderImage>>>>,
    last_accessed: Arc<parking_lot::Mutex<Instant>>,
    audio_player: Option<Arc<AudioPlayer>>,
    on_frame: Option<Arc<dyn Fn() + Send + Sync>>,
}

impl VideoPlayer {
    pub fn new(
        src: PathBuf,
        autoplay: bool,
        on_frame: Option<Arc<dyn Fn() + Send + Sync>>,
    ) -> Arc<Self> {
        let id = NEXT_PLAYER_ID.fetch_add(1, Ordering::SeqCst);
        let is_playing = Arc::new(AtomicBool::new(autoplay));
        let is_looping = Arc::new(AtomicBool::new(false));
        let is_muted = Arc::new(AtomicBool::new(false));
        let volume = Arc::new(RwLock::new(1.0));
        let current_image = Arc::new(RwLock::new(None));
        #[cfg(target_os = "windows")]
        let current_surface = Arc::new(RwLock::new(None));
        let worker_cancel = Arc::new(AtomicBool::new(false));
        let duration_secs = Arc::new(RwLock::new(0.0));
        let position_secs = Arc::new(RwLock::new(0.0));
        let seek_request = Arc::new(RwLock::new(None));
        let prev_images = Arc::new(parking_lot::Mutex::new(std::collections::VecDeque::new()));
        let last_accessed = Arc::new(parking_lot::Mutex::new(Instant::now()));

        // Open audio stream if present in video file
        let audio_player = match FfmpegAudioDecoder::open(&src) {
            Ok(_) => {
                let ap = AudioPlayer::new(src.clone());
                ap.set_volume(*volume.read());
                if autoplay {
                    ap.play();
                }
                Some(ap)
            }
            Err(_) => None,
        };

        let player = Arc::new(Self {
            id,
            src: src.clone(),
            is_playing: Arc::clone(&is_playing),
            is_looping: Arc::clone(&is_looping),
            is_muted: Arc::clone(&is_muted),
            volume,
            current_image: Arc::clone(&current_image),
            #[cfg(target_os = "windows")]
            current_surface: Arc::clone(&current_surface),
            worker_cancel: Arc::clone(&worker_cancel),
            duration_secs: Arc::clone(&duration_secs),
            position_secs: Arc::clone(&position_secs),
            seek_request: Arc::clone(&seek_request),
            prev_images: Arc::clone(&prev_images),
            last_accessed: Arc::clone(&last_accessed),
            audio_player: audio_player.clone(),
            on_frame: on_frame.clone(),
        });

        let audio_player_worker = audio_player.clone();

        // Spawn video decoding worker thread
        std::thread::Builder::new()
            .name(format!("gpui-video-decoder-{id}"))
            .spawn(move || {
                let mut decoder = match FfmpegVideoDecoder::open(&src) {
                    Ok(d) => {
                        *duration_secs.write() = d.duration_secs();
                        d
                    }
                    Err(e) => {
                        log::error!("Failed to open video file {}: {e}", src.display());
                        return;
                    }
                };

                let frame_duration = Duration::from_secs_f64(1.0 / decoder.fps());
                let mut buffer = Vec::new();
                let mut frame_idx = 0u64;

                #[cfg(target_os = "windows")]
                let mut d3d11_hw_active = true;

                #[cfg(target_os = "windows")]
                if d3d11_hw_active {
                    if let Ok(Some(d3d_tex)) = decoder.read_next_d3d11_texture() {
                        let size = gpui::size(
                            gpui::DevicePixels(decoder.width() as i32),
                            gpui::DevicePixels(decoder.height() as i32),
                        );
                        let directx_surface = gpui::DirectXSurface::new(d3d_tex, size);
                        *current_surface.write() =
                            Some(gpui::SurfaceSource::DirectX(directx_surface));
                        if let Some(cb) = &on_frame {
                            cb();
                        }
                        frame_idx = 1;
                    } else {
                        d3d11_hw_active = false;
                    }
                }

                if frame_idx == 0 {
                    if let Ok(true) = decoder.read_next_frame(&mut buffer) {
                        let frame_data = std::mem::take(&mut buffer);
                        if let Some(rgba_img) = image::RgbaImage::from_raw(
                            decoder.width(),
                            decoder.height(),
                            frame_data,
                        ) {
                            let render_img = Arc::new(gpui::RenderImage::new(smallvec::smallvec![
                                image::Frame::new(rgba_img)
                            ]));
                            *current_image.write() = Some(render_img);
                            if let Some(cb) = &on_frame {
                                cb();
                            }
                        }
                        frame_idx = 1;
                    }
                }

                while !worker_cancel.load(Ordering::Relaxed) {
                    // Handle seek request
                    if let Some(target) = seek_request.write().take() {
                        let _ = decoder.seek(target);
                        if let Some(ap) = &audio_player_worker {
                            ap.seek(target);
                        }
                        frame_idx = (target * decoder.fps()) as u64;
                        *position_secs.write() = target;

                        // Decode frame at seek position immediately
                        #[cfg(target_os = "windows")]
                        let seek_hw = if d3d11_hw_active {
                            if let Ok(Some(d3d_tex)) = decoder.read_next_d3d11_texture() {
                                let size = gpui::size(
                                    gpui::DevicePixels(decoder.width() as i32),
                                    gpui::DevicePixels(decoder.height() as i32),
                                );
                                let directx_surface = gpui::DirectXSurface::new(d3d_tex, size);
                                *current_surface.write() =
                                    Some(gpui::SurfaceSource::DirectX(directx_surface));
                                if let Some(cb) = &on_frame {
                                    cb();
                                }
                                true
                            } else {
                                false
                            }
                        } else {
                            false
                        };

                        #[cfg(not(target_os = "windows"))]
                        let seek_hw = false;

                        if !seek_hw {
                            if let Ok(true) = decoder.read_next_frame(&mut buffer) {
                                let frame_data = std::mem::take(&mut buffer);
                                if let Some(rgba_img) = image::RgbaImage::from_raw(
                                    decoder.width(),
                                    decoder.height(),
                                    frame_data,
                                ) {
                                    let render_img =
                                        Arc::new(gpui::RenderImage::new(smallvec::smallvec![
                                            image::Frame::new(rgba_img)
                                        ]));
                                    *current_image.write() = Some(render_img);
                                    if let Some(cb) = &on_frame {
                                        cb();
                                    }
                                }
                            }
                        }
                    }

                    if !is_playing.load(Ordering::Relaxed) {
                        std::thread::sleep(Duration::from_millis(20));
                        continue;
                    }

                    let frame_start = Instant::now();
                    #[cfg(target_os = "windows")]
                    let decoded_hw = if d3d11_hw_active {
                        match decoder.read_next_d3d11_texture() {
                            Ok(Some(d3d_tex)) => {
                                let size = gpui::size(
                                    gpui::DevicePixels(decoder.width() as i32),
                                    gpui::DevicePixels(decoder.height() as i32),
                                );
                                let directx_surface = gpui::DirectXSurface::new(d3d_tex, size);
                                *current_surface.write() =
                                    Some(gpui::SurfaceSource::DirectX(directx_surface));
                                if let Some(cb) = &on_frame {
                                    cb();
                                }
                                frame_idx += 1;
                                *position_secs.write() = frame_idx as f64 / decoder.fps();
                                true
                            }
                            _ => false,
                        }
                    } else {
                        false
                    };

                    #[cfg(not(target_os = "windows"))]
                    let decoded_hw = false;

                    if !decoded_hw {
                        match decoder.read_next_frame(&mut buffer) {
                            Ok(true) => {
                                let frame_data = std::mem::take(&mut buffer);
                                if let Some(rgba_img) = image::RgbaImage::from_raw(
                                    decoder.width(),
                                    decoder.height(),
                                    frame_data,
                                ) {
                                    let render_img =
                                        Arc::new(gpui::RenderImage::new(smallvec::smallvec![
                                            image::Frame::new(rgba_img)
                                        ]));
                                    *current_image.write() = Some(render_img);

                                    if let Some(cb) = &on_frame {
                                        cb();
                                    }
                                }
                                frame_idx += 1;
                                *position_secs.write() = frame_idx as f64 / decoder.fps();
                            }
                            Ok(false) => {
                                // Seamless loop without dropped frame
                                if is_looping.load(Ordering::Relaxed) {
                                    if decoder.rewind().is_ok() {
                                        if let Some(ap) = &audio_player_worker {
                                            ap.seek(0.0);
                                            ap.play();
                                        }
                                        frame_idx = 0;
                                        continue;
                                    }
                                }
                                is_playing.store(false, Ordering::Relaxed);
                                if let Some(ap) = &audio_player_worker {
                                    ap.pause();
                                }
                            }
                            Err(e) => {
                                log::warn!("Video decoding error: {e}");
                                break;
                            }
                        }
                    }

                    // Clock-disciplined frame pacing
                    let elapsed = frame_start.elapsed();
                    if frame_duration > elapsed {
                        std::thread::sleep(frame_duration - elapsed);
                    }
                }
            })
            .ok();

        player
    }

    /// Creates a stream-backed video player without a file worker thread.
    /// Frames are pushed in via `push_rgba_frame` (e.g. from WebRTC or Camera).
    pub fn new_stream(src: PathBuf, on_frame: Option<Arc<dyn Fn() + Send + Sync>>) -> Arc<Self> {
        let id = NEXT_PLAYER_ID.fetch_add(1, Ordering::SeqCst);
        let is_playing = Arc::new(AtomicBool::new(true));
        let is_looping = Arc::new(AtomicBool::new(false));
        let is_muted = Arc::new(AtomicBool::new(false));
        let volume = Arc::new(RwLock::new(1.0));
        let current_image = Arc::new(RwLock::new(None));
        #[cfg(target_os = "windows")]
        let current_surface = Arc::new(RwLock::new(None));
        let worker_cancel = Arc::new(AtomicBool::new(false));
        let duration_secs = Arc::new(RwLock::new(0.0));
        let position_secs = Arc::new(RwLock::new(0.0));
        let seek_request = Arc::new(RwLock::new(None));
        let prev_images = Arc::new(parking_lot::Mutex::new(std::collections::VecDeque::new()));
        let last_accessed = Arc::new(parking_lot::Mutex::new(Instant::now()));

        Arc::new(Self {
            id,
            src,
            is_playing,
            is_looping,
            is_muted,
            volume,
            current_image,
            #[cfg(target_os = "windows")]
            current_surface,
            worker_cancel,
            duration_secs,
            position_secs,
            seek_request,
            prev_images,
            last_accessed,
            audio_player: None,
            on_frame,
        })
    }

    /// Pushes a BGRA frame directly into this video player (native GPUI format).
    pub fn push_bgra_frame(&self, width: u32, height: u32, bgra_data: Vec<u8>) {
        if let Some(rgba_img) = image::RgbaImage::from_raw(width, height, bgra_data) {
            let render_img = Arc::new(gpui::RenderImage::new(smallvec::smallvec![
                image::Frame::new(rgba_img)
            ]));
            *self.current_image.write() = Some(render_img);
            *self.last_accessed.lock() = Instant::now();
            if let Some(cb) = &self.on_frame {
                cb();
            }
        }
    }

    /// Pushes a decoded RGBA frame into this video player, swapping R and B channels to match GPUI's BGRA pipeline.
    pub fn push_rgba_frame(&self, width: u32, height: u32, mut rgba_data: Vec<u8>) {
        for pixel in rgba_data.chunks_exact_mut(4) {
            pixel.swap(0, 2);
        }
        self.push_bgra_frame(width, height, rgba_data);
    }
    /// Pushes an already constructed RenderImage into this video player.
    pub fn push_render_image(&self, img: Arc<gpui::RenderImage>) {
        *self.current_image.write() = Some(img);
        *self.last_accessed.lock() = Instant::now();
        if let Some(cb) = &self.on_frame {
            cb();
        }
    }
    pub fn id(&self) -> u64 {
        self.id
    }

    pub fn current_render_image(&self) -> Option<Arc<gpui::RenderImage>> {
        *self.last_accessed.lock() = Instant::now();
        self.current_image.read().clone()
    }
    #[cfg(target_os = "windows")]
    pub fn current_surface(&self) -> Option<gpui::SurfaceSource> {
        *self.last_accessed.lock() = Instant::now();
        self.current_surface.read().clone()
    }

    #[cfg(target_os = "windows")]
    pub fn push_directx_surface(&self, surface: gpui::DirectXSurface) {
        *self.current_surface.write() = Some(gpui::SurfaceSource::DirectX(surface));
        *self.last_accessed.lock() = Instant::now();
        if let Some(cb) = &self.on_frame {
            cb();
        }
    }

    #[cfg(target_os = "windows")]
    pub fn push_d3d11_texture(
        &self,
        texture: windows_061::Win32::Graphics::Direct3D11::ID3D11Texture2D,
        width: u32,
        height: u32,
    ) {
        let size = gpui::size(
            gpui::DevicePixels(width as i32),
            gpui::DevicePixels(height as i32),
        );
        let surface = gpui::DirectXSurface::new(texture, size);
        self.push_directx_surface(surface);
    }

    pub fn last_accessed(&self) -> Instant {
        *self.last_accessed.lock()
    }

    /// Takes a stale frame (from 2 frames ago) to drop from the window's sprite atlas.
    /// Keeps the current frame and immediate previous frame alive on the GPU to prevent flickering.
    pub fn take_stale_render_image(
        &self,
        current: &Arc<gpui::RenderImage>,
    ) -> Option<Arc<gpui::RenderImage>> {
        let mut queue = self.prev_images.lock();
        if queue.back().map_or(true, |last| last.id != current.id) {
            queue.push_back(Arc::clone(current));
            if queue.len() > 2 {
                return queue.pop_front();
            }
        }
        None
    }

    pub fn seek(&self, seconds: f64) {
        *self.seek_request.write() = Some(seconds);
        if let Some(ap) = &self.audio_player {
            ap.seek(seconds);
        }
    }

    pub fn replay(&self) {
        self.seek(0.0);
        self.play();
    }

    pub fn play(&self) {
        self.is_playing.store(true, Ordering::Relaxed);
        if let Some(ap) = &self.audio_player {
            ap.play();
        }
    }

    pub fn pause(&self) {
        self.is_playing.store(false, Ordering::Relaxed);
        if let Some(ap) = &self.audio_player {
            ap.pause();
        }
    }

    pub fn is_playing(&self) -> bool {
        self.is_playing.load(Ordering::Relaxed)
    }

    pub fn set_loop(&self, looping: bool) {
        self.is_looping.store(looping, Ordering::Relaxed);
        if let Some(ap) = &self.audio_player {
            ap.set_loop(looping);
        }
    }

    pub fn set_muted(&self, muted: bool) {
        self.is_muted.store(muted, Ordering::Relaxed);
        if let Some(ap) = &self.audio_player {
            if muted {
                ap.set_volume(0.0);
            } else {
                ap.set_volume(*self.volume.read());
            }
        }
    }

    pub fn set_volume(&self, vol: f32) {
        let v = vol.clamp(0.0, 1.0);
        *self.volume.write() = v;
        if let Some(ap) = &self.audio_player {
            if !self.is_muted.load(Ordering::Relaxed) {
                ap.set_volume(v);
            }
        }
    }

    pub fn duration_secs(&self) -> f64 {
        *self.duration_secs.read()
    }

    pub fn position_secs(&self) -> f64 {
        *self.position_secs.read()
    }
}

impl Drop for VideoPlayer {
    fn drop(&mut self) {
        self.worker_cancel.store(true, Ordering::Relaxed);
        if let Some(ap) = &self.audio_player {
            ap.stop();
        }
    }
}
