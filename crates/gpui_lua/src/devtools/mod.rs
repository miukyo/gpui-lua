pub mod overlay;
pub mod state;
pub mod view;

pub use overlay::render_devtools_overlay;
pub use state::{
    DevToolsState, DevToolsTab, ElementsSubTab, LogLevel, NetworkEntry, NetworkType,
    StorageSubTab,
};
pub use view::DevToolsView;

use crate::runtime::LuaRuntime;
use gpui::{
    AnyWindowHandle, App, AppContext, Bounds, Pixels, Point, TitlebarOptions, WindowBounds,
    WindowKind, WindowOptions, px, size,
};
use parking_lot::RwLock;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

pub struct DevToolsManager {
    pub state: Arc<DevToolsState>,
    pub window_handle: Arc<RwLock<Option<AnyWindowHandle>>>,
    pub is_open: AtomicBool,
}

impl Default for DevToolsManager {
    fn default() -> Self {
        Self {
            state: DevToolsState::new(),
            window_handle: Arc::new(RwLock::new(None)),
            is_open: AtomicBool::new(false),
        }
    }
}

impl DevToolsManager {
    pub fn new() -> Arc<Self> {
        let state = DevToolsState::new();
        state::set_active_devtools(state.clone());
        Arc::new(Self {
            state,
            window_handle: Arc::new(RwLock::new(None)),
            is_open: AtomicBool::new(false),
        })
    }

    /// Toggle the detached DevTools window.
    pub fn toggle_window(&self, runtime: Arc<LuaRuntime>, cx: &mut App) {
        if self.is_open.load(Ordering::SeqCst) {
            self.close_window(cx);
        } else {
            self.open_window(runtime, cx);
        }
    }

    /// Open the detached DevTools window.
    pub fn open_window(&self, runtime: Arc<LuaRuntime>, cx: &mut App) {
        if self.is_open.load(Ordering::SeqCst) {
            if let Some(handle) = *self.window_handle.read() {
                let _ = cx.update_window(handle, |_, window, _| window.activate());
                return;
            }
        }

        let state = self.state.clone();
        let window_handle_ref = self.window_handle.clone();

        self.is_open.store(true, Ordering::SeqCst);

        let window_options = WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(Bounds {
                origin: Point::new(px(80.0), px(80.0)),
                size: size(px(1040.0), px(720.0)),
            })),
            titlebar: Some(TitlebarOptions {
                title: Some("GPUI DevTools".into()),
                appears_transparent: false,
                traffic_light_position: None,
            }),
            window_min_size: Some(size(px(600.0), px(400.0))),
            is_resizable: true,
            is_minimizable: true,
            is_movable: true,
            focus: true,
            show: true,
            kind: WindowKind::Normal,
            ..Default::default()
        };

        let handle = cx.open_window(window_options, move |_window, cx| {
            cx.new(|cx| DevToolsView::new(state, runtime, cx))
        });

        if let Ok(handle) = handle {
            *window_handle_ref.write() = Some(handle.into());
        }
    }

    /// Close the detached DevTools window.
    pub fn close_window(&self, cx: &mut App) {
        self.is_open.store(false, Ordering::SeqCst);
        self.state.inspect_cursor_active.store(false, Ordering::SeqCst);
        if let Some(handle) = self.window_handle.write().take() {
            let _ = cx.update_window(handle, |_, window, _| {
                window.remove_window();
            });
        }
    }
    /// Toggle inspect cursor mode.
    pub fn toggle_inspect_mode(&self) {
        let current = self.state.inspect_cursor_active.load(Ordering::SeqCst);
        self.state.inspect_cursor_active.store(!current, Ordering::SeqCst);
    }

    /// Record layout bounds for an element in the active element tree.
    pub fn record_node_bounds(&self, path: &[usize], bounds: Bounds<Pixels>) {
        self.state.node_bounds.write().insert(path.to_vec(), bounds);
    }

    /// Hit-test elements under a mouse coordinate, selecting the deepest node.
    pub fn hit_test_inspect(&self, x: Pixels, y: Pixels) -> Option<Vec<usize>> {
        let bounds_map = self.state.node_bounds.read();
        let mut best: Option<(Vec<usize>, f32)> = None;

        for (path, bounds) in bounds_map.iter() {
            let (bx, by, bw, bh) = (
                bounds.origin.x,
                bounds.origin.y,
                bounds.size.width,
                bounds.size.height,
            );

            if x >= bx && x <= bx + bw && y >= by && y <= by + bh {
                let area = f32::from(bw) * f32::from(bh);
                if let Some((_, best_area)) = best {
                    if area < best_area {
                        best = Some((path.clone(), area));
                    }
                } else {
                    best = Some((path.clone(), area));
                }
            }
        }
        best.map(|(path, _)| path)
    }
}
