//! The pictures of the "sprites", "images" and "clip-mask" scenes. THE TWO WAYS a picture
//! reaches the crate, both shown here: (1) from the BACKEND, decoded there with CodeBrix.Imaging
//! and sent as pixels (the generated `cube_pets_colormap_bytes` call; the sample's pixel form:
//! width and height as two little-endian u32, then straight RGBA) - kept by `set_backend_picture`
//! for the scenes; (2) made in the frontend's own code as RGBA bytes, as the C# demo paints its
//! pictures in code (a landscape, an emblem, pixel art, a ball sheet, a flame sheet). The C#
//! scenes paint with the pixel planes and their noise (a later step of the crate); these paint
//! with plain loops and a small value noise of the sample's own, so the pictures look alike but
//! are not the same pixels.

use std::cell::RefCell;

use codebrix_webgpu_animation::animation::{ColorRamp, Easing};
use codebrix_webgpu_animation::images::ImageSource;
use codebrix_webgpu_animation::Color;

thread_local! {
    static BACKEND_PICTURE: RefCell<Option<ImageSource>> = const { RefCell::new(None) };
    static BACKEND_PNG_PICTURE: RefCell<Option<ImageSource>> = const { RefCell::new(None) };
}

/// Reads the picture-pixels form (the Animation package's: "CBPX", versions, width, height, RGBA) into a picture.
pub fn from_pixel_form(bytes: &[u8]) -> Result<ImageSource, String> {
    // The Animation package's picture-pixels form (written by PicturePixelsWriter on the backend).
    ImageSource::from_picture_pixels(bytes)
}

/// Keeps the picture the backend decoded (the page sets it before the catalogue is made).
pub fn set_backend_picture(picture: ImageSource) {
    BACKEND_PICTURE.with(|p| *p.borrow_mut() = Some(picture));
}

/// Keeps the picture read from the backend's PNG file (the page sets it before the catalogue is made).
pub fn set_backend_png_picture(picture: ImageSource) {
    BACKEND_PNG_PICTURE.with(|p| *p.borrow_mut() = Some(picture));
}

/// The picture read from the backend's PNG file (a 1 x 1 grey pixel when none was set).
pub fn backend_png_picture() -> ImageSource {
    BACKEND_PNG_PICTURE.with(|p| p.borrow().clone()).unwrap_or_else(|| ImageSource::from_rgba(&[128, 128, 128, 255], 1, 1).unwrap())
}

/// The picture the backend decoded (a 1 x 1 grey pixel when none was set).
pub fn backend_picture() -> ImageSource {
    BACKEND_PICTURE.with(|p| p.borrow().clone()).unwrap_or_else(|| ImageSource::from_rgba(&[128, 128, 128, 255], 1, 1).unwrap())
}

/// A straight-alpha RGBA canvas in floats (0..1, sRGB values), composited source-over.
pub struct Paint {
    pub width: usize,
    pub px: Vec<[f32; 4]>,
}

impl Paint {
    pub fn new(width: usize, height: usize) -> Paint {
        Paint { width, px: vec![[0.0; 4]; width * height] }
    }

    /// Puts a straight colour over the pixel at `coverage`.
    pub fn over(&mut self, x: usize, y: usize, c: [f32; 4], coverage: f32) {
        let a = c[3] * coverage;
        if a <= 0.0 {
            return;
        }
        let d = &mut self.px[y * self.width + x];
        let out_a = a + d[3] * (1.0 - a);
        for i in 0..3 {
            d[i] = (c[i] * a + d[i] * d[3] * (1.0 - a)) / out_a;
        }
        d[3] = out_a;
    }

    /// The coverage of an ellipse over a pixel (4 x 4 samples).
    pub fn ellipse_coverage(x: usize, y: usize, cx: f32, cy: f32, rx: f32, ry: f32) -> f32 {
        let mut inside = 0;
        for sy in 0..4 {
            for sx in 0..4 {
                let px = x as f32 + (sx as f32 + 0.5) / 4.0 - cx;
                let py = y as f32 + (sy as f32 + 0.5) / 4.0 - cy;
                if (px / rx).powi(2) + (py / ry).powi(2) <= 1.0 {
                    inside += 1;
                }
            }
        }
        inside as f32 / 16.0
    }

    pub fn to_rgba(&self) -> Vec<u8> {
        self.px.iter().flat_map(|p| p.map(|v| (v.clamp(0.0, 1.0) * 255.0).round() as u8)).collect()
    }
}

fn lerp(a: [f32; 4], b: [f32; 4], t: f32) -> [f32; 4] {
    [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t, a[2] + (b[2] - a[2]) * t, a[3] + (b[3] - a[3]) * t]
}

fn rgb(c: Color) -> [f32; 4] {
    [c.r as f32 / 255.0, c.g as f32 / 255.0, c.b as f32 / 255.0, c.a as f32 / 255.0]
}

/// A lattice hash (SplitMix64's finaliser) to a value in -1..1.
fn hash(x: i64, y: i64, seed: u64) -> f64 {
    let mut z = (x as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ (y as u64).wrapping_mul(0xC2B2_AE3D_27D4_EB4F) ^ seed.wrapping_mul(0x1656_67B1_9E37_79F9);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^= z >> 31;
    (z >> 11) as f64 / (1u64 << 53) as f64 * 2.0 - 1.0
}

/// Smooth value noise, -1..1.
fn value_noise(x: f64, y: f64, seed: u64) -> f64 {
    let (x0, y0) = (x.floor(), y.floor());
    let (fx, fy) = (Easing::smooth_step(x - x0), Easing::smooth_step(y - y0));
    let (ix, iy) = (x0 as i64, y0 as i64);
    let top = hash(ix, iy, seed) + (hash(ix + 1, iy, seed) - hash(ix, iy, seed)) * fx;
    let bottom = hash(ix, iy + 1, seed) + (hash(ix + 1, iy + 1, seed) - hash(ix, iy + 1, seed)) * fx;
    top + (bottom - top) * fy
}

/// Fractal value noise (octaves halving in amplitude), about -1..1.
pub fn fractal(x: f64, y: f64, seed: u64, octaves: u32) -> f64 {
    let (mut sum, mut amplitude, mut frequency, mut total) = (0.0, 1.0, 1.0, 0.0);
    for o in 0..octaves {
        sum += value_noise(x * frequency, y * frequency, seed + o as u64) * amplitude;
        total += amplitude;
        amplitude *= 0.5;
        frequency *= 2.0;
    }
    sum / total
}

/// The "images" scene's landscape: a sky gradient, a sun, two rows of noisy hills (C#
/// ImageScene.PaintLandscape's design).
pub fn landscape(width: usize, height: usize) -> ImageSource {
    let mut p = Paint::new(width, height);
    for y in 0..height {
        for x in 0..width {
            let v = y as f32 / height as f32;
            let sky = [0.30 + 0.55 * v, 0.45 + 0.35 * v, 0.85 - 0.25 * v, 1.0];
            let (dx, dy) = (x as f32 - 210.0, y as f32 - 70.0);
            let sun = (1.2 - (dx * dx + dy * dy).sqrt() / 26.0).clamp(0.0, 1.0);
            let mut color = lerp(sky, [1.0, 0.92, 0.6, 1.0], sun);
            let far = 120.0 + 22.0 * fractal(x as f64 * 0.012, 0.5, 11, 3);
            let near = 150.0 + 26.0 * fractal(x as f64 * 0.02, 3.5, 12, 3);
            color = lerp(color, [0.30, 0.42, 0.50, 1.0], (y as f64 + 0.5 - far).clamp(0.0, 1.0) as f32);
            color = lerp(color, [0.16 + 0.0004 * x as f32, 0.34, 0.20, 1.0], (y as f64 + 0.5 - near).clamp(0.0, 1.0) as f32);
            p.px[y * width + x] = color;
        }
    }
    ImageSource::from_rgba_vec(p.to_rgba(), width as u32, height as u32).unwrap()
}

/// The round emblem: rings of colour, a soft edge fading to transparent (C# PaintEmblem).
pub fn emblem(size: usize) -> ImageSource {
    let mut p = Paint::new(size, size);
    let c = size as f32 / 2.0;
    for y in 0..size {
        for x in 0..size {
            let (dx, dy) = (x as f32 + 0.5 - c, y as f32 + 0.5 - c);
            let r = (dx * dx + dy * dy).sqrt() / c;
            let alpha = ((1.0 - r) * 12.0).clamp(0.0, 1.0);
            let ring = if (r * 5.0).floor() % 2.0 == 0.0 { 1.0 } else { 0.55 };
            p.px[y * size + x] = [ring, (0.55 + 0.4 * r) * ring, 0.2 + 0.3 * (1.0 - r), alpha];
        }
    }
    ImageSource::from_rgba_vec(p.to_rgba(), size as u32, size as u32).unwrap()
}

const HEART_MAP: [&str; 11] = ["..XXX...XXX..", ".XoowX.XoooX.", "XoowwoXoooooX", "XoowooooooooX", "XoooooooooooX", ".XoooooooooX.", "..XoooooooX..", "...XoooooX...", "....XoooX....", ".....XoX.....", "......X......"];

/// The 13 x 11 pixel-art heart (C# PaintPixelArt): outline, fill, highlight, transparent surround.
pub fn pixel_heart() -> ImageSource {
    let mut rgba = Vec::new();
    for row in HEART_MAP {
        for ch in row.chars() {
            rgba.extend_from_slice(&match ch {
                'X' => [60, 20, 40, 255],
                'o' => [230, 50, 80, 255],
                'w' => [255, 230, 235, 255],
                _ => [0, 0, 0, 0],
            });
        }
    }
    ImageSource::from_rgba_vec(rgba, 13, 11).unwrap()
}

/// The "sprites" scene's ball sheet: eight 96 x 96 cells of a ball bouncing on a floor, squashed at
/// the bottom, with its shadow (C# SpriteScene.BuildBallSheet's design, rasterised here).
pub fn ball_sheet() -> ImageSource {
    const CELL: usize = 96;
    let mut p = Paint::new(CELL * 8, CELL);
    for i in 0..8 {
        let phase = i as f64 / 8.0;
        let height = (std::f64::consts::PI * phase).sin().abs() as f32;
        let squash = 1.0 + 0.35 * (1.0 - height).powi(6);
        let radius = 22.0;
        let cy = 82.0 - radius / squash - 34.0 * height;
        let (rx, ry) = (radius * squash, radius / squash);
        let shadow = [0.0, 0.0, 0.0, (150.0 - 90.0 * height) / 255.0];
        let (warm, hot) = (rgb(Color::from_rgb(255, 236, 170)), rgb(Color::from_rgb(240, 110, 60)));
        for y in 0..CELL {
            for x in 0..CELL {
                let px = i * CELL + x;
                p.over(px, y, shadow, Paint::ellipse_coverage(x, y, 48.0, 86.0, 24.0 * (1.2 - 0.5 * height), 5.0));
                let cover = Paint::ellipse_coverage(x, y, 48.0, cy, rx, ry);
                if cover > 0.0 {
                    let d = ((x as f32 + 0.5 - 40.0).powi(2) + (y as f32 + 0.5 - (cy - 8.0)).powi(2)).sqrt() / 30.0;
                    p.over(px, y, lerp(warm, hot, d.clamp(0.0, 1.0)), cover);
                }
            }
        }
    }
    ImageSource::from_rgba_vec(p.to_rgba(), (CELL * 8) as u32, CELL as u32).unwrap()
}

/// The pixel-art flame: twelve 16 x 24 cells of noise rising with time, shaped into a flame and
/// coloured by a ramp (C# SpriteScene.BuildFlameSheet's design).
pub fn flame_sheet() -> ImageSource {
    const W: usize = 16;
    const H: usize = 24;
    const FRAMES: usize = 12;
    let ramp = ColorRamp::from_colors(&[Color::from_rgba(80, 0, 0, 0), Color::from_rgb(200, 30, 10), Color::from_rgb(255, 140, 20), Color::from_rgb(255, 230, 120), Color::WHITE]).unwrap();
    let mut rgba = Vec::with_capacity(W * FRAMES * H * 4);
    for y in 0..H {
        for x in 0..W * FRAMES {
            let frame = x / W;
            let (fx, fy) = ((x % W) as f64 + 0.5, y as f64 + 0.5);
            let t = frame as f64 / FRAMES as f64;
            let n = fractal(fx * 0.28, fy * 0.22 + t * 6.0, 7, 3);
            let across = ((fx - W as f64 / 2.0) / (W as f64 / 2.0)).abs();
            let up = 1.0 - fy / H as f64;
            let heat = (1.1 - across * (0.6 + up) - up) + n * 0.5;
            rgba.extend_from_slice(&ramp.at(heat.clamp(0.0, 1.0)).to_rgba32());
        }
    }
    ImageSource::from_rgba_vec(rgba, (W * FRAMES) as u32, H as u32).unwrap()
}

/// The dissolve's alpha mask: noise normalised to 0..1, a threshold applied (C#'s per-frame
/// plane), as a white picture whose alpha is the mask - drawn on a hidden layer that is the
/// dissolving layer's matte.
pub struct DissolveNoise {
    width: usize,
    height: usize,
    values: Vec<f64>,
}

impl DissolveNoise {
    pub fn new(width: usize, height: usize) -> DissolveNoise {
        let mut values: Vec<f64> = (0..width * height).map(|i| fractal((i % width) as f64 * 0.08, (i / width) as f64 * 0.08, 21, 4)).collect();
        let (low, high) = values.iter().fold((f64::MAX, f64::MIN), |(l, h), v| (l.min(*v), h.max(*v)));
        for v in &mut values {
            *v = (*v - low) / (high - low);
        }
        DissolveNoise { width, height, values }
    }

    /// The mask for a threshold: (value - threshold) x 10, clamped.
    pub fn mask(&self, threshold: f64) -> ImageSource {
        let rgba: Vec<u8> = self.values.iter().flat_map(|v| [255, 255, 255, (((v - threshold) * 10.0).clamp(0.0, 1.0) * 255.0).round() as u8]).collect();
        ImageSource::from_rgba_vec(rgba, self.width as u32, self.height as u32).unwrap()
    }
}

/// C#'s DemoScene.PingPong: 0 up to 1 and back every period, eased at both ends.
pub fn ping_pong(seconds: f64, period: f64) -> f64 {
    let phase = (seconds % period) / period;
    Easing::sine_in_out(if phase < 0.5 { 2.0 * phase } else { 2.0 - 2.0 * phase })
}
