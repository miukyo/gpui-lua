use gpui::{Context, Pixels, Task, px};
use std::time::Duration;

static INTERVAL: Duration = Duration::from_millis(500);
static PAUSE_DELAY: Duration = Duration::from_millis(300);

pub const CURSOR_WIDTH: Pixels = px(2.0);

/// Manages Input cursor blinking.
pub struct BlinkCursor {
    visible: bool,
    paused: bool,
    epoch: usize,
    _task: Task<()>,
}

impl Default for BlinkCursor {
    fn default() -> Self {
        Self::new()
    }
}

impl BlinkCursor {
    pub fn new() -> Self {
        Self {
            visible: true,
            paused: false,
            epoch: 0,
            _task: Task::ready(()),
        }
    }

    /// Start the blinking loop.
    pub fn start(&mut self, cx: &mut Context<Self>) {
        self.visible = true;
        self.paused = false;
        let epoch = self.next_epoch();
        self.schedule_blink(epoch, cx);
    }

    /// Stop the blinking and hide cursor.
    pub fn stop(&mut self, cx: &mut Context<Self>) {
        self.epoch = 0;
        self.paused = false;
        self.visible = false;
        self._task = Task::ready(());
        cx.notify();
    }

    fn next_epoch(&mut self) -> usize {
        self.epoch = self.epoch.wrapping_add(1);
        self.epoch
    }

    fn schedule_blink(&mut self, epoch: usize, cx: &mut Context<Self>) {
        self._task = cx.spawn(async move |this, cx| {
            cx.background_executor().timer(INTERVAL).await;
            if let Some(this) = this.upgrade() {
                let _ = this.update(cx, |this, cx| {
                    if epoch != this.epoch {
                        return;
                    }
                    if !this.paused {
                        this.visible = !this.visible;
                        cx.notify();
                    }
                    let next = this.next_epoch();
                    this.schedule_blink(next, cx);
                });
            }
        });
    }

    pub fn visible(&self) -> bool {
        self.paused || self.visible
    }

    /// Show cursor immediately and pause blinking briefly on keystrokes.
    pub fn pause(&mut self, cx: &mut Context<Self>) {
        self.paused = true;
        self.visible = true;
        cx.notify();

        let epoch = self.next_epoch();
        self._task = cx.spawn(async move |this, cx| {
            cx.background_executor().timer(PAUSE_DELAY).await;
            if let Some(this) = this.upgrade() {
                let _ = this.update(cx, |this, cx| {
                    if epoch != this.epoch {
                        return;
                    }
                    this.paused = false;
                    this.schedule_blink(epoch, cx);
                });
            }
        });
    }
}
