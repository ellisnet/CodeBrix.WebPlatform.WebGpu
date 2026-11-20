//! The layer of the two validation scenes ("Skinned model" and "Morph targets"): ONE model from
//! the backend's model data, one instance of it playing a clip on the host's clock, drawn by the
//! ModelRenderer - which skins it on the GPU from the instance's joint palette, or morphs it from
//! its morph weights - lit by the context's default environment (the procedural sky with its sun)
//! and drawn over the skybox, under an orbit camera framed on the model's bounds.
//!
//! It takes its time from the frame: the player follows the frame's TIME
//! (AnimationPlayer::update_to), so pausing the clock holds the pose, resuming goes on from it,
//! and a seek of the clock moves the clip to the matching place. It draws straight into the
//! canvas frame (the sRGB view encodes; no HDR target), loading the colour and clearing only its
//! own depth.

use std::rc::Rc;

use codebrix_webgpu::glam::Vec3;
use codebrix_webgpu::{wgpu, CanvasFrame, CanvasInput, CanvasRenderer, CanvasTarget, GpuContext};
use codebrix_webgpu_graphics::{Camera, DepthTextureOptions, GpuTexture, GraphicsContext, ModelInstance, ModelLoadOptions, ModelRenderer, OrbitCamera, OrbitController, OrbitLimits, PlayOptions, SceneLighting, SkyboxRenderer};
use dioxus::prelude::*;

/// What the layer reads out of the pose for the page: a node's world position (the skinned
/// archer's arm), or a node's morph weights (the morph shape).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Probe {
    /// The world position of the node of this name.
    Node(&'static str),
    /// The morph weights of the first node that has any.
    MorphWeights,
}

/// What the page shows about the layer.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ModelView {
    /// The frame number on the clock.
    pub frame: u64,
    /// The frame's time on the clock.
    pub time: f64,
    /// The model's clips, in order.
    pub clips: Vec<String>,
    /// The clip playing (the newest state's), and its playhead in seconds.
    pub clip: String,
    /// The playhead of the clip, in seconds.
    pub clip_time: f32,
    /// The clips playing (2 during a cross-fade).
    pub states: usize,
    /// The probe's value as text.
    pub probe: String,
    /// The model's skins, joints, morph targets and triangles.
    pub skins: usize,
    /// The joints of all its skins.
    pub joints: usize,
    /// The morph targets of all its primitives.
    pub morph_targets: usize,
    /// The triangles of one frame.
    pub triangles: u32,
    /// The draw calls of the renderer's last end.
    pub draw_calls: usize,
    /// The pipelines the renderer has made.
    pub pipelines: usize,
}

/// What the layer draws with.
struct Gpu {
    renderer: ModelRenderer,
    skybox: SkyboxRenderer,
    instance: ModelInstance,
    probe_node: Option<usize>,
}

/// Draws one animated model.
pub struct ModelLayer {
    graphics: GraphicsContext,
    bytes: Rc<Vec<u8>>,
    clip: Signal<String>,
    probe: Probe,
    gpu: Option<Gpu>,
    depth: Option<GpuTexture>,
    controller: OrbitController,
    camera: Camera,
    playing: String,
    view: Signal<ModelView>,
    errors: Signal<Vec<String>>,
}

/// The depth format of the layer.
const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth24Plus;

impl ModelLayer {
    /// A layer for the one model in `bytes`, playing the clip `clip` names (read every frame: a
    /// change cross-fades to the new clip), reading out `probe`; it publishes its view into
    /// `view` and every error into `errors`.
    pub fn new(graphics: GraphicsContext, bytes: Rc<Vec<u8>>, clip: Signal<String>, probe: Probe, view: Signal<ModelView>, errors: Signal<Vec<String>>) -> ModelLayer {
        let mut camera = Camera::new();
        camera.field_of_view = 40f32.to_radians();
        ModelLayer { graphics, bytes, clip, probe, gpu: None, depth: None, controller: OrbitController::new(OrbitCamera::new()), camera, playing: String::new(), view, errors }
    }

    fn report(&mut self, error: String) {
        let mut errors = self.errors.write();
        if errors.len() < 20 {
            errors.push(error);
        }
    }

    /// Loads the model, makes its instance, starts the clip and frames the camera on it.
    fn create_gpu(&mut self) -> Result<Gpu, String> {
        let model = self.graphics.load_model(&self.bytes, &ModelLoadOptions::default()).map_err(|e| e.to_string())?;
        let mut instance = model.create_instance();
        let clip = self.clip.peek().clone();
        instance.animation().play_named(&clip, PlayOptions::default()).map_err(|e| e.to_string())?;
        self.playing = clip;
        // Frame the model: the orbit's target is its centre, the distance a little over twice
        // its radius; it turns slowly by itself until the first drag.
        let center = model.bounds_center();
        let radius = ((model.bounds_max() - model.bounds_min()).length() * 0.5).max(0.01);
        let mut orbit = OrbitCamera::new();
        orbit.target = center;
        let mut controller = OrbitController::new(orbit);
        controller.limits = OrbitLimits { min_distance: radius * 1.2, max_distance: radius * 12.0, ..controller.limits };
        controller.orbit.set_distance(radius * 3.2);
        controller.orbit.set_pitch(0.25);
        controller.auto_orbit_start_yaw = 0.6;
        controller.auto_orbit_speed = 0.25;
        self.controller = controller;
        self.camera.near_plane = radius * 0.05;
        self.camera.far_plane = radius * 60.0;
        let probe_node = match self.probe {
            Probe::Node(name) => model.find_node(name),
            Probe::MorphWeights => model.nodes().iter().position(|n| !n.default_morph_weights.is_empty()),
        };
        let renderer = self.graphics.create_model_renderer().map_err(|e| e.to_string())?;
        let skybox = self.graphics.create_skybox_renderer().map_err(|e| e.to_string())?;
        Ok(Gpu { renderer, skybox, instance, probe_node })
    }
}

impl CanvasRenderer for ModelLayer {
    fn prepare(&mut self, _context: &GpuContext, _target: &CanvasTarget) {
        match self.create_gpu() {
            Ok(gpu) => self.gpu = Some(gpu),
            Err(error) => self.report(error),
        }
        for error in self.graphics.take_errors() {
            self.report(error.to_string());
        }
    }

    fn resize(&mut self, _context: &GpuContext, width: u32, height: u32) {
        match self.graphics.create_depth_texture(width, height, DepthTextureOptions { format: DEPTH_FORMAT, label: Some("model scene depth".to_string()) }) {
            Ok(depth) => self.depth = Some(depth),
            Err(error) => self.report(error.to_string()),
        }
    }

    fn render(&mut self, frame: &mut CanvasFrame<'_>) {
        let time = frame.info.time;
        let number = frame.info.number;
        self.controller.update(time);
        self.controller.apply(&mut self.camera);
        let wanted = self.clip.peek().clone();
        let mut failures = Vec::new();
        let mut view = ModelView { frame: number, time: (time * 1000.0).round() / 1000.0, ..ModelView::default() };
        if let (Some(gpu), Some(depth)) = (self.gpu.as_mut(), self.depth.as_ref()) {
            {
                let mut player = gpu.instance.animation();
                if wanted != self.playing {
                    // A new clip from the page: cross-fade to it over a third of a second.
                    match player.cross_fade_named(&wanted, 0.3, PlayOptions::default()) {
                        Ok(_) => self.playing = wanted.clone(),
                        Err(error) => failures.push(error.to_string()),
                    }
                }
                // The pose at the frame's time.
                player.update_to(time);
                if let Some(state) = player.get_state(player.state_count().saturating_sub(1)) {
                    view.clip = state.name().unwrap_or("").to_string();
                    view.clip_time = (state.time() * 1000.0).round() / 1000.0;
                }
                view.states = player.state_count();
            }
            let lighting = SceneLighting::default();
            let graphics = self.graphics.clone();
            let result: Result<(), codebrix_webgpu_graphics::GraphicsException> = (|| {
                let mut graphics_frame = graphics.begin_canvas_frame(frame, Some(depth));
                let mut pass = graphics_frame.begin_render_pass(None, Some(1.0))?;
                gpu.renderer.begin(&pass, &self.camera, Some(&lighting))?;
                gpu.renderer.draw(&gpu.instance)?;
                gpu.renderer.end(&mut pass)?;
                gpu.skybox.draw(&mut pass, &self.camera, Some(&lighting))?;
                Ok(())
            })();
            if let Err(error) = result {
                failures.push(error.to_string());
            }
            let model = gpu.instance.model().clone();
            view.clips = model.animations().iter().map(|a| a.name.clone().unwrap_or_else(|| a.index.to_string())).collect();
            view.skins = model.skins().len();
            view.joints = model.skins().iter().map(|s| s.joint_count()).sum();
            view.morph_targets = model.meshes().iter().flat_map(|m| m.primitives.iter()).map(|p| p.morph_targets.len()).sum();
            view.triangles = model.triangle_count();
            view.draw_calls = gpu.renderer.last_draw_call_count();
            view.pipelines = gpu.renderer.pipeline_count();
            view.probe = match (self.probe, gpu.probe_node) {
                (Probe::Node(name), Some(node)) => {
                    let p: Vec3 = gpu.instance.get_node_world_matrix(node).w_axis.truncate();
                    format!("{name} at ({:.4}, {:.4}, {:.4})", p.x, p.y, p.z)
                }
                (Probe::MorphWeights, Some(node)) => {
                    let weights: Vec<String> = gpu.instance.get_morph_weights(node).iter().map(|w| format!("{w:.4}")).collect();
                    format!("weights {}", weights.join(" "))
                }
                (Probe::Node(name), None) => format!("the model has no node named '{name}'"),
                (Probe::MorphWeights, None) => "the model has no morph weights".to_string(),
            };
        }
        for error in failures {
            self.report(error);
        }
        for error in self.graphics.take_errors() {
            self.report(error.to_string());
        }
        if *self.view.peek() != view {
            self.view.set(view);
        }
    }

    /// Drag (or one finger): orbit. Wheel: zoom. Two fingers: pinch-zoom.
    fn input(&mut self, input: &CanvasInput) {
        self.controller.input(input);
    }
}
