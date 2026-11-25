//! What a render pass's colour attachment does (Not in C#): which view it resolves into and
//! whether it loads or clears - worked out on the CPU from the target's sample count, so that the
//! multisampling rules are tested on the host - and the check that a pipeline agrees with the
//! target of the pass it is set in.

use crate::common::Color4;
use crate::context::{label_text, GraphicsException};
use crate::presentation::TargetFormats;

/// How the colour attachment begins.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum ColorLoad {
    /// Keep what the colour view holds.
    Load,
    /// Clear it to this colour.
    Clear(Color4),
}

/// Which view the colour attachment resolves into at the end of the pass.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ResolveInto {
    /// None: a 1-sample target is drawn into directly.
    Nothing,
    /// The target's own resolve view.
    TargetResolve,
    /// The view in `RenderPassOptions::resolve_target`.
    OptionResolve,
}

/// The colour attachment of one pass.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct ColorAttachmentPlan {
    /// How it begins.
    pub(crate) load: ColorLoad,
    /// Where it resolves.
    pub(crate) resolve: ResolveInto,
}

/// The colour attachment of a pass on a target of `sample_count` samples. `target_resolves`: the
/// target has a resolve view; `option_resolves`: the pass's options name one (it wins);
/// `clear`: the pass's clear colour (None loads); `first_layer_pass`: the first pass of a frame on
/// a multisampled canvas layer, which must clear (the layers below are not in the multisampled
/// buffer; the frame lays its resolved picture over them at the end).
pub(crate) fn plan_color_attachment(sample_count: u32, target_resolves: bool, option_resolves: bool, clear: Option<Color4>, first_layer_pass: bool) -> Result<ColorAttachmentPlan, GraphicsException> {
    let resolve = if sample_count > 1 {
        if option_resolves {
            ResolveInto::OptionResolve
        } else if target_resolves {
            ResolveInto::TargetResolve
        } else {
            return Err(GraphicsException::new(format!("The pass's target has {sample_count} samples per pixel but no view to resolve into; make it with RenderTargetView::with_resolve (or use a HeadlessRenderTarget's view), or set RenderPassOptions::resolve_target.")));
        }
    } else if option_resolves {
        return Err(GraphicsException::new("RenderPassOptions::resolve_target is set but the pass's target has 1 sample per pixel; only a multisampled target resolves. Leave resolve_target None, or draw into a 4-sample target."));
    } else {
        ResolveInto::Nothing
    };
    let load = match (clear, first_layer_pass) {
        (Some(color), _) => ColorLoad::Clear(color),
        (None, true) => ColorLoad::Clear(Color4::TRANSPARENT),
        (None, false) => ColorLoad::Load,
    };
    Ok(ColorAttachmentPlan { load, resolve })
}

/// None when a pipeline that draws into `colors` / `depth` with `samples` samples (`label` its
/// label) may be set in a pass on a target of `target`; else the message saying what differs
/// (WebGPU requires all three to be equal). `target_text` describes the target for the message.
pub(crate) fn pipeline_mismatch(colors: &[wgpu::TextureFormat], depth: Option<wgpu::TextureFormat>, samples: u32, label: Option<&str>, target: TargetFormats, target_text: &str) -> Option<String> {
    let what = format!("The render pipeline{}", label_text(label));
    if samples.max(1) != target.sample_count {
        return Some(format!(
            "{what} draws with {samples} sample{} per pixel, but the pass's target ({target_text}) has {}. Make the pipeline with .with_sample_count(pass.target().sample_count()) - or keep one per pass.target().formats(), as the library's renderers do - so that it follows the target's multisampling setting.",
            if samples.max(1) == 1 { "" } else { "s" },
            target.sample_count
        ));
    }
    if colors != [target.color] {
        return Some(format!("{what} draws into {colors:?}, but the pass's target ({target_text}) is {:?}; make it with with_color_target(pass.target().color_format(), ..).", target.color));
    }
    if depth != target.depth {
        return Some(format!("{what} has depth format {depth:?}, but the pass's target ({target_text}) has {:?}; a pipeline must test the target's depth format exactly (with_depth(format, ..), or none for a target without depth).", target.depth));
    }
    None
}

#[cfg(test)]
mod tests {
    use super::{pipeline_mismatch, plan_color_attachment, ColorAttachmentPlan, ColorLoad, ResolveInto};
    use crate::common::Color4;
    use crate::presentation::TargetFormats;

    #[test]
    fn a_one_sample_pass_draws_straight_into_its_view_and_loads_or_clears_as_asked() {
        assert_eq!(plan_color_attachment(1, false, false, None, false).unwrap(), ColorAttachmentPlan { load: ColorLoad::Load, resolve: ResolveInto::Nothing });
        assert_eq!(plan_color_attachment(1, false, false, Some(Color4::BLACK), false).unwrap(), ColorAttachmentPlan { load: ColorLoad::Clear(Color4::BLACK), resolve: ResolveInto::Nothing });
        let message = plan_color_attachment(1, false, true, None, false).unwrap_err().to_string();
        assert!(message.contains("only a multisampled target resolves"), "{message}");
    }

    #[test]
    fn every_four_sample_pass_ends_by_resolving() {
        assert_eq!(plan_color_attachment(4, true, false, None, false).unwrap(), ColorAttachmentPlan { load: ColorLoad::Load, resolve: ResolveInto::TargetResolve });
        assert_eq!(plan_color_attachment(4, true, false, Some(Color4::BLACK), false).unwrap().resolve, ResolveInto::TargetResolve);
        // The pass's own resolve view wins over the target's.
        assert_eq!(plan_color_attachment(4, true, true, None, false).unwrap().resolve, ResolveInto::OptionResolve);
        assert_eq!(plan_color_attachment(4, false, true, None, false).unwrap().resolve, ResolveInto::OptionResolve);
        let message = plan_color_attachment(4, false, false, None, false).unwrap_err().to_string();
        assert!(message.contains("4 samples per pixel but no view to resolve into"), "{message}");
    }

    #[test]
    fn the_first_pass_of_a_multisampled_canvas_layer_clears_and_the_next_ones_load() {
        assert_eq!(plan_color_attachment(4, true, false, None, true).unwrap(), ColorAttachmentPlan { load: ColorLoad::Clear(Color4::TRANSPARENT), resolve: ResolveInto::TargetResolve });
        let red = Color4::new(1.0, 0.0, 0.0, 1.0);
        assert_eq!(plan_color_attachment(4, true, false, Some(red), true).unwrap().load, ColorLoad::Clear(red));
        assert_eq!(plan_color_attachment(4, true, false, None, false).unwrap().load, ColorLoad::Load);
    }

    #[test]
    fn a_pipeline_must_agree_with_the_pass_target_in_sample_count_and_formats() {
        let color = wgpu::TextureFormat::Bgra8UnormSrgb;
        let depth = Some(wgpu::TextureFormat::Depth32Float);
        let four = TargetFormats::new(color, depth, 4);
        assert_eq!(pipeline_mismatch(&[color], depth, 4, None, four, "t"), None);
        let message = pipeline_mismatch(&[color], depth, 1, Some("overlay"), four, "800x600 Bgra8UnormSrgb x4").unwrap();
        assert!(message.starts_with("The render pipeline 'overlay' draws with 1 sample per pixel, but the pass's target (800x600 Bgra8UnormSrgb x4) has 4."), "{message}");
        assert!(message.contains(".with_sample_count(pass.target().sample_count())"), "{message}");
        let one = TargetFormats { sample_count: 1, ..four };
        assert!(pipeline_mismatch(&[color], depth, 4, None, one, "t").unwrap().contains("draws with 4 samples per pixel"));
        // A wrapped pipeline recorded with count 0 counts as 1.
        assert_eq!(pipeline_mismatch(&[color], depth, 0, None, one, "t"), None);
        assert!(pipeline_mismatch(&[wgpu::TextureFormat::Rgba16Float], depth, 1, None, one, "t").unwrap().contains("draws into"));
        assert!(pipeline_mismatch(&[], depth, 1, None, one, "t").unwrap().contains("draws into"));
        assert!(pipeline_mismatch(&[color], None, 1, None, one, "t").unwrap().contains("depth format"));
    }
}
