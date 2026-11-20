//! `DrawingTransition`: dissolve, wipe, slide, push or fade through a colour, as a function of a
//! progress value (C# `DrawingTransition`).

use crate::planes::drawing_plane::validate_size;
use crate::planes::drawing_plane_image::{mix_in_place, mix_to, to_vector};
use crate::planes::plane_kernels as k;
use crate::planes::{DrawingPlane, DrawingPlaneImage, DrawingTransitionDirection, DrawingTransitionKind};
use crate::Color;

/// A frame-to-frame transition applied at a FRACTION from 0 (all outgoing frame) to 1 (all
/// incoming frame) (C# `DrawingTransition`) - a pure function of the frames and the fraction,
/// which the caller computes from the time. It works on 8-bit straight-alpha RGBA frames and on
/// [`DrawingPlaneImage`]s, with the same rules:
/// - a fraction of 0 gives the outgoing frame and 1 the incoming frame EXACTLY; fractions outside
///   0..1 are clamped (NaN is refused);
/// - pixels mix by the premultiplied rule of [`DrawingPlaneImage::lerp`]; with equal alphas a
///   byte channel is `(byte)(a (1 - t) + b t + 0.5)`;
/// - slides and pushes move whole pixels: the offset is `floor(fraction * extent + 0.5)`.
///
/// A transition is an immutable value (its kind, direction, softness and colour are the whole of
/// it - the parameters a GPU version takes as well).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DrawingTransition {
    kind: DrawingTransitionKind,
    direction: DrawingTransitionDirection,
    softness: f32,
    color: Color,
}

impl DrawingTransition {
    fn make(kind: DrawingTransitionKind, direction: DrawingTransitionDirection, softness: f32, color: Color) -> DrawingTransition {
        DrawingTransition { kind, direction, softness, color }
    }

    /// A cross-fade (C# `Dissolve`).
    pub fn dissolve() -> DrawingTransition {
        DrawingTransition::make(DrawingTransitionKind::Dissolve, DrawingTransitionDirection::Right, 0.0, Color::TRANSPARENT)
    }

    /// A wipe: an edge travels in `direction`, leaving the incoming frame behind it (C# `Wipe(direction,
    /// softness = 0)`). With `c` the distance of a pixel's centre from the side the edge starts
    /// at: hard edge - weight 1 where `c < fraction * extent`; soft edge of width s - weight
    /// `clamp((fraction * (extent + s) - c) / s, 0, 1)`. The error says the softness must be a
    /// finite number of pixels, 0 or more.
    pub fn wipe(direction: DrawingTransitionDirection, softness: Option<f32>) -> Result<DrawingTransition, String> {
        let softness = softness.unwrap_or(0.0);
        if !(softness >= 0.0) || softness.is_infinite() {
            return Err("A wipe's softness must be a finite number of pixels, 0 or more.".to_string());
        }
        Ok(DrawingTransition::make(DrawingTransitionKind::Wipe, direction, softness, Color::TRANSPARENT))
    }

    /// A slide: the incoming frame travels in `direction` over the still outgoing frame (C# `Slide`).
    pub fn slide(direction: DrawingTransitionDirection) -> DrawingTransition {
        DrawingTransition::make(DrawingTransitionKind::Slide, direction, 0.0, Color::TRANSPARENT)
    }

    /// A push: the incoming frame travels in `direction` and pushes the outgoing frame out ahead
    /// of it (C# `Push`).
    pub fn push(direction: DrawingTransitionDirection) -> DrawingTransition {
        DrawingTransition::make(DrawingTransitionKind::Push, direction, 0.0, Color::TRANSPARENT)
    }

    /// A fade through a colour (C# `FadeThroughColor`): below 0.5 the outgoing frame mixes
    /// towards the colour by `2 * fraction`; from 0.5 the colour mixes towards the incoming frame
    /// by `2 * fraction - 1`. A transparent colour fades through transparency.
    pub fn fade_through_color(color: Color) -> DrawingTransition {
        DrawingTransition::make(DrawingTransitionKind::FadeThroughColor, DrawingTransitionDirection::Right, 0.0, color)
    }

    /// The kind (C# `Kind`).
    pub fn kind(&self) -> DrawingTransitionKind {
        self.kind
    }

    /// The direction of travel (wipes, slides, pushes; `Right` otherwise) (C# `Direction`).
    pub fn direction(&self) -> DrawingTransitionDirection {
        self.direction
    }

    /// A wipe's soft edge width, pixels (0 = hard; 0 for the other kinds) (C# `Softness`).
    pub fn softness(&self) -> f32 {
        self.softness
    }

    /// The colour a fade passes through (transparent black for the other kinds) (C# `Color`).
    pub fn color(&self) -> Color {
        self.color
    }

    /// Applies the transition to two 8-bit straight-alpha RGBA frames (tightly packed,
    /// row-major) into a destination of its own (C# `Apply(from, to, destination, width, height,
    /// fraction)`; Rust's borrowing keeps the three apart - for C#'s "destination may be the
    /// outgoing frame" use [`DrawingTransition::apply_bytes_onto`]).
    pub fn apply_bytes(&self, from: &[u8], to: &[u8], destination: &mut [u8], width: u32, height: u32, fraction: f32) -> Result<(), String> {
        let length = check_bytes(from, to, destination.len(), width, height)?;
        let f = clamp_fraction(fraction)?;
        let (from, to, destination) = (&from[..length], &to[..length], &mut destination[..length]);
        match self.kind {
            DrawingTransitionKind::Dissolve => {
                for o in (0..length).step_by(4) {
                    write(destination, o, k::mix_bytes(k::pixel_at(from, o), k::pixel_at(to, o), f));
                }
            }
            DrawingTransitionKind::FadeThroughColor => {
                for o in (0..length).step_by(4) {
                    write(destination, o, self.fade_pixel(k::pixel_at(from, o), k::pixel_at(to, o), f));
                }
            }
            DrawingTransitionKind::Wipe => {
                let line = self.wipe_line(width as usize, height as usize, f);
                for o in (0..length).step_by(4) {
                    write(destination, o, k::mix_bytes(k::pixel_at(from, o), k::pixel_at(to, o), self.wipe_weight_at(&line, o / 4, width as usize)));
                }
            }
            _ => {
                for s in self.move_segments(width as usize, height as usize, f) {
                    let source = if s.from_incoming { to } else { from };
                    let src = ((s.source_y * width as usize) + s.source_x) * 4;
                    let dst = ((s.destination_y * width as usize) + s.destination_x) * 4;
                    destination[dst..dst + s.length * 4].copy_from_slice(&source[src..src + s.length * 4]);
                }
            }
        }
        Ok(())
    }

    /// Applies a DISSOLVE, WIPE or FADE to two 8-bit frames, writing the result over the outgoing
    /// frame (C#'s `Apply` with the destination = `from`). A slide or push needs a destination of
    /// its own: the error says to use [`DrawingTransition::apply_bytes`].
    pub fn apply_bytes_onto(&self, from: &mut [u8], to: &[u8], width: u32, height: u32, fraction: f32) -> Result<(), String> {
        let length = check_bytes(from, to, from.len(), width, height)?;
        let f = clamp_fraction(fraction)?;
        let line = match self.kind {
            DrawingTransitionKind::Slide | DrawingTransitionKind::Push => return Err("A slide or push needs a destination separate from both frames: use apply_bytes.".to_string()),
            DrawingTransitionKind::Wipe => self.wipe_line(width as usize, height as usize, f),
            _ => Vec::new(),
        };
        for o in (0..length).step_by(4) {
            let (a, b) = (k::pixel_at(from, o), k::pixel_at(to, o));
            let v = match self.kind {
                DrawingTransitionKind::Dissolve => k::mix_bytes(a, b, f),
                DrawingTransitionKind::Wipe => k::mix_bytes(a, b, self.wipe_weight_at(&line, o / 4, width as usize)),
                _ => self.fade_pixel(a, b, f),
            };
            write(from, o, v);
        }
        Ok(())
    }

    /// Applies the transition to two float images into a NEW image (C# `Apply(from, to, fraction)`).
    pub fn apply(&self, from: &DrawingPlaneImage, to: &DrawingPlaneImage, fraction: f32) -> Result<DrawingPlaneImage, String> {
        let mut destination = DrawingPlaneImage::new(from.width(), from.height())?;
        self.apply_into(from, to, &mut destination, fraction)?;
        Ok(destination)
    }

    /// Applies the transition to two float images, writing into an existing image - no
    /// allocation per frame for the per-pixel kinds (C# `Apply(from, to, destination, fraction)`).
    pub fn apply_into(&self, from: &DrawingPlaneImage, to: &DrawingPlaneImage, destination: &mut DrawingPlaneImage, fraction: f32) -> Result<(), String> {
        from.require_same_size(to)?;
        from.require_same_size(destination)?;
        let f = clamp_fraction(fraction)?;
        match self.kind {
            DrawingTransitionKind::Dissolve => mix_to(from, to, destination, f, None),
            DrawingTransitionKind::FadeThroughColor => {
                let color = self.color_image(from.width(), from.height())?;
                if f < 0.5 {
                    mix_to(from, &color, destination, f * 2.0, None);
                } else {
                    mix_to(&color, to, destination, (f * 2.0) - 1.0, None);
                }
            }
            DrawingTransitionKind::Wipe => {
                let weights = self.wipe_weights(from.width(), from.height(), f)?;
                mix_to(from, to, destination, 0.0, Some(&weights));
            }
            _ => self.move_images(from, to, destination, f),
        }
        Ok(())
    }

    /// Applies a DISSOLVE, WIPE or FADE to two float images, writing the result over the
    /// outgoing image (C#'s `Apply` with the destination = `from`). A slide or push needs a
    /// destination of its own: the error says to use [`DrawingTransition::apply_into`].
    pub fn apply_onto(&self, from: &mut DrawingPlaneImage, to: &DrawingPlaneImage, fraction: f32) -> Result<(), String> {
        from.require_same_size(to)?;
        let f = clamp_fraction(fraction)?;
        match self.kind {
            DrawingTransitionKind::Dissolve => mix_in_place(from, to, f, None),
            DrawingTransitionKind::FadeThroughColor => {
                let color = self.color_image(from.width(), from.height())?;
                if f < 0.5 {
                    mix_in_place(from, &color, f * 2.0, None);
                } else {
                    mix_to(&color, to, from, (f * 2.0) - 1.0, None);
                }
            }
            DrawingTransitionKind::Wipe => {
                let weights = self.wipe_weights(from.width(), from.height(), f)?;
                mix_in_place(from, to, 0.0, Some(&weights));
            }
            _ => return Err("A slide or push needs a destination separate from both images: use apply_into.".to_string()),
        }
        Ok(())
    }

    // ---- Internals --------------------------------------------------------------------------------

    fn is_horizontal(&self) -> bool {
        matches!(self.direction, DrawingTransitionDirection::Left | DrawingTransitionDirection::Right)
    }

    fn wipe_weight(&self, centre: f32, extent: usize, fraction: f32) -> f32 {
        if self.softness > 0.0 {
            let lead = fraction * (extent as f32 + self.softness);
            return k::clamp01((lead - centre) / self.softness);
        }
        if centre < fraction * extent as f32 {
            1.0
        } else {
            0.0
        }
    }

    /// One weight per column (horizontal wipes) or per row (vertical wipes) (C# `WipeLine`).
    fn wipe_line(&self, width: usize, height: usize, fraction: f32) -> Vec<f32> {
        let extent = if self.is_horizontal() { width } else { height };
        let from_start = matches!(self.direction, DrawingTransitionDirection::Right | DrawingTransitionDirection::Down);
        (0..extent)
            .map(|i| {
                let centre = if from_start { i as f32 + 0.5 } else { (extent - i) as f32 - 0.5 };
                self.wipe_weight(centre, extent, fraction)
            })
            .collect()
    }

    fn wipe_weight_at(&self, line: &[f32], pixel: usize, width: usize) -> f32 {
        if self.is_horizontal() {
            line[pixel % width]
        } else {
            line[pixel / width]
        }
    }

    fn wipe_weights(&self, width: u32, height: u32, fraction: f32) -> Result<DrawingPlane, String> {
        let line = self.wipe_line(width as usize, height as usize, fraction);
        let mut plane = DrawingPlane::new(width, height)?;
        let horizontal = self.is_horizontal();
        for y in 0..height {
            let row = plane.get_row_mut(y)?;
            if horizontal {
                row.copy_from_slice(&line);
            } else {
                row.fill(line[y as usize]);
            }
        }
        Ok(plane)
    }

    fn fade_pixel(&self, from: [u8; 4], to: [u8; 4], fraction: f32) -> [u8; 4] {
        let c = self.color.to_rgba32();
        if fraction < 0.5 {
            k::mix_bytes(from, c, fraction * 2.0)
        } else {
            k::mix_bytes(c, to, (fraction * 2.0) - 1.0)
        }
    }

    fn color_image(&self, width: u32, height: u32) -> Result<DrawingPlaneImage, String> {
        let c = to_vector(self.color);
        let mut image = DrawingPlaneImage::new(width, height)?;
        image.fill(c[0], c[1], c[2], Some(c[3]));
        Ok(image)
    }

    /// The copy instructions of a slide or push (C# `MoveSegments`).
    fn move_segments(&self, width: usize, height: usize, fraction: f32) -> Vec<Segment> {
        let mut segments = Vec::new();
        let push = self.kind == DrawingTransitionKind::Push;
        let mut add = |from_incoming: bool, source_x: usize, source_y: usize, destination_x: usize, destination_y: usize, length: usize| {
            if length > 0 {
                segments.push(Segment { from_incoming, source_x, source_y, destination_x, destination_y, length });
            }
        };
        if self.is_horizontal() {
            let offset = offset(fraction, width);
            for y in 0..height {
                if self.direction == DrawingTransitionDirection::Right {
                    add(true, width - offset, y, 0, y, offset);
                    add(false, if push { 0 } else { offset }, y, offset, y, width - offset);
                } else {
                    add(false, if push { offset } else { 0 }, y, 0, y, width - offset);
                    add(true, 0, y, width - offset, y, offset);
                }
            }
        } else {
            let offset = offset(fraction, height);
            for y in 0..height {
                if self.direction == DrawingTransitionDirection::Down {
                    if y < offset {
                        add(true, 0, y + height - offset, 0, y, width);
                    } else {
                        add(false, 0, if push { y - offset } else { y }, 0, y, width);
                    }
                } else if y >= height - offset {
                    add(true, 0, y - (height - offset), 0, y, width);
                } else {
                    add(false, 0, if push { y + offset } else { y }, 0, y, width);
                }
            }
        }
        segments
    }

    fn move_images(&self, from: &DrawingPlaneImage, to: &DrawingPlaneImage, destination: &mut DrawingPlaneImage, fraction: f32) {
        let width = from.width() as usize;
        for s in self.move_segments(width, from.height() as usize, fraction) {
            let source = if s.from_incoming { to } else { from };
            let si = (s.source_y * width) + s.source_x;
            let di = (s.destination_y * width) + s.destination_x;
            destination.r_mut().data_mut()[di..di + s.length].copy_from_slice(&source.r().data()[si..si + s.length]);
            destination.g_mut().data_mut()[di..di + s.length].copy_from_slice(&source.g().data()[si..si + s.length]);
            destination.b_mut().data_mut()[di..di + s.length].copy_from_slice(&source.b().data()[si..si + s.length]);
            destination.a_mut().data_mut()[di..di + s.length].copy_from_slice(&source.a().data()[si..si + s.length]);
        }
    }
}

/// A copy instruction for slides and pushes: `length` pixels from (source_x, source_y) of the
/// incoming or outgoing frame to (destination_x, destination_y).
struct Segment {
    from_incoming: bool,
    source_x: usize,
    source_y: usize,
    destination_x: usize,
    destination_y: usize,
    length: usize,
}

fn offset(fraction: f32, extent: usize) -> usize {
    let offset = ((fraction * extent as f32) + 0.5).floor() as i64;
    offset.clamp(0, extent as i64) as usize
}

fn clamp_fraction(fraction: f32) -> Result<f32, String> {
    if fraction.is_nan() {
        return Err("A transition's fraction must be a number (0 = outgoing, 1 = incoming).".to_string());
    }
    Ok(if fraction < 0.0 {
        0.0
    } else if fraction > 1.0 {
        1.0
    } else {
        fraction
    })
}

fn check_bytes(from: &[u8], to: &[u8], destination_length: usize, width: u32, height: u32) -> Result<usize, String> {
    validate_size(width, height)?;
    let length = width as usize * height as usize * 4;
    if from.len() < length {
        return Err("The outgoing frame must hold at least width x height x 4 bytes.".to_string());
    }
    if to.len() < length {
        return Err("The incoming frame must hold at least width x height x 4 bytes.".to_string());
    }
    if destination_length < length {
        return Err("The destination must hold at least width x height x 4 bytes.".to_string());
    }
    Ok(length)
}

fn write(destination: &mut [u8], offset: usize, pixel: [u8; 4]) {
    destination[offset..offset + 4].copy_from_slice(&pixel);
}
