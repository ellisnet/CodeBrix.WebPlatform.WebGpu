//! [`RenderTargetView`]: what a render pass draws into - a colour view, optionally a depth view,
//! their formats and the size (C#: `RenderTargetView`).

use std::fmt;

use crate::resources::GpuTexture;

/// What a render pass draws into (C#: `RenderTargetView`): a colour view and optionally a depth
/// view, with their formats and the size in pixels. The views are `wgpu` handles (cheap clones).
#[derive(Clone, Debug)]
pub struct RenderTargetView {
    color_view: wgpu::TextureView,
    color_format: wgpu::TextureFormat,
    width: u32,
    height: u32,
    depth: Option<(wgpu::TextureView, wgpu::TextureFormat)>,
}

impl RenderTargetView {
    /// A target from its parts (C#: the constructor; `depth` is the depth view and its format).
    pub fn new(color_view: wgpu::TextureView, color_format: wgpu::TextureFormat, width: u32, height: u32, depth: Option<(wgpu::TextureView, wgpu::TextureFormat)>) -> RenderTargetView {
        RenderTargetView { color_view, color_format, width, height, depth }
    }

    /// The target of a canvas frame: its view in the canvas's draw format, at its size (C#:
    /// `RenderTargetView.From(IWebGpuTextureTarget)`).
    pub fn from_canvas(frame: &codebrix_webgpu::CanvasFrame<'_>) -> RenderTargetView {
        RenderTargetView::new(frame.view.clone(), frame.target.format, frame.info.width, frame.info.height, None)
    }

    /// The colour view (C#: `ColorView`).
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

    /// Width over height, 1 for a zero height (C#: `AspectRatio`).
    pub fn aspect_ratio(&self) -> f32 {
        if self.height == 0 {
            1.0
        } else {
            self.width as f32 / self.height as f32
        }
    }

    /// The same target with a depth texture (C#: `WithDepth`).
    pub fn with_depth(&self, depth: &GpuTexture) -> RenderTargetView {
        RenderTargetView { depth: Some((depth.view().clone(), depth.format())), ..self.clone() }
    }
}

impl fmt::Display for RenderTargetView {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}x{} {:?}", self.width, self.height, self.color_format)?;
        if let Some(format) = self.depth_format() {
            write!(f, " + {format:?}")?;
        }
        Ok(())
    }
}
