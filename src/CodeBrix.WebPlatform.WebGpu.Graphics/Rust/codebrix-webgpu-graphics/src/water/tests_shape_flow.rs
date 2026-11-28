//! Host tests of the water surface's mask, flow, reflection environment and sun cascades (Not in
//! C#): the CPU twins of the shader's mask read and edge fade, the flow's wrapped offsets, the
//! reflection's choice, the cascade pick against the model renderer's, and that a surface using
//! none of them packs the bytes it packed before they existed.

use glam::{Mat4, Vec2, Vec3, Vec4};

use crate::drawing3d::{AmbientLighting, Camera, Ray, SceneLighting};
use crate::lighting::shadows::{cascade_pick, sun_cascades};
use crate::lighting::SunShadowSettings;
use crate::models::sun_shadow_numbers;
use crate::models::pbr_data::PbrFrameUniforms;
use crate::water::water_mask::{coverage_opacity, edge_coverage, ellipse_distance, polygon_distance, shader_threshold};
use crate::water::water_math::{caustic_flow, caustic_net, caustic_net_moved, caustic_phases, detail_phases, detail_phases_flowing, detail_slope, detail_waves, mask_opacity, sheen_flow_rate, sheen_phases, sheen_phases_flowing, WaterUniforms, SHEEN_BAND_AXIS, WATER_SURFACE_PART};
use crate::water::water_renderer::{environment_levels, pack_uniforms, PackTarget};
use crate::water::{OpenWaterWaves, WaterCaustics, WaterCoverage, WaterDetail, WaterLook, WaterSheen, WaterSurface};

fn near(a: f32, b: f32, tolerance: f32) -> bool {
    (a - b).abs() <= tolerance
}

fn camera() -> Camera {
    Camera { position: Vec3::new(0.0, 4.0, 8.0), target: Vec3::new(0.0, 0.0, -2.0), ..Camera::default() }
}

const TARGET: PackTarget = PackTarget { format: wgpu::TextureFormat::Rgba16Float, width: 1280, height: 720 };

/// A surface with every older part in use (waves, caustics, sheen, detail), none of the new ones.
fn busy_surface() -> WaterSurface {
    WaterSurface {
        center: Vec3::new(1.0, -0.5, 2.0),
        size: Vec2::new(14.0, 6.0),
        rotation: 0.7,
        grid: 100,
        open_waves: Some(OpenWaterWaves::from_wind(0.3, 5.0, 8, 2)),
        detail: WaterDetail { strength: 0.08, wavelength: 0.5, speed: 0.7, seed: 9 },
        look: WaterLook { caustics: Some(WaterCaustics { strength: 0.5, ..WaterCaustics::default() }), sheen: WaterSheen { strength: 0.4, ..WaterSheen::default() }, ..WaterLook::default() },
        ..WaterSurface::default()
    }
}

fn bytes(u: &WaterUniforms) -> &[u8] {
    bytemuck::bytes_of(u)
}

#[test]
fn a_surface_using_none_of_the_new_parts_packs_the_bytes_it_packed_before() {
    // Before the mask, reflection, flow and cascades, the record was the first 2160 bytes; what
    // they add after it stays zero, and the time-driven fields are the unflowing formulas'.
    let mut lighting = SceneLighting { exposure: 1.1, ..SceneLighting::default() };
    lighting.local_lights.push(crate::PointLight::new(Vec3::new(0.0, 1.0, -2.0), crate::Color4::WHITE, 5.0, 10.0).into());
    for surface in [WaterSurface::default(), busy_surface()] {
        for (environment, time) in [(None, 0.0), (Some(7), 12.5), (None, 3600.0 * 9.0 + 0.3)] {
            let (u, _) = pack_uniforms(&camera(), &lighting, environment, None, None, &surface, time, None, TARGET);
            assert!(bytes(&u)[WATER_SURFACE_PART..].iter().all(|b| *b == 0), "the new fields are zero");
            assert_eq!(u.detail_phase, detail_phases(&u.detail_waves, surface.detail.speed, time));
            let (drift, band) = sheen_phases(time, surface.look.sheen.drift);
            assert_eq!(u.sheen_phase, Vec4::new(drift, band, 0.0, 0.0));
            if let Some(caustics) = surface.look.caustics {
                let (a, b) = caustic_phases(time, caustics.speed);
                assert_eq!(u.caustic_phase, Vec4::new(a, b, 0.0, 0.0));
            }
        }
    }
}

#[test]
fn each_new_part_writes_only_its_own_fields() {
    let lighting = SceneLighting::default();
    let plain = busy_surface();
    let (before, _) = pack_uniforms(&camera(), &lighting, None, None, None, &plain, 40.0, None, TARGET);
    let differing = |after: &WaterUniforms| -> Vec<usize> { bytes(&before).iter().zip(bytes(after)).enumerate().filter(|(_, (a, b))| a != b).map(|(i, _)| i).collect() };
    // A mask (its threshold as the draw hands it on): only the mask field.
    let (u, _) = pack_uniforms(&camera(), &lighting, None, None, Some(0.2), &plain, 40.0, None, TARGET);
    assert_eq!(u.mask, Vec4::new(1.0, 0.2, 0.0, 0.0));
    assert!(differing(&u).iter().all(|i| (2160..2176).contains(i)));
    // A reflection environment: only the reflection field.
    let (u, _) = pack_uniforms(&camera(), &lighting, None, Some(6), None, &plain, 40.0, None, TARGET);
    assert_eq!(u.reflection, Vec4::new(1.0, lighting.environment_intensity, 5.0, lighting.environment_rotation));
    assert!(differing(&u).iter().all(|i| (2176..2192).contains(i)));
    // A flow: the phases of the detail, the sheen's band and the caustics, and the flow fields.
    let flowing = WaterSurface { flow: Vec2::new(0.8, -0.3), ..plain.clone() };
    let (u, _) = pack_uniforms(&camera(), &lighting, None, None, None, &flowing, 40.0, None, TARGET);
    let offset = |f: usize| f;
    for i in differing(&u) {
        let detail_phase = offset(400)..offset(416);
        let sheen_band = offset(496 + 4)..offset(496 + 8);
        let caustic_shifts = offset(384 + 8)..offset(384 + 16);
        let new_flow = 2192..2208;
        let cells = 2208 + 4..2208 + 12;
        assert!(detail_phase.contains(&i) || sheen_band.contains(&i) || caustic_shifts.contains(&i) || new_flow.contains(&i) || cells.contains(&i), "byte {i}");
    }
    assert_eq!((u.flow.x, u.flow.y), (0.8, -0.3));
    // A flow that is not finite does not flow.
    let broken = WaterSurface { flow: Vec2::new(f32::NAN, 1.0), ..plain.clone() };
    let (u, _) = pack_uniforms(&camera(), &lighting, None, None, None, &broken, 40.0, None, TARGET);
    assert!(differing(&u).is_empty());
}

#[test]
fn the_coverage_is_read_as_the_gpu_reads_it_bilinear_and_clamped() {
    let mut grid = WaterCoverage::new(4, 2);
    for (i, value) in [0.0, 0.2, 0.6, 1.0, 1.0, 0.6, 0.2, 0.0].iter().enumerate() {
        grid.set(i as u32 % 4, i as u32 / 4, *value);
    }
    // Stored as the byte the picture holds.
    assert_eq!(grid.cells()[1], 51);
    assert!(near(grid.get(1, 0), 51.0 / 255.0, 1e-7));
    // At a cell centre: the cell; between two centres: halfway; beyond the outer centres: held.
    assert!(near(grid.sample(Vec2::new(0.375, 0.25)), grid.get(1, 0), 1e-6));
    assert!(near(grid.sample(Vec2::new(0.25, 0.25)), 0.5 * (grid.get(0, 0) + grid.get(1, 0)), 1e-6));
    assert!(near(grid.sample(Vec2::new(0.0, 0.0)), grid.get(0, 0), 1e-6));
    assert!(near(grid.sample(Vec2::new(1.0, 1.0)), grid.get(3, 1), 1e-6));
    assert!(near(grid.sample(Vec2::new(-3.0, 0.25)), grid.get(0, 0), 1e-6));
    assert!(near(grid.sample(Vec2::new(0.125, 0.5)), 0.5 * (grid.get(0, 0) + grid.get(0, 1)), 1e-6));
    assert_eq!(grid.sample(Vec2::new(f32::NAN, 0.5)), 0.0);
    assert_eq!(WaterCoverage::new(0, 9999).columns(), 1);
    assert_eq!(WaterCoverage::new(0, 9999).rows(), WaterCoverage::MAX_CELLS);
}

#[test]
fn the_masks_edge_fades_smoothly_from_the_threshold_and_draws_nothing_below_it() {
    assert_eq!(mask_opacity(0.0, 0.0), None, "no coverage: nothing drawn");
    assert_eq!(mask_opacity(0.3, 0.3), None, "at the threshold: nothing drawn");
    assert_eq!(mask_opacity(f32::NAN, 0.0), None);
    assert_eq!(mask_opacity(1.0, 0.0), Some(1.0));
    assert_eq!(mask_opacity(1.0, 0.5), Some(1.0));
    assert!(near(mask_opacity(0.5, 0.0).expect("drawn"), 0.5, 1e-6), "half coverage, half opacity");
    // Continuous from 0 just above the threshold, rising, no step.
    for threshold in [0.0, 0.25, 0.9] {
        let mut previous = 0.0;
        for i in 1..=200 {
            let c = threshold + (1.0 - threshold) * i as f32 / 200.0;
            let o = mask_opacity(c, threshold).expect("drawn");
            assert!(o >= previous - 1e-6 && o - previous < 0.02, "{threshold} {c}");
            previous = o;
        }
        assert!(mask_opacity(threshold + 1e-4, threshold).expect("drawn") < 1e-3);
    }
}

#[test]
fn ellipse_and_outline_coverage_are_water_inside_none_outside_and_half_on_the_line() {
    // Distances in cells.
    assert!(near(ellipse_distance(Vec2::new(5.0, 0.0), Vec2::new(4.0, 2.0)), 1.0, 1e-5));
    assert!(near(ellipse_distance(Vec2::new(0.0, 1.0), Vec2::new(4.0, 2.0)), -1.0, 1e-5));
    assert!(near(ellipse_distance(Vec2::new(3.0, 4.0), Vec2::splat(2.0)), 3.0, 1e-5), "a circle exactly");
    assert!(ellipse_distance(Vec2::ZERO, Vec2::new(4.0, 2.0)) < 0.0);
    let square = [Vec2::new(0.0, 0.0), Vec2::new(4.0, 0.0), Vec2::new(4.0, 4.0), Vec2::new(0.0, 4.0)];
    assert!(near(polygon_distance(Vec2::new(2.0, 1.0), &square), -1.0, 1e-6));
    assert!(near(polygon_distance(Vec2::new(6.0, 2.0), &square), 2.0, 1e-6));
    let reversed: Vec<Vec2> = square.iter().rev().copied().collect();
    assert_eq!(polygon_distance(Vec2::new(2.0, 1.0), &reversed), polygon_distance(Vec2::new(2.0, 1.0), &square), "either way round");
    assert_eq!(edge_coverage(0.0, 4.0), 0.5);
    assert_eq!(edge_coverage(-2.0, 4.0), 1.0);
    assert_eq!(edge_coverage(2.0, 4.0), 0.0);
    assert_eq!((edge_coverage(-0.1, 0.0), edge_coverage(0.1, 0.0)), (1.0, 0.0), "no feather: a hard edge");
    // A round pool: water in the middle, none in the corners, its fade ending at the edges.
    let pool = WaterCoverage::ellipse(64, 64, 4.0);
    assert_eq!(pool.sample(Vec2::splat(0.5)), 1.0);
    for corner in [Vec2::ZERO, Vec2::new(1.0, 0.0), Vec2::ONE, Vec2::new(0.0, 1.0), Vec2::new(0.1, 0.1)] {
        assert_eq!(pool.sample(corner), 0.0, "{corner:?}");
    }
    // The fade ends at the rectangle's edge: the edge cell's centre (half a cell in) is 1.5 cells
    // outside the outline, 2 cells in from the edge.
    assert!(near(pool.get(0, 32), 0.5 - 1.5 / 4.0, 3e-3), "{}", pool.get(0, 32));
    assert!(near(pool.sample(Vec2::new(2.0 / 64.0, 0.5)), 0.5, 0.01), "half on the outline");
    assert_eq!(pool.sample(Vec2::new(0.1, 0.5)), 1.0);
    // A pond's outline (an L shape): water inside, none in the notch.
    let outline = [Vec2::new(0.1, 0.1), Vec2::new(0.9, 0.1), Vec2::new(0.9, 0.5), Vec2::new(0.5, 0.5), Vec2::new(0.5, 0.9), Vec2::new(0.1, 0.9)];
    let pond = WaterCoverage::polygon(80, 80, &outline, 2.0);
    assert_eq!(pond.sample(Vec2::new(0.3, 0.3)), 1.0);
    assert_eq!(pond.sample(Vec2::new(0.75, 0.75)), 0.0, "the notch");
    assert!(near(pond.sample(Vec2::new(0.1, 0.3)), 0.5, 0.05), "half on the outline");
    assert!(WaterCoverage::polygon(8, 8, &outline[..2], 2.0).cells().iter().all(|c| *c == 0), "two corners: no water");
}

#[test]
fn the_surface_answers_outside_its_mask() {
    let coverage = WaterCoverage::ellipse(64, 64, 2.0);
    let surface = WaterSurface { center: Vec3::new(2.0, 0.5, -1.0), size: Vec2::new(8.0, 8.0), rotation: 0.3, ..WaterSurface::default() };
    // Without a mask: water over the whole rectangle, none off it.
    let corner = surface.to_world(Vec2::new(0.2, 0.2));
    assert_eq!(surface.opacity_at(corner.x, corner.z), 1.0);
    assert_eq!(surface.opacity_at(50.0, 50.0), 0.0);
    let down = |p: Vec3| Ray::new(p + Vec3::Y * 5.0, Vec3::NEG_Y);
    assert!(surface.field_point(&down(corner)).is_some());
    // The mask's coordinates: (0, 0) at the corner of least local X and Z, (1, 1) opposite.
    assert!(surface.to_mask(surface.to_world(Vec2::ZERO)).distance(Vec2::ZERO) < 1e-5);
    assert!(surface.to_mask(surface.to_world(surface.size)).distance(Vec2::ONE) < 1e-5);
    assert!(surface.to_mask(surface.center).distance(Vec2::splat(0.5)) < 1e-6);
    // With a round mask (what opacity_at and field_point consult): no water in the corner, water
    // in the middle, the fade between; a mask without CPU coverage counts as water everywhere.
    let at = |world: Vec3, threshold: f32| coverage_opacity(Some(&coverage), threshold, surface.to_mask(world));
    assert_eq!(at(corner, 0.0), 0.0);
    assert_eq!(at(surface.center, 0.0), 1.0);
    let rim = surface.to_world(Vec2::new(0.5, 0.03) * surface.size);
    assert!(at(rim, 0.0) > 0.0 && at(rim, 0.0) < 1.0, "{}", at(rim, 0.0));
    assert_eq!(at(rim, 0.99), 0.0, "a high threshold cuts the rim off");
    assert_eq!(coverage_opacity(None, 0.5, Vec2::ZERO), 1.0);
    assert_eq!((shader_threshold(f32::NAN), shader_threshold(-1.0), shader_threshold(2.0)), (0.0, 0.0, 0.99));
}

#[test]
fn flow_carries_the_detail_ripples_exactly_and_keeps_their_phases_wrapped() {
    let waves = detail_waves(0.06, 0.35, 3);
    let flow = Vec2::new(1.2, -0.4);
    // The flowing pattern at p is the still pattern at p - flow * t.
    for t in [0.0, 1.5, 33.0] {
        let flowing = detail_phases_flowing(&waves, 0.4, flow, t);
        let still = detail_phases(&waves, 0.4, t);
        for p in [Vec2::new(0.3, 0.2), Vec2::new(-4.0, 7.0)] {
            let a = detail_slope(&waves, flowing, p, 0.001);
            let b = detail_slope(&waves, still, p - flow * t as f32, 0.001);
            assert!(a.distance(b) < 2e-4, "{t} {p:?}: {a:?} {b:?}");
        }
    }
    // Zero flow is the unflowing formula exactly.
    assert_eq!(detail_phases_flowing(&waves, 0.4, Vec2::ZERO, 1234.5), detail_phases(&waves, 0.4, 1234.5));
    // After hours: wrapped into 0..TAU and continuous from one frame to the next.
    for t in [3600.0 * 10.0, 3600.0 * 1000.0] {
        let now = detail_phases_flowing(&waves, 0.4, flow, t);
        let next = detail_phases_flowing(&waves, 0.4, flow, t + 1.0 / 60.0);
        for i in 0..4 {
            assert!((0.0..=std::f32::consts::TAU).contains(&now[i]));
            let step = (next[i] - now[i]).rem_euclid(std::f32::consts::TAU);
            let expected = ((waves[i].z as f64 * (0.4 + (waves[i].x * flow.x + waves[i].y * flow.y) as f64) / 60.0).rem_euclid(std::f64::consts::TAU)) as f32;
            assert!(near(step, expected, 2e-3) || near(step + std::f32::consts::TAU, expected, 2e-3) || near(step, expected + std::f32::consts::TAU, 2e-3), "{t} {i}: {step} vs {expected}");
        }
    }
}

#[test]
fn flow_carries_the_caustic_net_with_no_growing_number_and_no_jump() {
    let seed = 4;
    let flow = Vec2::new(0.9, 0.35);
    let scale = 1.4;
    // Zero flow: nothing moves.
    let still = caustic_flow(Vec2::ZERO, scale, 5000.0);
    assert_eq!((still.cells, still.fraction, still.shifts), ([0, 0], Vec2::ZERO, Vec2::ZERO));
    // Early on, where f32 is exact enough: the moved net at p is the unmoved net at p - offset.
    for t in [0.0, 0.7, 3.3, 11.0] {
        let moved = caustic_flow(flow, scale, t);
        assert!(moved.fraction.min_element() >= 0.0 && moved.fraction.max_element() < 1.0);
        assert!(moved.shifts.min_element() >= 0.0 && moved.shifts.max_element() <= std::f32::consts::TAU);
        for p in [Vec2::new(0.4, 0.7), Vec2::new(3.1, -2.2), Vec2::new(-5.5, 1.25)] {
            let net = caustic_net_moved(p - moved.fraction, 0.3, 1.1, moved.shifts, moved.cells, seed);
            let expected = caustic_net(p - flow * scale * t as f32, 0.3, 1.1, seed);
            assert!(near(net, expected, 2e-3), "{t} {p:?}: {net} vs {expected}");
        }
    }
    // After many hours: every number the shader gets stays small, and a frame's step is the net
    // carried by that step (no pop where the offset crosses a cell or wraps).
    for t in [3600.0 * 10.0, 3600.0 * 500.0] {
        for k in 0..40 {
            let t0 = t + k as f64 * 0.37;
            let a = caustic_flow(flow, scale, t0);
            let b = caustic_flow(flow, scale, t0 + 1.0 / 120.0);
            for p in [Vec2::new(0.4, 0.7), Vec2::new(3.1, -2.2)] {
                let nb = caustic_net_moved(p - b.fraction, 0.3, 1.1, b.shifts, b.cells, seed);
                // The same as the net carried by the small step from the earlier offset.
                let step = flow * scale / 120.0;
                let carried = caustic_net_moved(p - step - a.fraction, 0.3, 1.1, a.shifts, a.cells, seed);
                assert!(near(nb, carried, 3e-3), "{t0} {p:?}: {nb} vs {carried}");
            }
        }
    }
}

#[test]
fn flow_carries_the_sheens_bands_and_keeps_its_phase_wrapped() {
    let flow = Vec2::new(-0.6, 0.25);
    let scale = 0.35;
    let rate = sheen_flow_rate(flow, scale);
    let band = |p: Vec2, phase: f32| ((p.x as f64 * SHEEN_BAND_AXIS[0] + p.y as f64 * SHEEN_BAND_AXIS[1]) as f32 * scale + phase).sin();
    for t in [0.0, 2.0, 77.0] {
        let (_, flowing) = sheen_phases_flowing(t, 0.05, rate);
        let (_, still) = sheen_phases(t, 0.05);
        for p in [Vec2::new(1.0, 2.0), Vec2::new(-3.0, 0.5)] {
            assert!(near(band(p, flowing), band(p - flow * t as f32, still), 1e-3), "{t} {p:?}");
        }
    }
    assert_eq!(sheen_phases_flowing(999.0, 0.05, 0.0), sheen_phases(999.0, 0.05), "zero flow: exactly the still phases");
    let (drift, late) = sheen_phases_flowing(3600.0 * 200.0, 0.05, rate);
    assert!((0.0..=std::f32::consts::TAU).contains(&late) && (0.0..1.0).contains(&drift));
}

#[test]
fn the_reflection_environment_is_bound_in_place_of_the_ambients_and_none_changes_nothing() {
    // (ambient levels, reflection levels) -> what the packing is given.
    assert_eq!(environment_levels(None, None), (None, None), "hemisphere, no reflection: as before");
    assert_eq!(environment_levels(Some(7), None), (Some(7), None), "environment ambient: as before");
    assert_eq!(environment_levels(None, Some(5)), (None, Some(5)), "hemisphere: the sky light stays the hemisphere's");
    assert_eq!(environment_levels(Some(7), Some(5)), (Some(5), Some(5)), "one cube bound: the reflection's, for both");
    let lighting = SceneLighting { ambient: AmbientLighting::Hemisphere, environment_intensity: 0.8, environment_rotation: 1.25, ..SceneLighting::default() };
    let (u, _) = pack_uniforms(&camera(), &lighting, None, Some(5), None, &WaterSurface::default(), 0.0, None, TARGET);
    assert_eq!(u.environment.x, 0.0, "the water's own light: the hemisphere");
    assert_eq!(u.reflection, Vec4::new(1.0, 0.8, 4.0, 1.25));
    let (u, _) = pack_uniforms(&camera(), &lighting, None, None, None, &WaterSurface::default(), 0.0, None, TARGET);
    assert_eq!(u.reflection, Vec4::ZERO);
}

/// A frame of the model renderer with `count` sun cascades, and its cascades.
fn cascaded_frame(count: u32) -> (PbrFrameUniforms, Camera, Vec<Mat4>) {
    let camera = Camera { position: Vec3::new(10.0, 3.0, 20.0), target: Vec3::new(30.0, 0.0, -10.0), far_plane: 4000.0, ..Camera::default() };
    let mut lighting = SceneLighting { sun_shadows: SunShadowSettings::covering(Vec3::new(-1000.0, -10.0, -1000.0), Vec3::new(1000.0, 100.0, 1000.0)), ..SceneLighting::default() };
    if count > 1 {
        lighting.sun_shadows = lighting.sun_shadows.with_cascades(count, 600.0);
    }
    let cascades = sun_cascades(&lighting.sun_shadows, lighting.light_direction, &camera, 1.5).expect("a region");
    let mut frame = PbrFrameUniforms::new(&camera, &lighting, 1.5);
    frame.set_shadows(&lighting, cascades.views.first(), &[]);
    frame.set_sun_cascades(&cascades);
    (frame, camera, cascades.views.iter().map(|v| v.view_projection).collect())
}

#[test]
fn one_cascade_packs_the_single_map_as_before_and_more_pack_every_cascade() {
    let lighting = SceneLighting::default();
    let surface = WaterSurface::default();
    let (frame, camera, matrices) = cascaded_frame(1);
    let single = sun_shadow_numbers(&frame).expect("the sun casts");
    assert!(single.cascades.is_none());
    let (u, _) = pack_uniforms(&camera, &lighting, None, None, None, &surface, 0.0, Some(&single), TARGET);
    assert!(bytes(&u)[WATER_SURFACE_PART..].iter().all(|b| *b == 0), "one cascade: the new fields stay zero");
    assert_eq!(u.sun_matrix, matrices[0]);
    assert_eq!(u.sun_shadow, Vec4::new(single.strength, single.texel, single.depth_bias, single.normal_offset));
    assert_eq!(u.counts[2], single.softness);
    for count in [2u32, 3, 4] {
        let (frame, camera, matrices) = cascaded_frame(count);
        let numbers = sun_shadow_numbers(&frame).expect("the sun casts");
        let (u, _) = pack_uniforms(&camera, &lighting, None, None, None, &surface, 0.0, Some(&numbers), TARGET);
        assert_eq!(u.extra[0], count);
        // The single map's fields are still the last cascade's, as handed to any one-map receiver.
        assert_eq!(u.sun_matrix, matrices[count as usize - 1]);
        assert_eq!(u.sun_view, frame.sun_view);
        assert_eq!(u.sun_splits, frame.sun_splits);
        assert_eq!(u.sun_biases, frame.sun_biases);
        assert_eq!(u.sun_offsets, frame.sun_offsets);
        for (i, matrix) in matrices.iter().enumerate() {
            assert_eq!(u.sun_cascade_matrices[i], *matrix, "{count} {i}");
        }
        // No shadow received: nothing of the sun.
        let unshadowed = WaterSurface { look: WaterLook { receives_shadows: false, ..WaterLook::default() }, ..WaterSurface::default() };
        let (u, _) = pack_uniforms(&camera, &lighting, None, None, None, &unshadowed, 0.0, Some(&numbers), TARGET);
        assert_eq!((u.extra[0], u.counts[2]), (0, 0));
    }
}

#[test]
fn the_waters_cascade_pick_is_the_model_shaders() {
    // The WGSL: the same function text but for the uniform's name.
    fn function<'a>(source: &'a str, name: &str) -> &'a str {
        let start = source.find(&format!("fn {name}(")).expect("the function");
        let end = start + source[start..].find("\n}\n").expect("its end") + 3;
        &source[start..end]
    }
    let water = function(crate::shaders::WaterShaders::SURFACE_CASCADED, "cascade_pick");
    let model = function(include_str!("../shaders/wgsl/pbr_body.wgsl"), "cascade_pick");
    assert_eq!(water.replace("water.", "frame."), model);
    // The numbers: the twin picks the same cascade and weight from the water's uniforms as from
    // the model renderer's frame, at points along the view (the depth as each shader takes it).
    for count in [2u32, 3, 4] {
        let (frame, camera, _) = cascaded_frame(count);
        let numbers = sun_shadow_numbers(&frame).expect("the sun casts");
        let (u, _) = pack_uniforms(&camera, &SceneLighting::default(), None, None, None, &WaterSurface::default(), 0.0, Some(&numbers), TARGET);
        for i in 0..400 {
            let world = camera.position + camera.forward() * (i as f32 * 1.7) + Vec3::new(3.0, -1.0, 2.0);
            let model_depth = (world - frame.camera_position.truncate()).dot(frame.sun_view.truncate());
            let water_depth = (world - u.camera.truncate()).dot(u.sun_view.truncate());
            assert_eq!(model_depth, water_depth);
            let model_pick = cascade_pick(model_depth, frame.sun_splits.to_array(), frame.local.w as usize, frame.sun_view.w);
            let water_pick = cascade_pick(water_depth, u.sun_splits.to_array(), u.extra[0] as usize, u.sun_view.w);
            assert_eq!(model_pick, water_pick, "{count} {i}");
            if let Some((cascade, _)) = water_pick {
                // The cascade's numbers the water samples with are the model shader's.
                let model_matrix = if cascade == 0 { frame.sun_matrix } else { frame.sun_cascade_matrices[cascade - 1] };
                assert_eq!(u.sun_cascade_matrices[cascade], model_matrix);
                assert_eq!((u.sun_biases[cascade], u.sun_offsets[cascade]), (frame.sun_biases[cascade], frame.sun_offsets[cascade]));
            }
        }
    }
}
