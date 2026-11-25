//! Tests of the colour pieces: the crate's own Oklab against the vendored `color` crate,
//! ColorMix in every space (Srgb exactly NoSkia's channel rule), palettes, broken colour and the
//! CPU mirror of the GPU hash.

use peniko::color::{AlphaColor, Oklab, Oklch, Srgb};

use crate::animation::{ColorRamp, TrackValue};
use crate::color::Color;
use crate::painting::color_mix::{color_to_oklab, oklab_to_srgb, srgb_to_oklab};
use crate::painting::color_rules::noise_hash;
use crate::painting::{BrokenColor, ColorInterpolation, ColorMix, ColorRules, PaintPalette, PaletteMode};

const BLUE: Color = Color::from_rgb(0, 0, 255);
const YELLOW: Color = Color::from_rgb(255, 255, 0);

fn samples() -> Vec<Color> {
    let mut colors = vec![Color::BLACK, Color::WHITE, BLUE, YELLOW, Color::from_rgb(255, 0, 0), Color::from_rgb(0, 255, 0)];
    for i in 0..40u32 {
        colors.push(Color::from_rgb(((i * 53) % 256) as u8, ((i * 101 + 17) % 256) as u8, ((i * 197 + 99) % 256) as u8));
    }
    colors
}

fn chroma(c: Color) -> f32 {
    let lab = color_to_oklab(c);
    lab[1].hypot(lab[2])
}

#[test]
fn own_oklab_agrees_with_the_color_crate() {
    for c in samples() {
        let ours = color_to_oklab(c);
        let theirs = c.to_peniko(1.0).convert::<Oklab>().components;
        for k in 0..3 {
            assert!((ours[k] - theirs[k]).abs() < 1.0e-5, "{c:?} {ours:?} {theirs:?}");
        }
        let back = oklab_to_srgb(ours);
        let rgb = [c.r as f32 / 255.0, c.g as f32 / 255.0, c.b as f32 / 255.0];
        for k in 0..3 {
            assert!((back[k] - rgb[k]).abs() < 1.0e-4, "{c:?} {back:?}");
        }
        let lch = AlphaColor::<Srgb>::new([rgb[0], rgb[1], rgb[2], 1.0]).convert::<Oklch>().components;
        assert!((lch[1] - ours[1].hypot(ours[2])).abs() < 1.0e-5);
    }
    assert!((srgb_to_oklab([1.0, 1.0, 1.0])[0] - 1.0).abs() < 1.0e-4);
}

#[test]
fn srgb_mix_is_the_ramp_and_track_rule_exactly() {
    let colors = samples();
    for pair in colors.windows(2) {
        let (a, b) = (pair[0].with_alpha(30), pair[1]);
        let ramp = ColorRamp::from_colors(&[a, b]).unwrap();
        for i in 0..=20 {
            let t = i as f32 / 20.0;
            let mixed = ColorMix::mix(a, b, t, ColorInterpolation::Srgb);
            assert_eq!(mixed, ramp.at(t as f64), "{a:?} {b:?} {t}");
            assert_eq!(mixed, Color::interpolate(&a, &b, t as f64));
        }
    }
    assert_eq!(ColorInterpolation::default(), ColorInterpolation::Srgb);
}

#[test]
fn every_space_keeps_its_ends_and_clamps_t() {
    for space in [ColorInterpolation::Srgb, ColorInterpolation::LinearRgb, ColorInterpolation::Oklab, ColorInterpolation::Oklch] {
        for pair in samples().windows(2) {
            let (a, b) = (pair[0], pair[1]);
            let near = |x: Color, y: Color| (x.r as i32 - y.r as i32).abs() <= 1 && (x.g as i32 - y.g as i32).abs() <= 1 && (x.b as i32 - y.b as i32).abs() <= 1 && x.a == y.a;
            assert!(near(ColorMix::mix(a, b, 0.0, space), a), "{space:?} {a:?}");
            assert!(near(ColorMix::mix(a, b, 1.0, space), b), "{space:?} {b:?}");
            assert_eq!(ColorMix::mix(a, b, -3.0, space), ColorMix::mix(a, b, 0.0, space));
            assert_eq!(ColorMix::mix(a, b, 7.0, space), ColorMix::mix(a, b, 1.0, space));
            assert_eq!(ColorMix::mix(a, b, f32::NAN, space), ColorMix::mix(a, b, 0.0, space));
        }
    }
}

#[test]
fn oklab_blue_to_yellow_does_not_pass_through_grey() {
    let srgb = ColorMix::mix(BLUE, YELLOW, 0.5, ColorInterpolation::Srgb);
    let oklab = ColorMix::mix(BLUE, YELLOW, 0.5, ColorInterpolation::Oklab);
    let oklch = ColorMix::mix(BLUE, YELLOW, 0.5, ColorInterpolation::Oklch);
    assert_eq!(srgb, Color::from_rgb(128, 128, 128));
    assert!(chroma(oklab) > chroma(srgb) + 0.01, "{oklab:?} {} vs {}", chroma(oklab), chroma(srgb));
    assert!(chroma(oklch) > chroma(oklab), "{oklch:?}");
    // Oklab's middle is perceptually half way in lightness.
    let l = |c: Color| color_to_oklab(c)[0];
    assert!((l(oklab) - ((l(BLUE) + l(YELLOW)) / 2.0)).abs() < 0.01);
    // Linear light: lighter than sRGB in the middle.
    let linear = ColorMix::mix(Color::BLACK, Color::WHITE, 0.5, ColorInterpolation::LinearRgb);
    assert_eq!(linear, Color::from_rgb(188, 188, 188));
}

#[test]
fn oklch_grey_end_takes_the_other_hue_and_transparent_ends_keep_colour() {
    let grey = Color::from_rgb(128, 128, 128);
    let mid = ColorMix::mix(grey, BLUE, 0.5, ColorInterpolation::Oklch);
    let hue = |c: Color| {
        let lab = color_to_oklab(c);
        lab[2].atan2(lab[1]).to_degrees()
    };
    assert!((hue(mid) - hue(BLUE)).abs() < 2.0, "{mid:?}");
    // Premultiplied: half way to transparent black keeps red's colour at half alpha.
    for space in [ColorInterpolation::LinearRgb, ColorInterpolation::Oklab, ColorInterpolation::Oklch] {
        let half = ColorMix::mix(Color::from_rgb(255, 0, 0), Color::TRANSPARENT, 0.5, space);
        assert_eq!((half.r, half.g, half.b, half.a), (255, 0, 0, 128), "{space:?}");
    }
}

#[test]
fn a_pigment_snaps_to_itself_and_snap_zero_changes_nothing() {
    let pigments = vec![Color::from_rgb(200, 40, 30), Color::from_rgb(30, 60, 160), Color::from_rgb(240, 220, 120)];
    for mode in [PaletteMode::Nearest, PaletteMode::MixOfTwo] {
        let rules = ColorRules { palette: Some(PaintPalette { pigments: pigments.clone(), snap: 1.0, mode }), broken: None };
        for &p in &pigments {
            assert_eq!(rules.apply(p, 9), p, "{mode:?}");
        }
        let off = ColorRules { palette: Some(PaintPalette { pigments: pigments.clone(), snap: 0.0, mode }), broken: None };
        for c in samples() {
            assert_eq!(off.apply(c, 3), c);
        }
    }
    assert_eq!(ColorRules::default().apply(BLUE.with_alpha(77), 1), BLUE.with_alpha(77));
}

#[test]
fn nearest_takes_the_closest_pigment_and_half_snap_goes_half_way() {
    let pigments = vec![Color::from_rgb(200, 40, 30), Color::from_rgb(30, 60, 160)];
    let rules = ColorRules { palette: Some(PaintPalette::new(pigments.clone())), broken: None };
    assert_eq!(rules.apply(Color::from_rgb(220, 60, 60), 0), pigments[0]);
    assert_eq!(rules.apply(Color::from_rgb(20, 40, 200).with_alpha(9), 0), pigments[1].with_alpha(9));
    let half = ColorRules { palette: Some(PaintPalette { snap: 0.5, ..PaintPalette::new(pigments.clone()) }), broken: None };
    let c = Color::from_rgb(220, 120, 60);
    let got = half.apply_float([c.r as f32 / 255.0, c.g as f32 / 255.0, c.b as f32 / 255.0, 1.0], 0);
    let (a, p) = (color_to_oklab(c), color_to_oklab(pigments[0]));
    let expected = oklab_to_srgb([(a[0] + p[0]) / 2.0, (a[1] + p[1]) / 2.0, (a[2] + p[2]) / 2.0]);
    for k in 0..3 {
        assert!((got[k] - expected[k].clamp(0.0, 1.0)).abs() < 1.0e-5);
    }
}

#[test]
fn mix_of_two_puts_a_mix_on_its_segment() {
    let (a, b) = (Color::from_rgb(200, 40, 30), Color::from_rgb(30, 60, 160));
    let far = Color::from_rgb(250, 250, 250);
    let (la, lb) = (color_to_oklab(a), color_to_oklab(b));
    let middle = [(la[0] + lb[0]) / 2.0, (la[1] + lb[1]) / 2.0, (la[2] + lb[2]) / 2.0];
    let rgb = oklab_to_srgb(middle);
    let rules = ColorRules { palette: Some(PaintPalette { pigments: vec![a, far, b], snap: 1.0, mode: PaletteMode::MixOfTwo }), broken: None };
    let got = rules.apply_float([rgb[0], rgb[1], rgb[2], 0.5], 0);
    let lab = srgb_to_oklab([got[0], got[1], got[2]]);
    for k in 0..3 {
        assert!((lab[k] - middle[k]).abs() < 1.0e-4, "{lab:?} {middle:?}");
    }
    assert_eq!(got[3], 0.5);
    // A colour off the segment lands on it: between the two pigments, nearer than either.
    let off = Color::from_rgb(140, 30, 120);
    let nearest = ColorRules { palette: Some(PaintPalette { pigments: vec![a, b], snap: 1.0, mode: PaletteMode::Nearest }), broken: None };
    let mixed = ColorRules { palette: Some(PaintPalette { pigments: vec![a, b], snap: 1.0, mode: PaletteMode::MixOfTwo }), broken: None };
    let d = |x: Color| {
        let (p, q) = (color_to_oklab(x), color_to_oklab(off));
        ((p[0] - q[0]).powi(2) + (p[1] - q[1]).powi(2) + (p[2] - q[2]).powi(2)).sqrt()
    };
    assert!(d(mixed.apply(off, 0)) < d(nearest.apply(off, 0)));
}

#[test]
fn broken_colour_keeps_lightness_and_stays_inside_its_hue_range() {
    let rules = ColorRules { palette: None, broken: Some(BrokenColor { hue: 12.0, chroma: 0.1, lightness: 0.0, seed: 5 }) };
    let base = [0.55, 0.45, 0.35, 1.0];
    let lab0 = srgb_to_oklab([base[0], base[1], base[2]]);
    let (h0, c0) = (lab0[2].atan2(lab0[1]).to_degrees(), lab0[1].hypot(lab0[2]));
    let mut differing = 0;
    for seed in 0..200u32 {
        let got = rules.apply_float(base, seed);
        let lab = srgb_to_oklab([got[0], got[1], got[2]]);
        assert!((lab[0] - lab0[0]).abs() < 1.0e-4, "seed {seed}: L {} vs {}", lab[0], lab0[0]);
        let dh = lab[2].atan2(lab[1]).to_degrees() - h0;
        assert!(dh.abs() <= 12.0 + 1.0e-2, "seed {seed}: hue moved {dh}");
        let c = lab[1].hypot(lab[2]);
        assert!(c >= (c0 * 0.9) - 1.0e-4 && c <= (c0 * 1.1) + 1.0e-4, "seed {seed}: chroma {c} vs {c0}");
        if (got[0] - base[0]).abs() > 1.0e-3 {
            differing += 1;
        }
        assert_eq!(rules.apply_float(base, seed), got, "the same seed gives the same colour");
    }
    assert!(differing > 150, "{differing}");
    // The lightness amount moves L, within its range.
    let light = ColorRules { palette: None, broken: Some(BrokenColor { lightness: 0.05, ..BrokenColor::default() }) };
    let moved = (0..50u32).map(|s| srgb_to_oklab({ let g = light.apply_float(base, s); [g[0], g[1], g[2]] })[0] - lab0[0]).collect::<Vec<_>>();
    assert!(moved.iter().all(|d| d.abs() <= 0.05 + 1.0e-3) && moved.iter().any(|d| d.abs() > 0.01));
}

#[test]
fn the_cpu_hash_is_the_shaders_bit_for_bit() {
    // Values computed by hand from paint.wgsl's noise_hash (32-bit wrapping arithmetic).
    assert_eq!(noise_hash(0, 0, 0, 0), 3_680_091_388);
    assert_eq!(noise_hash(7, 123, 2, 0), 4_012_080_169);
    assert_eq!(noise_hash(0xDEAD_BEEF, -5, 1, 3), 3_437_864_188);
}

#[test]
fn validate_says_what_is_wrong() {
    let palette = |pigments: Vec<Color>, snap: f32| ColorRules { palette: Some(PaintPalette { pigments, snap, mode: PaletteMode::Nearest }), broken: None };
    assert!(palette(Vec::new(), 1.0).validate().unwrap_err().contains("1 to 32 pigments"));
    assert!(palette(vec![BLUE; 33], 1.0).validate().unwrap_err().contains("it has 33"));
    assert!(palette(vec![BLUE], 1.5).validate().unwrap_err().contains("snap"));
    assert!(palette(vec![BLUE], f32::NAN).validate().is_err());
    assert!(palette(vec![BLUE; 32], 0.25).validate().is_ok());
    let broken = |b: BrokenColor| ColorRules { palette: None, broken: Some(b) }.validate();
    assert!(broken(BrokenColor { hue: -1.0, ..BrokenColor::default() }).unwrap_err().contains("hue"));
    assert!(broken(BrokenColor { chroma: 2.0, ..BrokenColor::default() }).unwrap_err().contains("chroma"));
    assert!(broken(BrokenColor { lightness: f32::INFINITY, ..BrokenColor::default() }).unwrap_err().contains("lightness"));
    assert!(broken(BrokenColor::default()).is_ok());
    // apply never fails on unusable rules: an empty palette is none.
    assert_eq!(palette(Vec::new(), 1.0).apply(BLUE, 0), BLUE);
}

// ---- Opt-in perceptual ramps, gradients and colour as a function of time -----------------------

#[test]
fn color_ramp_defaults_to_srgb_and_opts_into_a_space() {
    let colors = [BLUE, Color::from_rgb(250, 240, 20).with_alpha(200), Color::from_rgb(200, 30, 60)];
    let ramp = ColorRamp::from_colors(&colors).unwrap();
    assert_eq!(ramp.interpolation(), ColorInterpolation::Srgb);
    for space in [ColorInterpolation::LinearRgb, ColorInterpolation::Oklab, ColorInterpolation::Oklch] {
        let perceptual = ramp.clone().with_interpolation(space);
        assert_eq!(perceptual.interpolation(), space);
        assert_ne!(perceptual, ramp);
        for i in 0..=40 {
            let t = i as f64 / 40.0;
            let (segment, u) = if t < 0.5 { (0, t / 0.5) } else { (1, (t - 0.5) / 0.5) };
            let expected = if t >= 1.0 { colors[2] } else { ColorMix::mix(colors[segment], colors[segment + 1], u as f32, space) };
            assert_eq!(perceptual.at(t), expected, "{space:?} at {t}");
            assert_eq!(perceptual.at_with_alpha(t, 9), expected.with_alpha(9));
        }
        // The stops themselves are kept exactly.
        assert_eq!(perceptual.at(0.0), colors[0]);
        assert_eq!(perceptual.at(1.0), colors[2]);
    }
    // Srgb chosen explicitly is the default ramp.
    assert_eq!(ramp.clone().with_interpolation(ColorInterpolation::Srgb), ramp);
}

#[test]
fn gradient_fills_carry_their_interpolation_to_the_brush() {
    use crate::fills::{GradientSpread, LinearGradientFill, RadialGradientFill, ShapeFill};
    use crate::geometry::PointF;
    use peniko::color::ColorSpaceTag;
    use peniko::Brush;
    let linear = LinearGradientFill::from_colors(PointF::new(0.0, 0.0), PointF::new(10.0, 0.0), BLUE, YELLOW).unwrap();
    let mut radial = RadialGradientFill::from_colors(PointF::new(0.0, 0.0), 5.0, BLUE, YELLOW).unwrap();
    assert_eq!((linear.interpolation(), radial.interpolation()), (ColorInterpolation::Srgb, ColorInterpolation::Srgb));
    let tag = |fill: ShapeFill| match fill.to_brush(1.0) {
        Brush::Gradient(g) => g.interpolation_cs,
        _ => panic!("a gradient fill is a gradient brush"),
    };
    assert_eq!(tag(linear.clone().into()), ColorSpaceTag::Srgb);
    assert_eq!(tag(radial.clone().into()), ColorSpaceTag::Srgb);
    assert_eq!(tag(linear.clone().with_interpolation(ColorInterpolation::Oklab).into()), ColorSpaceTag::Oklab);
    radial.set_interpolation(ColorInterpolation::Oklch);
    assert_eq!(radial.interpolation(), ColorInterpolation::Oklch);
    assert_eq!(tag(radial.into()), ColorSpaceTag::Oklch);
    assert_eq!(tag(LinearGradientFill::new(PointF::new(0.0, 0.0), PointF::new(1.0, 1.0), &ColorRamp::from_colors(&[BLUE, YELLOW]).unwrap().get_stops(), GradientSpread::Repeat).unwrap().with_interpolation(ColorInterpolation::LinearRgb).into()), ColorSpaceTag::LinearSrgb);
}

#[test]
fn a_colour_function_binds_like_a_colour_track_and_follows_seeks() {
    use crate::animation::{ColorTrack, EasingKind};
    use crate::models::{AnimatedProperty, BindingTarget, BoundTrack};
    use crate::shapes::DrawingGroup;
    use crate::{DrawingSession, DrawingSessionOptions};
    let mut session = DrawingSession::new(DrawingSessionOptions::default());
    session.add_layer("Sky", Color::BLACK);
    let mix = |t: f64| ColorMix::mix(BLUE, YELLOW, (t / 10.0) as f32, ColorInterpolation::Oklab);
    session.animations_mut().bind(BindingTarget::Layer("Sky".to_string()), AnimatedProperty::Color, BoundTrack::color_fn(mix)).unwrap();
    session.set_time(5.0);
    assert_eq!(session.get_layer("Sky").unwrap().color(), mix(5.0));
    session.set_time(10.0);
    assert_eq!(session.get_layer("Sky").unwrap().color(), YELLOW);
    // Seeking back re-applies the function.
    session.set_time(2.5);
    assert_eq!(session.get_layer("Sky").unwrap().color(), mix(2.5));
    // A group's content colour too, as a ColorTrack binds.
    let group = session.get_layer_mut("Sky").unwrap().add_element(DrawingGroup::new(None));
    session.animations_mut().bind(group, AnimatedProperty::Color, BoundTrack::color_fn(|_| Color::from_rgb(1, 2, 3))).unwrap();
    session.set_time(1.0);
    match session.find_element(group).unwrap().kind() {
        crate::shapes::ShapeKind::Group(g) => assert_eq!(g.content_color(), Some(Color::from_rgb(1, 2, 3))),
        _ => panic!("a group"),
    }
    // It is a colour source: a float property refuses it with the same message as a ColorTrack.
    let by_fn = session.animations_mut().bind(BindingTarget::Layer("Sky".to_string()), AnimatedProperty::Opacity, BoundTrack::color_fn(|_| BLUE)).unwrap_err();
    let by_track = session.animations_mut().bind(BindingTarget::Layer("Sky".to_string()), AnimatedProperty::Opacity, ColorTrack::from_keys(&[(0.0, BLUE, EasingKind::Linear)]).unwrap()).unwrap_err();
    assert_eq!(by_fn, by_track);
    assert_eq!(format!("{:?}", BoundTrack::color_fn(|_| BLUE)), "ColorFn");
}
