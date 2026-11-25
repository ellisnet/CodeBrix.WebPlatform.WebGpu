//! [`LodInstance`], [`LodLevel`] and [`LodState`]: level of detail with a fade band (Not in C#).
//! An object made of several levels, most detailed first, each used out to a set distance from
//! the camera, drawn by [`ModelRenderer::draw_lod`](crate::ModelRenderer::draw_lod). The choice of
//! the levels (distances, fade bands, hysteresis, the quality scale) is CPU-only and tested on the
//! host; the fade itself is the dither pattern of `lod_dither`.

use std::cell::Cell;

use glam::{Mat4, Vec3};

use crate::context::GraphicsException;
use crate::models::lod_dither::{dither_code, DITHER_STEPS};
use crate::models::ModelInstance;
use crate::visibility::CullVolume;

/// One level of a [`LodInstance`] (Not in C#): ONE instance, or a SET of instances (one canopy
/// surface that gives way to many trees), used out to [`LodLevel::max_distance`] from the camera.
/// Each instance's [`ModelInstance::world`] is its place WITHIN the level-of-detail object: it is
/// drawn at `lod.world * instance.world`. Everything else of the instance (its pose, animation,
/// tint, alpha, hidden nodes, shadow switches, `never_cull`) works as for an instance drawn alone.
pub struct LodLevel {
    /// The instances of the level, drawn together (none is allowed: a gap in which nothing shows).
    pub instances: Vec<ModelInstance>,
    max_distance: f32,
}

impl LodLevel {
    /// A level of `instances` used out to `max_distance` (world units from the camera to the
    /// object's position, before the renderer's quality scale). `f32::INFINITY` for the last level
    /// means NO CUT-OFF: the object never disappears with distance.
    pub fn new(instances: Vec<ModelInstance>, max_distance: f32) -> LodLevel {
        LodLevel { instances, max_distance }
    }

    /// A level of one instance.
    pub fn single(instance: ModelInstance, max_distance: f32) -> LodLevel {
        LodLevel::new(vec![instance], max_distance)
    }

    /// The distance out to which the level is used (infinite: no end).
    pub fn max_distance(&self) -> f32 {
        self.max_distance
    }
}

/// What a [`LodInstance`] showed at its last selection (Not in C#) - read it to advance animation
/// only for the levels that are shown ([`LodInstance::shown_levels_mut`]).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum LodState {
    /// Never selected yet.
    NotDrawn,
    /// Left out by culling (outside the camera's view and every shadow map's volume): no distance
    /// was measured, nothing drawn.
    Culled,
    /// One level, whole.
    Level(usize),
    /// Inside a fade band: `from` fades out while `to` fades in (both drawn, with complementary
    /// dither patterns), or - `to` None - the last level fades out towards its cut-off.
    /// `progress` is how far through the fade, in (0, 1) and in steps of 1/64: `from` covers
    /// `1 - progress`, `to` covers `progress`.
    Fading {
        /// The nearer level, fading out.
        from: usize,
        /// The farther level, fading in; None at the final cut-off.
        to: Option<usize>,
        /// How far through the fade, 0 (all `from`) to 1 (all `to`), exclusive.
        progress: f32,
    },
    /// Beyond the last level's maximum distance: nothing drawn.
    BeyondCutOff,
}

impl LodState {
    /// The share of a level's pixels that are drawn, 0 when it is not shown.
    pub fn coverage(&self, level: usize) -> f32 {
        match *self {
            LodState::Level(shown) if shown == level => 1.0,
            LodState::Fading { from, progress, .. } if from == level => 1.0 - progress,
            LodState::Fading { to: Some(to), progress, .. } if to == level => progress,
            _ => 0.0,
        }
    }

    /// True when `level` is drawn (wholly or fading).
    pub fn is_shown(&self, level: usize) -> bool {
        self.coverage(level) > 0.0
    }

    /// True inside a fade band.
    pub fn is_fading(&self) -> bool {
        matches!(self, LodState::Fading { .. })
    }

    /// The levels drawn, each with its coverage: none, one, or two while fading (nearer first).
    pub fn shown_levels(&self) -> Vec<(usize, f32)> {
        match *self {
            LodState::Level(level) => vec![(level, 1.0)],
            LodState::Fading { from, to, progress } => {
                let mut shown = vec![(from, 1.0 - progress)];
                shown.extend(to.map(|to| (to, progress)));
                shown
            }
            _ => Vec::new(),
        }
    }

    /// The draws of the state: each level drawn with its dither code (0: whole).
    pub(crate) fn draws(&self) -> Vec<(usize, u32)> {
        match *self {
            LodState::Level(level) => vec![(level, 0)],
            LodState::Fading { from, to, progress } => {
                let step = (progress * DITHER_STEPS as f32).round() as u32;
                let mut draws = vec![(from, dither_code(false, step))];
                draws.extend(to.map(|to| (to, dither_code(true, step))));
                draws
            }
            _ => Vec::new(),
        }
    }
}

/// A level-of-detail object (Not in C#): several [`LodLevel`]s, most detailed first, sharing ONE
/// position ([`LodInstance::world`]); the renderer draws only the level the camera's distance
/// calls for ([`ModelRenderer::draw_lod`](crate::ModelRenderer::draw_lod)). The application makes
/// the simpler models itself.
///
/// - THE CHOICE: the distance from the camera to the object's position (the translation of
///   `world`), against each level's maximum distance times the renderer's quality scale
///   ([`ModelRenderer::set_lod_distance_scale`](crate::ModelRenderer::set_lod_distance_scale)):
///   the first level whose distance is not yet reached is shown.
/// - THE FADE BAND ([`LodInstance::fade_band`], default [`LodInstance::DEFAULT_FADE_BAND`]): the
///   last part of each level's range - from `(1 - fade_band) x` its distance up to its distance,
///   never reaching back into the level before - is a cross-fade in which BOTH levels are drawn,
///   one fading out as the other fades in, with a fine dither (dot) pattern: no sorting, depth
///   written as usual, and the two levels' patterns complementary, so the pair never leaves a hole
///   and never covers a pixel twice. At its distance a level is gone and the next is whole. 0 is a
///   hard cut.
/// - NO CUT-OFF unless the last level has a finite maximum distance; then it fades out over its
///   band and nothing is drawn beyond it ([`LodState::BeyondCutOff`]).
/// - HYSTERESIS ([`LodInstance::hysteresis`], default [`LodInstance::DEFAULT_HYSTERESIS`]): the
///   distance the choice uses follows the camera's with a slack of that fraction of the distance,
///   so a camera that hovers at a boundary (or at the edge of a band) does not make the object
///   flicker: moving out, a switch happens that much past its distance, moving in, that much before
///   it. The fade stays continuous.
/// - WHAT IS SHOWN: [`LodInstance::state`] after each draw (or
///   [`ModelRenderer::select_lod`](crate::ModelRenderer::select_lod) before it): advance the
///   animation of the shown levels only ([`LodInstance::shown_levels_mut`]).
/// - CULLED FIRST: an object whose levels' bounds are outside the camera's view (and outside every
///   shadow map's volume) is left out before any distance is measured.
/// - SHADOWS: the levels that are drawn cast; a fading level's shadow fades with the same dither
///   pattern (in the shadow map's own texels), so a cross-fade casts one whole shadow and a
///   cut-off's shadow thins out with the object instead of popping.
/// - BLENDED PARTS of a fading level fade by alpha (their alpha times the coverage) instead of the
///   dither: they write no depth, so nothing needs a hard per-pixel choice, and glass stays smooth.
///   No holes, but in a cross-fade the two half-faded parts are less opaque together than one
///   whole one: a part of opacity `a` shows as `1 - (1 - a / 2)^2` at mid-fade (0.3 as 0.28; a
///   fully opaque blended part as 0.75), and the level drawn last leads the colour.
///
/// One `LodInstance` follows ONE camera: the hysteresis keeps the last distance it was selected
/// at, so draw an object seen by two views as two `LodInstance`s.
pub struct LodInstance {
    /// The object's place: every level's instances are drawn at `world * instance.world`, and its
    /// translation is the position distances are measured to.
    pub world: Mat4,
    /// The fade band as a fraction of each switching distance, 0 to 1 (0: hard cuts). Default
    /// [`LodInstance::DEFAULT_FADE_BAND`].
    pub fade_band: f32,
    /// The slack of the distance the choice follows, as a fraction of the distance, 0 to 0.5.
    /// Default [`LodInstance::DEFAULT_HYSTERESIS`].
    pub hysteresis: f32,
    /// True to keep the object even when its bounds are outside every view (as
    /// [`ModelInstance::never_cull`] for one instance). Default false.
    pub never_cull: bool,
    levels: Vec<LodLevel>,
    choice: LodChoice,
    /// The levels' bounds in the object's space, worked out when first needed.
    bounds: Cell<Option<Option<(Vec3, Vec3)>>>,
}

impl LodInstance {
    /// The default fade band: the last tenth of each level's range.
    pub const DEFAULT_FADE_BAND: f32 = 0.1;
    /// The default hysteresis: 2 % of the distance.
    pub const DEFAULT_HYSTERESIS: f32 = 0.02;

    /// An object at `world` with `levels`, most detailed first. An error when there is no level,
    /// when a maximum distance is not above 0 (or is NaN), when the distances do not grow from
    /// level to level, or when a level before the last has no end (`f32::INFINITY`).
    pub fn new(world: Mat4, levels: Vec<LodLevel>) -> Result<LodInstance, GraphicsException> {
        check_distances(&levels.iter().map(|l| l.max_distance).collect::<Vec<f32>>())?;
        Ok(LodInstance { world, fade_band: LodInstance::DEFAULT_FADE_BAND, hysteresis: LodInstance::DEFAULT_HYSTERESIS, never_cull: false, levels, choice: LodChoice::default(), bounds: Cell::new(None) })
    }

    /// The levels, most detailed first.
    pub fn levels(&self) -> &[LodLevel] {
        &self.levels
    }

    /// A level to change (its instances, their places); None out of range. The object's bounds
    /// are worked out again at the next draw.
    pub fn level_mut(&mut self, level: usize) -> Option<&mut LodLevel> {
        self.bounds.set(None);
        self.levels.get_mut(level)
    }

    /// Sets a level's maximum distance; an error (and no change) when the distances would no
    /// longer grow, or a level before the last would have no end.
    pub fn set_max_distance(&mut self, level: usize, max_distance: f32) -> Result<(), GraphicsException> {
        let mut distances: Vec<f32> = self.levels.iter().map(|l| l.max_distance).collect();
        let Some(slot) = distances.get_mut(level) else {
            return Err(GraphicsException::new(format!("LodInstance::set_max_distance: there is no level {level} (the object has {} levels).", self.levels.len())));
        };
        *slot = max_distance;
        check_distances(&distances)?;
        self.levels[level].max_distance = max_distance;
        Ok(())
    }

    /// The levels that are shown at the last selection, to advance their animation (and only
    /// theirs): `for level in lod.shown_levels_mut() { AnimationPlayer::update_all_to(&mut
    /// level.instances, time); }`. Change an instance's place (`world`) through
    /// [`LodInstance::level_mut`], which makes the object's bounds be worked out again.
    pub fn shown_levels_mut(&mut self) -> impl Iterator<Item = &mut LodLevel> {
        let state = self.choice.state;
        self.levels.iter_mut().enumerate().filter(move |(index, _)| state.is_shown(*index)).map(|(_, level)| level)
    }

    /// What the last selection showed.
    pub fn state(&self) -> LodState {
        self.choice.state
    }

    /// The position distances are measured to: the translation of [`LodInstance::world`].
    pub fn position(&self) -> Vec3 {
        self.world.w_axis.truncate()
    }

    /// Chooses the levels for a camera at `camera_position` with every distance times
    /// `distance_scale` (the renderer's quality scale; 1 for the distances as given), WITHOUT
    /// culling, and returns the state (kept, as [`LodInstance::state`]). The renderer calls it;
    /// call it yourself to choose before a frame, outside a renderer.
    pub fn select(&mut self, camera_position: Vec3, distance_scale: f32) -> LodState {
        let distances: Vec<f32> = self.levels.iter().map(|l| l.max_distance).collect();
        self.choice.select(&distances, camera_position.distance(self.position()), self.fade_band, self.hysteresis, distance_scale)
    }

    /// Records that culling left the object out (no distance measured).
    pub(crate) fn mark_culled(&mut self) {
        self.choice.state = LodState::Culled;
    }

    /// The bounds of every level's instances (their models' culling bounds placed by their
    /// `world`) in the object's space; None when no level has an instance.
    pub fn bounds(&self) -> Option<(Vec3, Vec3)> {
        if let Some(bounds) = self.bounds.get() {
            return bounds;
        }
        let bounds = union_bounds(self.levels.iter().flat_map(|l| l.instances.iter()).map(|i| (i.model().culling_bounds(), i.world)));
        self.bounds.set(Some(bounds));
        bounds
    }
}

/// The state of the choice that persists between frames: the distance followed (hysteresis) and
/// the last state.
#[derive(Clone, Copy, Debug)]
pub(crate) struct LodChoice {
    pub(crate) followed: Option<f32>,
    pub(crate) state: LodState,
}

impl Default for LodChoice {
    fn default() -> LodChoice {
        LodChoice { followed: None, state: LodState::NotDrawn }
    }
}

impl LodChoice {
    /// One selection at camera distance `distance`: follows it with the hysteresis' slack, then
    /// chooses ([`choose_levels`]); keeps and returns the state.
    pub(crate) fn select(&mut self, max_distances: &[f32], distance: f32, fade_band: f32, hysteresis: f32, scale: f32) -> LodState {
        let followed = follow_distance(self.followed, distance, hysteresis);
        self.followed = Some(followed);
        self.state = choose_levels(max_distances, fade_band, scale, followed);
        self.state
    }
}

/// The distance the choice uses: the camera's distance, or - after an earlier selection - the
/// earlier followed distance moved only as far as needed to stay within `hysteresis` (a fraction,
/// clamped to 0..0.5) of the camera's distance. A camera that hovers within that slack changes
/// nothing; one that moves on drags the followed distance along, continuously.
pub(crate) fn follow_distance(previous: Option<f32>, distance: f32, hysteresis: f32) -> f32 {
    let slack = if hysteresis.is_finite() { hysteresis.clamp(0.0, 0.5) } else { 0.0 };
    match previous {
        Some(previous) if previous.is_finite() && distance.is_finite() => previous.clamp(distance * (1.0 - slack), distance * (1.0 + slack)),
        _ => distance,
    }
}

/// The levels for distance `distance` (already followed): levels with maximum distances
/// `max_distances` (each times `scale`; a scale that is not a positive number counts as 1), and a
/// fade band of `fade_band` (0..1) of each switching distance, never reaching back past the level
/// before. Progress is rounded to the dither's steps; a fade at step 0 is the nearer level whole,
/// at the last step the farther one (or nothing, at the cut-off).
pub(crate) fn choose_levels(max_distances: &[f32], fade_band: f32, scale: f32, distance: f32) -> LodState {
    let band = if fade_band.is_finite() { fade_band.clamp(0.0, 1.0) } else { 0.0 };
    let scale = if scale.is_finite() && scale > 0.0 { scale } else { 1.0 };
    let mut previous_end = 0.0f32;
    for (level, &max_distance) in max_distances.iter().enumerate() {
        let end = max_distance * scale;
        if distance < end {
            if band == 0.0 || end.is_infinite() {
                return LodState::Level(level);
            }
            let start = (end * (1.0 - band)).max(previous_end);
            if distance < start || end <= start {
                return LodState::Level(level);
            }
            let step = (((distance - start) / (end - start)) * DITHER_STEPS as f32).round() as u32;
            let to = (level + 1 < max_distances.len()).then_some(level + 1);
            return match step {
                0 => LodState::Level(level),
                s if s >= DITHER_STEPS => to.map_or(LodState::BeyondCutOff, LodState::Level),
                s => LodState::Fading { from: level, to, progress: s as f32 / DITHER_STEPS as f32 },
            };
        }
        previous_end = end;
    }
    LodState::BeyondCutOff
}

/// True when the renderer leaves a level-of-detail object out before choosing (culling on, the
/// object not `never_cull`, its bounds placed by `world` outside the camera's `view` and outside
/// every one of `shadow_volumes` - a level that would be drawn might cast into one). No bounds (no
/// instance): left out.
pub(crate) fn lod_culled(view: &CullVolume, shadow_volumes: &[&CullVolume], culling: bool, never_cull: bool, bounds: Option<(Vec3, Vec3)>, world: Mat4) -> bool {
    let Some(bounds) = bounds else {
        return true;
    };
    let (in_view, shadow_mask) = crate::models::model_draw_plan::cull_passes(view, shadow_volumes, culling, never_cull, true, bounds, world);
    !in_view && shadow_mask == 0
}

/// The axis-aligned union of boxes (min, max) each placed by a matrix; None for none.
pub(crate) fn union_bounds(boxes: impl Iterator<Item = ((Vec3, Vec3), Mat4)>) -> Option<(Vec3, Vec3)> {
    let mut union: Option<(Vec3, Vec3)> = None;
    for ((min, max), world) in boxes {
        if !min.cmple(max).all() {
            continue;
        }
        for corner in 0..8 {
            let local = Vec3::new(if corner & 1 == 0 { min.x } else { max.x }, if corner & 2 == 0 { min.y } else { max.y }, if corner & 4 == 0 { min.z } else { max.z });
            let point = world.transform_point3(local);
            union = Some(match union {
                Some((low, high)) => (low.min(point), high.max(point)),
                None => (point, point),
            });
        }
    }
    union
}

/// The error for maximum distances that cannot be levels.
fn check_distances(distances: &[f32]) -> Result<(), GraphicsException> {
    if distances.is_empty() {
        return Err(GraphicsException::new("A LodInstance needs at least one level; give it its levels, most detailed first."));
    }
    let mut previous = 0.0f32;
    for (level, &distance) in distances.iter().enumerate() {
        if distance.is_nan() || distance <= previous {
            return Err(GraphicsException::new(format!("Level {level} of a LodInstance has the maximum distance {distance}: each level's distance must be above 0 and above the level before's ({previous}).")));
        }
        if distance.is_infinite() && level + 1 < distances.len() {
            return Err(GraphicsException::new(format!("Level {level} of a LodInstance has no end (infinite distance), but more levels follow it; only the last level may have no end.")));
        }
        previous = distance;
    }
    Ok(())
}

/// The draws of a state for levels of `sizes` instances: (level, instance, dither code) for every
/// instance of every shown level - a set-of-instances level fades as one, every instance with the
/// level's code.
pub(crate) fn lod_draws(state: &LodState, sizes: &[usize]) -> Vec<(usize, usize, u32)> {
    state.draws().into_iter().filter(|(level, _)| *level < sizes.len()).flat_map(|(level, code)| (0..sizes[level]).map(move |instance| (level, instance, code))).collect()
}
