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

use codebrix_webgpu::{wgpu, CanvasClock, CanvasFrame, CanvasInput, CanvasLayers, CanvasRenderer, CanvasTarget, GpuContext, GpuErrorSubscription, RendererHandle, WebGpuCanvas};
use codebrix_webgpu_animation::animation::PlaybackController;
use codebrix_webgpu_animation::rendering::{AnimationLayer, AnimationLayerControl, CanvasCalibration, DebugOverlay, DebugOverlayCorner, RenderStats};
use codebrix_webgpu_animation::text::FontRegistry;
use codebrix_webgpu_animation::DrawingSession;
use dioxus::prelude::*;

use crate::demo::{Expect, SceneCatalog, FRAME_HEIGHT, FRAME_WIDTH};

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
}

struct DemoState {
    catalog: SceneCatalog,
    playback: PlaybackController,
    index: usize,
    local: f64,
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
}

/// Seeks the host clock to a catalogue time (so the frame's time IS the catalogue time again).
fn seek(clock: &CanvasClock, total: f64, seconds: f64) {
    clock.seek(seconds.clamp(0.0, total - 1e-6));
}

fn scene_step(clock: &CanvasClock, state: &DemoState, forward: bool) {
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
        let new_errors: Vec<String> = self.errors.as_ref().map(|e| e.take().iter().map(|x| x.to_string()).collect()).unwrap_or_default();
        let (index, local, timecode) = {
            let state = self.state.borrow();
            let timecode = state.catalog.timeline.format_timecode(state.playback.time());
            (state.index, state.local, timecode)
        };
        let stats = self.control.stats();
        let mut view = self.view.write();
        view.index = index;
        view.local = local;
        view.timecode = timecode;
        view.frame = frame.info.number;
        view.overlay = self.overlay.get();
        view.stats = stats;
        view.paused = self.clock.is_paused();
        view.errors.extend(new_errors);
    }

    fn input(&mut self, input: &CanvasInput) {
        let state = self.state.borrow();
        let total = state.catalog.total_seconds;
        match input {
            CanvasInput::KeyDown { key } => match key.as_str() {
                "Shift" => self.shift = true,
                " " => {
                    self.clock.toggle();
                }
                "ArrowLeft" if self.shift => scene_step(&self.clock, &state, false),
                "ArrowRight" if self.shift => scene_step(&self.clock, &state, true),
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
    }
}

/// The start-up choices from the address: ?scene=<name>, &t=<seconds into it>, &paused=1.
fn start_choice(catalog: &SceneCatalog) -> (f64, bool) {
    let search = web_sys::window().and_then(|w| w.location().search().ok()).unwrap_or_default();
    let get = |key: &str| search.trim_start_matches('?').split('&').find_map(|kv| kv.strip_prefix(key).and_then(|v| v.strip_prefix('='))).map(|v| v.to_string());
    let start = get("scene").and_then(|name| catalog.index_of(&name)).map(|i| catalog.start_of(i)).unwrap_or(0.0);
    let offset = get("t").and_then(|t| t.parse::<f64>().ok()).unwrap_or(0.0);
    (start + offset, get("paused").as_deref() == Some("1"))
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
        Rc::new(RefCell::new(DemoState { catalog, playback, index: 0, local: 0.0 }))
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
        let (start, start_paused) = start_choice(&state.borrow().catalog);
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
            let (index, local) = s.catalog.at(s.playback.time());
            s.index = index;
            s.local = local;
            s.catalog.scenes[index].render(local, scene, frame.info.width, frame.info.height);
            if show_overlay.get() {
                overlay_session.set_time(s.playback.time());
                overlay_session.render(scene, frame.info.width, frame.info.height);
            }
        });
        let control = inner.control();
        let layer = DemoLayer { inner, control: control.clone(), state: state.clone(), clock: clock.clone(), view, errors: None, shift: false, overlay: overlay_on.clone() };
        (CanvasLayers::single(RendererHandle::new(layer)), control)
    });
    let names: Vec<(usize, &'static str)> = state.borrow().catalog.scenes.iter().enumerate().map(|(i, s)| (i, s.name())).collect();
    let v = view();
    let (title, description, duration) = {
        let s = state.borrow();
        let scene = &s.catalog.scenes[v.index];
        (scene.title(), scene.description(), scene.duration_seconds())
    };
    let button = "rounded-md bg-slate-800 px-3 py-1.5 text-sm text-slate-100 ring-1 ring-white/10 hover:bg-slate-700";
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
        move |_| scene_step(&clock, &state.borrow(), forward)
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
            for (i, name) in names {
                button {
                    id: "scene-{name}",
                    class: "{button}",
                    onclick: {
                        let clock = clock.clone();
                        let state = state.clone();
                        move |_| {
                            let start = clock.points().time(name).unwrap_or_else(|| state.borrow().catalog.start_of(i));
                            seek(&clock, total, start)
                        }
                    },
                    "{name}"
                }
            }
            button { id: "shapes-check-run", class: "{button}", onclick: run_checks, "Check the sample points" }
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
            p { class: "mt-1 text-slate-300", "Errors while drawing: " span { id: "demo-error-count", class: "font-mono text-white", "{v.errors.len()}" } }
            for error in v.errors.iter() {
                p { class: "font-mono text-red-300", "{error}" }
            }
            p { class: "mt-2 text-slate-400", "Click the canvas to give it the keys: Space pauses and plays (the host's clock), Left and Right seek one second, Shift with Left or Right goes to the previous or next scene, Home starts again, D shows or hides the debug overlay. The scene list loops. Every piece of text on the canvas is drawn by codebrix-webgpu-animation in fonts the backend served." }
            pre { id: "shapes-check", class: "mt-2 whitespace-pre-wrap font-mono text-xs text-slate-400", "{checks}" }
        }
    }
}

fn names_at(state: &Rc<RefCell<DemoState>>, index: usize) -> &'static str {
    state.borrow().catalog.scenes[index].name()
}
