//! [`GraphicsContext`]: the device the library draws with, owned or shared, the queue of
//! WebGPU's uncaptured errors, and the async validation scope (C#: `GraphicsContext`).

use std::cell::{Cell, RefCell};
use std::fmt;
use std::rc::Rc;

use codebrix_webgpu::{GpuContext, GpuErrorSubscription, GpuUnavailable};

use crate::context::{GraphicsContextOptions, GraphicsError, GraphicsException};
use crate::images::MipmapGenerator;
use crate::lighting::EnvironmentCache;

/// The device the library draws with, its queue, and what the library keeps per device (C#:
/// `GraphicsContext`). Clone it to share it: the clones are the same context.
///
/// A context OWNS its device when it made it ([`GraphicsContext::create`]) and SHARES it when it
/// wraps one ([`GraphicsContext::wrap_gpu_context`], [`GraphicsContext::wrap`]) - the usual case
/// in a page, where the canvas host's [`GpuContext`] holds the device and every layer of the
/// canvas draws with it. Low-level `wgpu` always stays available: [`GraphicsContext::device`],
/// [`GraphicsContext::queue`] and the `handle()` of every resource are the `wgpu` objects.
///
/// ERRORS (the one deliberate difference from C#): a browser cannot wait for WebGPU's answer, so
/// a factory hands back its object at once, and a validation error WebGPU finds later is queued:
/// [`GraphicsContext::take_errors`] (C# `TakeErrors()`). Code that wants C#'s "throw with
/// WebGPU's message" awaits a `_checked` variant or wraps its calls in
/// [`GraphicsContext::validate`].
#[derive(Clone)]
pub struct GraphicsContext {
    inner: Rc<Inner>,
}

struct Inner {
    gpu: GpuContext,
    wrapped: bool,
    errors: GpuErrorSubscription,
    next_frame_id: Cell<u64>,
    white: RefCell<Option<(wgpu::Texture, wgpu::TextureView)>>,
    mipmaps: RefCell<MipmapGenerator>,
    environments: RefCell<EnvironmentCache>,
}

impl GraphicsContext {
    /// Asks the browser for an adapter and opens a device on it (C#: `GraphicsContext.Create` /
    /// `TryCreate`; async here, because the browser answers with a promise). The error is the
    /// "WebGPU is not available" result of `codebrix-webgpu`, which its `WebGpuUnavailable`
    /// component shows.
    ///
    /// A page that draws through the canvas host asks for the host's [`GpuContext`] instead and
    /// wraps it ([`GraphicsContext::wrap_gpu_context`]), so that everything shares one device.
    pub async fn create(options: GraphicsContextOptions) -> Result<GraphicsContext, GpuUnavailable> {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let adapter_options = wgpu::RequestAdapterOptions { power_preference: options.power_preference, force_fallback_adapter: options.force_fallback_adapter, ..wgpu::RequestAdapterOptions::default() };
        let adapter = instance.request_adapter(&adapter_options).await.map_err(|_| GpuUnavailable::NoAdapter)?;
        let required_limits = if options.request_adapter_limits { adapter.limits() } else { options.required_limits.clone().unwrap_or_default() };
        let descriptor = wgpu::DeviceDescriptor {
            label: Some(options.label.as_deref().unwrap_or("codebrix-webgpu-graphics device")),
            required_features: options.features_for(adapter.features()),
            required_limits,
            ..wgpu::DeviceDescriptor::default()
        };
        let (device, queue) = adapter.request_device(&descriptor).await.map_err(|error| GpuUnavailable::NoDevice(error.to_string()))?;
        Ok(GraphicsContext::new(GpuContext::from_parts(instance, adapter, device, queue), false))
    }

    /// A context that SHARES the device of a `codebrix-webgpu` [`GpuContext`] - the canvas host's
    /// (C#: `Wrap(IWebGpuDeviceSource)`). The clones of `gpu` and this context use one device.
    ///
    /// The context SUBSCRIBES to the device's uncaptured errors, which `codebrix-webgpu` collects
    /// through the device's one handler (installed once per device), so that
    /// [`GraphicsContext::take_errors`] sees every error - and any other library wrapping the same
    /// device sees every error too: nobody takes errors away from anybody else.
    pub fn wrap_gpu_context(gpu: &GpuContext) -> GraphicsContext {
        GraphicsContext::new(gpu.clone(), true)
    }

    /// A context around `wgpu` objects the application already has (C#: `Wrap(Device, Queue,
    /// Instance, Adapter)`; a browser always has all four).
    pub fn wrap(device: wgpu::Device, queue: wgpu::Queue, instance: wgpu::Instance, adapter: wgpu::Adapter) -> GraphicsContext {
        GraphicsContext::new(GpuContext::from_parts(instance, adapter, device, queue), true)
    }

    fn new(gpu: GpuContext, wrapped: bool) -> GraphicsContext {
        let errors = gpu.subscribe_errors();
        GraphicsContext { inner: Rc::new(Inner { gpu, wrapped, errors, next_frame_id: Cell::new(0), white: RefCell::new(None), mipmaps: RefCell::new(MipmapGenerator::new()), environments: RefCell::new(EnvironmentCache::new()) }) }
    }

    /// The `codebrix-webgpu` context this one draws with (hand it to the canvas host).
    pub fn gpu_context(&self) -> &GpuContext {
        &self.inner.gpu
    }

    /// The `wgpu` instance (C#: `Instance`).
    pub fn instance(&self) -> &wgpu::Instance {
        self.inner.gpu.instance()
    }

    /// The adapter (C#: `Adapter`).
    pub fn adapter(&self) -> &wgpu::Adapter {
        self.inner.gpu.adapter()
    }

    /// The device (C#: `Device`).
    pub fn device(&self) -> &wgpu::Device {
        self.inner.gpu.device()
    }

    /// The queue (C#: `Queue`).
    pub fn queue(&self) -> &wgpu::Queue {
        self.inner.gpu.queue()
    }

    /// True when the context shares a device it did not make (C#: `IsWrapped`).
    pub fn is_wrapped(&self) -> bool {
        self.inner.wrapped
    }

    /// What the browser says about the adapter (C#: `AdapterName` and `GraphicsAdapterInfo`; a
    /// browser often leaves the description empty on purpose).
    pub fn adapter_info(&self) -> wgpu::AdapterInfo {
        self.inner.gpu.adapter_info()
    }

    /// The device's limits (C#: `Limits`).
    pub fn limits(&self) -> wgpu::Limits {
        self.device().limits()
    }

    /// The device's optional features.
    pub fn features(&self) -> wgpu::Features {
        self.device().features()
    }

    /// True when the device has the feature (C#: `HasFeature`).
    pub fn has_feature(&self, feature: wgpu::Features) -> bool {
        self.features().contains(feature)
    }

    /// True once the browser has reported the device lost (C#: `IsLost`).
    pub fn is_lost(&self) -> bool {
        self.lost_reason().is_some()
    }

    /// The browser's reason for losing the device, once it is lost (C#: `LostReason`). A lost
    /// device draws nothing more: make a new context (and reload the canvases).
    pub fn lost_reason(&self) -> Option<String> {
        self.inner.gpu.lost_reason()
    }

    /// Hands out, oldest first, the errors WebGPU reported for work outside an error scope since
    /// the last call, and forgets them (C#: `TakeErrors()`). A page calls it once per frame (or
    /// after loading) and shows or logs what it gets: an empty list means no error arrived.
    ///
    /// The errors are those of the whole DEVICE (WebGPU does not say which library caused one),
    /// from the moment this context was made; the clones of this context share one queue. Another
    /// context or library on the same device has its own queue and sees the same errors.
    pub fn take_errors(&self) -> Vec<GraphicsError> {
        self.inner.errors.take().iter().map(GraphicsError::from_gpu_error).collect()
    }

    /// True when the ADAPTER offers `feature` - whether or not the device was given it (C#:
    /// `AdapterHasFeature`).
    pub fn adapter_has_feature(&self, feature: wgpu::Features) -> bool {
        self.adapter().features().contains(feature)
    }

    /// The features the adapter offers (C#: `AdapterCapabilities.Features`); the device's are
    /// [`GraphicsContext::features`] (C#: `Capabilities.Features`).
    pub fn adapter_features(&self) -> wgpu::Features {
        self.adapter().features()
    }

    /// The limits the adapter offers (C#: `AdapterCapabilities.Limits`); the device's are
    /// [`GraphicsContext::limits`].
    pub fn adapter_limits(&self) -> wgpu::Limits {
        self.adapter().limits()
    }

    /// The nanoseconds of one timestamp tick (C#: `TimestampPeriod`; 1 in a browser, whose
    /// timestamps are nanoseconds).
    pub fn timestamp_period(&self) -> f32 {
        self.queue().get_timestamp_period()
    }

    /// Submits finished command buffers in order (C#: `Submit(commandBuffers)`; the buffers come
    /// from `GraphicsFrame::finish`). Returns the queue's submission index.
    pub fn submit(&self, command_buffers: impl IntoIterator<Item = wgpu::CommandBuffer>) -> wgpu::SubmissionIndex {
        self.queue().submit(command_buffers)
    }

    /// Completes when the GPU has finished the work submitted so far (C#:
    /// `WhenSubmittedWorkDoneAsync`; C#'s synchronous `WaitIdle` and `WaitForSubmission` have no
    /// counterpart - a browser cannot wait).
    pub async fn when_submitted_work_done(&self) {
        let (sender, receiver) = crate::internal::oneshot::channel();
        self.queue().on_submitted_work_done(move || sender.send(()));
        receiver.await;
    }

    /// Runs `work` inside WebGPU error scopes (validation and out-of-memory) and waits for WebGPU's
    /// verdict: `work`'s result, or a [`GraphicsException`] with WebGPU's message (C#: what every
    /// C# factory does before it returns or throws). Use it at load time and in tests; the render
    /// loop stays synchronous and reads [`GraphicsContext::take_errors`].
    pub async fn validate<T>(&self, work: impl FnOnce(&GraphicsContext) -> T) -> Result<T, GraphicsException> {
        self.validate_named("The work", work).await
    }

    /// As [`GraphicsContext::validate`], with `what` ("The buffer 'x'") starting the message.
    pub(crate) async fn validate_named<T>(&self, what: &str, work: impl FnOnce(&GraphicsContext) -> T) -> Result<T, GraphicsException> {
        let out_of_memory = self.device().push_error_scope(wgpu::ErrorFilter::OutOfMemory);
        let validation = self.device().push_error_scope(wgpu::ErrorFilter::Validation);
        let value = work(self);
        let validation = validation.pop();
        let out_of_memory = out_of_memory.pop();
        let first = validation.await;
        let second = out_of_memory.await;
        match first.or(second) {
            None => Ok(value),
            Some(error) => Err(GraphicsException::new(format!("{what} was refused by WebGPU: {error}. Correct the call (the message names what WebGPU did not accept) and try again."))),
        }
    }

    /// The context's 1 x 1 opaque white texture and its view (C#: `WhiteTexture`, internal there
    /// too): what an untextured sprite or material samples. Made on first use.
    pub(crate) fn white_texture(&self) -> (wgpu::Texture, wgpu::TextureView) {
        if let Some(white) = self.inner.white.borrow().as_ref() {
            return white.clone();
        }
        let texture = self.device().create_texture(&wgpu::TextureDescriptor {
            label: Some("white"),
            size: wgpu::Extent3d { width: 1, height: 1, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        self.queue().write_texture(
            wgpu::TexelCopyTextureInfo { texture: &texture, mip_level: 0, origin: wgpu::Origin3d::ZERO, aspect: wgpu::TextureAspect::All },
            &[255, 255, 255, 255],
            wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(4), rows_per_image: Some(1) },
            wgpu::Extent3d { width: 1, height: 1, depth_or_array_layers: 1 },
        );
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        *self.inner.white.borrow_mut() = Some((texture.clone(), view.clone()));
        (texture, view)
    }

    /// The context's GPU mipmap generator (C#: its `MipmapGenerator`).
    pub(crate) fn mipmap_generator(&self) -> &RefCell<MipmapGenerator> {
        &self.inner.mipmaps
    }

    /// What the context keeps for environments: the baker's pipelines, the BRDF LUT and the
    /// default environment (C#: its `EnvironmentBaker` and `DefaultEnvironment`). They hold `wgpu`
    /// objects only, never the context, so nothing keeps the context alive by itself.
    pub(crate) fn environment_cache(&self) -> &RefCell<EnvironmentCache> {
        &self.inner.environments
    }

    /// The next frame number of this context.
    pub(crate) fn next_frame_id(&self) -> u64 {
        let id = self.inner.next_frame_id.get() + 1;
        self.inner.next_frame_id.set(id);
        id
    }
}

impl PartialEq for GraphicsContext {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.inner, &other.inner)
    }
}

impl fmt::Debug for GraphicsContext {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "GraphicsContext {}", if self.inner.wrapped { "(wrapped)" } else { "(owned)" })
    }
}

/// " 'label'" for messages, or nothing.
pub(crate) fn label_text(label: Option<&str>) -> String {
    match label {
        Some(label) => format!(" '{label}'"),
        None => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::label_text;

    #[test]
    fn label_text_quotes_a_label() {
        assert_eq!(label_text(Some("quad")), " 'quad'");
        assert_eq!(label_text(None), "");
    }
}
