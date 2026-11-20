//! BIT-FOR-BIT parity with NoSkia: every case builds the same inputs as a reference console run
//! against CodeBrix.Imaging.Drawing.NoSkia (the package's own DLLs; numbers only, no images) and
//! asserts the FNV-1a 64 hash of the bits of every value NoSkia produced. A hash that differs
//! means at least one value differs in at least one bit. The math-library cases (blur kernels'
//! exp, gamma's powf, particles' cos / sin / sqrt) hold on the host; a browser's math library may
//! differ in the last bit.

use crate::planes::{
    DrawingFrameAccumulator, DrawingMotionBlur, DrawingNoise, DrawingNoiseFillOptions, DrawingNoiseFractalOptions, DrawingNoiseKind, DrawingParticleEmitter, DrawingParticleSystem, DrawingPlane, DrawingPlaneBloomOptions, DrawingPlaneImage,
    DrawingPlaneResampling, DrawingRandom, DrawingTransition, DrawingTransitionDirection,
};
use crate::{Color, Rectangle};

struct Fnv(u64);

impl Fnv {
    fn new() -> Fnv {
        Fnv(14695981039346656037)
    }
    fn b(&mut self, b: u8) {
        self.0 ^= b as u64;
        self.0 = self.0.wrapping_mul(1099511628211);
    }
    fn u32(&mut self, v: u32) {
        v.to_le_bytes().iter().for_each(|b| self.b(*b));
    }
    fn u64(&mut self, v: u64) {
        v.to_le_bytes().iter().for_each(|b| self.b(*b));
    }
    fn f(&mut self, v: f32) {
        self.u32(v.to_bits());
    }
    fn d(&mut self, v: f64) {
        self.u64(v.to_bits());
    }
    fn plane(&mut self, p: &DrawingPlane) {
        self.u32(p.width());
        self.u32(p.height());
        p.data().iter().for_each(|v| self.f(*v));
    }
    fn image(&mut self, i: &DrawingPlaneImage) {
        self.plane(i.r());
        self.plane(i.g());
        self.plane(i.b());
        self.plane(i.a());
    }
    fn bytes(&mut self, bytes: &[u8]) {
        bytes.iter().for_each(|b| self.b(*b));
    }
}

fn check(name: &str, h: &Fnv, expected: u64) {
    assert_eq!(h.0, expected, "{name}: 0x{:016X} instead of NoSkia's 0x{expected:016X}", h.0);
}

const W: u32 = 37;
const H: u32 = 23;

fn rand_plane(seed: i64, w: u32, h: u32, lo: f32, hi: f32) -> DrawingPlane {
    let mut r = DrawingRandom::new(seed);
    let mut p = DrawingPlane::new(w, h).unwrap();
    p.data_mut().iter_mut().for_each(|v| *v = lo + ((hi - lo) * r.next_single()));
    p
}

fn alpha_plane(seed: i64, w: u32, h: u32) -> DrawingPlane {
    let mut r = DrawingRandom::new(seed);
    let mut p = DrawingPlane::new(w, h).unwrap();
    p.data_mut().iter_mut().for_each(|v| {
        let u = r.next_single();
        *v = if u < 0.2 {
            0.0
        } else if u > 0.8 {
            1.0
        } else {
            u
        };
    });
    p
}

fn rand_image(seed: i64, w: u32, h: u32, opaque: bool) -> DrawingPlaneImage {
    let alpha = if opaque { DrawingPlane::with_value(w, h, 1.0).unwrap() } else { alpha_plane(seed + 3, w, h) };
    DrawingPlaneImage::from_planes(rand_plane(seed, w, h, 0.0, 1.5), rand_plane(seed + 1, w, h, 0.0, 1.0), rand_plane(seed + 2, w, h, 0.0, 1.0), alpha).unwrap()
}

fn rand_bytes(seed: i64, count: usize, opaque: bool) -> Vec<u8> {
    let mut r = DrawingRandom::new(seed);
    let mut bytes = vec![0u8; count * 4];
    for i in 0..count {
        bytes[i * 4] = r.next_int(256).unwrap() as u8;
        bytes[i * 4 + 1] = r.next_int(256).unwrap() as u8;
        bytes[i * 4 + 2] = r.next_int(256).unwrap() as u8;
        let a = r.next_int(4).unwrap();
        bytes[i * 4 + 3] = if opaque {
            255
        } else if a == 0 {
            0
        } else if a == 1 {
            255
        } else {
            r.next_int(256).unwrap() as u8
        };
    }
    bytes
}

fn inputs() -> (DrawingPlane, DrawingPlane, DrawingPlane) {
    (rand_plane(1, W, H, -0.5, 1.5), rand_plane(2, W, H, -1.0, 2.0), rand_plane(3, W, H, 0.0, 1.0))
}

#[test]
fn plane_arithmetic_is_noskias_bit_for_bit() {
    let (p, q, m) = inputs();
    let mut h = Fnv::new();
    h.plane(&p);
    h.plane(&q);
    h.plane(&m);
    check("inputs", &h, 0xF353C0D08A8E205A);

    let mut h = Fnv::new();
    let mut c = p.clone();
    c.add(0.25).add_plane(&q, Some(1.5)).unwrap().subtract(&q).unwrap().multiply(0.7).multiply_plane(&q).unwrap();
    h.plane(&c);
    check("arith", &h, 0x38AD3D3E7E3F8EC4);

    let mut h = Fnv::new();
    h.plane(p.clone().lerp(&q, 0.3).unwrap());
    h.plane(p.clone().lerp_mask(&q, &m).unwrap());
    check("lerp", &h, 0xD2AE39702A6A8ED4);

    let mut h = Fnv::new();
    h.plane(p.clone().min(0.8));
    h.plane(p.clone().max(-0.1));
    h.plane(p.clone().min_plane(&q).unwrap());
    h.plane(p.clone().max_plane(&q).unwrap());
    h.plane(p.clone().clamp(0.0, 1.0).unwrap());
    check("minmax", &h, 0xD3B287DF536BA73A);

    let mut h = Fnv::new();
    h.plane(p.clone().apply_tone_curve(0.15, 1.08));
    h.plane(p.clone().apply_gamma(1.0 / 2.2).unwrap());
    h.plane(p.clone().apply(|v| (v * v) - 0.5));
    check("tone_gamma", &h, 0xE65E18AAF88478FB);

    let mut h = Fnv::new();
    h.plane(p.clone().apply_vignette(18.5, 11.0, 20.0, 13.0, 0.5, Some(0.2)).unwrap());
    h.plane(&DrawingPlane::create_vignette(W, H, 10.25, 3.0, 9.0, 30.0, 0.9, None).unwrap());
    check("vignette", &h, 0x27FCBCF6C4BE9DE0);

    let mut h = Fnv::new();
    h.bytes(&p.to_bytes());
    h.bytes(&DrawingPlane::from_data(8, 1, vec![-0.1, 0.0, 0.5, 0.498, 1.0, 1.5, f32::NAN, 1.0 / 255.0]).unwrap().to_bytes());
    check("to_bytes", &h, 0xFC3864DD8CBED50F);
}

#[test]
fn plane_filters_are_noskias_bit_for_bit() {
    let (p, _, _) = inputs();
    for (sigma, expected) in [(0.35f32, 0xC4B7563C12400BD7u64), (1.5, 0xDC9B52C3301FE0A4), (3.7, 0x24E5B4EED01D6EA1), (12.0, 0xF5008DDA2CB496FD)] {
        let mut h = Fnv::new();
        h.plane(p.clone().gaussian_blur(sigma).unwrap());
        check(&format!("blur_{sigma}"), &h, expected);
    }

    let mut h = Fnv::new();
    for mode in [DrawingPlaneResampling::Bilinear, DrawingPlaneResampling::Nearest, DrawingPlaneResampling::BilinearAlignedCorners] {
        h.plane(&p.resized(80, 50, Some(mode)).unwrap());
        h.plane(&p.resized(9, 7, Some(mode)).unwrap());
        h.plane(&DrawingPlane::with_value(1, 1, 0.7).unwrap().resized(3, 2, Some(mode)).unwrap());
        h.plane(&p.resized(37, 1, Some(mode)).unwrap());
    }
    check("resize", &h, 0x576E58E219486E96);

    let mut h = Fnv::new();
    for factor in [1, 2, 4, 5] {
        h.plane(&p.downsampled(factor).unwrap());
    }
    check("downsample", &h, 0x138F403D931E7E02);

    let mut h = Fnv::new();
    h.plane(&p.create_bloom(4, &[(1.5, 0.8), (4.0, 0.9)], DrawingPlaneBloomOptions { gain: 0.6, output_size: None, upsampling: DrawingPlaneResampling::BilinearAlignedCorners }).unwrap());
    h.plane(&p.create_bloom(2, &[(2.0, 1.0)], DrawingPlaneBloomOptions { output_size: Some((70, 40)), ..DrawingPlaneBloomOptions::default() }).unwrap());
    check("bloom", &h, 0xC786DA5AC972F04B);
}

#[test]
fn plane_images_are_noskias_bit_for_bit() {
    let (_, _, m) = inputs();
    let a = rand_image(10, W, H, false);
    let b = rand_image(20, W, H, false);
    let o = rand_image(30, W, H, true);

    let mut h = Fnv::new();
    let mut c = a.clone();
    c.add_image(&b, Some(0.5)).unwrap().add(0.1, -0.2, 0.3).multiply(0.9).multiply_rgb(1.1, 0.8, 1.2).multiply_plane(&m).unwrap().multiply_image(&b).unwrap();
    h.image(&c);
    check("img_arith", &h, 0xBFC47E97E588518E);

    let mut h = Fnv::new();
    h.image(a.clone().min(0.9).max(0.05));
    h.image(a.clone().clamp(0.1, 0.7).unwrap());
    h.image(a.clone().apply_tone_curve(0.2, 1.1).apply_gamma(0.8).unwrap());
    h.image(a.clone().apply_vignette(18.0, 11.0, 25.0, 15.0, 0.6, Some(0.1)).unwrap());
    check("img_colour", &h, 0x0D663A54A3BD0F59);

    let mut h = Fnv::new();
    h.image(a.clone().lerp(&b, 0.3).unwrap());
    h.image(a.clone().lerp_mask(&b, &m).unwrap());
    h.image(o.clone().lerp(&a, 0.45).unwrap());
    check("img_lerp", &h, 0x90AE413899BE683A);

    let mut h = Fnv::new();
    h.image(a.clone().composite_over(&b, Some(0.8)).unwrap());
    h.image(a.clone().composite_over(&b, None).unwrap());
    h.image(a.clone().composite_add(&b, Some(0.7)).unwrap());
    check("img_composite", &h, 0x7092B2557F41AB8E);

    let mut h = Fnv::new();
    h.image(a.clone().blend_color(Color::from_rgba(200, 100, 50, 180), &m).unwrap());
    h.image(a.clone().blend_color_values(0.3, 1.2, 0.1, 0.9, &m).unwrap());
    h.image(a.clone().blend_image(&b, &m).unwrap());
    check("img_blend", &h, 0x1DE6A47F81CD38B0);

    let mut h = Fnv::new();
    h.image(a.clone().gaussian_blur(2.5).unwrap());
    h.image(o.clone().gaussian_blur(2.5).unwrap());
    check("img_blur", &h, 0x871C82FADADD534F);

    let mut h = Fnv::new();
    h.image(&a.resized(50, 31, None).unwrap());
    h.image(&a.resized(50, 31, Some(DrawingPlaneResampling::Nearest)).unwrap());
    h.image(&o.resized(20, 12, Some(DrawingPlaneResampling::BilinearAlignedCorners)).unwrap());
    h.image(&a.downsampled(3).unwrap());
    h.image(&o.downsampled(3).unwrap());
    check("img_resample", &h, 0x5B5617073B844271);

    let mut h = Fnv::new();
    let blurs = [(1.5f32, 0.8f32), (3.0, 0.5)];
    h.image(&a.create_bloom(2, &blurs, DrawingPlaneBloomOptions { gain: 0.6, ..DrawingPlaneBloomOptions::default() }).unwrap());
    h.image(&o.create_bloom(2, &blurs, DrawingPlaneBloomOptions { gain: 0.6, output_size: Some((40, 30)), ..DrawingPlaneBloomOptions::default() }).unwrap());
    check("img_bloom", &h, 0x2A6D73F7A5E99B3E);

    let mut h = Fnv::new();
    h.bytes(&a.to_rgba());
    h.image(&DrawingPlaneImage::from_rgba(&rand_bytes(5, (W * H) as usize, false), W, H).unwrap());
    check("img_rgba", &h, 0xDB6755202326782A);

    let mut h = Fnv::new();
    let mut filled = DrawingPlaneImage::new(W, H).unwrap();
    filled.fill_color(Color::from_rgba(10, 128, 255, 64));
    filled.fill_area(Rectangle::new(5, 3, 100, 4), Color::from_rgb(1, 2, 3));
    filled.fill_area_with(Rectangle::new(-3, 10, 8, 30), |x, y| [x as f32 * 0.1, y as f32 * 0.01, 0.5, 0.75]);
    h.image(&filled);
    check("img_fill", &h, 0x852288794269C3FA);
}

#[test]
fn transitions_are_noskias_bit_for_bit() {
    let a = rand_image(10, W, H, false);
    let b = rand_image(20, W, H, false);
    let o = rand_image(30, W, H, true);
    let count = (W * H) as usize;
    let (fa, fb) = (rand_bytes(40, count, false), rand_bytes(41, count, false));
    let (oa, ob) = (rand_bytes(42, count, true), rand_bytes(43, count, true));
    let mut transitions = vec![("dissolve", DrawingTransition::dissolve(), 0x82701FF64440750Cu64), ("fade", DrawingTransition::fade_through_color(Color::from_rgba(40, 200, 90, 160)), 0xB997267A73DD134B)];
    let expected = [
        (DrawingTransitionDirection::Left, [0x37B656EB4F327DAFu64, 0x70A2508A5195E17A, 0xC3E3755973C02AF0, 0x7C105F0E3C04F25A]),
        (DrawingTransitionDirection::Right, [0x90F2F9F06B5C6A62, 0xEC6053EDDF9FC865, 0xB07461FC9F79F5F5, 0x977DB5CD8742311F]),
        (DrawingTransitionDirection::Up, [0x414045E96FC47881, 0xB67D3B6C5CCDE5B7, 0xAC2221B89C27CC9F, 0x68018E80330689B6]),
        (DrawingTransitionDirection::Down, [0x1F591A5418A5AF9A, 0xE8F160B3A9170289, 0xEC2D22A264B1A82C, 0xAF54D434EE5A839D]),
    ];
    for (d, hashes) in expected {
        transitions.push(("wipe", DrawingTransition::wipe(d, None).unwrap(), hashes[0]));
        transitions.push(("softwipe", DrawingTransition::wipe(d, Some(5.0)).unwrap(), hashes[1]));
        transitions.push(("slide", DrawingTransition::slide(d), hashes[2]));
        transitions.push(("push", DrawingTransition::push(d), hashes[3]));
    }
    for (name, t, expected) in transitions {
        let mut h = Fnv::new();
        for f in [-0.5f32, 0.0, 0.37, 0.5, 0.81, 1.0] {
            let mut d1 = vec![0u8; count * 4];
            t.apply_bytes(&fa, &fb, &mut d1, W, H, f).unwrap();
            h.bytes(&d1);
            let mut d2 = vec![0u8; count * 4];
            t.apply_bytes(&oa, &ob, &mut d2, W, H, f).unwrap();
            h.bytes(&d2);
            h.image(&t.apply(&a, &b, f).unwrap());
            h.image(&t.apply(&o, &b, f).unwrap());
        }
        check(&format!("tr_{name}_{:?}", t.direction()), &h, expected);
    }
}

#[test]
fn accumulation_and_motion_blur_are_noskias_bit_for_bit() {
    let a = rand_image(10, W, H, false);
    let b = rand_image(20, W, H, false);
    let o = rand_image(30, W, H, true);
    let count = (W * H) as usize;
    let (fa, fb) = (rand_bytes(40, count, false), rand_bytes(41, count, false));
    let (oa, ob) = (rand_bytes(42, count, true), rand_bytes(43, count, true));
    let mut h = Fnv::new();
    let mut acc = DrawingFrameAccumulator::new(W, H).unwrap();
    acc.add_rgba(&fa).unwrap();
    acc.add_rgba(&fb).unwrap();
    acc.add_rgba(&oa).unwrap();
    h.bytes(&acc.get_average().unwrap());
    h.image(&acc.get_average_image().unwrap());
    acc.reset();
    acc.add_image(&a).unwrap();
    acc.add_image(&b).unwrap();
    acc.add_image(&o).unwrap();
    h.bytes(&acc.get_average().unwrap());
    h.image(&acc.get_average_image().unwrap());
    h.image(&DrawingMotionBlur::average(&[a.clone(), b.clone()]).unwrap());
    h.bytes(&DrawingMotionBlur::average_bytes(&[&fa, &ob], W, H).unwrap());
    DrawingMotionBlur::get_sample_times(2.0, 1.0 / 60.0, 5, Some(0.75)).unwrap().iter().for_each(|t| h.d(*t));
    DrawingMotionBlur::get_sample_times(0.1, 1.0 / 24.0, 3, None).unwrap().iter().for_each(|t| h.d(*t));
    check("accumulate", &h, 0x0BAEA85CCF60A57C);
}

#[test]
fn noise_is_noskias_bit_for_bit() {
    let mut h = Fnv::new();
    let mut nr = DrawingRandom::new(77);
    let five = DrawingNoiseFractalOptions { octaves: 5, lacunarity: 2.1, gain: 0.45 };
    for _ in 0..2000 {
        let (x, y, z) = (nr.uniform(-40.0, 40.0), nr.uniform(-40.0, 40.0), nr.uniform(-10.0, 10.0));
        h.d(DrawingNoise::value(x, y, 9));
        h.d(DrawingNoise::value_3d(x, y, z, 9));
        h.d(DrawingNoise::gradient_3d(x, y, z, -3));
        h.d(DrawingNoise::fractal(DrawingNoiseKind::Value, x, y, 4, DrawingNoiseFractalOptions::default()).unwrap());
        h.d(DrawingNoise::fractal_3d(DrawingNoiseKind::Gradient, x, y, z, 5, five).unwrap());
    }
    check("noise_points", &h, 0x4AA741EE59F14430);

    let mut h = Fnv::new();
    let mut plane = DrawingPlane::new(W, H).unwrap();
    h.plane(DrawingNoise::fill(&mut plane, DrawingNoiseKind::Gradient, 42, 0.1, DrawingNoiseFillOptions { z: Some(2.5), octaves: 3, offset_x: 4.0, offset_y: -1.0, ..DrawingNoiseFillOptions::default() }).unwrap());
    let mut plane = DrawingPlane::new(W, H).unwrap();
    h.plane(DrawingNoise::fill(&mut plane, DrawingNoiseKind::Value, 3, 0.25, DrawingNoiseFillOptions::default()).unwrap());
    let mut plane = DrawingPlane::new(W, H).unwrap();
    h.plane(DrawingNoise::fill(&mut plane, DrawingNoiseKind::Value, -8, 0.07, DrawingNoiseFillOptions { z: Some(0.3), octaves: 4, lacunarity: 1.9, gain: 0.6, ..DrawingNoiseFillOptions::default() }).unwrap());
    let mut plane = DrawingPlane::new(W, H).unwrap();
    h.plane(DrawingNoise::fill(&mut plane, DrawingNoiseKind::Gradient, 5, 0.03, DrawingNoiseFillOptions { octaves: 6, ..DrawingNoiseFillOptions::default() }).unwrap());
    check("noise_fill", &h, 0x699620275358F5A8);
}

#[test]
fn particles_are_noskias_bit_for_bit() {
    let mut system = DrawingParticleSystem::new(123456789);
    let e = system.add_emitter(DrawingParticleEmitter::new());
    e.set_x(100.0);
    e.set_y(80.0);
    e.set_burst(12);
    e.set_rate(30.0).unwrap();
    e.set_start_time(0.25).unwrap();
    e.set_end_time(3.5).unwrap();
    e.set_lifetime(1.2).unwrap();
    e.set_lifetime_variation(0.4).unwrap();
    e.set_direction(-90.0);
    e.set_spread(70.0);
    e.set_speed(140.0);
    e.set_speed_variation(35.0).unwrap();
    e.set_gravity_y(98.0);
    e.set_gravity_x(-5.0);
    e.set_drift_x(7.0);
    e.set_drift_y(-2.0);
    e.set_spawn_radius(6.0).unwrap();
    e.set_start_size(9.0);
    e.set_end_size(2.0);
    e.set_size_variation(0.3).unwrap();
    e.set_start_opacity(0.9);
    e.set_end_opacity(0.05);
    e.set_start_color(Color::from_rgba(255, 200, 40, 255));
    e.set_end_color(Color::from_rgba(120, 10, 0, 90));
    let e = system.add_emitter(DrawingParticleEmitter::new());
    e.set_x(3.0);
    e.set_y(-4.0);
    e.set_rate(7.5).unwrap();
    e.set_lifetime(2.0).unwrap();
    e.set_spread(360.0);
    e.set_speed(20.0);
    let mut h = Fnv::new();
    for t in [0.0, 0.25, 0.3, 1.0, 1.7333, 3.49, 3.6, 5.0, 10.0] {
        let ps = system.get_particles(t).unwrap();
        h.u32(ps.len() as u32);
        for q in ps {
            h.u32(q.emitter_index() as u32);
            h.u64(q.index() as u64);
            h.d(q.birth_time());
            h.d(q.age());
            h.d(q.lifetime());
            h.f(q.x());
            h.f(q.y());
            h.f(q.size());
            h.f(q.opacity());
            h.bytes(&q.color().to_rgba32());
        }
    }
    check("particles", &h, 0x1A790CE7D6F6FB6A);
}
