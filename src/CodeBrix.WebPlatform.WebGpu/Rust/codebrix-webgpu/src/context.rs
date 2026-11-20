//! The WebGPU context - the `wgpu` instance, adapter, device and queue a page draws with, asked
//! for once and shared by every canvas of the page, with the device's ONE uncaptured-error
//! handler - and [`GpuUnavailable`], the reason a page cannot have one.

use std::fmt;
use std::rc::Rc;

use crate::errors::{ErrorHub, GpuErrorSubscription};

/// The `wgpu` objects a page draws with. Clone it to share it: the clones are the same context
/// (the same device), and two contexts compare equal when they are the same one.
///
/// Ask for it ONCE per page ([`GpuContext::request`]) and hand it to every canvas; or wrap a
/// device the application already has with [`GpuContext::from_parts`].
///
/// ERRORS: WebGPU has ONE uncaptured-error handler per device. The context installs it (and the
/// device-lost callback) the first time a context is made for a device - and never again for
/// that device, however many contexts wrap it - and every reader SUBSCRIBES instead
/// ([`GpuContext::subscribe_errors`]): every subscription sees every error (see
/// [`GpuErrorSubscription`]). Never call `on_uncaptured_error` or `set_device_lost_callback` on
/// the device yourself: that would take the errors away from every subscription.
#[derive(Clone)]
pub struct GpuContext {
    inner: Rc<Parts>,
}

struct Parts {
    instance: wgpu::Instance,
    adapter: wgpu::Adapter,
    device: wgpu::Device,
    queue: wgpu::Queue,
    errors: ErrorHub,
}

impl GpuContext {
    /// Asks the browser for a WebGPU adapter (the default one) and opens a device on it with the
    /// default limits and no optional features.
    ///
    /// WebGPU only: there is no WebGL fallback. The adapter is asked for exactly once - there is
    /// no separate "is WebGPU supported" probe first, which would ask a second time (and make the
    /// browser warn twice). A browser without `navigator.gpu`, with WebGPU switched off, or that
    /// hands out no adapter, gives [`GpuUnavailable::NoAdapter`].
    pub async fn request() -> Result<GpuContext, GpuUnavailable> {
        GpuContext::request_with(
            &wgpu::RequestAdapterOptions::default(),
            &wgpu::DeviceDescriptor { label: Some("codebrix-webgpu device"), ..wgpu::DeviceDescriptor::default() },
        )
        .await
    }

    /// Like [`GpuContext::request`], with the application's own adapter options (for example a
    /// power preference) and device descriptor (for example optional features or higher limits
    /// the application needs). A device the adapter cannot open with those features or limits
    /// gives [`GpuUnavailable::NoDevice`] with the browser's reason.
    pub async fn request_with(adapter_options: &wgpu::RequestAdapterOptions<'_, '_>, device_descriptor: &wgpu::DeviceDescriptor<'_>) -> Result<GpuContext, GpuUnavailable> {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let adapter = instance.request_adapter(adapter_options).await.map_err(|_| GpuUnavailable::NoAdapter)?;
        let (device, queue) = adapter.request_device(device_descriptor).await.map_err(|error| GpuUnavailable::NoDevice(error.to_string()))?;
        Ok(GpuContext::from_parts(instance, adapter, device, queue))
    }

    /// A context around a device the application already has, so that it can share that device
    /// with the canvas host and with other code built on this crate. The device's error handler is
    /// installed now if no context of this device installed it before (and shared if one did).
    pub fn from_parts(instance: wgpu::Instance, adapter: wgpu::Adapter, device: wgpu::Device, queue: wgpu::Queue) -> GpuContext {
        let errors = ErrorHub::for_device(&device);
        GpuContext { inner: Rc::new(Parts { instance, adapter, device, queue, errors }) }
    }

    /// A new reader of the device's uncaptured errors: it sees every error that arrives while it
    /// exists, independently of every other subscription (see [`GpuErrorSubscription`]). Each
    /// library that wraps the device (and the application, if it wants) keeps one and reads it
    /// once per frame.
    pub fn subscribe_errors(&self) -> GpuErrorSubscription {
        self.inner.errors.subscribe()
    }

    /// The browser's reason for losing the device, once it is lost (a lost device draws nothing
    /// more: request a new context and reload the canvases).
    pub fn lost_reason(&self) -> Option<String> {
        self.inner.errors.lost_reason()
    }

    /// True once the browser has reported the device lost.
    pub fn is_lost(&self) -> bool {
        self.lost_reason().is_some()
    }

    /// The `wgpu` instance (it makes the canvas surfaces).
    pub fn instance(&self) -> &wgpu::Instance {
        &self.inner.instance
    }

    /// The adapter the browser handed out.
    pub fn adapter(&self) -> &wgpu::Adapter {
        &self.inner.adapter
    }

    /// The device every renderer of the page creates its resources on.
    pub fn device(&self) -> &wgpu::Device {
        &self.inner.device
    }

    /// The device's queue.
    pub fn queue(&self) -> &wgpu::Queue {
        &self.inner.queue
    }

    /// What `wgpu` knows about the adapter. In a browser that is little: the browser's
    /// description of it (often empty, on purpose), whether it is a fallback (software) adapter,
    /// and the backend, which is always `BrowserWebGpu`.
    pub fn adapter_info(&self) -> wgpu::AdapterInfo {
        self.inner.adapter.get_info()
    }
}

impl PartialEq for GpuContext {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.inner, &other.inner)
    }
}

/// Why a page cannot draw with WebGPU: the error of [`GpuContext::request`]. Show it with
/// [`WebGpuUnavailable`](crate::WebGpuUnavailable), or with the application's own markup from
/// [`GpuUnavailable::TITLE`] and [`GpuUnavailable::advice`].
#[derive(Clone, Debug, PartialEq)]
pub enum GpuUnavailable {
    /// The browser has no WebGPU, has it switched off, or hands out no adapter.
    NoAdapter,
    /// The browser handed out an adapter but would not open a device on it (the browser's reason).
    NoDevice(String),
}

impl GpuUnavailable {
    /// The heading to show: the same for every reason.
    pub const TITLE: &'static str = "WebGPU is not available in this browser";

    /// One plain sentence saying what the visitor can do.
    pub fn advice(&self) -> String {
        match self {
            GpuUnavailable::NoAdapter => "Open this page in a browser with WebGPU switched on, such as a current Chrome or Edge, on a computer whose graphics the browser is allowed to use.".to_string(),
            GpuUnavailable::NoDevice(reason) => format!("The browser found a graphics adapter but would not open it ({reason}); reload the page, or try another browser."),
        }
    }
}

impl fmt::Display for GpuUnavailable {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}. {}", GpuUnavailable::TITLE, self.advice())
    }
}

impl std::error::Error for GpuUnavailable {}

#[cfg(test)]
mod tests {
    use super::GpuUnavailable;

    #[test]
    fn no_adapter_says_what_the_visitor_can_do() {
        let advice = GpuUnavailable::NoAdapter.advice();
        assert!(advice.starts_with("Open this page in a browser with WebGPU switched on"));
    }

    #[test]
    fn no_device_carries_the_browsers_reason() {
        let advice = GpuUnavailable::NoDevice("the limits are too high".to_string()).advice();
        assert!(advice.contains("(the limits are too high)"));
        assert!(advice.contains("reload the page"));
    }

    #[test]
    fn display_is_the_title_then_the_advice() {
        let text = GpuUnavailable::NoAdapter.to_string();
        assert_eq!(text, format!("{}. {}", GpuUnavailable::TITLE, GpuUnavailable::NoAdapter.advice()));
    }
}
