use tokio::sync::broadcast::{self, Receiver, Sender};
use uuid::Uuid;

use crate::domain::Event;

const CAPACITY: usize = 256;

#[derive(Clone)]
pub struct EventBus {
    tx: Sender<(Uuid, Event)>,
}

impl EventBus {
    pub fn new() -> Self {
        let (tx, _) = broadcast::channel(CAPACITY);
        Self { tx }
    }

    pub fn publish(&self, dealership_id: Uuid, event: Event) {
        let _ = self.tx.send((dealership_id, event));
    }

    pub fn subscribe(&self) -> Receiver<(Uuid, Event)> {
        self.tx.subscribe()
    }
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new()
    }
}
