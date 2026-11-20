//! The lit-shapes layer: the built-in mesh primitives of codebrix-webgpu-graphics (a cube, a box,
//! spheres and a plane) on a checkered ground, lit by the MeshRenderer's key light and ambient
//! term, with depth, seen through a perspective or orthographic Camera that an OrbitController
//! drives from the canvas's input: drag (or one finger) to orbit - the scene follows the pointer -
//! the wheel to zoom, two fingers to pinch-zoom; it auto-orbits until the first drag. A ring of
//! small cubes is ONE instanced draw; one of them is mirrored (a negative scale) and still
//! shades like the others; one sphere is translucent and is drawn after the opaque shapes. It is
//! a LAYER of the canvas host: it loads the colour (never clears it), clears only its own depth
//! texture, and takes its time from the frame (it stands still while the clock is paused).

use codebrix_webgpu::glam::{Affine2, Mat4, Quat, Vec2, Vec3};
use codebrix_webgpu::{CanvasFrame, CanvasInput, CanvasRenderer, CanvasTarget, GpuContext};
use codebrix_webgpu_graphics::{
    Camera, Color4, DepthTextureOptions, GpuSampler, GpuTexture, GraphicsContext, GraphicsException, LitMaterial, MaterialOptions, Mesh, MeshRenderer, MeshShapes, OrbitCamera, OrbitController, ProjectionKind, RgbaTextureOptions, SamplerDescription, SceneLighting,
};
use dioxus::prelude::*;

/// The number of small cubes in the instanced ring.
pub const RING_CUBES: usize = 12;

/// What the page shows about the camera and the renderer.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct LitView {
    /// The frame number on the clock.
    pub frame: u64,
    /// The orbit's yaw, in radians.
    pub yaw: f32,
    /// The orbit's pitch, in radians.
    pub pitch: f32,
    /// The orbit's distance.
    pub distance: f32,
    /// True while the camera orbits by itself.
    pub auto_orbit: bool,
    /// True for the orthographic projection.
    pub orthographic: bool,
    /// The draw calls of the renderer's last end.
    pub draw_calls: usize,
    /// The instances of the renderer's last end.
    pub instances: usize,
    /// The pointers pressed on the canvas now.
    pub pointers: usize,
}

/// What the layer draws with.
struct Gpu {
    renderer: MeshRenderer,
    ground: Mesh,
    cube: Mesh,
    tall_box: Mesh,
    sphere: Mesh,
    small_cube: Mesh,
    ground_material: LitMaterial,
    cube_material: LitMaterial,
    box_material: LitMaterial,
    sphere_material: LitMaterial,
    glass_material: LitMaterial,
    ring_material: LitMaterial,
    // Kept alive with the materials that sample them.
    _checker: GpuTexture,
    _repeat: GpuSampler,
}

/// Draws the lit shapes.
pub struct LitShapesRenderer {
    graphics: GraphicsContext,
    gpu: Option<Gpu>,
    depth: Option<GpuTexture>,
    controller: OrbitController,
    camera: Camera,
    lighting: SceneLighting,
    view: Signal<LitView>,
    errors: Signal<Vec<String>>,
    orthographic: Signal<bool>,
    resume_requests: Signal<u64>,
    resumes_seen: u64,
}

impl LitShapesRenderer {
    /// A renderer drawing with `graphics`, publishing its view into `view` and every error into
    /// `errors`. `orthographic` chooses the projection; each increase of `resume_requests` starts
    /// the auto-orbit again.
    pub fn new(graphics: GraphicsContext, view: Signal<LitView>, errors: Signal<Vec<String>>, orthographic: Signal<bool>, resume_requests: Signal<u64>) -> LitShapesRenderer {
        let mut orbit = OrbitCamera::new();
        orbit.target = Vec3::new(0.0, 0.6, 0.0);
        orbit.set_distance(9.0);
        orbit.set_pitch(0.45);
        let lighting = SceneLighting { ambient_color: Color4::new(0.22, 0.24, 0.3, 1.0), light_color: Color4::new(1.0, 0.95, 0.86, 1.0), ..SceneLighting::default() };
        LitShapesRenderer { graphics, gpu: None, depth: None, controller: OrbitController::new(orbit), camera: Camera::new(), lighting, view, errors, orthographic, resume_requests, resumes_seen: 0 }
    }

    fn report(&mut self, error: String) {
        let mut errors = self.errors.write();
        if errors.len() < 20 {
            errors.push(error);
        }
    }
}

/// The ground's texture: a soft two-tone checker, sRGB, RGBA.
pub fn checker_pixels(size: u32) -> Vec<u8> {
    let mut pixels = Vec::with_capacity((size * size * 4) as usize);
    for y in 0..size {
        for x in 0..size {
            let dark = (x * 2 / size + y * 2 / size) % 2 == 1;
            let pixel: [u8; 3] = if dark { [92, 140, 74] } else { [128, 178, 104] };
            pixels.extend_from_slice(&[pixel[0], pixel[1], pixel[2], 255]);
        }
    }
    pixels
}

/// Makes the renderer, the meshes, the texture and the materials.
fn create_gpu(graphics: &GraphicsContext) -> Result<Gpu, GraphicsException> {
    let renderer = MeshRenderer::new(graphics)?;
    let ground = graphics.create_mesh(&MeshShapes::plane(24.0, 24.0, None, 1.0), Some("ground"))?;
    let cube = graphics.create_mesh(&MeshShapes::cube(1.4, None), Some("cube"))?;
    let tall_box = graphics.create_mesh(&MeshShapes::box_shape(Vec3::new(0.8, 2.4, 0.8), None), Some("tall box"))?;
    let sphere = graphics.create_mesh(&MeshShapes::sphere(0.8, 40, 20, None), Some("sphere"))?;
    let small_cube = graphics.create_mesh(&MeshShapes::cube(0.35, None), Some("small cube"))?;
    let checker = graphics.create_texture_from_rgba(64, 64, &checker_pixels(64), RgbaTextureOptions { mipmaps: true, srgb: true, label: Some("ground checker".to_string()) })?;
    let repeat = graphics.create_sampler(&SamplerDescription::linear_repeat());
    let ground_material = renderer.create_material(Color4::WHITE, MaterialOptions { base_color_texture: Some(&checker), sampler: Some(&repeat), label: Some("ground"), ..MaterialOptions::default() })?;
    ground_material.set_specular_strength(0.05);
    // The checker repeats 12 times across the ground through the material's texture transform.
    ground_material.set_texture_transform(Affine2::from_scale(Vec2::splat(12.0)));
    let cube_material = renderer.create_material(Color4::new(0.85, 0.3, 0.2, 1.0), MaterialOptions { label: Some("cube"), ..MaterialOptions::default() })?;
    let box_material = renderer.create_material(Color4::new(0.95, 0.75, 0.25, 1.0), MaterialOptions { label: Some("tall box"), ..MaterialOptions::default() })?;
    box_material.set_shininess(8.0);
    let sphere_material = renderer.create_material(Color4::new(0.25, 0.45, 0.9, 1.0), MaterialOptions { label: Some("sphere"), ..MaterialOptions::default() })?;
    sphere_material.set_shininess(64.0);
    sphere_material.set_specular_strength(0.6);
    let glass_material = renderer.create_material(Color4::new(0.7, 0.95, 1.0, 0.45), MaterialOptions { label: Some("translucent sphere"), ..MaterialOptions::default() })?;
    let ring_material = renderer.create_material(Color4::new(0.9, 0.9, 0.95, 1.0), MaterialOptions { label: Some("ring"), ..MaterialOptions::default() })?;
    Ok(Gpu { renderer, ground, cube, tall_box, sphere, small_cube, ground_material, cube_material, box_material, sphere_material, glass_material, ring_material, _checker: checker, _repeat: repeat })
}

/// The world matrices of the ring of small cubes at `time`; the one at index 3 is mirrored.
pub fn ring_worlds(time: f32) -> Vec<Mat4> {
    (0..RING_CUBES)
        .map(|index| {
            let angle = index as f32 * std::f32::consts::TAU / RING_CUBES as f32 + time * 0.25;
            let position = Vec3::new(angle.cos() * 4.2, 0.175 + 0.15 * (time * 2.0 + index as f32).sin().abs(), angle.sin() * 4.2);
            let scale = if index == 3 { Vec3::new(-1.0, 1.0, 1.0) } else { Vec3::ONE };
            Mat4::from_scale_rotation_translation(scale, Quat::from_rotation_y(-angle + time), position)
        })
        .collect()
}

impl LitShapesRenderer {
    /// Draws the scene with the renderer's begin/draw/end.
    fn draw(gpu: &mut Gpu, pass: &mut codebrix_webgpu_graphics::RenderPass<'_>, camera: &Camera, lighting: &SceneLighting, time: f32) -> Result<(), GraphicsException> {
        let renderer = &mut gpu.renderer;
        renderer.begin(pass, camera, Some(lighting))?;
        renderer.draw(&gpu.ground, &gpu.ground_material, Mat4::IDENTITY)?;
        renderer.draw(&gpu.cube, &gpu.cube_material, Mat4::from_rotation_translation(Quat::from_rotation_y(time * 0.6) * Quat::from_rotation_x(0.3), Vec3::new(-1.8, 1.1, 0.4)))?;
        renderer.draw(&gpu.tall_box, &gpu.box_material, Mat4::from_translation(Vec3::new(1.9, 1.2, -1.2)))?;
        renderer.draw(&gpu.sphere, &gpu.sphere_material, Mat4::from_translation(Vec3::new(0.9, 0.8 + 0.25 * (time * 1.3).sin().abs(), 1.6)))?;
        renderer.draw(&gpu.sphere, &gpu.glass_material, Mat4::from_scale_rotation_translation(Vec3::splat(0.9), Quat::IDENTITY, Vec3::new(-0.6, 0.75, -2.2)))?;
        renderer.draw_instanced(&gpu.small_cube, &gpu.ring_material, &ring_worlds(time))?;
        renderer.end(pass)
    }
}

impl CanvasRenderer for LitShapesRenderer {
    fn prepare(&mut self, _context: &GpuContext, _target: &CanvasTarget) {
        match create_gpu(&self.graphics) {
            Ok(gpu) => self.gpu = Some(gpu),
            Err(error) => self.report(error.to_string()),
        }
    }

    fn resize(&mut self, _context: &GpuContext, width: u32, height: u32) {
        match self.graphics.create_depth_texture(width, height, DepthTextureOptions { label: Some("lit shapes depth".to_string()), ..DepthTextureOptions::default() }) {
            Ok(depth) => self.depth = Some(depth),
            Err(error) => self.report(error.to_string()),
        }
    }

    fn render(&mut self, frame: &mut CanvasFrame<'_>) {
        let time = frame.info.time;
        let number = frame.info.number;
        let resumes = *self.resume_requests.peek();
        if resumes != self.resumes_seen {
            self.resumes_seen = resumes;
            self.controller.set_auto_orbit(true);
        }
        self.controller.update(time);
        self.controller.apply(&mut self.camera);
        let orthographic = *self.orthographic.peek();
        self.camera.projection = if orthographic { ProjectionKind::Orthographic } else { ProjectionKind::Perspective };
        self.camera.orthographic_height = self.controller.orbit.distance() * 0.9;
        let graphics = self.graphics.clone();
        let mut failure = None;
        let mut counts = (0, 0);
        if let (Some(gpu), Some(depth)) = (self.gpu.as_mut(), self.depth.as_ref()) {
            let mut graphics_frame = graphics.begin_canvas_frame(frame, Some(depth));
            // None: LOAD the colour the host and the layers below drew; Some(1.0): clear OUR depth.
            match graphics_frame.begin_render_pass(None, Some(1.0)) {
                Ok(mut pass) => {
                    if let Err(error) = LitShapesRenderer::draw(gpu, &mut pass, &self.camera, &self.lighting, time as f32) {
                        failure = Some(error.to_string());
                    }
                    counts = (gpu.renderer.last_draw_call_count(), gpu.renderer.last_instance_count());
                }
                Err(error) => failure = Some(error.to_string()),
            };
        }
        if let Some(error) = failure {
            self.report(error);
        }
        for error in graphics.take_errors() {
            self.report(error.to_string());
        }
        let orbit = self.controller.orbit;
        let view = LitView {
            frame: number,
            yaw: orbit.yaw,
            pitch: orbit.pitch(),
            distance: orbit.distance(),
            auto_orbit: self.controller.is_auto_orbiting(),
            orthographic,
            draw_calls: counts.0,
            instances: counts.1,
            pointers: self.controller.pressed_pointer_count(),
        };
        if *self.view.peek() != view {
            self.view.set(view);
        }
    }

    /// Drag (or one finger): orbit, the scene following the pointer. Wheel: zoom. Two fingers:
    /// pinch-zoom.
    fn input(&mut self, input: &CanvasInput) {
        self.controller.input(input);
    }
}
