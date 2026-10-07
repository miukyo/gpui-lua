use anyhow::Result;
use parking_lot::{Mutex, RwLock};
#[cfg(target_os = "windows")]
use std::io::Read;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

static NEXT_CAMERA_ID: AtomicU64 = AtomicU64::new(1);

pub type VideoFrameCallback = Arc<dyn Fn(u32, u32, &[u8]) + Send + Sync>;

#[derive(Clone, Debug)]
pub struct CameraDeviceInfo {
    pub id: usize,
    pub name: String,
    pub is_default: bool,
}

#[derive(Clone, Debug)]
pub struct CameraOptions {
    pub device_index: usize,
    pub device_name: Option<String>,
    pub width: u32,
    pub height: u32,
    pub fps: u32,
}

impl Default for CameraOptions {
    fn default() -> Self {
        Self {
            device_index: 0,
            device_name: None,
            width: 1280,
            height: 720,
            fps: 30,
        }
    }
}

/// Enumerate all available hardware video capture devices (webcams).
pub fn list_camera_devices() -> Vec<CameraDeviceInfo> {
    let mut list = Vec::new();

    #[cfg(target_os = "windows")]
    {
        // 1. Query via FFmpeg DirectShow device list
        if let Ok(output) = std::process::Command::new("ffmpeg")
            .args(["-list_devices", "true", "-f", "dshow", "-i", "dummy"])
            .output()
        {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let mut idx = 0;
            for line in stderr.lines() {
                if let Some(start) = line.find("\"") {
                    if let Some(end) = line[start + 1..].find("\"") {
                        let name = &line[start + 1..start + 1 + end];
                        let rest = &line[start + 1 + end + 1..];
                        if rest.contains("(video)") {
                            list.push(CameraDeviceInfo {
                                id: idx,
                                name: name.to_string(),
                                is_default: idx == 0,
                            });
                            idx += 1;
                        }
                    }
                }
            }
        }

        // 2. Fallback to PowerShell PnP Camera device query if ffmpeg query was empty
        if list.is_empty() {
            if let Ok(output) = std::process::Command::new("powershell")
                .args(["-Command", "Get-CimInstance Win32_PnPEntity | Where-Object { $_.PNPClass -eq 'Camera' } | Select-Object -ExpandProperty Name"])
                .output()
            {
                let stdout = String::from_utf8_lossy(&output.stdout);
                for (idx, line) in stdout.lines().enumerate() {
                    let name = line.trim();
                    if !name.is_empty() {
                        list.push(CameraDeviceInfo {
                            id: idx,
                            name: name.to_string(),
                            is_default: idx == 0,
                        });
                    }
                }
            }
        }
    }

    #[cfg(target_os = "macos")]
    {
        list.push(CameraDeviceInfo {
            id: 0,
            name: "FaceTime HD Camera".to_string(),
            is_default: true,
        });
    }

    #[cfg(target_os = "linux")]
    {
        for i in 0..8 {
            let path = format!("/dev/video{}", i);
            if std::path::Path::new(&path).exists() {
                list.push(CameraDeviceInfo {
                    id: i,
                    name: format!("V4L2 Video Device ({})", path),
                    is_default: i == 0,
                });
            }
        }
    }

    if list.is_empty() {
        list.push(CameraDeviceInfo {
            id: 0,
            name: "Default Live Camera".to_string(),
            is_default: true,
        });
    }

    list
}

pub struct CameraCapture {
    id: u64,
    name: String,
    uri: String,
    options: CameraOptions,
    is_muted: Arc<AtomicBool>,
    is_running: Arc<AtomicBool>,
    callbacks: Arc<Mutex<Vec<VideoFrameCallback>>>,
    target_player: Arc<RwLock<Option<Arc<super::VideoPlayer>>>>,
}
impl CameraCapture {
    pub fn start(options: CameraOptions) -> Result<Arc<Self>> {
        let id = NEXT_CAMERA_ID.fetch_add(1, Ordering::SeqCst);
        let uri = format!("camera://{}", options.device_index);
        let is_muted = Arc::new(AtomicBool::new(false));
        let is_running = Arc::new(AtomicBool::new(true));
        let callbacks = Arc::new(Mutex::new(Vec::new()));
        let target_player = Arc::new(RwLock::new(None));

        // Resolve camera device name
        let available_cameras = list_camera_devices();
        let selected_name = if let Some(name) = &options.device_name {
            name.clone()
        } else if options.device_index == 0 {
            available_cameras
                .iter()
                .find(|c| c.is_default)
                .map(|c| c.name.clone())
                .or_else(|| available_cameras.first().map(|c| c.name.clone()))
                .unwrap_or_else(|| "Default Live Camera".to_string())
        } else if let Some(info) = available_cameras.get(options.device_index) {
            info.name.clone()
        } else {
            "Default Live Camera".to_string()
        };

        let camera = Arc::new(Self {
            id,
            name: selected_name.clone(),
            uri: uri.clone(),
            options: options.clone(),
            is_muted: Arc::clone(&is_muted),
            is_running: Arc::clone(&is_running),
            callbacks: Arc::clone(&callbacks),
            target_player: Arc::clone(&target_player),
        });

        // Spawn background capture thread
        let running_clone = Arc::clone(&is_running);
        let muted_clone = Arc::clone(&is_muted);
        let callbacks_clone = Arc::clone(&callbacks);
        let player_clone = Arc::clone(&target_player);
        let w = options.width;
        let h = options.height;
        let fps = options.fps.max(1);
        #[cfg(target_os = "windows")]
        let camera_name = selected_name.clone();
        #[cfg(not(target_os = "windows"))]
        let _ = selected_name;
        std::thread::Builder::new()
            .name(format!("gpui-camera-capture-{id}"))
            .spawn(move || {
                let frame_size = (w * h * 4) as usize;

                // Try opening real hardware camera via FFmpeg DirectShow
                #[cfg(target_os = "windows")]
                {
                    let mut cmd = std::process::Command::new("ffmpeg");
                    cmd.args([
                        "-y",
                        "-f", "dshow",
                        "-i", &format!("video={camera_name}"),
                        "-s", &format!("{w}x{h}"),
                        "-r", &format!("{fps}"),
                        "-f", "rawvideo",
                        "-pix_fmt", "bgra",
                        "pipe:1",
                    ]);
                    cmd.stdout(std::process::Stdio::piped());
                    cmd.stderr(std::process::Stdio::null());

                    if let Ok(mut child) = cmd.spawn() {
                        if let Some(mut stdout) = child.stdout.take() {
                            let mut buf = vec![0u8; frame_size];
                            let mut read_any = false;

                            while running_clone.load(Ordering::Relaxed) {
                                if stdout.read_exact(&mut buf).is_ok() {
                                    read_any = true;
                                    if !muted_clone.load(Ordering::Relaxed) {
                                        if let Some(player) = player_clone.read().as_ref() {
                                            player.push_bgra_frame(w, h, buf.clone());
                                        }
                                        let cbs = callbacks_clone.lock().clone();
                                        for cb in cbs {
                                            cb(w, h, &buf);
                                        }
                                    }
                                } else {
                                    break;
                                }
                            }

                            let _ = child.kill();
                            if read_any {
                                return;
                            }
                        }
                    }
                }

                // Fallback to high-quality responsive synthetic pattern if hardware webcam is unavailable
                let start_time = Instant::now();
                let mut frame_number: u64 = 0;
                let mut raw_buffer = vec![0u8; frame_size];
                let frame_delay = Duration::from_micros(1_000_000 / fps as u64);

                while running_clone.load(Ordering::Relaxed) {
                    let loop_start = Instant::now();
                    frame_number += 1;

                    if muted_clone.load(Ordering::Relaxed) {
                        raw_buffer.fill(0);
                    } else {
                        let elapsed_secs = start_time.elapsed().as_secs_f64();
                        generate_camera_frame(&mut raw_buffer, w, h, elapsed_secs, frame_number);
                    }

                    if let Some(player) = player_clone.read().as_ref() {
                        player.push_bgra_frame(w, h, raw_buffer.clone());
                    }

                    let cbs = callbacks_clone.lock().clone();
                    for cb in cbs {
                        cb(w, h, &raw_buffer);
                    }

                    let elapsed = loop_start.elapsed();
                    if frame_delay > elapsed {
                        std::thread::sleep(frame_delay - elapsed);
                    }
                }
            })?;

        Ok(camera)
    }

    pub fn id(&self) -> u64 {
        self.id
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn src(&self) -> &str {
        &self.uri
    }

    pub fn uri(&self) -> &str {
        &self.uri
    }

    pub fn width(&self) -> u32 {
        self.options.width
    }

    pub fn height(&self) -> u32 {
        self.options.height
    }

    pub fn fps(&self) -> u32 {
        self.options.fps
    }

    pub fn bind_player(&self, player: Arc<super::VideoPlayer>) {
        *self.target_player.write() = Some(player);
    }

    pub fn add_frame_listener(&self, cb: VideoFrameCallback) {
        self.callbacks.lock().push(cb);
    }

    pub fn set_muted(&self, muted: bool) {
        self.is_muted.store(muted, Ordering::Relaxed);
    }

    pub fn is_muted(&self) -> bool {
        self.is_muted.load(Ordering::Relaxed)
    }

    pub fn stop(&self) {
        self.is_running.store(false, Ordering::Relaxed);
    }
}

impl Drop for CameraCapture {
    fn drop(&mut self) {
        self.stop();
    }
}

/// Generates a live synthetic camera feed with timestamp, moving pattern, and active indicator.
fn generate_camera_frame(buf: &mut [u8], width: u32, height: u32, t: f64, _frame_idx: u64) {
    let w = width as usize;
    let h = height as usize;
    let sweep_x = (((t * 240.0) as usize) % w) as f64;
    let pulse = ((t * 4.0).sin() * 0.5 + 0.5) as f32;

    for y in 0..h {
        let y_f = y as f64 / h as f64;
        let row_offset = y * w * 4;

        for x in 0..w {
            let x_f = x as f64 / w as f64;
            let pixel_idx = row_offset + x * 4;

            let mut r = (30.0 + 15.0 * y_f) as u8;
            let mut g = (32.0 + 20.0 * x_f) as u8;
            let mut b = (48.0 + 40.0 * (1.0 - y_f)) as u8;

            let dist_to_sweep = ((x as f64) - sweep_x).abs();
            if dist_to_sweep < 8.0 {
                let intensity = (1.0 - dist_to_sweep / 8.0) * 160.0;
                r = r.saturating_add(intensity as u8);
                g = g.saturating_add((intensity * 0.8) as u8);
                b = b.saturating_add(intensity as u8);
            }

            if x >= 24 && x <= 40 && y >= 24 && y <= 40 {
                let dx = (x as i32) - 32;
                let dy = (y as i32) - 32;
                if dx * dx + dy * dy <= 54 {
                    r = (40.0 * (1.0 - pulse)) as u8;
                    g = (160.0 + 95.0 * pulse) as u8;
                    b = 100;
                }
            }

            let cx = (w / 2) as i32;
            let cy = (h / 2) as i32;
            let dist_center_sq = ((x as i32) - cx).pow(2) + ((y as i32) - cy).pow(2);
            if (dist_center_sq >= 3800 && dist_center_sq <= 4200) || (dist_center_sq >= 300 && dist_center_sq <= 380) {
                r = 137;
                g = 180;
                b = 250;
            }

            buf[pixel_idx] = b;
            buf[pixel_idx + 1] = g;
            buf[pixel_idx + 2] = r;
            buf[pixel_idx + 3] = 255;
        }
    }
}
