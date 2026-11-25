//! `PictureTarget`: a picture of a fixed pixel size that the crate DRAWS each frame (another
//! session, drawn by its own [`AnimationLayer::to_picture`](crate::rendering::AnimationLayer::to_picture)
//! layer) and other drawing READS in the same frame - strokes that pick their colours from it
//! ([`StrokeColorSource::Target`](crate::strokes::StrokeColorSource::Target)) or a picture element
//! that shows it ([`ImageShape::from_picture_target`](crate::shapes::ImageShape::from_picture_target)).
//! No C# equivalent.
//!
//! THE PICTURE is a GPU texture made at its first use: `Rgba8Unorm` holding PREMULTIPLIED
//! sRGB-ENCODED colour (the crate's convention for a premultiplied picture), cleared to
//! transparent and drawn again every frame by its layer. All canvas layers share ONE command
//! encoder, so a picture layer listed BELOW its readers in `CanvasLayers` gives them this frame's
//! picture; listed above them it gives the previous frame's (and a transparent one on the first
//! frame) - a stroke pickup that reads a picture before it was drawn in the frame is reported once
//! through `DrawingSession::take_reports`.
//!
//! Crate-internal: the registry that finds a target by its stand-in picture's id (the renderer
//! draws an `ImageShape` of a target through the ordinary picture path, keyed by that id) and the
//! canvas frame number every `AnimationLayer` publishes when it starts a frame (one thread).

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::fmt;
use std::rc::{Rc, Weak};
use std::sync::Arc;

use codebrix_webgpu::wgpu;
use peniko::{ImageAlphaType, ImageData, ImageFormat};

/// The format of a picture target's texture: premultiplied sRGB-encoded bytes.
pub(crate) const PICTURE_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;

/// A picture the crate draws each frame and other drawing reads (no C# equivalent). Clones are
/// the SAME picture (equality is identity). Its GPU texture is made at its first use on the
/// canvas host's device.
#[derive(Clone)]
pub struct PictureTarget {
    inner: Rc<Inner>,
}

struct Inner {
    size: (u32, u32),
    /// The picture's stand-in for the picture path: no pixels (4 bytes), the target's size; its
    /// blob id is the target's key in the registry.
    stand_in: ImageData,
    gpu: RefCell<Option<PictureTexture>>,
    /// The canvas frame the picture was last drawn in (0: never).
    drawn_frame: Cell<u64>,
}

/// A picture target's texture on one device.
#[derive(Clone)]
pub(crate) struct PictureTexture {
    device: wgpu::Device,
    /// Kept with its view (the view keeps the texture alive; this keeps it visible to the code).
    #[allow(dead_code)]
    texture: wgpu::Texture,
    pub(crate) view: wgpu::TextureView,
}

thread_local! {
    static TARGETS: RefCell<HashMap<u64, Weak<Inner>>> = RefCell::new(HashMap::new());
    static CANVAS_FRAME: Cell<u64> = const { Cell::new(0) };
}

impl Drop for Inner {
    fn drop(&mut self) {
        let id = self.stand_in.data.id();
        let _ = TARGETS.try_with(|t| t.borrow_mut().remove(&id));
    }
}

impl PictureTarget {
    /// The largest side a picture target may have (WebGPU's guaranteed texture size).
    pub const MAX_SIZE: u32 = 8192;

    /// A picture of `width` x `height` pixels. The error says what is wrong with the size.
    /// Recommended: half the canvas or less for a colour source (pickup averages anyway); the
    /// picture costs its layer's multisampled targets at its size plus the picture itself.
    pub fn new(width: u32, height: u32) -> Result<PictureTarget, String> {
        if width < 1 || height < 1 || width > Self::MAX_SIZE || height > Self::MAX_SIZE {
            return Err(format!("A picture target needs a width and a height of 1 to {} pixels (it was {width} x {height}).", Self::MAX_SIZE));
        }
        let stand_in = ImageData { data: peniko::Blob::new(Arc::new(vec![0u8; 4])), format: ImageFormat::Rgba8, alpha_type: ImageAlphaType::AlphaPremultiplied, width, height };
        let id = stand_in.data.id();
        let inner = Rc::new(Inner { size: (width, height), stand_in, gpu: RefCell::new(None), drawn_frame: Cell::new(0) });
        TARGETS.with(|t| t.borrow_mut().insert(id, Rc::downgrade(&inner)));
        Ok(PictureTarget { inner })
    }

    /// The size in pixels.
    pub fn size(&self) -> (u32, u32) {
        self.inner.size
    }

    /// The target's identity (the same for clones).
    pub(crate) fn id(&self) -> u64 {
        self.inner.stand_in.data.id()
    }

    /// The stand-in the picture path draws a target with (its id finds the target again).
    pub(crate) fn stand_in(&self) -> &ImageData {
        &self.inner.stand_in
    }

    /// The target whose stand-in has this blob id.
    pub(crate) fn find(blob_id: u64) -> Option<PictureTarget> {
        TARGETS.with(|t| t.borrow().get(&blob_id).and_then(Weak::upgrade)).map(|inner| PictureTarget { inner })
    }

    /// The picture's texture on this device, made (transparent) at its first use or when the
    /// device changed.
    pub(crate) fn texture(&self, device: &wgpu::Device) -> PictureTexture {
        let mut gpu = self.inner.gpu.borrow_mut();
        if let Some(t) = gpu.as_ref() {
            if &t.device == device {
                return t.clone();
            }
        }
        let (width, height) = self.inner.size;
        let texture = device.create_texture(&wgpu::TextureDescriptor { label: Some("animation picture target"), size: wgpu::Extent3d { width, height, depth_or_array_layers: 1 }, mip_level_count: 1, sample_count: 1, dimension: wgpu::TextureDimension::D2, format: PICTURE_FORMAT, usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING, view_formats: &[] });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let made = PictureTexture { device: device.clone(), texture, view };
        *gpu = Some(made.clone());
        made
    }

    /// Records that the picture is drawn in this canvas frame.
    pub(crate) fn mark_drawn(&self, frame: u64) {
        self.inner.drawn_frame.set(frame);
    }

    /// The canvas frame the picture was last drawn in (0: never).
    pub(crate) fn drawn_frame(&self) -> u64 {
        self.inner.drawn_frame.get()
    }
}

impl PartialEq for PictureTarget {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.inner, &other.inner)
    }
}

impl fmt::Debug for PictureTarget {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "PictureTarget({} x {}, id {})", self.inner.size.0, self.inner.size.1, self.id())
    }
}

/// Publishes the canvas frame being drawn (every `AnimationLayer` at the start of its frame).
pub(crate) fn set_canvas_frame(frame: u64) {
    CANVAS_FRAME.with(|c| c.set(frame));
}

/// The canvas frame being drawn (0 outside a canvas: a session rendered by itself).
pub(crate) fn canvas_frame() -> u64 {
    CANVAS_FRAME.with(|c| c.get())
}

/// True when a reader in canvas frame `current` sees a picture drawn in frame `drawn` - that is,
/// NOT this frame's picture (its layer is listed above the reader, or not on the canvas). False
/// outside a canvas (`current` 0).
pub(crate) fn read_too_early(drawn: u64, current: u64) -> bool {
    current > 0 && drawn != current
}

/// The report for a picture read before it was drawn in the frame.
pub(crate) fn early_read_report(target: &PictureTarget) -> String {
    let (w, h) = target.size();
    format!("A stroke set picks its colours from a {w} x {h} picture target that was not drawn yet in this frame: put the picture's AnimationLayer::to_picture layer BELOW the layers that read it in CanvasLayers (until then they see the previous frame's picture, and a transparent one on the first frame).")
}
