//! Event bus implementations.

use crate::ports::{EventBus, SubscriptionBus};
use crate::query::Event;
use std::sync::{Mutex, RwLock, mpsc};
use tesela_core::{ApiName, Error, lock_mutex};

/// Event bus that discards all events.
pub struct NoopEventBus;

impl EventBus for NoopEventBus {
    fn publish(&self, _event: Event) -> Result<(), Error> {
        Ok(())
    }
}

/// Event bus that stores events in a Vec for testing.
pub struct VecEventBus {
    events: Mutex<Vec<Event>>,
}

impl VecEventBus {
    /// Create a new vec bus.
    pub fn new() -> Self {
        Self {
            events: Mutex::new(Vec::new()),
        }
    }

    /// Drain all events.
    pub fn drain(&self) -> Result<Vec<Event>, Error> {
        Ok(lock_mutex(&self.events)?.drain(..).collect())
    }
}

impl Default for VecEventBus {
    fn default() -> Self {
        Self::new()
    }
}

impl EventBus for VecEventBus {
    fn publish(&self, event: Event) -> Result<(), Error> {
        lock_mutex(&self.events)?.push(event);
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// BroadcastEventBus — fan-out to multiple subscribers
// ---------------------------------------------------------------------------

/// Subscription handle: an optional object-type filter paired with a sender.
struct Subscription {
    object_type_filter: Option<ApiName>,
    tx: mpsc::SyncSender<Event>,
}

/// Event bus that fans published events out to all active subscribers.
///
/// Subscribers call [`SubscriptionBus::subscribe`] to obtain an
/// `mpsc::Receiver<Event>`.  Stale senders (where the receiver has been
/// dropped) are pruned automatically on each publish.
pub struct BroadcastEventBus {
    subscriptions: RwLock<Vec<Subscription>>,
}

impl BroadcastEventBus {
    /// Create a new broadcast bus with no subscribers.
    pub fn new() -> Self {
        Self {
            subscriptions: RwLock::new(Vec::new()),
        }
    }
}

impl Default for BroadcastEventBus {
    fn default() -> Self {
        Self::new()
    }
}

impl EventBus for BroadcastEventBus {
    fn publish(&self, event: Event) -> Result<(), Error> {
        let mut subs = self
            .subscriptions
            .write()
            .map_err(|_| Error::internal("broadcast event bus lock poisoned"))?;

        // Fan out, collecting indices of dead subscriptions.
        let mut dead: Vec<usize> = Vec::new();
        for (i, sub) in subs.iter().enumerate() {
            let matches = sub
                .object_type_filter
                .as_ref()
                .is_none_or(|f| event.object_type.as_deref() == Some(f.as_ref()));
            if matches && sub.tx.try_send(event.clone()).is_err() {
                dead.push(i);
            }
        }

        // Prune in reverse order to preserve indices.
        for i in dead.into_iter().rev() {
            subs.swap_remove(i);
        }
        Ok(())
    }
}

impl SubscriptionBus for BroadcastEventBus {
    fn subscribe(&self, object_type: Option<&ApiName>) -> Result<mpsc::Receiver<Event>, Error> {
        // Channel capacity of 1024 events per subscriber before back-pressure.
        let (tx, rx) = mpsc::sync_channel(1024);
        self.subscriptions
            .write()
            .map_err(|_| Error::internal("broadcast event bus lock poisoned"))?
            .push(Subscription {
                object_type_filter: object_type.cloned(),
                tx,
            });
        Ok(rx)
    }
}
