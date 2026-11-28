//! The canvas host component: a Dioxus `canvas` that becomes a `wgpu` surface when it is
//! mounted, kept at the canvas's size in device pixels, cleared and drawn by its layers once per
//! animation frame with the time of one clock, and passing pointer, wheel and key input to them.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use dioxus::prelude::*;
use wasm_bindgen::JsCast;

use crate::clock::CanvasClock;
use crate::context::GpuContext;
use crate::drag::DragTracker;
use crate::frame_loop::FrameLoop;
use crate::idle::{IdleWatch, Shown};
use crate::input::{is_canvas_key, modifier_keys, pointer_button, pointer_buttons, pointer_kind, wheel_in_pixels, CanvasInput, PointerButton};
use crate::layers::{CanvasLayers, LayerState};
use crate::page_compose::PageCompose;
use crate::renderer::{CanvasFrame, CanvasTarget, FrameInfo, RendererHandle, SurfaceFailure};
use crate::sizing::{pixel_ratio, surface_size};
use crate::taps::{TapCounter, TapOptions};

/// Draws a canvas's `layers` (bottom first) every animation frame, through `context`'s device,
/// with the time of `clock`.
///
/// THE HOST DECIDES, in this one place, and every layer reads it from its frame:
/// - THE SURFACE ([`CanvasTarget`]): the browser's preferred canvas format, with its sRGB form
///   as the format layers draw in (shaders write linear colour, the view encodes it); the alpha
///   mode `Opaque`, or `PreMultiplied` when `transparent` is true; FIFO presentation (the only one
///   a browser has); the size below.
/// - THE CLEAR: each frame begins with the target cleared to `clear_color` (default: opaque black,
///   or transparent black for a `transparent` canvas); the layers then draw over it, in order,
///   into ONE command encoder, which the host submits once before it presents the frame.
/// - A TRANSPARENT CANVAS (drawing "on the page"): the page composites a canvas as colour
///   premultiplied in sRGB values, while the layers write LINEAR premultiplied colour. So the
///   layers of a transparent canvas draw into a texture of the host's (same format, same size -
///   nothing changes for them), and a last pass of the host writes it onto the surface
///   un-premultiplied, sRGB-encoded and premultiplied again: half-transparent colour and
///   anti-aliased edges then blend with the page exactly as the page expects (no light fringes).
///   Leave the page's styling free of a frame (no border, ring or rounded corners on the canvas
///   or its container) for a drawing that floats on the page.
/// - THE TIME: from ONE clock ([`CanvasClock`]: pause, resume, speed, seek, named points in
///   time), the same for every layer in a frame.
///
/// The component carries no styling of its own: size and style the canvas with `class` (the
/// application's CSS classes) or `style`. The surface follows the canvas's DISPLAYED size times
/// the browser's device pixel ratio, so it stays sharp when the window or the zoom changes; a
/// canvas without a displayed size draws nothing. When it is mounted the host sets the canvas's
/// `touch-action` to `none` through the element's style object (a drag on a touch screen then
/// reaches the layers instead of scrolling the page); that is not an inline style attribute, so
/// a strict Content-Security-Policy allows it. The canvas takes the keyboard focus when it is
/// pressed, so that key input reaches the layers, and while it has the focus the arrow keys and
/// the space bar do not scroll the page; nor does the wheel over it. Pressing the middle button on
/// the canvas starts no autoscroll (the press is the layers'), and with `suppress_context_menu`
/// (the default) a right-click opens no browser menu over it, so that a right-button drag reaches
/// the layers too. Every pointer event says which buttons are down, the kind of pointer and the
/// modifier keys held ([`CanvasInput`]), and a press says its place in a series of quick presses
/// (a double-click or a double tap: `tap_count`, with the window of `tap_options`).
///
/// A DECORATIVE CANVAS (Not in C#): with `interactive: false` the canvas is not part of the
/// page's interaction at all - it takes no focus (no `tabindex`), keeps the page's own
/// `touch-action`, listens for no pointer, wheel or key events (its layers get no input), and its
/// style object gets `pointer-events: none`, so pointer events reach the page beneath it (a drawing
/// laid over the page's content as an ornament). The default, true, is the interactive canvas
/// described above.
///
/// IDLING WHILE PAUSED (Not in C#): by default every animation frame is drawn, also while the
/// clock is paused. With `idle_when_paused: true`, while the clock is paused the host draws a
/// frame only when it would show something new, and otherwise records, submits and presents
/// nothing (the browser keeps showing the last picture; the animation-frame callback itself
/// still runs and reads the canvas's size): the first frame; a new displayed size or pixel ratio;
/// a layer added, removed, moved or given a time scale; [`CanvasLayers::request_redraw`]; a seek
/// or resume of the clock, or a named point defined, moved or removed; an input event on the
/// canvas (its layers may change what they draw); a frame the surface could not give; and the
/// first frame after the loop was held back (a hidden page coming back). A layer that changes
/// what it draws in any other way while the clock is paused (a picture that finished loading, a
/// session the page edits, a readback that needs further frames) asks with `request_redraw`.
///
/// WHEN THE PROPERTIES ARE READ. `context`, `layers`, `transparent` and `clear_color` are taken
/// when the canvas element is MOUNTED, and `clock` at the component's first render: keep each in
/// the page (`use_hook`) and change the layers through the [`CanvasLayers`] handle, not by passing
/// another (drawing AND input use the layers taken at mount). Changing `interactive` replaces the
/// canvas element, so the host starts again on the new one and takes `context`, `layers`,
/// `transparent` and `clear_color` as that render passes them (a new surface; every layer is
/// prepared again). Every other property follows a change on a live canvas: `class`, `style`,
/// `id` and `label` (the element's attributes), `suppress_context_menu`, `tap_options` and
/// `idle_when_paused` (from the next event or frame).
#[component]
pub fn WebGpuCanvas(
    /// The context whose device draws (the page's one context, shared).
    context: GpuContext,
    /// The renderers that draw the canvas, bottom first.
    layers: CanvasLayers,
    /// The clock of the frame time; a canvas given none has a clock of its own.
    #[props(default)]
    clock: Option<CanvasClock>,
    /// The colour the host clears each frame to (linear, premultiplied); None for opaque black, or
    /// transparent black on a `transparent` canvas.
    #[props(default)]
    clear_color: Option<wgpu::Color>,
    /// True when the page shows through where the drawing's alpha is below 1 (the canvas is
    /// configured with premultiplied alpha); false (the default) for an opaque canvas.
    #[props(default)]
    transparent: bool,
    /// The canvas's classes (the application's CSS).
    #[props(default)]
    class: String,
    /// The canvas's inline style attribute. A strict Content-Security-Policy (such as the
    /// platform samples') refuses inline style attributes: prefer `class` there.
    #[props(default)]
    style: String,
    /// The canvas element's id.
    #[props(default)]
    id: String,
    /// The canvas's accessible name (`aria-label`): say what it shows and how to use it.
    #[props(default)]
    label: String,
    /// True (the default) when a right-click on the canvas opens no browser context menu, so that
    /// the secondary button belongs to the layers (a right-button drag); false to keep the
    /// browser's menu. Not in C#.
    #[props(default = true)]
    suppress_context_menu: bool,
    /// True (the default) for a canvas that takes the focus and the pointer, wheel and key input;
    /// false for a DECORATIVE canvas (see above): no focus, no input listeners, pointer events
    /// pass through to the page beneath. Not in C#.
    #[props(default = true)]
    interactive: bool,
    /// True to draw nothing while the clock is paused unless something changed (see above);
    /// false (the default) draws every animation frame. A change applies from the next frame of
    /// the live canvas. Not in C#.
    #[props(default)]
    idle_when_paused: bool,
    /// How quick and how near presses must be to count as one series (the `tap_count` of
    /// [`CanvasInput::PointerDown`]); the default is the usual double-click. Not in C#.
    #[props(default)]
    tap_options: TapOptions,
) -> Element {
    let host = use_hook(|| Rc::new(RefCell::new(Host::default())));
    let clock = use_hook(|| clock.clone().unwrap_or_default());
    {
        let host = host.clone();
        use_drop(move || host.borrow_mut().stop());
    }
    // Followed on a live canvas: the frame loop reads it every frame.
    host.borrow().idle_when_paused.set(idle_when_paused);

    let mounted = {
        let host = host.clone();
        let context = context.clone();
        let layers = layers.clone();
        move |event: MountedEvent| {
            let Some(canvas) = event.data().downcast::<web_sys::Element>().and_then(|e| e.clone().dyn_into::<web_sys::HtmlCanvasElement>().ok()) else {
                return;
            };
            if interactive {
                let _ = canvas.style().set_property("touch-action", "none");
            } else {
                let _ = canvas.style().set_property("pointer-events", "none");
            }
            let settings = Settings { transparent, clear_color };
            host.borrow_mut().start(canvas, context.clone(), layers.clone(), clock.clone(), settings);
        }
    };
    let pointer_down = {
        let host = host.clone();
        let layers = layers.clone();
        move |event: PointerEvent| {
            let point = event.element_coordinates();
            let id = event.pointer_id();
            let button = pointer_button(event.trigger_button());
            let kind = pointer_kind(&event.pointer_type());
            let (tap_count, layers) = {
                let mut host = host.borrow_mut();
                if let Some(canvas) = host.canvas.as_ref() {
                    let _ = canvas.set_pointer_capture(id);
                    let _ = canvas.focus();
                }
                let others_down = host.drag.others_pressed(id);
                host.drag.press(id, point.x, point.y);
                (host.taps.press(&tap_options, now_ms(), point.x, point.y, kind, button, others_down), host.input_layers(&layers))
            };
            let input = CanvasInput::PointerDown {
                x: point.x,
                y: point.y,
                pointer_id: id,
                button,
                buttons: pointer_buttons(event.held_buttons()),
                kind,
                modifiers: modifier_keys(event.modifiers()),
                tap_count,
            };
            send(&layers, &input);
        }
    };
    let pointer_move = {
        let host = host.clone();
        let layers = layers.clone();
        move |event: PointerEvent| {
            let point = event.element_coordinates();
            let id = event.pointer_id();
            let buttons = pointer_buttons(event.held_buttons());
            let pressed = !buttons.is_empty();
            let ((dx, dy), layers) = {
                let mut host = host.borrow_mut();
                (host.drag.moved(id, point.x, point.y, pressed), host.input_layers(&layers))
            };
            let input = CanvasInput::PointerMove { x: point.x, y: point.y, dx, dy, pressed, pointer_id: id, buttons, kind: pointer_kind(&event.pointer_type()), modifiers: modifier_keys(event.modifiers()) };
            send(&layers, &input);
        }
    };
    let pointer_up = {
        let host = host.clone();
        let layers = layers.clone();
        move |event: PointerEvent| {
            let point = event.element_coordinates();
            let id = event.pointer_id();
            let layers = {
                let mut host = host.borrow_mut();
                host.drag.release(id);
                host.input_layers(&layers)
            };
            let input = CanvasInput::PointerUp {
                x: point.x,
                y: point.y,
                pointer_id: id,
                button: pointer_button(event.trigger_button()),
                buttons: pointer_buttons(event.held_buttons()),
                kind: pointer_kind(&event.pointer_type()),
                modifiers: modifier_keys(event.modifiers()),
            };
            send(&layers, &input);
        }
    };
    let pointer_cancel = pointer_up.clone();
    // The middle button's press is the layers': no autoscroll starts over the canvas.
    let mouse_down = move |event: MouseEvent| {
        if pointer_button(event.trigger_button()) == PointerButton::Middle {
            event.prevent_default();
        }
    };
    let context_menu = move |event: MouseEvent| {
        if suppress_context_menu {
            event.prevent_default();
        }
    };
    let wheel = {
        let host = host.clone();
        let layers = layers.clone();
        move |event: WheelEvent| {
            // The wheel is the layers' over the canvas: the page does not scroll.
            event.prevent_default();
            let (delta_x, delta_y) = wheel_in_pixels(event.delta());
            let layers = host.borrow().input_layers(&layers);
            send(&layers, &CanvasInput::Wheel { delta_x, delta_y });
        }
    };
    let key_down = {
        let host = host.clone();
        let layers = layers.clone();
        move |event: KeyboardEvent| {
            let key = event.key().to_string();
            if is_canvas_key(&key) {
                event.prevent_default();
            }
            let layers = host.borrow().input_layers(&layers);
            send(&layers, &CanvasInput::KeyDown { key });
        }
    };
    let key_up = move |event: KeyboardEvent| {
        let key = event.key().to_string();
        let layers = host.borrow().input_layers(&layers);
        send(&layers, &CanvasInput::KeyUp { key });
    };

    if !interactive {
        // A decorative canvas: no focus and no input listeners (see the documentation above).
        return rsx! {
            canvas {
                id: if id.is_empty() { None } else { Some(id) },
                class: if class.is_empty() { None } else { Some(class) },
                style: if style.is_empty() { None } else { Some(style) },
                "aria-label": if label.is_empty() { None } else { Some(label) },
                onmounted: mounted,
            }
        };
    }
    rsx! {
        canvas {
            id: if id.is_empty() { None } else { Some(id) },
            class: if class.is_empty() { None } else { Some(class) },
            style: if style.is_empty() { None } else { Some(style) },
            tabindex: "0",
            "aria-label": if label.is_empty() { None } else { Some(label) },
            onmounted: mounted,
            onpointerdown: pointer_down,
            onpointermove: pointer_move,
            onpointerup: pointer_up,
            onpointercancel: pointer_cancel,
            onmousedown: mouse_down,
            oncontextmenu: context_menu,
            onwheel: wheel,
            onkeydown: key_down,
            onkeyup: key_up,
        }
    }
}

/// Hands one input event to every layer, bottom first. A layer may change what it draws on
/// input, so a canvas that idles while its clock is paused draws the next frame.
fn send(layers: &CanvasLayers, input: &CanvasInput) {
    for layer in layers.snapshot() {
        layer.with(|r| r.input(input));
    }
    layers.request_redraw();
}

/// The time of an input event, in milliseconds (the tap count's clock: the page's wall clock,
/// which is all a double-click window needs).
fn now_ms() -> f64 {
    web_sys::js_sys::Date::now()
}

/// What the host takes from the component's properties when the canvas is mounted.
#[derive(Clone, Copy)]
struct Settings {
    transparent: bool,
    clear_color: Option<wgpu::Color>,
}

/// The host's state: the canvas element, the layers it draws (taken at mount), the running frame
/// loop, the pressed pointers, the last presses' series, and the `idle_when_paused` property as
/// the latest render passed it (shared with the frame loop).
#[derive(Default)]
struct Host {
    canvas: Option<web_sys::HtmlCanvasElement>,
    layers: Option<CanvasLayers>,
    frame_loop: Option<FrameLoop>,
    drag: DragTracker,
    taps: TapCounter,
    idle_when_paused: Rc<Cell<bool>>,
}

impl Host {
    /// The layers an input event goes to: the layers the host draws (taken when the canvas was
    /// mounted), so that input never reaches a list that is not drawn; `given` (the render's
    /// property) only while the host has not started.
    fn input_layers(&self, given: &CanvasLayers) -> CanvasLayers {
        self.layers.clone().unwrap_or_else(|| given.clone())
    }

    fn start(&mut self, canvas: web_sys::HtmlCanvasElement, context: GpuContext, layers: CanvasLayers, clock: CanvasClock, settings: Settings) {
        self.stop();
        self.layers = Some(layers.clone());
        let surface = match canvas_surface(&context, &canvas) {
            Ok(surface) => surface,
            Err(reason) => {
                let failure = SurfaceFailure::CreateSurface(reason);
                for layer in layers.snapshot() {
                    layer.with(|r| r.surface_failed(&failure));
                }
                return;
            }
        };
        let (alpha_mode, default_clear) = if settings.transparent {
            (wgpu::CompositeAlphaMode::PreMultiplied, wgpu::Color::TRANSPARENT)
        } else {
            (wgpu::CompositeAlphaMode::Opaque, wgpu::Color::BLACK)
        };
        let mut presenter = Presenter {
            context,
            canvas: canvas.clone(),
            surface,
            alpha_mode,
            clear_color: settings.clear_color.unwrap_or(default_clear),
            config: None,
            target: None,
            compose: None,
            failed: false,
            layers,
            states: Vec::new(),
            clock,
            idle_when_paused: self.idle_when_paused.clone(),
            watch: IdleWatch::default(),
        };
        self.canvas = Some(canvas);
        self.frame_loop = Some(FrameLoop::start(move |time| presenter.frame(time)));
    }

    fn stop(&mut self) {
        if let Some(frame_loop) = self.frame_loop.take() {
            frame_loop.stop();
        }
        self.canvas = None;
        self.layers = None;
        self.drag.clear();
        self.taps.clear();
    }
}

/// Makes the canvas a `wgpu` surface (the browser's WebGPU backend).
#[cfg(target_arch = "wasm32")]
fn canvas_surface(context: &GpuContext, canvas: &web_sys::HtmlCanvasElement) -> Result<wgpu::Surface<'static>, String> {
    context.instance().create_surface(wgpu::SurfaceTarget::Canvas(canvas.clone())).map_err(|error| error.to_string())
}

/// Outside a browser build there is no canvas surface (the crate still compiles for the host, so
/// that its unit tests run there).
#[cfg(not(target_arch = "wasm32"))]
fn canvas_surface(_context: &GpuContext, _canvas: &web_sys::HtmlCanvasElement) -> Result<wgpu::Surface<'static>, String> {
    Err("a canvas becomes a WebGPU surface only in a browser build (target wasm32-unknown-unknown)".to_string())
}

/// The surface and what draws into it, owned by the frame loop.
struct Presenter {
    context: GpuContext,
    canvas: web_sys::HtmlCanvasElement,
    surface: wgpu::Surface<'static>,
    alpha_mode: wgpu::CompositeAlphaMode,
    clear_color: wgpu::Color,
    config: Option<wgpu::SurfaceConfiguration>,
    target: Option<CanvasTarget>,
    /// The last pass of a transparent canvas (None for an opaque one).
    compose: Option<PageCompose>,
    failed: bool,
    layers: CanvasLayers,
    /// What each layer drawn so far was told (prepared for which target, sized to what) and its
    /// own timeline.
    states: Vec<(RendererHandle, LayerState)>,
    clock: CanvasClock,
    /// Draw only what shows something new while the clock is paused (see `WebGpuCanvas`): the
    /// property as the latest render passed it.
    idle_when_paused: Rc<Cell<bool>>,
    watch: IdleWatch,
}

impl Presenter {
    /// The canvas's displayed size in device pixels and the pixel ratio; None while the canvas
    /// is not laid out.
    fn wanted_size(&self) -> Option<(u32, u32, f64)> {
        let ratio = pixel_ratio(web_sys::window().map(|w| w.device_pixel_ratio()));
        let limit = self.context.device().limits().max_texture_dimension_2d;
        let (width, height) = surface_size(self.canvas.client_width(), self.canvas.client_height(), ratio, limit)?;
        Some((width, height, ratio))
    }

    /// Configures the surface on the first frame and again whenever the wanted size changes (or
    /// `force`, after the surface was lost). False when there is no surface to draw into.
    fn fit(&mut self, width: u32, height: u32, force: bool) -> bool {
        if self.failed {
            return false;
        }
        let unchanged = self.config.as_ref().is_some_and(|c| c.width == width && c.height == height);
        if unchanged && !force {
            return true;
        }
        let config = match self.config.take() {
            Some(mut config) => {
                config.width = width;
                config.height = height;
                config
            }
            None => match self.surface.get_default_config(self.context.adapter(), width, height) {
                Some(mut config) => {
                    // The browser's preferred canvas format comes first; the layers draw through
                    // its sRGB view.
                    let surface_format = config.format.remove_srgb_suffix();
                    let format = surface_format.add_srgb_suffix();
                    config.format = surface_format;
                    config.view_formats = if format == surface_format { Vec::new() } else { vec![format] };
                    config.alpha_mode = self.alpha_mode;
                    config.present_mode = wgpu::PresentMode::Fifo;
                    self.target = Some(CanvasTarget { format, surface_format, alpha_mode: self.alpha_mode });
                    if self.alpha_mode == wgpu::CompositeAlphaMode::PreMultiplied {
                        self.compose = Some(PageCompose::new(self.context.device(), surface_format));
                    }
                    config
                }
                None => {
                    self.failed = true;
                    for layer in self.layers.snapshot() {
                        layer.with(|r| r.surface_failed(&SurfaceFailure::NotPresentable));
                    }
                    return false;
                }
            },
        };
        self.surface.configure(self.context.device(), &config);
        self.config = Some(config);
        true
    }

    fn frame(&mut self, time_ms: f64) {
        let Some((width, height, pixel_ratio)) = self.wanted_size() else {
            return;
        };
        // Read before the layers draw: a change they make while drawing asks for the next frame.
        let shown = Shown { width, height, pixel_ratio, layers: self.layers.changes(), clock: self.clock.changes() };
        if !self.watch.decide(self.idle_when_paused.get(), self.clock.is_paused(), shown, time_ms) {
            return;
        }
        if !self.fit(width, height, false) {
            return;
        }
        let texture = match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(texture) | wgpu::CurrentSurfaceTexture::Suboptimal(texture) => texture,
            wgpu::CurrentSurfaceTexture::Outdated | wgpu::CurrentSurfaceTexture::Lost => {
                self.fit(width, height, true);
                self.watch.request();
                return;
            }
            wgpu::CurrentSurfaceTexture::Timeout | wgpu::CurrentSurfaceTexture::Occluded | wgpu::CurrentSurfaceTexture::Validation => {
                self.watch.request();
                return;
            }
        };
        let (Some(config), Some(target)) = (self.config.as_ref(), self.target) else {
            return;
        };

        // Prepare and size each layer that needs it, in order; forget the layers that are gone.
        let layers = self.layers.snapshot_with_scales();
        let mut states = Vec::with_capacity(layers.len());
        for (layer, _) in &layers {
            let mut state = self.states.iter().find(|(known, _)| known == layer).map(|(_, state)| *state).unwrap_or_default();
            let calls = state.before_frame(target, (width, height));
            layer.with(|r| {
                if calls.prepare {
                    r.prepare(&self.context, &target);
                }
                if calls.resize {
                    r.resize(&self.context, width, height);
                }
            });
            states.push((layer.clone(), state));
        }
        self.states = states;

        // A transparent canvas: the layers draw into the host's texture, written onto the surface
        // by the compose pass at the end of the frame (see the component's documentation).
        let surface_view = texture.texture.create_view(&wgpu::TextureViewDescriptor::default());
        let own = self.compose.as_mut().map(|c| c.target(self.context.device(), target.format, width, height));
        let (layer_texture, view) = match own {
            Some((t, v)) => (t.clone(), v.clone()),
            None => (texture.texture.clone(), texture.texture.create_view(&wgpu::TextureViewDescriptor { format: Some(target.format), ..wgpu::TextureViewDescriptor::default() })),
        };
        let mut encoder = self.context.device().create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("codebrix-webgpu frame") });
        {
            // The host's clear: the layers only ever load the target.
            let _clear = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("codebrix-webgpu clear"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations { load: wgpu::LoadOp::Clear(self.clear_color), store: wgpu::StoreOp::Store },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
        }
        let tick = self.clock.tick(time_ms);
        let points = self.clock.points();
        let info = FrameInfo { number: tick.number, time: tick.time, delta: tick.delta, previous_time: tick.previous, width, height, pixel_ratio };
        let mut frame = CanvasFrame {
            context: &self.context,
            surface: &self.surface,
            config,
            target,
            texture: &layer_texture,
            view: &view,
            encoder: &mut encoder,
            info,
            points: &points,
        };
        for ((layer, scale), (_, state)) in layers.iter().zip(self.states.iter_mut()) {
            // Each layer on its own timeline: the clock's, unless it has a time scale.
            frame.info = state.time.frame(*scale, info);
            layer.with(|r| r.render(&mut frame));
        }
        if let Some(compose) = &self.compose {
            compose.write(&mut encoder, &surface_view);
        }
        self.context.queue().submit([encoder.finish()]);
        drop(view);
        drop(surface_view);
        self.context.queue().present(texture);
        self.watch.drawn(shown);
    }
}

#[cfg(test)]
mod tests {
    use super::Host;
    use crate::layers::CanvasLayers;
    use crate::renderer::{CanvasFrame, CanvasRenderer, RendererHandle};

    struct Nothing;

    impl CanvasRenderer for Nothing {
        fn render(&mut self, _frame: &mut CanvasFrame<'_>) {}
    }

    #[test]
    fn input_goes_to_the_layers_that_are_drawn() {
        //Arrange: the layers taken at mount, and a page that passes ANOTHER list on a later render
        let drawn = CanvasLayers::single(RendererHandle::new(Nothing));
        let passed_later = CanvasLayers::single(RendererHandle::new(Nothing));
        let mut host = Host::default();
        //Act
        let before_mount = host.input_layers(&passed_later);
        host.layers = Some(drawn.clone());
        let mounted = host.input_layers(&passed_later);
        host.layers = None;
        let stopped = host.input_layers(&passed_later);
        //Assert
        assert!(before_mount == passed_later, "nothing is drawn yet: the render's layers");
        assert!(mounted == drawn, "the drawn layers, not the later render's");
        assert!(stopped == passed_later);
    }

    #[test]
    fn the_idle_property_is_shared_with_the_frame_loop() {
        let host = Host::default();
        let frame_loop_copy = host.idle_when_paused.clone();
        host.idle_when_paused.set(true);
        assert!(frame_loop_copy.get(), "a later render's value reaches the running loop");
        host.idle_when_paused.set(false);
        assert!(!frame_loop_copy.get());
    }
}
