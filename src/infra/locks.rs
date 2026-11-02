use std::collections::HashMap;
use std::sync::{Arc, Mutex, PoisonError};
use std::time::Duration;

use tokio::sync::OwnedMutexGuard;
use tokio::time;
use uuid::Uuid;

#[derive(Clone)]
pub struct Locks {
    inner: Arc<Inner>,
}

impl Locks {
    pub fn new(timeout: Duration) -> Self {
        Self {
            inner: Arc::new(Inner {
                entries: Mutex::new(HashMap::new()),
                timeout,
            }),
        }
    }

    pub async fn acquire(&self, key: Uuid) -> Result<LockGuard, LockTimeout> {
        let entry = {
            let mut entries = self
                .inner
                .entries
                .lock()
                .unwrap_or_else(PoisonError::into_inner);
            Arc::clone(entries.entry(key).or_default())
        };
        match time::timeout(self.inner.timeout, entry.lock_owned()).await {
            Ok(guard) => Ok(LockGuard {
                guard: Some(guard),
                inner: Arc::clone(&self.inner),
                key,
            }),
            Err(_) => {
                self.inner.prune(key);
                Err(LockTimeout)
            }
        }
    }
}

#[derive(Debug)]
pub struct LockTimeout;

pub struct LockGuard {
    guard: Option<OwnedMutexGuard<()>>,
    inner: Arc<Inner>,
    key: Uuid,
}

impl Drop for LockGuard {
    fn drop(&mut self) {
        self.guard.take();
        self.inner.prune(self.key);
    }
}

struct Inner {
    entries: Mutex<HashMap<Uuid, Arc<tokio::sync::Mutex<()>>>>,
    timeout: Duration,
}

impl Inner {
    fn prune(&self, key: Uuid) {
        let mut entries = self.entries.lock().unwrap_or_else(PoisonError::into_inner);
        if entries.get(&key).is_some_and(|e| Arc::strong_count(e) == 1) {
            entries.remove(&key);
        }
    }
}
