//! The painterly scenes' shared helpers (AnimationDemoApp's BrushworkKit, as far as these scenes
//! use it - sample code, not library API): distinct colours per element and the seeded stroke layout.

/// `count` clearly different colours (golden-angle hues, alternating lightness), as floats 0..1.
pub fn distinct_colors(count: usize) -> Vec<[f32; 4]> {
    (0..count)
        .map(|i| {
            let h = (i as f32 * 137.508).rem_euclid(360.0) / 60.0;
            let (s, v) = (0.62, if i % 2 == 0 { 0.95 } else { 0.78 });
            let c = v * s;
            let x = c * (1.0 - ((h % 2.0) - 1.0).abs());
            let (r, g, b) = match h as i32 {
                0 => (c, x, 0.0),
                1 => (x, c, 0.0),
                2 => (0.0, c, x),
                3 => (0.0, x, c),
                4 => (x, 0.0, c),
                _ => (c, 0.0, x),
            };
            let m = v - c;
            [r + m, g + m, b + m, 1.0]
        })
        .collect()
}

/// A seeded layout of brushstrokes in jittered rows, each turned along a gentle swirl, in a
/// shuffled painting order (BrushworkKit.Layout).
pub fn layout(width: u32, height: u32, size: f32, seed: i64) -> Vec<codebrix_webgpu_animation::shapes::BrushStrokeShape> {
    use codebrix_webgpu_animation::planes::DrawingRandom;
    use codebrix_webgpu_animation::shapes::{BrushStrokeShape, BrushStrokeShapeOptions};
    let flow = |u: f64, v: f64| (0.5 * ((u * 4.0) + (v * 2.5)).sin()) + (0.35 * ((v * 5.0) - (u * 1.5)).cos());
    let mut random = DrawingRandom::new(seed);
    let mut strokes = Vec::new();
    let (s, w, h) = (size as f64, width as f64, height as f64);
    let mut y = -s * 0.2;
    while y < h + (s * 0.3) {
        let mut x = -s * 0.3 + random.uniform(0.0, s * 0.3);
        while x < w + (s * 0.4) {
            let cx = x + random.normal(0.0, s * 0.08);
            let cy = y + random.normal(0.0, s * 0.06);
            let angle = flow(cx / w, cy / h) + random.normal(0.0, 0.12);
            let length = s * random.uniform(0.75, 1.25);
            let width = length * random.uniform(0.28, 0.4);
            let bend = random.normal(0.0, 0.1);
            strokes.push(BrushStrokeShape::new(cx as f32, cy as f32, length as f32, width as f32, (angle * (180.0 / std::f64::consts::PI)) as f32, BrushStrokeShapeOptions { bend: bend as f32, ..BrushStrokeShapeOptions::default() }).unwrap());
            x += s * 0.5;
        }
        y += s * 0.28;
    }
    for i in (1..strokes.len()).rev() {
        let j = random.next_int(i as i32 + 1).unwrap() as usize;
        strokes.swap(i, j);
    }
    strokes
}
