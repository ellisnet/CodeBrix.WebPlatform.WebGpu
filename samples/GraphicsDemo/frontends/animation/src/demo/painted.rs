//! What the painterly feature scenes share: the picture they paint (a field of iris-like flowers
//! under a sky, drawn into a `DrawingPlaneImage` once, in code - no artwork; a second picture, a
//! shore at dusk, for the stroke transition; a bank at dusk for the reflection scene; and a pond
//! with lily pads and irises, with its floating parts apart, for the painted pond), the colour under a
//! point of it, brush tips made in code (bristle ridges) and a seeded jittered grid of stroke
//! records that take the picture's colour under their centres.

use codebrix_webgpu_animation::planes::{DrawingPlane, DrawingPlaneImage, DrawingRandom};
use codebrix_webgpu_animation::strokes::{BrushStrokeRecord, BrushTip, BrushTips};
use codebrix_webgpu_animation::{Color, PointF, RectangleF};

/// The picture's size in pixels.
pub const PICTURE_SIZE: (u32, u32) = (480, 270);

fn mix(a: [f32; 3], b: [f32; 3], t: f32) -> [f32; 3] {
    let t = t.clamp(0.0, 1.0);
    [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t, a[2] + (b[2] - a[2]) * t]
}

fn over(image: &mut DrawingPlaneImage, x: i32, y: i32, color: [f32; 3], coverage: f32) {
    let (w, h) = image.size();
    if x < 0 || y < 0 || x >= w as i32 || y >= h as i32 || coverage <= 0.0 {
        return;
    }
    let c = coverage.min(1.0);
    let old = image.get_pixel(x as u32, y as u32).unwrap();
    let _ = image.set_pixel(x as u32, y as u32, [old[0] + (color[0] - old[0]) * c, old[1] + (color[1] - old[1]) * c, old[2] + (color[2] - old[2]) * c, 1.0]);
}

thread_local! {
    static PICTURE: std::cell::RefCell<Option<DrawingPlaneImage>> = const { std::cell::RefCell::new(None) };
}

/// The picture of [`iris_field`], made once per page and shared by the scenes.
pub fn picture() -> DrawingPlaneImage {
    PICTURE.with(|p| p.borrow_mut().get_or_insert_with(iris_field).clone())
}

/// A filled ellipse turned by `angle` (radians), with a one-pixel soft edge.
fn ellipse(image: &mut DrawingPlaneImage, cx: f32, cy: f32, rx: f32, ry: f32, angle: f32, color: [f32; 3]) {
    let r = rx.max(ry) + 1.0;
    let (s, c) = angle.sin_cos();
    for y in (cy - r).floor() as i32..=(cy + r).ceil() as i32 {
        for x in (cx - r).floor() as i32..=(cx + r).ceil() as i32 {
            let (dx, dy) = (x as f32 + 0.5 - cx, y as f32 + 0.5 - cy);
            let (u, v) = (dx * c + dy * s, -dx * s + dy * c);
            let d = ((u / rx).powi(2) + (v / ry).powi(2)).sqrt();
            let edge = (1.0 - d) * rx.min(ry);
            over(image, x, y, color, edge + 0.5);
        }
    }
}

/// A line of a given width from a to b (a stem, a blade).
fn segment(image: &mut DrawingPlaneImage, a: (f32, f32), b: (f32, f32), width: f32, color: [f32; 3]) {
    let (minx, maxx) = (a.0.min(b.0) - width, a.0.max(b.0) + width);
    let (miny, maxy) = (a.1.min(b.1) - width, a.1.max(b.1) + width);
    let (ux, uy) = (b.0 - a.0, b.1 - a.1);
    let len2 = (ux * ux + uy * uy).max(1e-6);
    for y in miny.floor() as i32..=maxy.ceil() as i32 {
        for x in minx.floor() as i32..=maxx.ceil() as i32 {
            let (px, py) = (x as f32 + 0.5 - a.0, y as f32 + 0.5 - a.1);
            let t = ((px * ux + py * uy) / len2).clamp(0.0, 1.0);
            let (qx, qy) = (px - t * ux, py - t * uy);
            let d = (qx * qx + qy * qy).sqrt();
            // Blades taper towards their far end.
            let half = width * 0.5 * (1.0 - 0.7 * t);
            over(image, x, y, color, half - d + 0.5);
        }
    }
}

/// THE PICTURE: a sky (deep blue to a pale horizon, a low warm sun), a meadow darkening towards
/// the front, and about sixty iris-like flowers - green blades and stems, three purple / blue
/// petals and a yellow beard each - larger and lower the nearer they are. Seeded: the same every
/// time.
pub fn iris_field() -> DrawingPlaneImage {
    let (w, h) = PICTURE_SIZE;
    let horizon = h as f32 * 0.42;
    let mut image = DrawingPlaneImage::new(w, h).unwrap();
    image.fill_with(|x, y| {
        let (u, v) = (x as f32 / w as f32, y as f32);
        let c = if v < horizon {
            let t = v / horizon;
            let sky = mix([0.16, 0.30, 0.68], [0.86, 0.80, 0.74], t * t);
            let sun = (1.0 - (((u - 0.78) * 1.6).powi(2) + ((t - 0.62) * 0.9).powi(2)).sqrt() / 0.08).clamp(0.0, 1.0);
            mix(sky, [1.0, 0.86, 0.55], sun)
        } else {
            let t = (v - horizon) / (h as f32 - horizon);
            let ripple = 0.5 + 0.5 * ((u * 31.0 + t * 7.0).sin() * (t * 23.0).cos());
            mix(mix([0.42, 0.56, 0.30], [0.12, 0.30, 0.12], t), [0.30, 0.48, 0.20], ripple * 0.35)
        };
        [c[0], c[1], c[2], 1.0]
    });
    let mut random = DrawingRandom::new(31);
    let mut flowers: Vec<(f32, f32, f32)> = (0..60)
        .map(|_| {
            let depth = random.next_single();
            let y = horizon + 14.0 + depth * (h as f32 - horizon - 20.0);
            (random.uniform(0.0, w as f64) as f32, y, 0.45 + depth * 1.1)
        })
        .collect();
    flowers.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());
    for (x, y, s) in flowers {
        let stem_top = (x, y - 26.0 * s);
        for k in 0..3 {
            let lean = (k as f32 - 1.0) * 9.0 * s + random.uniform(-3.0, 3.0) as f32;
            segment(&mut image, (x + (k as f32 - 1.0) * 2.0, y), (x + lean, y - random.uniform(18.0, 30.0) as f32 * s), 3.2 * s, [0.18, 0.46, 0.20]);
        }
        segment(&mut image, (x, y), stem_top, 1.6 * s, [0.22, 0.50, 0.22]);
        let hue = random.next_single();
        let petal = mix([0.42, 0.22, 0.70], [0.28, 0.34, 0.86], hue);
        let dark = mix(petal, [0.12, 0.06, 0.26], 0.45);
        let (hx, hy) = (stem_top.0, stem_top.1 - 3.0 * s);
        ellipse(&mut image, hx - 4.5 * s, hy + 2.5 * s, 4.0 * s, 2.2 * s, 0.6, dark);
        ellipse(&mut image, hx + 4.5 * s, hy + 2.5 * s, 4.0 * s, 2.2 * s, -0.6, dark);
        ellipse(&mut image, hx, hy - 2.0 * s, 2.6 * s, 5.0 * s, 0.0, petal);
        ellipse(&mut image, hx, hy + 1.0 * s, 1.2 * s, 1.0 * s, 0.0, [0.98, 0.84, 0.30]);
    }
    image
}

thread_local! {
    static DUSK: std::cell::RefCell<Option<DrawingPlaneImage>> = const { std::cell::RefCell::new(None) };
}

/// The picture of [`dusk_shore`], made once per page (the second painting of the
/// "stroke-transition" scene).
pub fn dusk_picture() -> DrawingPlaneImage {
    DUSK.with(|p| p.borrow_mut().get_or_insert_with(dusk_shore).clone())
}

/// A SECOND PICTURE, of another place and hour (the iris field's size, made in code, seeded): a
/// dusk sky from deep violet to orange, a large low sun, a dark sea with the sun's broken
/// reflection, and a dark headland on the left with a row of slender trees. Its horizon and
/// masses lie elsewhere than the iris field's, so a transition between the two really changes
/// the picture.
pub fn dusk_shore() -> DrawingPlaneImage {
    let (w, h) = PICTURE_SIZE;
    let horizon = h as f32 * 0.58;
    let mut image = DrawingPlaneImage::new(w, h).unwrap();
    image.fill_with(|x, y| {
        let (u, v) = (x as f32 / w as f32, y as f32);
        let c = if v < horizon {
            let t = v / horizon;
            let sky = mix(mix([0.20, 0.10, 0.34], [0.62, 0.26, 0.40], t), [0.98, 0.58, 0.26], (t - 0.55).max(0.0) / 0.45);
            let sun = (1.0 - (((u - 0.62) * 1.778).powi(2) + (t - 0.86).powi(2)).sqrt() / 0.13).clamp(0.0, 1.0);
            mix(sky, [1.0, 0.86, 0.52], (sun * 3.0).min(1.0))
        } else {
            let t = (v - horizon) / (h as f32 - horizon);
            let sea = mix([0.10, 0.12, 0.26], [0.03, 0.05, 0.14], t);
            // The sun's road: broken horizontal glints under it.
            let road = (1.0 - ((u - 0.62) / (0.05 + 0.12 * t)).abs()).clamp(0.0, 1.0);
            let broken = (0.5 + 0.5 * (v * 1.9).sin() * (u * 57.0 + v * 0.7).cos()).powi(3);
            mix(sea, [0.98, 0.62, 0.30], road * broken)
        };
        [c[0], c[1], c[2], 1.0]
    });
    // The headland: a dark hill on the left reaching into the sea.
    let headland = |u: f32| horizon + 6.0 - (1.0 - (u / 0.42).min(1.0)).powf(0.6) * 70.0;
    for y in 0..h {
        for x in 0..w {
            let u = x as f32 / w as f32;
            let top = headland(u);
            let cover = (y as f32 - top + 0.5).clamp(0.0, 1.0);
            if u < 0.45 {
                over(&mut image, x as i32, y as i32, mix([0.10, 0.07, 0.14], [0.05, 0.04, 0.08], (y as f32 - top) / 80.0), cover);
            }
        }
    }
    // Slender trees on the headland.
    let mut random = DrawingRandom::new(57);
    for i in 0..9 {
        let u = 0.04 + i as f32 * 0.038 + random.uniform(-0.01, 0.01) as f32;
        let base = headland(u) + 2.0;
        let height = 40.0 + random.uniform(0.0, 30.0) as f32;
        ellipse(&mut image, u * w as f32, base - height * 0.5, 4.0 + random.uniform(0.0, 2.0) as f32, height * 0.5, 0.0, [0.04, 0.05, 0.06]);
    }
    image
}

/// The picture's colour (nearest pixel) under a point of `area`, the rectangle the picture is
/// laid over.
pub fn pick(image: &DrawingPlaneImage, area: RectangleF, x: f32, y: f32) -> Color {
    let (w, h) = image.size();
    let px = (((x - area.x) / area.width) * w as f32).floor().clamp(0.0, w as f32 - 1.0) as u32;
    let py = (((y - area.y) / area.height) * h as f32).floor().clamp(0.0, h as f32 - 1.0) as u32;
    let p = image.get_pixel(px, py).unwrap();
    let byte = |v: f32| (v.clamp(0.0, 1.0) * 255.0).round() as u8;
    Color::from_rgb(byte(p[0]), byte(p[1]), byte(p[2]))
}

/// Brush tips made in code: bristle ridges across the stroke (a soft edge times a ripple whose
/// phase and count differ per tip), fading a little towards the tail. Four tips of 64 x 16.
pub fn bristle_tips() -> BrushTips {
    make_bristle_tips(false)
}

/// The same bristle tips with a HEIGHT plane each (for a surface finish): the paint stands highest
/// on the ridges and at the head, where the brush set it down, and thins out towards the edges and
/// the tail.
pub fn bristle_tips_with_height() -> BrushTips {
    make_bristle_tips(true)
}

fn make_bristle_tips(with_height: bool) -> BrushTips {
    let (w, h) = (64u32, 16u32);
    let tips: Vec<BrushTip> = (0..4)
        .map(|k| {
            let ridges = 7.0 + 3.0 * k as f32;
            let mut data = Vec::with_capacity((w * h) as usize);
            let mut heights = Vec::with_capacity((w * h) as usize);
            for y in 0..h {
                for x in 0..w {
                    let across = (y as f32 + 0.5) / h as f32 * 2.0 - 1.0;
                    let along = (x as f32 + 0.5) / w as f32;
                    let edge = (1.0 - across * across).max(0.0).sqrt();
                    let ripple = 0.7 + 0.3 * (across * ridges + k as f32 * 1.7 + along * 2.0).sin();
                    let dry = 1.0 - 0.35 * along * (0.5 + 0.5 * (across * 23.0 + k as f32).sin());
                    data.push((edge * ripple * dry).clamp(0.0, 1.0));
                    // Ridges stand up, the head is thickest, the edges and the tail thin out.
                    let ridge = 0.45 + 0.55 * (0.5 + 0.5 * (across * ridges + k as f32 * 1.7 + along * 2.0).sin());
                    heights.push((edge.powf(0.7) * ridge * (1.0 - 0.6 * along)).clamp(0.0, 1.0));
                }
            }
            let height = if with_height { Some(DrawingPlane::from_data(w, h, heights).unwrap()) } else { None };
            BrushTip::new(DrawingPlane::from_data(w, h, data).unwrap(), height)
        })
        .collect();
    BrushTips::new((w, h), &tips).unwrap()
}

/// `count` stroke records on a seeded jittered grid over `area`, each with the picture's colour
/// under its centre, `angle(x, y)` degrees, a slight bend, one of the four tips (1-based) and a
/// seed of its own; times ALWAYS (whole) - the caller sets start / end.
pub fn jittered_strokes(image: &DrawingPlaneImage, area: RectangleF, count: usize, seed: i64, length: f32, width: f32, angle: &dyn Fn(f32, f32) -> f32) -> Vec<BrushStrokeRecord> {
    let cols = ((count as f32 * area.width / area.height).sqrt()).ceil().max(1.0) as usize;
    let rows = count.div_ceil(cols);
    let (cw, ch) = (area.width / cols as f32, area.height / rows as f32);
    let mut random = DrawingRandom::new(seed);
    let mut records = Vec::with_capacity(count);
    for i in 0..count {
        let (c, r) = (i % cols, i / cols);
        let x = area.x + (c as f32 + random.next_single()) * cw;
        let y = area.y + (r as f32 + random.next_single()) * ch;
        let scale = 0.75 + 0.5 * random.next_single();
        let mut record = BrushStrokeRecord::new(PointF::new(x, y), length * scale, width * scale, angle(x, y) + random.uniform(-12.0, 12.0) as f32, pick(image, area, x, y));
        record.bend = random.uniform(-0.12, 0.12) as f32;
        record.tip = 1 + (i % 4) as u16;
        record.load = 0.9;
        record.seed = i as u32;
        records.push(record);
    }
    records
}

/// Fills the rectangle `x0..x1 x y0..y1` (pixels, half-open) with a colour.
fn fill_box(image: &mut DrawingPlaneImage, x0: f32, y0: f32, x1: f32, y1: f32, color: [f32; 3]) {
    for y in y0.floor() as i32..y1.ceil() as i32 {
        for x in x0.floor() as i32..x1.ceil() as i32 {
            let cover = ((x as f32 + 1.0).min(x1) - (x as f32).max(x0)).clamp(0.0, 1.0) * ((y as f32 + 1.0).min(y1) - (y as f32).max(y0)).clamp(0.0, 1.0);
            over(image, x, y, color, cover);
        }
    }
}

/// The size of [`dusk_bank`] in pixels (a wide strip: the bank above a waterline).
pub const BANK_SIZE: (u32, u32) = (608, 150);

thread_local! {
    static BANK: std::cell::RefCell<Option<DrawingPlaneImage>> = const { std::cell::RefCell::new(None) };
}

/// The picture of [`dusk_bank`], made once per page (the "water-reflection" scene).
pub fn bank_picture() -> DrawingPlaneImage {
    BANK.with(|p| p.borrow_mut().get_or_insert_with(dusk_bank).clone())
}

/// A BANK AT DUSK, for a reflection (made in code, seeded, opaque; its bottom row is the shore at
/// the waterline): a sky from deep violet to orange, a low sun, a far line of hills, a row of
/// trees on the bank and a few houses with lit windows among them.
pub fn dusk_bank() -> DrawingPlaneImage {
    let (w, h) = BANK_SIZE;
    let (wf, hf) = (w as f32, h as f32);
    let mut image = DrawingPlaneImage::new(w, h).unwrap();
    image.fill_with(|x, y| {
        let (u, t) = (x as f32 / wf, y as f32 / hf);
        let sky = mix(mix([0.18, 0.10, 0.32], [0.58, 0.24, 0.42], t * 1.3), [1.0, 0.62, 0.28], ((t - 0.45) / 0.55).max(0.0));
        let sun = (1.0 - (((u - 0.70) * 4.05).powi(2) + (t - 0.62).powi(2)).sqrt() / 0.16).clamp(0.0, 1.0);
        let c = mix(sky, [1.0, 0.88, 0.56], (sun * 2.5).min(1.0));
        [c[0], c[1], c[2], 1.0]
    });
    // The far hills, a soft purple line.
    for x in 0..w {
        let u = x as f32 / wf;
        let top = hf * (0.62 - 0.08 * (u * 7.0 + 0.6).sin() - 0.05 * (u * 17.0).sin());
        fill_box(&mut image, x as f32, top, x as f32 + 1.0, hf, [0.30, 0.16, 0.30]);
    }
    // The bank itself: a dark strip along the bottom.
    for x in 0..w {
        let u = x as f32 / wf;
        let top = hf * (0.86 - 0.03 * (u * 11.0).sin());
        fill_box(&mut image, x as f32, top, x as f32 + 1.0, hf, [0.08, 0.06, 0.10]);
    }
    let mut random = DrawingRandom::new(83);
    // Houses: dark boxes with a pitched roof and lit windows.
    for i in 0..4 {
        let cx = wf * (0.12 + 0.23 * i as f32) + random.uniform(-14.0, 14.0) as f32;
        let (bw, bh) = (22.0 + random.uniform(0.0, 14.0) as f32, 16.0 + random.uniform(0.0, 10.0) as f32);
        let base = hf * 0.88;
        fill_box(&mut image, cx - bw * 0.5, base - bh, cx + bw * 0.5, base, [0.10, 0.07, 0.11]);
        for k in 0..((bw / 2.0) as i32) {
            let half = bw * 0.55 * (1.0 - k as f32 / (bw / 2.0));
            fill_box(&mut image, cx - half, base - bh - k as f32 - 1.0, cx + half, base - bh - k as f32, [0.10, 0.07, 0.11]);
        }
        for j in 0..2 {
            let wx = cx - bw * 0.25 + j as f32 * bw * 0.35;
            fill_box(&mut image, wx, base - bh * 0.7, wx + 3.0, base - bh * 0.7 + 4.0, [1.0, 0.80, 0.40]);
        }
    }
    // Trees on the bank: round crowns on thin trunks.
    for i in 0..22 {
        let x = random.uniform(0.0, wf as f64) as f32;
        let height = 26.0 + random.uniform(0.0, 34.0) as f32;
        let base = hf * 0.88;
        segment(&mut image, (x, base), (x, base - height * 0.6), 2.2, [0.06, 0.05, 0.07]);
        let crown = mix([0.05, 0.07, 0.06], [0.14, 0.12, 0.16], random.next_single());
        ellipse(&mut image, x, base - height * 0.65, 6.0 + random.uniform(0.0, 7.0) as f32, height * 0.38, 0.0, crown);
        if i % 3 == 0 {
            ellipse(&mut image, x + 4.0, base - height * 0.8, 5.0, height * 0.25, 0.3, crown);
        }
    }
    image
}

/// Where the pond's waterline lies in [`pond_picture`], as a fraction of its height.
pub const POND_LINE: f32 = 0.46;

/// The pond's water colour (made in code) at a point below the waterline: `u` across 0..1, `depth`
/// 0 at the waterline to 1 at the picture's bottom - the sky's pale tone far away, deep green
/// near, with a faint horizontal play of light.
pub fn pond_water_color(u: f32, depth: f32) -> [f32; 3] {
    let d = depth.clamp(0.0, 1.0);
    let far = mix([0.66, 0.74, 0.82], [0.52, 0.64, 0.70], u);
    let base = mix(far, [0.10, 0.24, 0.24], d.powf(0.7));
    let play = 0.5 + 0.5 * (u * 37.0 + d * 11.0).sin() * (d * 53.0 + u * 3.0).cos();
    mix(base, [0.30, 0.46, 0.46], play * 0.18 * d)
}

/// One lily pad of the pond (fractions of the picture): its centre, its half width (its half
/// height is a third of it - the pond is seen at a low angle), the notch's direction (radians)
/// and its flower, if any.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LilyPad {
    pub x: f32,
    pub y: f32,
    pub radius: f32,
    pub notch: f32,
    pub flower: Option<[f32; 3]>,
}

/// The pond's lily pads, seeded: smaller and higher the farther they float.
pub fn lily_pads() -> Vec<LilyPad> {
    let mut random = DrawingRandom::new(61);
    let mut pads = Vec::new();
    for i in 0..26 {
        let depth = random.next_single();
        let y = POND_LINE + 0.06 + depth * 0.36;
        let x = 0.12 + random.next_single() * 0.76;
        let flower = (i % 3 == 0).then_some(if i % 2 == 0 { [0.98, 0.72, 0.80] } else { [0.98, 0.96, 0.92] });
        pads.push(LilyPad { x, y, radius: 0.012 + depth * 0.028, notch: random.uniform(0.0, std::f64::consts::TAU) as f32, flower });
    }
    pads.sort_by(|a, b| a.y.partial_cmp(&b.y).unwrap());
    pads
}

/// Whether a point of the pond picture (pixels) lies on a lily pad: the pad (or None).
pub fn pad_at(pads: &[LilyPad], px: f32, py: f32) -> Option<LilyPad> {
    let (w, h) = PICTURE_SIZE;
    pads.iter().rev().copied().find(|p| {
        let (dx, dy) = ((px / w as f32 - p.x) * w as f32, (py / h as f32 - p.y) * h as f32);
        let r = p.radius * w as f32;
        let inside = (dx / r).powi(2) + (dy / (r / 3.0)).powi(2) < 1.0;
        let angle = (dy * 3.0).atan2(dx);
        let notch = ((angle - p.notch + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU) - std::f32::consts::PI).abs() < 0.22;
        inside && !notch
    })
}

thread_local! {
    static POND: std::cell::RefCell<Option<(DrawingPlaneImage, DrawingPlaneImage)>> = const { std::cell::RefCell::new(None) };
}

/// The pond pictures (made once per page): the whole picture, and its FLOATING parts alone (the
/// lily pads, their flowers and the near bank with its irises) on a clear ground (alpha = cover).
pub fn pond_pictures() -> (DrawingPlaneImage, DrawingPlaneImage) {
    POND.with(|p| p.borrow_mut().get_or_insert_with(pond_scene).clone())
}

/// A POND (made in code, seeded, the iris field's size): a soft sky, a far bank of trees with a
/// willow and a red boathouse, still water below the waterline (`pond_water_color`), lily pads
/// with pink and white flowers on it, and a near bank with irises at both lower corners.
fn pond_scene() -> (DrawingPlaneImage, DrawingPlaneImage) {
    let (w, h) = PICTURE_SIZE;
    let (wf, hf) = (w as f32, h as f32);
    let line = hf * POND_LINE;
    let mut image = DrawingPlaneImage::new(w, h).unwrap();
    image.fill_with(|x, y| {
        let (u, v) = (x as f32 / wf, y as f32);
        let c = if v < line {
            let t = v / line;
            let sky = mix([0.42, 0.60, 0.86], [0.90, 0.88, 0.82], t * t);
            let cloud = (1.0 - (((u - 0.30) * 3.0).powi(2) + ((t - 0.25) * 9.0).powi(2)).sqrt()).clamp(0.0, 1.0) + (1.0 - (((u - 0.66) * 4.0).powi(2) + ((t - 0.15) * 11.0).powi(2)).sqrt()).clamp(0.0, 1.0);
            mix(sky, [0.97, 0.96, 0.94], cloud.min(1.0) * 0.8)
        } else {
            pond_water_color(u, (v - line) / (hf - line))
        };
        [c[0], c[1], c[2], 1.0]
    });
    let mut random = DrawingRandom::new(47);
    // The far bank: masses of trees standing on a grassy strip.
    for i in 0..40 {
        let x = random.uniform(-10.0, wf as f64 + 10.0) as f32;
        let height = 26.0 + random.uniform(0.0, 40.0) as f32;
        let green = mix([0.12, 0.28, 0.14], [0.36, 0.50, 0.24], random.next_single());
        ellipse(&mut image, x, line - height * 0.5, 9.0 + random.uniform(0.0, 12.0) as f32, height * 0.5, random.uniform(-0.2, 0.2) as f32, green);
        if i % 4 == 0 {
            ellipse(&mut image, x + 6.0, line - height * 0.72, 7.0, height * 0.3, 0.2, mix(green, [0.55, 0.65, 0.35], 0.3));
        }
    }
    // A willow on the left: a pale crown with drooping strands.
    let (wx, wy) = (wf * 0.18, line - 46.0);
    ellipse(&mut image, wx, wy, 34.0, 24.0, 0.0, [0.48, 0.60, 0.30]);
    for k in 0..18 {
        let sx = wx - 32.0 + k as f32 * 3.8;
        segment(&mut image, (sx, wy - 4.0), (sx + random.uniform(-3.0, 3.0) as f32, line - 2.0), 1.6, mix([0.40, 0.54, 0.24], [0.62, 0.72, 0.38], random.next_single()));
    }
    segment(&mut image, (wx, line), (wx + 2.0, wy + 8.0), 4.0, [0.22, 0.16, 0.10]);
    // A red boathouse on the right, at the water's edge.
    let (bx, bw, bh) = (wf * 0.74, 34.0, 18.0);
    fill_box(&mut image, bx - bw * 0.5, line - bh, bx + bw * 0.5, line, [0.62, 0.22, 0.16]);
    for k in 0..12 {
        let half = bw * 0.62 * (1.0 - k as f32 / 12.0);
        fill_box(&mut image, bx - half, line - bh - k as f32 - 1.0, bx + half, line - bh - k as f32, [0.24, 0.16, 0.14]);
    }
    fill_box(&mut image, bx - 5.0, line - 12.0, bx + 5.0, line, [0.10, 0.07, 0.06]);
    // The grassy edge of the far bank.
    fill_box(&mut image, 0.0, line - 3.0, wf, line, [0.26, 0.40, 0.18]);
    // THE FLOATING PARTS on a clear ground.
    let mut floating = DrawingPlaneImage::new(w, h).unwrap();
    floating.fill_with(|_, _| [0.0, 0.0, 0.0, 0.0]);
    let pads = lily_pads();
    for y in 0..h {
        for x in 0..w {
            if let Some(pad) = pad_at(&pads, x as f32 + 0.5, y as f32 + 0.5) {
                let shade = 0.5 + 0.5 * (((x as f32 / wf - pad.x) * 300.0).sin() * 0.5 + 0.5);
                let c = mix([0.16, 0.36, 0.14], [0.36, 0.58, 0.22], shade);
                let _ = floating.set_pixel(x, y, [c[0], c[1], c[2], 1.0]);
            }
        }
    }
    for pad in &pads {
        if let Some(color) = pad.flower {
            let (fx, fy, r) = (pad.x * wf, pad.y * hf, pad.radius * wf);
            for k in 0..5 {
                let a = k as f32 / 5.0 * std::f32::consts::TAU;
                petal_over(&mut floating, fx + a.cos() * r * 0.32, fy - r * 0.18 + a.sin() * r * 0.1, r * 0.34, r * 0.16, a * 0.3, color);
            }
            petal_over(&mut floating, fx, fy - r * 0.2, r * 0.12, r * 0.1, 0.0, [0.98, 0.84, 0.30]);
        }
    }
    // The near bank at both lower corners: dark ground, blades and irises.
    for y in 0..h {
        for x in 0..w {
            let (u, v) = (x as f32 / wf, y as f32 / hf);
            // Two low mounds that run out into the water (no straight edge).
            let left = 0.84 + 0.17 * (u / 0.32).min(1.0).powf(1.5);
            let right = 0.85 + 0.16 * ((1.0 - u) / 0.30).min(1.0).powf(1.5);
            if v > left || v > right {
                let c = mix([0.16, 0.26, 0.10], [0.08, 0.14, 0.06], (v - 0.8) / 0.2);
                let _ = floating.set_pixel(x, y, [c[0], c[1], c[2], 1.0]);
            }
        }
    }
    for k in 0..22 {
        let right = k % 2 == 1;
        let u = if right { 0.76 + random.next_single() * 0.23 } else { 0.01 + random.next_single() * 0.27 };
        let (x, y) = (u * wf, hf * (0.90 + random.next_single() * 0.09));
        let s = 0.9 + random.next_single() * 0.6;
        for b in 0..3 {
            let lean = (b as f32 - 1.0) * 8.0 * s + random.uniform(-3.0, 3.0) as f32;
            segment_over(&mut floating, (x, y), (x + lean, y - random.uniform(22.0, 36.0) as f32 * s), 3.0 * s, [0.20, 0.48, 0.20]);
        }
        let top = (x, y - 30.0 * s);
        segment_over(&mut floating, (x, y), top, 1.6 * s, [0.24, 0.52, 0.22]);
        let petal = mix([0.44, 0.22, 0.72], [0.30, 0.36, 0.88], random.next_single());
        let dark = mix(petal, [0.12, 0.06, 0.26], 0.45);
        petal_over(&mut floating, top.0 - 4.5 * s, top.1 + 0.5 * s, 4.0 * s, 2.2 * s, 0.6, dark);
        petal_over(&mut floating, top.0 + 4.5 * s, top.1 + 0.5 * s, 4.0 * s, 2.2 * s, -0.6, dark);
        petal_over(&mut floating, top.0, top.1 - 5.0 * s, 2.6 * s, 5.0 * s, 0.0, petal);
        petal_over(&mut floating, top.0, top.1 - 2.0 * s, 1.2 * s, 1.0 * s, 0.0, [0.98, 0.84, 0.30]);
    }
    // The whole picture: the floating parts laid over the rest.
    let (base, top) = (image.clone(), floating.clone());
    image.fill_with(|x, y| {
        let (b, t) = (base.get_pixel(x, y).unwrap(), top.get_pixel(x, y).unwrap());
        let a = t[3];
        [b[0] + (t[0] - b[0]) * a, b[1] + (t[1] - b[1]) * a, b[2] + (t[2] - b[2]) * a, 1.0]
    });
    (image, floating)
}

/// [`over`] for a picture on a clear ground: the cover adds to the alpha (straight colour).
fn over_clear(image: &mut DrawingPlaneImage, x: i32, y: i32, color: [f32; 3], coverage: f32) {
    let (w, h) = image.size();
    if x < 0 || y < 0 || x >= w as i32 || y >= h as i32 || coverage <= 0.0 {
        return;
    }
    let c = coverage.min(1.0);
    let old = image.get_pixel(x as u32, y as u32).unwrap();
    let alpha = old[3] + (1.0 - old[3]) * c;
    let keep = if alpha > 0.0 { old[3] * (1.0 - c) / alpha } else { 0.0 };
    let _ = image.set_pixel(x as u32, y as u32, [color[0] + (old[0] - color[0]) * keep, color[1] + (old[1] - color[1]) * keep, color[2] + (old[2] - color[2]) * keep, alpha]);
}

/// [`ellipse`] onto a picture on a clear ground.
fn petal_over(image: &mut DrawingPlaneImage, cx: f32, cy: f32, rx: f32, ry: f32, angle: f32, color: [f32; 3]) {
    let r = rx.max(ry) + 1.0;
    let (s, c) = angle.sin_cos();
    for y in (cy - r).floor() as i32..=(cy + r).ceil() as i32 {
        for x in (cx - r).floor() as i32..=(cx + r).ceil() as i32 {
            let (dx, dy) = (x as f32 + 0.5 - cx, y as f32 + 0.5 - cy);
            let (u, v) = (dx * c + dy * s, -dx * s + dy * c);
            let d = ((u / rx).powi(2) + (v / ry).powi(2)).sqrt();
            over_clear(image, x, y, color, (1.0 - d) * rx.min(ry) + 0.5);
        }
    }
}

/// [`segment`] onto a picture on a clear ground.
fn segment_over(image: &mut DrawingPlaneImage, a: (f32, f32), b: (f32, f32), width: f32, color: [f32; 3]) {
    let (minx, maxx) = (a.0.min(b.0) - width, a.0.max(b.0) + width);
    let (miny, maxy) = (a.1.min(b.1) - width, a.1.max(b.1) + width);
    let (ux, uy) = (b.0 - a.0, b.1 - a.1);
    let len2 = (ux * ux + uy * uy).max(1e-6);
    for y in miny.floor() as i32..=maxy.ceil() as i32 {
        for x in minx.floor() as i32..=maxx.ceil() as i32 {
            let (px, py) = (x as f32 + 0.5 - a.0, y as f32 + 0.5 - a.1);
            let t = ((px * ux + py * uy) / len2).clamp(0.0, 1.0);
            let (qx, qy) = (px - t * ux, py - t * uy);
            let half = width * 0.5 * (1.0 - 0.7 * t);
            over_clear(image, x, y, color, half - (qx * qx + qy * qy).sqrt() + 0.5);
        }
    }
}
