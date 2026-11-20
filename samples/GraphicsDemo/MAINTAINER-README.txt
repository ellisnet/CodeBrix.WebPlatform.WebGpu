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
  - that models the backend converted are drawn with physically based
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
               opens). The device is asked for with the optional feature
               timestamp-query WHERE THE ADAPTER HAS IT
               (GraphicsContextOptions::optional_features in src/main.rs;
               otherwise as GpuContext::request asks), for the "More"
               scene's GPU timer. What a scene fetched from the backend (the
               scene description, the blobs, the panorama) is KEPT FOR THE
               LIFE OF THE PAGE (src/page_cache.rs): coming back to a scene
               asks the backend for nothing:

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
               (generated::api::models::scene) and the scene's model data
               (models::scene_data_bytes, the generated call of a [WireFile]
               action), showing a loading line while they arrive and a
               message that says what to do when a request fails; then
               GraphicsContext::load_models makes the GPU models (each pack's
               colour map uploaded once), every placement becomes a
               ModelInstance, and the ModelRenderer draws them with
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
               the exposure is the desktop's 1.2. The SKYBOX is drawn after
               the opaque models. THE HDR PATH: the scene goes into an
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
               0.7 + 0.18 t and the controller's limits. The legend is HTML
               over the canvas; under it: the light, the cube size and the
               CPU time of the last environment's decoding and recording, the
               panorama's state, the render path, the operator and exposure
               actually drawn, the skybox, the counts (bytes, models,
               instances, triangles, draw calls, pipelines), the camera's
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
               follows the pointer -, the wheel to zoom, two fingers to
               pinch-zoom, within pitch 0.05..1.35 and distance 3..30; it
               auto-orbits until the first drag. Buttons: pause/resume, the
               projection, orbit by itself again. The controls' legend is
               HTML over the canvas; the camera's values are shown under it.

               SKINNED MODEL and MORPH TARGETS (src/model_scene.rs,
               src/model_layer.rs): the two validation scenes of skinning and
               morph targets (none of the viewer's models has either). Each
               fetches ONE model's data (models::model_data_bytes) and draws
               one instance of it with the ModelRenderer, lit by the context's
               default environment and drawn over the skybox, under an orbit
               camera framed on the model's bounds (auto-orbit until the first
               drag). Its player follows the frame's TIME
               (AnimationPlayer::update_to), so pause, speed and seeks of the
               clock ("Clock -1 s", "Clock +1 s", "Half speed") move the clip
               to the matching place. SKINNED MODEL: the Kenney archer
               (character-archer, Kenney Mini Forest, CC0) playing "walk"; a
               list chooses any of its clips (the player cross-fades to it in
               0.3 s); the read-out is its left arm's world position. MORPH
               TARGETS: a sphere the backend builds in code
               (GraphicsDemo.Assets, Conversion/MorphShapeModel.cs; no asset
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
               that stand up, each scaled to one size) turn in a row, drawn
               with HDR ON (an RGBA16Float target cleared to transparent, the
               ToneMapper with preserve_alpha) and no skybox, lit by the
               context's default environment. Buttons: pause the clock, "Hide
               the canvas" (a class: the page alone at the same place),
               "Change the page's background" (where the models are, nothing
               changes; around them the new page shows).

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
               generated roboto_regular_bytes / open_sans_regular_bytes calls
               (FontsController, api/fonts/*.ttf, from GraphicsDemo.Assets),
               registers the bytes with one FontRegistry (Roboto first, the
               default), and only then builds the catalogue; a failed fetch
               shows its message (id fonts-failed) instead of the demo. The catalogue LOOPS; the scene starts are NAMED
               POINTS of the clock (the scene buttons seek to them). Keys,
               once the canvas has the focus: Space pauses / resumes the
               clock, Left / Right seek one second, Shift with Left / Right
               moves to the previous / next scene, Home starts again, D shows
               or hides the DEBUG OVERLAY (the crate's DebugOverlay on the
               catalogue's timeline - timecode, frame and the active cues -
               bottom right, as the C# demo's live window shows it). The
               address options ?scene=<name>&t=<seconds>&paused=1 start a
               given scene at a given time. ?scene=on-the-page is not a scene
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
                          (the C# demo's TextScene);
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
                          decoded by the crate; emblems over a checkerboard in a turning
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
                 noise    value, gradient and fractal noise evolving with
                          time, DRAWN ON THE GPU (NoiseShape - not C#'s exact
                          pattern), the seeded DrawingRandom dots (exact) and
                          a small exact CPU noise fill stretched (NoiseScene);
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
               missing-characters, the SVG scenes (svg-parts first) and the
               Lottie scenes at the end (demo/mod.rs, REGISTRATIONS).

Both show "WebGPU is not available in this browser", with one sentence of
what to do, when the browser gives no adapter.

At the site root, "/" is the developer's OWN page,
backend/GraphicsDemo.Api/wwwroot/index.html (plain HTML with site.css beside
it and its own icon link to /favicon.ico; no script): with every application
named, nothing of the platform's is at "/" (see SEVERAL FRONTEND
APPLICATIONS in the platform's AGENT-README.txt), and the frontend build
leaves the web root's other files alone.

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
    (FontsController, api/fonts/roboto-regular.ttf and
    api/fonts/open-sans-regular.ttf, unchanged as font/ttf with the same
    caching; FontLibrary of GraphicsDemo.Assets holds them as embedded
    resources, Fonts/<pack>/ with each font's original OFL.txt - OFL fonts are
    allowed in the sample only, never in a package) and the PICTURES of the
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
    (ImageSource::from_png): the "images" scene shows both side by side.
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
    wwwroot/THIRD-PARTY-NOTICES.txt and a
    copy of the sample's own file as wwwroot/SAMPLE-THIRD-PARTY-NOTICES.txt,
    both written by the build and linked from the page at "/".
  - ONE TEST PROJECT, for the model conversion (tests/GraphicsDemo.Assets
    .Tests); everything else the sample proves is checked by building it and
    looking at it in a browser (CHECKING IN A BROWSER below).


THE RULES
=========
The rules of the repository root's maintainer file hold here too (no
Node.js, no JavaScript of ours, no shell scripts, stable releases only, 0
warnings). For this sample:

  - PACKAGES ONLY. The build program takes the WebGpu, Graphics and
    Animation packages from the temporary feed (see TEMPORARY PACKAGES),
    never a project reference into src/: the sample uses them exactly as any
    application does. It calls GraphicsCrates.Register() and
    AnimationCrates.Register(), each of which registers the WebGpu bundle
    first (once).
  - ONLY THE APPLICATION PUTS THE TWO TOGETHER. The graphics application
    names codebrix-webgpu-graphics, the animation application
    codebrix-webgpu-animation; neither crate names the other.
  - NO TEXTURES, MODELS OR PANORAMAS FROM FILES IN THE FRONTEND: the
    models and the HDR panorama come from the backend; the textures of the
    quad and sprite scenes are made in code.
  - THE BACKEND NAMES NO FRONTEND APPLICATION. It serves what the frontend
    build wrote into its wwwroot, read from the manifest
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
    GraphicsDemo.Assets/        the models and their conversion (see THE
                                MODELS below): Models/<pack>/ (embedded),
                                Conversion/, Scene/, ModelLibrary.cs; the
                                panorama: Environments/<pack>/ (embedded),
                                EnvironmentLibrary.cs, EnvironmentFile.cs;
                                the fonts: Fonts/<pack>/ (embedded, each
                                with its OFL.txt), FontLibrary.cs,
                                FontFile.cs
    GraphicsDemo.Api/           the one backend: Program.cs (the two Backend
                                calls, the controllers and the
                                Content-Security-Policy), WireDeclarations.cs
                                (the request marker), Controllers/ and
                                Contracts/ (api/models, api/environments)
      wwwroot/index.html, site.css, favicon.ico
                                the developer's own page at "/" (committed)
      wwwroot/graphics/, wwwroot/animation/, wwwroot/codebrix-frontends
      .toml, wwwroot/THIRD-PARTY-NOTICES.txt, wwwroot/SAMPLE-THIRD-PARTY-
      NOTICES.txt               BUILD OUTPUT (never committed)
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
      src/model_scene.rs, src/model_layer.rs
                                graphics: the "Skinned model" and "Morph
                                targets" scenes and their one layer
      src/more_scene.rs, src/more_layer.rs, src/more_cubes.wgsl,
      src/more_compute.wgsl, src/more_particles.wgsl
                                graphics: the "More" scene, its layer and
                                its three shaders
      src/page_cache.rs         graphics: what the page keeps of what it
                                fetched
      src/demo/mod.rs           animation: the scene catalogue (the scenes,
                                their lengths, the named points of their
                                starts, the timeline)
      src/demo/page.rs          animation: the page - the canvas, the one
                                AnimationLayer, the clock, the keys, the
                                buttons and read-outs, the sample-points check
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
      src/demo/on_page.rs       animation: the ?scene=on-the-page page
      src/page_scene.rs         graphics: the "On the page" scene
      src/generated/            BUILD OUTPUT: the client code generated from
                                the backend
      platform/                 BUILD OUTPUT: the shared Rust crate; in
                                graphics/ also the vendor folder with the
                                platform's crates and the bundles' (wgpu,
                                glam, codebrix-webgpu, codebrix-webgpu-
                                graphics ...), which animation
                                uses from there
      .cargo/config.toml        BUILD OUTPUT: tells cargo to build from that
                                vendor folder, offline
      assets/tailwind.css       BUILD OUTPUT: the generated stylesheet
      target/                   BUILD OUTPUT: cargo's and dx's


BUILDING AND RUNNING THE SAMPLE
===============================
What must be installed (the build program checks and tells you; it installs
nothing): the .NET 10 SDK; Rust at the platform's minimum version or later
with the WebAssembly target (`rustup target add wasm32-unknown-unknown`); the
standalone Tailwind program on PATH. dx and wasm-bindgen come prebuilt in the
host-tools package.

From samples/GraphicsDemo:

    dotnet run --project build/GraphicsDemo.BuildFrontend -- check
    dotnet run --project build/GraphicsDemo.BuildFrontend -- generate
    dotnet run --project build/GraphicsDemo.BuildFrontend -- build
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
  build     generate, then build both with dx into the backend's wwwroot
            (with the manifest), and write the site's notices there.
  run       build, then start the backend at http://localhost:5220/ (the
            page at /, graphics at /graphics, animation at /animation).
            Ctrl+C stops it.
  serve     generate, then `dx serve` with hot reload for ONE application,
            named.

The backend alone builds with plain dotnet: `dotnet build GraphicsDemo.slnx`.
Restart the backend after a new build of the frontends: it reads the
manifest once, when it starts.

A FRESH CLONE: the frontends do not compile until `generate` has written
src/generated, platform/ and .cargo/. So: clone, `generate`, `cargo check`.


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
adapter-name, frame-count, webgpu-canvas, webgpu-unavailable; on graphics
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
    handful of colours, changes while the clock runs, does NOT change while
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
    viewer-distance, viewer-blob-bytes, viewer-models, viewer-instances,
    viewer-triangles, viewer-draw-calls, viewer-drawn, viewer-pipelines,
    viewer-error-count): the network shows api/models/scene (200, JSON) and
    api/models/scene.cbmd ONCE (200, application/octet-stream, its
    Content-Length = viewer-blob-bytes); viewer-instances is the ground, the
    21 props and the 10 actors; the canvas is not blank;
    running, viewer-yaw grows; paused, viewer-yaw and every pixel stay the
    same while viewer-frame goes on; a rightward drag lowers viewer-yaw
    (0.008 per pixel) and turns the mode "user"; a drag down raises
    viewer-pitch; a wheel step towards the visitor raises viewer-distance by
    1.1 times; "orbit by itself again" and a resume make viewer-yaw grow;
    viewer-error-count is 0;
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
    -pipelines, -error-count; skinned-clip-select): one request
    api/models/character-archer.cbmd or api/models/morph-shape.cbmd (200);
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
  - THE PAGE CACHE: switching Viewer, Skinned model, Morph targets, More
    and back again (and the Viewer's panorama off and on) requests
    api/models/scene, api/models/scene.cbmd, character-archer.cbmd,
    morph-shape.cbmd and small-cathedral.hdr ONCE each. A page RELOAD
    requests the Viewer's two again and gets 200 with the whole body: the
    platform's client fetches with the browser's HTTP cache off (cache:
    "no-store"), so it never sends the ETag back, although the backend
    answers a request that does (If-None-Match) with 304;
  - WITHOUT an adapter (Chrome without the switch): both applications show
    "WebGPU is not available in this browser" and the console has no
    uncaught error (Chrome itself warns "No available adapters.");
  - the console shows no errors and no Content-Security-Policy violations,
    and no request fails (the one expected console entry is Chrome's
    warning for the Sprites scene's deliberate error, once its check button
    has been pressed);
  - the icon is served for all three: /favicon.ico,
    /graphics/assets/favicon.ico, /animation/assets/favicon.ico.


THE MODELS: GraphicsDemo.Assets AND api/models
==============================================
The models of the "graphics" application's Viewer scene stay on the backend
and travel to the browser as model data blobs (the model data format of
CodeBrix.WebPlatform.WebGpu.Graphics, ".cbmd"). Nothing in the browser reads
glTF or decodes images.

  - GraphicsDemo.Assets holds the .glb files and each pack's
    Textures/colormap.png and licence file as EMBEDDED RESOURCES (resource
    name = the path below Models/). Only third-party files whose source and
    licence are recorded are carried, and THIRD-PARTY-NOTICES.txt beside
    this file lists them; the owner's own pack (crystals-and-avian) needs
    neither. A new model goes into its pack's folder with an entry there.
  - Conversion/GltfModelConverter mirrors the CPU half of the C# library's
    glTF loader (CodeBrix.WebGpu.Graphics, its internal GltfModelLoader) step
    for step, with its defaults: the same nodes, materials, texture slots,
    samplers, vertices, indices, generated normals and tangents, skins,
    morph targets, animations and warnings. Keep it in step with that loader.
    Textures are decoded to RGBA8 level 0 and marked for mipmaps; the graphics
    crate makes the mip chain (on the CPU) when it creates the texture. ModelTextureCache stores an external
    image in one colour space ONCE per set, so a pack's shared colormap is
    one texture in the scene blob. GroundModel builds the ground exactly as
    the desktop viewer does (a glTF document made in memory, then converted).
    MorphShapeModel BUILDS the model of the "Morph targets" scene in code, as
    model data directly (no glTF, no asset file): a unit sphere with three
    morph targets (position and normal deltas: stretch, twist, bumps) and the
    weights clip "morph"; ModelLibrary serves it as "morph-shape".
  - Scene/ViewerSceneLayout is the desktop viewer's scene: the actors (the
    chest, the eight animals and the Avian, as Scene/ViewerActor: model, clip
    or routine, time offset, motion and its numbers, and WorldAt(t), the
    desktop's placement function), the routine and cross-fade times, the
    props table and the camera and lighting values (the sun, the procedural
    sky's colours and turbidity, the exposure under the HDR panorama). The
    scene blob holds the ground and every model the scene places
    (SceneModels: the actors' models, then the props'); the scene JSON
    carries the actors (Contracts/SceneActor).
  - Models/kenney-mini-forest/ holds the Kenney archer (character-archer.glb,
    a skinned model) of the "Skinned model" scene, its colour map and its
    licence file, copied byte for byte from the CodeBrix.WebGpu repository's
    test assets (Kenney Mini Forest, CC0, recorded there). It is converted
    like every other model and served alone as api/models/character-archer.cbmd.
  - ModelLibrary converts each blob on first use, keeps it (bytes and a
    strong ETag, the SHA-256 of the bytes) and is safe to share; the backend
    registers ONE as a singleton.
  - GraphicsDemo.Assets takes CodeBrix.WebPlatform.WebGpu.Graphics as a
    PACKAGE at the same version as the build program, with its build and
    buildTransitive assets excluded: a backend never builds a frontend, so
    the crate archives the package copies into a build program's folder stay
    out of the backend's.
  - Controllers/ModelsController (api/models, anonymous):
      GET api/models/scene        the scene description (JSON, Contracts/)
      GET api/models/scene.cbmd   [WireFile] SceneData: the scene blob
      GET api/models/{name}.cbmd  [WireFile] ModelData: one model's blob, or
                                  404 with the standard problem body
    Blobs are answered as application/octet-stream with an ETag and
    "Cache-Control: public, no-cache": a cache may keep them but asks again
    each time, so a rebuilt backend is never answered from a copy that no
    longer agrees with the scene description; a matching If-None-Match is
    answered 304 with no body. The browser never sends one, though: the
    platform's client (codebrix-webplatform, api/client.rs) fetches every
    request with cache "no-store", which bypasses the HTTP cache, so a page
    reload downloads each blob in full again; within a page, the frontend's
    page_cache.rs keeps what it fetched. The answers of these [WireFile]
    actions are compressed as they are sent (Program.cs sets the platform's
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
    AnimationDemoApp draws all its text with) and Fonts/open-sans/
    (OpenSans-Regular.ttf, for the missing-characters scene - its .notdef has
    no ink - and the SVG text of svg-features), each with its ORIGINAL
    OFL.txt, copied byte for byte from the family's font repositories
    (CodeBrix.Platform.Fonts.Roboto and CodeBrix.Platform.Fonts.OpenSans;
    THIRD-PARTY-NOTICES.txt beside this file names the files), under the SIL
    Open Font License 1.1. FontLibrary reads them on first use and keeps them
    with a strong ETag; Controllers/FontsController serves them unchanged:
      GET api/fonts/roboto-regular.ttf      [WireFile] RobotoRegular
      GET api/fonts/open-sans-regular.ttf   [WireFile] OpenSansRegular
    The animation crate reads and shapes them in the browser. OFL fonts are
    allowed in samples and tests only: never move them into a package.
  - The tests (tests/GraphicsDemo.Assets.Tests) check the caption file (the
    four cues, kept once, a strong ETag), the SVG documents (each embedded
    SVG text, kept once, a strong ETag, an unknown name refused), the Lottie
    files (each one's JSON and width, the image-layer file's badge asset,
    kept once, a strong ETag, an unknown name refused), the pictures (the
    colour map's pixel form - a 512 x 512 header and length, kept once, a
    strong ETag; its PNG file's signature; the badge made in code, opaque
    inside and transparent at its corners; a buffer that does not match its
    size is refused), a few models against
    the counts the desktop viewer's files have, the ground, the shared
    textures of the scene set, the blob header, that every embedded model
    converts with no warning, the actors' placements against the desktop's
    formulas, that every actor's clips exist in its model, the archer's skins
    and clips, and the morph shape (its sphere, the three targets, the
    weights clip, and that it is written the same way every time).


TEMPORARY PACKAGES
==================
The sample uses CodeBrix packages that are not on nuget.org yet: the
Backend package (backend/GraphicsDemo.Api), FrontendBuilder with the
host-tools package of the machine, and this repository's own WebGpu,
Graphics and Animation packages (build/GraphicsDemo.BuildFrontend; the
Graphics and Animation packages also in backend/GraphicsDemo.Assets). They come from the ONE temporary feed at
the repository root - nuget.config and temp-packages/. There is no
nuget.config in samples/GraphicsDemo.

  - The CodeBrix.WebPlatform version is set in WebPlatformPackageVersion in
    both project files; the ONE version of this repository's packages
    (WebGpu, Graphics and Animation) in WebGpuPackageVersion in the build program's
    project file and in backend/GraphicsDemo.Assets/GraphicsDemo.Assets.csproj
    (the two must name the same version).
  - The sample does not see a change to a library of this repository until
    the feed is refreshed: TEMPORARY PACKAGES in the root
    MAINTAINER-README.txt says how.

When the packages are on nuget.org the feed is deleted, and this section
with it.


GIT
===
Never commit: the backend's built wwwroot content (graphics/, animation/,
the manifest, THIRD-PARTY-NOTICES.txt) - its index.html, site.css and
favicon.ico ARE committed - any bin/, obj/ or target/ folder, and what is
written into each frontend: src/generated, platform/, .cargo/ and
assets/tailwind.css. The .gitignore at the repository root covers all of
them. Each frontend's Cargo.lock IS committed: it is that application's own.


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
