//! [`HeadlessRenderTarget`]: an offscreen colour texture and optional depth texture to render
//! into, the HDR intermediate of the viewer's rendering path (C#: `HeadlessRenderTarget`); Not in
//! C#: a sample-count setting (multisampling, MSAA) that may change while the page runs.

use crate::context::{DepthTextureOptions, GraphicsContext, GraphicsException, RenderTextureOptions};
use crate::presentation::multisampling::{check_sample_count, create_multisampled_texture, plan_buffers, BufferSets};
use crate::presentation::RenderTargetView;
use crate::resources::GpuTexture;

/// The defaulted arguments of [`GraphicsContext::create_headless_target`] (C#: `format =
/// RGBA8Unorm`, `withDepth = true`, `depthFormat = Depth32Float`). The sample count is not here:
/// it is [`GraphicsContext::create_headless_target_with_samples`]'s argument and
/// [`HeadlessRenderTarget::set_sample_count`]'s, and 1 otherwise.
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

/// The multisampled buffers of a 4-sample target: colour and, if the target has depth, depth.
struct MultisampledBuffers {
    color: GpuTexture,
    depth: Option<GpuTexture>,
}

/// An offscreen colour texture (and optional depth texture) to render into (C#:
/// `HeadlessRenderTarget`; the name is kept - "headless" means "not the canvas"). In the browser
/// its use is as an intermediate: the scene is drawn into an RGBA16Float target, then a
/// [`crate::ToneMapper`] maps [`HeadlessRenderTarget::color`] into the canvas frame. Make one
/// per canvas size (again in the layer's `resize`).
///
/// MULTISAMPLING (Not in C#): with a sample count of 4 ([`HeadlessRenderTarget::set_sample_count`]
/// or [`GraphicsContext::create_headless_target_with_samples`]) the target also holds a
/// multisampled colour texture and a multisampled depth texture; [`HeadlessRenderTarget::view`]
/// then hands out a multisampled target whose every render pass ENDS BY RESOLVING into
/// [`HeadlessRenderTarget::color`], which stays an ordinary texture: tone map it, bloom it, read
/// it exactly as with 1 sample. The library's renderers pick their 4-sample pipelines by
/// themselves. The buffers of a count are made the first time it is used and KEPT while the
/// target lives (switching back and forth makes nothing new;
/// [`HeadlessRenderTarget::release_unused_buffers`] frees the other count's). With 4 samples a
/// pass that LOADS continues from what the previous pass on this target left in the multisampled
/// buffer: something written into `color()` another way (a copy, a compute pass, bloom) is not
/// seen by a later pass on the target and is overwritten by its resolve - so bloom comes after the
/// target's last scene pass. Only geometric edges are smoothed (texture shimmer is mipmaps' job),
/// and on the HDR path the resolve averages BEFORE tone mapping, so very bright edges stay
/// slightly rough.
///
/// Its reads are ASYNC here (C#'s `ReadPixels` / `ReadRgba` / `ReadDepth` wait for the GPU; a
/// browser cannot). Not here: `SaveAs`, `SaveAsPng` and `ToImage` - a browser has no file system
/// and no image encoder; an application shows its frames, or hands the read pixels on.
pub struct HeadlessRenderTarget {
    context: GraphicsContext,
    options: HeadlessTargetOptions,
    color: GpuTexture,
    depth: Option<GpuTexture>,
    multi: Option<MultisampledBuffers>,
    sample_count: u32,
}

impl HeadlessRenderTarget {
    /// The colour texture (render attachment, sampled, copyable) (C#: `Color`). With 4 samples it
    /// is the RESOLVED picture, an ordinary single-sampled texture: what every pass resolves into.
    pub fn color(&self) -> &GpuTexture {
        &self.color
    }

    /// The depth texture the passes use, if the target has one (C#: `Depth`): multisampled (and
    /// so neither readable nor sampled as a plain texture) with 4 samples.
    pub fn depth(&self) -> Option<&GpuTexture> {
        if self.sample_count > 1 {
            self.multi.as_ref().and_then(|multi| multi.depth.as_ref())
        } else {
            self.depth.as_ref()
        }
    }

    /// The multisampled colour texture the passes draw into with 4 samples; None with 1 (Not in C#).
    pub fn multisampled_color(&self) -> Option<&GpuTexture> {
        if self.sample_count > 1 {
            self.multi.as_ref().map(|multi| &multi.color)
        } else {
            None
        }
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

    /// The samples per pixel the passes draw with: 1 (default) or 4 (Not in C#).
    pub fn sample_count(&self) -> u32 {
        self.sample_count
    }

    /// Sets the samples per pixel, 1 or 4, at any time (between frames) (Not in C#). The
    /// multisampled buffers are made the first time 4 is used; both counts' buffers are then kept.
    /// Err for any other count, or when a buffer cannot be made; the setting is then unchanged.
    pub fn set_sample_count(&mut self, count: u32) -> Result<(), GraphicsException> {
        check_sample_count(count, "a headless render target")?;
        let have = BufferSets { single: self.depth.is_some(), multi: self.multi.is_some() };
        let size = (self.width(), self.height());
        let plan = plan_buffers(have, Some((size, self.options.format)), (size, self.options.format), count, self.options.with_depth);
        if plan.create_single {
            self.depth = Some(create_depth(&self.context, size.0, size.1, &self.options)?);
        }
        if plan.create_multi {
            self.multi = Some(create_multi(&self.context, size.0, size.1, &self.options, count)?);
        }
        self.sample_count = count;
        Ok(())
    }

    /// Frees the buffers of the sample count NOT in use (the multisampled ones with 1 sample, the
    /// single-sampled depth with 4) (Not in C#); switching back makes them again.
    pub fn release_unused_buffers(&mut self) {
        if self.sample_count > 1 {
            self.depth = None;
        } else {
            self.multi = None;
        }
    }

    /// The target to pass to [`crate::GraphicsFrame::begin_render_pass_to`]: the colour and, if
    /// there is one, the depth (C#: `View`). With 4 samples: the multisampled colour and depth,
    /// resolving into [`HeadlessRenderTarget::color`].
    pub fn view(&self) -> RenderTargetView {
        match (&self.multi, self.sample_count > 1) {
            (Some(multi), true) => {
                let view = RenderTargetView::new(multi.color.view().clone(), multi.color.format(), self.width(), self.height(), multi.depth.as_ref().map(|depth| (depth.view().clone(), depth.format())));
                view.with_resolve(self.sample_count, self.color.view().clone()).expect("the count was checked when it was set")
            }
            _ => {
                let view = self.color.as_render_target();
                match &self.depth {
                    Some(depth) => view.with_depth(depth),
                    None => view,
                }
            }
        }
    }

    /// Reads the colour texture (the resolved picture with 4 samples) as tight rows in its own
    /// byte order, after the work submitted so far (C#: `ReadPixels`; async here).
    pub async fn read_pixels(&self) -> Result<Vec<u8>, GraphicsException> {
        self.color.read_pixels(0).await
    }

    /// Reads the colour texture (RGBA8 or BGRA8; the resolved picture with 4 samples) as RGBA
    /// pixels, red first (C#: `ReadRgba`; async here).
    pub async fn read_rgba(&self) -> Result<Vec<u8>, GraphicsException> {
        self.color.read_rgba().await
    }

    /// Reads the depth texture (Depth32Float only) as one f32 per pixel, row by row (C#:
    /// `ReadDepth`; async here). Err with 4 samples: WebGPU cannot copy a multisampled texture.
    pub async fn read_depth(&self) -> Result<Vec<f32>, GraphicsException> {
        if self.sample_count > 1 {
            return Err(GraphicsException::new("The target draws with 4 samples, and WebGPU cannot read a multisampled depth texture; set_sample_count(1) and draw again to read depth."));
        }
        match &self.depth {
            Some(depth) if depth.format() == wgpu::TextureFormat::Depth32Float => {
                let bytes = depth.read_pixels(0).await?;
                Ok(bytemuck::pod_collect_to_vec(&bytes))
            }
            _ => Err(GraphicsException::new("The target has no Depth32Float depth texture; create it with HeadlessTargetOptions { with_depth: true, depth_format: Depth32Float, .. } to read depth.")),
        }
    }
}

fn create_depth(context: &GraphicsContext, width: u32, height: u32, options: &HeadlessTargetOptions) -> Result<GpuTexture, GraphicsException> {
    context.create_depth_texture(width, height, DepthTextureOptions { format: options.depth_format, label: Some("headless depth".to_string()) })
}

fn create_multi(context: &GraphicsContext, width: u32, height: u32, options: &HeadlessTargetOptions, count: u32) -> Result<MultisampledBuffers, GraphicsException> {
    let color = create_multisampled_texture(context, width, height, options.format, count, "headless multisampled colour")?;
    let depth = if options.with_depth { Some(create_multisampled_texture(context, width, height, options.depth_format, count, "headless multisampled depth")?) } else { None };
    Ok(MultisampledBuffers { color, depth })
}

impl GraphicsContext {
    /// An offscreen target of `width` x `height` pixels (C#: `CreateHeadlessTarget(width, height,
    /// format, withDepth, depthFormat)`), drawing with 1 sample per pixel.
    pub fn create_headless_target(&self, width: u32, height: u32, options: HeadlessTargetOptions) -> Result<HeadlessRenderTarget, GraphicsException> {
        self.create_headless_target_with_samples(width, height, options, 1)
    }

    /// An offscreen target drawing with `sample_count` samples per pixel, 1 or 4 (Not in C#; see
    /// [`HeadlessRenderTarget`]'s MULTISAMPLING). Only the buffers that count needs are made.
    pub fn create_headless_target_with_samples(&self, width: u32, height: u32, options: HeadlessTargetOptions, sample_count: u32) -> Result<HeadlessRenderTarget, GraphicsException> {
        check_sample_count(sample_count, "a headless render target")?;
        let color = self.create_render_texture(width, height, RenderTextureOptions { format: options.format, label: Some("headless colour".to_string()) })?;
        let mut target = HeadlessRenderTarget { context: self.clone(), options, color, depth: None, multi: None, sample_count: 1 };
        target.set_sample_count(sample_count)?;
        Ok(target)
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
