use parking_lot::RwLock;
use std::sync::Arc;

#[derive(Clone, Default)]
pub struct ReactiveBridge {
    senders: Arc<RwLock<Vec<async_channel::Sender<()>>>>,
}

impl ReactiveBridge {
    pub fn new() -> Self {
        Self {
            senders: Arc::new(RwLock::new(Vec::new())),
        }
    }

    pub fn add_sender(&self, sender: async_channel::Sender<()>) {
        self.senders.write().push(sender);
    }

    pub fn set_sender(&self, sender: async_channel::Sender<()>) {
        let mut senders = self.senders.write();
        senders.clear();
        senders.push(sender);
    }

    pub fn notify(&self) {
        let mut senders = self.senders.write();
        senders.retain(|s| !s.is_closed());
        for sender in senders.iter() {
            let _ = sender.try_send(());
        }
    }
}
