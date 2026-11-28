//! Host tests of the water surface (Not in C#): the CPU twins in `water_math.rs`, the surface's
//! mappings, the uniform packing, the capture and draw rules and the pipeline keys.

use glam::{Mat4, Vec2, Vec3, Vec4};

use crate::drawing3d::{Camera, ProjectionKind, Ray, SceneLighting};
use crate::presentation::{TargetFormats, SAMPLE_COUNTS};
use crate::water::water_math::{
    bottom_normal, capture_plan, caustic_net, caustic_phases, cycles_to_phase, detail_phases, detail_slope, detail_waves, draw_refusal, edge_fade, facing_fade, fresnel, march, project, reconstruct, sheen_color, sheen_contribution, sheen_phases, shore, tint, transmittance, uses_shifted,
    DepthRead, WaterUniforms, WATER_F0, WATER_SLOT_SIZE, WATER_UNIFORM_SIZE,
};
use crate::water::water_renderer::{pack_uniforms, PackTarget};
use crate::water::{GerstnerWave, OpenWaterWaves, WaterCaustics, WaterLook, WaterSheen, WaterSurface};

fn near(a: f32, b: f32, tolerance: f32) -> bool {
    (a - b).abs() <= tolerance
}

fn camera() -> Camera {
    Camera { position: Vec3::new(0.0, 4.0, 8.0), target: Vec3::new(0.0, 0.0, -2.0), ..Camera::default() }
}

#[test]
fn fresnel_is_f0_head_on_rises_to_one_at_grazing_and_scales_by_reflectivity() {
    assert!(near(fresnel(1.0, 1.0), WATER_F0, 1e-7));
    assert!(near(fresnel(0.0, 1.0), 1.0, 1e-7));
    let mut previous = 1.0;
    for i in 0..=100 {
        let value = fresnel(i as f32 / 100.0, 1.0);
        assert!(value <= previous + 1e-7, "falls as the view turns head on");
        previous = value;
    }
    assert!(near(fresnel(0.5, 0.5), 0.5 * fresnel(0.5, 1.0), 1e-7));
    assert_eq!(fresnel(0.0, 3.0), 1.0, "kept at 1");
    assert_eq!(fresnel(0.3, 0.0), 0.0);
}

#[test]
fn absorption_clears_the_shallows_and_deep_water_turns_the_scattering_colour() {
    let look = WaterLook::default();
    let bottom = Vec3::new(0.8, 0.6, 0.4);
    let light = Vec3::ONE;
    // Path 0: the bottom unchanged.
    assert!(tint(bottom, look.absorption, look.scattering_color, look.turbidity, light, 0.0).distance(bottom) < 1e-6);
    // Red fades first.
    let t = transmittance(look.absorption, 2.0);
    assert!(t.x < t.y && t.y < t.z, "{t:?}");
    assert!(near(t.x, (-0.9f32).exp(), 1e-6));
    // Deep water: the scattering colour (times the light), the bottom gone.
    let deep = tint(bottom, look.absorption, look.scattering_color, look.turbidity, light, 500.0);
    assert!(deep.distance(look.scattering_color) < 1e-4, "{deep:?}");
    // Lit by half the light: half the scattered colour.
    let dim = tint(Vec3::ZERO, look.absorption, look.scattering_color, look.turbidity, Vec3::splat(0.5), 500.0);
    assert!(dim.distance(look.scattering_color * 0.5) < 1e-4);
    // Monotonic from shallow to deep in red (the bottom's red is above the scattering's).
    let mut previous = f32::MAX;
    for i in 0..50 {
        let value = tint(bottom, look.absorption, look.scattering_color, look.turbidity, light, i as f32 * 0.2).x;
        assert!(value <= previous + 1e-6);
        previous = value;
    }
}

#[test]
fn the_soft_shore_fades_thin_water_out_smoothly() {
    assert_eq!(shore(0.0, 0.12), 0.0);
    assert_eq!(shore(0.12, 0.12), 1.0);
    assert_eq!(shore(5.0, 0.12), 1.0);
    assert!(near(shore(0.06, 0.12), 0.5, 1e-6));
    assert_eq!(shore(0.0, 0.0), 1.0, "0 switches the shore off");
}

#[test]
fn a_point_projected_and_reconstructed_comes_back_within_1e4() {
    for projection in [ProjectionKind::Perspective, ProjectionKind::Orthographic] {
        let camera = Camera { projection, ..camera() };
        let view_projection = camera.get_view_projection_matrix(16.0 / 9.0);
        let inverse = view_projection.inverse();
        for point in [Vec3::new(0.0, 0.0, -2.0), Vec3::new(1.5, -0.4, -3.0), Vec3::new(-2.0, 1.0, 0.5)] {
            let (uv, depth) = project(view_projection, point).expect("in front");
            assert!((0.0..=1.0).contains(&depth));
            assert!(reconstruct(inverse, uv, depth).distance(point) < 1e-4, "{projection:?} {point:?}");
        }
    }
    let view_projection = camera().get_view_projection_matrix(1.0);
    assert!(project(view_projection, Vec3::new(0.0, 4.0, 20.0)).is_none(), "behind the camera");
}

#[test]
fn the_refraction_falls_back_to_the_unshifted_view_where_something_is_in_front() {
    // The water's own depth 0.6: the bottom (0.8) behind it is used; an occluder (0.4) in front
    // of the water (a rock above the surface) is not.
    assert!(uses_shifted(0.8, 0.6));
    assert!(uses_shifted(1.0, 0.6), "the sky behind the water");
    assert!(!uses_shifted(0.4, 0.6));
    assert!(uses_shifted(0.6, 0.6), "at the surface exactly");
}

#[test]
fn reflections_fade_at_the_screens_edge_and_where_the_ray_turns_back() {
    assert_eq!(edge_fade(Vec2::splat(0.5)), 1.0);
    assert_eq!(edge_fade(Vec2::new(0.0, 0.5)), 0.0);
    assert_eq!(edge_fade(Vec2::new(0.5, 1.0)), 0.0);
    assert!(near(edge_fade(Vec2::new(0.05, 0.5)), 0.5, 1e-6));
    let v = Vec3::new(0.0, 0.6, 0.8).normalize();
    assert_eq!(facing_fade(Vec3::new(0.0, 0.6, -0.8), v), 1.0, "away from the camera");
    assert_eq!(facing_fade(v, v), 0.0, "straight back");
    assert_eq!(facing_fade(Vec3::new(0.0, -0.5, -0.8).normalize(), v), 0.0, "into the water");
}

/// The scene depth of a synthetic world: a wall facing +Z at z = `wall` (from x -3 to 3, y 0 to
/// 3), else sky - what each pixel of `camera` sees.
fn wall_depth(camera: &Camera, aspect: f32, wall: f32) -> impl Fn(Vec2) -> f32 {
    let view_projection = camera.get_view_projection_matrix(aspect);
    let inverse = view_projection.inverse();
    move |uv: Vec2| {
        let near = reconstruct(inverse, uv, 0.0);
        let far = reconstruct(inverse, uv, 1.0);
        let direction = far - near;
        if direction.z.abs() < 1e-9 {
            return 1.0;
        }
        let t = (wall - near.z) / direction.z;
        let hit = near + direction * t;
        if !(0.0..=1.0).contains(&t) || hit.x.abs() > 3.0 || !(0.0..=3.0).contains(&hit.y) {
            return 1.0;
        }
        project(view_projection, hit).map_or(1.0, |(_, d)| d)
    }
}

#[test]
fn the_screen_space_march_finds_a_wall_on_synthetic_depth_and_misses_open_sky() {
    let camera = Camera { position: Vec3::new(0.0, 1.0, 6.0), target: Vec3::new(0.0, 0.5, 0.0), ..Camera::default() };
    let aspect = 1.5;
    let view_projection = camera.get_view_projection_matrix(aspect);
    let inverse = view_projection.inverse();
    let depth = wall_depth(&camera, aspect, -2.0);
    // A point on the water in front of the wall, its ray reflected up and away toward the wall.
    let start = Vec3::new(0.3, 0.0, 1.0);
    let direction = Vec3::new(0.05, 0.35, -1.0).normalize();
    let (uv, t) = march(&depth, view_projection, inverse, start, direction, 12.0).expect("the wall is hit");
    let hit = start + direction * t;
    assert!(near(hit.z, -2.0, 0.15), "the hit lies on the wall: {hit:?}");
    let expected = project(view_projection, start + direction * ((-2.0 - start.z) / direction.z)).expect("on screen").0;
    assert!(uv.distance(expected) < 0.01, "{uv:?} vs {expected:?}");
    // Up into open sky: no hit.
    let sky = |_: Vec2| 1.0;
    assert!(march(&sky, view_projection, inverse, start, Vec3::new(0.0, 1.0, -0.2).normalize(), 12.0).is_none());
    // A ray running off the screen: no hit.
    assert!(march(&depth, view_projection, inverse, start, Vec3::new(1.0, 0.05, 0.0), 40.0).is_none());
}

#[test]
fn the_bottoms_normal_from_three_depth_points_is_the_planes_normal_facing_the_eye() {
    let plane = Vec3::new(0.2, 1.0, -0.1).normalize();
    let u = plane.cross(Vec3::Z).normalize();
    let w = plane.cross(u);
    let b = Vec3::new(1.0, -2.0, 0.5);
    for (bx, by) in [(b + u * 0.1, b + w * 0.1), (b + w * 0.07, b + u * 0.05)] {
        let n = bottom_normal(b, bx, by, Vec3::Y);
        assert!(n.distance(plane) < 1e-5, "{n:?}");
    }
}

#[test]
fn the_caustic_net_stays_in_range_moves_smoothly_and_follows_its_seed() {
    let mut sum = 0.0;
    let mut bright = 0;
    for i in 0..400 {
        let p = Vec2::new(i as f32 * 0.037, (i % 17) as f32 * 0.21);
        let value = caustic_net(p, 0.3, 1.1, 1);
        assert!((0.0..=1.0).contains(&value), "{value}");
        sum += value;
        if value > 0.5 {
            bright += 1;
        }
        // Same inputs, same value.
        assert_eq!(value, caustic_net(p, 0.3, 1.1, 1));
        // Continuous in time: a tiny change of phase changes it a little.
        assert!((value - caustic_net(p, 0.3 + 1e-3, 1.1 + 6e-4, 1)).abs() < 0.05, "{p:?}");
    }
    assert!(bright > 10 && bright < 300, "a net of thin lines: {bright} of 400 bright");
    assert!(sum / 400.0 > 0.02 && sum / 400.0 < 0.6);
    // The seed changes the pattern.
    let differs = (0..100).filter(|i| (caustic_net(Vec2::new(*i as f32 * 0.13, 0.4), 0.0, 0.0, 1) - caustic_net(Vec2::new(*i as f32 * 0.13, 0.4), 0.0, 0.0, 2)).abs() > 1e-3).count();
    assert!(differs > 30, "{differs}");
    // The phases wrap without a jump: a whole cycle later the net is the same.
    let (a, b) = caustic_phases(10.0, 0.35);
    let (a2, b2) = caustic_phases(10.0 + 1.0 / 0.35, 0.35);
    assert!(near(a, a2, 1e-3));
    assert!(near(caustic_net(Vec2::new(0.4, 0.7), a, b, 1), caustic_net(Vec2::new(0.4, 0.7), a2, b, 1), 1e-3));
    assert!(b != b2, "the second motion has another period");
}

#[test]
fn phases_are_computed_in_f64_and_stay_exact_after_hours() {
    assert!(near(cycles_to_phase(0.25), std::f32::consts::FRAC_PI_2, 1e-6));
    assert!(near(cycles_to_phase(1e5 + 0.25), std::f32::consts::FRAC_PI_2, 1e-4));
    let waves = detail_waves(0.06, 0.35, 1);
    let late = detail_phases(&waves, 0.4, 36_000.0);
    for i in 0..4 {
        let expected = ((36_000.0f64 * waves[i].z as f64 * 0.4f32 as f64).rem_euclid(std::f64::consts::TAU)) as f32;
        // The same angle (compared on the circle: either side of the wrap is the same phase).
        assert!(near(late[i].sin(), expected.sin(), 2e-3) && near(late[i].cos(), expected.cos(), 2e-3), "{i}: {} vs {expected}", late[i]);
        assert!((0.0..=std::f32::consts::TAU).contains(&late[i]));
    }
    let (drift, band) = sheen_phases(1000.25, 0.05);
    assert!(near(drift, 0.0125, 1e-6), "{drift}");
    assert!(near(band, cycles_to_phase(0.075), 1e-4), "{band}");
}

#[test]
fn the_detail_ripples_follow_their_seed_and_their_strength_and_fade_far_away() {
    let a = detail_waves(0.06, 0.35, 1);
    let b = detail_waves(0.06, 0.35, 2);
    assert_ne!(a, b);
    assert_eq!(a, detail_waves(0.06, 0.35, 1));
    let total: f32 = a.iter().map(|w| w.w).sum();
    assert!(near(total, 0.06, 1e-6), "the slopes add up to the strength");
    assert!(near(a[0].z, std::f32::consts::TAU / 0.35, 1e-3), "the longest at the wavelength");
    let phases = detail_phases(&a, 0.4, 3.0);
    let close = detail_slope(&a, phases, Vec2::new(1.0, 2.0), 0.001);
    assert!(close.length() > 0.0 && close.length() <= 0.06 + 1e-6);
    assert_eq!(detail_slope(&a, phases, Vec2::new(1.0, 2.0), 1.0), Vec2::ZERO, "a pixel larger than every ripple: none");
    assert_eq!(detail_slope(&detail_waves(0.0, 0.35, 1), phases, Vec2::ONE, 0.001), Vec2::ZERO);
}

#[test]
fn the_sheens_colour_is_periodic_and_nothing_at_strength_zero() {
    for i in 0..20 {
        let film = i as f32 * 0.137;
        assert!(sheen_color(film).distance(sheen_color(film + 1.0)) < 1e-4);
        let c = sheen_color(film);
        assert!(c.min_element() >= 0.0 && c.max_element() <= 1.0);
    }
    // The three channels are a third of a cycle apart: their mean is the middle.
    let mean = (0..90).map(|i| sheen_color(i as f32 / 90.0)).fold(Vec3::ZERO, |a, b| a + b) / 90.0;
    assert!(mean.distance(Vec3::splat(0.5)) < 1e-3);
    assert_eq!(sheen_contribution(0.3, 0.2, 0.0), Vec3::ZERO);
    assert!(sheen_contribution(0.3, 0.5, 1.0).length() > sheen_contribution(0.3, 0.0, 1.0).length(), "stronger where it slopes");
    assert_eq!(WaterSheen::default().strength, 0.0, "off by default");
}

#[test]
fn field_point_maps_a_ray_into_field_units_with_rotation_and_round_trips() {
    let mut surface = WaterSurface { center: Vec3::new(2.0, 0.5, -1.0), size: Vec2::new(8.0, 4.0), rotation: 0.6, ..WaterSurface::default() };
    // Straight down onto the centre: the middle of the field.
    let down = Ray::new(Vec3::new(2.0, 10.0, -1.0), Vec3::NEG_Y);
    let middle = surface.field_point(&down).expect("hits");
    assert!(middle.distance(Vec2::new(4.0, 2.0)) < 1e-4, "{middle:?}");
    for field in [Vec2::new(0.5, 0.5), Vec2::new(7.5, 1.0), Vec2::new(3.0, 3.9)] {
        let world = surface.to_world(field);
        assert!(near(world.y, 0.5, 1e-6));
        assert!(surface.to_field(world).distance(field) < 1e-4);
        let ray = Ray::new(world + Vec3::new(0.3, 5.0, -0.2), (world - (world + Vec3::new(0.3, 5.0, -0.2))).normalize());
        assert!(surface.field_point(&ray).expect("hits").distance(field) < 1e-3);
    }
    // Column 0, row 0 is the corner of least local X and Z.
    let corner = surface.to_world(Vec2::ZERO);
    let expected = surface.center + crate::water::water_math::rotate_y(Vec3::new(-4.0, 0.0, -2.0), 0.6);
    assert!(corner.distance(expected) < 1e-5);
    // Off the rectangle, parallel, behind: None.
    assert!(surface.field_point(&Ray::new(Vec3::new(30.0, 5.0, 0.0), Vec3::NEG_Y)).is_none());
    assert!(surface.field_point(&Ray::new(Vec3::new(2.0, 5.0, -1.0), Vec3::X)).is_none());
    assert!(surface.field_point(&Ray::new(Vec3::new(2.0, 5.0, -1.0), Vec3::Y)).is_none());
    // With a wave field of another extent: its units.
    surface.rotation = 0.0;
    surface.field = None;
    let plain = surface.field_point(&down).expect("hits");
    assert!(plain.distance(Vec2::new(4.0, 2.0)) < 1e-4);
}

#[test]
fn height_and_normal_pass_the_open_water_waves_through_and_ignore_rotation() {
    let waves = OpenWaterWaves::new(vec![GerstnerWave { direction: 0.4, wavelength: 6.0, amplitude: 0.3, steepness: 0.5, phase: 0.0, speed: None }]).expect("waves");
    let surface = WaterSurface { center: Vec3::new(0.0, 1.5, 0.0), rotation: 1.0, open_waves: Some(waves.clone()), ..WaterSurface::default() };
    for (x, z, t) in [(0.0, 0.0, 0.0), (3.0, -1.0, 2.5), (-4.0, 2.0, 100.0)] {
        assert!(near(surface.height_at(x, z, t), 1.5 + waves.height_at(x, z, t), 1e-6));
        assert!(surface.normal_at(x, z, t).distance(waves.normal_at(x, z, t)) < 1e-6);
    }
    let still = WaterSurface { center: Vec3::new(0.0, -0.2, 0.0), ..WaterSurface::default() };
    assert_eq!(still.height_at(1.0, 2.0, 3.0), -0.2);
    assert_eq!(still.normal_at(1.0, 2.0, 3.0), Vec3::Y);
}

#[test]
fn the_uniform_record_has_the_shaders_offsets() {
    use std::mem::offset_of;
    assert_eq!(WATER_UNIFORM_SIZE, 2544);
    assert_eq!(WATER_SLOT_SIZE, 2560);
    assert_eq!(offset_of!(WaterUniforms, inverse_view_projection), 64);
    assert_eq!(offset_of!(WaterUniforms, camera), 128);
    assert_eq!(offset_of!(WaterUniforms, surface), 256);
    assert_eq!(offset_of!(WaterUniforms, detail_phase), 400);
    assert_eq!(offset_of!(WaterUniforms, detail_waves), 416);
    assert_eq!(offset_of!(WaterUniforms, switches), 512);
    assert_eq!(offset_of!(WaterUniforms, counts), 528);
    assert_eq!(offset_of!(WaterUniforms, sun_matrix), 544);
    assert_eq!(offset_of!(WaterUniforms, sun_shadow), 608);
    assert_eq!(offset_of!(WaterUniforms, waves), 624);
    assert_eq!(offset_of!(WaterUniforms, local_lights), 1136);
    // The mask, reflection, flow and cascade fields follow the surface's own (naga's layout of
    // `struct Water`: the same offsets, size 2544).
    assert_eq!(offset_of!(WaterUniforms, mask), crate::water::water_math::WATER_SURFACE_PART);
    assert_eq!(offset_of!(WaterUniforms, mask), 2160);
    assert_eq!(offset_of!(WaterUniforms, reflection), 2176);
    assert_eq!(offset_of!(WaterUniforms, flow), 2192);
    assert_eq!(offset_of!(WaterUniforms, extra), 2208);
    assert_eq!(offset_of!(WaterUniforms, sun_view), 2224);
    assert_eq!(offset_of!(WaterUniforms, sun_splits), 2240);
    assert_eq!(offset_of!(WaterUniforms, sun_biases), 2256);
    assert_eq!(offset_of!(WaterUniforms, sun_offsets), 2272);
    assert_eq!(offset_of!(WaterUniforms, sun_cascade_matrices), 2288);
    // The WGSL struct names the same fields in the same order.
    let wgsl = crate::shaders::WaterShaders::SURFACE;
    let order = ["view_projection:", "inverse_view_projection:", "camera:", "viewport:", "key_direction:", "key_color:", "environment:", "ambient_sky:", "ambient_ground:", "ambient_up:", "surface:", "extent:", "field:", "field_extent:", "absorption:", "scattering:", "look:", "caustics:", "caustic_phase:", "detail_phase:", "detail_waves:", "sheen:", "sheen_phase:", "switches:", "counts:", "sun_matrix:", "sun_shadow:", "waves:", "local_lights:", "mask:", "reflection:", "flow:", "extra:", "sun_view:", "sun_splits:", "sun_biases:", "sun_offsets:", "sun_cascade_matrices:"];
    let start = wgsl.find("struct Water {").expect("the struct");
    let body = &wgsl[start..start + wgsl[start..].find("};").expect("its end")];
    let cascaded = crate::shaders::WaterShaders::SURFACE_CASCADED;
    let other = cascaded.find("struct Water {").expect("the struct");
    assert_eq!(body, &cascaded[other..other + body.len()], "both variants share the struct");
    let mut at = 0;
    for name in order {
        let found = body[at..].find(&format!("    {name}")).unwrap_or_else(|| panic!("{name} in order"));
        at += found + name.len();
    }
}

#[test]
fn packing_writes_the_open_water_waves_the_look_and_the_switches() {
    let camera = camera();
    let mut lighting = SceneLighting { exposure: 1.25, ..SceneLighting::default() };
    lighting.local_lights.push(crate::PointLight::new(Vec3::new(0.0, 1.0, -2.0), crate::Color4::WHITE, 5.0, 10.0).into());
    let waves = OpenWaterWaves::from_wind(0.3, 5.0, 8, 2);
    let surface = WaterSurface { grid: 128, open_waves: Some(waves.clone()), look: WaterLook { caustics: Some(WaterCaustics { strength: 0.5, ..WaterCaustics::default() }), screen_reflections: false, ..WaterLook::default() }, ..WaterSurface::default() };
    let target = PackTarget { format: wgpu::TextureFormat::Rgba16Float, width: 800, height: 400 };
    let (u, lights) = pack_uniforms(&camera, &lighting, Some(6), None, None, &surface, 12.5, None, target);
    assert_eq!(lights, 1);
    assert_eq!(u.counts[0], 8);
    let records = waves.records(12.5);
    assert_eq!(u.waves[0], records[0].direction);
    assert_eq!(u.waves[15], records[7].motion);
    assert_eq!(u.waves[16], Vec4::ZERO, "unused waves stay zero");
    assert_eq!(u.camera.w, 1.25, "the exposure");
    assert_eq!(u.viewport, Vec4::new(800.0, 400.0, 1.0 / 800.0, 1.0 / 400.0));
    assert_eq!(u.extent.z, 128.0);
    assert!(near(u.extent.w, 10.0 / 128.0, 1e-7));
    assert_eq!(u.environment.z, 5.0, "the last mip level");
    assert_eq!(u.caustics.x, 0.5);
    assert_eq!(u.switches, [0, 1, 0, 0], "no screen reflections, highlights, no shadow (no map), float target");
    assert_eq!(u.counts[2], 0, "no sun map, no filter grid");
    assert!(u.key_direction.w > 0.5);
    assert!((u.inverse_view_projection * u.view_projection).abs_diff_eq(Mat4::IDENTITY, 1e-4));
    // An 8-bit linear target asks the shader to decode and encode.
    let target8 = PackTarget { format: wgpu::TextureFormat::Rgba8Unorm, width: 800, height: 400 };
    let (u8, _) = pack_uniforms(&camera, &lighting, None, None, None, &surface, 0.0, None, target8);
    assert_eq!(u8.switches[3], 1);
    assert_eq!(u8.environment.x, 0.0, "the hemisphere");
}

#[test]
fn the_capture_reads_one_or_four_samples_of_depth32float_and_refuses_the_rest() {
    assert_eq!(capture_plan(1, Some(wgpu::TextureFormat::Depth32Float)).expect("ok"), DepthRead::Single);
    assert_eq!(capture_plan(4, Some(wgpu::TextureFormat::Depth32Float)).expect("ok"), DepthRead::Multisampled);
    let none = capture_plan(1, None).unwrap_err().message().to_string();
    assert!(none.contains("has none") && none.contains("Depth32Float"), "{none}");
    let other = capture_plan(1, Some(wgpu::TextureFormat::Depth24Plus)).unwrap_err().message().to_string();
    assert!(other.contains("Depth24Plus") && other.contains("depth_format: Depth32Float"), "{other}");
    assert!(capture_plan(2, Some(wgpu::TextureFormat::Depth32Float)).is_err());
    // The 4-sample route needs the multisampled depth to be bindable (the presentation change).
    assert!(crate::presentation::multisampling::multisampled_usage(wgpu::TextureFormat::Depth32Float).contains(wgpu::TextureUsages::TEXTURE_BINDING));
}

#[test]
fn a_draw_needs_this_frames_capture_on_the_same_target() {
    assert!(draw_refusal(Some(3), 3, true).is_none());
    assert!(draw_refusal(None, 3, true).expect("refused").contains("capture_scene"));
    assert!(draw_refusal(Some(2), 3, true).expect("refused").contains("capture_scene"), "last frame's capture");
    assert!(draw_refusal(Some(3), 3, false).expect("refused").contains("another target"));
}

#[test]
fn the_water_pipelines_are_kept_per_target_formats() {
    let keys: std::collections::HashSet<TargetFormats> = SAMPLE_COUNTS
        .iter()
        .flat_map(|count| [TargetFormats::new(wgpu::TextureFormat::Rgba16Float, Some(wgpu::TextureFormat::Depth32Float), *count), TargetFormats::new(wgpu::TextureFormat::Rgba8Unorm, Some(wgpu::TextureFormat::Depth32Float), *count)])
        .collect();
    assert_eq!(keys.len(), 4);
}

#[test]
fn the_gerstner_displacement_of_the_shader_matches_the_cpu_twin() {
    // The vertex stage's formula, written out as the shader does it, against gerstner_math.
    let waves = OpenWaterWaves::from_wind(1.0, 6.0, 8, 4);
    let records = waves.records(7.25);
    for p in [Vec2::new(0.0, 0.0), Vec2::new(3.5, -2.0), Vec2::new(-11.0, 6.5)] {
        let mut d = Vec3::ZERO;
        for r in &records {
            let a = r.direction.z * (r.direction.x * p.x + r.direction.y * p.y) - r.motion.y;
            d += Vec3::new(-r.motion.x * r.direction.x * a.sin(), r.direction.w * a.cos(), -r.motion.x * r.direction.y * a.sin());
        }
        assert!(d.distance(waves.displacement_at(p.x, p.y, 7.25)) < 1e-4, "{p:?}");
    }
}
