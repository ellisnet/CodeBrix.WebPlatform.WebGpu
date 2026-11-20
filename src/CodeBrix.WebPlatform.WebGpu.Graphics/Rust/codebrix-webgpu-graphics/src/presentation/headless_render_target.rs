//! [`HeadlessRenderTarget`]: an offscreen colour texture and optional depth texture to render into
//! - the HDR intermediate of the viewer's rendering path (C#: `HeadlessRenderTarget`).

use crate::context::{DepthTextureOptions, GraphicsContext, GraphicsException, RenderTextureOptions};
use crate::presentation::RenderTargetView;
use crate::resources::GpuTexture;

/// The defaulted arguments of [`GraphicsContext::create_headless_target`] (C#: `format =
/// RGBA8Unorm`, `withDepth = true`, `depthFormat = Depth32Float`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HeadlessTargetOptions {
    /// The colour format (`Rgba16Float` for an HDR scene that a [`crate::ToneMapper`] maps).
    pub format: wgpu::TextureFormat,
    /// True for a depth texture beside the colour.
    pub with_depth: bool,
    /// The depth format.
    pub depth_format: wgpu::TextureFormat,
}

impl Default for HeadlessTargetOptions {
    fn default() -> HeadlessTargetOptions {
        HeadlessTargetOptions { format: wgpu::TextureFormat::Rgba8Unorm, with_depth: true, depth_format: wgpu::TextureFormat::Depth32Float }
    }
}

/// An offscreen colour texture (and optional depth texture) to render into (C#:
/// `HeadlessRenderTarget`; the name is kept - "headless" means "not the canvas"). In the browser
/// its use is as an intermediate: the scene is drawn into an RGBA16Float target, then a
/// [`crate::ToneMapper`] maps [`HeadlessRenderTarget::color`] into the canvas frame. Make one
/// per canvas size (again in the layer's `resize`).
///
/// Its reads are ASYNC here (C#'s `ReadPixels` / `ReadRgba` / `ReadDepth` wait for the GPU; a
/// browser cannot). Not here: `SaveAs`, `SaveAsPng` and `ToImage` - a browser has no file system
/// and no image encoder; an application shows its frames, or hands the read pixels on.
pub struct HeadlessRenderTarget {
    color: GpuTexture,
    depth: Option<GpuTexture>,
}

impl HeadlessRenderTarget {
    /// The colour texture (render attachment, sampled, copyable) (C#: `Color`).
    pub fn color(&self) -> &GpuTexture {
        &self.color
    }

    /// The depth texture, if the target has one (C#: `Depth`).
    pub fn depth(&self) -> Option<&GpuTexture> {
        self.depth.as_ref()
    }

    /// The width in pixels (C#: `Width`).
    pub fn width(&self) -> u32 {
        self.color.width()
    }

    /// The height in pixels (C#: `Height`).
    pub fn height(&self) -> u32 {
        self.color.height()
    }

    /// The colour format (C#: `Format`).
    pub fn format(&self) -> wgpu::TextureFormat {
        self.color.format()
    }

    /// The target to pass to [`crate::GraphicsFrame::begin_render_pass_to`]: the colour and, if
    /// there is one, the depth (C#: `View`).
    pub fn view(&self) -> RenderTargetView {
        let view = self.color.as_render_target();
        match &self.depth {
            Some(depth) => view.with_depth(depth),
            None => view,
        }
    }

    /// Reads the colour texture as tight rows in its own byte order, after the work submitted so
    /// far (C#: `ReadPixels`; async here).
    pub async fn read_pixels(&self) -> Result<Vec<u8>, GraphicsException> {
        self.color.read_pixels(0).await
    }

    /// Reads the colour texture (RGBA8 or BGRA8) as RGBA pixels, red first (C#: `ReadRgba`; async here).
    pub async fn read_rgba(&self) -> Result<Vec<u8>, GraphicsException> {
        self.color.read_rgba().await
    }

    /// Reads the depth texture (Depth32Float only) as one f32 per pixel, row by row (C#:
    /// `ReadDepth`; async here).
    pub async fn read_depth(&self) -> Result<Vec<f32>, GraphicsException> {
        match &self.depth {
            Some(depth) if depth.format() == wgpu::TextureFormat::Depth32Float => {
                let bytes = depth.read_pixels(0).await?;
                Ok(bytemuck::pod_collect_to_vec(&bytes))
            }
            _ => Err(GraphicsException::new("The target has no Depth32Float depth texture; create it with HeadlessTargetOptions { with_depth: true, depth_format: Depth32Float, .. } to read depth.")),
        }
    }
}

impl GraphicsContext {
    /// An offscreen target of `width` x `height` pixels (C#: `CreateHeadlessTarget(width, height,
    /// format, withDepth, depthFormat)`).
    pub fn create_headless_target(&self, width: u32, height: u32, options: HeadlessTargetOptions) -> Result<HeadlessRenderTarget, GraphicsException> {
        let color = self.create_render_texture(width, height, RenderTextureOptions { format: options.format, label: Some("headless colour".to_string()) })?;
        let depth = if options.with_depth { Some(self.create_depth_texture(width, height, DepthTextureOptions { format: options.depth_format, label: Some("headless depth".to_string()) })?) } else { None };
        Ok(HeadlessRenderTarget { color, depth })
    }
}

#[cfg(test)]
mod tests {
    use super::HeadlessTargetOptions;

    #[test]
    fn the_options_carry_the_csharp_defaults() {
        let options = HeadlessTargetOptions::default();
        assert_eq!(options.format, wgpu::TextureFormat::Rgba8Unorm);
        assert!(options.with_depth);
        assert_eq!(options.depth_format, wgpu::TextureFormat::Depth32Float);
    }
}
