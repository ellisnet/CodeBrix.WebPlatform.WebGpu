//! The viewer layer: the desktop model viewer's COMPLETE scene - the ground, the static props and
//! the animated actors (the chest opening and closing, the animals walking a ring or
//! cross-fading through their routine, the Avian circling overhead; viewer_actors.rs), the kits'
//! .glb files READ IN THE BROWSER by the graphics crate's glTF loader into ONE GltfModelSet (in two
//! loads - the actors' models, then the props' - which share each kit's colour map: it is decoded
//! and uploaded once), and the ground, built in code on the backend, from its model data blob;
//! placed as ModelInstances, animated by each instance's AnimationPlayer on the host's clock and
//! drawn by the ModelRenderer with physically based shading - lit the way the desktop viewer
//! lights it:
//!
//! - IMAGE-BASED LIGHTING from an environment: the viewer's procedural late-morning sky (baked
//!   once on the GPU, its sun matching the key light), or - switched by a button - the Poly Haven
//!   "Small Cathedral" panorama, whose Radiance .hdr bytes the page fetched from the backend and
//!   the graphics crate decoded and baked (the sun light is off then: the panorama carries its own
//!   light);
//! - the SKYBOX drawn between the opaque and the blended parts of the models (the renderer's
//!   `end_opaque`, the sky, then `end_blended`: the sky fills only what nothing opaque covered, and
//!   translucent parts blend over it); `?no-skybox` clears to the flat sky colour instead;
//! - the HDR PATH: the scene into an RGBA16Float HeadlessRenderTarget with its own depth, then the
//!   ToneMapper (Khronos PBR Neutral by default, as the desktop viewer) into the canvas frame, with
//!   the operator and the exposure the page's controls choose (`?ldr` draws straight into the
//!   canvas instead, with no tone mapping).
//!
//! The camera is the viewer's orbit camera (target, distance, pitch, field of view) driven by the
//! OrbitController: it orbits by itself as the desktop viewer does (0.7 + 0.18 t) until the first
//! drag; a right-button, middle-button or Shift drag pans (the controller is told the canvas's size
//! every frame, so the scene follows the pointer). It is a LAYER of the canvas host: it takes its time from the frame; in the LDR path it
//! loads the colour and clears only its own depth; in the HDR path its tone-map pass covers the
//! whole canvas (the scene is opaque), loading what is there first.

use std::rc::Rc;

use codebrix_webgpu::glam::{Mat4, Quat, Vec3};
use codebrix_webgpu::{wgpu, CanvasFrame, CanvasInput, CanvasRenderer, CanvasTarget, GpuContext};
use codebrix_webgpu_graphics::{Camera, Color4, DepthTextureOptions, EnvironmentMap, EnvironmentOptions, GltfLoadOptions, GltfModelSet, GpuTexture, GraphicsContext, HeadlessRenderTarget, HeadlessTargetOptions, Model, ModelInstance, ModelLoadOptions, ModelRenderer, OrbitCamera, OrbitController, ProceduralSkyOptions, SceneLighting, SkyboxRenderer, ToneMapOperator, ToneMapper};
use dioxus::prelude::*;

use crate::generated::models::{SceneVector, ViewerScene as ViewerSceneData};
use crate::model_files::{self, ModelFiles};
use crate::viewer_actors::ViewerActors;

/// The bytes of the scene's models as the page fetched them: the model files (parsed, with the
/// files they name) and the blobs of the models built in code, by their index in the scene.
#[derive(Clone, PartialEq)]
pub struct ViewerModels {
    /// The model files, in the order of the scene's models that have a file.
    pub files: ModelFiles,
    /// The models built in code: (index in the scene's models, the blob's bytes).
    pub blobs: Rc<Vec<(usize, Rc<Vec<u8>>)>>,
}

impl ViewerModels {
    /// The bytes fetched from the backend for the models: every model file, every file they name
    /// and every blob.
    pub fn download_bytes(&self) -> usize {
        self.files.bytes + self.blobs.iter().map(|(_, bytes)| bytes.len()).sum::<usize>()
    }
}

/// How the viewer renders: the desktop viewer's `--ldr` and `--no-skybox` switches, read from
/// the page's query string (`?ldr`, `?no-skybox`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ViewerOptions {
    /// True (the default): the scene into an RGBA16Float target, tone mapped into the canvas.
    pub hdr: bool,
    /// True (the default): the environment as the background; false: the flat sky colour.
    pub skybox: bool,
}

impl Default for ViewerOptions {
    fn default() -> ViewerOptions {
        ViewerOptions { hdr: true, skybox: true }
    }
}

/// Which environment lights the scene.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum EnvironmentChoice {
    /// The viewer's procedural sky (the default).
    #[default]
    ProceduralSky,
    /// The Small Cathedral HDR panorama from the backend.
    Cathedral,
}

/// Where the panorama's bytes are.
#[derive(Clone, Debug, Default, PartialEq)]
pub enum PanoramaBytes {
    /// Not asked for yet (it is fetched the first time the page switches to it).
    #[default]
    NotRequested,
    /// The request is on its way.
    Loading,
    /// The bytes arrived.
    Loaded(Rc<Vec<u8>>),
    /// The request failed, with the message to show.
    Failed(String),
}

/// What the page's controls set, read by the layer every frame.
#[derive(Clone, Copy)]
pub struct ViewerControls {
    /// The environment to light with.
    pub environment: Signal<EnvironmentChoice>,
    /// The panorama's bytes once fetched.
    pub panorama: Signal<PanoramaBytes>,
    /// The tone mapper's curve.
    pub operator: Signal<ToneMapOperator>,
    /// The tone mapper's exposure.
    pub exposure: Signal<f32>,
    /// Each increase starts the auto-orbit again.
    pub resume_requests: Signal<u64>,
}

/// What the page shows about the viewer.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ViewerView {
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
    /// The models loaded (from the model files and the blobs).
    pub models: usize,
    /// The GPU textures of the model files (each kit's colour map once).
    pub textures: usize,
    /// Whether the two loads of the scene's model files share their textures ("ok: ..."), or why not.
    pub texture_sharing: String,
    /// How long building and uploading the models took (milliseconds of the page's clock).
    pub build_ms: f64,
    /// The instances placed.
    pub instances: usize,
    /// The triangles of one frame.
    pub triangles: u64,
    /// The draw calls of the renderer's last end.
    pub draw_calls: usize,
    /// The primitive instances of the renderer's last end.
    pub drawn: usize,
    /// The pipelines the renderer has made.
    pub pipelines: usize,
    /// What lights the scene ("procedural sky", "Small Cathedral (HDR panorama)").
    pub environment: String,
    /// The environment's radiance cube size, in texels per face.
    pub environment_size: u32,
    /// The CPU time of building the last environment (decoding and recording the bake; the GPU
    /// does the baking afterwards), in milliseconds.
    pub environment_build_ms: f64,
    /// The skybox draws so far.
    pub skybox_draws: u64,
    /// The tone mapper's operator as drawn in this frame, None in the LDR path.
    pub operator: Option<ToneMapOperator>,
    /// The tone mapper's exposure as drawn in this frame.
    pub exposure: f32,
    /// The HDR target's size, or (0, 0) in the LDR path.
    pub hdr_size: (u32, u32),
    /// The scene time of the frame (the host clock's), in seconds.
    pub scene_time: f64,
    /// The animated actors.
    pub actors: usize,
    /// The clips playing on all of them together.
    pub clips_playing: usize,
    /// A fingerprint of the actors' pose (changes whenever an animated node moves).
    pub pose_signature: f64,
    /// The Avian's world position.
    pub bird: (f32, f32, f32),
    /// The first standing animal's routine: its clips and their weights.
    pub routine: String,
    /// How often the actors were re-posed after a jump of the clock.
    pub seeks: u64,
}

/// What the layer draws with.
struct Gpu {
    renderer: ModelRenderer,
    instances: Vec<ModelInstance>,
    actors: ViewerActors,
    skybox: SkyboxRenderer,
    tone_mapper: ToneMapper,
    sky: EnvironmentMap,
    cathedral: Option<EnvironmentMap>,
}

/// Draws the viewer's scene.
pub struct ViewerRenderer {
    graphics: GraphicsContext,
    loaded: ViewerModels,
    scene: ViewerSceneData,
    options: ViewerOptions,
    controls: ViewerControls,
    gpu: Option<Gpu>,
    depth: Option<GpuTexture>,
    hdr: Option<HeadlessRenderTarget>,
    controller: OrbitController,
    camera: Camera,
    view: Signal<ViewerView>,
    errors: Signal<Vec<String>>,
    resumes_seen: u64,
    triangles: u64,
    models: usize,
    textures: usize,
    texture_sharing: String,
    build_ms: f64,
    environment_build_ms: f64,
    cathedral_failed: bool,
}

/// The depth format of the scene (the desktop viewer's).
const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth24Plus;

impl ViewerRenderer {
    /// A renderer for the scene `scene` describes, whose models' bytes are in `loaded`, rendering
    /// as `options` say and reading `controls` every frame; it publishes its view into `view` and
    /// every error into `errors`.
    pub fn new(graphics: GraphicsContext, loaded: ViewerModels, scene: ViewerSceneData, options: ViewerOptions, controls: ViewerControls, view: Signal<ViewerView>, errors: Signal<Vec<String>>) -> ViewerRenderer {
        let mut orbit = OrbitCamera::new();
        orbit.target = vec3(&scene.camera.target);
        orbit.set_distance(scene.camera.distance);
        orbit.set_pitch(scene.camera.pitch);
        let mut controller = OrbitController::new(orbit);
        controller.auto_orbit_start_yaw = scene.camera.yaw_start;
        controller.auto_orbit_speed = scene.camera.yaw_per_second;
        let mut camera = Camera::new();
        camera.field_of_view = scene.camera.field_of_view_degrees.to_radians();
        camera.near_plane = scene.camera.near_plane;
        camera.far_plane = scene.camera.far_plane;
        ViewerRenderer { graphics, loaded, scene, options, controls, gpu: None, depth: None, hdr: None, controller, camera, view, errors, resumes_seen: 0, triangles: 0, models: 0, textures: 0, texture_sharing: String::new(), build_ms: 0.0, environment_build_ms: 0.0, cathedral_failed: false }
    }

    fn report(&mut self, error: String) {
        let mut errors = self.errors.write();
        if errors.len() < 20 {
            errors.push(error);
        }
    }

    /// Makes the GPU models of the scene, in the scene's model order: the models built in code
    /// from their blobs, the model files through ONE GltfModelSet in TWO calls - the actors' models
    /// first, then the props' - so the second call reuses the textures the first uploaded (each
    /// kit's colour map is decoded and uploaded once for the whole scene).
    fn create_models(&mut self) -> Result<Vec<Model>, String> {
        let started = model_files::now_ms();
        let count = self.scene.models.len();
        let mut models: Vec<Option<Model>> = (0..count).map(|_| None).collect();
        for (index, bytes) in self.loaded.blobs.iter() {
            let model = self.graphics.load_model(bytes, &ModelLoadOptions::default()).map_err(|e| e.to_string())?;
            if let Some(slot) = models.get_mut(*index) {
                *slot = Some(model);
            }
        }
        // The scene's models with a file, in the order the files were fetched (scene order).
        let with_file: Vec<usize> = self.scene.models.iter().filter(|m| !m.file.is_empty()).map(|m| m.index as usize).collect();
        let actor_models: Vec<usize> = self.scene.actors.iter().map(|a| a.model_index as usize).collect();
        let (first, second): (Vec<usize>, Vec<usize>) = (0..with_file.len()).partition(|&k| actor_models.contains(&with_file[k]));
        let mut set = GltfModelSet::new();
        let mut made: Vec<(usize, Model)> = Vec::new();
        for batch in [&first, &second] {
            model_files::add_models(&mut set, &self.loaded.files, batch, &GltfLoadOptions::default())?;
            let gpu = set.create_models(&self.graphics, &ModelLoadOptions::default()).map_err(|e| e.to_string())?;
            for (k, model) in batch.iter().zip(gpu) {
                made.push((with_file[*k], model));
            }
        }
        // THE PROOF of texture sharing across the two calls: a model of the first load and one of
        // the second from the same kit hold the SAME GPU texture.
        let pack_of = |index: usize| self.scene.models.get(index).map(|m| m.pack.clone()).unwrap_or_default();
        let first_load: Vec<&(usize, Model)> = made.iter().filter(|(index, _)| actor_models.contains(index)).collect();
        let second_load: Vec<&(usize, Model)> = made.iter().filter(|(index, _)| !actor_models.contains(index)).collect();
        self.texture_sharing = match second_load.iter().find_map(|(b, mb)| first_load.iter().find(|(a, _)| pack_of(*a) == pack_of(*b)).map(|(a, ma)| (*a, ma, *b, mb))) {
            Some((a, ma, b, mb)) => match (ma.textures().first(), mb.textures().first()) {
                (Some(ta), Some(tb)) if Rc::ptr_eq(ta, tb) => format!("ok: '{}' (first load) and '{}' (second load) hold the same texture of {}", self.scene.models[a].name, self.scene.models[b].name, pack_of(a)),
                (Some(_), Some(_)) => format!("NOT SHARED: '{}' and '{}' hold different textures", self.scene.models[a].name, self.scene.models[b].name),
                _ => "a model of the pair has no texture".to_string(),
            },
            None => "no kit has a model in both loads".to_string(),
        };
        self.textures = set.texture_count();
        for (index, model) in made {
            if let Some(slot) = models.get_mut(index) {
                *slot = Some(model);
            }
        }
        self.build_ms = ((model_files::now_ms() - started) * 10.0).round() / 10.0;
        models.into_iter().enumerate().map(|(index, model)| model.ok_or_else(|| format!("The scene's model {index} was neither a model file nor a blob."))).collect()
    }

    /// Loads the models, places the instances, bakes the procedural sky and makes the skybox and
    /// tone-map renderers.
    fn create_gpu(&mut self) -> Result<Gpu, String> {
        let models = self.create_models()?;
        let mut instances = Vec::with_capacity(self.scene.placements.len());
        let mut triangles = 0u64;
        for placement in &self.scene.placements {
            let Some(model) = models.get(placement.model_index as usize) else {
                return Err(format!("The placement of '{}' names model {} of the scene, which has {}.", placement.model, placement.model_index, models.len()));
            };
            // The desktop viewer's order: scale, then the turn about Y, then the translation.
            let world = Mat4::from_scale_rotation_translation(Vec3::splat(placement.scale), Quat::from_rotation_y(placement.rotation_y), vec3(&placement.translation));
            triangles += model.triangle_count() as u64;
            instances.push(model.create_instance_at(world));
        }
        // The animated actors follow the static placements; each is posed and placed every frame.
        let first_actor = instances.len();
        for actor in &self.scene.actors {
            let Some(model) = models.get(actor.model_index as usize) else {
                return Err(format!("The actor '{}' names model {} of the scene, which has {}.", actor.model, actor.model_index, models.len()));
            };
            triangles += model.triangle_count() as u64;
            instances.push(model.create_instance_at(ViewerActors::world_at(actor, 0.0)));
        }
        let actors = ViewerActors::new(self.scene.actors.clone(), first_actor, self.scene.routine_seconds, self.scene.cross_fade_seconds);
        let renderer = self.graphics.create_model_renderer().map_err(|e| e.to_string())?;
        // The desktop viewer's late-morning sky, baked once into an environment: its bright side
        // matches the sun light, its ground half is a dark meadow green.
        let light = &self.scene.lighting;
        let sky_options = ProceduralSkyOptions {
            sun_direction: vec3(&light.sun_direction),
            zenith_color: color(&light.sky_zenith_color),
            horizon_color: color(&light.sky_horizon_color),
            ground_color: color(&light.sky_ground_color),
            turbidity: light.sky_turbidity,
            ..ProceduralSkyOptions::default()
        };
        let started = now_ms();
        let sky = self.graphics.create_procedural_sky(Some(&sky_options), Some(&EnvironmentOptions { label: Some("viewer sky".to_string()), ..EnvironmentOptions::default() })).map_err(|e| e.to_string())?;
        self.environment_build_ms = now_ms() - started;
        let skybox = self.graphics.create_skybox_renderer().map_err(|e| e.to_string())?;
        let mut tone_mapper = self.graphics.create_tone_mapper().map_err(|e| e.to_string())?;
        // Khronos PBR Neutral keeps the kits' flat colours true and only rolls off the highlights.
        tone_mapper.operator = *self.controls.operator.peek();
        self.triangles = triangles;
        self.models = models.len();
        Ok(Gpu { renderer, instances, actors, skybox, tone_mapper, sky, cathedral: None })
    }

    /// The lighting for the chosen environment, building the panorama's environment the first
    /// time it is chosen and its bytes are there. None while the bytes are on their way.
    fn lighting(&mut self) -> Option<(SceneLighting, String, u32)> {
        let light = self.scene.lighting.clone();
        let choice = *self.controls.environment.peek();
        let gpu = self.gpu.as_mut()?;
        if choice == EnvironmentChoice::Cathedral && gpu.cathedral.is_none() && !self.cathedral_failed {
            let PanoramaBytes::Loaded(bytes) = self.controls.panorama.peek().clone() else {
                return None;
            };
            let started = now_ms();
            match self.graphics.create_environment_from_bytes(&bytes, Some(&EnvironmentOptions { label: Some("small cathedral".to_string()), ..EnvironmentOptions::default() })) {
                Ok(environment) => {
                    gpu.cathedral = Some(environment);
                    self.environment_build_ms = now_ms() - started;
                }
                Err(error) => {
                    self.cathedral_failed = true;
                    self.report(format!("The panorama could not be made into an environment: {error}"));
                    return None;
                }
            }
        }
        let gpu = self.gpu.as_ref()?;
        match (choice, &gpu.cathedral) {
            (EnvironmentChoice::Cathedral, Some(cathedral)) => {
                // The desktop viewer with --environment: the panorama carries its own light, so
                // the sun light is off; a little more exposure.
                let lighting = SceneLighting { light_intensity: 0.0, environment: Some(cathedral.clone()), exposure: light.panorama_exposure, ..SceneLighting::default() };
                Some((lighting, "Small Cathedral (HDR panorama)".to_string(), cathedral.size()))
            }
            (EnvironmentChoice::Cathedral, None) => None,
            (EnvironmentChoice::ProceduralSky, _) => {
                let lighting = SceneLighting {
                    light_direction: vec3(&light.sun_direction),
                    light_color: color(&light.light_color),
                    light_intensity: light.light_intensity,
                    environment: Some(gpu.sky.clone()),
                    environment_intensity: 1.0,
                    exposure: 1.0,
                    ..SceneLighting::default()
                };
                Some((lighting, "procedural sky".to_string(), gpu.sky.size()))
            }
        }
    }

    /// The flat sky colour as linear light (the HDR target's clear without the skybox).
    fn flat_sky_linear(&self) -> Color4 {
        let light = &self.scene.lighting;
        Color4::new(srgb_to_linear(light.sky_red), srgb_to_linear(light.sky_green), srgb_to_linear(light.sky_blue), 1.0)
    }
}

fn vec3(value: &SceneVector) -> Vec3 {
    Vec3::new(value.x, value.y, value.z)
}

fn color(value: &SceneVector) -> Color4 {
    Color4::new(value.x, value.y, value.z, 1.0)
}

/// An sRGB byte as linear light.
pub fn srgb_to_linear(byte: i32) -> f32 {
    let c = byte as f32 / 255.0;
    if c <= 0.04045 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

/// The page's clock in milliseconds (for the build-time read-out only; the scene's time comes
/// from the frame).
fn now_ms() -> f64 {
    web_sys::window().and_then(|w| w.performance()).map_or(0.0, |p| p.now())
}

impl CanvasRenderer for ViewerRenderer {
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
        if self.options.hdr {
            let options = HeadlessTargetOptions { format: wgpu::TextureFormat::Rgba16Float, with_depth: true, depth_format: DEPTH_FORMAT };
            match self.graphics.create_headless_target(width, height, options) {
                Ok(hdr) => self.hdr = Some(hdr),
                Err(error) => self.report(error.to_string()),
            }
        } else {
            match self.graphics.create_depth_texture(width, height, DepthTextureOptions { format: DEPTH_FORMAT, label: Some("viewer depth".to_string()) }) {
                Ok(depth) => self.depth = Some(depth),
                Err(error) => self.report(error.to_string()),
            }
        }
    }

    fn render(&mut self, frame: &mut CanvasFrame<'_>) {
        let time = frame.info.time;
        let delta = frame.info.delta;
        let number = frame.info.number;
        // The actors at the frame's time (the host's clock: pause, speed and seeks apply).
        let mut actor_failure = None;
        if let Some(gpu) = self.gpu.as_mut() {
            if let Err(error) = gpu.actors.update(time, delta, &mut gpu.instances) {
                actor_failure = Some(error);
            }
        }
        if let Some(error) = actor_failure {
            self.report(error);
        }
        let resumes = *self.controls.resume_requests.peek();
        if resumes != self.resumes_seen {
            self.resumes_seen = resumes;
            self.controller.set_auto_orbit(true);
        }
        self.controller.update(time);
        // The canvas's CSS height: a pan moves the scene exactly with the pointer.
        self.controller.set_viewport(&self.camera, frame.info.height as f32 / frame.info.pixel_ratio as f32);
        self.controller.apply(&mut self.camera);
        let lighting = self.lighting();
        let graphics = self.graphics.clone();
        let flat_sky = self.flat_sky_linear();
        let operator = *self.controls.operator.peek();
        let exposure = *self.controls.exposure.peek();
        let options = self.options;
        let mut failure = None;
        let mut counts = (0, 0, 0, 0u64);
        let mut drawn_operator = None;
        let mut environment = (String::from("loading the panorama..."), 0);
        if let (Some(gpu), Some((lighting, name, size))) = (self.gpu.as_mut(), lighting.as_ref()) {
            environment = (name.clone(), *size);
            gpu.tone_mapper.operator = operator;
            gpu.tone_mapper.exposure = exposure;
            let result: Result<(), codebrix_webgpu_graphics::GraphicsException> = (|| {
                match (&self.hdr, &self.depth) {
                    (Some(hdr), _) if options.hdr => {
                        let mut graphics_frame = graphics.begin_canvas_frame(frame, None);
                        {
                            // The scene into the HDR target: cleared (black under the skybox, the
                            // flat sky without it), its own depth cleared to 1.
                            let clear = if options.skybox { Color4::BLACK } else { flat_sky };
                            let mut pass = graphics_frame.begin_render_pass_to(&hdr.view(), Some(clear), Some(1.0))?;
                            gpu.renderer.begin(&pass, &self.camera, Some(lighting))?;
                            for instance in &gpu.instances {
                                gpu.renderer.draw(instance)?;
                            }
                            // Opaque parts, then the sky (it fills only the pixels nothing opaque
                            // covered), then the blended parts over both.
                            gpu.renderer.end_opaque(&mut pass)?;
                            if options.skybox {
                                gpu.skybox.draw(&mut pass, &self.camera, Some(lighting))?;
                            }
                            gpu.renderer.end_blended(&mut pass)?;
                        }
                        // Then the tone map into the canvas frame (LOAD: a layer never clears).
                        let mut pass = graphics_frame.begin_render_pass(None, None)?;
                        gpu.tone_mapper.apply_in_pass(&mut pass, hdr.color())?;
                        drawn_operator = Some(operator);
                        Ok(())
                    }
                    (_, Some(depth)) if !options.hdr => {
                        let mut graphics_frame = graphics.begin_canvas_frame(frame, Some(depth));
                        // None: LOAD the colour the host drew (its clear is the flat sky);
                        // Some(1.0): clear OUR depth.
                        let mut pass = graphics_frame.begin_render_pass(None, Some(1.0))?;
                        gpu.renderer.begin(&pass, &self.camera, Some(lighting))?;
                        for instance in &gpu.instances {
                            gpu.renderer.draw(instance)?;
                        }
                        gpu.renderer.end_opaque(&mut pass)?;
                        if options.skybox {
                            gpu.skybox.draw(&mut pass, &self.camera, Some(lighting))?;
                        }
                        gpu.renderer.end_blended(&mut pass)?;
                        Ok(())
                    }
                    _ => Ok(()),
                }
            })();
            if let Err(error) = result {
                failure = Some(error.to_string());
            }
            counts = (gpu.renderer.last_draw_call_count(), gpu.renderer.last_instance_count(), gpu.renderer.pipeline_count(), gpu.skybox.draw_count());
        }
        if let Some(error) = failure {
            self.report(error);
        }
        for error in graphics.take_errors() {
            self.report(error.to_string());
        }
        let orbit = self.controller.orbit;
        let (actors, clips_playing, pose_signature, bird, routine, seeks) = match self.gpu.as_mut() {
            Some(gpu) => {
                let bird = gpu.actors.bird_position(&gpu.instances).unwrap_or(Vec3::ZERO);
                (gpu.actors.len(), gpu.actors.clips_playing(&mut gpu.instances), gpu.actors.pose_signature(&gpu.instances), (bird.x, bird.y, bird.z), gpu.actors.routine_state(&mut gpu.instances), gpu.actors.seeks)
            }
            None => (0, 0, 0.0, (0.0, 0.0, 0.0), String::new(), 0),
        };
        let view = ViewerView {
            frame: number,
            yaw: orbit.yaw,
            pitch: orbit.pitch(),
            distance: orbit.distance(),
            auto_orbit: self.controller.is_auto_orbiting(),
            models: self.models,
            textures: self.textures,
            texture_sharing: self.texture_sharing.clone(),
            build_ms: self.build_ms,
            instances: self.gpu.as_ref().map_or(0, |g| g.instances.len()),
            triangles: self.triangles,
            draw_calls: counts.0,
            drawn: counts.1,
            pipelines: counts.2,
            environment: environment.0,
            environment_size: environment.1,
            environment_build_ms: (self.environment_build_ms * 10.0).round() / 10.0,
            skybox_draws: counts.3,
            operator: drawn_operator,
            exposure,
            hdr_size: self.hdr.as_ref().map_or((0, 0), |h| (h.width(), h.height())),
            scene_time: (time * 1000.0).round() / 1000.0,
            actors,
            clips_playing,
            pose_signature: (pose_signature * 10000.0).round() / 10000.0,
            bird,
            routine,
            seeks,
        };
        if *self.view.peek() != view {
            self.view.set(view);
        }
    }

    /// Drag (or one finger): orbit, the scene following the pointer. Right-button, middle-button or
    /// Shift drag: pan. Wheel: zoom. Two fingers: pan and pinch-zoom.
    fn input(&mut self, input: &CanvasInput) {
        self.controller.input(input);
    }
}
