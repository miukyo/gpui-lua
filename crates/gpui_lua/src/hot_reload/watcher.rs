use crate::hot_reload::error_view::HotReloadError;
use crate::reactive::bridge::ReactiveBridge;
use notify::{Config, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use parking_lot::RwLock;
use std::path::{Path, PathBuf};
use std::sync::mpsc::channel;
use std::sync::Arc;
use std::time::Duration;

pub type ReloadCallback = Arc<dyn Fn(&Path) -> Result<(), HotReloadError> + Send + Sync>;

pub struct ScriptWatcher {
    _watcher: RecommendedWatcher,
    error_state: Arc<RwLock<Option<HotReloadError>>>,
    _worker_handle: std::thread::JoinHandle<()>,
}

impl ScriptWatcher {
    pub fn new(
        target_path: impl AsRef<Path>,
        bridge: ReactiveBridge,
        reload_callback: ReloadCallback,
    ) -> notify::Result<Self> {
        let error_state = Arc::new(RwLock::new(None));
        let (event_tx, event_rx) = channel::<PathBuf>();

        let path = target_path.as_ref().to_path_buf();
        let watch_root = if path.is_file() {
            path.parent().unwrap_or(&path).to_path_buf()
        } else {
            path.clone()
        };

        let tx_clone = event_tx.clone();
        let mut watcher = RecommendedWatcher::new(
            move |res: notify::Result<Event>| {
                if let Ok(event) = res {
                    match event.kind {
                        EventKind::Modify(_) | EventKind::Create(_) => {
                            for p in event.paths {
                                if p.extension().is_some_and(|ext| ext == "lua") {
                                    let _ = tx_clone.send(p);
                                }
                            }
                        }
                        _ => {}
                    }
                }
            },
            Config::default(),
        )?;

        watcher.watch(&watch_root, RecursiveMode::Recursive)?;

        // Worker thread with 50ms debounce
        let error_state_worker = error_state.clone();
        let target_file = path.clone();
        let worker_handle = std::thread::spawn(move || {
            while let Ok(first_path) = event_rx.recv() {
                // Debounce: drain events until silence for 50ms
                let mut last_path = first_path;
                while let Ok(next_path) = event_rx.recv_timeout(Duration::from_millis(50)) {
                    last_path = next_path;
                }

                let file_to_reload = if target_file.is_file() {
                    &target_file
                } else {
                    &last_path
                };

                match reload_callback(file_to_reload) {
                    Ok(()) => {
                        *error_state_worker.write() = None;
                        bridge.notify();
                    }
                    Err(err) => {
                        log::error!("Hot reload failed: {}", err.message);
                        *error_state_worker.write() = Some(err);
                        bridge.notify();
                    }
                }
            }
        });

        Ok(Self {
            _watcher: watcher,
            error_state,
            _worker_handle: worker_handle,
        })
    }

    pub fn current_error(&self) -> Option<HotReloadError> {
        self.error_state.read().clone()
    }

    pub fn set_error(&self, err: HotReloadError) {
        *self.error_state.write() = Some(err);
    }

    pub fn clear_error(&self) {
        *self.error_state.write() = None;
    }
}
