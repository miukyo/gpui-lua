use anyhow::Result;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{SampleFormat, StreamConfig};
use parking_lot::{Mutex, RwLock};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;

static NEXT_MIC_ID: AtomicU64 = AtomicU64::new(1);

pub type AudioSamplesCallback = Arc<dyn Fn(&[f32]) + Send + Sync>;

#[derive(Clone, Debug)]
pub struct MicrophoneDeviceInfo {
    pub id: usize,
    pub name: String,
    pub is_default: bool,
}

#[derive(Clone, Debug)]
pub struct MicrophoneOptions {
    pub device_index: usize,
    pub device_name: Option<String>,
    pub sample_rate: u32,
    pub channels: u16,
}

impl Default for MicrophoneOptions {
    fn default() -> Self {
        Self {
            device_index: 0,
            device_name: None,
            sample_rate: 48000,
            channels: 2,
        }
    }
}

/// Enumerate all available hardware audio input devices (microphones).
pub fn list_microphone_devices() -> Vec<MicrophoneDeviceInfo> {
    let host = cpal::default_host();
    let mut list = Vec::new();

    // 1. Put the OS default input device as index 0
    if let Some(default_dev) = host.default_input_device() {
        if let Ok(name) = default_dev.name() {
            list.push(MicrophoneDeviceInfo {
                id: 0,
                name,
                is_default: true,
            });
        }
    }

    // 2. Append all other input devices
    if let Ok(devices) = host.input_devices() {
        let mut idx = 1;
        for dev in devices {
            if let Ok(name) = dev.name() {
                if list.first().map_or(true, |def| def.name != name) {
                    list.push(MicrophoneDeviceInfo {
                        id: idx,
                        name,
                        is_default: false,
                    });
                    idx += 1;
                }
            }
        }
    }

    if list.is_empty() {
        list.push(MicrophoneDeviceInfo {
            id: 0,
            name: "Default Microphone".to_string(),
            is_default: true,
        });
    }

    list
}

pub struct MicrophoneCapture {
    id: u64,
    name: String,
    uri: String,
    options: MicrophoneOptions,
    is_muted: Arc<AtomicBool>,
    is_running: Arc<AtomicBool>,
    callbacks: Arc<Mutex<Vec<AudioSamplesCallback>>>,
    target_player: Arc<RwLock<Option<Arc<super::AudioPlayer>>>>,
}

impl MicrophoneCapture {
    pub fn start(options: MicrophoneOptions) -> Result<Arc<Self>> {
        let id = NEXT_MIC_ID.fetch_add(1, Ordering::SeqCst);
        let uri = format!("microphone://{}", options.device_index);
        let is_muted = Arc::new(AtomicBool::new(false));
        let is_running = Arc::new(AtomicBool::new(true));
        let callbacks = Arc::new(Mutex::new(Vec::new()));
        let target_player = Arc::new(RwLock::new(None));

        let available_mics = list_microphone_devices();
        let selected_name = if let Some(name) = &options.device_name {
            name.clone()
        } else if let Some(info) = available_mics.get(options.device_index) {
            info.name.clone()
        } else {
            "Default Audio Input".to_string()
        };

        let mic = Arc::new(Self {
            id,
            name: selected_name.clone(),
            uri: uri.clone(),
            options: options.clone(),
            is_muted: Arc::clone(&is_muted),
            is_running: Arc::clone(&is_running),
            callbacks: Arc::clone(&callbacks),
            target_player: Arc::clone(&target_player),
        });

        let is_muted_stream = Arc::clone(&is_muted);
        let is_running_stream = Arc::clone(&is_running);
        let callbacks_stream = Arc::clone(&callbacks);
        let target_player_stream = Arc::clone(&target_player);
        let mic_name = selected_name.clone();
        let target_idx = options.device_index;

        std::thread::Builder::new()
            .name(format!("gpui-microphone-capture-{id}"))
            .spawn(move || {
                let host = cpal::default_host();
                let device = if target_idx == 0 && options.device_name.is_none() {
                    host.default_input_device()
                } else {
                    let mut found = None;
                    if let Ok(devices) = host.input_devices() {
                        for dev in devices {
                            if let Ok(n) = dev.name() {
                                if n == mic_name {
                                    found = Some(dev);
                                    break;
                                }
                            }
                        }
                    }
                    found.or_else(|| host.default_input_device())
                };

                let device = match device {
                    Some(d) => d,
                    None => {
                        log::warn!("No audio input device found, running ambient silence generator");
                        while is_running_stream.load(Ordering::Relaxed) {
                            std::thread::sleep(std::time::Duration::from_millis(20));
                            if !is_muted_stream.load(Ordering::Relaxed) {
                                let silence = [0.0f32; 960];
                                let cbs = callbacks_stream.lock().clone();
                                for cb in cbs {
                                    cb(&silence);
                                }
                            }
                        }
                        return;
                    }
                };

                let config = match device.default_input_config() {
                    Ok(cfg) => cfg,
                    Err(e) => {
                        log::error!("Failed to get default input config: {e}");
                        return;
                    }
                };

                let sample_format = config.sample_format();
                let stream_config: StreamConfig = config.into();

                let stream_res = match sample_format {
                    SampleFormat::F32 => {
                        let is_muted = Arc::clone(&is_muted_stream);
                        let cbs = Arc::clone(&callbacks_stream);
                        let player = Arc::clone(&target_player_stream);
                        device.build_input_stream(
                            &stream_config,
                            move |data: &[f32], _: &_| {
                                if !is_muted.load(Ordering::Relaxed) {
                                    if let Some(p) = player.read().as_ref() {
                                        p.push_samples(data);
                                    }
                                    let listeners = cbs.lock().clone();
                                    for cb in listeners {
                                        cb(data);
                                    }
                                }
                            },
                            |err| log::error!("Microphone stream error: {err}"),
                            None,
                        )
                    }
                    SampleFormat::I16 => {
                        let is_muted = Arc::clone(&is_muted_stream);
                        let cbs = Arc::clone(&callbacks_stream);
                        let player = Arc::clone(&target_player_stream);
                        device.build_input_stream(
                            &stream_config,
                            move |data: &[i16], _: &_| {
                                if !is_muted.load(Ordering::Relaxed) {
                                    let f32_samples: Vec<f32> = data
                                        .iter()
                                        .map(|&s| s as f32 / i16::MAX as f32)
                                        .collect();
                                    if let Some(p) = player.read().as_ref() {
                                        p.push_samples(&f32_samples);
                                    }
                                    let listeners = cbs.lock().clone();
                                    for cb in listeners {
                                        cb(&f32_samples);
                                    }
                                }
                            },
                            |err| log::error!("Microphone stream error: {err}"),
                            None,
                        )
                    }
                    SampleFormat::U16 => {
                        let is_muted = Arc::clone(&is_muted_stream);
                        let cbs = Arc::clone(&callbacks_stream);
                        let player = Arc::clone(&target_player_stream);
                        device.build_input_stream(
                            &stream_config,
                            move |data: &[u16], _: &_| {
                                if !is_muted.load(Ordering::Relaxed) {
                                    let f32_samples: Vec<f32> = data
                                        .iter()
                                        .map(|&s| (s as f32 - 32768.0) / 32768.0)
                                        .collect();
                                    if let Some(p) = player.read().as_ref() {
                                        p.push_samples(&f32_samples);
                                    }
                                    let listeners = cbs.lock().clone();
                                    for cb in listeners {
                                        cb(&f32_samples);
                                    }
                                }
                            },
                            |err| log::error!("Microphone stream error: {err}"),
                            None,
                        )
                    }
                    _ => {
                        log::error!("Unsupported microphone sample format: {sample_format:?}");
                        return;
                    }
                };

                if let Ok(stream) = stream_res {
                    let _ = stream.play();
                    while is_running_stream.load(Ordering::Relaxed) {
                        std::thread::sleep(std::time::Duration::from_millis(50));
                    }
                }
            })?;

        Ok(mic)
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

    pub fn bind_player(&self, player: Arc<super::AudioPlayer>) {
        *self.target_player.write() = Some(player);
    }

    pub fn options(&self) -> &MicrophoneOptions {
        &self.options
    }

    pub fn add_samples_listener(&self, cb: AudioSamplesCallback) {
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

impl Drop for MicrophoneCapture {
    fn drop(&mut self) {
        self.stop();
    }
}
