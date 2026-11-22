//! [`WebGpuBoundary`]: a part of the page that exists only where WebGPU works, with an optional
//! fallback in its place where it does not; and [`use_gpu`], which hands the boundary's context to
//! the components inside it.
//!
//! THE ONE REQUEST OF THE PAGE: every boundary of the page shares one answer. The first boundary
//! to appear asks the browser ([`GpuContext::request`]); boundaries that appear while it waits wait
//! for the same answer, and boundaries that appear later take it at once. So a page with several
//! boundaries asks the browser once and draws on one device. Two cases ask again: the asking
//! boundary goes away before the browser has answered (the next boundary that waits asks in its
//! place), and the device of the answer has been lost (the next boundary to appear asks for a new
//! one).
//!
//! HOW IT ASKS: by default exactly as [`GpuContext::request`] does. A page that needs more of the
//! device - a power preference, features (required, or optional: asked for only where the adapter
//! has them), limits - says so ONCE, before its first boundary appears, with
//! [`set_page_gpu_options`] ([`PageGpuOptions`]).

use std::cell::RefCell;
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;
use std::task::{Context, Poll, Waker};

use dioxus::prelude::*;

use crate::context::{GpuContext, GpuUnavailable};

/// A part of the page that is made only where WebGPU works. Where it does not, the boundary shows
/// its `fallback` instead (given the reason), or nothing when it has none, and the part is never
/// made: its components do not run, so they load nothing and show no message.
///
/// The components inside read the context with [`use_gpu`]. While the browser is still answering
/// (usually a few milliseconds) the boundary shows nothing; give the space around it a fixed size
/// if the page must not move when the part appears.
///
/// Every boundary of the page shares ONE request to the browser and one device (see the module's
/// documentation). The request asks as [`GpuContext::request`] does, unless the page said otherwise
/// with [`set_page_gpu_options`] before its first boundary appeared.
///
/// ```ignore
/// // Nothing at all where WebGPU is not available:
/// WebGpuBoundary { HeroAnimation {} }
///
/// // A picture in its place:
/// WebGpuBoundary {
///     fallback: |_| rsx! { img { src: "/hero-still.webp" } },
///     HeroAnimation {}
/// }
/// ```
#[component]
pub fn WebGpuBoundary(
    /// What to show where WebGPU is not available, given the reason. Leave it out to show nothing.
    fallback: Option<Callback<GpuUnavailable, Element>>,
    /// The part of the page that draws with WebGPU. It is made only once WebGPU is available.
    children: Element,
) -> Element {
    let answer = use_resource(page_gpu);
    let state = answer.read();
    match &*state {
        None => rsx! {},
        Some(Err(reason)) => match fallback {
            Some(fallback) => fallback.call(reason.clone()),
            None => rsx! {},
        },
        Some(Ok(context)) => rsx! { GpuProvider { context: context.clone(), {children} } },
    }
}

/// The context of the [`WebGpuBoundary`] this component is inside: the page's one WebGPU context.
/// Clone it freely; every clone is the same device.
///
/// # Panics
/// When the component is not inside a [`WebGpuBoundary`] (nor below a context the application
/// provided itself with `use_context_provider`).
pub fn use_gpu() -> GpuContext {
    match try_use_context::<GpuContext>() {
        Some(context) => context,
        None => panic!("use_gpu() was called in a component that is not inside a WebGpuBoundary: put the component inside one."),
    }
}

/// Provides the context to the boundary's children.
#[component]
fn GpuProvider(context: GpuContext, children: Element) -> Element {
    use_context_provider(|| context);
    children
}

/// How the page's ONE request for WebGPU (the one every [`WebGpuBoundary`] shares) asks for the
/// adapter and the device. The default asks exactly as [`GpuContext::request`] does: no power
/// preference, no features, WebGPU's default limits.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct PageGpuOptions {
    /// Which adapter the browser should prefer (the default: no preference).
    pub power_preference: wgpu::PowerPreference,
    /// Features the device MUST have. An adapter without one of them gives
    /// [`GpuUnavailable::NoDevice`] (so the boundaries show their fallbacks).
    pub required_features: wgpu::Features,
    /// Features asked for only WHERE THE ADAPTER HAS THEM: a browser without one still gives a
    /// device, without that feature. Read `use_gpu().device().features()` to see which it got.
    pub optional_features: wgpu::Features,
    /// The device's limits; `None` asks for WebGPU's default limits. Limits the adapter cannot give
    /// make [`GpuUnavailable::NoDevice`].
    pub required_limits: Option<wgpu::Limits>,
}

impl PageGpuOptions {
    /// The features the device is asked for, given the adapter's: every required feature, and the
    /// optional ones the adapter has.
    pub fn features_for(&self, adapter_features: wgpu::Features) -> wgpu::Features {
        self.required_features | (self.optional_features & adapter_features)
    }
}

/// Says how the page's one request for WebGPU asks (see [`PageGpuOptions`]). Call it ONCE, before
/// the page's first [`WebGpuBoundary`] appears - at the top of the application's root component:
///
/// ```ignore
/// #[component]
/// fn App() -> Element {
///     use_hook(|| set_page_gpu_options(PageGpuOptions { optional_features: wgpu::Features::TIMESTAMP_QUERY, ..PageGpuOptions::default() }));
///     rsx! { WebGpuBoundary { fallback: |_| rsx! { p { "..." } }, Scenes {} } }
/// }
/// ```
///
/// Options set after the page has asked do not change the answer it has: they are used only if the
/// page has to ask again (after its device was lost).
pub fn set_page_gpu_options(options: PageGpuOptions) {
    PAGE_GPU_OPTIONS.with(|current| *current.borrow_mut() = options);
}

thread_local! {
    /// The page's one answer to "may this page use WebGPU?".
    static PAGE_GPU: SharedAnswer<Result<GpuContext, GpuUnavailable>> = SharedAnswer::new();
    /// How the page asks.
    static PAGE_GPU_OPTIONS: RefCell<PageGpuOptions> = RefCell::new(PageGpuOptions::default());
}

/// The page's WebGPU context: asked for once (with the page's options), shared by every boundary.
async fn page_gpu() -> Result<GpuContext, GpuUnavailable> {
    let shared = PAGE_GPU.with(SharedAnswer::clone);
    let options = PAGE_GPU_OPTIONS.with(|current| current.borrow().clone());
    shared.get(move || request_page(options), |answer| matches!(answer, Ok(context) if context.is_lost())).await
}

/// Asks the browser for an adapter and a device with the page's options.
async fn request_page(options: PageGpuOptions) -> Result<GpuContext, GpuUnavailable> {
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
    let adapter_options = wgpu::RequestAdapterOptions { power_preference: options.power_preference, ..wgpu::RequestAdapterOptions::default() };
    let adapter = instance.request_adapter(&adapter_options).await.map_err(|_| GpuUnavailable::NoAdapter)?;
    let descriptor = wgpu::DeviceDescriptor {
        label: Some("codebrix-webgpu device"),
        required_features: options.features_for(adapter.features()),
        required_limits: options.required_limits.clone().unwrap_or_default(),
        ..wgpu::DeviceDescriptor::default()
    };
    let (device, queue) = adapter.request_device(&descriptor).await.map_err(|error| GpuUnavailable::NoDevice(error.to_string()))?;
    Ok(GpuContext::from_parts(instance, adapter, device, queue))
}

/// One answer that any number of callers wait for, asked for by the first of them only.
struct SharedAnswer<T> {
    state: Rc<RefCell<State<T>>>,
}

impl<T> Clone for SharedAnswer<T> {
    fn clone(&self) -> Self {
        SharedAnswer { state: self.state.clone() }
    }
}

enum State<T> {
    /// Nobody has asked yet (or the asker went away before the answer came).
    NotAsked,
    /// One caller is asking; the others wait (their wakers).
    Asking(Vec<Waker>),
    /// The answer, for everybody.
    Answered(T),
}

impl<T: Clone> SharedAnswer<T> {
    fn new() -> Self {
        SharedAnswer { state: Rc::new(RefCell::new(State::NotAsked)) }
    }

    /// The answer: asked for with `ask` when nobody has asked yet (or the answer is `stale`),
    /// otherwise the answer another caller is waiting for or has already had.
    async fn get<F, Fut>(&self, ask: F, stale: impl Fn(&T) -> bool) -> T
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = T>,
    {
        let mut ask = Some(ask);
        loop {
            // The state is borrowed only inside this block, never across an await.
            let asking = {
                let mut state = self.state.borrow_mut();
                let asking = match &*state {
                    State::Answered(answer) if !stale(answer) => return answer.clone(),
                    State::Asking(_) => false,
                    _ => true,
                };
                if asking {
                    *state = State::Asking(Vec::new());
                }
                asking
            };
            if asking {
                // A caller asks once at most: it returns with the answer it asked for.
                let ask = ask.take().expect("a caller asks once at most");
                let mut guard = AskGuard { shared: self, answered: false };
                let answer = ask().await;
                guard.answer(answer.clone());
                return answer;
            }
            Changed { shared: self, waiting: false }.await;
        }
    }

    /// Sets the state and wakes every waiting caller.
    fn settle(&self, new_state: State<T>) {
        let old = std::mem::replace(&mut *self.state.borrow_mut(), new_state);
        if let State::Asking(wakers) = old {
            for waker in wakers {
                waker.wake();
            }
        }
    }
}

/// The asking caller's promise to settle the state: with the answer, or - when the caller goes
/// away before it has one - back to "not asked", so that a waiting caller asks in its place.
struct AskGuard<'a, T: Clone> {
    shared: &'a SharedAnswer<T>,
    answered: bool,
}

impl<T: Clone> AskGuard<'_, T> {
    fn answer(&mut self, answer: T) {
        self.answered = true;
        self.shared.settle(State::Answered(answer));
    }
}

impl<T: Clone> Drop for AskGuard<'_, T> {
    fn drop(&mut self) {
        if !self.answered {
            self.shared.settle(State::NotAsked);
        }
    }
}

/// Waits until the state may have changed (the asking caller settled it).
struct Changed<'a, T> {
    shared: &'a SharedAnswer<T>,
    waiting: bool,
}

impl<T> Future for Changed<'_, T> {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
        if self.waiting {
            return Poll::Ready(());
        }
        let shared = self.shared;
        match &mut *shared.state.borrow_mut() {
            State::Asking(wakers) => wakers.push(cx.waker().clone()),
            _ => return Poll::Ready(()),
        }
        self.waiting = true;
        Poll::Pending
    }
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;
    use std::future::Future;
    use std::pin::{pin, Pin};
    use std::rc::Rc;
    use std::task::{Context, Poll, Waker};

    use super::{set_page_gpu_options, PageGpuOptions, SharedAnswer, PAGE_GPU_OPTIONS};

    /// Polls a future once.
    fn poll_once<F: Future>(future: Pin<&mut F>) -> Poll<F::Output> {
        future.poll(&mut Context::from_waker(Waker::noop()))
    }

    /// A future that waits until its gate is opened.
    struct Gate(Rc<Cell<bool>>);

    impl Future for Gate {
        type Output = ();

        fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<()> {
            if self.0.get() {
                Poll::Ready(())
            } else {
                Poll::Pending
            }
        }
    }

    /// An ask that counts its calls, waits for the gate, then answers `value`.
    fn ask(count: &Rc<Cell<u32>>, gate: &Rc<Cell<bool>>, value: u32) -> impl FnOnce() -> Pin<Box<dyn Future<Output = u32>>> {
        let count = count.clone();
        let gate = gate.clone();
        move || {
            Box::pin(async move {
                count.set(count.get() + 1);
                Gate(gate).await;
                value
            })
        }
    }

    #[test]
    fn callers_that_wait_together_share_one_request() {
        //Arrange
        let shared = SharedAnswer::<u32>::new();
        let count = Rc::new(Cell::new(0));
        let gate = Rc::new(Cell::new(false));
        let mut first = pin!(shared.get(ask(&count, &gate, 7), |_| false));
        let mut second = pin!(shared.get(ask(&count, &gate, 9), |_| false));

        //Act
        let first_before = poll_once(first.as_mut());
        let second_before = poll_once(second.as_mut());
        gate.set(true);
        let first_after = poll_once(first.as_mut());
        let second_after = poll_once(second.as_mut());

        //Assert
        assert_eq!(first_before, Poll::Pending);
        assert_eq!(second_before, Poll::Pending);
        assert_eq!(first_after, Poll::Ready(7));
        assert_eq!(second_after, Poll::Ready(7));
        assert_eq!(count.get(), 1);
    }

    #[test]
    fn a_later_caller_takes_the_answer_without_asking() {
        //Arrange
        let shared = SharedAnswer::<u32>::new();
        let count = Rc::new(Cell::new(0));
        let gate = Rc::new(Cell::new(true));
        let first = pin!(shared.get(ask(&count, &gate, 7), |_| false));
        assert_eq!(poll_once(first), Poll::Ready(7));

        //Act
        let later = poll_once(pin!(shared.get(ask(&count, &gate, 9), |_| false)));

        //Assert
        assert_eq!(later, Poll::Ready(7));
        assert_eq!(count.get(), 1);
    }

    #[test]
    fn a_waiting_caller_asks_when_the_asker_goes_away() {
        //Arrange
        let shared = SharedAnswer::<u32>::new();
        let count = Rc::new(Cell::new(0));
        let closed = Rc::new(Cell::new(false));
        let open = Rc::new(Cell::new(true));
        let mut first = Box::pin(shared.get(ask(&count, &closed, 7), |_| false));
        let mut second = pin!(shared.get(ask(&count, &open, 9), |_| false));
        assert_eq!(poll_once(first.as_mut()), Poll::Pending);
        assert_eq!(poll_once(second.as_mut()), Poll::Pending);

        //Act
        drop(first);
        let second_after = poll_once(second.as_mut());

        //Assert
        assert_eq!(second_after, Poll::Ready(9));
        assert_eq!(count.get(), 2);
    }

    #[test]
    fn a_stale_answer_is_asked_for_again() {
        //Arrange
        let shared = SharedAnswer::<u32>::new();
        let count = Rc::new(Cell::new(0));
        let gate = Rc::new(Cell::new(true));
        assert_eq!(poll_once(pin!(shared.get(ask(&count, &gate, 7), |_| false))), Poll::Ready(7));

        //Act
        let again = poll_once(pin!(shared.get(ask(&count, &gate, 9), |answer| *answer == 7)));

        //Assert
        assert_eq!(again, Poll::Ready(9));
        assert_eq!(count.get(), 2);
    }

    #[test]
    fn the_default_options_ask_as_request_does() {
        //Arrange
        let options = PageGpuOptions::default();

        //Act
        let features = options.features_for(wgpu::Features::all());

        //Assert
        assert_eq!(options.power_preference, wgpu::PowerPreference::None);
        assert_eq!(features, wgpu::Features::empty());
        assert_eq!(options.required_limits, None);
    }

    #[test]
    fn optional_features_are_asked_for_only_where_the_adapter_has_them() {
        //Arrange
        let options = PageGpuOptions { required_features: wgpu::Features::DEPTH_CLIP_CONTROL, optional_features: wgpu::Features::TIMESTAMP_QUERY | wgpu::Features::SHADER_F16, ..PageGpuOptions::default() };

        //Act
        let with = options.features_for(wgpu::Features::TIMESTAMP_QUERY);
        let without = options.features_for(wgpu::Features::empty());

        //Assert
        assert_eq!(with, wgpu::Features::DEPTH_CLIP_CONTROL | wgpu::Features::TIMESTAMP_QUERY);
        assert_eq!(without, wgpu::Features::DEPTH_CLIP_CONTROL);
    }

    #[test]
    fn the_page_options_are_the_ones_set_last() {
        //Arrange
        let options = PageGpuOptions { power_preference: wgpu::PowerPreference::HighPerformance, optional_features: wgpu::Features::TIMESTAMP_QUERY, ..PageGpuOptions::default() };

        //Act
        set_page_gpu_options(options.clone());
        let current = PAGE_GPU_OPTIONS.with(|current| current.borrow().clone());

        //Assert
        assert_eq!(current, options);
    }
}
