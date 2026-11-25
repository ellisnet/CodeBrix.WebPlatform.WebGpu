//! The demo page: the scene catalogue on ONE canvas layer of codebrix-webgpu-animation, on the
//! host's clock (the catalogue's PlaybackController follows the frame's time; pausing the clock
//! stops everything), with AnimationDemoApp's keys - Space pause/play, Left/Right seek 1 s,
//! Shift+Left/Right previous/next scene, Home restart, D the debug overlay (timecode, frame and
//! the active cues, drawn by the crate in the bottom-right corner, as the C# demo's live window
//! does) - and buttons that do the same. Every piece of text on the canvas is drawn by the
//! crate, in the fonts the page registered from the backend's bytes. The catalogue loops at its
//! end.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use codebrix_webgpu::{wgpu, CanvasClock, CanvasFrame, CanvasInput, CanvasLayers, CanvasRenderer, CanvasTarget, GpuContext, GpuErrorSubscription, PointerButton, RendererHandle, WebGpuCanvas};
use codebrix_webgpu_animation::animation::PlaybackController;
use codebrix_webgpu_animation::rendering::{AnimationLayer, AnimationLayerControl, CanvasCalibration, DebugOverlay, DebugOverlayCorner, RenderStats};
use codebrix_webgpu_animation::text::FontRegistry;
use codebrix_webgpu_animation::DrawingSession;
use dioxus::prelude::*;

use crate::demo::controls::trim;
use crate::demo::{Control, Expect, Readout, SceneCatalog, Settings, FRAME_HEIGHT, FRAME_WIDTH};

/// What the page shows about the running catalogue.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct DemoView {
    pub index: usize,
    pub timecode: String,
    pub local: f64,
    pub frame: u64,
    pub overlay: bool,
    pub stats: RenderStats,
    pub errors: Vec<String>,
    pub paused: bool,
    /// The scene's read-outs after the last frame.
    pub readouts: Vec<Readout>,
    /// RenderStats::cpu_ms() smoothed over the last frames (one frame's value is coarse).
    pub cpu_ms_smoothed: f64,
    /// The scene the page stays on ("Stay on this scene"), if any.
    pub hold: Option<usize>,
    /// The interval between animation frames in milliseconds (the page's clock, measurement only),
    /// smoothed over the last frames.
    pub frame_ms_smoothed: f64,
}

struct DemoState {
    catalog: SceneCatalog,
    playback: PlaybackController,
    index: usize,
    local: f64,
    /// The values of each scene's controls, in the catalogue's order.
    settings: Vec<Settings>,
    /// The current scene's read-outs after the last frame.
    readouts: Vec<Readout>,
    /// "Stay on this scene": the scene the catalogue repeats instead of moving on.
    hold: Option<usize>,
    /// A clock time the layer seeks to after the frame (the held scene starting again).
    reseek: Option<f64>,
}

/// The catalogue as a canvas layer: the AnimationLayer, plus the keys and the page's read-out.
struct DemoLayer {
    inner: AnimationLayer,
    control: AnimationLayerControl,
    state: Rc<RefCell<DemoState>>,
    clock: CanvasClock,
    view: Signal<DemoView>,
    errors: Option<GpuErrorSubscription>,
    shift: bool,
    overlay: Rc<Cell<bool>>,
    /// The page clock at the last frame (for the frame interval read-out only; nothing drawn
    /// depends on it).
    last_frame_ms: Option<f64>,
}

/// Seeks the host clock to a catalogue time (so the frame's time IS the catalogue time again).
fn seek(clock: &CanvasClock, total: f64, seconds: f64) {
    clock.seek(seconds.clamp(0.0, total - 1e-6));
}

fn scene_step(clock: &CanvasClock, state: &mut DemoState, forward: bool) {
    let total = state.catalog.total_seconds;
    let now = clock.elapsed() % total;
    let (index, local) = state.catalog.at(now);
    let count = state.catalog.scenes.len();
    let target = if forward {
        (index + 1) % count
    } else if local > 0.5 {
        index
    } else {
        (index + count - 1) % count
    };
    if state.hold.is_some() {
        state.hold = Some(target);
    }
    seek(clock, total, state.catalog.start_of(target));
}

impl CanvasRenderer for DemoLayer {
    fn prepare(&mut self, context: &GpuContext, target: &CanvasTarget) {
        if self.errors.is_none() {
            self.errors = Some(context.subscribe_errors());
        }
        self.inner.prepare(context, target);
    }

    fn resize(&mut self, context: &GpuContext, width: u32, height: u32) {
        self.inner.resize(context, width, height);
    }

    fn render(&mut self, frame: &mut CanvasFrame<'_>) {
        self.inner.render(frame);
        // The held scene came to its end: the clock goes back to its start (the frame just drawn
        // already showed the held scene).
        if let Some(seconds) = self.state.borrow_mut().reseek.take() {
            self.clock.seek(seconds);
        }
        let new_errors: Vec<String> = self.errors.as_ref().map(|e| e.take().iter().map(|x| x.to_string()).collect()).unwrap_or_default();
        let stats = self.control.stats();
        // The scene sees the frame's statistics (the water scenes' ripple counters), then its
        // read-outs are taken again.
        {
            let mut state = self.state.borrow_mut();
            let index = state.index;
            state.catalog.scenes[index].see_stats(&stats);
            state.readouts = state.catalog.scenes[index].readouts();
        }
        let (index, local, timecode, readouts, hold) = {
            let state = self.state.borrow();
            let timecode = state.catalog.timeline.format_timecode(state.playback.time());
            (state.index, state.local, timecode, state.readouts.clone(), state.hold)
        };
        let mut view = self.view.write();
        view.index = index;
        view.local = local;
        view.timecode = timecode;
        view.frame = frame.info.number;
        view.overlay = self.overlay.get();
        view.stats = stats;
        view.paused = self.clock.is_paused();
        view.errors.extend(new_errors);
        view.readouts = readouts;
        view.hold = hold;
        view.cpu_ms_smoothed = if view.cpu_ms_smoothed == 0.0 { stats.cpu_ms() } else { 0.9 * view.cpu_ms_smoothed + 0.1 * stats.cpu_ms() };
        // The frame interval (measurement only): the page's clock between two frames.
        if let Some(now) = web_sys::window().and_then(|w| w.performance()).map(|p| p.now()) {
            if let Some(last) = self.last_frame_ms {
                let interval = now - last;
                view.frame_ms_smoothed = if view.frame_ms_smoothed == 0.0 { interval } else { 0.95 * view.frame_ms_smoothed + 0.05 * interval };
            }
            self.last_frame_ms = Some(now);
        }
    }

    fn input(&mut self, input: &CanvasInput) {
        let mut state = self.state.borrow_mut();
        let total = state.catalog.total_seconds;
        match input {
            CanvasInput::KeyDown { key } => match key.as_str() {
                "Shift" => self.shift = true,
                " " => {
                    self.clock.toggle();
                }
                "ArrowLeft" if self.shift => scene_step(&self.clock, &mut state, false),
                "ArrowRight" if self.shift => scene_step(&self.clock, &mut state, true),
                "ArrowLeft" => seek(&self.clock, total, (self.clock.elapsed() % total) - 1.0),
                "ArrowRight" => seek(&self.clock, total, (self.clock.elapsed() % total) + 1.0),
                "Home" => {
                    seek(&self.clock, total, 0.0);
                    self.clock.resume();
                }
                "d" | "D" => self.overlay.set(!self.overlay.get()),
                _ => {}
            },
            CanvasInput::KeyUp { key } if key == "Shift" => self.shift = false,
            _ => {}
        }
        // Every pointer event also goes to the scene shown, with the map to the frame's units; a
        // press of the primary button besides as a point of the frame (the water scenes drop a
        // stone there; most scenes ignore both).
        if matches!(input, CanvasInput::PointerDown { .. } | CanvasInput::PointerMove { .. } | CanvasInput::PointerUp { .. }) {
            let (width, height) = canvas_css_size();
            let rect = CanvasCalibration::get_drawing_rect(width, height, (FRAME_WIDTH, FRAME_HEIGHT));
            if rect.width > 0.0 && rect.height > 0.0 {
                let to_frame = |x: f64, y: f64| ((x as f32 - rect.x) * FRAME_WIDTH as f32 / rect.width, (y as f32 - rect.y) * FRAME_HEIGHT as f32 / rect.height);
                let index = state.index;
                let scene = &mut state.catalog.scenes[index];
                scene.pointer_input(input, &to_frame);
                if let CanvasInput::PointerDown { x, y, button: PointerButton::Primary, .. } = input {
                    let (fx, fy) = to_frame(*x, *y);
                    scene.pointer_down(fx, fy);
                }
            }
        }
    }
}

/// The start-up choices from the address: ?scene=<name>, &t=<seconds into it>, &paused=1, and
/// &hold=1 (stay on that scene).
fn start_choice(catalog: &SceneCatalog) -> (f64, bool, Option<usize>) {
    let search = web_sys::window().and_then(|w| w.location().search().ok()).unwrap_or_default();
    let get = |key: &str| search.trim_start_matches('?').split('&').find_map(|kv| kv.strip_prefix(key).and_then(|v| v.strip_prefix('='))).map(|v| v.to_string());
    let index = get("scene").and_then(|name| catalog.index_of(&name));
    let start = index.map(|i| catalog.start_of(i)).unwrap_or(0.0);
    let offset = get("t").and_then(|t| t.parse::<f64>().ok()).unwrap_or(0.0);
    let hold = if get("hold").as_deref() == Some("1") { Some(index.unwrap_or(0)) } else { None };
    (start + offset, get("paused").as_deref() == Some("1"), hold)
}

/// Reads the layer's own pixels and checks the scene's sample points (a run-check, not an image
/// comparison): one line per point, and a summary.
fn check_points(control: &AnimationLayerControl, state: &Rc<RefCell<DemoState>>, mut out: Signal<String>) {
    let points = {
        let s = state.borrow();
        s.catalog.scenes[s.index].checks()
    };
    if points.is_empty() {
        out.set("this scene has no sample points".to_string());
        return;
    }
    control.read_pixels_for_testing(move |pixels, w, h| {
        if pixels.is_empty() {
            out.set("no pixels were read".to_string());
            return;
        }
        let rect = CanvasCalibration::get_drawing_rect(w as f32, h as f32, (FRAME_WIDTH, FRAME_HEIGHT));
        let scale = rect.width / FRAME_WIDTH as f32;
        let read = |x: f32, y: f32| {
            let px = ((rect.x + x * scale) as u32).min(w - 1);
            let py = ((rect.y + y * scale) as u32).min(h - 1);
            let k = ((py * w + px) * 4) as usize;
            [pixels[k], pixels[k + 1], pixels[k + 2], pixels[k + 3]]
        };
        let values: Vec<[u8; 4]> = points.iter().map(|p| read(p.x, p.y)).collect();
        // The largest alpha inside a box of frame units (every pixel of it).
        let max_alpha = |x: f32, y: f32, bw: f32, bh: f32| {
            let (x0, y0) = (((rect.x + x * scale) as u32).min(w - 1), ((rect.y + y * scale) as u32).min(h - 1));
            let (x1, y1) = (((rect.x + (x + bw) * scale) as u32).min(w - 1), ((rect.y + (y + bh) * scale) as u32).min(h - 1));
            let mut most = 0u8;
            for py in y0..=y1 {
                for px in x0..=x1 {
                    most = most.max(pixels[((py * w + px) * 4 + 3) as usize]);
                }
            }
            most
        };
        let mut failed = Vec::new();
        let mut lines = Vec::new();
        for (i, p) in points.iter().enumerate() {
            let v = values[i];
            let ok = match p.expect {
                Expect::Drawn => v[3] > 200,
                Expect::Empty => v[3] < 30,
                Expect::Translucent => v[3] > 100 && v[3] < 160,
                Expect::Differs(j) => (0..4).any(|c| (v[c] as i32 - values[j][c] as i32).abs() > 12),
                Expect::InkInBox(bw, bh) => max_alpha(p.x, p.y, bw, bh) > 200,
                Expect::NoInkInBox(bw, bh) => max_alpha(p.x, p.y, bw, bh) < 30,
            };
            if !ok {
                failed.push(p.name);
            }
            lines.push(format!("{} {:?} {} rgba={},{},{},{}", if ok { "PASS" } else { "FAIL" }, p.expect, p.name, v[0], v[1], v[2], v[3]));
        }
        out.set(format!("points={} passed={} failed={} size={}x{}\n{}", points.len(), points.len() - failed.len(), failed.len(), w, h, lines.join("\n")));
    });
}

/// The demo: the canvas (every scene's header and labels drawn on it by the crate), controls and
/// read-outs. `fonts` holds the fonts the backend served (Roboto, then Open Sans).
#[component]
pub fn DemoPage(context: GpuContext, fonts: FontRegistry) -> Element {
    let clock = use_hook(CanvasClock::new);
    let view = use_signal(DemoView::default);
    let checks = use_signal(String::new);
    let overlay_on = use_hook(|| Rc::new(Cell::new(false)));
    let state = use_hook(|| {
        let catalog = SceneCatalog::new(&fonts);
        let mut playback = PlaybackController::new(catalog.timeline.clone());
        playback.set_is_looping(true);
        playback.play();
        let settings = catalog.scenes.iter().map(|s| Settings::from_controls(s.controls())).collect();
        Rc::new(RefCell::new(DemoState { catalog, playback, index: 0, local: 0.0, settings, readouts: Vec::new(), hold: None, reseek: None }))
    });
    let (layers, control) = use_hook(|| {
        // NAMED POINTS ON THE HOST'S CLOCK: each scene's start, defined once; the scene buttons
        // seek to them by name and the catalogue's timeline carries them as markers.
        {
            let mut s = state.borrow_mut();
            for i in 0..s.catalog.scenes.len() {
                let name = s.catalog.scenes[i].name();
                clock.define_point(name, s.catalog.start_of(i));
                let points = clock.points();
                let _ = s.catalog.timeline.add_marker_at_point(&format!("{name} starts"), &points, name);
            }
            // A scene's own named points (the Lottie scenes' markers), as "<scene>/<marker>".
            for i in 0..s.catalog.scenes.len() {
                let (name, start) = (s.catalog.scenes[i].name(), s.catalog.start_of(i));
                for (point, seconds) in s.catalog.scenes[i].time_points() {
                    let point = format!("{name}/{point}");
                    clock.define_point(&point, start + seconds);
                    let points = clock.points();
                    let _ = s.catalog.timeline.add_marker_at_point(&point, &points, &point);
                }
            }
        }
        let (start, start_paused, hold) = start_choice(&state.borrow().catalog);
        state.borrow_mut().hold = hold;
        clock.seek(start);
        if start_paused {
            clock.pause();
        }
        let scenes = state.clone();
        // THE DEBUG OVERLAY: an empty session that draws only its overlay, over the scene, on the
        // catalogue's timeline (the C# live window's way); D switches it.
        let mut overlay_session = DrawingSession::default();
        let mut overlay = DebugOverlay::new(Some(fonts.clone()), Some(state.borrow().catalog.timeline.clone()));
        overlay.show_cues = true;
        overlay.corner = DebugOverlayCorner::BottomRight;
        overlay_session.set_debug_overlay(Some(overlay));
        let show_overlay = overlay_on.clone();
        let inner = AnimationLayer::new(move |frame, scene| {
            let mut s = scenes.borrow_mut();
            let total = s.catalog.total_seconds;
            // THE CATALOGUE IS ON THE HOST'S CLOCK: the playback follows the frame's time.
            s.playback.seek(frame.info.time % total);
            let now = s.playback.time();
            let (mut index, mut local) = s.catalog.at(now);
            // "Stay on this scene": past its end the held scene starts again (the layer seeks
            // the clock back after the frame).
            if let Some(held) = s.hold {
                if index != held {
                    let (start, duration) = (s.catalog.start_of(held), s.catalog.scenes[held].duration_seconds());
                    local = (now - start).rem_euclid(duration);
                    index = held;
                    s.reseek = Some(start + local);
                }
            }
            s.index = index;
            s.local = local;
            s.catalog.scenes[index].render(local, scene, frame.info.width, frame.info.height);
            s.readouts = s.catalog.scenes[index].readouts();
            if show_overlay.get() {
                overlay_session.set_time(s.playback.time());
                overlay_session.render(scene, frame.info.width, frame.info.height);
            }
        });
        let control = inner.control();
        let layer = DemoLayer { inner, control: control.clone(), state: state.clone(), clock: clock.clone(), view, errors: None, shift: false, overlay: overlay_on.clone(), last_frame_ms: None };
        // The "live-pickup" scene's source layers: BELOW the catalogue's layer the picture layer (a
        // session drawn into a picture target) and the 3D scene of the graphics crate (both draw
        // only while that scene asks for them), so the strokes read THIS frame's pictures; ABOVE it
        // the small layer that shows the 3D picture itself.
        let (below, above) = crate::demo::live_pickup::source_layers();
        let mut all = below;
        // The "simulated-pond-2d" scene's wave field, stepped below the catalogue's layer (only
        // while that scene asks for it) so its refraction reads this frame's water.
        all.push(crate::demo::simulated_pond::source_layer());
        all.push(RendererHandle::new(layer));
        all.extend(above);
        (CanvasLayers::new(all), control)
    });
    let names: Vec<(usize, &'static str)> = state.borrow().catalog.scenes.iter().enumerate().map(|(i, s)| (i, s.name())).collect();
    let v = view();
    let (title, description, duration) = {
        let s = state.borrow();
        let scene = &s.catalog.scenes[v.index];
        (scene.title(), scene.description(), scene.duration_seconds())
    };
    let button = "rounded-md bg-slate-800 px-3 py-1.5 text-sm text-slate-100 ring-1 ring-white/10 hover:bg-slate-700";
    let current_button = "rounded-md bg-sky-300 px-3 py-1.5 text-sm font-semibold text-slate-950 ring-2 ring-white";
    // When the scene on show changes, its button is scrolled into view inside the button box
    // (never the page; the keyboard focus stays where it is).
    let shown_index = use_memo(move || view().index);
    {
        let state = state.clone();
        use_effect(move || {
            let index = shown_index();
            let name = names_at(&state, index);
            keep_in_view("scene-buttons", &format!("scene-{name}"));
        });
    }
    let total = state.borrow().catalog.total_seconds;
    let toggle = {
        let clock = clock.clone();
        move |_| {
            clock.toggle();
        }
    };
    let step = |forward: bool| {
        let clock = clock.clone();
        let state = state.clone();
        move |_| scene_step(&clock, &mut state.borrow_mut(), forward)
    };
    let restart = {
        let clock = clock.clone();
        move |_| {
            seek(&clock, total, 0.0);
            clock.resume();
        }
    };
    let toggle_overlay = {
        let overlay_on = overlay_on.clone();
        move |_| overlay_on.set(!overlay_on.get())
    };
    let run_checks = {
        let control = control.clone();
        let state = state.clone();
        move |_| check_points(&control, &state, checks)
    };
    let stats = v.stats;
    // THE SCENE'S OWN PANEL: its legend, controls and read-outs (the feature scenes), its named
    // points, the scene-time scrubber and "Stay on this scene".
    let mut revision = use_signal(|| 0u64);
    let _ = revision();
    let (scene_name, scene_controls, legend, points, settings_now, scene_start) = {
        let s = state.borrow();
        let scene = &s.catalog.scenes[v.index];
        (scene.name(), scene.controls(), scene.legend(), scene.time_points(), s.settings[v.index].clone(), s.catalog.start_of(v.index))
    };
    let current = v.index;
    let field = "rounded-md bg-slate-800 px-2 py-1 text-sm text-slate-100 ring-1 ring-white/10";
    let toggle_hold = {
        let state = state.clone();
        move |_| {
            let mut s = state.borrow_mut();
            s.hold = if s.hold.is_some() { None } else { Some(s.index) };
            drop(s);
            revision += 1;
        }
    };
    let scrub = {
        let clock = clock.clone();
        move |event: Event<FormData>| {
            if let Ok(value) = event.value().parse::<f64>() {
                seek(&clock, total, scene_start + value.clamp(0.0, duration));
            }
        }
    };
    let change = {
        let state = state.clone();
        move |key: &'static str, value: f32| {
            let mut s = state.borrow_mut();
            s.settings[current].set(key, value);
            let settings = s.settings[current].clone();
            s.catalog.scenes[current].apply_settings(&settings);
            drop(s);
            revision += 1;
        }
    };
    rsx! {
        div { class: "relative h-[720px] w-[1280px] ring-1 ring-white/10",
            WebGpuCanvas {
                context: context.clone(),
                layers,
                clock: clock.clone(),
                clear_color: wgpu::Color { r: 0.0065, g: 0.0070, b: 0.0123, a: 1.0 },
                id: "animation-canvas",
                label: "The animation demo's scenes. Space pauses and plays, Left and Right seek one second, Shift with Left or Right goes to the previous or next scene, Home starts again, D shows or hides the debug overlay.",
                class: "block h-[720px] w-[1280px] touch-none select-none focus-visible:outline-2 focus-visible:outline-sky-400",
            }
        }
        div { class: "flex flex-wrap gap-3",
            button { id: "demo-clock-toggle", class: "{button}", onclick: toggle, if v.paused { "Resume the clock" } else { "Pause the clock" } }
            button { id: "demo-previous", class: "{button}", onclick: step(false), "Previous scene (Shift+Left)" }
            button { id: "demo-next", class: "{button}", onclick: step(true), "Next scene (Shift+Right)" }
            button { id: "demo-restart", class: "{button}", onclick: restart, "Start again (Home)" }
            button { id: "demo-overlay-toggle", class: "{button}", onclick: toggle_overlay, if v.overlay { "Hide the debug overlay (D)" } else { "Show the debug overlay (D)" } }
            button { id: "shapes-check-run", class: "{button}", onclick: run_checks, "Check the sample points" }
        }
        // THE SCENE BUTTONS: the button of the scene being drawn is filled (a light fill, dark text,
        // a white ring) and carries aria-current="true"; it follows the layer's own scene index
        // (the one that decides what is drawn), so a click, the catalogue moving on by itself, a
        // `?scene=` address, the keys and the previous / next buttons all move it. The rows scroll
        // inside their own box, which keeps the current button in view.
        nav { id: "scene-buttons", class: "flex max-h-44 flex-wrap gap-2 overflow-y-auto rounded-lg p-1 ring-1 ring-white/10", "aria-label": "Scenes",
            for (i, name) in names {
                button {
                    key: "{name}",
                    id: "scene-{name}",
                    class: if i == v.index { "{current_button}" } else { "{button}" },
                    "aria-current": if i == v.index { "true" } else { "false" },
                    onclick: {
                        let clock = clock.clone();
                        let state = state.clone();
                        move |_| {
                            let start = clock.points().time(name).unwrap_or_else(|| state.borrow().catalog.start_of(i));
                            {
                                let mut s = state.borrow_mut();
                                if s.hold.is_some() {
                                    s.hold = Some(i);
                                }
                            }
                            seek(&clock, total, start)
                        }
                    },
                    "{name}"
                }
            }
        }
        div { id: "scene-panel", class: "flex flex-col gap-3 rounded-lg bg-slate-900 p-3 text-sm text-slate-300 ring-1 ring-white/10",
            div { class: "flex flex-wrap items-center gap-3",
                button {
                    id: "demo-hold",
                    class: if v.hold.is_some() { "rounded-md bg-sky-700 px-3 py-1.5 text-sm text-white ring-1 ring-sky-300/40" } else { "{button}" },
                    "aria-pressed": if v.hold.is_some() { "true" } else { "false" },
                    onclick: toggle_hold,
                    if v.hold.is_some() { "Stay on this scene: on" } else { "Stay on this scene: off" }
                }
                label { class: "flex items-center gap-2",
                    "Scene time"
                    input { id: "demo-scrub", class: "w-72 accent-sky-400", r#type: "range", min: "0", max: "{duration}", step: "0.01", value: "{v.local}", oninput: scrub }
                    span { id: "demo-scrub-value", class: "w-14 font-mono text-white", "{v.local:.2}" }
                }
                for (point, seconds) in points {
                    button {
                        key: "{scene_name}-{point}",
                        id: "point-{scene_name}-{slug(&point)}",
                        class: "{button}",
                        onclick: {
                            let clock = clock.clone();
                            move |_| seek(&clock, total, scene_start + seconds)
                        },
                        "{point}"
                    }
                }
            }
            if !legend.is_empty() {
                div { id: "scene-legend", class: "rounded-md bg-slate-950/60 px-3 py-2 text-xs text-slate-200 ring-1 ring-white/10",
                    p { class: "font-semibold text-white", "What to look at" }
                    for line in legend.iter() {
                        p { "{line}" }
                    }
                }
            }
            if !scene_controls.is_empty() {
                div { id: "scene-controls", class: "flex flex-wrap items-center gap-x-5 gap-y-3",
                    for control in scene_controls.iter().copied() {
                        match control {
                            Control::Toggle { key, label, .. } => {
                                let on = settings_now.on(key);
                                let mut change = change.clone();
                                rsx! {
                                    button {
                                        key: "{scene_name}-{key}",
                                        id: "{scene_name}-{key}",
                                        class: if on { "rounded-md bg-sky-700 px-3 py-1.5 text-sm text-white ring-1 ring-sky-300/40" } else { "{button}" },
                                        "aria-pressed": "{on}",
                                        onclick: move |_| change(key, if on { 0.0 } else { 1.0 }),
                                        "{label}: " if on { "on" } else { "off" }
                                    }
                                }
                            }
                            Control::Slider { key, label, min, max, step, .. } => {
                                let value = settings_now.get(key);
                                let mut change = change.clone();
                                rsx! {
                                    label { key: "{scene_name}-{key}", class: "flex items-center gap-2",
                                        "{label}"
                                        input {
                                            id: "{scene_name}-{key}",
                                            class: "w-32 accent-sky-400",
                                            r#type: "range",
                                            min: "{min}",
                                            max: "{max}",
                                            step: "{step}",
                                            value: "{value}",
                                            oninput: move |event: Event<FormData>| {
                                                if let Ok(v) = event.value().parse::<f32>() {
                                                    change(key, v.clamp(min, max));
                                                }
                                            },
                                        }
                                        span { id: "{scene_name}-{key}-value", class: "w-12 font-mono text-white", "{trim(value)}" }
                                    }
                                }
                            }
                            Control::Button { key, label } => {
                                let presses = settings_now.get(key);
                                let mut change = change.clone();
                                rsx! {
                                    button {
                                        key: "{scene_name}-{key}",
                                        id: "{scene_name}-{key}",
                                        class: "{button}",
                                        onclick: move |_| change(key, presses + 1.0),
                                        "{label}"
                                    }
                                }
                            }
                            Control::Choice { key, label, options, .. } => {
                                let value = settings_now.get(key);
                                let mut change = change.clone();
                                rsx! {
                                    label { key: "{scene_name}-{key}", class: "flex items-center gap-2",
                                        "{label}"
                                        select {
                                            id: "{scene_name}-{key}",
                                            class: "{field}",
                                            onchange: move |event: Event<FormData>| {
                                                if let Some((_, v)) = options.iter().find(|(name, _)| *name == event.value()) {
                                                    change(key, *v);
                                                }
                                            },
                                            for (name, v) in options.iter() {
                                                option { value: "{name}", selected: *v == value, "{name}" }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            for readout in v.readouts.iter() {
                p { key: "{scene_name}-{readout.key}",
                    "{readout.label}: " span { id: "{scene_name}-{readout.key}", class: "font-mono text-white", "{readout.value}" }
                }
            }
        }
        div { class: "rounded-lg bg-slate-900 p-4 text-sm ring-1 ring-white/10",
            p { class: "text-slate-300",
                "Scene " span { id: "demo-scene", class: "text-white", "{names_at(&state, v.index)}" }
                " (" span { id: "scene-title", "{title}" } ")"
                " at " span { id: "demo-local", class: "font-mono text-white", "{v.local:.2}" } " s of {duration} s; catalogue time "
                span { id: "demo-time", class: "font-mono text-white", "{v.timecode}" }
                "; frame " span { id: "demo-frame", class: "font-mono text-white", "{v.frame}" }
                "; clock " span { id: "demo-clock-state", if v.paused { "paused" } else { "running" } }
                "; debug overlay " span { id: "demo-overlay", if v.overlay { "on" } else { "off" } }
            }
            p { id: "scene-description", class: "mt-1 text-slate-400", "{description}" }
            p { id: "demo-stats", class: "mt-1 font-mono text-xs text-slate-400 break-all",
                "draws={stats.draws} culled={stats.culled} cache_hits={stats.cache_hits} cache_misses={stats.cache_misses} cached={stats.cached} fill_batches={stats.fill_batches} draw_calls={stats.draw_calls} passes={stats.passes} offscreen={stats.offscreen} folded={stats.folded} backdrop_copies={stats.backdrop_copies} vertices={stats.vertices} unsupported={stats.unsupported}"
            }
            p { id: "demo-stats-2", class: "mt-1 font-mono text-xs text-slate-400 break-all",
                "cpu_ms={stats.cpu_ms():.3} cpu_ms_smoothed={v.cpu_ms_smoothed:.3} record_ms={stats.record_ms:.3} build_ms={stats.build_ms:.3} encode_ms={stats.encode_ms:.3} stroke_draws={stats.stroke_draws} strokes={stats.strokes} stroke_uploads={stats.stroke_uploads} stroke_upload_bytes={stats.stroke_upload_bytes} tip_uploads={stats.tip_uploads} tip_upload_bytes={stats.tip_upload_bytes} image_uploads={stats.image_uploads} image_upload_bytes={stats.image_upload_bytes} color_table_uploads={stats.color_table_uploads} color_table_upload_bytes={stats.color_table_upload_bytes}"
            }
            p { id: "demo-stats-3", class: "mt-1 font-mono text-xs text-slate-400 break-all",
                "strokes={stats.strokes} strokes_restored={stats.strokes_restored} checkpoint_hits={stats.checkpoint_hits} checkpoint_bakes={stats.checkpoint_bakes} checkpoint_misses={stats.checkpoint_misses} checkpoints={stats.checkpoints} checkpoint_mb={stats.checkpoint_bytes as f64 / 1048576.0:.1} ripples_evaluated={stats.ripples_evaluated} ripples_skipped={stats.ripples_skipped} frame_ms_smoothed={v.frame_ms_smoothed:.2}"
            }
            p { class: "mt-1 text-slate-300", "Errors while drawing: " span { id: "demo-error-count", class: "font-mono text-white", "{v.errors.len()}" } }
            for error in v.errors.iter() {
                p { class: "font-mono text-red-300", "{error}" }
            }
            p { class: "mt-2 text-slate-400", "Click the canvas to give it the keys (on the water scenes a click on the water also drops a stone there): Space pauses and plays (the host's clock), Left and Right seek one second, Shift with Left or Right goes to the previous or next scene, Home starts again, D shows or hides the debug overlay. The scene list loops. Every piece of text on the canvas is drawn by codebrix-webgpu-animation in fonts the backend served." }
            pre { id: "shapes-check", class: "mt-2 whitespace-pre-wrap font-mono text-xs text-slate-400", "{checks}" }
        }
    }
}

/// The canvas's size in CSS pixels (the element's client size; the frame's size if it is not
/// found), for turning a pointer's position into the frame's units.
fn canvas_css_size() -> (f32, f32) {
    web_sys::window()
        .and_then(|w| w.document())
        .and_then(|d| d.get_element_by_id("animation-canvas"))
        .map(|e| (e.client_width() as f32, e.client_height() as f32))
        .filter(|(w, h)| *w > 0.0 && *h > 0.0)
        .unwrap_or((FRAME_WIDTH as f32, FRAME_HEIGHT as f32))
}

/// A name made fit for an element id (letters and digits kept, anything else a dash).
fn slug(name: &str) -> String {
    name.chars().map(|c| if c.is_ascii_alphanumeric() { c.to_ascii_lowercase() } else { '-' }).collect()
}

fn names_at(state: &Rc<RefCell<DemoState>>, index: usize) -> &'static str {
    state.borrow().catalog.scenes[index].name()
}

/// Scrolls the element `container_id` (only it: never the page, and the keyboard focus stays
/// where it is) so that its child `child_id` is wholly in view; nothing when it already is.
fn keep_in_view(container_id: &str, child_id: &str) {
    let Some(document) = web_sys::window().and_then(|w| w.document()) else { return };
    let (Some(container), Some(child)) = (document.get_element_by_id(container_id), document.get_element_by_id(child_id)) else { return };
    let (outer, inner) = (container.get_bounding_client_rect(), child.get_bounding_client_rect());
    let top = inner.top() - outer.top() + f64::from(container.scroll_top());
    let bottom = top + inner.height();
    let height = f64::from(container.client_height());
    let scroll = f64::from(container.scroll_top());
    if top < scroll {
        container.set_scroll_top(top.floor() as i32);
    } else if bottom > scroll + height {
        container.set_scroll_top((bottom - height).ceil() as i32);
    }
}
