//! The canvas host component: a Dioxus `canvas` that becomes a `wgpu` surface when it is
//! mounted, kept at the canvas's size in device pixels, cleared and drawn by its layers once per
//! animation frame with the time of one clock, and passing pointer, wheel and key input to them.

use std::cell::RefCell;
use std::rc::Rc;

use dioxus::prelude::*;
use wasm_bindgen::JsCast;

use crate::clock::CanvasClock;
use crate::context::GpuContext;
use crate::drag::DragTracker;
use crate::frame_loop::FrameLoop;
use crate::input::{is_canvas_key, modifier_keys, pointer_button, pointer_buttons, pointer_kind, wheel_in_pixels, CanvasInput, PointerButton};
use crate::layers::{CanvasLayers, LayerState};
use crate::page_compose::PageCompose;
use crate::renderer::{CanvasFrame, CanvasTarget, FrameInfo, RendererHandle, SurfaceFailure};
use crate::sizing::{pixel_ratio, surface_size};

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
/// modifier keys held ([`CanvasInput`]).
///
/// `context`, `layers` and `clock` are taken when the canvas is mounted: keep each in the page
/// (`use_hook`) and change the layers through the [`CanvasLayers`] handle, not by passing another.
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
) -> Element {
    let host = use_hook(|| Rc::new(RefCell::new(Host::default())));
    let clock = use_hook(|| clock.clone().unwrap_or_default());
    {
        let host = host.clone();
        use_drop(move || host.borrow_mut().stop());
    }

    let mounted = {
        let host = host.clone();
        let context = context.clone();
        let layers = layers.clone();
        move |event: MountedEvent| {
            let Some(canvas) = event.data().downcast::<web_sys::Element>().and_then(|e| e.clone().dyn_into::<web_sys::HtmlCanvasElement>().ok()) else {
                return;
            };
            let _ = canvas.style().set_property("touch-action", "none");
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
            {
                let mut host = host.borrow_mut();
                if let Some(canvas) = host.canvas.as_ref() {
                    let _ = canvas.set_pointer_capture(id);
                    let _ = canvas.focus();
                }
                host.drag.press(id, point.x, point.y);
            }
            let input = CanvasInput::PointerDown {
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
    let pointer_move = {
        let host = host.clone();
        let layers = layers.clone();
        move |event: PointerEvent| {
            let point = event.element_coordinates();
            let id = event.pointer_id();
            let buttons = pointer_buttons(event.held_buttons());
            let pressed = !buttons.is_empty();
            let (dx, dy) = host.borrow_mut().drag.moved(id, point.x, point.y, pressed);
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
            host.borrow_mut().drag.release(id);
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
        let layers = layers.clone();
        move |event: WheelEvent| {
            // The wheel is the layers' over the canvas: the page does not scroll.
            event.prevent_default();
            let (delta_x, delta_y) = wheel_in_pixels(event.delta());
            send(&layers, &CanvasInput::Wheel { delta_x, delta_y });
        }
    };
    let key_down = {
        let layers = layers.clone();
        move |event: KeyboardEvent| {
            let key = event.key().to_string();
            if is_canvas_key(&key) {
                event.prevent_default();
            }
            send(&layers, &CanvasInput::KeyDown { key });
        }
    };
    let key_up = move |event: KeyboardEvent| {
        let key = event.key().to_string();
        send(&layers, &CanvasInput::KeyUp { key });
    };

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

/// Hands one input event to every layer, bottom first.
fn send(layers: &CanvasLayers, input: &CanvasInput) {
    for layer in layers.snapshot() {
        layer.with(|r| r.input(input));
    }
}

/// What the host decides from the component's properties.
#[derive(Clone, Copy)]
struct Settings {
    transparent: bool,
    clear_color: Option<wgpu::Color>,
}

/// The host's state: the canvas element, the running frame loop and the pressed pointers.
#[derive(Default)]
struct Host {
    canvas: Option<web_sys::HtmlCanvasElement>,
    frame_loop: Option<FrameLoop>,
    drag: DragTracker,
}

impl Host {
    fn start(&mut self, canvas: web_sys::HtmlCanvasElement, context: GpuContext, layers: CanvasLayers, clock: CanvasClock, settings: Settings) {
        self.stop();
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
        };
        self.canvas = Some(canvas);
        self.frame_loop = Some(FrameLoop::start(move |time| presenter.frame(time)));
    }

    fn stop(&mut self) {
        if let Some(frame_loop) = self.frame_loop.take() {
            frame_loop.stop();
        }
        self.canvas = None;
        self.drag.clear();
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
    /// What each layer drawn so far was told (prepared for which target, sized to what).
    states: Vec<(RendererHandle, LayerState)>,
    clock: CanvasClock,
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
        if !self.fit(width, height, false) {
            return;
        }
        let texture = match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(texture) | wgpu::CurrentSurfaceTexture::Suboptimal(texture) => texture,
            wgpu::CurrentSurfaceTexture::Outdated | wgpu::CurrentSurfaceTexture::Lost => {
                self.fit(width, height, true);
                return;
            }
            wgpu::CurrentSurfaceTexture::Timeout | wgpu::CurrentSurfaceTexture::Occluded | wgpu::CurrentSurfaceTexture::Validation => return,
        };
        let (Some(config), Some(target)) = (self.config.as_ref(), self.target) else {
            return;
        };

        // Prepare and size each layer that needs it, in order; forget the layers that are gone.
        let layers = self.layers.snapshot();
        let mut states = Vec::with_capacity(layers.len());
        for layer in &layers {
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
        let mut frame = CanvasFrame {
            context: &self.context,
            surface: &self.surface,
            config,
            target,
            texture: &layer_texture,
            view: &view,
            encoder: &mut encoder,
            info: FrameInfo { number: tick.number, time: tick.time, delta: tick.delta, previous_time: tick.previous, width, height, pixel_ratio },
            points: &points,
        };
        for layer in &layers {
            layer.with(|r| r.render(&mut frame));
        }
        if let Some(compose) = &self.compose {
            compose.write(&mut encoder, &surface_view);
        }
        self.context.queue().submit([encoder.finish()]);
        drop(view);
        drop(surface_view);
        self.context.queue().present(texture);
    }
}
