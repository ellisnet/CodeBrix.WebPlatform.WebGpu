//! Host tests of the water's GPU half (package A2): CPU MIRRORS of effects.wgsl's water code
//! (the ripple loop, the device -> water map and the offset, the light, the picture taps, the
//! mirror and its fade) checked against the CPU reference (water/ripple_math.rs `evaluate`) and
//! against hand values; the composite's pass data; the builder's plan (refraction and reflection
//! on the layer's ONE effect group, the frame's ripple buffer, region growth, the nesting budget,
//! the checkpoint keys of live sources); the session refraction; the looks.

use kurbo::{Affine, Point, Rect};

use crate::effects::effect_sizes::{ReflectionParams, MAX_WATER_OFFSET, WATER_CAUSTICS, WATER_GLINTS, WATER_REFLECTION, WATER_REFRACTION, WATER_SAME};
use crate::effects::{DrawingEffects, SurfaceFinish};
use crate::images::ImageSource;
use crate::models::DrawingCamera;
use crate::rendering::{core_fx_pass_data as FxPassData, PictureTarget, TestBuilder as Builder, VectorScene};
use crate::shapes::{BrushStrokesOptions, BrushStrokesShape, RectangleShape, RectangleShapeOptions};
use crate::strokes::{BrushStrokeRecord, BrushStrokeSet, StrokeCheckpoints};
use crate::water::ripple_math::RippleSample;
use crate::water::{CausticLight, RippleDrop, RippleField, RippleOptions, RippleRain, RippleSwell, WaterGlints, WaterReflection, WaterRefraction, WaterSource};
use crate::{Color, DrawingSession, DrawingSessionOptions, PointF, RectangleF, SessionBlendMode, SessionPaint, SessionRenderOptions, SessionWipe, WipeEdge};

// ---- CPU mirrors of effects.wgsl (keep them equal to the shader) ------------------------------

/// effects.wgsl ripple_ring, line for line.
fn gpu_ring(w: &mut RippleSample, q: [f32; 2], cx: f32, cy: f32, a: [f32; 4], b: [f32; 4]) {
    let (front, amplitude, lambda, width) = (a[2], a[3], b[0], b[1]);
    let dx = q[0] - cx;
    let dy = q[1] - cy;
    let r = (dx * dx + dy * dy).sqrt();
    let u = r - front;
    if u > 1.0 * lambda || u < -3.0 * width {
        return;
    }
    let k = 6.283_185_5_f32 / lambda;
    let c = if u < 0.0 { width } else { 0.25 * lambda };
    let qq = u / c;
    let e = (-(qq * qq)).exp();
    let e1 = -2.0 * qq / c * e;
    let e2 = (4.0 * qq * qq - 2.0) / (c * c) * e;
    let g = 1.0 + r / lambda;
    let s = 1.0 / g.sqrt();
    let s1 = -0.5 / lambda * s / g;
    let s2 = 0.75 / (lambda * lambda) * s / (g * g);
    let sn = (k * u).sin();
    let cs = (k * u).cos();
    let p1 = k * cs;
    let p2 = -k * k * sn;
    let h = amplitude * s * e * sn;
    let h1 = amplitude * (s1 * e * sn + s * e1 * sn + s * e * p1);
    let h2 = amplitude * (s2 * e * sn + s * e2 * sn + s * e * p2 + 2.0 * (s1 * e1 * sn + s1 * e * p1 + s * e1 * p1));
    w.height += h;
    if r > 0.0 {
        w.slope[0] += h1 * dx / r;
        w.slope[1] += h1 * dy / r;
    }
    w.focus += h2 + h1 / r.max(0.25 * lambda);
}

/// effects.wgsl ripple_water: the slot's header words and place over the frame's records.
fn gpu_ripple_water(words: [u32; 4], place: [f32; 4], ripples: &[[f32; 4]], q: [f32; 2]) -> RippleSample {
    let mut w = RippleSample::default();
    let (offset, drops, waves, walls) = (words[0] as usize, words[1] as usize, words[2] as usize, words[3] & 1 != 0);
    for i in 0..drops {
        let a = ripples[offset + i * 2];
        let b = ripples[offset + i * 2 + 1];
        if walls {
            let xs = [2.0 * place[0] - a[0], a[0], 2.0 * place[2] - a[0]];
            let ys = [2.0 * place[1] - a[1], a[1], 2.0 * place[3] - a[1]];
            for y in ys {
                for x in xs {
                    gpu_ring(&mut w, q, x, y, a, b);
                }
            }
        } else {
            gpu_ring(&mut w, q, a[0], a[1], a, b);
        }
    }
    let base = offset + drops * 2;
    for j in 0..waves {
        let wave = ripples[base + j];
        let phase = wave[0] * q[0] + wave[1] * q[1] + wave[3];
        let (sn, cs) = (phase.sin(), phase.cos());
        let h = wave[2] * sn;
        w.height += h;
        w.slope[0] += wave[2] * cs * wave[0];
        w.slope[1] += wave[2] * cs * wave[1];
        w.focus -= (wave[0] * wave[0] + wave[1] * wave[1]) * h;
    }
    w
}

/// effects.wgsl water_point.
fn gpu_water_point(fp: &FxPassData, p: [f32; 2]) -> [f32; 2] {
    [fp.v14[0] * p[0] + fp.v14[2] * p[1] + fp.v15[0], fp.v14[1] * p[0] + fp.v14[3] * p[1] + fp.v15[1]]
}

/// effects.wgsl water_offset.
fn gpu_water_offset(fp: &FxPassData, slope: [f32; 2], strength: f32, reach: f32) -> [f32; 2] {
    let d = [strength * slope[0], strength * slope[1]];
    let v = [fp.v16[0] * d[0] + fp.v16[2] * d[1], fp.v16[1] * d[0] + fp.v16[3] * d[1]];
    let len = (v[0] * v[0] + v[1] * v[1]).sqrt();
    if len > reach {
        return [v[0] * (reach / len), v[1] * (reach / len)];
    }
    v
}

/// effects.wgsl in_region.
fn gpu_in_region(fp: &FxPassData, p: [f32; 2]) -> [f32; 2] {
    [p[0].clamp(fp.v0[0] + 0.5, fp.v0[2] - 0.5), p[1].clamp(fp.v0[1] + 0.5, fp.v0[3] - 0.5)]
}

fn normalize(v: [f32; 3]) -> [f32; 3] {
    let l = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
    [v[0] / l, v[1] / l, v[2] / l]
}

fn dot(a: [f32; 3], b: [f32; 3]) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

/// effects.wgsl water_light (the caustic pattern through its CPU twin, which is the same code).
fn gpu_water_light(c: [f32; 4], w: &RippleSample, q: [f32; 2], fp: &FxPassData) -> [f32; 4] {
    if c[3] <= 0.0 {
        return c;
    }
    let flags = fp.u4[0];
    let l = [fp.v18[0], fp.v18[1], fp.v18[2]];
    let n = normalize([-w.slope[0], -w.slope[1], 1.0]);
    let mut rgb = [c[0], c[1], c[2]];
    let mut lit = false;
    if fp.v17[1] != 0.0 {
        let shade = (1.0 + fp.v17[1] * (dot(n, l) - l[2])).max(0.0);
        rgb = rgb.map(|v| v * shade);
        lit = true;
    }
    if flags & WATER_CAUSTICS != 0 {
        let v = crate::planes::DrawingNoise::caustics((q[0] * fp.v15[3]) as f64, (q[1] * fp.v15[3]) as f64, fp.v15[2] as f64, fp.u4[3] as i64) as f32;
        let gather = (1.0 - fp.v19[3] * fp.v17[0] * w.focus).max(0.0);
        let k = fp.v18[3] * 0.5 * (v + 1.0) * gather;
        rgb = [rgb[0] + rgb[0] * fp.v20[0] * k, rgb[1] + rgb[1] * fp.v20[1] * k, rgb[2] + rgb[2] * fp.v20[2] * k];
        lit = true;
    }
    if flags & WATER_GLINTS != 0 {
        let d = dot(l, n);
        let reflected = [-l[0] + 2.0 * d * n[0], -l[1] + 2.0 * d * n[1], -l[2] + 2.0 * d * n[2]];
        let glint = fp.v17[2] * reflected[2].max(1e-6).powf(fp.v17[3]);
        rgb = [rgb[0] + fp.v19[0] * glint * c[3], rgb[1] + fp.v19[1] * glint * c[3], rgb[2] + fp.v19[2] * glint * c[3]];
        lit = true;
    }
    if lit {
        rgb = rgb.map(|v| v.clamp(0.0, c[3]));
    }
    [rgb[0], rgb[1], rgb[2], c[3]]
}

/// effects.wgsl picture_height over a picture's red bytes (textureSampleLevel's bilinear filter
/// with clamp-to-edge addressing).
fn gpu_picture_height(red: &dyn Fn(i32, i32) -> f32, size: (u32, u32), place: [f32; 4], scale: f32, bias: f32, q: [f32; 2]) -> f32 {
    let uv = [(q[0] - place[0]) / (place[2] - place[0]), (q[1] - place[1]) / (place[3] - place[1])];
    let (x, y) = (uv[0] * size.0 as f32 - 0.5, uv[1] * size.1 as f32 - 0.5);
    let (x0, y0) = (x.floor(), y.floor());
    let (fx, fy) = (x - x0, y - y0);
    let at = |i: i32, j: i32| red(i.clamp(0, size.0 as i32 - 1), j.clamp(0, size.1 as i32 - 1));
    let (i, j) = (x0 as i32, y0 as i32);
    let top = at(i, j) * (1.0 - fx) + at(i + 1, j) * fx;
    let bottom = at(i, j + 1) * (1.0 - fx) + at(i + 1, j + 1) * fx;
    ((top * (1.0 - fy) + bottom * fy) - bias) * scale
}

/// effects.wgsl picture_water.
fn gpu_picture_water(red: &dyn Fn(i32, i32) -> f32, size: (u32, u32), place: [f32; 4], scale: f32, bias: f32, q: [f32; 2]) -> RippleSample {
    let texel = [(place[2] - place[0]) / size.0 as f32, (place[3] - place[1]) / size.1 as f32];
    let h = |dx: f32, dy: f32| gpu_picture_height(red, size, place, scale, bias, [q[0] + dx, q[1] + dy]);
    let (h0, hl, hr, hu, hd) = (h(0.0, 0.0), h(-texel[0], 0.0), h(texel[0], 0.0), h(0.0, -texel[1]), h(0.0, texel[1]));
    RippleSample {
        height: h0,
        slope: [(hr - hl) / (2.0 * texel[0]), (hd - hu) / (2.0 * texel[1])],
        focus: (hl + hr - 2.0 * h0) / (texel[0] * texel[0]) + (hu + hd - 2.0 * h0) / (texel[1] * texel[1]),
    }
}

/// effects.wgsl with_reflection's sampling point of tap `i` (0..6) and the weight.
fn gpu_reflection_tap(fp: &FxPassData, p: [f32; 2], slope: [f32; 2], with_water: bool, i: usize) -> ([f32; 2], f32) {
    let n = [fp.v23[2], fp.v23[3]];
    let depth = (p[0] - fp.v23[0]) * n[0] + (p[1] - fp.v23[1]) * n[1];
    if depth <= 0.0 {
        return (p, 0.0);
    }
    let t = (depth / fp.v24[1]).clamp(0.0, 1.0);
    let weight = fp.v24[0] * (1.0 - t * t * (3.0 - 2.0 * t));
    let m = [fp.v23[0] + n[0] * fp.v27[0], fp.v23[1] + n[1] * fp.v27[0]];
    let d = (p[0] - m[0]) * n[0] + (p[1] - m[1]) * n[1];
    let mut mp = [p[0] - 2.0 * d * n[0], p[1] - 2.0 * d * n[1]];
    if with_water {
        let o = gpu_water_offset(fp, slope, fp.v24[3], fp.v27[1]);
        mp = [mp[0] + o[0], mp[1] + o[1]];
    }
    let s = fp.v24[2] * (i as f32 / 5.0 - 0.5);
    (gpu_in_region(fp, [mp[0] + n[0] * s, mp[1] + n[1] * s]), weight)
}

// ---- helpers ------------------------------------------------------------------------------

fn place() -> RectangleF {
    RectangleF::new(0.0, 0.0, 640.0, 360.0)
}

fn field() -> RippleField {
    RippleField::new(place(), RippleOptions::default())
        .unwrap()
        .with_drops(vec![RippleDrop::new(PointF::new(200.0, 150.0), 0.5), RippleDrop { center: PointF::new(420.0, 220.0), time: 1.2, strength: 0.7, size: 1.3 }])
        .unwrap()
        .with_rain(RippleRain::new(3.0, 9))
        .unwrap()
        .with_swell(RippleSwell::gentle(4, 160.0, 0.6))
        .unwrap()
}

fn session() -> DrawingSession {
    DrawingSession::new(DrawingSessionOptions { calibration_size: (640, 360), layer_opacity: 255, ..DrawingSessionOptions::default() })
}

fn floor(s: &mut DrawingSession, name: &str) {
    s.add_layer(name, Color::WHITE).add_shape(RectangleShape::new(0.0, 0.0, 640.0, 360.0, RectangleShapeOptions { is_filled: true, color: Some(Color::from_rgb(90, 150, 200)), ..RectangleShapeOptions::default() }).unwrap());
}

fn bank(s: &mut DrawingSession, name: &str) {
    // A shape ABOVE the waterline at 200 (y 60 .. 180).
    s.add_layer(name, Color::WHITE).add_shape(RectangleShape::new(100.0, 60.0, 200.0, 120.0, RectangleShapeOptions { is_filled: true, color: Some(Color::from_rgb(40, 120, 60)), ..RectangleShapeOptions::default() }).unwrap());
}

fn build_at(s: &DrawingSession, options: &SessionRenderOptions, width: u32, height: u32) -> Builder {
    let mut scene = VectorScene::new();
    s.render_with(&mut scene, width, height, options);
    let mut b = Builder::default();
    b.build(&scene, width, height);
    b
}

fn build(s: &DrawingSession) -> Builder {
    build_at(s, &SessionRenderOptions::default(), 640, 360)
}

fn kinds(s: &DrawingSession, options: &SessionRenderOptions) -> String {
    let mut scene = VectorScene::new();
    s.render_with(&mut scene, 640, 360, options);
    scene.item_kinds()
}

/// The composite pass data of the frame's effect `k` (its LayerEnd quad names it).
fn composite(b: &Builder, k: usize) -> FxPassData {
    use crate::rendering::{TestCompositeMode as CompositeMode, TestOp as Op};
    let quad = b.ops.iter().find_map(|o| match o {
        Op::LayerEnd { mode: CompositeMode::Effect(e), quad, .. } if *e as usize == k => Some(quad.clone()),
        _ => None,
    });
    let pass = b.cover[quad.unwrap().start as usize].paint;
    b.fx_passes[pass as usize]
}

// ---- tests --------------------------------------------------------------------------------

#[test]
fn the_composites_ripple_loop_equals_the_cpu_reference_through_the_frames_buffer() {
    //Arrange: two refracted layers with different fields (the second one's records after the
    // first's in the frame's ONE buffer), one with walls.
    let walled = RippleField::new(RectangleF::new(100.0, 50.0, 300.0, 200.0), RippleOptions { walls: true, ..RippleOptions::default() }).unwrap().with_drop(RippleDrop::new(PointF::new(150.0, 120.0), 0.2)).unwrap();
    for t in [0.75, 1.9, 3.3, 6.0, 11.25] {
        let mut s = session();
        floor(&mut s, "A");
        s.get_layer_mut("A").unwrap().set_refraction(Some(WaterRefraction::new(WaterSource::Ripples(field())))).unwrap();
        floor(&mut s, "B");
        s.get_layer_mut("B").unwrap().set_refraction(Some(WaterRefraction::new(WaterSource::Ripples(walled.clone())))).unwrap();
        s.set_time(t);
        //Act
        let b = build(&s);
        //Assert
        assert_eq!(b.effects.len(), 2);
        let (wa, wb) = (b.effects[0].water.as_ref().unwrap(), b.effects[1].water.as_ref().unwrap());
        assert_eq!(wa.slots[0].words[0], 0);
        assert_eq!(wb.slots[0].words[0] as usize, wa.ripples.end as usize, "the second field's records follow the first's");
        assert_eq!(b.stats.ripples_evaluated, field().live_drops(t).len() + walled.live_drops(t).len());
        for (f, w) in [(field(), wa), (walled.clone(), wb)] {
            for i in 0..60 {
                let q = [7.0 + i as f32 * 10.6, 13.0 + (i * 37 % 60) as f32 * 5.7];
                let gpu = gpu_ripple_water(w.slots[0].words, w.slots[0].place, &b.ripples, q);
                let cpu = f.sample_at(q[0], q[1], t);
                assert_eq!(gpu, cpu, "t {t} at {q:?}: the shader's arithmetic is the reference's, step for step");
                assert_eq!(gpu.height, f.height_at(q[0], q[1], t));
            }
        }
    }
}

#[test]
fn the_cap_counts_skipped_drops_and_reports_once() {
    //Arrange: 300 drops alive at once.
    let drops: Vec<RippleDrop> = (0..300).map(|i| RippleDrop { center: PointF::new((i % 20) as f32 * 30.0, (i / 20) as f32 * 20.0), time: 1.0 + i as f64 * 0.001, strength: 1.0, size: 1.0 }).collect();
    let many = RippleField::new(place(), RippleOptions::default()).unwrap().with_drops(drops).unwrap();
    let mut s = session();
    floor(&mut s, "A");
    s.get_layer_mut("A").unwrap().set_refraction(Some(WaterRefraction::new(WaterSource::Ripples(many)))).unwrap();
    s.set_time(2.0);
    //Act
    let b = build(&s);
    let again = build(&s);
    //Assert
    assert_eq!((b.stats.ripples_evaluated, b.stats.ripples_skipped), (256, 44));
    assert_eq!(again.stats.ripples_skipped, 44, "counted every frame");
    let reports = s.take_reports();
    assert_eq!(reports.len(), 1, "{reports:?}");
    assert!(reports[0].contains("MAX_LIVE_DROPS") && reports[0].contains("44"));
    assert!(s.take_reports().is_empty());
}

#[test]
fn a_pixels_water_point_and_offset_follow_the_layers_drawing_space() {
    //Arrange: a 1280 x 720 canvas (twice the calibration), a camera panned, zoomed and rolled, the
    // layer at parallax 0.5 and MOVED by its own transform (which must not move the water).
    let mut s = session();
    floor(&mut s, "Pool");
    let mut camera = DrawingCamera::new();
    camera.set_pan(PointF::new(30.0, -12.0)).unwrap();
    camera.set_zoom(1.4).unwrap();
    camera.set_rotation(20.0).unwrap();
    s.set_camera(Some(camera.clone()));
    let layer = s.get_layer_mut("Pool").unwrap();
    layer.set_parallax(0.5).unwrap();
    layer.set_transform(crate::models::DrawingTransform::translate(40.0, 25.0));
    layer.set_refraction(Some(WaterRefraction { strength: 9.0, ..WaterRefraction::new(WaterSource::Ripples(field())) })).unwrap();
    s.set_time(2.5);
    //Act
    let b = build_at(&s, &SessionRenderOptions::default(), 1280, 720);
    let fp = composite(&b, 0);
    //Assert
    let drawing = Affine::scale(2.0) * camera.get_matrix((640, 360), 0.5);
    for p in [[100.5f32, 80.5], [640.5, 360.5], [1100.5, 650.5]] {
        let q = gpu_water_point(&fp, p);
        let back = drawing * Point::new(q[0] as f64, q[1] as f64);
        assert!((back.x - p[0] as f64).abs() < 1e-3 && (back.y - p[1] as f64).abs() < 1e-3, "device -> water -> device: {back:?} for {p:?}");
        let slope = field().slope_at(q[0], q[1], 2.5);
        let offset = gpu_water_offset(&fp, [slope.0, slope.1], fp.v17[0], 1e9);
        let moved = drawing * Point::new((q[0] + 9.0 * slope.0) as f64, (q[1] + 9.0 * slope.1) as f64);
        assert!((moved.x - back.x - offset[0] as f64).abs() < 1e-3 && (moved.y - back.y - offset[1] as f64).abs() < 1e-3, "the offset is the water move seen on the device");
    }
    assert_eq!(fp.v17[0], 9.0);
    assert!(fp.v20[3] > 0.0 && fp.v20[3] <= MAX_WATER_OFFSET);
    let long = gpu_water_offset(&fp, [50.0, 0.0], 9.0, fp.v20[3]);
    assert!(((long[0] * long[0] + long[1] * long[1]).sqrt() - fp.v20[3]).abs() < 1e-3, "an offset is never longer than the grown reach");
}

#[test]
fn a_refraction_of_strength_0_without_shading_glints_or_caustics_leaves_the_picture_unchanged() {
    //Arrange
    let mut s = session();
    floor(&mut s, "Pool");
    s.get_layer_mut("Pool").unwrap().set_refraction(Some(WaterRefraction { strength: 0.0, shading: 0.0, ..WaterRefraction::new(WaterSource::Ripples(field())) })).unwrap();
    s.set_time(3.0);
    //Act
    let b = build(&s);
    let fp = composite(&b, 0);
    //Assert
    let w = b.effects[0].water.as_ref().unwrap();
    for p in [[0.5f32, 0.5], [320.5, 180.5], [639.5, 359.5], [211.5, 149.5]] {
        let q = gpu_water_point(&fp, p);
        let water = gpu_ripple_water(w.slots[0].words, w.slots[0].place, &b.ripples, q);
        let at = gpu_in_region(&fp, { let o = gpu_water_offset(&fp, water.slope, fp.v17[0], fp.v20[3]); [p[0] + o[0], p[1] + o[1]] });
        assert_eq!(at, p, "the content is read where it is");
        let c = [0.2, 0.31, 0.47, 0.8];
        assert_eq!(gpu_water_light(c, &water, q, &fp), c, "exactly the same colour");
    }
    assert_eq!(fp.v20[3], 0.0, "no offset: the region does not grow");
}

#[test]
fn shading_is_1_on_flat_water_and_brighter_on_a_slope_facing_the_light_and_glints_follow_the_formula() {
    //Arrange
    let mut s = session();
    floor(&mut s, "Pool");
    s.get_layer_mut("Pool").unwrap().set_refraction(Some(WaterRefraction { glints: Some(WaterGlints { amount: 0.8, sharpness: 20.0, color: Color::WHITE }), ..WaterRefraction::new(WaterSource::Ripples(field())) })).unwrap();
    let b = build(&s);
    let fp = composite(&b, 0);
    let flat = RippleSample::default();
    // The light comes from the upper left (225 degrees): water rising towards the lower right
    // (its surface turned towards the light) is lit, the other way shaded.
    let facing = RippleSample { slope: [0.3, 0.3], ..RippleSample::default() };
    let away = RippleSample { slope: [-0.3, -0.3], ..RippleSample::default() };
    let c = [0.3, 0.3, 0.3, 1.0];
    //Act
    let (lf, lt, la) = (gpu_water_light(c, &flat, [0.0, 0.0], &fp), gpu_water_light(c, &facing, [0.0, 0.0], &fp), gpu_water_light(c, &away, [0.0, 0.0], &fp));
    //Assert
    let l = crate::water::water_refraction::light_vector(225.0, 45.0);
    let glint_flat = 0.8 * l[2].powf(20.0);
    assert!((lf[0] - (0.3 + glint_flat)).abs() < 1e-6, "flat water: shading exactly 1 (and the flat glint, tiny): {lf:?}");
    assert!(lt[0] > 0.3 && la[0] < 0.3, "{lt:?} {la:?}");
    assert_eq!(fp.u4[0] & (WATER_REFRACTION | WATER_GLINTS), WATER_REFRACTION | WATER_GLINTS);
    // A glint where the slope reflects the light straight up: n halfway between l and (0, 0, 1).
    let h = normalize([l[0], l[1], l[2] + 1.0]);
    let mirror = RippleSample { slope: [-h[0] / h[2], -h[1] / h[2]], ..RippleSample::default() };
    let shine = gpu_water_light([0.0, 0.0, 0.0, 1.0], &mirror, [0.0, 0.0], &fp);
    assert!((shine[0] - 0.8).abs() < 1e-4, "amount x pow(1, sharpness): {shine:?}");
    assert_eq!(gpu_water_light([0.0, 0.0, 0.0, 0.0], &mirror, [0.0, 0.0], &fp), [0.0; 4], "no glint where nothing is drawn");
}

#[test]
fn the_caustic_light_lights_the_contents_colour_from_the_pattern_gathered_by_the_curvature() {
    //Arrange
    let caustics = CausticLight { strength: 1.0, focus: 0.0, ..CausticLight::default() };
    let mut s = session();
    floor(&mut s, "Pool");
    s.get_layer_mut("Pool").unwrap().set_refraction(Some(WaterRefraction { shading: 0.0, caustics: Some(caustics), ..WaterRefraction::new(WaterSource::Ripples(field())) })).unwrap();
    s.set_time(8.0);
    let fp = composite(&build(&s), 0);
    //Act / Assert
    assert_eq!((fp.v15[2], fp.v15[3]), (2.0, 0.02), "z = time x speed, the frequency");
    let flat = RippleSample::default();
    let mut lit = 0;
    for i in 0..400 {
        let q = [i as f32 * 3.1, (i * 7 % 400) as f32 * 2.3];
        let v = crate::planes::DrawingNoise::caustics((q[0] * 0.02) as f64, (q[1] * 0.02) as f64, 2.0, 0) as f32;
        let c = gpu_water_light([0.4, 0.4, 0.4, 1.0], &flat, q, &fp);
        let expect = 0.4 + 0.4 * fp.v20[0] * 0.5 * (v + 1.0);
        assert!((c[0] - expect.min(1.0)).abs() < 1e-5);
        if v > 0.5 {
            lit += 1;
        }
    }
    assert!(lit > 0, "the net's bright lines");
    assert_eq!(gpu_water_light([0.0, 0.0, 0.0, 1.0], &flat, [10.0, 10.0], &fp), [0.0, 0.0, 0.0, 1.0], "it lights the content's colour: black stays black");
}

#[test]
fn a_height_picture_is_decoded_from_its_red_channel_and_its_slope_comes_from_four_taps() {
    //Arrange: a 64 x 32 picture of the plane h = 0.004 x (byte = 128 + x * 2 per pixel, clamped).
    let (w, h) = (64u32, 32u32);
    let mut rgba = Vec::new();
    for _y in 0..h {
        for x in 0..w {
            rgba.extend_from_slice(&[(64 + 2 * x) as u8, 0, 0, 255]);
        }
    }
    let picture = ImageSource::from_rgba(&rgba, w, h).unwrap();
    let place = RectangleF::new(0.0, 0.0, 320.0, 160.0);
    let mut s = session();
    floor(&mut s, "Pool");
    s.get_layer_mut("Pool").unwrap().set_refraction(Some(WaterRefraction::new(WaterSource::Picture { picture: picture.clone(), place, height: 5.0 }))).unwrap();
    //Act
    let b = build(&s);
    let slot = b.effects[0].water.as_ref().unwrap().slots[0];
    //Assert
    assert_eq!((slot.kind, slot.scale, slot.bias), (2, 10.0, 0.5), "byte 128 = still, 0 and 255 = -height and +height");
    let red = |i: i32, _j: i32| (64 + 2 * i) as f32 / 255.0;
    let decode = |byte: f32| (byte / 255.0 - slot.bias) * slot.scale;
    assert!(decode(128.0).abs() < 0.04 && (decode(255.0) - 5.0).abs() < 1e-5 && (decode(0.0) + 5.0).abs() < 1e-5);
    let q = [100.0, 80.0];
    let water = gpu_picture_water(&red, (w, h), slot.place, slot.scale, slot.bias, q);
    // Per texel (5 units) the byte rises by 2: dh/dx = 2 / 255 x 10 / 5.
    assert!((water.slope[0] - 2.0 / 255.0 * 10.0 / 5.0).abs() < 1e-4 && water.slope[1].abs() < 1e-6, "{water:?}");
    assert!(water.focus.abs() < 1e-4, "a plane does not focus light");
    assert!(matches!(b.effects[0].water.as_ref().unwrap().textures[0], Some(crate::rendering::TestWaterBinding::Image(1))), "uploaded as one of the frame's pictures");
    // An application texture's red is read as stored (0 = still), a picture target's like a picture.
    let target = PictureTarget::new(32, 32).unwrap();
    let tap = crate::water::water_source::WaterTap::new(&WaterSource::Target { picture: target, place, height: 2.0 }, 0.0);
    assert!(matches!(tap, crate::water::water_source::WaterTap::Picture { scale, bias, .. } if scale == 4.0 && bias == 0.5));
}

#[test]
fn a_reflection_mirrors_device_points_about_the_waterline_and_fades_with_depth() {
    //Arrange: a level line at y 200 (device = calibrated here), and the camera rolled 30 degrees.
    let level = ReflectionParams { point: [0.0, 200.0], normal: [0.0, 1.0], gap: 0.0, opacity: 0.6, fade: 300.0, streak: 0.0, water_strength: 0.0, tint: [1.0; 4], reach: 0.0, has_water: false, same: false };
    //Act / Assert
    assert_eq!(level.mirror([50.0, 260.0]), [50.0, 140.0]);
    assert_eq!(level.mirror(level.mirror([12.0, 345.0])), [12.0, 345.0]);
    let gapped = ReflectionParams { gap: 10.0, ..level };
    assert_eq!(gapped.mirror([50.0, 260.0]), [50.0, 160.0], "the mirror line sits gap lower");
    assert_eq!(level.fade_at(-5.0), 0.0, "nothing above the line");
    assert!((level.fade_at(1e-3) - 0.6).abs() < 1e-4, "opacity at the line");
    assert!((level.fade_at(150.0) - 0.3).abs() < 1e-6, "half way: half");
    assert_eq!(level.fade_at(300.0), 0.0, "gone at the fade distance");
    // The same through a session: the line under a rolled camera is turned with it.
    let mut s = session();
    bank(&mut s, "Bank");
    let mut camera = DrawingCamera::new();
    camera.set_rotation(30.0).unwrap();
    s.set_camera(Some(camera.clone()));
    s.get_layer_mut("Bank").unwrap().set_reflection(Some(WaterReflection { gap: 5.0, streak: 4.0, ..WaterReflection::new(200.0) })).unwrap();
    let b = build(&s);
    let fp = composite(&b, 0);
    let r = b.effects[0].params.water.unwrap().reflection.unwrap();
    let drawing = camera.get_matrix((640, 360), 1.0);
    let on_line = drawing * Point::new(77.0, 200.0);
    assert!(r.depth([on_line.x, on_line.y]).abs() < 1e-3, "a point of the line is on the device line");
    let below = drawing * Point::new(77.0, 260.0);
    let mirrored = drawing * Point::new(77.0, 150.0);
    let m = r.mirror([below.x, below.y]);
    assert!((m[0] - mirrored.x).abs() < 1e-3 && (m[1] - mirrored.y).abs() < 1e-3, "mirrored about y = 205 in the drawing: {m:?} vs {mirrored:?}");
    // The shader's taps: the middle two straddle the mirrored point by the streak.
    let (t0, weight) = gpu_reflection_tap(&fp, [below.x as f32, below.y as f32], [0.0, 0.0], false, 0);
    let (t5, _) = gpu_reflection_tap(&fp, [below.x as f32, below.y as f32], [0.0, 0.0], false, 5);
    assert!(weight > 0.0 && weight < 0.6);
    let span = ((t5[0] - t0[0]).powi(2) + (t5[1] - t0[1]).powi(2)).sqrt();
    assert!((span - 4.0).abs() < 1e-3, "the streak spans 4 units: {span}");
    assert_eq!(fp.u4[0] & WATER_REFLECTION, WATER_REFLECTION);
}

#[test]
fn the_region_grows_by_the_refractions_reach_and_the_mirrored_content_box() {
    //Arrange
    let mut still = session();
    bank(&mut still, "Bank");
    let mut refracted = still.clone();
    refracted.get_layer_mut("Bank").unwrap().set_refraction(Some(WaterRefraction::new(WaterSource::Ripples(field())))).unwrap();
    refracted.set_time(2.0);
    let mut reflected = still.clone();
    reflected.get_layer_mut("Bank").unwrap().set_reflection(Some(WaterReflection { streak: 0.0, ..WaterReflection::new(200.0) })).unwrap();
    let mut below = session();
    below.add_layer("Low", Color::WHITE).add_shape(RectangleShape::new(100.0, 250.0, 200.0, 50.0, RectangleShapeOptions { is_filled: true, ..RectangleShapeOptions::default() }).unwrap());
    below.get_layer_mut("Low").unwrap().set_reflection(Some(WaterReflection::new(200.0))).unwrap();
    //Act
    let r = build(&refracted);
    let m = build(&reflected);
    let low = build(&below);
    //Assert
    let content = r.effects[0].content;
    let reach = r.effects[0].params.water.unwrap().refraction.unwrap().reach as f64;
    assert!(reach > 1.0 && reach <= MAX_WATER_OFFSET as f64, "{reach}");
    assert!(r.effects[0].region.contains_rect(content.inflate(reach.floor(), reach.floor()).intersect(Rect::new(0.0, 0.0, 640.0, 360.0))), "{:?} {:?}", r.effects[0].region, content);
    // The bank (y 60 .. 180) mirrored about 200: y 220 .. 340.
    let region = m.effects[0].region;
    assert!(region.y1 >= 340.0 && region.y1 <= 342.0 && region.y0 <= 60.0, "{region:?}");
    let bank_box = m.effects[0].content;
    assert_eq!((region.x0, region.x1), ((bank_box.x0 - 1.0).floor(), (bank_box.x1 + 1.0).ceil()), "the mirrored box (grown one pixel) - no wider: {bank_box:?}");
    let lone = &low.effects[0];
    assert_eq!(lone.region, lone.content, "content wholly below the line: nothing to mirror");
    // Clipped to the canvas.
    let mut tall = session();
    tall.add_layer("Tall", Color::WHITE).add_shape(RectangleShape::new(10.0, 0.0, 50.0, 190.0, RectangleShapeOptions { is_filled: true, ..RectangleShapeOptions::default() }).unwrap());
    tall.get_layer_mut("Tall").unwrap().set_reflection(Some(WaterReflection::new(190.0))).unwrap();
    assert_eq!(build(&tall).effects[0].region.y1, 360.0);
}

#[test]
fn refraction_and_reflection_ride_the_layers_one_effect_group() {
    //Arrange
    let mut s = session();
    bank(&mut s, "Bank");
    let plain = kinds(&s, &SessionRenderOptions::default());
    s.get_layer_mut("Bank").unwrap().set_refraction(Some(WaterRefraction::new(WaterSource::Ripples(field())))).unwrap();
    let only = kinds(&s, &SessionRenderOptions::default());
    let layer = s.get_layer_mut("Bank").unwrap();
    layer.set_effects(Some(DrawingEffects::default().with_blur_radius(4.0).unwrap()));
    layer.set_reflection(Some(WaterReflection { water: Some(WaterSource::Ripples(field())), ..WaterReflection::new(200.0) })).unwrap();
    layer.set_surface_finish(Some(SurfaceFinish::default())).unwrap();
    //Act
    let all = build(&s);
    //Assert
    assert_eq!(plain, "LFP", "no water: the plain layer group (folded)");
    assert_eq!(only, "EFP", "refraction alone: one effect group");
    assert_eq!((all.stats.effects, all.stats.offscreen), (1, 1), "blur, finish, refraction and reflection: ONE group");
    let p = all.effects[0].params;
    assert!(p.blur_sigma > 0.0 && p.finish.is_some() && p.water.unwrap().refraction.is_some());
    let r = p.water.unwrap().reflection.unwrap();
    assert!(r.same && r.has_water, "the same field: evaluated once (slot A)");
    let fp = composite(&all, 0);
    assert_eq!(fp.u4[0] & (WATER_REFRACTION | WATER_REFLECTION | WATER_SAME), WATER_REFRACTION | WATER_REFLECTION | WATER_SAME);
    assert_eq!((fp.u4[1], fp.u4[2]), (1, 0), "slot A ripples; slot B none (it is slot A's)");
    assert_eq!(all.stats.ripples_evaluated, field().live_drops(0.0).len());
}

#[test]
fn a_session_refraction_is_one_group_and_the_blends_child_two_three_inside_a_wipe() {
    //Arrange
    let mut a = session();
    bank(&mut a, "A");
    let mut b = session();
    floor(&mut b, "Paint");
    b.get_layer_mut("Paint").unwrap().set_surface_finish(Some(SurfaceFinish::default())).unwrap();
    b.get_layer_mut("Paint").unwrap().set_refraction(Some(WaterRefraction::new(WaterSource::Ripples(field())))).unwrap();
    let water = Some(WaterRefraction::new(WaterSource::Ripples(field())));
    let session_water = SessionRenderOptions { refraction: water.clone(), ..SessionRenderOptions::default() };
    let multiply = SessionRenderOptions { blend: SessionBlendMode::Multiply, refraction: water, ..SessionRenderOptions::default() };
    //Act
    let one = kinds(&a, &session_water);
    let two = kinds(&a, &multiply);
    let wipe = SessionWipe::new(0.5, WipeEdge::Line { angle_degrees: 0.0 }, 20.0);
    let plan = |options: &SessionRenderOptions| {
        let mut scene = VectorScene::new();
        wipe.render(&mut scene, RectangleF::new(0.0, 0.0, 640.0, 360.0), |sc| a.render(sc, 640, 360), |sc| b.render_with(sc, 640, 360, options));
        let mut builder = Builder::default();
        builder.build(&scene, 640, 360);
        builder
    };
    let inside = plan(&SessionRenderOptions { blend: SessionBlendMode::Multiply, ..SessionRenderOptions::default() });
    let past = plan(&multiply);
    //Assert
    assert_eq!(one, "ELFPP", "the session's picture through one effect group");
    assert_eq!(two, "LELFPPP", "inside the blend's group");
    assert_eq!(inside.levels_needed, 4, "a refracted, finished layer in a blended session inside a wipe: three levels");
    assert_eq!(inside.stats.unsupported, 0);
    assert!(inside.effects.iter().any(|e| e.params.finish.is_some() && e.params.water.is_some()), "finish and refraction share the layer's group");
    assert!(past.stats.unsupported >= 1, "a session refraction there too is one level past the limit");
}

#[test]
fn a_checkpointed_set_after_a_live_water_source_never_restores_a_stale_frame() {
    //Arrange: a refracted floor, then (on a later layer) a checkpointed stroke set.
    let set = BrushStrokeSet::new((0..20).map(|i| BrushStrokeRecord::new(PointF::new(40.0 + i as f32 * 25.0, 100.0), 40.0, 10.0, 0.0, Color::from_rgb(200, 80, 40))).collect()).unwrap();
    let painting = BrushStrokesShape::new(set, BrushStrokesOptions::default()).unwrap().with_checkpoints(Some(StrokeCheckpoints::default())).unwrap();
    let with = |source: WaterSource| {
        let mut s = session();
        floor(&mut s, "Pool");
        s.get_layer_mut("Pool").unwrap().set_refraction(Some(WaterRefraction::new(source))).unwrap();
        s.add_layer("Paint", Color::WHITE).add_shape(painting.clone());
        s
    };
    let key = |b: &Builder| b.stroke_ops[0].checkpoint.unwrap().key;
    let two_frames = |s: &DrawingSession| {
        let mut scene = VectorScene::new();
        s.render(&mut scene, 640, 360);
        let mut b = Builder::default();
        b.build(&scene, 640, 360);
        let first = key(&b);
        b.build(&scene, 640, 360);
        (first, key(&b))
    };
    let mut rippled = with(WaterSource::Ripples(field()));
    let target = with(WaterSource::Target { picture: PictureTarget::new(64, 64).unwrap(), place: place(), height: 1.0 });
    //Act
    rippled.set_time(4.0);
    let (r1, r2) = two_frames(&rippled);
    rippled.set_time(4.5);
    let (r3, _) = two_frames(&rippled);
    let (t1, t2) = two_frames(&target);
    //Assert
    assert_eq!(r1, r2, "ripples at the same time: the same frame below the set, the same key");
    assert_ne!(r1, r3, "another time: other water, another key");
    assert_ne!(t1, t2, "a picture target the CPU cannot name: volatile, a new key every frame");
}

#[test]
fn the_water_stays_with_effects_off_and_under_the_line_look() {
    //Arrange
    let mut s = session();
    bank(&mut s, "Bank");
    let layer = s.get_layer_mut("Bank").unwrap();
    layer.set_effects(Some(DrawingEffects::default().with_blur_radius(4.0).unwrap()));
    layer.set_refraction(Some(WaterRefraction::new(WaterSource::Ripples(field())))).unwrap();
    layer.set_reflection(Some(WaterReflection::new(200.0))).unwrap();
    //Act
    let off = build_at(&s, &SessionRenderOptions { effects: false, ..SessionRenderOptions::default() }, 640, 360);
    let line = build_at(&s, &SessionRenderOptions { paint: SessionPaint::Line(crate::LinePaint::new(crate::paths::PathStrokeStyle::new(2.0, crate::paths::PathStrokeStyleOptions::default()).unwrap(), Color::BLACK)), ..SessionRenderOptions::default() }, 640, 360);
    //Assert
    assert_eq!(off.effects[0].params.blur_sigma, 0.0, "the blur goes");
    assert!(off.effects[0].params.water.is_some(), "the water stays");
    assert!(line.effects[0].params.water.is_some());
}

#[test]
fn the_pass_data_grows_by_the_water_vectors_after_the_finish() {
    //Arrange / Act / Assert
    assert_eq!(std::mem::size_of::<FxPassData>(), 544, "34 vec4");
    assert_eq!(std::mem::offset_of!(FxPassData, u2), 256);
    assert_eq!(std::mem::offset_of!(FxPassData, v14), 272);
    assert_eq!(std::mem::offset_of!(FxPassData, v27), 480);
    assert_eq!(std::mem::offset_of!(FxPassData, u3), 496);
    assert_eq!(std::mem::offset_of!(FxPassData, u5), 528);
    let mut s = session();
    floor(&mut s, "Pool");
    let caustics = CausticLight { seed: 7, ..CausticLight::default() };
    s.get_layer_mut("Pool").unwrap().set_refraction(Some(WaterRefraction { caustics: Some(caustics), ..WaterRefraction::new(WaterSource::Ripples(field())) })).unwrap();
    let fp = composite(&build(&s), 0);
    assert_eq!(fp.u4[0], WATER_REFRACTION | WATER_CAUSTICS);
    assert_eq!(fp.u4[3], 7);
    assert_eq!(fp.v21, [0.0, 0.0, 640.0, 360.0], "slot A's place");
    assert_eq!(fp.v17[..2], [12.0, 0.3]);
    let plain = composite(&build(&{
        let mut s = session();
        floor(&mut s, "Pool");
        s.get_layer_mut("Pool").unwrap().set_effects(Some(DrawingEffects::default().with_blur_radius(2.0).unwrap()));
        s
    }), 0);
    assert_eq!((plain.u4, plain.v14, plain.v23), ([0; 4], [0.0; 4], [0.0; 4]), "a group without water: no water words");
}

#[test]
fn refractions_and_reflections_are_checked_and_a_layer_keeps_its_own_when_a_new_one_is_unusable() {
    //Arrange
    let mut s = session();
    let layer = s.add_layer("L", Color::WHITE);
    let good = WaterRefraction::new(WaterSource::Ripples(field()));
    //Act / Assert
    layer.set_refraction(Some(good.clone())).unwrap();
    assert!(layer.set_refraction(Some(WaterRefraction { light_elevation: 0.0, ..good.clone() })).unwrap_err().contains("light_elevation"));
    assert!(layer.set_refraction(Some(WaterRefraction { strength: f32::NAN, ..good.clone() })).unwrap_err().contains("strength"));
    assert!(layer.set_refraction(Some(WaterRefraction { glints: Some(WaterGlints { sharpness: 0.0, ..WaterGlints::default() }), ..good.clone() })).unwrap_err().contains("sharpness"));
    assert!(layer.set_refraction(Some(WaterRefraction { caustics: Some(CausticLight { frequency: 0.0, ..CausticLight::default() }), ..good.clone() })).unwrap_err().contains("frequency"));
    let picture = ImageSource::from_rgba(&[128, 0, 0, 255], 1, 1).unwrap();
    assert!(layer.set_refraction(Some(WaterRefraction::new(WaterSource::Picture { picture, place: RectangleF::new(0.0, 0.0, 0.0, 10.0), height: 1.0 }))).unwrap_err().contains("place"));
    assert_eq!(layer.refraction(), Some(&good), "kept");
    layer.set_reflection(Some(WaterReflection::new(100.0))).unwrap();
    assert!(layer.set_reflection(Some(WaterReflection { opacity: 1.5, ..WaterReflection::new(100.0) })).unwrap_err().contains("opacity"));
    assert!(layer.set_reflection(Some(WaterReflection { fade_distance: 0.0, ..WaterReflection::new(100.0) })).unwrap_err().contains("fade_distance"));
    assert!(layer.set_reflection(Some(WaterReflection { streak: -1.0, ..WaterReflection::new(100.0) })).unwrap_err().contains("streak"));
    assert_eq!(layer.reflection().map(|r| r.line), Some(100.0));
    assert!(WaterSource::Ripples(field()).is_exact_in_time());
    // A session refraction that is unusable is not drawn and is reported once.
    let mut s = session();
    bank(&mut s, "B");
    let bad = SessionRenderOptions { refraction: Some(WaterRefraction { shading: f32::INFINITY, ..good }), ..SessionRenderOptions::default() };
    assert_eq!(kinds(&s, &bad), "LFP");
    assert_eq!(s.take_reports().len(), 1);
}
