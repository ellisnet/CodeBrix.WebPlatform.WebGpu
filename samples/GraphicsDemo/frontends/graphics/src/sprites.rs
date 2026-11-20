//! The sprites layer: a SpriteBatch of codebrix-webgpu-graphics drawing sprites from a texture
//! atlas made in code, and shapes, animated from the frame's time. It shows each part of the
//! batch: TINT (the four atlas cells in a row, each tinted with a colour that cycles), SOURCE
//! RECTANGLES (the cells are regions of one texture), ROTATION, SCALE AND ORIGIN (an arrow turning
//! about its centre and breathing in size, and a star turning about its tip), ORDERING (three
//! overlapping cards whose drawing order changes every two seconds: the card drawn last is on
//! top), the TRANSFORM STACK (a ring of small squares turned as one group), and the SHAPES
//! (rectangles and outlines, lines, triangles, circles and rings). It is a LAYER of the canvas
//! host: it loads what is there, never clears, and takes its time from the frame (it stands still
//! while the clock is paused). It draws in CSS pixels: the batch's initial transform scales by the
//! device pixel ratio.

use codebrix_webgpu::glam::{Affine2, Vec2};
use codebrix_webgpu::{CanvasFrame, CanvasRenderer, CanvasTarget, GpuContext};
use codebrix_webgpu_graphics::{BlendPreset, CircleOptions, Color4, GpuTexture, GraphicsContext, GraphicsException, RectangleF, RgbaTextureOptions, SpriteBatch, SpriteBatchBeginOptions};
use dioxus::prelude::*;

/// The side of one atlas cell, in texels. The largest drawing of a cell is the arrow at the top of
/// its breathing: 1.4 x 1.35 x 96 = 181.4 CSS pixels, 363 device pixels on a canvas of pixel ratio
/// 2; 512 texels per cell is the first power of two at least that large, so that no cell is ever
/// magnified there.
pub const CELL: u32 = 512;
/// The samples per texel along each axis when the atlas's coverage is computed (4 x 4 = 16 per
/// texel, on a regular grid at the centres of a 4 x 4 split of the texel).
const SAMPLES: u32 = 4;
/// The side the star is drawn at, in CSS pixels (its scale is this over [`CELL`], so that the
/// atlas's resolution does not change its size on the canvas).
const STAR_SIDE: f32 = 38.4;
/// The label of the buffer the shared error handler check asks for (WebGPU refuses it). Every
/// graphics context of the device made before that request receives the error, the layer's own
/// among them: the layer counts it apart from its drawing errors by this label.
pub const REFUSED_LABEL: &str = "refused on purpose by the shared error handler check";
/// The atlas's cells, left to right.
pub const CELL_NAMES: [&str; 4] = ["disc", "star", "arrow", "checker"];
/// The names of the three cards, in the colours they are tinted with.
pub const CARD_NAMES: [&str; 3] = ["red", "green", "blue"];
/// Seconds each drawing order of the cards lasts.
const ORDER_SECONDS: f64 = 2.0;

/// What the page shows about the sprites.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct SpriteView {
    /// The frame number on the clock.
    pub frame: u64,
    /// The clock's time, in seconds.
    pub time: f64,
    /// The draw calls of the batch's last end.
    pub draw_calls: usize,
    /// The quads of the batch's last end.
    pub quads: usize,
    /// The card drawn last (on top), an index into [`CARD_NAMES`].
    pub top_card: usize,
    /// The arrow's angle, in degrees (0 to 360).
    pub arrow_degrees: f32,
    /// The atlas texture's width, height and mip levels (zeros until it is made).
    pub atlas: (u32, u32, u32),
}

/// Draws the sprites.
pub struct SpritesRenderer {
    graphics: GraphicsContext,
    gpu: Option<(SpriteBatch, GpuTexture)>,
    view: Signal<SpriteView>,
    errors: Signal<Vec<String>>,
    deliberate: Signal<usize>,
    nearest: Signal<bool>,
}

impl SpritesRenderer {
    /// A renderer drawing with `graphics`, publishing its view into `view`, every error into
    /// `errors` and the number of the shared error handler check's deliberate errors its context
    /// received into `deliberate`; `nearest` chooses nearest-texel filtering.
    pub fn new(graphics: GraphicsContext, view: Signal<SpriteView>, errors: Signal<Vec<String>>, deliberate: Signal<usize>, nearest: Signal<bool>) -> SpritesRenderer {
        SpritesRenderer { graphics, gpu: None, view, errors, deliberate, nearest }
    }

    fn report(&mut self, error: String) {
        if error.contains(REFUSED_LABEL) {
            // Not a drawing error: the check's deliberate one, which reaches this context too.
            *self.deliberate.write() += 1;
            return;
        }
        let mut errors = self.errors.write();
        if errors.len() < 20 {
            errors.push(error);
        }
    }
}

/// The atlas: four cells of [`CELL`] x [`CELL`] texels side by side - a soft disc, a five-pointed
/// star, an arrow pointing right and a checker - white and grey on transparent, so that the tint
/// gives them their colour. RGBA, straight (not premultiplied) alpha: the alpha of a texel is the
/// share of its [`SAMPLES`] x [`SAMPLES`] samples inside the shape (anti-aliased edges), its
/// colour the shade at its centre (also where it is transparent, so that the mip levels' box
/// filter blends no black into the edges).
pub fn atlas_pixels() -> Vec<u8> {
    let width = CELL * 4;
    let mut pixels = vec![0u8; (width * CELL * 4) as usize];
    let c = CELL as f32 / 2.0;
    let step = 1.0 / SAMPLES as f32;
    let samples = (SAMPLES * SAMPLES) as f32;
    for y in 0..CELL {
        for x in 0..width {
            let cell = x / CELL;
            // The texel's top-left corner, from the cell's centre.
            let left = (x % CELL) as f32 - c;
            let top = y as f32 - c;
            let coverage = if cell == 3 {
                1.0
            } else {
                let mut inside = 0u32;
                for sy in 0..SAMPLES {
                    let py = top + (sy as f32 + 0.5) * step;
                    for sx in 0..SAMPLES {
                        let px = left + (sx as f32 + 0.5) * step;
                        if inside_shape(cell, px, py, c) {
                            inside += 1;
                        }
                    }
                }
                inside as f32 / samples
            };
            let (px, py) = (left + 0.5, top + 0.5);
            let shade = match cell {
                0 => 1.0 - 0.35 * (px * px + py * py).sqrt() / c,
                1 => 1.0,
                2 => {
                    if py < 0.0 {
                        1.0
                    } else {
                        0.8
                    }
                }
                _ => {
                    let dark = ((x % CELL) * 8 / CELL + y * 8 / CELL) % 2 == 1;
                    if dark {
                        0.55
                    } else {
                        1.0
                    }
                }
            };
            let i = ((y * width + x) * 4) as usize;
            let v = (shade.clamp(0.0, 1.0) * 255.0) as u8;
            pixels[i..i + 4].copy_from_slice(&[v, v, v, (coverage * 255.0).round() as u8]);
        }
    }
    pixels
}

/// True when the point (`px`, `py`), in texels from the centre of atlas cell `cell` whose half side
/// is `c`, lies inside that cell's shape (the checker covers its whole cell).
fn inside_shape(cell: u32, px: f32, py: f32, c: f32) -> bool {
    match cell {
        0 => px * px + py * py <= (0.95 * c) * (0.95 * c),
        1 => {
            // Five points: the radius runs from the inner 0.4 to the outer 0.95 of the half cell
            // and back between two points, the first point straight up.
            let angle = py.atan2(px) + std::f32::consts::FRAC_PI_2;
            let sector = std::f32::consts::TAU / 5.0;
            let point = ((angle.rem_euclid(sector) / sector) - 0.5).abs() * 2.0;
            let radius = c * (0.4 + 0.55 * point);
            px * px + py * py <= radius * radius
        }
        2 => {
            let shaft = px > -c * 0.85 && px < c * 0.1 && py.abs() < c * 0.18;
            let head = px >= c * 0.1 && px < c * 0.9 && py.abs() < (c * 0.9 - px) * 0.75;
            shaft || head
        }
        _ => true,
    }
}

/// The source rectangle of atlas cell `index`.
fn cell(index: usize) -> RectangleF {
    RectangleF::new((index as u32 * CELL) as f32, 0.0, CELL as f32, CELL as f32)
}

/// A colour of the hue circle (`turn` 0..1), fully saturated, linear.
fn hue(turn: f32) -> Color4 {
    let h = turn.rem_euclid(1.0) * 6.0;
    let x = 1.0 - (h % 2.0 - 1.0).abs();
    let (r, g, b) = match h as u32 {
        0 => (1.0, x, 0.0),
        1 => (x, 1.0, 0.0),
        2 => (0.0, 1.0, x),
        3 => (0.0, x, 1.0),
        4 => (x, 0.0, 1.0),
        _ => (1.0, 0.0, x),
    };
    Color4::new(r, g, b, 1.0)
}

/// The card drawn on top at time `time`.
pub fn top_card(time: f64) -> usize {
    ((time / ORDER_SECONDS).floor() as i64).rem_euclid(3) as usize
}

impl SpritesRenderer {
    /// Collects the whole scene into the batch, in CSS pixels of a `width` x `height` canvas.
    fn collect(batch: &mut SpriteBatch, atlas: &GpuTexture, time: f32, width: f32, height: f32) -> f32 {
        let panel = RectangleF::new(16.0, 16.0, width - 32.0, height - 32.0);
        batch.fill_rectangle(panel, Color4::new(0.02, 0.025, 0.05, 1.0));
        batch.draw_rectangle(panel, Color4::new(0.25, 0.3, 0.45, 1.0), 2.0);

        // TINT and SOURCE RECTANGLES: the four cells of the atlas in a row, each tinted with a
        // colour that cycles round the hue circle.
        let size = (height * 0.16).clamp(32.0, 96.0);
        for index in 0..4 {
            let destination = RectangleF::new(36.0 + index as f32 * (size + 12.0), 36.0, size, size);
            batch.draw_region(atlas, destination, cell(index), hue(time * 0.1 + index as f32 * 0.25));
        }

        // ROTATION, SCALE AND ORIGIN: the arrow turns about its centre and breathes; the star
        // turns the other way about its top-left corner.
        let arrow_angle = time * 0.9;
        let breathe = 1.0 + 0.35 * (time * 1.7).sin();
        let centre = Vec2::new(width * 0.5, height * 0.55);
        batch.draw_rotated(atlas, centre, cell(2), Color4::new(1.0, 0.85, 0.3, 1.0), arrow_angle, Vec2::splat(CELL as f32 / 2.0), Vec2::splat(breathe * size / CELL as f32 * 1.4));
        batch.draw_rotated(atlas, Vec2::new(width * 0.5, height * 0.25), cell(1), Color4::new(0.6, 0.9, 1.0, 1.0), -time * 0.6, Vec2::ZERO, Vec2::splat(STAR_SIDE / CELL as f32));

        // ORDERING: three overlapping cards; the one drawn last covers the others, and the order
        // turns every two seconds.
        let tints = [Color4::new(0.9, 0.2, 0.2, 1.0), Color4::new(0.2, 0.8, 0.3, 1.0), Color4::new(0.25, 0.4, 1.0, 1.0)];
        let top = top_card(time as f64);
        let card = size * 1.2;
        let base = Vec2::new(width * 0.72, height * 0.42);
        for step in 1..=3 {
            let index = (top + step) % 3;
            let offset = Vec2::new(index as f32 * card * 0.45, index as f32 * card * 0.3);
            let corner = base + offset;
            batch.draw_region(atlas, RectangleF::new(corner.x, corner.y, card, card), cell(3), tints[index]);
            batch.draw_rectangle(RectangleF::new(corner.x, corner.y, card, card), Color4::WHITE, 2.0);
        }

        // THE TRANSFORM STACK: a ring of small squares, turned as one group about its centre.
        let ring_centre = Vec2::new(width * 0.2, height * 0.68);
        batch.push_transform(Affine2::from_translation(ring_centre) * Affine2::from_angle(time * 0.5));
        for index in 0..8 {
            let angle = index as f32 * std::f32::consts::TAU / 8.0;
            let p = Vec2::new(angle.cos(), angle.sin()) * size * 0.6;
            batch.fill_rectangle(RectangleF::new(p.x - 6.0, p.y - 6.0, 12.0, 12.0), hue(index as f32 / 8.0));
        }
        let _ = batch.pop_transform();

        // SHAPES: a clock face of lines, a triangle, a filled circle orbiting a ring.
        let clock = Vec2::new(width * 0.2, height * 0.68);
        batch.draw_circle(clock, size * 0.9, Color4::new(0.5, 0.55, 0.7, 1.0), CircleOptions { thickness: 3.0, segments: 0 });
        let hand = Vec2::new((time * 1.2).cos(), (time * 1.2).sin()) * size * 0.4;
        batch.draw_line(clock, clock + hand, Color4::WHITE, 4.0);
        batch.draw_line(clock, clock + Vec2::new(-hand.y, hand.x) * 0.6, Color4::new(1.0, 0.6, 0.2, 1.0), 3.0);
        let orbit = Vec2::new(width * 0.5, height * 0.55) + Vec2::new((time * 0.7).cos(), (time * 0.7).sin()) * size * 1.6;
        batch.fill_circle(orbit, size * 0.15, Color4::new(0.95, 0.35, 0.75, 1.0), 0);
        let tri = Vec2::new(width * 0.84, height * 0.18);
        batch.fill_triangle(tri, tri + Vec2::new(size * 0.6, size * 0.5), tri + Vec2::new(-size * 0.2, size * 0.6), hue(time * 0.05 + 0.5));
        arrow_angle
    }
}

impl CanvasRenderer for SpritesRenderer {
    fn prepare(&mut self, _context: &GpuContext, _target: &CanvasTarget) {
        let made: Result<(SpriteBatch, GpuTexture), GraphicsException> = (|| {
            let batch = SpriteBatch::new(&self.graphics)?;
            let atlas = self.graphics.create_texture_from_rgba(CELL * 4, CELL, &atlas_pixels(), RgbaTextureOptions { mipmaps: true, srgb: true, label: Some("sprite atlas".to_string()) })?;
            Ok((batch, atlas))
        })();
        match made {
            Ok(gpu) => self.gpu = Some(gpu),
            Err(error) => self.report(error.to_string()),
        }
    }

    fn render(&mut self, frame: &mut CanvasFrame<'_>) {
        let time = frame.info.time;
        let ratio = frame.info.pixel_ratio as f32;
        let width = frame.info.width as f32 / ratio;
        let height = frame.info.height as f32 / ratio;
        let number = frame.info.number;
        let nearest = *self.nearest.peek();
        let graphics = self.graphics.clone();
        let mut failures = Vec::new();
        let mut arrow = 0.0;
        let mut counts = (0, 0);
        let mut atlas_size = (0, 0, 0);
        if let Some((batch, atlas)) = self.gpu.as_mut() {
            atlas_size = (atlas.width(), atlas.height(), atlas.mip_level_count());
            batch.set_use_nearest_filtering(nearest);
            let mut graphics_frame = graphics.begin_canvas_frame(frame, None);
            // None: LOAD what the host and the layers below drew; a layer never clears.
            match graphics_frame.begin_render_pass(None, None) {
                Ok(mut pass) => {
                    let options = SpriteBatchBeginOptions { transform: Affine2::from_scale(Vec2::splat(ratio)), blend: BlendPreset::AlphaBlend };
                    if let Err(error) = batch.begin(&pass, options) {
                        failures.push(error.to_string());
                    }
                    arrow = SpritesRenderer::collect(batch, atlas, time as f32, width, height);
                    if let Err(error) = batch.end(&mut pass) {
                        failures.push(error.to_string());
                    }
                    counts = (batch.last_draw_call_count(), batch.last_quad_count());
                }
                Err(error) => failures.push(error.to_string()),
            };
        }
        for error in failures {
            self.report(error);
        }
        for error in graphics.take_errors() {
            self.report(error.to_string());
        }
        let view = SpriteView { frame: number, time, draw_calls: counts.0, quads: counts.1, top_card: top_card(time), arrow_degrees: arrow.to_degrees().rem_euclid(360.0), atlas: atlas_size };
        if *self.view.peek() != view {
            self.view.set(view);
        }
    }
}
