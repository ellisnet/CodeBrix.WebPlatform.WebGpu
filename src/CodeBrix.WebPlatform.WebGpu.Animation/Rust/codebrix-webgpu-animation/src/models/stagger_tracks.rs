//! `StaggerTracks`: what a staggered entrance binds to each element, and each element's state at
//! a time (the calculation of C# `DrawingAnimations.BindStagger`).

use crate::animation::{FloatTrack, PointTrack};
use crate::geometry::{PointF, RectangleF};
use crate::models::{DrawingTransform, StaggerOptions};

/// What a staggered entrance does to ONE element (the calculation of C#
/// `DrawingAnimations.BindStagger`): its place in the starting order, its time span, the anchor
/// its transform turns about, and the keyframe tracks the stagger binds - opacity always, and
/// position (the offset - the stagger then owns the translation), scale and rotation only when
/// the options change them. Every value is a PURE function of the time passed in:
/// [`state_at`](StaggerTracks::state_at). Before its start an element holds its "from" state,
/// after its end its final state.
#[derive(Clone, Debug)]
pub struct StaggerTracks {
    /// The element's index in the list given.
    pub index: usize,
    /// Its place in the starting order (0 starts first).
    pub rank: usize,
    /// When it starts: start_time + rank x interval, seconds.
    pub start_time: f64,
    /// When it ends: its start + duration, seconds.
    pub end_time: f64,
    /// The anchor its transform is given first - the centre of its bounds - when the options
    /// anchor at the centre and it has bounds; None to leave its anchor as it is.
    pub anchor: Option<PointF>,
    /// Its opacity: the "from" opacity at the start, eased to the "to" opacity at the end.
    pub opacity: FloatTrack,
    /// Its translation: the "from" offset eased to none (None when the options give no offset).
    pub position: Option<PointTrack>,
    /// Its uniform scale: the "from" scale eased to 1 (None when the options give scale 1).
    pub scale: Option<FloatTrack>,
    /// Its rotation, degrees: the "from" rotation eased to 0 (None when the options give 0).
    pub rotation: Option<FloatTrack>,
}

/// One element's stagger state at a time (from [`StaggerTracks::state_at`]): what the bindings
/// set. A None member is not bound - the element keeps its own value.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StaggerState {
    /// The opacity, clamped to 0..1 (as C#'s binding clamps it).
    pub opacity: f32,
    /// The translation.
    pub offset: Option<PointF>,
    /// The uniform scale.
    pub scale: Option<f32>,
    /// The rotation, degrees.
    pub rotation: Option<f32>,
}

impl StaggerState {
    /// The element's transform with the bound parts set (C#'s Position, Scale and Rotation
    /// setters: `WithTranslation`, `WithScale`, `WithRotation`) and the stagger's anchor, when it
    /// has one.
    pub fn apply_to(&self, transform: DrawingTransform, anchor: Option<PointF>) -> DrawingTransform {
        let mut t = transform;
        if let Some(anchor) = anchor {
            t = t.with_anchor(anchor);
        }
        if let Some(offset) = self.offset {
            t = t.with_translation(offset);
        }
        if let Some(scale) = self.scale {
            t = t.with_scale(scale);
        }
        if let Some(rotation) = self.rotation {
            t = t.with_rotation(rotation);
        }
        t
    }
}

impl StaggerTracks {
    /// The stagger of a list of elements, given each element's local bounds (None for an element
    /// with none - its anchor is left alone) (C# `BindStagger(elements, options = null)` without
    /// the binding: the animation model binds these tracks). The error says which option is out
    /// of range.
    pub fn create(element_bounds: &[Option<RectangleF>], options: &StaggerOptions) -> Result<Vec<StaggerTracks>, String> {
        options.validate()?;
        let count = element_bounds.len();
        let mut list = Vec::with_capacity(count);
        for (i, bounds) in element_bounds.iter().enumerate() {
            let rank = options.order.rank_of(i, count);
            let start = options.start_time + (rank as f64 * options.interval);
            let end = start + options.duration;
            let easing = options.easing;
            let anchor = if options.anchor_at_center { bounds.map(|b| PointF::new(b.x + (b.width / 2.0), b.y + (b.height / 2.0))) } else { None };
            let mut opacity = FloatTrack::new();
            opacity.add(start, options.from_opacity, crate::animation::EasingKind::Linear)?.add(end, options.to_opacity, easing)?;
            let position = if options.from_offset.x != 0.0 || options.from_offset.y != 0.0 {
                let mut track = PointTrack::new();
                track.add(start, options.from_offset, crate::animation::EasingKind::Linear)?.add(end, PointF::new(0.0, 0.0), easing)?;
                Some(track)
            } else {
                None
            };
            let scale = if options.from_scale != 1.0 {
                let mut track = FloatTrack::new();
                track.add(start, options.from_scale, crate::animation::EasingKind::Linear)?.add(end, 1.0, easing)?;
                Some(track)
            } else {
                None
            };
            let rotation = if options.from_rotation != 0.0 {
                let mut track = FloatTrack::new();
                track.add(start, options.from_rotation, crate::animation::EasingKind::Linear)?.add(end, 0.0, easing)?;
                Some(track)
            } else {
                None
            };
            list.push(StaggerTracks { index: i, rank, start_time: start, end_time: end, anchor, opacity, position, scale, rotation });
        }
        Ok(list)
    }

    /// How far through its span the element is at a time, 0..1 before easing (0 before its start,
    /// 1 from its end on; a zero duration jumps at the start).
    pub fn progress_at(&self, seconds: f64) -> f64 {
        if seconds.is_nan() || seconds < self.start_time {
            return 0.0;
        }
        if seconds >= self.end_time {
            return 1.0;
        }
        (seconds - self.start_time) / (self.end_time - self.start_time)
    }

    /// The element's state at a time - what its bindings set when the animation model applies
    /// that time.
    pub fn state_at(&self, seconds: f64) -> StaggerState {
        let opacity = self.opacity.evaluate(seconds).unwrap_or(1.0);
        StaggerState {
            opacity: if opacity.is_nan() { 0.0 } else { opacity.clamp(0.0, 1.0) },
            offset: self.position.as_ref().and_then(|t| t.evaluate(seconds)),
            scale: self.scale.as_ref().and_then(|t| t.evaluate(seconds)),
            rotation: self.rotation.as_ref().and_then(|t| t.evaluate(seconds)),
        }
    }
}
