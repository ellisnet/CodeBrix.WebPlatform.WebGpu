//! A one-shot value delivered by a `wgpu` callback (a buffer map) and awaited as a future, so
//! that asynchronous readback needs no channel crate.

use std::cell::RefCell;
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll, Waker};

/// The shared slot: the value once it arrives, and the waker of the task that awaits it.
struct Slot<T> {
    value: Option<T>,
    waker: Option<Waker>,
}

/// The sending half: [`Sender::send`] stores the value and wakes the receiver.
pub(crate) struct Sender<T> {
    slot: Arc<Mutex<Slot<T>>>,
}

/// The receiving half: a future of the value.
pub(crate) struct Receiver<T> {
    slot: Arc<Mutex<Slot<T>>>,
    // Not Send: the library runs on the page's one thread.
    _local: Rc<RefCell<()>>,
}

/// A connected sender and receiver.
pub(crate) fn channel<T>() -> (Sender<T>, Receiver<T>) {
    let slot = Arc::new(Mutex::new(Slot { value: None, waker: None }));
    (Sender { slot: slot.clone() }, Receiver { slot, _local: Rc::new(RefCell::new(())) })
}

impl<T> Sender<T> {
    /// Delivers the value (once) and wakes the task that awaits it.
    pub(crate) fn send(self, value: T) {
        let waker = match self.slot.lock() {
            Ok(mut slot) => {
                slot.value = Some(value);
                slot.waker.take()
            }
            Err(_) => None,
        };
        if let Some(waker) = waker {
            waker.wake();
        }
    }
}

impl<T> Future for Receiver<T> {
    type Output = T;

    fn poll(self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<T> {
        let mut slot = match self.slot.lock() {
            Ok(slot) => slot,
            Err(poisoned) => poisoned.into_inner(),
        };
        match slot.value.take() {
            Some(value) => Poll::Ready(value),
            None => {
                slot.waker = Some(context.waker().clone());
                Poll::Pending
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::future::Future;
    use std::pin::pin;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;
    use std::task::{Context, Poll, Wake, Waker};

    use super::channel;

    struct CountingWaker(AtomicUsize);

    impl Wake for CountingWaker {
        fn wake(self: Arc<Self>) {
            self.0.fetch_add(1, Ordering::SeqCst);
        }
    }

    #[test]
    fn the_receiver_waits_until_the_value_is_sent_and_is_woken() {
        let counter = Arc::new(CountingWaker(AtomicUsize::new(0)));
        let waker = Waker::from(counter.clone());
        let mut context = Context::from_waker(&waker);
        let (sender, receiver) = channel::<u32>();
        let mut receiver = pin!(receiver);
        assert_eq!(receiver.as_mut().poll(&mut context), Poll::Pending);
        sender.send(7);
        assert_eq!(counter.0.load(Ordering::SeqCst), 1);
        assert_eq!(receiver.as_mut().poll(&mut context), Poll::Ready(7));
    }
}
