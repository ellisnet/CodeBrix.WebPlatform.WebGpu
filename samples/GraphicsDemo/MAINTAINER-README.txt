================================================================================
MAINTAINER-README: the GraphicsDemo sample (CodeBrix.WebPlatform.WebGpu)
For people (and AI coding agents) changing the sample under
samples/GraphicsDemo
================================================================================

The libraries of this repository (src/, libs/, tests/, tools/ and the NuGet
packages) have their own maintainer file, MAINTAINER-README.txt at the
repository root. This file is about this sample only. How the packages are
used is in AGENT-README.txt at the repository root; how a CodeBrix.WebPlatform
application works (named frontend applications, the frontend build, crate
bundles) is in the AGENT-README.txt of the CodeBrix.WebPlatform packages.

PURPOSE AND SCOPE
=================
GraphicsDemo is a CodeBrix.WebPlatform application that draws with WebGPU
in the browser, through the Rust crates that the CodeBrix.WebPlatform.WebGpu
package brings as a CRATE BUNDLE: `wgpu`, and the package's own crate
`codebrix-webgpu` (the context, the canvas host with its layers and clock,
the maths); and through the crate the CodeBrix.WebPlatform.WebGpu.Graphics
package brings as a second bundle, `codebrix-webgpu-graphics` (the
counterpart of the .NET library CodeBrix.WebGpu.Graphics); and through the
crates the CodeBrix.WebPlatform.WebGpu.Animation package brings as a third
bundle, `codebrix-webgpu-animation` (the animation toolkit of the .NET
library CodeBrix.Imaging.Drawing.NoSkia, drawn by its own renderer) with the
third-party crates it needs (kurbo, peniko, harfrust, i_overlay, png, usvg,
velato and what they bring). It proves:

  - that a build program which registers the bundles (GraphicsCrates.Register()
    and AnimationCrates.Register(), each of which registers the WebGpu bundle
    first - once - before materializing) gets the crates -
    codebrix-webgpu, codebrix-webgpu-graphics and codebrix-webgpu-animation
    among them, in
    vendored form - into the frontends' one vendor folder beside the
    platform's, merged (the crates the platform already carries are kept
    once), and that the frontends then build OFFLINE in the default crate
    mode;
  - that codebrix-webgpu draws into a Dioxus canvas every frame under the
    site's strict Content-Security-Policy, with NO JavaScript of ours;
  - that two INDEPENDENT layers share one canvas and ONE clock: pausing the
    clock stops both, resuming continues both from the same time, both react
    to the same named point in time in the same frame, and the upper layer's
    translucent colour is blended over the lower one by the host's
    convention (premultiplied, linear, through the sRGB view);
  - that every built-in WGSL shader of codebrix-webgpu-graphics compiles in
    the browser, and that a textured quad drawn THROUGH that crate's
    wrapper runs as a layer on the host's clock;
  - that the crate's SpriteBatch and its lit-mesh renderer draw as layers on
    the host's clock, that its orbit controls (drag, wheel, touch) move the
    camera the way the content-follows-the-pointer convention says, within
    their limits, and that two libraries on one device share its one error
    handler;
  - that glTF models the backend serves AS FILES (the kits' .glb files,
    unchanged) are read in the browser by the graphics crate's glTF loader
    (GltfDocument, GltfModelSet - textures shared within a set, decoded and
    uploaded once) and drawn with physically based
    shading, lit by image-based light from an environment the crate bakes
    on the GPU (a procedural sky, or an HDR panorama whose .hdr bytes the
    backend serves and the crate decodes), with the skybox, through the HDR
    path and the tone mapper - and the desktop viewer's switches as query
    parameters;
  - that the crate's animation players play the models' clips on the host's
    clock (pause, speed and seeks of the clock move them), completing the
    desktop viewer's scene, and that its model renderer skins a skinned
    model and morphs a model with morph targets on the GPU;
  - that the animation application's scenes - the C# demo AnimationDemoApp's
    scenes, and scenes of their own for what the C# demo has none of (SVG
    documents that animate themselves, Lottie) - draw through
    codebrix-webgpu-animation as a layer on the host's clock: each moving
    while the clock runs, still while it is paused, and back exactly after a
    seek there and back; and that its fonts, pictures (decoded pixels and a
    PNG file), caption text, SVG documents and Lottie files come from the
    backend as bytes through the platform's generated client;
  - that a browser without WebGPU gets a clear message and nothing breaks.

ONE backend, ONE build program, ONE build, and TWO NAMED frontend
applications (there is no default one), under frontends/:

    graphics   served below /graphics. Asks the browser for a WebGPU adapter
               and device (WebGPU only: no WebGL fallback) and shows one of
               its SCENES, chosen with the buttons at the top
               (scene-viewer, scene-layers, scene-shaders, scene-quad,
               scene-sprites, scene-lit, scene-skinned, scene-morph,
               scene-more, scene-on-the-page; "Viewer" first, and the one shown when the page
               opens; then a second row, "Feature demos": scene-lights,
               scene-shadows, scene-culling, scene-translucency,
               scene-glow, scene-particles, scene-picking,
               scene-antialiasing, scene-lod). The button of
               the scene on show is filled and marked aria-current="true". The device is asked for with the optional feature
               timestamp-query WHERE THE ADAPTER HAS IT
               (GraphicsContextOptions::optional_features in src/main.rs;
               otherwise as GpuContext::request asks), for the "More"
               scene's GPU timer. What a scene fetched from the backend (the
               scene description, the model files, the blobs, the panorama)
               is KEPT FOR THE LIFE OF THE PAGE (src/page_cache.rs): coming
               back to a scene asks the backend for nothing. EVERY ORBIT
               CAMERA (Viewer, Lit shapes, Skinned model, Morph targets,
               More, the feature demos) orbits with a left drag, PANS with a right-button or
               middle-button drag or a Shift + left drag (two fingers pan
               while their spread zooms), and zooms with the wheel; each
               layer calls OrbitController::set_viewport once per frame, so
               the content follows the pointer exactly. The canvas has no
               context menu and a middle-button press starts no autoscroll
               (the canvas host's defaults):

               VIEWER (src/viewer_scene.rs, src/viewer.rs,
               src/viewer_actors.rs): the desktop model viewer's COMPLETE
               scene - its ground, its static props and its animated actors,
               placed and played exactly as the desktop viewer's
               ViewerScene does it: the chest in the middle playing
               "open-close"; eight animals, the even ones walking a ring of
               radius 5.4 ("walk", each with its own clip offset), the odd
               ones standing at radius 2.9 facing the chest and cross-fading
               (0.4 s) every 3.5 s through idle, dance, eat,
               gesture-positive, idle, dance; the Avian circling overhead the
               other way round, bobbing ("fly"). The backend's scene JSON
               describes each actor (model, clip or routine, time offset,
               motion "fixed", "walk" or "fly" with its numbers); the frontend
               has the desktop's placement functions of the scene time. TIME
               COMES FROM THE PAGE'S CLOCK: each frame advances every
               instance's AnimationPlayer by the frame's delta (pause, speed),
               and when the clock's time JUMPS (a seek: buttons "Clock -5 s"
               and "Clock +5 s"), the actors are re-posed for the new time
               from the schedule itself, so that a seek lands on the pose that
               playing up to that time gives; "Half speed" halves the clock's
               speed. The page asks the backend for the scene description
               (generated::api::models::scene), which names each model and
               where it comes from, then THE MODEL FILES (src/model_files.rs):
               each kit's .glb file, served unchanged
               (models::model_file_bytes, a [WireFile] action), parsed in the
               browser (GltfDocument::parse) and asked which files it names
               (external_files, with the kit's folder as the base address),
               and each named file once - the kit's shared colour map
               (models::model_folder_file_bytes) - and the GROUND, which the
               backend builds in code and serves as a model data blob
               (models::model_data_bytes("ground")); it shows a loading line
               while they arrive and a message that says what to do when a
               request fails. Then ONE GltfModelSet builds the model files in
               TWO calls of create_models - the actors' models first, then
               the props' - and the second reuses the GPU textures the first
               uploaded (each kit's colour map is decoded and uploaded ONCE;
               viewer-texture-sharing shows the check), the ground comes from
               its blob (GraphicsContext::load_model), every placement becomes
               a ModelInstance, and the ModelRenderer draws them with
               physically based shading. LIGHT, as the desktop viewer: the
               viewer's procedural late-morning sky (zenith, horizon and
               meadow-green ground colours, turbidity 2, its sun along the
               key light; the values come from the backend's scene JSON),
               baked once into an environment, gives the image-based ambient
               light and reflections, and the sun light (colour (1, 0.95,
               0.86), intensity 1.6) the direct light. A button switches to
               the Poly Haven "Small Cathedral" HDR panorama: the page fetches
               its .hdr bytes ONCE, the first time
               (generated::api::environments::small_cathedral_bytes), and the
               layer makes an environment of them
               (create_environment_from_bytes); the sun light is off then and
               the exposure is the desktop's 1.2. THE DRAW ORDER: the
               renderer's opaque parts (end_opaque), then the SKYBOX (it fills
               only what nothing opaque covered), then the blended parts
               (end_blended) over both. THE HDR PATH: the scene goes into an
               RGBA16Float HeadlessRenderTarget with a Depth24Plus depth, then
               the ToneMapper maps it into the canvas (Khronos PBR Neutral by
               default, as the desktop). THE SWITCHES are query parameters of
               the page: ?ldr draws straight into the canvas (no HDR target,
               no tone mapping), ?no-skybox clears to the viewer's flat sky
               colour instead of drawing the sky, and both may be combined
               (?ldr&no-skybox); the page links to the four combinations.
               CONTROLS: a plain HTML select for the tone-map operator
               (viewer-tone-operator-select: AcesFitted, KhronosPbrNeutral,
               Reinhard, AcesNarkowicz, Clamp) and a range input for the
               exposure (viewer-exposure-input, 0.25 to 4; both disabled with
               ?ldr), pause/resume, orbit by itself again, the environment
               switch, and the clock's seek and speed. The camera is the viewer's (field of view
               50 degrees, target (0, 0.9, 0), distance 10.5, pitch 0.30),
               driven by the OrbitController with the viewer's auto-orbit
               0.7 + 0.18 t and the controller's limits (and panning). The legend is HTML
               over the canvas; under it: the light, the cube size and the
               CPU time of the last environment's decoding and recording, the
               panorama's state, the render path, the operator and exposure
               actually drawn, the skybox, the models' files and bytes from
               the backend and the time they took to fetch and parse and to
               build and upload, the counts (models, textures, instances,
               triangles, draw calls, pipelines), the texture-sharing check, the camera's
               values, and the animation: the scene time, the actors and the
               clips playing, a fingerprint of the actors' pose, the Avian's
               position, the first standing animal's clips with their
               weights, and how often the actors were re-posed after a jump.

               LAYERS (src/main.rs, Scene): one canvas with TWO LAYERS on one
               clock: a triangle
               (src/triangle.rs; it spins with the clock, a drag turns it,
               the wheel scales it, and keys do both: arrow keys, + and -,
               the space bar starts and stops its spin, r resets), and over
               it a translucent orange square (src/overlay.rs, which knows
               nothing of the triangle) that glides with the clock. The page
               owns the clock and defines its named point "title in", at
               which the triangle turns lighter and the square appears. Three
               buttons: pause/resume the clock, remove/add the overlay layer
               (a running canvas's layers change), and make the square solid
               or translucent again (solid shows plainly that the square is
               the upper layer). A drag follows the pointer: to the right
               turns the triangle counter-clockwise (the angle grows) - the
               convention for every drag control of the sample.

               SHADERS (src/shader_scene.rs): every built-in WGSL shader of
               codebrix-webgpu-graphics, and every feature combination of
               its PBR shader, compiled once by this browser
               (GraphicsContext::try_create_shader), listed with the
               compiler's messages, with a summary line.

               TEXTURED QUAD (src/quad_scene.rs, src/quad.rs): a square with
               a texture made in code (four coloured quarters, a white bar
               along the top edge), turning with the canvas clock, drawn
               entirely THROUGH codebrix-webgpu-graphics - its context,
               vertex, index and uniform buffers, a mipmapped texture, a
               sampler, two bind groups, the pipeline builder and the
               library's built-in sprite shader - as a layer of the canvas
               host (it loads, never clears, and takes its time from the
               frame). A pause button, a drag that turns it further, and
               three one-off async checks: a buffer read back from the GPU,
               a pipeline built with WebGPU's verdict awaited
               (build_checked), and a deliberately invalid buffer whose
               validation error validate() returns. The page shows what
               wgpu says about the adapter, a frame counter, the clock's
               time and state, and the frame in which each layer first
               reacted to "title in".

               SPRITES (src/sprite_scene.rs, src/sprites.rs): one
               SpriteBatch of codebrix-webgpu-graphics, as a layer on the
               host's clock, drawing from a 4-cell texture atlas made in code
               (disc, star, arrow, checker) and shapes: the cells in a row,
               tinted with cycling colours (tint, source rectangles); an
               arrow turning about its centre and breathing, and a star
               turning about its corner (rotation, scale, origin); three
               overlapping cards whose drawing order turns every two seconds
               (ordering: the last drawn is on top); a ring of squares turned
               as one group (the transform stack); outlines, lines, a
               triangle, a circle and a ring (the shapes). It draws in CSS
               pixels (the batch's initial transform scales by the pixel
               ratio). The atlas has 512 x 512 texels per cell - at least the
               largest size a cell is drawn at on a canvas of pixel ratio 2
               (the arrow at the top of its breathing, about 363 device
               pixels) - with anti-aliased edges (each texel's alpha is the
               share of its 4 x 4 samples inside the shape; straight alpha),
               mipmapped and drawn with linear filtering unless the nearest
               switch is on; the page shows the atlas texture's size and mip
               levels. Buttons: pause/resume, nearest/linear filtering, and
               "Check the shared error handler": a check of the device's ONE
               shared error handler, run only when pressed (so the console
               stays clean until then): two graphics contexts receive the same
               deliberate WebGPU error, a context made afterwards none (Chrome
               also writes that error to the console as a warning). The
               canvas's own context, made before the error, receives it too:
               the layer recognises it by its buffer label and counts it on
               its own line, not as a drawing error. A legend in HTML says
               what the canvas shows.

               LIT SHAPES (src/lit_scene.rs, src/lit_shapes.rs): the
               MeshRenderer drawing the built-in primitives (a turning cube,
               a pillar, a shiny bouncing sphere, a translucent sphere, a
               ring of small cubes drawn instanced with one mirrored) on a
               checkered ground (its texture repeats through the material's
               texture transform), with the layer's own depth texture, under
               a perspective or orthographic Camera driven by the
               OrbitController: drag (or one finger) to orbit - the scene
               follows the pointer -, a right/middle/Shift drag to pan, the
               wheel to zoom (in the orthographic projection too: the
               controller scales the view height with the distance; the
               layer sets the height to 0.9 x the distance once, when the
               projection switches to orthographic), two fingers to pan and
               pinch-zoom, within pitch 0.05..1.35 and distance 3..30; it
               auto-orbits until the first drag. Buttons: pause/resume, the
               projection, orbit by itself again. The controls' legend is
               HTML over the canvas; the camera's values are shown under it.

               SKINNED MODEL and MORPH TARGETS (src/model_scene.rs,
               src/model_layer.rs): the two validation scenes of skinning and
               morph targets (none of the viewer's models has either). Each
               fetches ONE model - the archer's .glb file and its colour map
               for the glTF loader (src/model_files.rs), the morph shape's
               model data blob (models::model_data_bytes; it is built in code
               and has no file) - and draws one instance of it with the
               ModelRenderer, lit by the context's default environment, its
               opaque parts, the skybox, then its blended parts, under an
               orbit camera framed on the model's bounds
               (OrbitController::frame_box; auto-orbit until the first
               drag). Its player follows the frame's TIME
               (AnimationPlayer::update_to), so pause, speed and seeks of the
               clock ("Clock -1 s", "Clock +1 s", "Half speed") move the clip
               to the matching place. SKINNED MODEL: the Kenney archer
               (character-archer, Kenney Mini Forest, CC0) playing "walk"; a
               list chooses any of its clips (the player cross-fades to it in
               0.3 s); the read-out is its left arm's world position. MORPH
               TARGETS: a sphere the backend builds in code
               (GraphicsDemo.Assets, BuiltModels/MorphShapeModel.cs; no asset
               file) with three morph targets - stretch, twist, bumps - and
               the clip "morph", which raises each weight to 1 and back in
               turn over six seconds; the read-out is the three weights. Both
               show the clip and its playhead, the skins, joints, morph
               targets, triangles, draw calls and pipelines, and every error.

               MORE (src/more_scene.rs, src/more_layer.rs, src/more_*.wgsl):
               the graphics crate's remaining features, each visible: an HDR
               equirectangular panorama MADE IN CODE (a dusk sky, twelve
               coloured posts on the horizon, a bright sun) baked into an
               environment and drawn as the background by the
               SkyboxRenderer; a ring of twelve cubes recorded ONCE into a
               RENDER BUNDLE and replayed every frame; 4096 particles on a
               turning double helix, moved every frame by a COMPUTE PASS,
               which also writes the arguments of their INDIRECT DRAW (the
               number drawn rises and falls with time, decided on the GPU);
               two ANIMATED TEXTURES at the top right (the same twelve frames
               made in code - a growing wedge - one as a texture array with
               GPU mipmaps, looping; one streaming through a ring of layers,
               ping-pong), drawn by the SpriteBatch; an orbit camera that
               turns by itself (drag, wheel). A line of QUERY RESULTS in
               HTML, read twice a second (codebrix_webplatform's sleep_ms in
               a spawned task): the GPU timer's render-pass and
               compute-pass milliseconds (or "not measured" without
               timestamp-query) and the instance count the compute pass
               wrote; and what the crate answers when asked for an occlusion
               query set (it refuses them in this version - the message says
               why). One-off checks: a headless target cleared and read
               back (colour and depth), a texture region written, copied and
               read back, a buffer mapped at creation and read back, a
               compute pipeline and an empty render bundle with WebGPU's
               verdict awaited. Pausing the clock freezes everything.
               ON THE PAGE (src/page_scene.rs; also ?scene=on-the-page):
               DRAWING ON THE PAGE - the page's own gradient (Tailwind
               classes), a checker pattern and HTML text, with a FRAMELESS,
               TRANSPARENT canvas (`transparent: true`, no border, ring or
               rounded corners) over them; four of the viewer's models (those
               that stand up - judged by each model file's bounds, built
               without textures first - each scaled to one size; the same
               model files through the glTF loader) turn in a row, drawn
               with HDR ON (an RGBA16Float target cleared to transparent, the
               ToneMapper with preserve_alpha) and no skybox, lit by the
               context's default environment. Buttons: pause the clock, "Hide
               the canvas" (a class: the page alone at the same place),
               "Change the page's background" (where the models are, nothing
               changes; around them the new page shows).
               THE FEATURE DEMOS (src/demos/; the second row of buttons,
               "Feature demos"; each also ?scene=<name>): one scene each
               for the graphics crate's newer capabilities, on ONE shell
               (src/demos/mod.rs): the kits' model files and the ground's
               blob fetched once for the page (src/demos/kit.rs; a demo that
               changes a model's materials builds its own copy and changes
               it while it holds the only handle), the HDR path (an
               RGBA16Float target, Khronos PBR Neutral tone mapping), an
               orbit camera framed with frame_box, the scene's own clock
               (pause, speed 1 / 0.5 / 0.25) - a demo may instead ask for 4
               samples per pixel (Demo::sample_count: the HDR target's
               set_sample_count), for the DIRECT path (Demo::direct_to_canvas:
               no HDR target or tone mapping, the scene drawn straight into
               the canvas through the layer's CanvasFrameTarget over the
               layers below) and may place the camera itself
               (Demo::place_camera); a demo kind may have a SECOND CANVAS of
               the same scene on the same clock, its camera following the
               first's, some settings held (DemoKind::comparison; ids
               <name>-compare-...), and a 2D layer below its own
               (DemoKind::backdrop); a WATER demo's HDR target has
               Depth32Float depth (DemoKind::depth_format: the water
               renderer's scene capture reads only that) and it draws a
               SECOND scene pass (Demo::has_water_pass / water_pass: after
               the pass with the opaque parts and the sky it captures the
               target and begins a pass that LOADS colour and depth for the
               water and the model renderer's end_blended) -, its CONTROLS (declared as a
               list: toggles, sliders, choices, buttons; ids
               <name>-<control>), its READ-OUTS (ids <name>-<read-out>), a
               LEGEND over the canvas saying what to look at, and the FRAME
               TIMINGS panel (src/demos/timings.rs: the crate's FrameTimer -
               CPU ms, GPU ms of the "scene" and "tone map" passes or "not
               available", smoothed, the renderer's counts - refreshed twice
               a second; "Hide the frame timings" hides it):
               - LIGHTS (lights): point lights and a spotlight at dusk - up
                 to 40 coloured lamps over two rows of barrels, a spotlight
                 above the chest, a cyan lamp carried by the walking archer
                 (its arm node's world matrix); the page counts the lights
                 given, those lit and reaching the view, and those shaded
                 (the renderer's own count, frame_stats().local_lights: at
                 most 16, SceneLighting::MAX_LOCAL_LIGHTS);
               - SHADOWS (shadows): the sun's shadow (on / off, map size,
                 softness, strength, sun azimuth and elevation, the covered
                 region - the framed box or the middle only - outlined with
                 glows on request) over the walking archer (skinned), three
                 walking foxes and the opening chest; a very tall tree
                 BEHIND the camera at the start (culled for the camera,
                 still casting); per-instance casts / receives switches;
                 0 to 6 shadow-casting spotlights (4 get shadows); the
                 renderer's bias constants are private and named in the
                 legend;
               - CULLING (culling): 576 trees, rocks and barrels, twelve
                 walking foxes and the archer, the camera inside the field;
                 the renderer's culled / drawn / draw-call counts beside the
                 page's own count of what lies outside the view (they agree);
                 "never cull" on the archer; an OVERVIEW inset (a second
                 renderer, culling off, from above, tone mapped into an
                 8-bit target and put in the corner by a SpriteBatch): red =
                 skipped by the main view, light = drawn, a white glow = the
                 camera;
               - TRANSLUCENCY (translucency): a glass copy (BLEND, 40 %,
                 double-sided) of the barrel, the tall tree or the Avian, the
                 left one drawn by a renderer with two-pass drawing on, the
                 right one by a renderer with it off (each switchable); five
                 foxes of one model: plain, tinted, blue, faded (alpha),
                 glowing (emissive strength); node visibility (the chest's
                 lid, the foxes' tails); emissive x vertex colour is NOT shown
                 (no model here has vertex colours);
               - GLOW AND BLOOM (glow): night; a glowing copy of the Avian
                 (flying, a glow on each wing tip) and of the camp fire,
                 eight pulsing lamps, a big warm glow, up to 2000 fireflies
                 (one draw); bloom on / off, threshold, soft knee, intensity,
                 radius; glow intensity, size, pulse rate and depth;
               - PARTICLES (particles): sparks from a circling cone emitter
                 (additive), a mist (transparent), bursts where the ground is
                 clicked or over the fire (a button); rate, gravity, drag,
                 cone angle, mist, bloom, reset; spawned / simulated time /
                 "alive at most" per system (the system's alive_at_most(),
                 worked out on the CPU from its spawn history: there is no
                 read-back of the live count);
               - PICKING AND LABELS (picking): a click (not a drag) picks the
                 nearest model box under the pointer (canvas_to_ray,
                 nearest_box_hit; the boxes are culling_bounds - every pose
                 of every clip - or the rest pose's bounds, a control) and
                 tints it gold; page-text labels (spans over the canvas,
                 placed with world_to_canvas through CSSOM properties - CSP
                 safe) follow the models and hide when theirs is behind the
                 camera or off the canvas;
               - ANTI-ALIASING (antialiasing): rows of fences receding to
                 120 m, tall thin trees far away, barrels and the walking
                 archer against a bright sky, the camera turning slowly;
                 "Anti-aliasing: Off / 4x" switches WHILE IT RUNS (default 4x)
                 and a SECOND CANVAS beside it draws the same scene with it
                 held Off; "Path": the HDR path (bloom, tone mapping; the
                 HDR target's sample count) or the DIRECT path (the scene
                 straight into the canvas through a CanvasFrameTarget, no
                 skybox, over a 2D layer BELOW it - bands of colour and a
                 disc drawn by a SpriteBatch, src/demos/antialiasing.rs
                 Backdrop - which must stay visible at 4x); the shell's
                 read-outs (ms-samples, ms-path, ms-hdr-sets, ms-canvas-sets:
                 the HDR target's buffer sets as the page counts them -
                 HeadlessRenderTarget has no counter - and the canvas
                 target's buffer_sets_made()) show that switching back makes
                 nothing new;
               - LEVEL OF DETAIL (lod): a 13 x 13 field of groves, each one
                 LodInstance of three levels - a SET of models (three trees,
                 a rock, a fox walking on the spot, animated only while shown:
                 shown_levels_mut) out to 40 m, ONE simple surface standing
                 for the grove (a kit rock stretched to its size) out to
                 110 m, a lower far shape out to 260 m (the cut-off, which a
                 control turns off: set_max_distance(2, f32::INFINITY)) - drawn
                 with select_lod / draw_lod; the camera FLIES in and out along
                 one line from the marked grove (10 m to 300 m and back in
                 48 s, even on a log scale), HOLDS at a slider's distance, or
                 is free; controls: fade band (0 = the hard switch), quality
                 (set_lod_distance_scale), hysteresis, levels tinted (1 green,
                 2 blue), the cut-off, sun shadows (the sun from the side of
                 the camera's line), anti-aliasing Off / 4x, a TEST set (the
                 same model on every level, tinted red / green / blue), the
                 groves drawn (all, the marked one, the marked one on black
                 for counting its pixels), frustum culling; read-outs: groves
                 at each level / fading / culled / beyond the cut-off beside
                 the renderer's fading_lods and culled_lods, the marked
                 grove's distance, state and coverage per level;
               - WATER POND (water-pond): a pool 14 x 8 units whose FLOOR is
                 the repository owner's own pastel glass MOSAIC of an
                 eight-petalled ROSETTE from his palm visualizer sample
                 (CodeBrix.Samples, PalmVisualizer, RippleWebGpuRenderer.cs:
                 his WGSL functions hash2, petal, rosette and tesserae and
                 the grout mix, carried over unchanged into
                 src/demos/pool_mosaic.wgsl - it is his own code, so no
                 third-party notice is needed for it), drawn ONCE into a
                 mipmapped 2800 x 1600 sRGB texture (a full-screen pass, 2 x 2
                 samples per texel, then generate_mipmaps) and laid UNLIT on
                 an even floor 0.6 deep (a control: sloping 0.3 to 1.2, or
                 0.3 to 2) by MeshRenderer, with tiled walls and a deck
                 (MeshRenderer), kit trees and rocks on the deck and an
                 island rock in the water (a WaveMask disc); a WaveField of
                 448 x 256 cells (the palm pool's grid; 240 steps per second,
                 Reflect edge) stirred by the POINTER (a left drag that STARTS
                 on the water: WaveField::stir through
                 WaterSurface::field_point and the camera's ray; a drag that
                 starts off the water orbits; "Left drag: orbits" turns
                 stirring off) and by seeded RAIN; the WaterRenderer draws it
                 (capture_scene, then draw in the second pass, the sun's
                 shadow map from the model renderer) with caustics, depth
                 tint, sky and screen-space reflections, glints and the
                 thin-film SHEEN switched ON (strength 0.4); THE DEFAULTS
                 ARE THE PALM POOL'S LOOK, derived value by value from his
                 RippleWebGpuRenderer.cs and RippleRain.cs (the comment THE
                 PALM POOL'S LOOK in water_pond.rs gives each derivation; 1
                 view height of his = 8 units here): camera straight down,
                 refraction 0.95 over the even 0.6 floor (his 0.07 view
                 heights per unit of slope), rain 1.6 / s with drops of
                 radius 0.064-0.128 and depth 0.0144-0.0304 at wave height 2,
                 wave speed 1.9, damping 0.66, the water "as the palm pool"
                 (absorption 0.64 / 0.49 / 0.315, turbidity 0.14, scattering
                 0.032 / 0.104 / 0.184), caustics 0.45 at scale 0.25 drifting
                 0.08 cycles / s, roughness 0.24 with sun_glint 0.35,
                 reflection strength 2.5, sheen 0.4 (scale 0.125, height gain
                 7.5), fine ripples as his swell (slope 0.045, wavelength
                 2.19, 0.4 units / s), bloom off; controls:
                 the camera (straight down as the palm pool, or tilted),
                 rain, wave speed (key wave-speed - "speed" is the shell's
                 clock button; set_wave_speed's answer is a read-out),
                 damping, the floor, the water preset (absorption: clear
                 pool / library default / murky / as the palm pool), caustics
                 strength and scale, glint sharpness (roughness), reflection strength,
                 sheen, screen-space reflections, fine ripples, "Water
                 effects" (off: no refraction, absorption, caustics,
                 reflection, glints, sheen, ripples or shore fade - the floor
                 exactly as it is), the island, bloom, anti-aliasing Off / 4x,
                 "Calm the water" (WaveField::reset); read-outs: the field's
                 steps this frame / taken / simulated time, the water
                 renderer's last_stats and pipeline count, and the MOSAIC
                 PROBES (probe-heart, -petal-pink / -orange / -violet /
                 -green, -sea-glass, -gold-ring-0..7: the canvas point of
                 each, the palm colour there from a CPU copy of his rosette
                 function, and the colour expected on screen with the water
                 effects off); under the canvas a panel runs the GPU-vs-CPU
                 check (a 64 x 64 probe WaveField and its CpuWaveField, 200
                 steps with a drop and rain, the largest height difference of
                 the picture read back; water-pond-probe);
               - WATER LAKE (water-lake): a 240 x 240 lake of OPEN-WATER WAVES
                 (OpenWaterWaves::from_wind: wind speed, direction, wave count
                 and seed are controls) over a lake bed made in code (a
                 MeshRenderer terrain: deep in the middle, a grassy bank on
                 the near side, two islands, hills round the far edge; NO
                 shadows on it - MeshRenderer has none), kit trees, rocks,
                 foxes and the walking archer on the bank and the islands,
                 fences and two lamps (point lights) along the shore; a
                 barrel FLOATS (WaterSurface::height_at / normal_at every
                 frame, read-out water-lake-barrel); a dog SWIMS in a circle
                 drawing a WAKE (set_wake) into a 128 x 128 WaveField shown on
                 a second WaterSurface over the same capture, its grid on the
                 lake's grid lines with the lake's cell and displace off, 3 mm
                 higher, so the two meet exactly (the pointer stirs it too;
                 "Water: waves only" leaves it out); controls: screen-space
                 reflections, reflection strength, the water preset,
                 caustics, the sun's height, sun shadows, the lamps, bloom,
                 anti-aliasing; read-outs: the water renderer's stats and
                 local lights, the model renderer's local lights and
                 shadowed spotlights, the wake field's steps, and probes of
                 a shallow and a deep point for the depth tint.
               THE CURRENT SCENE'S BUTTON is filled (light fill, dark text,
               white ring) and carries aria-current="true"; exactly one
               button at a time, driven by the same state that chooses the
               scene.

    animation  served below /animation: the scene catalogue of the desktop
               AnimationDemoApp (its scenes ported, then scenes of the
               sample's own for SVG documents and Lottie), a 1280 x 720 canvas (CSS size) drawn by one AnimationLayer on
               the host's clock - EVERY piece of text on it (each scene's
               header with its title, description and rule, as the C# demo's
               AddHeader draws it, and every label and read-out) drawn by the
               crate in fonts the backend serves - and buttons below (pause
               the clock, previous / next scene, start again, the debug
               overlay, one per scene, "Check the sample points"). THE FONTS:
               the page first fetches Roboto and Open Sans ONCE with the
               generated font_file_bytes call (FontsController,
               api/fonts/<file name in lower case>: roboto-regular.ttf and
               opensans-regular.ttf, from GraphicsDemo.Assets; the backend
               serves every font file it carries, src/main.rs FONT_FILES says
               which two the demo asks for),
               registers the bytes with one FontRegistry (Roboto first, the
               default), then fetches the VARIABLE Roboto (roboto.ttf) and
               registers it once per weight of the "text" scene
               (demo/text.rs VARIABLE_WEIGHTS), and only then builds the catalogue; a failed fetch
               shows its message (id fonts-failed) instead of the demo. The catalogue LOOPS; the scene starts are NAMED
               POINTS of the clock (the scene buttons seek to them; they sit
               in their own box, #scene-buttons, which scrolls by itself when
               it overflows; the button of the scene being drawn - the
               layer's own scene index, so it follows the clock, the keys,
               the buttons and ?scene= - is filled and marked
               aria-current="true", and is kept in view inside the box
               without scrolling the page or moving the focus). Keys,
               once the canvas has the focus: Space pauses / resumes the
               clock, Left / Right seek one second, Shift with Left / Right
               moves to the previous / next scene, Home starts again, D shows
               or hides the DEBUG OVERLAY (the crate's DebugOverlay on the
               catalogue's timeline - timecode, frame and the active cues -
               bottom right, as the C# demo's live window shows it). The
               address options ?scene=<name>&t=<seconds>&paused=1 start a
               given scene at a given time; &hold=1 keeps the page on that
               scene. THE SCENE PANEL under the scene buttons
               (#scene-panel, demo/controls.rs, the DemoScene methods
               controls / apply_settings / legend / readouts): "Stay on this
               scene" (#demo-hold: past its end the scene starts again
               instead of the catalogue moving on), a SCENE-TIME SCRUBBER
               (#demo-scrub: seeks the clock inside the scene; with the
               clock paused it shows any moment), a button per named point
               of the scene (ids point-<scene>-<point>), and - for the
               feature scenes of the second GO - a legend (#scene-legend:
               what to look at), the scene's CONTROLS (ids <scene>-<key>; a
               slider's value also in <scene>-<key>-value; a Control::Button
               is a push button whose value counts the presses) and its
               READ-OUTS (ids <scene>-<key>; control and read-out keys
               share the namespace, so they must differ). POINTER INPUT:
               every pointer event on the canvas goes to the scene shown
               (DemoScene::pointer_input, with the map from CSS pixels to
               the frame's units - the canvas's client size through
               CanvasCalibration), and a primary-button press also as a
               point of the frame (DemoScene::pointer_down; the water
               scenes drop a stone there); most scenes ignore both. After
               each frame the page hands the frame's RenderStats to the
               scene (DemoScene::see_stats) and takes its read-outs again.
               A second RenderStats line
               (#demo-stats-2) shows the CPU timings (cpu_ms, smoothed, and
               record / build / encode) and the upload and stroke counters
               (stroke_draws, strokes, stroke / tip uploads, image_uploads,
               color_table_uploads with their bytes); a third
               (#demo-stats-3) the stroke-set checkpoint counters
               (strokes drawn, strokes_restored, checkpoint_hits / _bakes /
               _misses, checkpoints held and their MB), the water's
               ripples_evaluated / ripples_skipped, and the interval
               between frames (frame_ms_smoothed, the page's clock read for
               this read-out only). ?scene=on-the-page is not a scene
               of the catalogue but its own page (src/demo/on_page.rs, the
               nav link "On the page"): DRAWING ON THE PAGE - the page's own
               gradient, checker pattern and HTML text under a FRAMELESS,
               TRANSPARENT canvas; a session drawn WITHOUT its background
               (the layer default) floats over it: translucent discs and a
               card with text gliding, a half-transparent blue square and an
               opaque grey disc standing still (fixed places for a test
               tool); "Hide the canvas" shows the page alone. The scenes:
                 easing   every EasingKind, three Steps and the four CSS
                          curves, each a graph with a dot running along it
                          and a dot sliding at the eased value (the C#
                          demo's EasingScene);
                 ramps    four colour ramps, each as sampled bars above a
                          gradient fill of the same stops, with a marker that
                          sweeps back and forth and the colour it reads (the
                          C# demo's ColorRampScene);
                 gradients  a rounded rectangle whose linear gradient turns,
                          the three spreads with a sliding gradient, a
                          radial glow with a circling highlight in a pulsing
                          halo, a spotlit ellipse, a turning star and the
                          word GRADIENT painted by a sliding gradient (the C#
                          demo's GradientScene);
                 text     "Animate" left / centre / right about one anchor
                          line with tracking easing in and out, each with its
                          measured width as a bracket and its bounds as a box,
                          and a row of sizes placed by the measured widths
                          (the C# demo's TextScene); at the bottom (not in
                          C#) the VARIABLE Roboto at five weights, each
                          registered by the page with
                          register_font_with_variations under its own family
                          name, and its axes as get_variation_axes reports
                          them;
                 fade     one composition faded in, held and faded out by
                          the session's opacity, drawn twice: over its black
                          background (a fade from and to black) and without
                          it over a checkerboard (the fade is transparency),
                          its caption layer blinking by its own opacity (the
                          C# demo's FadeScene; the C# scene exports pixels,
                          this one draws the composition into its two boxes
                          with SessionRenderOptions);
                 timeline  a timeline's four cues and four markers as a
                          ruler with a playhead, each cue's linear and eased
                          progress, a puck (a group) the cues move, spin and
                          fade, markers ringed in their frame, the timecode
                          (the C# demo's TimelineScene);
                 keyframes  balls on eight lanes bound to point tracks -
                          five easings, hold keys, Loop, PingPong - a star
                          badge whose rotation, scale and opacity are float
                          tracks about its anchor, swatches whose group colour
                          is a colour track (KeyframeScene);
                 transforms  element transforms bound to tracks with their
                          transformed bounds outlined, a layer transform, and
                          faded discs one by one beside a group faded as one,
                          over a checkerboard (TransformScene);
                 time-keys  480 threads with time windows, added once; the
                          pen and the read-out are animated shapes
                          (TimeKeyScene);
                 layout   one scene description in a portrait and a
                          landscape frame, placed by anchors of a safe-area
                          guide's safe rectangle (DrawingLayout::from_guide),
                          each frame showing its guide's overlay, as in C#,
                          and a marker touring the nine anchors (LayoutScene);
                 camera   a night landscape under a camera whose pan, zoom
                          and roll are tracks, each layer by its parallax
                          (HUD and header fixed) (CameraScene; its stars
                          from the crate's DrawingRandom with the C# scene's
                          seed, where the C# scene puts them);
                 playback  a playback controller replaying a script of
                          calls up to the scene's time, and the clip it plays
                          with a debug overlay, the state read out
                          (PlaybackScene);
                 draw-on  a signature written by its trim end with a nib
                          following it along the path, an underline, a
                          loading ring (trim start, end, offset), a piece
                          running round a star, marching ants (DrawOnScene;
                          the nib glows - DrawingEffects::glow);
                 motion-path  a ball and a facing dart along a figure eight,
                          a car (a group) round a rounded track, the dart's
                          tangent as an arrow with distance and heading
                          (MotionPathScene; the dart casts a drop shadow);
                 morph    circle -> star -> heart -> rounded square ->
                          circle, each step a morph with a gradient fill in
                          its own time window, and a countdown 3 2 1 0 from
                          the font's glyph outlines with the in-between's
                          resampled points as dots (MorphScene);
                 sprites  a bouncing-ball sheet and a pixel-art flame sheet
                          GENERATED in code (pictures.rs), played by time as
                          a loop, a ping-pong, a hold that starts at 2 s and
                          by a bound FrameIndex; the flame nearest, bilinear
                          and tinted; the loop's cell outlined on the sheet
                          (SpriteScene);
                 images   a landscape (painted in code) turning, scaling and
                          fading; a zoom and pan by a moving source
                          rectangle; a tint bound to a colour track; a pixel
                          heart nearest and bilinear; THE PICTURE THE BACKEND
                          DECODED (a quarter of the Cube Pets colour map,
                          nearest) beside THE SAME PICTURE AS A PNG FILE
                          and AS A JPEG FILE, both decoded by the crate
                          (ImageSource::from_png, ImageSource::from_jpeg); emblems over a checkerboard in a turning
                          group kept to its panel by a mask (ImageScene);
                 clip-mask  stripes inside a turning star (a group's mask), a
                          card with a keyhole cut out (a layer mask outside a
                          path, with a drop shadow), a spotlight (a layer mask
                          replaced every frame), a matte (stripes shown where a
                          hidden layer's text is) and a dissolve (C# uses a
                          noise PLANE as the mask; here the thresholded noise
                          is a picture on a hidden matte layer)
                          (ClipMaskScene);
                 effects  a lifting card whose shadow offset, softness and
                          colour are bound to tracks; a star whose glow
                          radius and colour are bound; a focus pull (two blur
                          radii bound to opposite tracks); one shadow for a
                          whole layer (its offset bound); the same effects
                          over a checkerboard (EffectsScene);
                 effects-follow  a tabletop under a zooming, rolling camera:
                          two spinning cards with a shadow fixed in the scene
                          and one turning with the card, a pulsing star whose
                          glow and shadow scale with it (EffectsFollowScene);
                 path-combine  a circle sliding across a star and the four
                          boolean results, a stroke turned into an outline
                          while it draws on, a star inflated and deflated,
                          outlined text with and without merged overlaps
                          (PathCombineScene);
                 kerning  the same words with the font's kerning (the
                          default) and with kerning off, each with its
                          measured width as a bracket, then a tracked line
                          whose tracking eases in and out - one shaped run, so
                          it stays kerned - above the same line unkerned
                          (KerningScene);
                 paragraph  one text wrapped and aligned four ways with its
                          line metrics, and a breathing box that re-wraps,
                          trims to two lines with an ellipsis and centres
                          vertically (ParagraphScene);
                 typewriter  a reveal by characters with a caret, a soft
                          reveal by words, word and line pieces staggered in
                          (TypewriterScene);
                 outlined-text  an outside outline, a gradient with an
                          outline, glyph outlines drawn on then filled, text
                          along a wave (OutlinedTextScene);
                 captions  SRT cues FROM THE BACKEND (CaptionsController)
                          burned into a "video" below the header, placed on
                          the broadcast title-safe area as the C# scene does -
                          the crate's OPT-IN; its default is the frame
                          (CaptionScene);
                 formation-spiral / formation-heap  dots spiralling in to
                          form a word; leaves falling and heaping up in a
                          heart and an arch (the Formation scenes);
                 rolling-text  a word's letters rolling in as a train and
                          settling in place, a wheel rolling over hills
                          (RollingTextScene);
                 additive  rotation, scale and position bindings that add
                          and multiply, with plots and read-outs
                          (AdditiveScene);
                 path-queries  inside / outside probes with their edge
                          distance and nearest point over a signed-distance
                          field, the gap between a circle and a rounded
                          hexagon (PathGeometry::rounded_polygon)
                          (PathQueriesScene);
                 layer-visibility  layers switched by bool tracks, a hidden
                          matte still masking (LayerVisibilityScene);
                 procedural  a sky, a sun and rippling water from one formula
                          per pixel (a CPU plane image at half the panel's
                          size, stretched; tone-mapped) (ProceduralScene);
                 light    one night scene four ways, each a session of its
                          own: as drawn, blurred (an effect), with a bloom,
                          and with bloom, vignette and tone curve - the
                          planes' whole-frame operations as GPU passes over a
                          layer (LightScene);
                 noise    value, gradient, CAUSTICS (the third kind, Not in
                          C#: a drifting net of bright lines, the same
                          pattern on the CPU and the GPU; its frequency,
                          z speed and octaves are controls) and fractal noise
                          evolving with time, DRAWN ON THE GPU (NoiseShape -
                          not C#'s exact pattern for value and gradient),
                          four panels 288 wide (the moved gradient and
                          fractal panels shift their noise back by the move,
                          so they show the pattern they showed when there
                          were three), the seeded DrawingRandom dots (exact)
                          and a small exact CPU noise fill stretched
                          (NoiseScene);
                 transitions  six transitions between two opaque cards at
                          once, each panel a session of its own with
                          set_layer_transition (on the GPU) (TransitionScene);
                 particles  a fountain, firework bursts and snow, each
                          system ONE instanced draw (ParticleScene);
                 motion-blur  a swinging ball and a spinning bar, sharp and
                          as the average of 16 sub-frames (the exact sample
                          times; the averaging by the renderer: each sample a
                          group composited Plus at 1/16, kept to its panel)
                          (MotionBlurScene);
                 hard-edges  a tiny drawing as the renderer draws it, its
                          hard-edged mask, and that mask used as a plane mask
                          (HardEdgeScene; C#'s comparison with the
                          anti-aliased readback mask is left out: the crate
                          computes only the HardEdged meaning);
                 ownership  forty drifting brushstrokes and three circles,
                          the ownership map made every frame on the CPU and
                          COLOURED ON THE GPU (OwnershipShape), with and
                          without fill_unowned and seams (OwnershipScene);
                 colour-pickup  a sunset painted per pixel and its
                          brushstroke mosaic (mean colours per stroke on the
                          CPU, the mosaic coloured on the GPU)
                          (ColorPickupScene);
                 brush-texture  eight bending strokes textured in their own
                          axes: flat, ridges, ramp, seams - CPU at the C#
                          demo's small size (BrushTextureScene);
                 brush-strokes  a stroke layout painting itself on with
                          BrushStrokeShape's progress (not a C# scene);
                 keyframe-playback  a painting per second, painted AHEAD
                          one keyframe per frame (prefetch) and dissolved ON
                          THE GPU between cached keyframes, with the two
                          keyframes, the strip and the cache read-out
                          (KeyframePlaybackScene);
                 safe-areas  a portrait post and a landscape frame cycling
                          through the guide presets with their overlay and the
                          check's findings (SafeAreaScene);
                 shapes   the renderer's feature sheet (not a C# scene):
                          fill rules, the session's shapes, strokes with
                          caps, joins, dashes and widths, linear, radial,
                          two-point radial and sweep gradients with pad /
                          repeat / reflect, every mix and compose mode,
                          clips, a translucent group, an image paint and an
                          image draw. It draws no backdrop, so its sample
                          points (read through the crate's testing readback
                          on "Check the sample points") tell drawn from
                          empty;
                 missing-characters  (not a C# scene: the owner's rule for
                          characters a font lacks) strings holding a
                          character neither font has, in the three
                          MissingCharacterMode choices side by side, with
                          Roboto (its .notdef is a box) and Open Sans (its
                          .notdef has no ink), the report under each line, a
                          marker running under it; no backdrop, and its
                          sample points check ink (or none) over each line's
                          missing cluster;
                 svg-parts  (C#'s SvgPartsScene) the gear train and the clock
                          as SVG elements, their documents FROM THE BACKEND
                          (SvgDocumentsController): gears turning at their
                          ratios by part id, the arrow fading in and drawing
                          on, the clock sliding in and growing, its hands
                          turning about its centre, its ticks drawing on, its
                          "PER-PART" label SVG text in the backend's Roboto;
                 svg-smil  (not a C# scene) a document that animates itself
                          with SMIL - a car along a road (animateMotion), lights
                          handing over by syncbases, a windmill, a ball on key
                          splines, a group fill the white square does not take,
                          a pulsing ring, a line drawing itself; its notes and
                          the document's report beside it;
                 svg-css  (not a C# scene) the same with CSS keyframes - a
                          spinner, a loading bar, hopping dots, a heart in
                          steps, a group fill the .keep square does not take, a
                          trace, a blinking caret, a pendulum;
                 svg-features  (not a C# scene) one document with the SVG
                          features the crate draws - a two-circle clip (their
                          union), alpha and luminance masks, multiply and
                          screen, a focal gradient, the blur, dashes and
                          markers, <use> instances, SVG text in Roboto and Open
                          Sans - a frame naming each in turn, the report below;
                 lottie-files  (no C# scene - NoSkia has no Lottie)
                          the renderer trial's three INVENTED Lottie files
                          FROM THE BACKEND (LottieController) played as
                          elements of a session: "many shapes" looping,
                          "painterly" ping-ponging, "busy scene" at 1.5x
                          speed, each with its size, length, draw count and a
                          live frame read-out (LottieFilesScene);
                 lottie-features  the sample's own hand-written Lottie file -
                          a null layer turning an arm that carries a star
                          (parenting), a ring drawn on by a trim path, a
                          gradient seen through a moving disc (an alpha
                          matte), three markers - played whole and looping,
                          and beside it only its "wave" marker's span
                          (LottieRange::Marker), ping-pong; the markers are
                          NAMED POINTS on the host's clock
                          ("lottie-features/build" ..., defined by the page
                          from DemoScene::time_points, which the scene fills
                          from DrawingLottieShape::marker_times) and the row
                          under the animation lights the marker the playhead
                          is in; the file's report() is shown
                          (LottieFeaturesScene);
                 lottie-images  the sample's own Lottie file with one IMAGE
                          LAYER (asset "badge", moving, turning and growing),
                          three times: fed the badge the backend MAKES IN
                          CODE and sends in the picture-pixels form
                          (PicturesController, api/pictures/lottie-badge.rgba;
                          ImageSource::from_picture_pixels), fed the Cube
                          Pets PNG file decoded in the browser
                          (ImageSource::from_png), and fed nothing - the
                          image layer then draws nothing and the document's
                          report says so (ImageNeedsPixels); every picture is
                          handed over with DrawingLottieShape::set_image
                          (LottieImagesScene).
               THE FEATURE SCENES OF THE SECOND GO (not in C#; each with
               controls, a legend and read-outs in the scene panel; the
               painterly ones paint the iris-field picture of
               demo/painted.rs, MADE IN CODE, with brush tips made in code):
                 fixed-width  the same drawing scaled by a bound track (and
                          squashed), strokes that scale with the element
                          above fixed-width strokes (StrokeWidthMode::Fixed),
                          a text block's outline, a measured square, and an
                          SVG line with vector-effect="non-scaling-stroke";
                          controls: largest scale, swing, a still scale,
                          the camera zoom (FixedWidthScene);
                 session-blend  a paper texture with ink drawn over a
                          picture in each SessionBlendMode, at an opacity
                          (SessionBlendScene);
                 paint-override  ONE rich session (gradients, a picture,
                          text, an SVG, a glow, a blur, a masked group, a
                          "Logo" layer) as authored, as a line drawing and
                          flat, side by side; effects and the authored Logo
                          layer are switches (PaintOverrideScene);
                 wipe     SessionWipe: one session from line to authored,
                          day to night, the night MULTIPLIED over the day
                          (render_sessions, the day drawn underneath);
                          straight edge at an angle or iris, softness,
                          progress or animated, and either render alone
                          for comparison (WipeScene);
                 sketch   THE GRID of the sketch helper's settings
                          beside its defaults: a house sketched at 4 x 4
                          settings (roughness x wobble length, or
                          roughness x passes), each cell labelled with its
                          values, SketchOptions::default() large on the
                          right; passes, corner angle, seed and the boil
                          rate are controls (SketchScene);
                 ownership-colours  a map made once, re-coloured every
                          frame with OwnershipShape::with_colors (only the
                          colour table is uploaded), or the old way
                          (OwnershipColoursScene);
                 stroke-sets  the iris picture painted on by a GPU stroke
                          set of 2,000 - 100,000 strokes (one draw), or the
                          old way (one BrushStrokeShape per stroke, 2,000);
                          head softness, mirroring, tips, the look
                          (StrokeSetsScene);
                 travelling-strokes  a figure of four moving parts painted
                          by one stroke set anchored to them (hem strokes
                          blended between two anchors): regenerated each
                          frame / anchored / anchored + boil, and flow
                          (TravellingStrokesScene);
                 direction-fields  strokes laid by a field from a
                          picture's structure (with a detail map), from
                          noise (with GPU flow) and from guides (around,
                          along, radial, fixed); ticks and the detail map
                          as overlays (DirectionFieldsScene);
                 painting-timeline  the iris picture painted in four
                          stages over 62 s (wash, block-in centre out,
                          detail dark to light, highlights light to dark);
                          the stage starts are named points (buttons) and
                          the scrubber drags back to any moment
                          (PaintingTimelineScene);
                 colour-rules  strokes picking up a landscape as picked,
                          through a six-pigment palette, with broken colour
                          and both; blue -> yellow bars bound to colours as
                          functions of the time in sRGB / Oklab / Oklch,
                          gradients and ColorRamp strips in the same spaces
                          (ColourRulesScene);
                 live-pickup  20,000 strokes (5,000 / 40,000) whose colours
                          are PICKED EVERY FRAME on the GPU
                          (BrushStrokesShape::with_color, StrokePickup over
                          the whole painting area) from a live source: the
                          iris picture (StrokeColorSource::Picture), a
                          session MOVING UNDERNEATH - a sun crossing the sky,
                          a cloud, a winding river, swaying stems - drawn
                          into a 640 x 360 PictureTarget by a picture layer
                          (AnimationLayer::to_picture; Target), or a 3D SCENE
                          drawn by the GRAPHICS crate (the archer walking
                          among the kits' trees, the camera circling, the
                          HDR path tone mapped into a 640 x 360
                          Rgba8UnormSrgb texture handed over as an
                          ExternalPicture with PictureEncoding::Linear;
                          Texture); "the source itself" shows what the
                          strokes read; taps, own colour weight, the colour
                          rules (six pigments, Nearest / MixOfTwo, snap,
                          broken hue and chroma) and the count are controls.
                          THE LAYERS: the picture layer and the 3D layer are
                          BELOW the catalogue's layer in CanvasLayers (the
                          strokes read THIS frame's pictures), a small layer
                          ABOVE it shows the 3D picture itself (an image
                          shape can show a picture target, not an
                          application's texture); all three draw only while
                          the scene asks (flags it sets every frame), so no
                          other scene pays for them (LivePickupScene,
                          demo/live_pickup.rs);
                 surface-finish  the iris picture painted by 10,000 /
                          30,000 strokes whose bristle tips CARRY HEIGHT
                          (painted::bristle_tips_with_height) on a layer
                          with a SurfaceFinish (set_surface_finish), and
                          beside it a flat rectangle and the picture as an
                          image on a finished layer with NO strokes (weave
                          only); finish on / off, a preset (the sliders,
                          thick impasto, smooth glaze), the light (circling
                          on the clock, or its direction) and its height,
                          relief, impasto, gloss, the weave and its spacing,
                          depth and irregularity (SurfaceFinishScene);
                 checkpoints  THE CHECKPOINT CACHE OF A STILL PAINTING: the
                          stroke-sets scene's records (10,000 / 30,000 /
                          60,000 strokes with bristle tips) painting on over
                          about 20 s and then still, drawn by ONE
                          BrushStrokesShape with_checkpoints(Some(
                          StrokeCheckpoints { spacing, capacity })) or None
                          (controls: on / off, spacing 500 / 2,000 / 5,000,
                          capacity 1 / 4 / 8, "move the painting" - a
                          drifting target rectangle, which switches the cache
                          off by itself); the shape is rebuilt only when a
                          setting changes; the frame and its labels are a
                          still session drawn FIRST, the text that changes
                          every frame a session drawn ABOVE the painting
                          (the cache's rule: nothing that changes below a
                          cached painting); the page's demo-stats-3 line
                          shows strokes drawn, strokes_restored, checkpoint
                          hits / bakes / misses, checkpoints held and their
                          MB, and frame_ms_smoothed (CheckpointsScene);
                 stroke-transition  a stroke-level transition between two
                          paintings made in code from two pictures - the
                          iris field and a shore at dusk
                          (painted::dusk_picture) - 8,000 and 7,000 strokes
                          (or 8,000 / 8,000, or 8,000 / 2,500), paired by
                          BrushStrokeMorph::pair and drawn by
                          BrushStrokesShape::morph; the fraction on the
                          clock (MorphFraction bound to a track: 0 at 1 s, 1
                          at 7 s, back to 0 at 15 s) or on a slider; spread
                          0 / 0.4 / 0.8 with a left-to-right, top-to-bottom
                          or centre-out sweep, search radius, colour weight,
                          tips; "Show" draws either painting alone or a
                          cross-dissolve (session opacity) for comparison;
                          read-outs: strokes, slots, paired, unpaired on each
                          side (StrokeTransitionScene);
                 water-ripples  a pool floor made in code (tiles, a lane
                          line, a ring of discs, a small petal rosette;
                          water_ripples::add_pool_floor) on a layer with a
                          WaterRefraction whose source is a RippleField over
                          the pool: three seeded opening drops, seeded rain,
                          a gentle swell, walls, and the STONES - a CLICK on
                          the water (the page hands primary-button presses
                          on the canvas to the scene shown:
                          DemoScene::pointer_down, in frame units) or "Drop
                          a stone at x / y" drops one at the scene time of
                          the click, so scrubbing back before it removes its
                          ring and forward brings the same ring back;
                          controls: water on / off, strength, shading, light,
                          glints (amount, sharpness), caustic light
                          (strength, frequency, focus), rain, swell, walls,
                          ring speed / decay / wavelength / height (1.5 here,
                          library 1), stone x / y, drop, clear; a leaf floats
                          on top (not refracted), carried by the field's CPU
                          twin (height_at / slope_at); read-outs: the exact
                          scene time, live drops, the ripples the renderer
                          evaluated / skipped in the last frame
                          (RenderStats, handed to the scene by
                          DemoScene::see_stats), the stones, the leaf's water
                          (WaterRipplesScene);
                 water-reflection  a bank at dusk made in code
                          (painted::dusk_bank: sky, low sun, hills, trees,
                          houses with lit windows) above a waterline at 404,
                          mirrored below it by a WaterReflection, and on the
                          SAME layer a pebbly bed (made in code, its alpha
                          growing towards the viewer) bent by a
                          WaterRefraction - one effect group; a plain "Deep"
                          layer of dark water under it; controls: reflection
                          on / off, waterline, fade distance (200 here),
                          break-up (water strength), the reflection's water
                          (the refraction's ripples - evaluated once, the
                          read-out says so -, ripples and a swell, a still
                          height picture made in code, none: a still mirror),
                          opacity, streak, tint amount, gap, the refraction
                          and its strength, the bed, rain; a click on the
                          water drops a stone (WaterReflectionScene);
                 painted-pond  THE PIECE THAT BRINGS THE ROUND TOGETHER: a
                          pond painted in strokes - three stroke sets with
                          bristle tips (bank about 6,000, water about 9,000,
                          floating parts about 4,400; "dense" about
                          twice as many) laid out by StrokeLayout::fill along
                          direction fields (the bank: the picture's structure
                          blended with a level field where it is weak, small
                          strokes where the detail map is high; the water:
                          DirectionGuide::Fixed at 0 degrees; the pads round
                          their centres, the irises upright), colours picked
                          from a picture made in code (painted::pond_pictures:
                          a sky, a far bank of trees with a willow and a red
                          boathouse, still water, lily pads with flowers,
                          irises on the near bank - and its floating parts
                          alone); ONE RippleField over the water (gentle
                          rain, a small swell, no walls; clicks drop stones)
                          is the source of the Water layer's WaterRefraction
                          (shading 0.8 with the light at 60 degrees and
                          glints, so the rings read on the low-contrast
                          painted water) and of the Bank layer's
                          WaterReflection (the bank's brushwork mirrored -
                          no second set); the floating parts lie on top; the
                          painting paints itself on in the first seven
                          seconds, the strokes boil (10 / s); controls:
                          water, rain, swell, refraction, water light, ring
                          height, reflection opacity and break-up, glints,
                          boil, the surface finish (off by default), paint
                          on, density (PaintedPondScene);
                 simulated-pond-2d  the graphics crate's WaveField (a
                          simulation; 304 x 140 cells over the pool of
                          water-ripples, walls that reflect, seeded rain)
                          handed to the animation crate's refraction as an
                          application texture (WaterSource::Texture over
                          ExternalPicture { field.surface().view(), .. }); a
                          canvas layer BELOW the catalogue's steps it with the
                          clock's delta, only while the scene is shown, so the
                          refraction reads this frame's water; every pointer
                          event on the canvas goes to the scene
                          (DemoScene::pointer_input, with the map to frame
                          units) and on to WaveField::stir (a press drops, a
                          drag draws a wake); THE PAGE SAYS PLAINLY THAT IT
                          CANNOT BE REWOUND (title, legend and a note on the
                          canvas): pausing stops it, the scrubber does not
                          take it back (the floor's caustic light, a function
                          of the scene time, does follow the scrubber)
                          (SimulatedPondScene).
               The C# demo's scenes that are NOT ported, and why:
                 masks    it shows the Coverage and VisibleAlpha mask
                          exports and the float plane export of a drawing:
                          here reading a drawing back from the GPU is a helper
                          kept for testing, not a feature for applications
                          (the crate computes the HardEdged meaning, which the
                          hard-edges scene shows);
                 sequence  it renders a clip ahead of time on several
                          threads (FrameSequenceRenderer), and
                 frame-cache  it renders frames ahead of the play head on
                          background workers (FrameCache): offline and
                          worker-thread rendering is out of scope by the
                          owner's decision - in the browser a frame is drawn
                          on the GPU at the host clock's time.
               The ported scenes play in the C# demo's order (the list above
               is grouped by area, not in playing order), except svg-parts,
               which plays with the sample's own SVG scenes. The sample's own
               scenes are added: brush-strokes after brush-texture; shapes,
               missing-characters, the SVG scenes (svg-parts first), the
               Lottie scenes and, last, the feature scenes of the second GO
               (demo/mod.rs, REGISTRATIONS).

WHEN THE BROWSER HAS NO WEBGPU the sample shows the three ways a page can
behave (codebrix-webgpu's WebGpuBoundary and its message component):
  - /graphics: its OWN fallback panel (id graphics-fallback: what the ten
    scenes would show, and the way back to "/"). The scenes are inside a
    WebGpuBoundary with a fallback; the page's one request adds the optional
    feature timestamp-query through set_page_gpu_options (main.rs, App).
  - /animation: the crate's standard message, "WebGPU is not available in
    this browser" with one sentence of what to do (WebGpuUnavailable).
  - /animation/?scene=on-the-page: NOTHING but the page itself - its
    gradient, checker pattern and text; the canvas and its controls are in
    two WebGpuBoundarys without a fallback (they share one request), and the
    fonts are never fetched.

At the site root, "/" is the developer's OWN page,
backend/GraphicsDemo.Api/wwwroot/index.html (plain HTML with site.css beside
it and its own icon link to /favicon.ico; no script): with every application
named, nothing of the platform's is at "/" (see SEVERAL FRONTEND
APPLICATIONS in the platform's AGENT-README.txt). The frontend build never
writes into wwwroot/: the built applications are in frontend-built/.

THE codebrix-webgpu CRATE. Both applications name it in their Cargo.toml
(`codebrix-webgpu = "1.0"`) and take it, like wgpu, from the vendor folder
the build program writes; neither names wgpu, web-sys or wasm-bindgen
itself (the crate re-exports wgpu). The crate carries no styling: the
applications pass their Tailwind classes into the canvas and the "not
available" message. Its API is in the root AGENT-README.txt (THE
codebrix-webgpu CRATE).


WHAT IS DELIBERATELY MISSING
============================
  - A SMALL API: the backend's controllers serve the models (THE
    MODELS below), the HDR panorama (EnvironmentsController,
    api/environments/small-cathedral.hdr, the file unchanged as
    image/vnd.radiance with the models' caching: "public, no-cache" and a
    strong ETag) and the fonts of the animation application
    (FontsController, api/fonts/<file>: every .ttf or .otf file below
    GraphicsDemo.Assets/Fonts/ by its file name in lower case -
    roboto-regular.ttf, roboto.ttf (the variable Roboto) and
    opensans-regular.ttf - unchanged as font/ttf
    (font/otf) with the same caching; FontLibrary of GraphicsDemo.Assets
    finds them as embedded resources, Fonts/<pack>/ with each font's original
    OFL.txt, so a new font is a new file there with no code to change - OFL
    fonts are allowed in the sample only, never in a package) and the PICTURES of the
    animation application (PicturesController,
    api/pictures/cube-pets-colormap.rgba: decoded on the backend by
    PictureLibrary with CodeBrix.Imaging and written by the Animation
    package's PicturePixelsWriter in THE PICTURE-PIXELS FORM - "CBPX", a
    major and a minor version, the width and the height, then width x height
    x 4 bytes of straight-alpha RGBA, rows top to bottom - as
    application/octet-stream with the same caching; the frontend reads it
    with the generated cube_pets_colormap_bytes and hands it to the crate's
    ImageSource::from_picture_pixels. The SAME picture is also served as the
    PNG FILE itself, api/pictures/cube-pets-colormap.png (image/png, the same
    caching; PngFile), which the frontend reads with
    cube_pets_colormap_png_bytes and the crate decodes in the browser
    (ImageSource::from_png), and as a JPEG FILE made from it on the backend,
    api/pictures/cube-pets-colormap.jpg (image/jpeg, quality 90, the same
    caching; JpegFile, PictureDecoder.ToJpeg), which the frontend reads with
    cube_pets_colormap_jpeg_bytes and the crate decodes with
    ImageSource::from_jpeg: the "images" scene shows all three side by side.
    A third picture, the badge of the lottie-images scene's image layer, is
    MADE IN CODE by PictureLibrary and served the same way
    (api/pictures/lottie-badge.rgba, read with lottie_badge_bytes). The
    one picture is the Cube Pets pack's colormap.png, already embedded for the
    models, so no new asset came with it) and the CAPTION FILE of the
    animation application's captions scene (CaptionsController,
    api/captions/caption-scene.srt: SubRip text served unchanged as
    "text/plain; charset=utf-8" with the same caching; CaptionLibrary of
    GraphicsDemo.Assets holds it as the embedded resource
    Captions/caption-scene.srt - the C# demo's own cues, the owner's work, so
    no notice; the frontend reads it with the generated
    caption_scene_srt_bytes and parses it with the crate's CaptionTrack) and
    the SVG DOCUMENTS of the animation application's SVG scenes
    (SvgDocumentsController, api/svg-documents/<name>.svg: gear-train, clock,
    smil-scene, css-scene and features, SVG text served unchanged as
    "image/svg+xml" with the same caching; SvgDocumentLibrary of
    GraphicsDemo.Assets holds them as the embedded resources SvgDocuments/
    <name>.svg - written for the sample, the gear train and the clock
    generated as the C# demo's SvgPartsScene generates them, so no notice; the
    frontend reads them with the generated <name>_svg_bytes calls and loads
    them with the crate's DrawingSvg, their text laid out in the Roboto and
    Open Sans bytes it already fetched) and
    the LOTTIE FILES of the Lottie scenes (LottieController,
    api/lottie/many-shapes.json, painterly.json, busy-scene.json,
    features.json and image-layer.json: Lottie JSON served unchanged as "application/json;
    charset=utf-8" with the same caching; LottieLibrary of GraphicsDemo.Assets
    holds them as the embedded resources Lottie/A_many_shapes.json,
    B_painterly.json, C_busy_scene.json - INVENTED for the renderer trial by a
    throw-away generator, nothing downloaded - and D_features.json and
    E_image_layer.json, written by hand for the sample, so no notice; the frontend reads them with the
    generated many_shapes_json_bytes ... features_json_bytes and hands the
    bytes to the crate's DrawingLottie::load once, when the page loads. The
    animation crate's own Lottie tests read the same four files from this
    folder through CODEBRIX_ANIMATION_TEST_LOTTIE - the crate itself carries
    only small test files); it has no hubs. Models, bitmaps, panoramas,
    fonts, caption files, SVG documents and Lottie files are resources of
    the backend that reach the frontends through API calls;
    none ships inside a frontend application (the sample's rule, not the
    libraries': they take bytes from anywhere).
  - NO SIGN-IN, NO DATABASE, NO CONTAINERS: `run` starts the backend with
    `dotnet run`.
  - ITS NOTICES: THIRD-PARTY-NOTICES.txt beside this file covers the
    third-party model packs, the HDR panorama and the fonts the backend
    carries (the repository's maintainer tool checks that it names every pack
    of Models/, Environments/ and Fonts/, its licence file and its files). The owner's own
    pack, crystals-and-avian (the Avian), has no entry and no licence file:
    his own work needs none, and the tool knows the pack by name
    (SampleNoticesCheck.OwnersOwnPacks). The site serves the
    crates' notices (PlatformCrates.ReadNotices, WebGpuCrates.ReadNotices,
    GraphicsCrates.ReadNotices and AnimationCrates.ReadNotices) as
    THIRD-PARTY-NOTICES.txt and a copy of the sample's own file as
    SAMPLE-THIRD-PARTY-NOTICES.txt beside the built applications in the
    backend project's frontend-built/ (build output: the build program writes
    them after every build with PlatformFrontend.WriteBuiltFile, which gives
    them their .br/.gz copies), served at /THIRD-PARTY-NOTICES.txt and
    /SAMPLE-THIRD-PARTY-NOTICES.txt and linked from the page at "/".
  - ONE TEST PROJECT, for the backend's assets (tests/GraphicsDemo.Assets
    .Tests); everything else the sample proves is checked by building it and
    looking at it in a browser (CHECKING IN A BROWSER below). What the glTF
    loader makes of the model files is checked on the repository's test side
    (the reference converter tests/CodeBrix.WebPlatform.WebGpu.Graphics.Tests
    .GltfReference LINKS this sample's Models/ folder).


THE RULES
=========
The rules of the repository root's maintainer file hold here too (no
Node.js, no JavaScript of ours, no shell scripts, stable releases only, 0
warnings). For this sample:

  - PACKAGES ONLY. The build program takes the WebGpu, Graphics and
    Animation packages from nuget.org (see THE PACKAGES), never a project
    reference into src/: the sample uses them exactly as any
    application does. It calls GraphicsCrates.Register() and
    AnimationCrates.Register(), each of which registers the WebGpu bundle
    first (once).
  - ONLY THE APPLICATION PUTS THE TWO TOGETHER. The graphics application
    names codebrix-webgpu-graphics, the animation application
    codebrix-webgpu-animation; neither crate names the other. ONE EXCEPTION,
    by the owner's ruling (PAINTERLY_DESIGNS ruling O7): the animation
    application also names codebrix-webgpu-graphics for its "live-pickup"
    scene's 3D source (a 3D scene drawn into a texture a stroke set picks its
    colours from) - the application puts the two together; the crates still
    do not name each other.
  - NO TEXTURES, MODELS OR PANORAMAS FROM FILES IN THE FRONTEND: the
    model files and the HDR panorama come from the backend; the textures of
    the quad and sprite scenes are made in code.
  - NO PROCESSING OF MODELS ON THE BACKEND: it serves the model files as
    they are stored (the browser's glTF loader reads them); only the two
    models that have no file (the ground and the morph shape) are built in
    code there and served as model data blobs.
  - THE BACKEND NAMES NO FRONTEND APPLICATION. It serves what the frontend
    build wrote into the backend project's frontend-built/, read from the
    manifest
    (codebrix-frontends.toml) by app.UsePlatformBackend(). The names exist in
    ONE place: the build program.
  - THE BASE PATH IS NOT IN Dioxus.toml. The build program gives dx each
    application's name, so the name is written once.
  - THE CONTENT-SECURITY-POLICY is the platform samples' strict one
    (Program.ContentSecurityPolicy): scripts, styles and connections from
    the site only, and 'wasm-unsafe-eval' for WebAssembly. WebGPU needs
    nothing more. Never loosen it without saying exactly what and why.
  - WebGPU ONLY, through codebrix-webgpu (its wgpu has default-features =
    false and the features std, webgpu and wgsl); no WebGL, no fallback.
  - LAYERS FOLLOW THE HOST'S CONVENTION: the host clears; each layer's
    passes load the target; colours are linear and premultiplied; the
    triangle blends with REPLACE, the overlay with
    PREMULTIPLIED_ALPHA_BLENDING, the quad with BlendPreset::
    PremultipliedAlpha. Layers take their time from the frame.
  - PLATFORM CRATES AND THE REGISTERED BUNDLE ONLY
    (CrateMode.PlatformCratesOnly, offline), and no crate beyond those each
    Cargo.toml names.
  - MOVING BETWEEN APPLICATIONS IS A FULL PAGE LOAD: plain anchors.
  - STYLING IS TAILWIND utility classes in the applications' own Rust (also
    those handed to codebrix-webgpu's components); no inline style
    attributes (the policy forbids them). The page at "/" uses
    its own stylesheet file, site.css.
  - 0 warnings, from the .NET build, `cargo check` and the dx build.


LAYOUT
======
All paths are inside samples/GraphicsDemo.

  GraphicsDemo.slnx             the backend, the models project, its tests and
                                the build program (the root solution does NOT
                                include it)
  MAINTAINER-README.txt         this file
  THIRD-PARTY-NOTICES.txt       the model packs and the HDR panorama the
                                sample carries: source, licence and files of
                                each
  backend/
    GraphicsDemo.Assets/        the models (see THE MODELS below):
                                Models/<pack>/ (embedded, served as files),
                                BuiltModels/ (the ground and the morph shape,
                                built in code), Scene/, ModelLibrary.cs,
                                ModelFile.cs; the
                                panorama: Environments/<pack>/ (embedded),
                                EnvironmentLibrary.cs, EnvironmentFile.cs;
                                the fonts: Fonts/<pack>/ (embedded, each
                                with its OFL.txt), FontLibrary.cs,
                                FontFile.cs; the pictures: PictureLibrary.cs
                                (Internal/PictureDecoder.cs decodes the one
                                picture served as pixels and makes its JPEG
                                file), PngFile.cs, JpegFile.cs
    GraphicsDemo.Api/           the one backend: Program.cs (the two Backend
                                calls, the controllers and the
                                Content-Security-Policy), WireDeclarations.cs
                                (the request marker), Controllers/ and
                                Contracts/ (api/models, api/environments)
      wwwroot/index.html, site.css, favicon.ico
                                the developer's own page at "/" (committed;
                                wwwroot/ holds nothing else)
      frontend-built/           BUILD OUTPUT (it ignores itself: its own
                                .gitignore): graphics/, animation/, the
                                manifest codebrix-frontends.toml,
                                THIRD-PARTY-NOTICES.txt and SAMPLE-THIRD-
                                PARTY-NOTICES.txt, each with .br/.gz copies
  tests/
    GraphicsDemo.Assets.Tests/  the tests of GraphicsDemo.Assets
  build/
    GraphicsDemo.BuildFrontend/ the build program: registers the crate
                                bundles (GraphicsCrates, AnimationCrates), names the two applications
                                (CreateOptions) and calls the platform's
                                FrontendBuilder; prints what the bundle added
                                and shared
  frontends/
    graphics/, animation/       one Dioxus crate each: Cargo.toml, its own
                                Cargo.lock (committed), Dioxus.toml,
                                index.html, tailwind.css,
                                assets/favicon.ico (the repository's icon as
                                an .ico, served without a hash suffix at
                                /<name>/assets/favicon.ico), src/
      src/triangle.rs, src/triangle.wgsl
                                graphics: the triangle layer and its shader
      src/overlay.rs, src/overlay.wgsl
                                graphics: the overlay layer and its shader
      src/shader_scene.rs       graphics: the "Shaders" scene
      src/quad_scene.rs, src/quad.rs
                                graphics: the "Textured quad" scene and its
                                layer (no shader of its own: the library's
                                sprite shader)
      src/sprite_scene.rs, src/sprites.rs
                                graphics: the "Sprites" scene and its layer
      src/lit_scene.rs, src/lit_shapes.rs
                                graphics: the "Lit shapes" scene and its
                                layer
      src/viewer_scene.rs, src/viewer.rs, src/viewer_actors.rs
                                graphics: the "Viewer" scene (the requests,
                                the query switches, the controls, the loading
                                line, the failure message), its layer
                                (lighting, skybox, HDR path, tone mapping)
                                and its animated actors (placement and
                                playback on the clock)
      src/model_files.rs        graphics: the model files - fetching a .glb
                                file and what it names through the generated
                                client, parsing it, building it into a
                                GltfModelSet
      src/model_scene.rs, src/model_layer.rs
                                graphics: the "Skinned model" and "Morph
                                targets" scenes and their one layer
      src/more_scene.rs, src/more_layer.rs, src/more_cubes.wgsl,
      src/more_compute.wgsl, src/more_particles.wgsl
                                graphics: the "More" scene, its layer and
                                its three shaders
      src/page_cache.rs         graphics: what the page keeps of what it
                                fetched
      src/demos/mod.rs          graphics: the feature demos' shell (the
                                demo list, the controls, read-outs, labels
                                and legend, the layer: HDR path or the
                                direct path, 1 or 4 samples, camera, timer,
                                a second canvas, a layer below) and the
                                Demo trait
      src/demos/kit.rs          graphics: the demos' models (fetched once,
                                built per demo; changed copies)
      src/demos/timings.rs      graphics: the frame timings panel
      src/demos/lights.rs, shadows.rs, culling.rs, translucency.rs,
      glow.rs, particles.rs, picking.rs, antialiasing.rs, lod.rs,
      water_pond.rs, water_lake.rs
                                graphics: one feature demo each
                                (antialiasing.rs also has Backdrop, the 2D
                                layer below its canvases; water_pond.rs also
                                has ProbePanel, the GPU-vs-CPU wave field
                                check under its canvas, and the CPU copy of
                                the rosette for the mosaic probes)
      src/demos/pool_mosaic.wgsl
                                graphics: the pond's floor - the owner's
                                mosaic rosette from his palm visualizer
                                sample (his own code, carried over
                                unchanged; drawn once into a texture)
      src/demo/mod.rs           animation: the scene catalogue (the scenes,
                                their lengths, the named points of their
                                starts, the timeline)
      src/demo/page.rs          animation: the page - the canvas, the one
                                AnimationLayer, the clock, the keys, the
                                buttons and read-outs, the scene panel
                                (stay, scrubber, named points, a scene's
                                controls, legend and read-outs), the
                                sample-points check
      src/demo/controls.rs      animation: Control, Settings, Readout - what
                                a scene puts in the scene panel
      src/demo/<scene>.rs       animation: one file per scene, named after
                                it (easing.rs, ramps.rs ... shapes.rs,
                                missing.rs; REGISTRATIONS in mod.rs maps
                                each scene name to its file)
      src/demo/pictures.rs      animation: the pictures - the one the
                                backend decoded (kept for the scenes), and
                                those painted in code (landscape, emblem,
                                pixel heart, ball and flame sheets, the
                                dissolve's noise)
      src/demo/svg_files.rs, svg_parts.rs, svg_smil.rs, svg_css.rs,
      svg_features.rs           animation: the SVG scenes (and the
                                documents read from the backend's files)
      src/demo/lottie_files.rs, lottie_features.rs, lottie_images.rs
                                animation: the Lottie scenes (and the
                                documents read from the backend's files,
                                and the image layer's badge)
      src/demo/kit.rs           animation: short shape and track
                                constructors the scenes share
      src/demo/fixed_width.rs, session_blend.rs, paint_override.rs,
      wipe.rs, sketch.rs, ownership_colours.rs, stroke_sets.rs,
      travelling_strokes.rs, direction_fields.rs, painting_timeline.rs,
      colour_rules.rs, live_pickup.rs, surface_finish.rs, checkpoints.rs,
      stroke_transition.rs
                                animation: the feature scenes of the second
                                GO (paint_override.rs also makes the rich
                                drawing and the line look the wipe uses;
                                live_pickup.rs also has the source layers
                                the page puts below and above the
                                catalogue's layer, and fetches the 3D
                                source's model files; water_ripples.rs also
                                has the pool floor made in code, which
                                simulated_pond.rs shares; simulated_pond.rs
                                also has the layer below the catalogue's
                                that steps its wave field)
      src/demo/painted.rs       animation: what the painterly scenes share -
                                the iris-field picture made in code (and
                                the shore at dusk, the stroke transition's
                                second picture; the bank at dusk of
                                "water-reflection"; the pond and its
                                floating parts, its lily pads and water
                                colour, for "painted-pond"), the colour
                                under a point,
                                brush tips made in
                                code (with or without height), a jittered
                                grid of stroke records
      src/demo/on_page.rs       animation: the ?scene=on-the-page page
      src/page_scene.rs         graphics: the "On the page" scene
      src/generated/            BUILD OUTPUT: the client code generated from
                                the backend
      platform/codebrix-webplatform/
                                BUILD OUTPUT: the shared Rust crate
      platform/vendor/          BUILD OUTPUT, in graphics/ only: the vendor
                                folder with the platform's crates and the
                                bundles' (wgpu, glam, codebrix-webgpu,
                                codebrix-webgpu-graphics, gltf ...), which
                                animation uses from there
      .cargo/config.toml        BUILD OUTPUT: tells cargo to build from that
                                vendor folder, offline
      assets/generated/tailwind.css
                                BUILD OUTPUT: the generated stylesheet (the
                                platform's fixed path; src/main.rs names it
                                in its asset!())
      target/                   BUILD OUTPUT: cargo's and dx's (dx's data
                                folder is target/dx-home)
                                Every BUILD OUTPUT folder above holds its
                                own .gitignore (written by the platform's
                                FrontendBuilder on every materialize and
                                build): nothing the build writes shows in
                                git, with no rule in the root .gitignore.


BUILDING AND RUNNING THE SAMPLE
===============================
What must be installed (the build program checks and tells you; it installs
nothing): the .NET 10 SDK; Rust at the platform's minimum version or later
with the WebAssembly target (`rustup target add wasm32-unknown-unknown`). No
C compiler: dx and wasm-bindgen come prebuilt in the host-tools package, and
the standalone Tailwind program in its Styling package dependency (nothing to
install for Tailwind; one on PATH is never used).

From samples/GraphicsDemo:

    dotnet run --project build/GraphicsDemo.BuildFrontend -- check
    dotnet run --project build/GraphicsDemo.BuildFrontend -- generate
    dotnet run --project build/GraphicsDemo.BuildFrontend -- build
    dotnet run --project build/GraphicsDemo.BuildFrontend -- build graphics
    dotnet run --project build/GraphicsDemo.BuildFrontend -- build animation
    dotnet run --project build/GraphicsDemo.BuildFrontend -- run
        (build and run also with --debug)
    dotnet run --project build/GraphicsDemo.BuildFrontend -- serve graphics
    dotnet run --project build/GraphicsDemo.BuildFrontend -- serve animation

The build program finds the sample's folder above its own folder, or - when
it is built elsewhere (dotnet's --artifacts-path, for example
`dotnet run --project build/GraphicsDemo.BuildFrontend --artifacts-path
<folder> -- build`) - in the current folder or one above it, so run it from
inside samples/GraphicsDemo.

  check     says what the machine has and lacks for a build.
  generate  builds the backend and writes, into each frontend, the generated
            client code, the shared crate, the cargo configuration and a
            missing Tailwind stylesheet; the platform's crates and the wgpu
            bundles' go into the one vendor folder. It prints, for each
            bundle ("webgpu", "webgpu-graphics", "webgpu-animation"), the
            crates it ADDED and the ones it SHARED with the platform. Afterwards plain
            `cargo check --target wasm32-unknown-unknown` works in each
            frontend folder, offline.
  build     generate, then build both with dx into the backend project's
            frontend-built/ (with the manifest), and write the site's two
            notices files there (PlatformFrontend.WriteBuiltFile, after
            every build). With a name (graphics or animation), only that
            application is built and only its folder of frontend-built/ is
            replaced; the other's output stays (the manifest still names
            both).
  run       build, then start the backend at http://localhost:5220/ (the
            page at /, graphics at /graphics, animation at /animation).
            Ctrl+C stops it. The backend alone, on the last build:
            `dotnet run --project backend/GraphicsDemo.Api -c Release
            --no-launch-profile --urls http://localhost:5220`.
  serve     generate, then `dx serve` with hot reload for ONE application,
            named.

The backend alone builds with plain dotnet: `dotnet build GraphicsDemo.slnx`.
Restart the backend after a new build of the frontends: it reads the
manifest once, when it starts.

A FRESH CLONE: the frontends do not compile until `generate` has written
src/generated, platform/, .cargo/ and assets/generated/. So: clone,
`generate`, `cargo check`.

PUBLISHING THE BACKEND: `dotnet publish backend/GraphicsDemo.Api -c Release
-o <folder>` carries frontend-built/ into the published folder (the Backend
package's buildTransitive targets; its .gitignore stays out). Start the
published backend with that folder as the current directory (its content
root): `dotnet GraphicsDemo.Api.dll --urls http://localhost:5220`.


CHECKING IN A BROWSER
=====================
WEBGPU ON A LINUX (DEBIAN-BASED) MACHINE. Chrome on Linux defines navigator.gpu but, with
its default settings, often hands out NO adapter; the pages then show the
"not available" message, which is correct. To see them draw, start Chrome
with the Vulkan switch:

    google-chrome --enable-features=Vulkan http://localhost:5220/

A HEADLESS Chrome needs the GPU switched on as well:

    google-chrome --headless=new --enable-features=Vulkan --enable-gpu
                  --remote-debugging-port=<port>
                  --user-data-dir=<a scratch folder>

(`--enable-unsafe-webgpu` alone gives the SwiftShader software adapter, not
the machine's GPU.)

There are no screenshots in this check. Start the site with `run`, then
drive headless Chrome over its debugging protocol (the Chrome DevTools
Protocol) with a small client kept OUTSIDE the repository (the repository
holds no scripts). Read the page text, the console, the network log, and
pixel probes evaluated through the protocol (that is the test tool's script,
not the site's). Stop the Chrome you started (by the process you started,
never by a name search) and the backend when you are done.

KNOWN SLOW SCENES AT DEVICE PIXEL RATIO 2. Measured in headless Chrome on a
laptop's integrated GPU (120 requestAnimationFrame intervals per scene): on
a 1280 x 720 canvas (pixel ratio 1) every scene holds 60 fps. On a
2560 x 1440 canvas (pixel ratio 2) the scenes with many OFFSCREEN GROUPS do
not: "shapes" (its blend-mode groups and backdrop copies) takes about 50 ms
a frame, "svg-features" (its masks, blend modes and blur) about 34 ms,
"clip-mask" about 30 ms and "motion-blur" (its Plus-composited groups) about
29 ms; every other scene holds 60 fps there too. That is the cost of
offscreen groups at that size (THE COST OF EFFECTS AND MASKS in the root
AGENT-README.txt), not a fault of the scenes. A check that measures frame
times in "shapes" at pixel ratio 2 sees the catalogue move on to the next
scene meanwhile: check that scene's motion and seek in a run of its own.

What to check (each page's elements carry ids for this: page-title,
adapter-name, frame-count, webgpu-canvas, webgpu-unavailable,
graphics-fallback; on graphics
also clock-time, clock-state, clock-toggle, overlay-toggle,
triangle-point-frame, overlay-point-frame):
  - "/" serves the page with links to /graphics and /animation and to the
    two notices files (/THIRD-PARTY-NOTICES.txt, /SAMPLE-THIRD-PARTY-
    NOTICES.txt), and the links load them; /graphics opens on the Viewer
    (scene-viewer is the selected button), so a check of another scene
    presses its button first;
  - WITH a WebGPU adapter: graphics names the adapter (in a browser this is
    little - Chrome gives no description on purpose; the probe
    navigator.gpu.requestAdapter() tells vendor and architecture), the frame
    counter advances, and the canvas is not blank. A WebGPU canvas can be
    read only in the task that drew it: sample its pixels inside an
    animation frame (requestAnimationFrame twice, then drawImage into a 2D
    canvas and getImageData); read at any other time it is transparent
    black. The triangle's colours change over time, a drag changes the
    rotation, the wheel changes the scale (and the page does not scroll),
    keys work once the canvas has the focus, and a narrower window or
    another device pixel ratio resizes the surface (canvas width = displayed
    width x devicePixelRatio);
  - THE ANIMATION APPLICATION: each scene of the catalogue (?scene=<name>)
    starts (demo-frame counts up), a grid of canvas pixels has more than a
    handful of colours, changes while the clock runs (except "sketch" with
    its boil rate at 0, its default: a still grid by design), does NOT change while
    the clock is paused (demo-clock-toggle) and changes again after resuming;
    with the clock paused, Right (a seek of one second) changes the picture
    and Left puts it back exactly;
    demo-error-count stays 0 (the page counts the device's uncaptured
    errors); the keys (Shift+Right easing -> ramps, Space pauses, Home back to
    easing) and the scene buttons work; in "shapes", "Check the sample
    points" (shapes-check-run) reports every sample point passed
    ("points=... passed=... failed=0"), at device pixel ratio 1 and 2, and
    in "missing-characters" its six points (ink over the missing cluster with
    ShowTofu in both fonts and FontDefault in Roboto, none with Blank and with
    FontDefault in Open Sans); the header area (title and description, drawn
    by the crate) has text pixels and differs between scenes; D switches the
    debug overlay (demo-overlay on / off) and changes the bottom-right corner
    only while it is on; each page load requests each font once (200,
    font/ttf);
  - DRAWING ON THE PAGE (/animation/?scene=on-the-page and the graphics
    "On the page" scene): only the browser's own compositing shows it, so the
    check reads the PAGE as composited - a protocol screenshot of the canvas
    region read back as bytes (a test-tool probe, never compared with any
    other image) - with the clock paused, once with the canvas and once with
    it hidden (page-canvas-toggle): where nothing is drawn the two are the
    same; the still half-transparent square (blue 30,60,200 at alpha
    128/255) equals the sRGB blend of its colour and the page's within a
    step; at the grey disc's anti-aliased edge every pixel lies between the
    grey and the page (no fringe); in graphics, after "Change the page's
    background" no pixel of the models changes and every other pixel is the
    new page; page-error-count stays 0; the console is empty;
  - THE LAYERS AND THE CLOCK (graphics): triangle-point-frame and
    overlay-point-frame show the SAME frame number once the clock passes
    "title in"; with the clock paused (clock-toggle) the clock time, the
    rotation and every pixel stay the same while the frame counter goes on;
    removing the overlay layer (overlay-toggle) while paused changes only
    the pixels under the square, and adding it back restores them; where
    the square covers the triangle a pixel is the premultiplied blend of
    the two (decode the triangle's pixel from sRGB, mix 0.5 of the square's
    linear colour with 0.5 of it, encode again: it matches within a step of
    rounding), not either colour alone; resuming continues the clock from
    the paused time (it gains about the time since the resume, not the time
    of the pause);
  - THE SOLID SQUARE (graphics, Layers): with the clock paused, overlay-solid
    changes only the pixels under the square, each to the square's own
    colour encoded to sRGB ([255,196,89]); pressing it again restores the
    translucent pixels exactly. A rightward drag (spin stopped) makes
    view-rotation GROW;
  - THE SHADERS (graphics, Shaders): shader-state reaches "done",
    shader-compiled equals shader-total, shader-failed and shader-uncaptured
    are 0; read shader-warnings and every row of #shader-list whose
    "Compiler messages" cell is not "(none)" (list them verbatim);
  - THE QUAD (graphics, Textured quad; ids quad-canvas, quad-frame,
    quad-angle, quad-clock-toggle, quad-clock-state, quad-readback,
    quad-pipeline-check, quad-validate, quad-error-count): the canvas is not
    blank and its pixels change from one probe to the next; with the clock
    paused quad-angle and every pixel stay the same while quad-frame goes
    on; a rightward drag makes quad-angle grow; after a resume the pixels
    change again; the three checks read "ok ..." and quad-error-count is 0;
  - THE SPRITES (graphics, Sprites; ids sprites-canvas, sprites-frame,
    sprites-time, sprites-clock-toggle, sprites-clock-state,
    sprites-filter-toggle, sprites-filtering, sprites-draw-calls,
    sprites-quads, sprites-arrow, sprites-top-card, sprites-atlas,
    sprites-run-check, sprites-error-check, sprites-deliberate-count,
    sprites-error-count): sprites-atlas reads "2048 x 512 texels ..."; until
    sprites-run-check is pressed sprites-error-check reads "not run yet ..."
    and the console has no entry from the scene; the canvas is not blank and
    changes; sprites-top-card shows all three cards within six seconds; with
    the clock paused every pixel and sprites-time stay the same; the
    filtering switch changes pixels; after a press of sprites-run-check,
    sprites-error-check reads "ok ..." and sprites-deliberate-count 1;
    sprites-error-count is 0 throughout;
  - THE LIT SHAPES (graphics, Lit shapes; ids lit-canvas, lit-frame,
    lit-clock-toggle, lit-clock-state, lit-projection-toggle,
    lit-projection, lit-auto-orbit, lit-orbit-mode, lit-yaw, lit-pitch,
    lit-distance, lit-pointers, lit-draw-calls, lit-instances,
    lit-error-count): the canvas is not blank; running, lit-yaw grows (the
    auto-orbit); paused, lit-yaw and every pixel stay the same; a rightward
    drag of 100 px LOWERS lit-yaw by 0.8 and lit-orbit-mode turns "user"; a
    drag down raises lit-pitch by 0.006 per pixel, and long drags stop it at
    1.35 and 0.05; a wheel step towards the visitor (deltaY > 0) multiplies
    lit-distance by 1.1, away by 0.9, and many steps stop it at 30 and 3 (the
    page does not scroll); with touch emulation, two touch points spreading
    apart lower lit-distance and coming together raise it, without changing
    lit-yaw, and one touch point dragged right lowers lit-yaw; the projection
    switch changes the pixels; "orbit by itself again" and a resume make
    lit-yaw grow again; lit-error-count is 0;
  - THE VIEWER (graphics, Viewer; ids viewer-loading, viewer-failed,
    viewer-canvas, viewer-frame, viewer-clock-toggle, viewer-clock-state,
    viewer-auto-orbit, viewer-orbit-mode, viewer-yaw, viewer-pitch,
    viewer-distance, viewer-models, viewer-instances,
    viewer-triangles, viewer-draw-calls, viewer-drawn, viewer-pipelines,
    viewer-error-count, viewer-files, viewer-download-bytes, viewer-fetch-ms,
    viewer-build-ms, viewer-textures, viewer-texture-sharing): the network
    shows api/models/scene (200, JSON), then each model file ONCE -
    api/models/files/<kit>/<model>.glb (200, model/gltf-binary, br or gzip),
    each kit's Textures/colormap.png (200, image/png, ONE request per kit) -
    and api/models/ground.cbmd (200, application/octet-stream): 24 files
    (viewer-files), their decoded sizes adding up to viewer-download-bytes;
    viewer-models is 21, viewer-textures 3 (one colour map per kit), and
    viewer-texture-sharing starts with "ok:" (a model of the first
    create_models call and one of the second, from the same kit, hold the
    same GPU texture); viewer-instances is the ground, the 21 props and the
    10 actors; the canvas is not blank;
    running, viewer-yaw grows; paused, viewer-yaw and every pixel stay the
    same while viewer-frame goes on; a rightward drag lowers viewer-yaw
    (0.008 per pixel) and turns the mode "user"; a drag down raises
    viewer-pitch; a wheel step towards the visitor raises viewer-distance by
    1.1 times; "orbit by itself again" and a resume make viewer-yaw grow;
    viewer-error-count is 0;
  - PANNING AND THE POINTER (every orbit scene; checked in the Viewer and
    Lit shapes with synthetic pointer events over the protocol,
    Input.dispatchMouseEvent): with the clock paused and the camera "user",
    a RIGHT-button drag, a MIDDLE-button drag and a Shift + left drag each
    change the pixels while yaw, pitch and distance stay exactly the same
    (the target moved); a middle drag back by the same amount restores the
    frame exactly (the pan follows the pointer); a "contextmenu" listener on
    the window sees the right press over the canvas with defaultPrevented
    true (no menu), and a middle-button "mousedown" over the canvas has
    defaultPrevented true (no autoscroll); in Lit shapes with the
    orthographic projection, wheel steps change lit-distance and the pixels
    (the orthographic view zooms) and a middle drag pans. What only a
    person can check: that the menu really does not appear, that no
    autoscroll icon appears on a real middle click, the feel of a trackpad
    and of two-finger touch panning;
  - THE VIEWER'S ANIMATION (ids viewer-scene-time, viewer-actors,
    viewer-clips-playing, viewer-pose-signature, viewer-bird, viewer-routine,
    viewer-seeks, viewer-seek-back, viewer-seek-forward, viewer-speed):
    viewer-actors is 10; running, viewer-scene-time, viewer-pose-signature
    and viewer-bird change; paused, all three stay exactly the same while
    viewer-frame goes on; with the clock paused, "Clock +5 s" moves
    viewer-scene-time by 5 and changes the pose, the bird and the pixels,
    "Clock -5 s" brings back the same bird position and (to a rounding of
    float sums) the same pose signature, and "Clock +5 s" again gives exactly
    the pose of the first jump; a resume goes on from the paused time; with
    "Half speed" viewer-scene-time advances half as fast as real time;
    within a routine change viewer-routine shows two clips with weights
    adding up to 1 (the cross-fade);
  - THE SKINNED MODEL and THE MORPH TARGETS (graphics, Skinned model and
    Morph targets; ids with the prefix skinned- or morph-: -loading,
    -failed, -canvas, -frame, -clock-time, -clock-state, -clock-toggle,
    -seek-back, -seek-forward, -speed, -clip, -clip-time, -states, -probe,
    -bytes, -skins, -joints, -morph-targets, -triangles, -draw-calls,
    -pipelines, -error-count; skinned-clip-select): Skinned model requests
    api/models/files/kenney-mini-forest/character-archer.glb and that kit's
    Textures/colormap.png (200), Morph targets api/models/morph-shape.cbmd
    (200);
    skinned-skins is at least 1 and skinned-joints more than 0,
    morph-morph-targets 3; the canvas is not blank; running, the probe
    (skinned: the left arm's world position; morph: the three weights) and
    the pixels change; paused, both stay exactly the same; "Clock +1 s"
    changes the probe and "Clock -1 s" brings back the same probe; a resume
    goes on from the paused time; choosing another clip in
    skinned-clip-select shows 2 states for a moment (the cross-fade), then 1
    with the new clip's name; the error counts are 0;
  - THE VIEWER'S LIGHT AND HDR PATH (ids viewer-environment,
    viewer-environment-size, viewer-environment-build-ms,
    viewer-panorama-state, viewer-environment-toggle, viewer-render-path,
    viewer-tone-operator, viewer-tone-exposure, viewer-skybox,
    viewer-tone-operator-select, viewer-exposure-input,
    viewer-exposure-value): by default viewer-environment reads "procedural
    sky", viewer-render-path "HDR: an RGBA16Float target of <canvas size>
    ...", viewer-tone-operator "KhronosPbrNeutral" and viewer-skybox "on
    (<n> draws)"; with the clock paused, setting each operator in the select
    (dispatch "change") changes the pixels and viewer-tone-operator, and an
    exposure of 2 or 0.5 (dispatch "input") changes them (back to 1 restores
    the frame exactly); viewer-environment-toggle makes ONE request
    api/environments/small-cathedral.hdr (200, image/vnd.radiance, ETag),
    viewer-environment becomes "Small Cathedral (HDR panorama)" and the
    pixels change; switching back and forth gives the same two frames and no
    further request; ?ldr shows "LDR ..." and "none (LDR)" with the controls
    disabled; ?no-skybox shows "off ..." and a top row of one colour (the
    flat sky); ?ldr&no-skybox both, its top row exactly the flat sky
    [150,196,232]; every one moves with the clock and has no error;
  - THE MORE SCENE (graphics, More; ids more-canvas, more-frame,
    more-clock-state, more-time, more-clock-toggle, more-array-frame,
    more-streaming-frame, more-query-reads, more-gpu-time,
    more-particles-drawn, more-occlusion, more-texture-readback,
    more-region, more-mapped, more-checked, more-error-count,
    more-query-failure): the canvas is not blank; running, the pixels,
    more-time, the two frame numbers and more-particles-drawn change;
    paused, every pixel and more-time stay exactly the same while
    more-frame goes on; more-query-reads grows; more-gpu-time shows two
    times where timestamp-query was granted; more-occlusion shows the
    crate's refusal; the four checks read "ok ..."; more-error-count is 0;
  - THE PAGE CACHE: switching Viewer, Skinned model, Morph targets, More,
    On the page and back again (and the Viewer's panorama off and on)
    requests api/models/scene, each model file, each colour map,
    ground.cbmd, morph-shape.cbmd and small-cathedral.hdr ONCE each. A page
    RELOAD asks for them again, and the browser's HTTP cache answers: the
    platform's client reads a file answer as bytes with the fetch cache mode
    "no-cache", so the browser keeps the answer and REVALIDATES it - it
    sends the ETag back (If-None-Match), the backend answers 304 with no body
    (about 300 bytes on the wire per file) and the browser hands over the
    bytes it kept. The scene description (JSON) is fetched with "no-store"
    and comes again in full;
  - THE CURRENT SCENE'S BUTTON (both applications): exactly ONE element
    carries aria-current="true" (and the filled style, bg-sky-300) and it is
    the scene being drawn - on /graphics/ scene-viewer; after a click on
    another scene's button that one; after loading /graphics/?scene=<name>
    (a feature demo or on-the-page) scene-<name>; after history.back() the
    scene of the address it returns to. In /animation/ it is
    scene-<demo-scene's text>: at load, after the catalogue moves on BY
    ITSELF (wait out the first scene, about 8 s), after a scene button,
    after demo-next / demo-previous and the keys, after loading
    ?scene=<name>. With the button box made short (set
    #scene-buttons' maxHeight to 44px through the protocol), choosing a
    scene whose button is out of the box scrolls THE BOX (its scrollTop
    changes) so the button is wholly visible, while window.scrollY and
    document.activeElement stay the same;
  - THE FEATURE SCENES (animation, ?scene=<name>&hold=1 for fixed-width,
    session-blend, paint-override, wipe, sketch, ownership-colours,
    stroke-sets, travelling-strokes, direction-fields, painting-timeline,
    colour-rules, live-pickup, surface-finish, checkpoints,
    stroke-transition, water-ripples, water-reflection, painted-pond,
    simulated-pond-2d, and the noise scene's caustics panel; controls
    <scene>-<key>, read-outs <scene>-<key>, the
    second stats line demo-stats-2): each draws with demo-error-count 0, no
    console message, no CSP violation, and the scene's button is the
    current one; demo-hold "on" keeps the page on the scene past its end.
    Read pixels at the canvas's own resolution (draw the canvas into a 2D
    canvas inside requestAnimationFrame):
    FIXED-WIDTH: swing off, still scale 1 then 3: a column through the
    measured square (frame x 860) shows the top row's edges 4 then 12 px
    thick, the bottom row's 4 and 4 (camera zoom 2: 24 and 8);
    SESSION-BLEND: the eight panels differ (normal, multiply, screen, ...);
    PAINT-OVERRIDE: the three panels differ; the Logo box (read-out
    logo-boxes) is the same in all three while "Keep the Logo layer" is on,
    and differs in the line panel when it is off; effects off changes the
    authored panel;
    WIPE (clock paused, animate off): for each pair, the area at progress 0
    equals "the first render alone" exactly and at progress 1 equals "the
    second render alone" (pixel hash); the iris at progress 1 equals the
    second render;
    SKETCH (?t=1&paused=1): the sixteen cells differ; a reload gives the
    same grid (same seed, same pixels); seed 2 changes it; with a boil rate
    and the clock paused the grid stays the same, running it changes
    (sketch-epoch moves);
    OWNERSHIP-COLOURS: with with_colors, demo-stats-2 shows image_uploads=0,
    color_table_uploads=1, color_table_upload_bytes=2240 every frame; the
    old way image_uploads=1, image_upload_bytes=943040;
    STROKE-SETS: stroke_draws=1 and strokes = stroke-sets-started at every
    count (2,000 - 100,000), draw_calls unchanged by the count, no stroke
    or tip uploads after the first frame of a set; paused, the pixels stay
    the same; the old path: stroke_draws 0, draw_calls and cpu_ms up;
    TRAVELLING-STROKES: anchored + boil running changes the pixels frame to
    frame, paused they stay the same; regenerated: a stroke upload every
    frame (stroke_uploads 1);
    DIRECTION-FIELDS: raising "small strokes where detail >=" lowers
    direction-fields-small; running, the noise panel's pixels change (flow)
    while the structure panel's stay;
    PAINTING-TIMELINE: ?t=20&paused=1, then scrub to 45 and back to 20: the
    pixels are the same as at the first visit; played to a time T and
    paused, the pixels equal a fresh ?t=T&paused=1 (T from
    painting-timeline-time-exact); the stage buttons
    (point-painting-timeline-<stage>) seek to the stage starts;
    COLOUR-RULES (?t=1.5&paused=1): the four stroke panels differ; at the
    bars' middle the sRGB bar is grey (140,140,130), Oklab and Oklch are not;
    each ColorRamp strip matches its gradient above it;
    LIVE-PICKUP (source: still picture, moving session, 3D scene): each
    source draws with live-pickup-scene3d-errors and live-pickup-reports
    "none"; running, demo-stats-2 shows stroke_draws=1, strokes=20000 and
    stroke_uploads=0 / image_uploads=0 every frame while the pixels change
    (moving session, 3D scene); paused, they stay the same; with "Show: the
    source itself", the mean colour of each cell of an 8 x 4 grid over the
    painting area matches the strokes' (correlation above 0.99); own colour
    weight 1 gives the iris picture whatever the source; the rules change
    the strokes (palette: fewer colours; broken colour: more);
    SURFACE-FINISH (light circles off, clock paused): another light
    direction changes the painting and the weave-only panels; back to 225
    gives the same pixels; the finish off changes all three areas; with the
    weave off the flat rectangle and the picture are EXACTLY as with the
    finish off (no strokes, no thickness), and impasto 0 -> 1.5 changes the
    painting only; demo-stats passes go up with the finish on;
    CHECKPOINTS (?scene=checkpoints&hold=1; clock paused, the scrubber at a
    few times - 3, 9.5, 14.25, 25, 6 s): the canvas's pixels are EXACTLY the
    same with "Checkpoints" on and off (hash of the full canvas), also
    right after a seek back; demo-stats-3 with them on shows strokes = the
    strokes after the restored checkpoint (a few thousand; 0 once the
    painting is finished) and strokes_restored = the rest, with them off
    strokes = every started stroke; playing, checkpoints climbs to its
    capacity (memory checkpoint_mb); "Move the painting" gives
    checkpoint_misses=1 and strokes_restored=0 every frame, and the cache
    comes back when it stops; checkpoints-refusal "no"; at a device pixel
    ratio of 2 (Chrome --force-device-scale-factor=2) frame_ms_smoothed of
    the finished painting drops from about 46 ms off to the 16.7 ms vsync
    on;
    STROKE-TRANSITION (?scene=stroke-transition&hold=1&t=4&paused=1): at
    the scene times 0.5 s (fraction 0) and 8 s (fraction 1) the canvas is
    EXACTLY "Show: the first painting alone" and "the second painting
    alone" (full-canvas hash); on the slider, 0 and 1 give the same two
    pictures, 0.5 is repeatable (0.5 -> 0.25 -> 0.5 gives the same hash)
    and equals the clock's t = 4 s (the track's eased fraction is 0.5
    there); scrubbing to 12 s and back to 4 s, and a fresh
    ?t=4&paused=1, give the same hash; stroke-transition-paired / -unpaired-from
    / -unpaired-to read 7000 / 1000 / 0 for 8,000 and 7,000 strokes and
    2500 / 5500 / 0 for 8,000 and 2,500;
    WATER-RIPPLES (?scene=water-ripples&hold=1&t=2&paused=1): the
    full-canvas hash at a scene time is the SAME on a fresh ?t= page, after
    scrubbing away and back (4 -> 9 -> 0.4 -> 2), and - played to a time
    and paused - on a fresh page at the exact time the read-out
    water-ripples-time-exact gives, and after Left then Right; a click on
    the water at t = 2 (paused) adds a stone (water-ripples-stones) and
    changes nothing at 2 and 1.9 but about 15,000 pixels at 3 and about
    24,000 at 4 (the ring spreads), and scrubbing back to 3 gives the same
    pixels again; strength 0, shading 0, glints and caustics off against
    "Water: off" differ only in about 300 anti-aliased edge pixels of the
    pool (the effect group's resolve); water-ripples-ripples-now equals the
    live drops; demo-error-count 0;
    WATER-REFLECTION (?scene=water-reflection&hold=1&t=3&paused=1): with
    "none: a still mirror", the refraction and the bed off, switching the
    reflection off changes NO pixel above the line (104-404) and none past
    the fade distance (606-664), and nearly every pixel between; ripples
    and a swell break it up (about 14,000 pixels against the still mirror,
    none above the line), break-up 0 equals the still mirror exactly;
    "the refraction's ripples" gives water-reflection-shared "yes" and
    ripples_evaluated equal to the live drops (one evaluation);
    scrubbing away and back, and a fresh ?t=5, give the same hash;
    NOISE: the value and gradient panels (32-320 and 336-624, y 112-452)
    have exactly the pixels the first 288 columns of the three-panel
    scene had (hashes at t = 2 and 5); the caustics panel (640-928)
    changes between t = 2 and 5;
    PAINTED-POND (?scene=painted-pond&hold=1&t=10&paused=1): painted-pond-
    sets about 6,000 / 9,000 / 4,400; scrubbing 1 -> 3 -> 5 -> 7 -> 18 ->
    10 and a fresh ?t=10 give the same hash; "Water: off" changes the
    water and (bar about 50 edge pixels) nothing above the line; a stone
    dropped at 8.5 changes nothing at 8.5 and shows a pale ring at 10;
    frame_ms_smoothed playing at device pixel ratio 1 about 18 ms (16.7
    with the water off), at 2 about 51 ms;
    SIMULATED-POND-2D: simulated-pond-2d-simulated counts up while
    running and stays while paused (and the hash with it); a drag across
    the water changes most of the pool within 0.4 s; scrubbing back does
    not take the simulated time back; errors 0.
  - THE FEATURE DEMOS (graphics, ?scene=lights, shadows, culling,
    translucency, glow, particles, picking, antialiasing, lod, water-pond,
    water-lake; ids <name>-canvas, -frame,
    -clock, -clock-toggle, -speed, -error-count, -timing-cpu, -timing-gpu,
    -timing-counts, -timing-line, one id per control <name>-<key> - a
    slider's value also in <name>-<key>-value - and per read-out): each
    starts (-frame counts up), -error-count stays 0, the console is empty,
    no CSP violation (the picking labels are placed through CSSOM style
    properties, which the CSP allows); -timing-gpu reads milliseconds where
    the browser grants timestamp-query ("not available ..." otherwise) and
    -timing-counts names the renderer's counts;
    LIGHTS: lights-given 10, lights-shaded 10 at the start; lamps 40 ->
    lights-given 42, lights-shaded 16, lights-skipped 26; lamps 0 with the
    spot and the hand lamp off -> 0 and a darker canvas; paused,
    lights-hand stays the same, running it moves;
    SHADOWS: sun shadows off -> shadows-shadow-passes 0; on -> 1, shadow
    draw calls above 0, the ground band (the lower half of the canvas)
    darker (mean luminance down) and "Tall tree casts" off makes it lighter
    again (the tall tree is culled for the camera: shadows-tree-in-view
    "no"); map size 1024 / 4096 -> shadows-map-memory 4 / 64; spotlights 4
    -> 5 shadow passes, spots-casting 4; spotlights 6 -> still 5 passes,
    spots-casting 4, spots-without 2; sun off -> 4 passes;
    CULLING: culling-agree "yes" whenever culling is on, culling-culled
    changes as the camera turns; with the clock paused, switching culling
    off gives culling-culled 0 and more drawn instances while the pixels of
    the main view (outside the inset) stay EXACTLY the same;
    TRANSLUCENCY: with the clock paused and turning off, switching
    two-pass on one side changes that side's pixels and its draw calls by
    one per glass part (translucency-left-draw-calls /
    -right-draw-calls); -lid-found true, -tails-found 5;
    GLOW: glow-glow-draw-calls 1 with 311 glows and with 2011 (fireflies
    2000); paused, the pixels stay the same; bloom off changes the
    picture (fewer colours, more dark pixels); glow-bloom-levels follows
    the radius;
    PARTICLES: running, particles-sparks-spawned and -sparks-time grow;
    paused, they and every pixel stay the same (particles-delta 0);
    quarter speed: -sparks-time grows by about 0.25 s a second; a click on
    the ground -> particles-clicks 1, -bursts-spawned +300 at the clicked
    place (-burst-at); "Burst over the fire" +300 more; reset -> spawned
    near 0;
    PICKING: with the clock paused, a click on the tall tree's trunk (about
    110 CSS px under the top of its label picking-label-5) -> picking-picked
    "tall tree" and that label highlighted (bg-amber-300); a click on the
    sky -> "nothing" and picking-misses +1; the labels' left / top follow
    the models; after zooming in, labels of models behind the camera or off
    the canvas carry "hidden". Picking is by BOX: a model whose box is in
    front wins (the flying Avian's box covers some of the tall tree's
    crown);
    ANTI-ALIASING (clock paused, both paths): with the first canvas at 4x
    and the second (antialiasing-compare-canvas) Off, high-contrast edges
    have far more in-between pixels on the first (about a third against a
    tenth); switching the first to Off gives EXACTLY the second canvas's
    pixels (hash), back to 4x the first's earlier pixels;
    antialiasing-ms-hdr-sets (HDR) and antialiasing-ms-canvas-sets (direct)
    stop at 2 however often it is switched; on the direct path the bands of
    the layer below show where no model is (the same pixels in both
    canvases, at 4x and Off); -error-count and -compare-error-count 0;
    LOD (camera "hold at a distance"): lod-level-0 / -1 / -2 / -fading /
    -culled / -beyond change with the distance (15, 60, 150, 300 m) and
    lod-renderer-fading / -culled equal the page's -fading / -culled; with
    the TEST set on black and hysteresis 0, the marked grove at 37 / 38 /
    39 m (band 0.1: progress 0.25 / 0.5 / 0.75) covers exactly the pixels it
    covers with band 0 (no hole), split red / green about 75/25, 50/50,
    25/75 - at 1 and 4 samples; flying through 40 m (and through the
    cut-off at quality 0.25, 65 m) the covered-pixel count changes by a few
    pixels a frame with band 0.1, and jumps in ONE frame with band 0;
    WATER-POND: the defaults are the palm pool's look (straight down, even
    floor, refraction 0.95, water "as the palm pool", bloom off ...: the
    controls read so at load); the mosaic is there and right - "Camera:
    straight down", rain 0, "Water effects: off", "Calm the water": at each
    mosaic probe (water-pond-probe-<name>: the canvas point, the palm colour,
    the colour expected on screen) the median of the canvas pixels round the
    point matches the expected colour within about 15 per channel (heart,
    the four petal colours, the sea glass; the gold ring at the angles where
    no grout line runs along it - the minified texture averages a grout line
    into the 8-pixel-wide ring elsewhere); with the effects on (caustics 0,
    sheen 0, fine ripples off, no rain) the picture holds still, a drag
    across the water changes about 100,000 pixels and it settles back
    (after 12 s only a faint remainder); rain changes it; paused, the hash
    and the field's steps stay the same; sheen 0 against 0.3 changes about
    100,000 pixels and 0.3 again gives exactly the earlier pixels; Off and
    4x both draw (water-pond-ms-samples 1 / 4, errors 0); the GPU-vs-CPU
    panel reads 200 steps each and a largest height difference of about
    1e-6;
    WATER-LAKE: water-lake-barrel's water line equals the surface height
    there and changes with the clock (frozen while paused); the shallow
    probe is sandy and the deep one blue-green (and the presets move both);
    screen-space reflections off, reflection strength 0, sun shadows off,
    caustics off, the lamps off and "waves only" each change the picture
    (paused), and switching back gives exactly the earlier pixels;
    water-lake-water-lights / -model-lights read 2 with the lamps on.
  - WITHOUT an adapter (Chrome without the switch, or Firefox without
    dom.webgpu.enabled): /graphics shows its own panel (graphics-fallback)
    and no "not available" message; /animation shows "WebGPU is not
    available in this browser"; /animation/?scene=on-the-page shows the page
    alone (page-surface, no page-canvas, no page-clock-toggle, no message);
    the console has no uncaught error (Chrome itself warns "No available
    adapters.");
  - the console shows no errors and no Content-Security-Policy violations,
    and no request fails (the one expected console entry is Chrome's
    warning for the Sprites scene's deliberate error, once its check button
    has been pressed);
  - the icon is served for all three: /favicon.ico,
    /graphics/assets/favicon.ico, /animation/assets/favicon.ico;
  - THE SITE AND ITS HEADERS: "/" and site.css and favicon.ico come from
    wwwroot/ (no-cache), the built applications and the notices from
    frontend-built/ (hashed files public, max-age=31536000, immutable;
    others no-cache; br or gzip with Vary: Accept-Encoding),
    /.gitignore and /codebrix-frontends.toml answer 404; the model files
    and blobs carry "public, no-cache" and a strong ETag (If-None-Match ->
    304), .glb/.cbmd answers are compressed, PNG is not; a pack's licence
    file (api/models/files/<kit>/LICENSE-...txt), a path with "..", an
    unknown model or font answer 404 with the standard problem body;
  - THE OFFLINE FILE: with an app-offline.html in wwwroot/ (made for the
    check, deleted after), every page (/, /graphics/..., /animation/...)
    answers 503 with that file and "Cache-Control: no-store", every
    api/... request 503 with the problem code "offline", the built files
    503, the developer's site.css 200; deleting the file brings the site
    back without a restart;
  - A PUBLISHED BACKEND (PUBLISHING THE BACKEND above) serves "/", both
    applications and the model files from its own folder.


THE MODELS: GraphicsDemo.Assets AND api/models
==============================================
The models of the "graphics" application stay on the backend and travel to
the browser AS FILES, unchanged: the kits' .glb files and the colour maps
they name, read in the browser by the graphics crate's glTF loader
(GltfDocument::parse, GltfDocument::external_files, GltfModelSet::add and
create_models; frontends/graphics/src/model_files.rs). NOTHING IS CONVERTED
OR DECODED FOR THE MODELS ON THE BACKEND. Only the two models that have no
file - the ground and the morph-target shape - are built in code there and
travel as model data blobs (the model data format of
CodeBrix.WebPlatform.WebGpu.Graphics, ".cbmd"), read by the crate's blob
reader (GraphicsContext::load_model).

  - GraphicsDemo.Assets holds the .glb files and each pack's
    Textures/colormap.png and licence file as EMBEDDED RESOURCES (resource
    name = the path below Models/). The repository's test-side reference
    converter (tests/CodeBrix.WebPlatform.WebGpu.Graphics.Tests.GltfReference,
    the C# converter that once ran here) LINKS this Models/ folder and checks
    the Rust loader against it: if the folder moves, change that link and run
    that project's tests. Only third-party files whose source and
    licence are recorded are carried, and THIRD-PARTY-NOTICES.txt beside
    this file lists them; the owner's own pack (crystals-and-avian) needs
    neither. A new model goes into its pack's folder with an entry there.
  - ModelLibrary finds the model files (a .glb or .gltf file directly in
    its pack's folder is a model, named by its file name) and serves every
    file a model is made of UNCHANGED (GetFile: .glb, .gltf, .bin, .png,
    .jpg; never a licence file), read once and kept with a strong ETag; it
    writes the blobs of the two models built in code on first use
    (GetModelBlob). BuiltModels/GroundModel builds the ground exactly as the
    glTF loader reads the desktop viewer's ground document (its blob is byte
    for byte what the old converter wrote); BuiltModels/MorphShapeModel
    BUILDS the model of the "Morph targets" scene in code, as model data
    directly (no glTF, no asset file): a unit sphere with three morph
    targets (position and normal deltas: stretch, twist, bumps) and the
    weights clip "morph".
  - Scene/ViewerSceneLayout is the desktop viewer's scene: the actors (the
    chest, the eight animals and the Avian, as Scene/ViewerActor: model, clip
    or routine, time offset, motion and its numbers, and WorldAt(t), the
    desktop's placement function), the routine and cross-fade times, the
    props table and the camera and lighting values (the sun, the procedural
    sky's colours and turbidity, the exposure under the HDR panorama). The
    scene JSON (api/models/scene) lists the ground and every model the scene
    places (SceneModels: the actors' models, then the props') with, for
    each, its file below Models/ and its pack (the base address of the files
    it names) - empty for the ground, which is a blob - and carries the
    placements and the actors (Contracts/).
  - Models/kenney-mini-forest/ holds the Kenney archer (character-archer.glb,
    a skinned model) of the "Skinned model" scene, its colour map and its
    licence file, copied byte for byte from the CodeBrix.WebGpu repository's
    test assets (Kenney Mini Forest, CC0, recorded there).
  - GraphicsDemo.Assets takes CodeBrix.WebPlatform.WebGpu.Graphics as a
    PACKAGE at the same version as the build program (the model data types
    and ModelDataWriter, for the two blobs), with its build and
    buildTransitive assets excluded: a backend never builds a frontend, so
    the crate archives the package copies into a build program's folder stay
    out of the backend's. It takes CodeBrix.Imaging for PictureLibrary only
    (the "animation" application's one picture in the picture-pixels form);
    it has no glTF package.
  - Controllers/ModelsController (api/models, anonymous):
      GET api/models/scene                  the scene description (JSON)
      GET api/models/files/{pack}/{file}    [WireFile] ModelFile: a model
                                            file, such as
                                            survival-kit/tree.glb
      GET api/models/files/{pack}/{folder}/{file}
                                            [WireFile] ModelFolderFile: such
                                            as survival-kit/Textures/
                                            colormap.png
      GET api/models/{name}.cbmd            [WireFile] ModelData: the blob
                                            of ground or morph-shape
    Anything else is 404 with the standard problem body. The path parts are
    plain names (no separator, no "." or ".."), joined and looked up among
    the embedded files - no file system path is formed. The frontend maps a
    loader address "<pack>/<file>" or "<pack>/<folder>/<file>" onto the
    generated model_file_bytes / model_folder_file_bytes. WHY A [WireFile]
    ACTION AND NOT PLAIN FILES: the models stay resources of the backend
    (the sample's rule) with no copy step and no second folder to serve, the
    frontend fetches them through the generated client like every other
    resource, and the platform compresses the answers by their content type
    (model/gltf-binary, model/gltf+json, application/octet-stream - the
    defaults of PlatformBackendOptions.CompressedFileTypes; PNG and JPEG are
    not compressed). Every file answer carries "Cache-Control: public,
    no-cache" and a strong ETag: a cache may keep it but asks again each
    time, so a rebuilt backend is never answered from a stale copy; a
    matching If-None-Match is answered 304 with no body, and the platform's
    client reads file answers with the fetch cache mode "no-cache", so the
    browser revalidates what it kept across page loads; within a page, the
    frontend's page_cache.rs keeps what it fetched. The answers are
    compressed as they are sent (Program.cs sets the platform's
    CompressResponses = FileAnswers: public assets, no user's data).
  - THE HDR PANORAMA: Environments/polyhaven-small-cathedral/ holds
    small_cathedral_1k.hdr (1024 x 512 Radiance RGBE) and its licence file,
    both copied byte for byte from the CodeBrix.WebGpu repository's test
    assets (Poly Haven, CC0), embedded like the models. EnvironmentLibrary
    reads it on first use and keeps it with a strong ETag; the backend
    registers ONE as a singleton, and Controllers/EnvironmentsController
    serves it unchanged:
      GET api/environments/small-cathedral.hdr   [WireFile] SmallCathedral
    The browser decodes it (the graphics crate's RadianceHdrDecoder) and
    bakes an environment from it; it is never decoded on the backend.
  - THE FONTS: Fonts/roboto/ (Roboto-Regular.ttf, the font the C#
    AnimationDemoApp draws all its text with, and the variable Roboto.ttf)
    and Fonts/open-sans/
    (OpenSans-Regular.ttf, for the missing-characters scene - its .notdef has
    no ink - and the SVG text of svg-features), each with its ORIGINAL
    OFL.txt, copied byte for byte from the family's font repositories
    (CodeBrix.Platform.Fonts.Roboto and CodeBrix.Platform.Fonts.OpenSans;
    THIRD-PARTY-NOTICES.txt beside this file names the files), under the SIL
    Open Font License 1.1. FontLibrary finds every .ttf and .otf file below
    Fonts/ (no file name in the code), reads each on first use and keeps it
    with a strong ETag; Controllers/FontsController serves them unchanged, by
    the file name in lower case:
      GET api/fonts/{file}   [WireFile] FontFile: roboto-regular.ttf,
                             roboto.ttf (the variable Roboto, axes wght
                             and wdth, for the "text" scene's weights),
                             opensans-regular.ttf
    A NEW FONT is its file (and its licence file) in a new Fonts/<pack>/
    folder, an entry in THIRD-PARTY-NOTICES.txt beside this file, and the
    frontend asking for it by name (frontends/animation/src/main.rs,
    FONT_FILES); nothing in the backend changes.
    The animation crate reads and shapes them in the browser. OFL fonts are
    allowed in samples and tests only: never move them into a package.
  - The tests (tests/GraphicsDemo.Assets.Tests) check the fonts (found by
    file name, any case, nothing else served, kept once, a strong ETag), the
    caption file (the
    four cues, kept once, a strong ETag), the SVG documents (each embedded
    SVG text, kept once, a strong ETag, an unknown name refused), the Lottie
    files (each one's JSON and width, the image-layer file's badge asset,
    kept once, a strong ETag, an unknown name refused), the pictures (the
    colour map's pixel form - a 512 x 512 header and length, kept once, a
    strong ETag; its PNG file's signature; its JPEG file's start and end markers; the badge made in code, opaque
    inside and transparent at its corners; a buffer that does not match its
    size is refused), the model files (the names and packs, the content
    types, that licence files, unknown paths and paths not written as
    embedded are refused, kept once with a strong ETag, that every model
    file is a glTF 2.0 binary whose external files are served from its
    pack), the ground and the blobs (only the two models built in code), the
    scene's models (each a file), the actors' placements against the
    desktop's formulas, that every actor's clips exist in its model file
    (read from the .glb's JSON chunk), the archer file's skins, clips and the
    node the skinned scene reads out, and the morph shape (its sphere, the
    three targets, the weights clip, and that it is written the same way
    every time).


THE PACKAGES
============
Every package the sample takes is published and comes from nuget.org. There
is no local package feed: no nuget.config, here or at the repository root.

  - The CodeBrix.WebPlatform packages: the Backend package
    (backend/GraphicsDemo.Api), and FrontendBuilder with the host-tools
    package of the machine (build/GraphicsDemo.BuildFrontend). Their ONE
    version is set in WebPlatformPackageVersion in both project files (THE
    PLATFORM'S PACKAGES in the root MAINTAINER-README.txt).
  - This repository's own WebGpu, Graphics and Animation packages
    (build/GraphicsDemo.BuildFrontend; the Graphics and Animation packages
    also in backend/GraphicsDemo.Assets). Their ONE version is set in
    WebGpuPackageVersion in the build program's project file and in
    backend/GraphicsDemo.Assets/GraphicsDemo.Assets.csproj (the two must
    name the same version).

The sample does not see a change to a library of this repository until the
three packages are published again and their new version is set here (THE
SAMPLE'S PACKAGES in the root MAINTAINER-README.txt).


GIT
===
Nothing the build writes needs a rule in the root .gitignore: every folder
the platform's FrontendBuilder owns holds its own .gitignore (a marker
comment and "*"), written on every materialize and build -
backend/GraphicsDemo.Api/frontend-built/, and in each frontend
platform/vendor/, platform/codebrix-webplatform/, src/generated/, .cargo/,
assets/generated/ and target/. A `cargo clean` that deletes target/ takes
its .gitignore with it; the next generate or build writes it again. bin/,
obj/ and TestResults/ are covered by the repository's general rules.
COMMITTED: wwwroot/index.html, site.css and favicon.ico (wwwroot/ holds
only those), each frontend's Cargo.lock (that application's own) and
assets/favicon.ico. A working tree holding output of the old layout
(wwwroot/graphics/, wwwroot/animation/, the manifest and notices in
wwwroot/, assets/tailwind.css, folders without their .gitignore): delete it
or run `generate` and `build` once with packages of the new layout.


CODING CONVENTIONS
==================
  - .NET: net10.0; nullable reference types OFF (never `?` on a reference
    type, never `!`); no implicit or global usings; file-scoped namespaces,
    one type per file, usings at the top (System.* first, then
    alphabetically by namespace); XML doc comments on public members; 0
    warnings in Debug and Release.
  - Rust: a `//!` comment at the top of each module saying what it is;
    0 warnings from `cargo check --target wasm32-unknown-unknown`.
  - The repository owner commits. Tools and agents never run git commit,
    push, add or reset.

================================================================================
END OF MAINTAINER-README
================================================================================
