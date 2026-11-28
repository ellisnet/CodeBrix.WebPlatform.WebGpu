//! THE TARGET WINDOW (Not in C#): the part of the canvas the renderer's INTERNAL targets cover -
//! every nesting level's 4x colour target and stencil, the mix modes' backdrop copy, the effect
//! textures, a finished layer's height record and the stroke checkpoints. TWO MODES:
//! - CANVAS-SIZED (the default): the window is the whole canvas ([`TargetWindow::canvas`]: origin
//!   0, the canvas's size), every frame - the old behaviour exactly (the shaders' origin is 0 and
//!   every mapping reduces bit for bit to the old one).
//! - CONTENT-SIZED (opt-in: `set_content_sized_targets(true)` on the layer's control handle or the
//!   offscreen renderer): the window is sized to the frame's DRAWING, so a small drawing on a
//!   large canvas costs memory and fill work in proportion to the drawing. Pixels on stroke-set
//!   edges and filtered water samples can then differ slightly (float rounding of the smaller
//!   coordinates) from the canvas-sized rendering; a drawing that fills the canvas gets the
//!   canvas's window and draws exactly as in the default mode.
//!
//! Either way the output (the canvas's view, a picture target's picture, the testing readback)
//! keeps the canvas's size and coordinates. The rest of this text is the content-sized mode.
//!
//! - THE NEED of a frame (build.rs `target_need`): the root content's box (what the final step
//!   puts on the canvas) and, for every effect group, its region grown by one pixel (a finish
//!   reads the height one pixel either side) and by the texels of every effect level its passes
//!   write and read (`level_rect`: one texel round the region at 1, 2 ... 16 canvas pixels per
//!   texel). No pass reads outside it, so nothing outside the window is ever read.
//! - ALIGNMENT: the window's origin is a multiple of [`WINDOW_ALIGN`] (16, the coarsest effect
//!   level), so every effect level's texels sit on the canvas's texel grid as with canvas-sized
//!   targets; its far edges are aligned too, or are the canvas's own edges (where a texture's
//!   edge then is the canvas's edge, exactly as before). The window never reaches past the
//!   canvas.
//! - THE RULE ([`WindowRule::next`]): a frame whose need fits the window keeps it; one that does
//!   not first SLIDES it (the same size, a new origin: nothing reallocated - an element flying
//!   across a large canvas is followed by a target a little larger than itself); when the size
//!   must change it GROWS to the need plus [`WINDOW_SLACK`] on each side and at least half as
//!   large again as before in each direction that overflowed (so a growing drawing reallocates a
//!   few times, not every frame). It SHRINKS (to the need plus the slack) only after
//!   [`SHRINK_FRAMES`] frames in a row whose need is at most a quarter of the window's area (an
//!   empty frame counts as such), so a drawing that is briefly small keeps its targets.
//! - THE MAPPINGS (common.wgsl's View: `size` = the window's width, height and origin, `canvas` =
//!   the output's size): a device point is put on a target at its point minus the origin
//!   (`to_clip`), a fragment's canvas position is its position plus the origin (`canvas_xy`),
//!   an effect level's texel is its canvas texel minus origin / level, and the final step puts
//!   the target back at the origin (`vs_final`, the load at the position minus the origin). The
//!   CPU twins below mirror each of them.

use kurbo::Rect;

/// The window's origin is a multiple of this many device pixels: the coarsest effect level
/// (`effect_sizes::MAX_LEVEL`), so every level's texels line up with the canvas's.
pub(crate) const WINDOW_ALIGN: u32 = 16;
/// Device pixels added on each side of the need when the window is made or grown.
pub(crate) const WINDOW_SLACK: u32 = 32;
/// Frames in a row with a need of at most a quarter of the window before it shrinks.
pub(crate) const SHRINK_FRAMES: u32 = 120;
/// The bytes per pixel of one nesting level's targets: an Rgba8UnormSrgb colour target and a
/// Stencil8 stencil, 4 samples each.
#[cfg(test)]
pub(crate) const LEVEL_BYTES_PER_PIXEL: usize = (4 + 1) * 4;

/// Whole device pixels [x0, x1) x [y0, y1) on the canvas.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct PixelBox {
    pub(crate) x0: u32,
    pub(crate) y0: u32,
    pub(crate) x1: u32,
    pub(crate) y1: u32,
}

impl PixelBox {
    fn width(&self) -> u32 {
        self.x1 - self.x0
    }

    fn height(&self) -> u32 {
        self.y1 - self.y0
    }

    fn area(&self) -> u64 {
        self.width() as u64 * self.height() as u64
    }
}

/// The part of the canvas the internal targets cover: origin and size in device pixels.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct TargetWindow {
    pub(crate) x: u32,
    pub(crate) y: u32,
    pub(crate) width: u32,
    pub(crate) height: u32,
}

impl Default for TargetWindow {
    fn default() -> TargetWindow {
        TargetWindow::canvas(1, 1)
    }
}

impl TargetWindow {
    /// The whole canvas (the old, canvas-sized targets); at least 1 x 1.
    pub(crate) fn canvas(width: u32, height: u32) -> TargetWindow {
        TargetWindow { x: 0, y: 0, width: width.max(1), height: height.max(1) }
    }

    /// The targets' size.
    pub(crate) fn size(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    /// The texels of an effect level's texture over the window (rounded up, as the canvas's).
    pub(crate) fn level_size(&self, level: u32) -> (u32, u32) {
        let level = level.max(1);
        (self.width.div_ceil(level), self.height.div_ceil(level))
    }

    /// The bytes of `levels` nesting levels' targets (colour and stencil, 4 samples).
    #[cfg(test)]
    pub(crate) fn level_bytes(&self, levels: usize) -> usize {
        self.width as usize * self.height as usize * LEVEL_BYTES_PER_PIXEL * levels
    }

    /// The view uniform (common.wgsl's View): the window's size and origin, the output's size.
    pub(crate) fn view_words(&self, canvas: (u32, u32)) -> [f32; 8] {
        [self.width as f32, self.height as f32, self.x as f32, self.y as f32, canvas.0 as f32, canvas.1 as f32, 0.0, 0.0]
    }

    fn contains(&self, b: &PixelBox) -> bool {
        b.x0 >= self.x && b.y0 >= self.y && b.x1 <= self.x + self.width && b.y1 <= self.y + self.height
    }

    fn area(&self) -> u64 {
        self.width as u64 * self.height as u64
    }
}

fn align_down(v: u32) -> u32 {
    v / WINDOW_ALIGN * WINDOW_ALIGN
}

fn align_up(v: u32) -> u32 {
    v.div_ceil(WINDOW_ALIGN) * WINDOW_ALIGN
}

/// A frame's need as whole pixels: the box rounded outward, its origin aligned down and its far
/// edges up, clipped to the canvas; None when nothing of it is on the canvas.
pub(crate) fn need_box(r: Rect, width: u32, height: u32) -> Option<PixelBox> {
    if !(r.width() > 0.0 && r.height() > 0.0) {
        return None;
    }
    let (w, h) = (width.max(1) as f64, height.max(1) as f64);
    let (x0, y0) = (r.x0.floor().clamp(0.0, w), r.y0.floor().clamp(0.0, h));
    let (x1, y1) = (r.x1.ceil().clamp(0.0, w), r.y1.ceil().clamp(0.0, h));
    if x1 <= x0 || y1 <= y0 {
        return None;
    }
    Some(PixelBox { x0: align_down(x0 as u32), y0: align_down(y0 as u32), x1: align_up(x1 as u32).min(width.max(1)), y1: align_up(y1 as u32).min(height.max(1)) })
}

/// One axis of a window: from at most `a` to at least `b`, `size` (aligned up) long where the
/// canvas (`extent`) allows, centred on [a, b), its start aligned, never past the canvas.
fn place(a: u32, b: u32, size: u32, extent: u32) -> (u32, u32) {
    let size = align_up(size).min(extent).max(b - a);
    let extra = size - (b - a);
    let mut x0 = align_down(a.saturating_sub(extra / 2));
    if x0 + size > extent {
        x0 = align_down(extent.saturating_sub(size));
    }
    let x1 = (x0 + size).min(extent).max(b);
    (x0, x1)
}

/// The window holding `need` with (where the canvas allows) the size asked for.
fn fit(need: PixelBox, width: u32, height: u32, canvas: (u32, u32)) -> TargetWindow {
    let (x0, x1) = place(need.x0, need.x1, width, canvas.0);
    let (y0, y1) = place(need.y0, need.y1, height, canvas.1);
    TargetWindow { x: x0, y: y0, width: x1 - x0, height: y1 - y0 }
}

/// The window of a frame with nothing to draw and no window yet: the smallest, at the origin.
fn smallest(canvas: (u32, u32)) -> TargetWindow {
    TargetWindow { x: 0, y: 0, width: WINDOW_ALIGN.min(canvas.0), height: WINDOW_ALIGN.min(canvas.1) }
}

/// The window kept from frame to frame (see the module's text).
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct WindowRule {
    window: Option<TargetWindow>,
    canvas: (u32, u32),
    small_frames: u32,
}

impl WindowRule {
    /// This frame's window for its `need` on a `width` x `height` canvas; `sized` false (the
    /// default mode) gives the canvas, every frame.
    pub(crate) fn next(&mut self, need: Option<PixelBox>, width: u32, height: u32, sized: bool) -> TargetWindow {
        let canvas = (width.max(1), height.max(1));
        if !sized {
            self.small_frames = 0;
            self.canvas = canvas;
            let w = TargetWindow::canvas(width, height);
            self.window = Some(w);
            return w;
        }
        let current = self.window.filter(|w| self.canvas == canvas && w.x + w.width <= canvas.0 && w.y + w.height <= canvas.1);
        self.canvas = canvas;
        let fresh = |n: PixelBox| fit(n, n.width() + 2 * WINDOW_SLACK, n.height() + 2 * WINDOW_SLACK, canvas);
        let window = match (current, need) {
            (None, None) => {
                self.small_frames = 0;
                smallest(canvas)
            }
            (None, Some(n)) => {
                self.small_frames = 0;
                fresh(n)
            }
            (Some(c), n) if n.is_none_or(|n| c.contains(&n)) => {
                // It fits: kept, unless it has been at most a quarter used for long enough.
                let small = n.is_none_or(|n| n.area() * 4 <= c.area());
                self.small_frames = if small { self.small_frames + 1 } else { 0 };
                if self.small_frames >= SHRINK_FRAMES {
                    self.small_frames = 0;
                    n.map_or(smallest(canvas), fresh)
                } else {
                    c
                }
            }
            (Some(c), None) => c,
            (Some(c), Some(n)) => {
                self.small_frames = 0;
                // Slid (same size, nothing reallocated) when it can be; else grown.
                let slid = fit(n, c.width, c.height, canvas);
                if slid.size() == c.size() {
                    slid
                } else {
                    // An axis that overflowed grows to the need and its slack, at least by half;
                    // one more than three quarters used grows by half with it (one reallocation
                    // for a drawing growing both ways, not two).
                    let grow = |need: u32, now: u32| {
                        if need > now {
                            (need + 2 * WINDOW_SLACK).max(now + now / 2)
                        } else if need as u64 * 4 > now as u64 * 3 {
                            now + now / 2
                        } else {
                            now
                        }
                    };
                    fit(n, grow(n.width(), c.width), grow(n.height(), c.height), canvas)
                }
            }
        };
        self.window = Some(window);
        window
    }
}

// ---- The CPU twins of the shaders' mappings (common.wgsl, composite.wgsl, effects.wgsl; tested
// below) ----

/// common.wgsl `to_clip`: a device point (canvas pixels) as a clip position on the targets.
#[cfg(test)]
pub(crate) fn to_clip(p: [f32; 2], view: &[f32; 8]) -> [f32; 2] {
    let q = [p[0] - view[2], p[1] - view[3]];
    [q[0] / view[0] * 2.0 - 1.0, 1.0 - q[1] / view[1] * 2.0]
}

/// common.wgsl `vs_final`: a device point as a clip position on the output (canvas size).
#[cfg(test)]
pub(crate) fn to_output_clip(p: [f32; 2], view: &[f32; 8]) -> [f32; 2] {
    [p[0] / view[4] * 2.0 - 1.0, 1.0 - p[1] / view[5] * 2.0]
}

/// common.wgsl `canvas_xy`: a fragment's position on a target as a canvas position.
#[cfg(test)]
pub(crate) fn canvas_xy(frag: [f32; 2], view: &[f32; 8]) -> [f32; 2] {
    [frag[0] + view[2], frag[1] + view[3]]
}

/// composite.wgsl `fs_final` (and effects.wgsl `fs_picture`): the target texel an output pixel
/// (its fragment position) reads.
#[cfg(test)]
pub(crate) fn final_texel(frag: [f32; 2], view: &[f32; 8]) -> [i32; 2] {
    [(frag[0] - view[2]) as i32, (frag[1] - view[3]) as i32]
}

/// effects.wgsl `vs_fx`: a device point as a clip position on an effect level's texture of
/// `size` texels.
#[cfg(test)]
pub(crate) fn fx_clip(p: [f32; 2], view: &[f32; 8], level: f32, size: [f32; 2]) -> [f32; 2] {
    let q = [(p[0] - view[2]) / level, (p[1] - view[3]) / level];
    [q[0] / size[0] * 2.0 - 1.0, 1.0 - q[1] / size[1] * 2.0]
}

/// effects.wgsl `load_valid`: the canvas texel centre (at the level) a window texel is checked
/// at against the pass's valid texels.
#[cfg(test)]
pub(crate) fn valid_point(texel: [i32; 2], view: &[f32; 8], level: f32) -> [f32; 2] {
    [texel[0] as f32 + view[2] / level + 0.5, texel[1] as f32 + view[3] / level + 0.5]
}

/// effects.wgsl `sample_level`: the texture coordinates a device position samples at a level.
#[cfg(test)]
pub(crate) fn level_uv(device: [f32; 2], view: &[f32; 8], level: f32, size: [f32; 2]) -> [f32; 2] {
    [(device[0] - view[2]) / level / size[0], (device[1] - view[3]) / level / size[1]]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn need(x0: u32, y0: u32, x1: u32, y1: u32) -> Option<PixelBox> {
        Some(PixelBox { x0, y0, x1, y1 })
    }

    #[test]
    fn the_alignment_is_the_coarsest_effect_level() {
        assert_eq!(WINDOW_ALIGN, crate::effects::effect_sizes::MAX_LEVEL);
    }

    #[test]
    fn a_need_is_rounded_outward_aligned_and_clipped_to_the_canvas() {
        assert_eq!(need_box(Rect::new(100.2, 50.7, 163.5, 99.1), 3840, 2160), need(96, 48, 176, 112));
        // Partly off the canvas: clipped; the far edges stop at the canvas's own edges.
        assert_eq!(need_box(Rect::new(-40.0, 2100.0, 30.0, 2300.0), 3840, 2161), need(0, 2096, 32, 2161));
        assert_eq!(need_box(Rect::new(3830.0, 10.0, 3900.0, 20.0), 3845, 2160), need(3824, 0, 3845, 32));
        // Wholly off the canvas, or empty: nothing.
        assert_eq!(need_box(Rect::new(4000.0, 10.0, 4100.0, 20.0), 3840, 2160), None);
        assert_eq!(need_box(Rect::new(-50.0, -50.0, -1.0, -1.0), 3840, 2160), None);
        assert_eq!(need_box(Rect::ZERO, 3840, 2160), None);
    }

    #[test]
    fn a_small_drawing_on_a_large_canvas_gets_a_small_window_and_keeps_it() {
        let mut rule = WindowRule::default();
        // A 64 pixel flourish at (3000, 1800) on a 3840 x 2160 canvas.
        let n = need_box(Rect::new(3000.0, 1800.0, 3064.0, 1864.0), 3840, 2160);
        let w = rule.next(n, 3840, 2160, true);
        assert_eq!(w, TargetWindow { x: 2960, y: 1760, width: 144, height: 144 });
        assert!(w.contains(&n.unwrap()));
        // 20 bytes a pixel a level: 414,720 bytes instead of 165,888,000.
        assert_eq!(w.level_bytes(1), 414_720);
        assert_eq!(TargetWindow::canvas(3840, 2160).level_bytes(1), 165_888_000);
        // Moving a little inside it: the same window, frame after frame.
        for k in 0..300u32 {
            let d = (k % 20) as f64;
            let n = need_box(Rect::new(2990.0 + d, 1795.0, 3054.0 + d, 1859.0), 3840, 2160);
            assert_eq!(rule.next(n, 3840, 2160, true), w, "frame {k}");
        }
    }

    #[test]
    fn a_drawing_flying_across_the_canvas_slides_the_window_without_reallocating() {
        let mut rule = WindowRule::default();
        let mut sizes = Vec::new();
        let mut moves = 0;
        let mut last = None;
        for k in 0..370u32 {
            let x = 10.0 + k as f64 * 10.0;
            let n = need_box(Rect::new(x, 1000.0, x + 64.0, 1064.0), 3840, 2160);
            let w = rule.next(n, 3840, 2160, true);
            assert!(w.contains(&n.unwrap()), "frame {k}");
            assert!(w.x + w.width <= 3840 && w.y + w.height <= 2160);
            if last.is_some_and(|l: TargetWindow| l != w) {
                moves += 1;
            }
            if !sizes.contains(&w.size()) {
                sizes.push(w.size());
            }
            last = Some(w);
        }
        assert_eq!(sizes.len(), 1, "never reallocated: {sizes:?}");
        assert!(moves > 10 && moves < 370 / 2, "slid now and then ({moves}), not every frame");
    }

    #[test]
    fn a_growing_drawing_reallocates_a_few_times_and_a_small_one_shrinks_only_after_a_while() {
        let mut rule = WindowRule::default();
        let mut sizes = Vec::new();
        for k in 0..500u32 {
            let half = 32.0 + k as f64 * 4.0;
            let n = need_box(Rect::new(1920.0 - half, 1080.0 - half.min(1080.0), 1920.0 + half, 1080.0 + half.min(1080.0)), 3840, 2160);
            let w = rule.next(n, 3840, 2160, true);
            assert!(w.contains(&n.unwrap()));
            if sizes.last() != Some(&w.size()) {
                sizes.push(w.size());
            }
        }
        assert!(sizes.len() <= 10, "a few reallocations, not one per frame: {sizes:?}");
        assert_eq!(*sizes.last().unwrap(), (3840, 2160));
        // Back to a small drawing: the large window stays for SHRINK_FRAMES - 1 frames ...
        let n = need_box(Rect::new(100.0, 100.0, 164.0, 164.0), 3840, 2160);
        for k in 1..SHRINK_FRAMES {
            assert_eq!(rule.next(n, 3840, 2160, true).size(), (3840, 2160), "frame {k}");
        }
        // ... then shrinks to the drawing and its slack.
        let w = rule.next(n, 3840, 2160, true);
        assert_eq!(w, TargetWindow { x: 64, y: 64, width: 144, height: 144 });
        // A drawing using more than a quarter of the window keeps it however long it lasts.
        let mut rule = WindowRule::default();
        let big = need_box(Rect::new(0.0, 0.0, 1000.0, 1000.0), 3840, 2160);
        let w = rule.next(big, 3840, 2160, true);
        let most = need_box(Rect::new(0.0, 0.0, 600.0, 600.0), 3840, 2160);
        for _ in 0..1000 {
            assert_eq!(rule.next(most, 3840, 2160, true), w);
        }
    }

    #[test]
    fn empty_frames_keep_the_window_then_shrink_to_the_smallest() {
        let mut rule = WindowRule::default();
        assert_eq!(rule.next(None, 3840, 2160, true), TargetWindow { x: 0, y: 0, width: 16, height: 16 });
        let n = need_box(Rect::new(500.0, 500.0, 900.0, 900.0), 3840, 2160);
        let w = rule.next(n, 3840, 2160, true);
        for _ in 1..SHRINK_FRAMES {
            assert_eq!(rule.next(None, 3840, 2160, true), w);
        }
        assert_eq!(rule.next(None, 3840, 2160, true), TargetWindow { x: 0, y: 0, width: 16, height: 16 });
        // A tiny canvas: never larger than it, never zero.
        let mut rule = WindowRule::default();
        assert_eq!(rule.next(None, 0, 0, true), TargetWindow { x: 0, y: 0, width: 1, height: 1 });
        assert_eq!(rule.next(None, 5, 3, true).size(), (5, 3));
    }

    #[test]
    fn a_window_never_reaches_past_the_canvas_and_a_new_canvas_starts_again() {
        let mut rule = WindowRule::default();
        // At the far corner of a canvas whose size is not a multiple of the alignment.
        let n = need_box(Rect::new(1270.0, 710.0, 1283.0, 721.0), 1283, 721);
        let w = rule.next(n, 1283, 721, true);
        assert_eq!((w.x + w.width, w.y + w.height), (1283, 721));
        assert_eq!((w.x % WINDOW_ALIGN, w.y % WINDOW_ALIGN), (0, 0));
        assert!(w.contains(&n.unwrap()));
        // The canvas shrinks under the window: a new one inside it.
        let n = need_box(Rect::new(10.0, 10.0, 20.0, 20.0), 640, 360);
        let w = rule.next(n, 640, 360, true);
        assert!(w.x + w.width <= 640 && w.y + w.height <= 360 && w.contains(&n.unwrap()));
    }

    #[test]
    fn the_default_mode_keeps_canvas_sized_targets() {
        let mut rule = WindowRule::default();
        let n = need_box(Rect::new(3000.0, 1800.0, 3064.0, 1864.0), 3840, 2160);
        assert_eq!(rule.next(n, 3840, 2160, false), TargetWindow { x: 0, y: 0, width: 3840, height: 2160 });
        assert_eq!(rule.next(None, 3840, 2160, false), TargetWindow::canvas(3840, 2160));
    }

    #[test]
    fn a_canvas_window_maps_exactly_as_the_old_shaders_did() {
        // Every mapping with origin 0 and the canvas's size is bit for bit the old formula.
        let (w, h) = (1283u32, 721u32);
        let view = TargetWindow::canvas(w, h).view_words((w, h));
        for &(x, y) in &[(0.0f32, 0.0f32), (0.37, 719.99), (640.5, 360.25), (1283.0, 721.0), (-3.7, 900.1)] {
            let old = [x / w as f32 * 2.0 - 1.0, 1.0 - y / h as f32 * 2.0];
            assert_eq!(to_clip([x, y], &view), old);
            assert_eq!(to_output_clip([x, y], &view), old);
            assert_eq!(canvas_xy([x, y], &view), [x, y]);
            for level in [1u32, 2, 4, 8, 16] {
                let size = [w.div_ceil(level) as f32, h.div_ceil(level) as f32];
                let l = level as f32;
                let q = [x / l, y / l];
                assert_eq!(fx_clip([x, y], &view, l, size), [q[0] / size[0] * 2.0 - 1.0, 1.0 - q[1] / size[1] * 2.0]);
                assert_eq!(level_uv([x, y], &view, l, size), [x / l / size[0], y / l / size[1]]);
            }
        }
        assert_eq!(final_texel([10.5, 20.5], &view), [10, 20]);
        assert_eq!(valid_point([7, 9], &view, 4.0), [7.5, 9.5]);
    }

    #[test]
    fn a_window_puts_every_canvas_pixel_at_the_same_place_less_its_origin() {
        let (cw, ch) = (3840u32, 2160u32);
        let win = TargetWindow { x: 2960, y: 1760, width: 144, height: 144 };
        let view = win.view_words((cw, ch));
        // A device point lands on the target at the point less the origin (the viewport's
        // transform of the clip position), within float rounding.
        for &(x, y) in &[(2960.0f32, 1760.0f32), (3000.25, 1800.75), (3103.9, 1903.5)] {
            let c = to_clip([x, y], &view);
            let fb = [(c[0] + 1.0) * 0.5 * win.width as f32, (1.0 - c[1]) * 0.5 * win.height as f32];
            assert!((fb[0] - (x - 2960.0)).abs() < 1e-2 && (fb[1] - (y - 1760.0)).abs() < 1e-2, "{fb:?}");
            let o = to_output_clip([x, y], &view);
            let out = [(o[0] + 1.0) * 0.5 * cw as f32, (1.0 - o[1]) * 0.5 * ch as f32];
            assert!((out[0] - x).abs() < 1e-2 && (out[1] - y).abs() < 1e-2, "{out:?}");
        }
        // A fragment's canvas position: exact (pixel centres plus a whole origin).
        assert_eq!(canvas_xy([40.5, 3.5], &view), [3000.5, 1763.5]);
        // The output pixel (3000, 1763) reads the target's texel (40, 3).
        assert_eq!(final_texel([3000.5, 1763.5], &view), [40, 3]);
        // Effect levels: the window's texel j at level L is the canvas's texel j + origin / L, so
        // the valid-texel check and the samples see the canvas's texels exactly.
        for level in [1u32, 2, 4, 8, 16] {
            let l = level as f32;
            let shift = [2960 / level, 1760 / level];
            assert_eq!(2960 % level + 1760 % level, 0);
            assert_eq!(valid_point([3, 5], &view, l), [(3 + shift[0]) as f32 + 0.5, (5 + shift[1]) as f32 + 0.5]);
            let (tw, th) = win.level_size(level);
            let uv = level_uv([3001.0, 1801.0], &view, l, [tw as f32, th as f32]);
            let texel = [uv[0] * tw as f32 + shift[0] as f32, uv[1] * th as f32 + shift[1] as f32];
            assert!((texel[0] - 3001.0 / l).abs() < 1e-3 && (texel[1] - 1801.0 / l).abs() < 1e-3);
        }
        // A level's texture over a window at the canvas's far edge ends where the canvas's does.
        let edge = TargetWindow { x: 1264, y: 704, width: 19, height: 17 };
        for level in [1u32, 2, 4, 8, 16] {
            let (tw, th) = edge.level_size(level);
            assert_eq!((tw + 1264 / level, th + 704 / level), (1283u32.div_ceil(level), 721u32.div_ceil(level)));
        }
    }
}
