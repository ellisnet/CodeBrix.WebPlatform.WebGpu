//! WebGPU's uncaptured errors and the device-lost report, collected ONCE per device and handed to
//! any number of readers: [`GpuError`], [`GpuErrorKind`] and [`GpuErrorSubscription`].
//!
//! WebGPU allows ONE uncaptured-error handler and ONE device-lost callback per device; whoever
//! installs a second replaces the first. So the crate installs both itself, exactly once per
//! device, when the first [`GpuContext`](crate::GpuContext) for that device is made (every way of
//! making one - [`GpuContext::request`](crate::GpuContext::request),
//! [`GpuContext::from_parts`](crate::GpuContext::from_parts), on the same device again - shares
//! the one installation). Code that wraps the device (a graphics context, an animation context,
//! the application itself) never installs a handler: it SUBSCRIBES
//! ([`GpuContext::subscribe_errors`](crate::GpuContext::subscribe_errors)).
//!
//! THE SEMANTICS: every subscription sees EVERY error that arrives while it exists, in the order
//! they arrived; taking errors from one subscription ([`GpuErrorSubscription::take`]) never takes
//! them from another. An error that arrives while nobody is subscribed is not kept. A
//! subscription that is never read keeps at most [`GpuErrorSubscription::CAPACITY`] errors (the
//! oldest are let go and counted, [`GpuErrorSubscription::dropped_count`]). Dropping a
//! subscription ends it. Errors caught by an error scope the caller pushed
//! (`wgpu::Device::push_error_scope`) are the scope's and never reach a subscription.

use std::cell::RefCell;
use std::collections::VecDeque;
use std::fmt;
use std::sync::{Arc, Mutex, Weak};

/// The kind of a [`GpuError`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GpuErrorKind {
    /// A validation error: the application asked for something WebGPU does not allow (a bug in
    /// the calling code or its data).
    Validation,
    /// The device ran out of memory.
    OutOfMemory,
    /// An internal error of the browser's WebGPU implementation (often a system limit).
    Internal,
}

impl fmt::Display for GpuErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            GpuErrorKind::Validation => "Validation",
            GpuErrorKind::OutOfMemory => "OutOfMemory",
            GpuErrorKind::Internal => "Internal",
        })
    }
}

/// One error WebGPU reported for work that was not inside an error scope.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GpuError {
    kind: GpuErrorKind,
    message: String,
}

impl GpuError {
    /// An error of `kind` with WebGPU's message.
    pub fn new(kind: GpuErrorKind, message: impl Into<String>) -> GpuError {
        GpuError { kind, message: message.into() }
    }

    /// The kind of error.
    pub fn kind(&self) -> GpuErrorKind {
        self.kind
    }

    /// WebGPU's message.
    pub fn message(&self) -> &str {
        &self.message
    }

    /// The `wgpu` error as a [`GpuError`].
    pub fn from_wgpu(error: &wgpu::Error) -> GpuError {
        let kind = match error {
            wgpu::Error::Validation { .. } => GpuErrorKind::Validation,
            wgpu::Error::OutOfMemory { .. } => GpuErrorKind::OutOfMemory,
            wgpu::Error::Internal { .. } => GpuErrorKind::Internal,
        };
        GpuError::new(kind, error.to_string())
    }
}

impl fmt::Display for GpuError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.kind, self.message)
    }
}

impl std::error::Error for GpuError {}

/// One reader's view of a device's uncaptured errors (made by
/// [`GpuContext::subscribe_errors`](crate::GpuContext::subscribe_errors)). It sees EVERY error
/// that arrives while it exists, in order; taking errors from it never takes them from another
/// subscription; an error that arrives while nobody is subscribed is not kept; unread, it keeps
/// at most [`GpuErrorSubscription::CAPACITY`] errors (the oldest are let go and counted); dropping
/// it ends it. Errors caught by an error scope the caller pushed are the scope's and never reach
/// it. The browser hands an error over when the device does work (a page that draws submits
/// every frame), so an error may arrive a frame or so after the call that caused it.
pub struct GpuErrorSubscription {
    id: u64,
    hub: Arc<Mutex<HubState>>,
}

impl GpuErrorSubscription {
    /// The most errors a subscription keeps unread; when more arrive, the oldest are let go (and
    /// counted by [`GpuErrorSubscription::dropped_count`]).
    pub const CAPACITY: usize = 256;

    /// Hands out, oldest first, the errors that arrived since the last call (or since the
    /// subscription began), and forgets them. Other subscriptions keep their own copies.
    pub fn take(&self) -> Vec<GpuError> {
        match self.hub.lock() {
            Ok(mut state) => state.take(self.id),
            Err(_) => Vec::new(),
        }
    }

    /// How many errors this subscription has let go unread because it held
    /// [`GpuErrorSubscription::CAPACITY`] already (read it more often to lose none).
    pub fn dropped_count(&self) -> u64 {
        match self.hub.lock() {
            Ok(state) => state.dropped(self.id),
            Err(_) => 0,
        }
    }

    /// The browser's reason for losing the device, once it is lost (the same for every
    /// subscription of the device).
    pub fn lost_reason(&self) -> Option<String> {
        self.hub.lock().ok().and_then(|state| state.lost.clone())
    }
}

impl Drop for GpuErrorSubscription {
    fn drop(&mut self) {
        if let Ok(mut state) = self.hub.lock() {
            state.unsubscribe(self.id);
        }
    }
}

impl fmt::Debug for GpuErrorSubscription {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "GpuErrorSubscription({})", self.id)
    }
}

/// The one collection point of a device: what its single handler and callback write into.
pub(crate) struct ErrorHub {
    state: Arc<Mutex<HubState>>,
}

impl ErrorHub {
    /// The hub of `device`: the one already installed for it, or a new one that installs the
    /// device's uncaptured-error handler and device-lost callback now (once per device).
    pub(crate) fn for_device(device: &wgpu::Device) -> ErrorHub {
        HUBS.with(|hubs| {
            let mut hubs = hubs.borrow_mut();
            hubs.retain(|(_, state)| state.strong_count() > 0);
            if let Some(state) = hubs.iter().find(|(known, _)| known == device).and_then(|(_, state)| state.upgrade()) {
                return ErrorHub { state };
            }
            let state = Arc::new(Mutex::new(HubState::default()));
            install(device, &state);
            hubs.push((device.clone(), Arc::downgrade(&state)));
            ErrorHub { state }
        })
    }

    /// A new subscription to the hub.
    pub(crate) fn subscribe(&self) -> GpuErrorSubscription {
        let id = match self.state.lock() {
            Ok(mut state) => state.subscribe(),
            Err(_) => 0,
        };
        GpuErrorSubscription { id, hub: self.state.clone() }
    }

    /// The browser's reason for losing the device, once it is lost.
    pub(crate) fn lost_reason(&self) -> Option<String> {
        self.state.lock().ok().and_then(|state| state.lost.clone())
    }
}

thread_local! {
    /// The devices that have a hub, and their hub (weak: a hub lives while a context of its
    /// device does). A browser page runs on one thread.
    static HUBS: RefCell<Vec<(wgpu::Device, Weak<Mutex<HubState>>)>> = const { RefCell::new(Vec::new()) };
}

/// Installs the device's ONE uncaptured-error handler and ONE device-lost callback, both writing
/// into `state` for as long as it lives.
fn install(device: &wgpu::Device, state: &Arc<Mutex<HubState>>) {
    let errors = Arc::downgrade(state);
    device.on_uncaptured_error(Arc::new(move |error: wgpu::Error| {
        if let Some(state) = errors.upgrade() {
            if let Ok(mut state) = state.lock() {
                state.publish(GpuError::from_wgpu(&error));
            }
        }
    }));
    let lost = Arc::downgrade(state);
    device.set_device_lost_callback(move |reason, message| {
        if let Some(state) = lost.upgrade() {
            if let Ok(mut state) = state.lock() {
                state.lost = Some(format!("{reason:?}: {message}"));
            }
        }
    });
}

/// The queues of the subscriptions of one device (CPU only, so that it is tested on the host).
#[derive(Default)]
pub(crate) struct HubState {
    next_id: u64,
    subscribers: Vec<Subscriber>,
    lost: Option<String>,
}

struct Subscriber {
    id: u64,
    queue: VecDeque<GpuError>,
    dropped: u64,
}

impl HubState {
    pub(crate) fn subscribe(&mut self) -> u64 {
        self.next_id += 1;
        self.subscribers.push(Subscriber { id: self.next_id, queue: VecDeque::new(), dropped: 0 });
        self.next_id
    }

    pub(crate) fn unsubscribe(&mut self, id: u64) {
        self.subscribers.retain(|s| s.id != id);
    }

    /// Hands a copy of the error to every subscription.
    pub(crate) fn publish(&mut self, error: GpuError) {
        for subscriber in &mut self.subscribers {
            if subscriber.queue.len() >= GpuErrorSubscription::CAPACITY {
                subscriber.queue.pop_front();
                subscriber.dropped += 1;
            }
            subscriber.queue.push_back(error.clone());
        }
    }

    pub(crate) fn take(&mut self, id: u64) -> Vec<GpuError> {
        match self.subscribers.iter_mut().find(|s| s.id == id) {
            Some(subscriber) => subscriber.queue.drain(..).collect(),
            None => Vec::new(),
        }
    }

    pub(crate) fn dropped(&self, id: u64) -> u64 {
        self.subscribers.iter().find(|s| s.id == id).map_or(0, |s| s.dropped)
    }

    #[cfg(test)]
    pub(crate) fn subscriber_count(&self) -> usize {
        self.subscribers.len()
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use super::{GpuError, GpuErrorKind, GpuErrorSubscription, HubState};

    fn error(n: usize) -> GpuError {
        GpuError::new(GpuErrorKind::Validation, format!("error {n}"))
    }

    fn subscription(hub: &Arc<Mutex<HubState>>) -> GpuErrorSubscription {
        let id = hub.lock().unwrap().subscribe();
        GpuErrorSubscription { id, hub: hub.clone() }
    }

    #[test]
    fn every_subscriber_sees_every_error_in_order() {
        let hub = Arc::new(Mutex::new(HubState::default()));
        let graphics = subscription(&hub);
        let animation = subscription(&hub);
        hub.lock().unwrap().publish(error(1));
        hub.lock().unwrap().publish(error(2));
        assert_eq!(graphics.take(), vec![error(1), error(2)]);
        assert_eq!(animation.take(), vec![error(1), error(2)]);
    }

    #[test]
    fn taking_from_one_subscriber_leaves_the_others_errors() {
        let hub = Arc::new(Mutex::new(HubState::default()));
        let first = subscription(&hub);
        let second = subscription(&hub);
        hub.lock().unwrap().publish(error(1));
        assert_eq!(first.take().len(), 1);
        assert!(first.take().is_empty());
        hub.lock().unwrap().publish(error(2));
        assert_eq!(second.take(), vec![error(1), error(2)]);
        assert_eq!(first.take(), vec![error(2)]);
    }

    #[test]
    fn a_subscriber_sees_only_errors_after_it_began() {
        let hub = Arc::new(Mutex::new(HubState::default()));
        hub.lock().unwrap().publish(error(1));
        let late = subscription(&hub);
        hub.lock().unwrap().publish(error(2));
        assert_eq!(late.take(), vec![error(2)]);
    }

    #[test]
    fn dropping_a_subscription_ends_it() {
        let hub = Arc::new(Mutex::new(HubState::default()));
        let kept = subscription(&hub);
        let gone = subscription(&hub);
        assert_eq!(hub.lock().unwrap().subscriber_count(), 2);
        drop(gone);
        assert_eq!(hub.lock().unwrap().subscriber_count(), 1);
        hub.lock().unwrap().publish(error(1));
        assert_eq!(kept.take(), vec![error(1)]);
    }

    #[test]
    fn an_unread_subscription_keeps_the_newest_errors_and_counts_the_rest() {
        let hub = Arc::new(Mutex::new(HubState::default()));
        let reader = subscription(&hub);
        let total = GpuErrorSubscription::CAPACITY + 3;
        for n in 0..total {
            hub.lock().unwrap().publish(error(n));
        }
        assert_eq!(reader.dropped_count(), 3);
        let errors = reader.take();
        assert_eq!(errors.len(), GpuErrorSubscription::CAPACITY);
        assert_eq!(errors[0], error(3));
        assert_eq!(errors.last(), Some(&error(total - 1)));
    }

    #[test]
    fn the_lost_reason_is_shared() {
        let hub = Arc::new(Mutex::new(HubState::default()));
        let first = subscription(&hub);
        let second = subscription(&hub);
        assert_eq!(first.lost_reason(), None);
        hub.lock().unwrap().lost = Some("Destroyed: gone".to_string());
        assert_eq!(second.lost_reason().as_deref(), Some("Destroyed: gone"));
        assert_eq!(first.lost_reason().as_deref(), Some("Destroyed: gone"));
    }

    #[test]
    fn display_is_the_kind_then_the_message() {
        assert_eq!(error(7).to_string(), "Validation: error 7");
        assert_eq!(GpuErrorKind::OutOfMemory.to_string(), "OutOfMemory");
    }
}
