//! [`RenderTargetView`]: what a render pass draws into - a colour view, optionally a depth view,
//! their formats, the size, and (Not in C#) the sample count with the view a multisampled target
//! resolves into (C#: `RenderTargetView`); and [`TargetFormats`], the part of a target its
//! pipelines must agree with.

use std::fmt;

use crate::context::GraphicsException;
use crate::presentation::multisampling::check_sample_count;
use crate::resources::GpuTexture;

/// What a pipeline must agree with to draw into a target (Not in C#): the colour format, the
/// depth format and the sample count - WebGPU's "attachment state" for the one colour target the
/// library's passes have. Every renderer of the library keys its pipelines by it, so a renderer
/// used on a multisampled target picks (or makes) the multisampled pipeline by itself.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct TargetFormats {
    /// The colour format.
    pub color: wgpu::TextureFormat,
    /// The depth format, None for a target without depth.
    pub depth: Option<wgpu::TextureFormat>,
    /// The samples per pixel: 1, or 4 for a multisampled target.
    pub sample_count: u32,
}

impl TargetFormats {
    /// The formats from their parts.
    pub fn new(color: wgpu::TextureFormat, depth: Option<wgpu::TextureFormat>, sample_count: u32) -> TargetFormats {
        TargetFormats { color, depth, sample_count }
    }

    /// True for more than one sample per pixel.
    pub fn is_multisampled(&self) -> bool {
        self.sample_count > 1
    }
}

impl fmt::Display for TargetFormats {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self.color)?;
        if let Some(depth) = self.depth {
            write!(f, " + {depth:?}")?;
        }
        write!(f, ", {} sample{}", self.sample_count, if self.sample_count == 1 { "" } else { "s" })
    }
}

/// What a render pass draws into (C#: `RenderTargetView`): a colour view and optionally a depth
/// view, with their formats and the size in pixels. The views are `wgpu` handles (cheap clones).
///
/// MULTISAMPLED TARGETS (Not in C#): a target with a sample count of 4 has a multisampled colour
/// view (and a multisampled depth view, if it has depth) and an ordinary single-sampled RESOLVE
/// view; every render pass begun on it ends by resolving the colour into the resolve view. A
/// [`crate::HeadlessRenderTarget`] made with `sample_count: 4` and a frame begun with
/// [`crate::GraphicsContext::begin_canvas_frame_on`] on a 4-sample [`crate::CanvasFrameTarget`]
/// hand out such views; [`RenderTargetView::with_resolve`] makes one from views of the
/// application's own.
#[derive(Clone, Debug)]
pub struct RenderTargetView {
    color_view: wgpu::TextureView,
    color_format: wgpu::TextureFormat,
    width: u32,
    height: u32,
    depth: Option<(wgpu::TextureView, wgpu::TextureFormat)>,
    sample_count: u32,
    resolve_view: Option<wgpu::TextureView>,
}

impl RenderTargetView {
    /// A single-sampled target from its parts (C#: the constructor; `depth` is the depth view and
    /// its format).
    pub fn new(color_view: wgpu::TextureView, color_format: wgpu::TextureFormat, width: u32, height: u32, depth: Option<(wgpu::TextureView, wgpu::TextureFormat)>) -> RenderTargetView {
        RenderTargetView { color_view, color_format, width, height, depth, sample_count: 1, resolve_view: None }
    }

    /// The target of a canvas frame: its view in the canvas's draw format, at its size (C#:
    /// `RenderTargetView.From(IWebGpuTextureTarget)`). Always single-sampled; a multisampled
    /// canvas frame is [`crate::GraphicsContext::begin_canvas_frame_on`]'s.
    pub fn from_canvas(frame: &codebrix_webgpu::CanvasFrame<'_>) -> RenderTargetView {
        RenderTargetView::new(frame.view.clone(), frame.target.format, frame.info.width, frame.info.height, None)
    }

    /// The same target drawn with `sample_count` samples per pixel (Not in C#): its colour view
    /// (and depth view, if any) must be views of textures made with that sample count, and every
    /// pass begun on it resolves the colour into `resolve_view` (a single-sampled view of the same
    /// size and format). Err for any count but 4: WebGPU's counts are 1 and 4, and a 1-sample
    /// target has nothing to resolve.
    pub fn with_resolve(&self, sample_count: u32, resolve_view: wgpu::TextureView) -> Result<RenderTargetView, GraphicsException> {
        check_sample_count(sample_count, "a render target")?;
        if sample_count == 1 {
            return Err(GraphicsException::new("A 1-sample target has nothing to resolve; with_resolve takes the multisampled count (4). Use the target as it is for 1 sample."));
        }
        Ok(RenderTargetView { sample_count, resolve_view: Some(resolve_view), ..self.clone() })
    }

    /// The colour view (C#: `ColorView`): the MULTISAMPLED view for a multisampled target.
    pub fn color_view(&self) -> &wgpu::TextureView {
        &self.color_view
    }

    /// The colour format (C#: `ColorFormat`).
    pub fn color_format(&self) -> wgpu::TextureFormat {
        self.color_format
    }

    /// The width in pixels (C#: `Width`).
    pub fn width(&self) -> u32 {
        self.width
    }

    /// The height in pixels (C#: `Height`).
    pub fn height(&self) -> u32 {
        self.height
    }

    /// The depth view (C#: `DepthView`).
    pub fn depth_view(&self) -> Option<&wgpu::TextureView> {
        self.depth.as_ref().map(|(view, _)| view)
    }

    /// The depth format (C#: `DepthFormat`; None for C#'s Undefined).
    pub fn depth_format(&self) -> Option<wgpu::TextureFormat> {
        self.depth.as_ref().map(|(_, format)| *format)
    }

    /// True when the target has a depth view (C#: `HasDepth`).
    pub fn has_depth(&self) -> bool {
        self.depth.is_some()
    }

    /// The samples per pixel: 1, or 4 for a multisampled target (C#: `SampleCount`, which C#
    /// carries on the pipeline only).
    pub fn sample_count(&self) -> u32 {
        self.sample_count
    }

    /// True for a multisampled target (Not in C#).
    pub fn is_multisampled(&self) -> bool {
        self.sample_count > 1
    }

    /// The single-sampled view every pass on a multisampled target resolves into; None for a
    /// 1-sample target (Not in C#).
    pub fn resolve_view(&self) -> Option<&wgpu::TextureView> {
        self.resolve_view.as_ref()
    }

    /// What a pipeline must agree with to draw into this target: colour format, depth format and
    /// sample count (Not in C#). The library's renderers key their pipelines by it.
    pub fn formats(&self) -> TargetFormats {
        TargetFormats { color: self.color_format, depth: self.depth_format(), sample_count: self.sample_count }
    }

    /// Width over height, 1 for a zero height (C#: `AspectRatio`).
    pub fn aspect_ratio(&self) -> f32 {
        if self.height == 0 {
            1.0
        } else {
            self.width as f32 / self.height as f32
        }
    }

    /// The same target with a depth texture (C#: `WithDepth`). On a multisampled target the depth
    /// texture must have the target's sample count.
    pub fn with_depth(&self, depth: &GpuTexture) -> RenderTargetView {
        RenderTargetView { depth: Some((depth.view().clone(), depth.format())), ..self.clone() }
    }

    /// The same target without its depth view - colour, sample count and resolve view kept (Not in
    /// C#; what [`crate::ToneMapper::apply`] draws into).
    pub fn without_depth(&self) -> RenderTargetView {
        RenderTargetView { depth: None, ..self.clone() }
    }
}

impl fmt::Display for RenderTargetView {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}x{} {:?}", self.width, self.height, self.color_format)?;
        if let Some(format) = self.depth_format() {
            write!(f, " + {format:?}")?;
        }
        if self.sample_count > 1 {
            write!(f, " x{} (resolved)", self.sample_count)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::TargetFormats;

    #[test]
    fn target_formats_differ_by_sample_count_and_say_so() {
        let one = TargetFormats::new(wgpu::TextureFormat::Bgra8UnormSrgb, Some(wgpu::TextureFormat::Depth32Float), 1);
        let four = TargetFormats { sample_count: 4, ..one };
        assert_ne!(one, four);
        assert!(!one.is_multisampled());
        assert!(four.is_multisampled());
        assert_eq!(one.to_string(), "Bgra8UnormSrgb + Depth32Float, 1 sample");
        assert_eq!(four.to_string(), "Bgra8UnormSrgb + Depth32Float, 4 samples");
    }
}
