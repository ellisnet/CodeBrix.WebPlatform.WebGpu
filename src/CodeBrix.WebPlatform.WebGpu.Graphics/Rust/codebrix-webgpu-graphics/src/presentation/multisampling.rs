//! Multisampling (MSAA) bookkeeping shared by the offscreen target and the canvas frame target
//! (Not in C#): the accepted sample counts, and the plan of which buffers a target makes, keeps
//! or drops when its size, format or sample count changes - worked out on the CPU so that the
//! rules are tested on the host.

use crate::context::{label_text, GraphicsContext, GraphicsException};
use crate::resources::GpuTexture;

/// The sample counts a target accepts: 1 (no multisampling, the default) and 4 - the two counts
/// every WebGPU implementation supports for every renderable format the library uses.
pub const SAMPLE_COUNTS: [u32; 2] = [1, 4];

/// Ok for 1 and 4; Err naming `what` for any other count.
pub(crate) fn check_sample_count(count: u32, what: &str) -> Result<(), GraphicsException> {
    if SAMPLE_COUNTS.contains(&count) {
        Ok(())
    } else {
        Err(GraphicsException::new(format!("{} cannot use a sample count of {count}: the counts are 1 (no multisampling) and 4 (4x MSAA), the two every WebGPU implementation supports.", capitalized(what))))
    }
}

/// The usage of a multisampled texture of `format`: a render attachment, and for a DEPTH format
/// also a texture binding - bound as `texture_depth_multisampled_2d` and read with `textureLoad`
/// by the water's scene capture (Not in C#; core WebGPU forbids only STORAGE on a multisampled
/// texture, and a browser probe on this crate's wgpu proved the binding). A multisampled colour
/// texture stays a render attachment only (what its passes resolve into is the readable one).
pub(crate) fn multisampled_usage(format: wgpu::TextureFormat) -> wgpu::TextureUsages {
    if format.is_depth_stencil_format() {
        wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING
    } else {
        wgpu::TextureUsages::RENDER_ATTACHMENT
    }
}

/// A multisampled render attachment of `width` x `height` pixels, `format` and `sample_count`
/// samples (colour or depth): drawn into and resolved from, never copied; a depth texture can also
/// be bound for reading ([`multisampled_usage`]).
pub(crate) fn create_multisampled_texture(context: &GraphicsContext, width: u32, height: u32, format: wgpu::TextureFormat, sample_count: u32, label: &str) -> Result<GpuTexture, GraphicsException> {
    check_sample_count(sample_count, "a multisampled texture")?;
    if width < 1 || height < 1 {
        return Err(GraphicsException::new(format!("The texture{} must be at least 1 x 1 pixels; it was asked for {width} x {height}.", label_text(Some(label)))));
    }
    let limit = context.limits().max_texture_dimension_2d;
    if width > limit || height > limit {
        return Err(GraphicsException::new(format!("The texture{} of {width} x {height} pixels is larger than the device allows ({limit} per side); make it smaller.", label_text(Some(label)))));
    }
    let usage = multisampled_usage(format);
    let handle = context.device().create_texture(&wgpu::TextureDescriptor {
        label: Some(label),
        size: wgpu::Extent3d { width, height, depth_or_array_layers: 1 },
        mip_level_count: 1,
        sample_count,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage,
        view_formats: &[],
    });
    let view = handle.create_view(&wgpu::TextureViewDescriptor::default());
    Ok(GpuTexture::new(context.clone(), handle, view, width, height, format, 1, usage, false, Some(label)))
}

fn capitalized(text: &str) -> String {
    let mut chars = text.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        None => String::new(),
    }
}

/// Which of a target's two buffer sets exist: the SINGLE-sampled set (what a 1-sample frame needs
/// beyond the target's own colour - its depth) and the MULTISAMPLED set (multisampled colour and
/// depth, and for a canvas the resolve texture).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct BufferSets {
    /// The single-sampled set exists.
    pub single: bool,
    /// The multisampled set exists.
    pub multi: bool,
}

/// What a target does before a frame: drop every buffer (its size or format changed), then make
/// the set the current sample count needs if it is missing. The other count's set, once made, is
/// KEPT while the size and format stay (switching the setting back and forth makes nothing new).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct BufferPlan {
    /// Drop both sets first (a resize or a new format).
    pub drop_all: bool,
    /// Make the single-sampled set.
    pub create_single: bool,
    /// Make the multisampled set.
    pub create_multi: bool,
}

/// The plan for a target that has `have`, made for `made_for` (size and format; None before its
/// first frame), about to draw a frame of `wanted` with `sample_count` samples. `needs_single` is
/// false when a 1-sample frame needs no buffer of the target's own (a canvas target without
/// depth: the canvas's picture is all it draws into).
pub(crate) fn plan_buffers<F: PartialEq + Copy>(have: BufferSets, made_for: Option<((u32, u32), F)>, wanted: ((u32, u32), F), sample_count: u32, needs_single: bool) -> BufferPlan {
    let drop_all = made_for != Some(wanted) && (have.single || have.multi);
    let have = if drop_all || made_for != Some(wanted) { BufferSets::default() } else { have };
    if sample_count > 1 {
        BufferPlan { drop_all, create_single: false, create_multi: !have.multi }
    } else {
        BufferPlan { drop_all, create_single: needs_single && !have.single, create_multi: false }
    }
}

#[cfg(test)]
mod tests {
    use super::{check_sample_count, multisampled_usage, plan_buffers, BufferPlan, BufferSets, SAMPLE_COUNTS};

    #[test]
    fn a_multisampled_depth_texture_can_be_bound_for_reading_and_a_colour_one_cannot() {
        let depth = multisampled_usage(wgpu::TextureFormat::Depth32Float);
        assert!(depth.contains(wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING));
        assert!(!depth.contains(wgpu::TextureUsages::STORAGE_BINDING), "WebGPU forbids storage on a multisampled texture");
        assert!(multisampled_usage(wgpu::TextureFormat::Depth24Plus).contains(wgpu::TextureUsages::TEXTURE_BINDING));
        for color in [wgpu::TextureFormat::Rgba16Float, wgpu::TextureFormat::Rgba8Unorm, wgpu::TextureFormat::Bgra8UnormSrgb] {
            assert_eq!(multisampled_usage(color), wgpu::TextureUsages::RENDER_ATTACHMENT, "{color:?}");
        }
    }

    #[test]
    fn only_one_and_four_samples_are_accepted() {
        assert_eq!(SAMPLE_COUNTS, [1, 4]);
        assert!(check_sample_count(1, "a canvas frame target").is_ok());
        assert!(check_sample_count(4, "a canvas frame target").is_ok());
        for refused in [0, 2, 3, 8, 16] {
            let message = check_sample_count(refused, "a headless render target").unwrap_err().to_string();
            assert!(message.starts_with("A headless render target cannot use a sample count of"), "{message}");
            assert!(message.contains(&refused.to_string()) && message.contains("1 (no multisampling) and 4"), "{message}");
        }
    }

    const SIZE: (u32, u32) = (800, 600);
    const FORMAT: u8 = 1;

    #[test]
    fn the_first_frame_makes_only_the_set_its_count_needs() {
        let none = BufferSets::default();
        assert_eq!(plan_buffers(none, None, (SIZE, FORMAT), 1, true), BufferPlan { drop_all: false, create_single: true, create_multi: false });
        assert_eq!(plan_buffers(none, None, (SIZE, FORMAT), 4, true), BufferPlan { drop_all: false, create_single: false, create_multi: true });
        // No depth on a 1-sample canvas: nothing of its own to make.
        assert_eq!(plan_buffers(none, None, (SIZE, FORMAT), 1, false), BufferPlan::default());
    }

    #[test]
    fn buffers_are_kept_frame_after_frame_and_across_setting_changes() {
        let made = Some((SIZE, FORMAT));
        let single = BufferSets { single: true, multi: false };
        assert_eq!(plan_buffers(single, made, (SIZE, FORMAT), 1, true), BufferPlan::default(), "the next frame makes nothing");
        // Switched on: the multisampled set is made once, the single set kept.
        assert_eq!(plan_buffers(single, made, (SIZE, FORMAT), 4, true), BufferPlan { drop_all: false, create_single: false, create_multi: true });
        let both = BufferSets { single: true, multi: true };
        assert_eq!(plan_buffers(both, made, (SIZE, FORMAT), 4, true), BufferPlan::default());
        // Switched back off and on again: nothing new.
        assert_eq!(plan_buffers(both, made, (SIZE, FORMAT), 1, true), BufferPlan::default());
        assert_eq!(plan_buffers(both, made, (SIZE, FORMAT), 4, true), BufferPlan::default());
        // Off for the first time after starting on: the single set is made then.
        let multi = BufferSets { single: false, multi: true };
        assert_eq!(plan_buffers(multi, made, (SIZE, FORMAT), 1, true), BufferPlan { drop_all: false, create_single: true, create_multi: false });
    }

    #[test]
    fn a_resize_or_a_new_format_drops_everything_and_remakes_only_the_current_set() {
        let both = BufferSets { single: true, multi: true };
        let made = Some((SIZE, FORMAT));
        assert_eq!(plan_buffers(both, made, ((1024, 600), FORMAT), 4, true), BufferPlan { drop_all: true, create_single: false, create_multi: true });
        assert_eq!(plan_buffers(both, made, (SIZE, 2u8), 1, true), BufferPlan { drop_all: true, create_single: true, create_multi: false });
        // Nothing to drop: no drop.
        assert_eq!(plan_buffers(BufferSets::default(), made, ((10, 10), FORMAT), 4, true), BufferPlan { drop_all: false, create_single: false, create_multi: true });
    }
}
