# CodeBrix.WebPlatform.WebGpu

WebGPU for the browser frontends of CodeBrix.WebPlatform applications: packages that bring the Rust crates for showing 3D graphics, scenes and animations in the browser window to an application's Rust frontend, as crate bundles that the application's build program registers - so that the frontend still builds offline, with nothing to download. The frontend uses WebGPU only, with no WebGL fallback, and low-level access to the `wgpu` API always stays available beside anything built on it. CodeBrix.WebPlatform.WebGpu is provided as .NET 10 libraries and the associated `CodeBrix.WebPlatform.WebGpu.ApacheLicenseForever`, `CodeBrix.WebPlatform.WebGpu.Graphics.ApacheLicenseForever` and `CodeBrix.WebPlatform.WebGpu.Animation.ApacheLicenseForever` NuGet packages.

CodeBrix.WebPlatform.WebGpu supports applications and assemblies that target Microsoft .NET version 10.0 and later.
Microsoft .NET version 10.0 is a Long-Term Supported (LTS) version of .NET, and was released on Nov 11, 2025; and will be actively supported by Microsoft until Nov 14, 2028.
Please update your C#/.NET code and projects to the latest LTS version of Microsoft .NET.

## Installation

```
dotnet add package CodeBrix.WebPlatform.WebGpu.ApacheLicenseForever
```

(Add it to the application's BUILD PROGRAM - the program that references `CodeBrix.WebPlatform.FrontendBuilder` and materializes the frontend.)

Note that the NuGet package IDs and the namespaces are different - there are no packages named plain `CodeBrix.WebPlatform.WebGpu`, `CodeBrix.WebPlatform.WebGpu.Graphics` or `CodeBrix.WebPlatform.WebGpu.Animation`:

* NuGet package ID: `CodeBrix.WebPlatform.WebGpu.ApacheLicenseForever` - assembly and primary namespace `CodeBrix.WebPlatform.WebGpu`
* NuGet package ID: `CodeBrix.WebPlatform.WebGpu.Graphics.ApacheLicenseForever` - assembly and primary namespace `CodeBrix.WebPlatform.WebGpu.Graphics`
* NuGet package ID: `CodeBrix.WebPlatform.WebGpu.Animation.ApacheLicenseForever` - assembly and primary namespace `CodeBrix.WebPlatform.WebGpu.Animation`

XML documentation (IntelliSense) ships alongside each assembly.

Every package depends on `CodeBrix.WebPlatform.Core`, at exactly the platform version its crates were chosen against; the Graphics and Animation packages also depend on exactly the same version of the `CodeBrix.WebPlatform.WebGpu` package (the three are built and published together), and never on each other. Use these packages with the CodeBrix.WebPlatform packages of that same version.

## CodeBrix.WebPlatform.WebGpu supports:

* The Rust `wgpu` crates for WebGPU in the browser, the `glam` maths crate, the `png` crate for PNG files and the `zune-jpeg` crate for JPEG files, and every crate they need, carried by the `CodeBrix.WebPlatform.WebGpu` package exactly as crates.io published them, as one archive
* The crate `codebrix-webgpu`, carried in the same archive: the WebGPU context that owns or shares the device, one canvas host component with its frame loop and input (pointer buttons, pointer kind and modifier keys, tap counts for double-clicks and double taps; no browser context menu or autoscroll over the canvas), layers of independent renderers on one canvas with one clock (pause, resume, speed, named points in time) and a time scale of their own per layer, a decorative canvas that takes no input and lets the pointer through to the page, a frame loop that can idle while the clock is paused, a transparent canvas for drawing "on the page" (the page shows through where nothing is drawn, with correct blending at translucent parts and edges), the "WebGPU is not available" result and message, the maths types, and PNG and JPEG files decoded into pixels in the browser, each format by its own function so that an application carries only the decoder it calls - all without JavaScript
* One registration call in the build program that hands those crates to the platform's frontend build as a crate bundle, so that they are written into the frontend beside the platform's own crates and the frontend builds offline
* Crates shared with the platform kept at the platform's exact versions and content, so that the platform's strict rule for crate bundles is met
* The generated third-party notices for those crates, handed to the build program to serve with the site
* WebGPU only, with no WebGL fallback: a frontend shows its own "WebGPU is not available" message when the browser gives it no adapter
* One uncaptured-error handler per device, installed by `codebrix-webgpu`, with any number of readers that each see every error - so that several libraries can share one device
* Drawing without a canvas: the graphics crate's frames on a target of their own and the animation crate's renderer drawing into a texture of the application's own, so that a program with no window (one that brings a native `wgpu` backend itself) draws with the same code as the page
* `CodeBrix.WebPlatform.WebGpu.Graphics` and its crate `codebrix-webgpu-graphics`: the graphics context, buffers, textures (also from PNG and JPEG files), samplers, shaders, bind groups, the render pipeline builder, frames and render passes of the .NET library CodeBrix.WebGpu.Graphics, with the same type names and argument order, drawing as a layer of the canvas host; the library's built-in WGSL shaders, compiled by the browser; device creation, readback and shader diagnostics as async calls, and WebGPU's validation errors as a queue
* 2D sprites and shapes with the `SpriteBatch` (texture atlases, tint, rotation, scale and origin, a transform stack, rectangles, lines, triangles and circles, one draw call per texture run)
* Lit 3D meshes: the built-in cube, box, plane and sphere, the `MeshRenderer` with lit materials, depth, instancing and translucency, perspective and orthographic cameras, and an orbit controller driven by the canvas's input (drag to orbit, the wheel and a pinch to zoom - orthographic views included -, right, middle or Shift drags and two fingers to pan, a box to frame the view on, limits of the application's own, auto-orbit until the first drag); world-to-canvas projection for labels, and rays from a canvas point for picking
* glTF models with physically based shading: `.gltf` and `.glb` files served as they are and read in the browser by the crate's glTF loader (the `gltf` crate, carried by the Graphics package exactly as crates.io published it, with the loader of the .NET library ported on top: meshes, materials, PNG and JPEG textures, nodes, skins, morph targets and animations, with a stated table of what is read, approximated and refused), textures and vertex data shared between models; and a binary model-data format owned by the Graphics package for models an application makes in .NET, with a .NET writer and the crate's reader - with `Model`, `ModelInstance` and the `ModelRenderer` (instanced draws, every glTF metallic-roughness material input, alpha modes, mirroring, opaque and blended parts drawn in two steps so the sky goes between them, translucent parts sorted by their animated position and double-sided ones drawn back faces first, a tint, opacity and emissive strength per instance, nodes shown or hidden per instance, up to eight directional lights, image-based or hemispheric ambient light); the crate takes the bytes from anywhere and never fetches them itself
* Point lights and spotlights, and shadows of the sun (in cascades on request) and of spotlights, in the model renderer - switched on in the scene's lighting, with no change to the drawing calls
* Frustum culling, on by default, with bounds that hold every pose of a model's animation clips, and level of detail: several models of one object chosen by distance, dissolving into one another through a fine pixel pattern so that nothing pops
* Multisampled anti-aliasing (4 samples) per canvas or offscreen target, switchable between frames, for every renderer of the crate
* Bloom on the HDR path, a glow layer of soft light sprites (facing the camera or fixed in the scene), GPU particle systems with several emitters, lit or stretched along their motion, driven by the frame's time, and frame timings (CPU and per-pass GPU time) for read-outs
* Water: a wave field simulated on the GPU (rain, a pointer, wakes, walls and shallows), open-water waves as an exact function of time, and a water surface for 3D scenes, shaped by a mask and flowing if asked, with refraction, depth tint, caustics, reflections and highlights
* Animation playback, skinning and morph targets: each model instance's `AnimationPlayer` plays the model's clips (step, linear and cubic-spline keys; loop, ping-pong and once; speed, seeking, cross-fades and weighted, layered blending), driven by the frame's time from the canvas host's clock, and the `ModelRenderer` skins and morphs the posed instances on the GPU
* Image-based lighting, a skybox and tone mapping: environments baked on the GPU from a procedural sky or from a high-dynamic-range panorama (given as linear pixels, or as the bytes of a Radiance `.hdr` file, which the crate decodes), the BRDF lookup table, the `SkyboxRenderer`, an HDR render path through an offscreen RGBA16Float target, and the `ToneMapper` with ACES (fitted and Narkowicz), Khronos PBR Neutral, Reinhard and clamping, with an exposure and an option that keeps the scene's alpha, so models can be drawn over the page with HDR on
* Animated textures (frames with durations, looping, ping-pong or once, as a texture array or streamed through a ring of layers, drawn by the `SpriteBatch`), render bundles recorded once and replayed every frame, indirect draws and dispatches, compute pipelines and passes, GPU time per pass from timestamp queries where the browser grants them, buffer mapping, texture regions and copies, and the asynchronous readback of textures and offscreen targets
* One list in the AGENT-README of what the .NET library has and a browser cannot do, with what to use instead
* `CodeBrix.WebPlatform.WebGpu.Animation` and its crate `codebrix-webgpu-animation`: real-time 2D animation with the names and concepts of the .NET library CodeBrix.Imaging.Drawing.NoSkia - a drawing session of calibrated units with layers of shapes (rectangles with rounded corners, circles, ellipses, lines, polylines and polygons, paths parsed from SVG path data or built, filled with colours or linear and radial gradient fills and stroked with caps, joins, miter limits and dashes), single lines of text in fonts the application supplies as bytes (shaped with kerning and ligatures, aligned, tracked and measured, never a system font, with a choice of what a character the font lacks shows and a report of it), whole-session and layer fades, the debug overlay, the animation model (keyframe tracks with per-segment easing, holds and loops, bound to the properties of elements, layers and a camera with parallax; transforms, groups, time windows, shapes rebuilt for each moment, and layout against the frame or a safe rectangle), paths that are drawn on by their trims, followed by moving elements and morphed into one another, pictures and sprite sheets from decoded pixels the application supplies or from PNG and JPEG files the library decodes itself (stretched into place, tinted, sampled smoothly or as hard pixels, a sprite's frame chosen by the time), masks (inside or outside a path, or a matte from another layer) and blur, glow and drop-shadow effects on elements, groups and layers that follow their scale and the camera, with every effect parameter bindable to a track, blocks of text wrapped, aligned and trimmed by the library's own line breaker in styled runs, revealed character by character, word by word or line by line, split into pieces that are staggered or sent rolling along a path, captions from SRT and WebVTT text placed against the frame (or, on request, inside a safe-area guide), path combining (union, intersection, difference, exclusive-or, overlap removal, stroke outlines, inflating and deflating) and path queries (inside, distance to the edge, nearest point, the gap between two areas), bindings that add to or multiply what other bindings set, elements rolling and travelling as trains along paths, formations of many elements filling a shape and arriving over time, layers switched on and off by tracks, and safe-area guides of video platforms with a preview overlay and a check of what reaches into their unsafe zones, SVG documents drawn as elements (shapes, paths, gradients, clips, masks, blend modes, a blur, text in fonts the application supplies, and pictures it supplies as pixels, PNG or JPEG files) whose parts are moved, turned, faded and trimmed by id and bound to tracks, and which play their own SMIL and CSS keyframe animations, every easing curve of the .NET library with steps and cubic-bezier curves, colour ramps, timelines with cues and markers, pixel planes with the .NET library's filters, blends, transitions, noise, particles and motion blur computed with its exact values, and their real-time counterparts drawn on the GPU (tone curve, gamma, vignette and bloom over a layer, transitions between layers, noise computed per pixel, particles in one instanced draw), animated brush strokes that paint themselves on, ownership maps with colour pickup and seams coloured on the GPU, hard-edged masks and keyframe players for painterly playback, and the playback controller - all driven by the frame's time from the canvas host's clock, never by a clock of their own
* The crate's own GPU renderer for 2D vector content, built on the `kurbo` curve crate and the `peniko` brush types, with text shaped by the `harfrust` crate, paths combined by the `i_overlay` crate on an integer grid, PNG and JPEG files read by the `png` and `zune-jpeg` crates (carried by the WebGpu package) and SVG documents read by the `usvg` crate (carried, with the others, by the Animation package exactly as crates.io published them): non-zero and even-odd fills, strokes, solid colours and linear, two-point radial and sweep gradients with pad, repeat and reflect, every blend and compose mode, nested clips, groups with opacity and blend, images, noise and ownership maps computed per pixel, instanced particles, stroke sets of tens of thousands of brushstrokes in one instanced draw, and the offscreen passes of masks, effects, whole-frame operations, transitions, the surface finish and water (separable Gaussian blurs, computed at a coarser resolution when they are wide); it draws as a layer of the canvas host over whatever the layers below drew, for example a 3D scene of the Graphics package, and caches outlines and batches draws between frames
* Lottie animations: Lottie JSON read through the `velato` crate (carried by the Animation package exactly as crates.io published it), played at the session's time as an element of a drawing session - loop, ping-pong or hold, speed, a range of frames or a marker, markers as named points in time on the clock, image layers drawn from pictures the application supplies as pixels, PNG or JPEG files - and drawn by the crate's own renderer, with a report of what a file uses that is not drawn as written; the crate's own pass over the JSON guards the reader against content it cannot handle
* Render-time looks for a whole drawing session that never change the scene - blend modes and opacity, a line drawing or flat colours, effects on or off, layers kept as authored - and soft wipes (a straight edge at any angle, or an iris) between any two renders; strokes of a fixed width whatever the scaling (SVG's non-scaling strokes included) or in CSS pixels of the page; sketched paths that boil like a drawn film; ownership maps re-coloured by sending only their colours; easing curves as constants; variable fonts at chosen axis values; and CPU and GPU frame timings for read-outs
* Painterly stroke sets: tens of thousands of brushstrokes in one GPU draw, painted on as a function of time, with the application's own brush tips, strokes that travel with the parts they depict, boil and flow; planning from a picture's structure, from noise or from guides, with detail maps, stroke layouts and painting timelines; colour rules (a limited palette, broken colour) and opt-in perceptual mixing (Oklab, Oklch) for ramps and gradients; colours picked every frame from a picture, another session or an application's texture such as a 3D scene; a lit canvas weave and paint thickness; a cache that draws a still painting from checkpoints, pixel for pixel the same; and stroke-by-stroke transitions between two paintings - with no artwork or brush styles included
* Water for 2D scenes: ripple fields of drops, seeded rain and swell as an exact function of time, a caustics pattern, layers and sessions seen through water (bent, shaded, with glints and caustic light) and mirrored below a waterline, broken up by the water - the water a ripple field, a height picture or an application's texture such as the Graphics package's simulated wave field
* An application may use Graphics or Animation, or both: only the application puts their renderers together, on one canvas with one clock

## Sample Code

### Register the WebGpu crates in the build program

```csharp
using CodeBrix.WebPlatform.FrontendBuilder;
using CodeBrix.WebPlatform.WebGpu;

WebGpuCrates.Register();

var frontend = new PlatformFrontend(new PlatformFrontendOptions
{
    FrontendDirectory = "frontend",
    BackendProjectPath = "backend/MyApp.Api/MyApp.Api.csproj",
});
await frontend.MaterializeAsync();
```

The frontend then names `codebrix-webgpu` in its own `Cargo.toml` and draws with it:

```rust
use codebrix_webgpu::{CanvasClock, CanvasLayers, GpuContext, RendererHandle, WebGpuCanvas, WebGpuUnavailable};
use dioxus::prelude::*;

#[component]
fn Page() -> Element {
    let gpu = use_resource(GpuContext::request);
    let clock = use_hook(CanvasClock::new);
    let layers = use_hook(|| CanvasLayers::new(vec![RendererHandle::new(Scene::new()), RendererHandle::new(Titles::new())]));
    let state = gpu.read();
    match &*state {
        None => rsx! { p { "Asking the browser for a WebGPU adapter..." } },
        Some(Err(reason)) => rsx! { WebGpuUnavailable { reason: reason.clone() } },
        Some(Ok(context)) => rsx! { WebGpuCanvas { context: context.clone(), layers, clock, class: "w-full aspect-video" } },
    }
}
```

(`Scene` and `Titles` are the application's renderers: each implements `CanvasRenderer` and draws with the `wgpu` device directly.)

### Leave WebGPU content out where the browser has no WebGPU

`WebGpuBoundary` makes its children only where WebGPU works. Elsewhere it shows its `fallback`, or nothing at all when it has none, so the visitor never sees a "not available" message. The children read the context with `use_gpu()`, and every boundary of a page shares one request to the browser. A page that needs more of the device (a power preference, features, limits) says so once with `set_page_gpu_options`:

```rust
use codebrix_webgpu::{use_gpu, CanvasClock, CanvasLayers, RendererHandle, WebGpuBoundary, WebGpuCanvas};
use dioxus::prelude::*;

#[component]
fn Page() -> Element {
    rsx! {
        h1 { "Welcome" }
        // An optional animation: where WebGPU is not available, it is simply not there.
        WebGpuBoundary { HeroAnimation {} }
        // A picture in its place instead.
        WebGpuBoundary {
            fallback: |_| rsx! { img { src: "/hero-still.webp" } },
            HeroAnimation {}
        }
    }
}

#[component]
fn HeroAnimation() -> Element {
    let gpu = use_gpu();
    let clock = use_hook(CanvasClock::new);
    let layers = use_hook(|| CanvasLayers::single(RendererHandle::new(Scene::new())));
    rsx! { WebGpuCanvas { context: gpu, layers, clock, class: "w-full aspect-video" } }
}
```

### Animate 2D shapes on the canvas

The build program registers the Animation package (`AnimationCrates.Register();`, which registers the WebGpu package's crates first), and the frontend names `codebrix-webgpu-animation` in its `Cargo.toml`:

```rust
use std::cell::RefCell;
use std::rc::Rc;
use codebrix_webgpu::{CanvasLayers, RendererHandle};
use codebrix_webgpu_animation::animation::{ColorRamp, Easing};
use codebrix_webgpu_animation::rendering::AnimationLayer;
use codebrix_webgpu_animation::shapes::{CircleShape, ShapeOptions};
use codebrix_webgpu_animation::{Color, DrawingSession, DrawingSessionOptions};

let mut session = DrawingSession::new(DrawingSessionOptions { calibration_size: (1280, 720), layer_opacity: 255, ..Default::default() });
session.add_layer("Dots", Color::from_rgb(255, 176, 72))
    .add_shape(CircleShape::new(640.0, 360.0, 40.0, ShapeOptions { is_filled: true, ..Default::default() })?);
let ramp = ColorRamp::from_colors(&[Color::from_rgb(214, 96, 120), Color::from_rgb(120, 90, 210)])?;
let session = Rc::new(RefCell::new(session));
let layer = AnimationLayer::new(move |frame, scene| {
    let mut s = session.borrow_mut();
    if let Some(dots) = s.get_layer_mut("Dots") {
        dots.set_color(ramp.at(Easing::sine_in_out((frame.info.time / 4.0).fract())));
    }
    s.set_time(frame.info.time);   // the frame's time from the host's one clock
    s.render(scene, frame.info.width, frame.info.height);
});
let layers = CanvasLayers::new(vec![RendererHandle::new(layer)]);
```

## Documentation

The NuGet packages include `AGENT-README.txt`, a complete API reference and usage guide written for AI coding agents - point your agent at that file when it is writing code against these libraries.

Additional sample code and usage examples are available in the test projects:
https://github.com/ellisnet/CodeBrix.WebPlatform.WebGpu/tree/main/tests

## License

CodeBrix.WebPlatform.WebGpu is licensed under the Apache License 2.0 - see the
[LICENSE](https://github.com/ellisnet/CodeBrix.WebPlatform.WebGpu/blob/main/LICENSE) file.

For licensing and provenance information about the open source code included in
this package, see [THIRD-PARTY-NOTICES.txt](https://github.com/ellisnet/CodeBrix.WebPlatform.WebGpu/blob/main/THIRD-PARTY-NOTICES.txt).
