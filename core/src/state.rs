//! Reactive state primitives, action dispatching, and change versioning.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

/// A light observable value container. Tracks modifications via an atomic version counter.
#[derive(Debug)]
pub struct Signal<T> {
    value: Mutex<T>,
    version: AtomicU64,
}

impl<T> Signal<T> {
    pub fn new(initial: T) -> Self {
        Self {
            value: Mutex::new(initial),
            version: AtomicU64::new(1),
        }
    }

    pub fn get_cloned(&self) -> T
    where
        T: Clone,
    {
        self.value.lock().unwrap().clone()
    }

    pub fn set(&self, new_val: T) {
        let mut guard = self.value.lock().unwrap();
        *guard = new_val;
        self.version.fetch_add(1, Ordering::Relaxed);
    }

    pub fn update<R>(&self, f: impl FnOnce(&mut T) -> R) -> R {
        let mut guard = self.value.lock().unwrap();
        let r = f(&mut *guard);
        self.version.fetch_add(1, Ordering::Relaxed);
        r
    }

    pub fn version(&self) -> u64 {
        self.version.load(Ordering::Relaxed)
    }
}

/// A decoupled FIFO command/action dispatcher.
#[derive(Debug, Clone)]
pub struct ActionBus<A> {
    queue: Arc<Mutex<Vec<A>>>,
}

impl<A> Default for ActionBus<A> {
    fn default() -> Self {
        Self {
            queue: Arc::new(Mutex::new(Vec::new())),
        }
    }
}

impl<A> ActionBus<A> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn send(&self, action: A) {
        self.queue.lock().unwrap().push(action);
    }

    pub fn drain(&self) -> Vec<A> {
        let mut guard = self.queue.lock().unwrap();
        std::mem::take(&mut *guard)
    }

    pub fn is_empty(&self) -> bool {
        self.queue.lock().unwrap().is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn signal_tracks_mutations_with_monotonically_increasing_versions() {
        let sig = Signal::new(42);
        assert_eq!(sig.get_cloned(), 42);
        let v1 = sig.version();

        sig.set(100);
        assert_eq!(sig.get_cloned(), 100);
        assert!(sig.version() > v1);
    }

    #[test]
    fn action_bus_drains_queued_items() {
        let bus = ActionBus::new();
        bus.send("OPEN_FILE");
        bus.send("SAVE_FILE");

        let actions = bus.drain();
        assert_eq!(actions, vec!["OPEN_FILE", "SAVE_FILE"]);
        assert!(bus.is_empty());
    }
}
