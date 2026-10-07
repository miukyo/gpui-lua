use parking_lot::RwLock;
use std::sync::Arc;

#[derive(Clone, Default)]
pub struct ReactiveBridge {
    sender: Arc<RwLock<Option<async_channel::Sender<()>>>>,
}

impl ReactiveBridge {
    pub fn new() -> Self {
        Self {
            sender: Arc::new(RwLock::new(None)),
        }
    }

    pub fn set_sender(&self, sender: async_channel::Sender<()>) {
        *self.sender.write() = Some(sender);
    }

    pub fn notify(&self) {
        if let Some(sender) = self.sender.read().as_ref() {
            let _ = sender.try_send(());
        }
    }
}
