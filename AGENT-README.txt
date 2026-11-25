================================================================================
AGENT-README: CodeBrix.WebPlatform.WebGpu
A Guide for AI Coding Agents — CONSUMING the
CodeBrix.WebPlatform.WebGpu.ApacheLicenseForever,
CodeBrix.WebPlatform.WebGpu.Graphics.ApacheLicenseForever and
CodeBrix.WebPlatform.WebGpu.Animation.ApacheLicenseForever NuGet packages
================================================================================


OVERVIEW
========

These packages bring WebGPU to the browser frontends of CodeBrix.WebPlatform
applications - for showing 3D graphics, scenes and animations in the browser
window. A CodeBrix.WebPlatform frontend is written in Rust and compiled to
WebAssembly; these packages bring it Rust crates, as CRATE BUNDLES (the
platform's mechanism by which a NuGet package brings Rust crates to a
frontend: see CRATE BUNDLES in the AGENT-README.txt of the
CodeBrix.WebPlatform packages).

This file documents every package of this repository. They target .NET 10
(net10.0).

    CodeBrix.WebPlatform.WebGpu
        Carries, as one archive with their generated third-party notices:
          * the Rust `wgpu` crate for WebGPU in the browser and the `glam`
            maths crate, the `png` crate for PNG files and the `zune-jpeg`
            crate for JPEG files, and every crate they need, exactly as
            crates.io published them;
          * the crate `codebrix-webgpu` of this repository (see THE
            codebrix-webgpu CRATE): the WebGPU context, ONE canvas host
            component with its frame loop, layers and clock, the input, the
            "WebGPU is not available" result and message, the maths types,
            and the decoding of PNG and JPEG files into pixels. It is not on
            crates.io; it travels only in this archive.
        Its ONE call, WebGpuCrates.Register(), registers them with the
        platform's frontend build; the build then writes them into the
        frontend beside the platform's own crates, so that the frontend
        still builds offline.

    CodeBrix.WebPlatform.WebGpu.Graphics
        Carries the Rust crate `codebrix-webgpu-graphics` of this repository
        (see THE codebrix-webgpu-graphics CRATE) and the third-party crates
        only it needs (`gltf` for glTF documents, and what it brings), as
        one archive with their notices: 2D and 3D graphics in a
        platform frontend through WebGPU, with the type names, concepts and
        argument order of the .NET library CodeBrix.WebGpu.Graphics, drawing
        as a LAYER of the `codebrix-webgpu` canvas host. Its purpose and
        scope: what makes sense in a browser of 2D sprites and shapes, lit 3D
        meshes, models with physically based shading, skinning, morph targets
        and animation playback, image-based lighting with a skybox, tone
        mapping, animated textures, render bundles, indirect draws, compute,
        timestamp queries and texture readback; what a browser has no
        equivalent for is listed in that section in one place. Beyond the
        .NET library: glTF files read in the browser, textures from PNG and
        JPEG files, frustum culling and level of detail, point lights and
        spotlights, shadows of the sun and of spotlights, multisampled
        anti-aliasing, bloom, a glow layer, GPU particles, frame timings,
        water (a simulated wave field, open-water waves and a water surface),
        panning and framing in the orbit controls, and picking. Its ONE
        call, GraphicsCrates.Register(), registers the WebGpu package's
        crates FIRST and then its own; its assembly also holds the crate's
        names (GraphicsRustCrate).

    CodeBrix.WebPlatform.WebGpu.Animation
        Carries the Rust crate `codebrix-webgpu-animation` of this repository
        (see THE codebrix-webgpu-animation CRATE) and the third-party crates
        only it needs (`kurbo` for curves and strokes, `peniko` for brushes
        and blend modes, `harfrust` for text shaping and glyph outlines,
        `i_overlay` for path combining, `usvg` for SVG
        documents, `velato` for Lottie animations, and what they bring), as
        one archive with their notices: REAL-TIME 2D animation that the user
        sees in the browser window, through WebGPU (no offline rendering of
        frames), with the names and concepts of the animation toolkit of the
        .NET library CodeBrix.Imaging.Drawing.NoSkia - a drawing session of
        calibrated units, layers of shapes, text lines and laid-out text
        blocks in fonts given as bytes (reveals, staggers, captions, text
        along a path), gradient fills and fades, the animation model (tracks
        bound to elements, layers and a camera, combined bindings), paths
        (draw-on, motion, rolling and trains, morphs, combining and
        queries), pictures and sprites (decoded pixels, PNG or JPEG files),
        masks and effects, pixel planes with their real-time GPU versions
        (whole-frame operations, transitions, noise, particles), animated
        brush strokes and painterly playback, formations, safe-area guides,
        easing curves, colour ramps, timelines, a playback controller and the
        debug overlay - and, beyond NoSkia, SVG documents that animate
        themselves (SMIL and CSS), Lottie animations, render-time looks
        (session blend modes and opacity, line and flat paint) with wipes
        between renders, fixed-width and sketched strokes, painterly stroke
        sets of tens of thousands of brushstrokes in one GPU draw with the
        tools to plan and colour them, and water (ripple fields seen
        through as refraction, reflections, caustics); all drawn by the
        crate's own renderer as a LAYER of the `codebrix-webgpu` canvas
        host. Its ONE call,
        AnimationCrates.Register(), registers the WebGpu package's crates
        FIRST and then its own; its assembly also holds the crate's names
        (AnimationRustCrate).

THE DEPENDENCY RULE OF THE THREE PACKAGES
    * CodeBrix.WebPlatform.WebGpu depends on CodeBrix.WebPlatform.Core, at
      EXACTLY the platform version its crates were chosen against.
    * Graphics and Animation each depend on CodeBrix.WebPlatform.Core (the
      same exact pin) AND on CodeBrix.WebPlatform.WebGpu, EXACTLY the version
      of the same build: the three packages are built and published together
      as one family with one version.
      Neither depends on the other: an application may use either one, or
      both. Both build on `codebrix-webgpu`; neither carries a copy of the
      wgpu crates. The application, not either package, puts their
      renderers together (see LAYERS AND THE CLOCK).

THE RULES THAT HOLD FOR ALL THREE
    * WebGPU ONLY. There is no WebGL fallback and no native backend in the
      crates: a browser without WebGPU gets no adapter, and the frontend
      shows a clear "WebGPU is not available in this browser" message.
    * LOW-LEVEL wgpu ACCESS ALWAYS STAYS AVAILABLE. A frontend may always use
      the `wgpu` API directly, beside anything built on it:
      `codebrix-webgpu` hands out the device, the queue, the surface and
      every frame's texture, and re-exports `wgpu` itself.
    * Use them with the CodeBrix.WebPlatform packages of the platform
      version they pin.


INSTALLATION
============

    dotnet add package CodeBrix.WebPlatform.WebGpu.ApacheLicenseForever

Add it to the application's BUILD PROGRAM - the program that references
CodeBrix.WebPlatform.FrontendBuilder and materializes the frontend. That is
where the crates are needed: the package copies its webgpu-crates/ folder
(the archive and the notices) into the build program's output folder, and the
registration call opens the archive from there. The backend and the frontend
do not reference it. A build program that uses the graphics crate references
CodeBrix.WebPlatform.WebGpu.Graphics.ApacheLicenseForever (it brings the WebGpu
package with it, as its dependency, and copies its own webgpu-graphics-crates/
folder beside the program); one that uses the animation crate references
CodeBrix.WebPlatform.WebGpu.Animation.ApacheLicenseForever the same way (it
copies webgpu-animation-crates/). A program may reference both.

    NuGet package ID:  CodeBrix.WebPlatform.WebGpu.ApacheLicenseForever
    Assembly and primary namespace:  CodeBrix.WebPlatform.WebGpu

    NuGet package ID:  CodeBrix.WebPlatform.WebGpu.Graphics.ApacheLicenseForever
    Assembly and primary namespace:  CodeBrix.WebPlatform.WebGpu.Graphics

    NuGet package ID:  CodeBrix.WebPlatform.WebGpu.Animation.ApacheLicenseForever
    Assembly and primary namespace:  CodeBrix.WebPlatform.WebGpu.Animation

There are no packages named plain CodeBrix.WebPlatform.WebGpu,
CodeBrix.WebPlatform.WebGpu.Graphics or CodeBrix.WebPlatform.WebGpu.Animation.

DEPENDENCIES: see THE DEPENDENCY RULE above. The build program already
references CodeBrix.WebPlatform.FrontendBuilder, which brings Core; take the
FrontendBuilder package of the same version as the Core these packages pin,
or NuGet reports a version conflict at restore.

LICENSE: Apache-2.0 for the code of this repository, `codebrix-webgpu`,
`codebrix-webgpu-graphics` and `codebrix-webgpu-animation` included (three
tone-mapping functions of the graphics crate and the logic of its Radiance
.hdr decoder follow outside sources, listed with their licences in
THIRD-PARTY-NOTICES-WebGpuGraphicsCrates.txt; the animation crate's renderer
contains code taken from vello and, inside it, a few lines from Skia, listed
in THIRD-PARTY-NOTICES-WebGpuAnimationCrates.txt). The third-party crates the
three packages carry keep their own permissive licences; they are listed
with their licence texts in THIRD-PARTY-NOTICES-WebGpuCrates.txt,
THIRD-PARTY-NOTICES-WebGpuGraphicsCrates.txt and
THIRD-PARTY-NOTICES-WebGpuAnimationCrates.txt (see NOTICES FOR A BUILT
SITE).

REQUIREMENTS: those of a CodeBrix.WebPlatform application (the .NET SDK, Rust
with its WebAssembly target). Nothing more is installed for WebGPU. Running
the frontend needs a browser with WebGPU and a GPU adapter it will hand out.


KEY NAMESPACES / USINGS
=======================

    using CodeBrix.WebPlatform.WebGpu;            // WebGpuCrates
    using CodeBrix.WebPlatform.WebGpu.Graphics;   // GraphicsCrates,
                                                  // GraphicsRustCrate
    using CodeBrix.WebPlatform.WebGpu.Graphics.Models;
                                                  // the model data and its
                                                  // writer (a backend)
    using CodeBrix.WebPlatform.WebGpu.Animation;  // AnimationCrates,
                                                  // AnimationRustCrate

In the build program, beside them:

    using CodeBrix.WebPlatform.FrontendBuilder;   // PlatformFrontend, options
    using CodeBrix.WebPlatform.Core.Crates;       // CrateBundles (only to
                                                  // look at what is
                                                  // registered)

In the frontend's Rust:

    use codebrix_webgpu::{set_page_gpu_options, use_gpu, CanvasClock,
                          CanvasFrame, CanvasLayers, CanvasRenderer,
                          CanvasTarget, GpuContext, PageGpuOptions,
                          RendererHandle, WebGpuBoundary, WebGpuCanvas,
                          WebGpuUnavailable};
    use codebrix_webgpu::wgpu;                    // the whole wgpu API
    use codebrix_webgpu::math::{Mat4, Quat, Vec3};// glam's types
    use codebrix_webgpu::{decode_png, decode_jpeg, decode_image,
                          DecodedImage};          // image files to pixels
    use codebrix_webgpu_graphics::{GraphicsContext, Color4, BindingEntry,
        BindingLayoutEntry, BlendPreset, ColorTargetOptions, DrawOptions,
        RgbaTextureOptions, SamplerDescription};  // the graphics crate
    use codebrix_webgpu_graphics::{GltfDocument, GltfLoadOptions,
        GltfModelSet, ModelRenderer, SceneLighting};
                                                  // glTF files, models
    use codebrix_webgpu_graphics::shaders::BuiltInShaders;
    use codebrix_webgpu_animation::{Color, DrawingSession,
        DrawingSessionOptions, PointF};           // the animation crate
    use codebrix_webgpu_animation::animation::{AnimationTimeline, ColorRamp,
        Easing, EasingKind, PlaybackController};
    use codebrix_webgpu_animation::shapes::{CircleShape, RectangleShape,
        RectangleShapeOptions, ShapeOptions};
    use codebrix_webgpu_animation::rendering::{AnimationLayer, VectorScene};


CORE API REFERENCE: WebGpuCrates
================================

CodeBrix.WebPlatform.WebGpu. The crates this package carries, and the call
that registers them.

    public static class WebGpuCrates
    {
        public const string BundleName = "webgpu";
            // the crate bundle's name, unique in a build
        public const string PackageId =
            "CodeBrix.WebPlatform.WebGpu.ApacheLicenseForever";
        public const string FolderName = "webgpu-crates";
        public const string ArchiveFileName = "webgpu-crates.zip";
        public const string NoticesFileName =
            "THIRD-PARTY-NOTICES-WebGpuCrates.txt";

        public static void Register();
            // hands CrateBundles.Register a CrateBundle named "webgpu", from
            // this package id and this package's own version, whose archive
            // is <build program folder>/webgpu-crates/webgpu-crates.zip
            // (opened when the frontend build needs it). Calling it again
            // changes nothing. InvalidOperationException when another
            // package, or another version of this one, already registered a
            // bundle named "webgpu".

        // each takes (string applicationBaseDirectory = null): the build
        // program's folder; null = AppContext.BaseDirectory of the program
        public static string GetDirectory(...);    // <it>/webgpu-crates
        public static string GetArchivePath(...);  // .../webgpu-crates.zip
        public static string GetNoticesPath(...);  // .../THIRD-PARTY-NOTICES-..
        public static string ReadNotices(...);     // the text (UTF-8, line
            // feeds); FileNotFoundException, saying what to do, when the
            // file is missing
    }

WHEN THE ARCHIVE IS MISSING. Register() itself never touches the file. When
the frontend build opens the archive and it is not there, the build fails
with a FileNotFoundException whose message names the path and says what to
do: reference the package from the build program, then restore and build the
build program again. (A program copied without its webgpu-crates folder fails
the same way.)


CORE API REFERENCE: GraphicsCrates
==================================

CodeBrix.WebPlatform.WebGpu.Graphics. The graphics crate, and the call that
registers it with the WebGpu crates it builds on. The same shape as
WebGpuCrates:

    public static class GraphicsCrates
    {
        public const string BundleName = "webgpu-graphics";
        public const string PackageId =
            "CodeBrix.WebPlatform.WebGpu.Graphics.ApacheLicenseForever";
        public const string FolderName = "webgpu-graphics-crates";
        public const string ArchiveFileName = "webgpu-graphics-crates.zip";
        public const string NoticesFileName =
            "THIRD-PARTY-NOTICES-WebGpuGraphicsCrates.txt";

        public static void Register();
            // calls WebGpuCrates.Register() FIRST, then registers the bundle
            // "webgpu-graphics" (codebrix-webgpu-graphics and the crates only
            // it needs - gltf, gltf-json, gltf-derive and what they bring
            // that neither the platform nor the WebGpu package carries; the
            // wgpu crates and codebrix-webgpu come from the WebGpu bundle).
            // Calling it again changes nothing; calling
            // WebGpuCrates.Register() as well changes nothing either.

        public static string GetDirectory(...);    // <program>/webgpu-graphics-crates
        public static string GetArchivePath(...);  // .../webgpu-graphics-crates.zip
        public static string GetNoticesPath(...);
        public static string ReadNotices(...);     // serve with the site
    }


CORE API REFERENCE: AnimationCrates
===================================

CodeBrix.WebPlatform.WebGpu.Animation. The animation crate and the
third-party crates only it needs, and the call that registers them with the
WebGpu crates they build on. The same shape as WebGpuCrates:

    public static class AnimationCrates
    {
        public const string BundleName = "webgpu-animation";
        public const string PackageId =
            "CodeBrix.WebPlatform.WebGpu.Animation.ApacheLicenseForever";
        public const string FolderName = "webgpu-animation-crates";
        public const string ArchiveFileName = "webgpu-animation-crates.zip";
        public const string NoticesFileName =
            "THIRD-PARTY-NOTICES-WebGpuAnimationCrates.txt";

        public static void Register();
            // calls WebGpuCrates.Register() FIRST, then registers the bundle
            // "webgpu-animation" (codebrix-webgpu-animation, kurbo, peniko,
            // harfrust, i_overlay, usvg, velato and the crates they
            // bring that neither the platform nor the WebGpu package
            // carries; the wgpu crates and codebrix-webgpu come from the
            // WebGpu bundle). Calling it again changes nothing; calling
            // GraphicsCrates.Register() as well registers the WebGpu bundle
            // only once.

        public static string GetDirectory(...);    // <program>/webgpu-animation-crates
        public static string GetArchivePath(...);  // .../webgpu-animation-crates.zip
        public static string GetNoticesPath(...);
        public static string ReadNotices(...);     // serve with the site
    }


CORE API REFERENCE: MODEL DATA (the .NET writer)
================================================

CodeBrix.WebPlatform.WebGpu.Graphics.Models. The CPU-side description of
models that the crate's reader (ModelDataSet::read, ctx.load_models) takes,
and the WRITER of its one binary blob. It is meant for a backend (or any
.NET tool) that makes or converts models. A glTF file needs none of it: the
crate reads .gltf and .glb files itself, in the browser (LOADING glTF
MODELS). The .NET package reads no glTF and decodes no image - the
application fills the types; the GraphicsDemo sample builds two models in
code this way (samples/GraphicsDemo/backend/GraphicsDemo.Assets/
BuiltModels: a ground and a morph-target shape). The types mirror what
the C# library's loader produces; mutable classes with C#'s defaults:

    ModelDataSet       Models (List<ModelData>), Textures (List<ModelTextureData>:
                       each texture once, however many models use it)
    ModelData          Name, Vertices (ModelVertex[]), Indices (uint[]),
                       IndexFormat (ModelIndexFormat.Uint16 / Uint32), SkinVertices
                       (ModelSkinVertex[] or null), Meshes, Materials,
                       DefaultMaterial, Nodes, RootNodes, Skins, Animations,
                       Textures (List<int>: indices into the set's Textures),
                       Samplers (List<ModelSamplerData>), Warnings
    ModelVertex        64 bytes (Position, Normal, Tangent, TexCoord0, TexCoord1,
                       ColorR..ColorA as Unorm16); ModelSkinVertex 24 bytes
    ModelMeshData      Name, Primitives, DefaultMorphWeights
    ModelPrimitiveData Material (-1 = default), FirstIndex, IndexCount,
                       BaseVertex, VertexCount, SkinBaseVertex (-1), BoundsMin/Max,
                       HasVertexColors, HasTexCoord1, HasTangents,
                       TangentsGenerated, NormalsGenerated, MorphTargets
    ModelMorphTargetData  PositionDeltas, NormalDeltas, TangentDeltas
    ModelMaterialData  the complete glTF metallic-roughness set and the
                       extensions the C# library captures (unlit, emissive
                       strength, ior, specular, clear coat, transmission),
                       ApproximatedFeatures; slots are ModelTextureSlotData
                       (Texture and Sampler model-local, -1 = none; TexCoord,
                       HasTransform, Transform as Matrix3x2, TextureIndex)
    ModelTextureData   Name, Width, Height (1..8192), ColorSpace
                       (TextureColorSpace.Srgb / Linear), Mipmaps, Pixels (RGBA8)
    ModelSamplerData   AddressModeU/V, MagFilter, MinFilter, MipmapFilter, MaxLod
    ModelNodeData      Name, Parent, Children, Mesh, Skin, Translation, Rotation,
                       Scale, HasFixedMatrix, LocalMatrix, DefaultMorphWeights,
                       InScene
    ModelSkinData      Name, Joints, InverseBindMatrices, Skeleton
    ModelAnimationData Name, Channels (ModelAnimationChannelData: Node, Path,
                       Interpolation, ComponentCount, Times, Values)

    public static class ModelDataWriter
    {
        public static byte[] Write(ModelDataSet set);
        public static byte[] Write(ModelData model, IReadOnlyList<ModelTextureData> textures);
        public static void Write(ModelDataSet set, Stream destination);
            // every reference, range and size is checked first; an
            // ArgumentException (or InvalidOperationException) says which
            // record is wrong and what to set instead. Deterministic output.
    }

    public static class ModelDataFormat
    {
        public static ReadOnlySpan<byte> Magic;    // "CBMD"
        public const int MajorVersion = ..;        // the reader refuses another
        public const int MinorVersion = ..;
        public const string FileExtension = ".cbmd";
        public const string ContentType = "application/octet-stream";
        public const int MaxTextureDimension = 8192;
    }

SERVING IT (the sample's way): a [WireFile] GET action answers with
File(bytes, ModelDataFormat.ContentType, ...), ideally with an ETag (the
blob does not change while the backend runs); the frontend calls the
generated <action>_bytes function. A blob with decoded textures is large
(four bytes per texel) but compresses very well: with the backend's
CompressResponses = FileAnswers the platform compresses such an answer
(application/octet-stream is in its CompressedFileTypes; see the
platform's AGENT-README).


CORE API REFERENCE: GraphicsRustCrate AND AnimationRustCrate
============================================================

    public static class GraphicsRustCrate    // ...WebGpu.Graphics
    {
        public const string CrateName = "codebrix-webgpu-graphics";
            // as a frontend's Cargo.toml names it
        public const string RustName = "codebrix_webgpu_graphics";
            // as Rust source names it
    }

    public static class AnimationRustCrate   // ...WebGpu.Animation
    {
        public const string CrateName = "codebrix-webgpu-animation";
        public const string RustName = "codebrix_webgpu_animation";
    }

The crates are registered by GraphicsCrates.Register() and
AnimationCrates.Register(); these two types only name them.


THE BUILD PROGRAM SIDE
======================

    1. Reference CodeBrix.WebPlatform.WebGpu.ApacheLicenseForever from the
       build program - or, for the graphics crate,
       CodeBrix.WebPlatform.WebGpu.Graphics.ApacheLicenseForever (it brings
       the WebGpu package with it).
    2. Call WebGpuCrates.Register() - or GraphicsCrates.Register(), which
       registers the WebGpu crates first - in the build program BEFORE
       MaterializeAsync. Nothing registers itself: a package that is only
       referenced adds no crate.
    3. Materialize as usual. MaterializeResult.CrateBundles has one
       CrateBundleOutcome per bundle: "webgpu" (AddedCrates lists the
       crates it added, codebrix-webgpu among them; SharedCrates the ones the
       platform already carries, identical, so one copy is kept) and, with
       the Graphics package, "webgpu-graphics" (codebrix-webgpu-graphics and
       the glTF crates only it needs).

In the default crate mode (CrateMode.PlatformCratesOnly) the crates are
unpacked into the frontend's vendor folder beside the platform's, and the
frontend builds offline.

A LIMIT OF CrateMode.ApplicationAddsCrates: in that mode nothing is unpacked
and cargo fetches every crate from crates.io. `wgpu` and `glam` are there
(pin them to the versions this package carries, with exact requirements or
a committed Cargo.lock), but `codebrix-webgpu` is NOT: it is not published on
crates.io and exists only in this package's archive. So an application in
CrateMode.ApplicationAddsCrates cannot use `codebrix-webgpu`, nor any crate
built on it. Stay in the default mode to use it.

The platform's strict rule for crate bundles holds: a crate that the
platform, or another registered bundle, carries in a different version stops
the build with one FrontendBuildException that lists every such crate. This
package's crates are chosen so that they meet the rule with the platform
version it pins; use the CodeBrix.WebPlatform packages of that version.


THE FRONTEND SIDE
=================

The frontend names `codebrix-webgpu` in its own Cargo.toml - and, for its own
components, dioxus as the platform's frontends do - and, to draw with the
graphics crate, `codebrix-webgpu-graphics`:

    [dependencies]
    codebrix-webgpu = "..."
    codebrix-webgpu-graphics = "..."

(the version the package carries: the codebrix-webgpu folder in
webgpu-crates.zip names it). `codebrix-webgpu` re-exports `wgpu` and `glam` (as `codebrix_webgpu::wgpu`,
`codebrix_webgpu::glam` and the maths types in `codebrix_webgpu::math`), so
the frontend need not name them. A frontend that names `wgpu` itself (for
example to use it without `codebrix-webgpu`) uses the version and features
the package carries:

    wgpu = { version = "=...", default-features = false,
             features = ["std", "webgpu", "wgsl"] }

    * default-features = false is REQUIRED. The default features switch on
      the native backends (dx12, metal, gles, vulkan), whose crates the
      bundle does not carry, so an offline build would stop with "no
      matching package".
    * The version is the one the package carries (the wgpu folder in
      webgpu-crates.zip, or THIRD-PARTY-NOTICES-WebGpuCrates.txt, names it).
    * Name those three features. Others either need crates the bundle does
      not carry - webgl (there is no WebGL fallback), spirv, glsl, serde,
      trace, exhaust: an offline build stops with "no matching package" - or
      compile the naga shader translator into the browser build (naga-ir).
    * Shaders are WGSL (the `wgsl` feature). The browser compiles them.
    * `glam` is carried with its default features and `bytemuck`; a frontend
      that names it itself takes no other feature (its other optional
      dependencies are not carried).


THE codebrix-webgpu CRATE
=========================

GETTING A DEVICE, AND THE "NOT AVAILABLE" MESSAGE.
    GpuContext::request().await -> Result<GpuContext, GpuUnavailable>
        Asks the browser for a WebGPU adapter ONCE and opens a device on it.
        Do not probe first with wgpu::util::is_browser_webgpu_supported():
        that probe asks for an adapter itself, so the browser asks twice
        (Chrome then warns twice). The one request covers every case: no
        `navigator.gpu`, WebGPU switched off, or no adapter handed out all
        give GpuUnavailable::NoAdapter.
    GpuContext::request_with(&adapter_options, &device_descriptor)
        The same with the application's own wgpu::RequestAdapterOptions and
        wgpu::DeviceDescriptor (features, limits); a device the adapter will
        not open gives GpuUnavailable::NoDevice(the browser's reason).
    GpuContext::from_parts(instance, adapter, device, queue)
        SHARES a device the application already has.
    context.instance() / adapter() / device() / queue() / adapter_info()
        Clone a GpuContext to share it (the clones are the same device).
        In a browser adapter_info() says little: Chrome gives no description
        on purpose; device_type "Other" means "not a fallback adapter".
    GpuUnavailable::TITLE  "WebGPU is not available in this browser"
    reason.advice()        one sentence of what the visitor can do
    WebGpuUnavailable { reason, class, heading_class, text_class }
        The message as a component (id `webgpu-unavailable`, role alert).

A PART OF THE PAGE ONLY WHERE WEBGPU WORKS: WebGpuBoundary.
    WebGpuBoundary { HeroAnimation {} }
        Its children are made ONLY where WebGPU works. Elsewhere it shows
        nothing: the children never run, so they load nothing and show no
        message (an optional animation simply is not there).
    WebGpuBoundary { fallback: |_| rsx! { img { src: "/still.webp" } }, HeroAnimation {} }
        The same, with something in its place where WebGPU is not
        available. The closure is given the reason (GpuUnavailable), the
        same shape as Dioxus's own SuspenseBoundary { fallback: |_| ... }.
    let gpu = use_gpu();
        In a component inside a boundary: the page's GpuContext (clone it
        freely). It panics outside a boundary.
    While the browser is still answering (usually a few milliseconds) the
    boundary shows nothing, never the fallback first; give the space around
    it a fixed size if the page must not move. EVERY BOUNDARY OF THE PAGE
    SHARES ONE REQUEST and one device: the first asks, the others wait for
    the same answer or take it at once.
    set_page_gpu_options(PageGpuOptions { .. })
        How that one request asks; by default exactly as GpuContext::request
        does. Fields: power_preference; required_features (missing on the
        adapter -> NoDevice, so the fallbacks show); optional_features
        (asked for ONLY where the adapter has them - read
        use_gpu().device().features() to see which the device got);
        required_limits (None = WebGPU's defaults). Call it ONCE, before the
        page's first boundary appears - at the top of the root component:
            use_hook(|| set_page_gpu_options(PageGpuOptions {
                optional_features: wgpu::Features::TIMESTAMP_QUERY,
                ..PageGpuOptions::default() }));
        Options set after the page has asked are used only if it has to ask
        again (after a lost device).

WebGPU'S ERRORS: ONE HANDLER PER DEVICE, ANY NUMBER OF READERS.
    WebGPU allows ONE uncaptured-error handler and ONE device-lost callback
    per device; whoever installs a second takes the errors away from the
    first. So the crate installs both ITSELF, exactly once per device, when
    the first GpuContext of that device is made (request, request_with, or
    from_parts - however many contexts wrap the same device, they share the
    one installation). Everything else SUBSCRIBES:
        let errors = gpu.subscribe_errors();     // GpuErrorSubscription
        for error in errors.take() { /* show or log it */ }   // once a frame
    THE SEMANTICS: every subscription sees EVERY error that arrives while it
    exists, in order; taking errors from one never takes them from another;
    an error that arrives while nobody is subscribed is not kept; an unread
    subscription keeps the newest GpuErrorSubscription::CAPACITY errors and
    counts the rest (dropped_count()); dropping it ends it. A GpuError has
    kind() (Validation, OutOfMemory, Internal) and message(). The errors are
    the DEVICE's: WebGPU does not say which library caused one, so a page
    with two libraries on one device sees each error in both. Errors caught by
    an error scope you pushed are the scope's and reach no subscription.
    gpu.lost_reason() / gpu.is_lost() report the device lost (the same for
    every reader). The graphics crate's GraphicsContext::take_errors() is a
    subscription of this kind. NEVER call device.on_uncaptured_error or
    set_device_lost_callback yourself: that takes the errors from every
    subscription.

THE CANVAS HOST.
    WebGpuCanvas { context, layers, clock, clear_color, transparent,
                   class, style, id, label, suppress_context_menu }
        A Dioxus `canvas` that becomes a wgpu surface when it is mounted,
        draws its layers every animation frame and hands them its pointer,
        wheel and key input. It carries NO styling and no CSS class of its
        own: size it with `class` (the application's own CSS; the surface
        follows the DISPLAYED size times devicePixelRatio, so a canvas needs
        a size from its CSS). `style` sets the inline style attribute, which
        a strict Content-Security-Policy refuses: prefer `class`. The host
        sets `touch-action: none` on the element through its style object
        (allowed under a strict policy), so a touch drag reaches the layers.
        The canvas takes the focus when pressed; while it has it, the arrow
        keys, the space bar and the wheel do not scroll the page. Pressing
        the middle button on the canvas starts no browser autoscroll, and
        with `suppress_context_menu` (default true) a right-click opens no
        browser context menu over it, so that middle- and right-button
        drags reach the layers; false keeps the browser's menu.
        `context`, `layers` and `clock` are taken when the canvas is
        mounted: keep each in the page (use_hook) and change the layers
        through their handle.

    A RENDERER (a layer) implements the trait CanvasRenderer:
        fn prepare(&mut self, context: &GpuContext, target: &CanvasTarget) {}
            // once before its first frame: pipelines for target.format
        fn resize(&mut self, context: &GpuContext, width: u32, height: u32) {}
        fn render(&mut self, frame: &mut CanvasFrame<'_>);   // required
        fn input(&mut self, input: &CanvasInput) {}
        fn surface_failed(&mut self, failure: &SurfaceFailure) {}
            // default: the reason, which says what to do, on the console
    RendererHandle::new(renderer) wraps one; two handles are equal when they
    are the same renderer.

    THE FRAME (CanvasFrame), the same for every layer in a frame:
        frame.context    device and queue
        frame.surface, frame.config, frame.texture   the surface, as the
                         host configured it, and the texture the layers
                         draw into (this frame's surface texture; on a
                         transparent canvas the host's own texture of the
                         same size and format - see DRAWING ON THE PAGE)
        frame.target     CanvasTarget { format, surface_format, alpha_mode }
        frame.view       this frame's view, in target.format
        frame.encoder    ONE command encoder for the frame: record render
                         passes into it; the host submits it once, after the
                         last layer, then presents
        frame.info       FrameInfo { number, time, delta, previous_time,
                         width, height, pixel_ratio } from the ONE clock
        frame.points     the clock's named points (and the helpers below)
    Read what you need from `frame` before a render pass borrows
    `frame.encoder`.

    CanvasInput: PointerDown { x, y, pointer_id, button, buttons, kind,
    modifiers }, PointerMove { x, y, dx, dy, pressed, pointer_id, buttons,
    kind, modifiers }, PointerUp { x, y, pointer_id, button, buttons, kind,
    modifiers }, Wheel { delta_x, delta_y } (CSS pixels, positive y towards
    the visitor), KeyDown { key }, KeyUp { key } (the browser's key names).
    Coordinates are the canvas's CSS pixels. Every pointer (a mouse button, a
    pen, each finger on a touch screen) has its own pointer_id, and each
    one's dx and dy are its own distances, so a layer can follow two fingers
    at once (a pinch: the graphics crate's OrbitController does). Match with
    `..` (`CanvasInput::PointerMove { dx, pressed: true, .. }`) so that a
    field added later does not break the match.
        button     PointerButton: Primary (left mouse button, a finger, a
                   pen's tip), Secondary (right button, a pen's side
                   button), Middle (the wheel pressed), Back, Forward,
                   Other (none, a cancelled pointer - which arrives as
                   PointerUp - or a pen's eraser)
        buttons    PointerButtons, the browser's `buttons` bits: NONE,
                   PRIMARY, SECONDARY, MIDDLE, BACK, FORWARD; contains(b),
                   primary(), secondary(), middle(), is_empty(), bits(),
                   from_bits(..), with(..). On PointerUp: what is still
                   down. `pressed` is !buttons.is_empty().
        kind       PointerKind: Mouse (also a trackpad, and any type the
                   browser does not name), Pen, Touch
        modifiers  ModifierKeys { shift, ctrl, alt, meta } (meta: the
                   Windows key, Command on a Mac); ModifierKeys::NONE,
                   is_empty()
    A SECOND mouse button pressed (or released) while another is down
    arrives as a PointerMove whose `buttons` changed: the browser sends no
    second PointerDown for the same pointer. Wheel and key events carry no
    modifiers.

LAYERS AND THE CLOCK.
    The application puts renderers together on one canvas - for example a
    3D scene from the Graphics crate and a 2D animation from the Animation
    crate over it - though neither knows the other:

        CanvasLayers::new(vec![scene, titles])   // bottom first
        layers.push(h) / insert(index, h) / remove(&h) -> bool /
        contains(&h) / position(&h) / len() / is_empty()
            change the layers of a RUNNING canvas: a layer added is prepared
            and sized before its first frame; one removed is no longer drawn;
            adding a layer that is there moves it. CanvasLayers::single(h)
            for one layer.

    THE LAYER CONTRACT, which the host keeps for every canvas:
      * Each frame the HOST clears the target to `clear_color` (default
        opaque black; transparent black on a `transparent` canvas). A layer
        never clears: its render passes LOAD the target (LoadOp::Load), so
        a later layer draws over an earlier one.
      * Every layer gets the same frame, in order: one encoder, one view,
        one target, one time.
      * Input goes to every layer, in order.

    THE SURFACE AND THE BLENDING CONVENTION are decided by the HOST alone
    (CanvasTarget), so two crates can never disagree:
      * target.format is the sRGB form of the browser's preferred canvas
        format (Bgra8UnormSrgb or Rgba8UnormSrgb): every pipeline's colour
        target uses it. Shaders write LINEAR colour; the view encodes it to
        sRGB on write and decodes it for blending. Never sRGB-encode in a
        shader.
      * Colour is PREMULTIPLIED by alpha. A layer that is opaque blends with
        wgpu::BlendState::REPLACE; a translucent one with
        wgpu::BlendState::PREMULTIPLIED_ALPHA_BLENDING.
      * target.alpha_mode is Opaque, or PreMultiplied on a canvas with
        `transparent: true` (the page then shows through where the alpha is
        below 1).

    DRAWING ON THE PAGE (a frameless, transparent canvas - the drawing floats
    over the page's own background and text):
      * WebGpuCanvas { transparent: true, .. } - the host clears to
        transparent (leave clear_color unset, or give it alpha 0).
      * No frame in the page's styling: no border, ring, shadow or rounded
        corners on the canvas or its container, and no background on them;
        give the page (or the element under the canvas) the background you
        want to see. Lay the canvas over the content with the application's
        CSS classes (for example `absolute inset-0`).
      * The layers change nothing: they draw linear, premultiplied colour
        into frame.view as always. On a transparent canvas the host has them
        draw into a texture of its own and, after the last layer, writes it
        onto the surface un-premultiplied, sRGB-encoded and premultiplied
        again - the form the browser composites. So half-transparent colour
        is exactly a blend of its colour and the page's, and anti-aliased
        edges show no light or dark fringe over the page. (Written straight
        to the surface, linear premultiplied colour through an sRGB view
        would reach the page too bright wherever alpha is between 0 and 1.)
        It costs one full-canvas pass per frame, on transparent canvases
        only.
      * Graphics: no skybox, and with HDR the ToneMapper's `preserve_alpha`
        (see the HDR path). Animation: a session drawn as a layer leaves its
        background out by default (see the Animation crate's DRAWING ON THE
        PAGE).

    THE CLOCK (CanvasClock): ONE clock per canvas (or shared by several
    canvases: it advances once per animation frame). The frame time every
    layer reads comes from it, derived from the requestAnimationFrame
    timestamp; pausing it or changing its speed keeps every layer in step.
        CanvasClock::new()          keep it in the page (use_hook) and pass
                                    it as `clock`; a canvas without one has
                                    a clock of its own
        clock.pause() / resume() / toggle() -> bool (now paused) /
        is_paused()                 paused: the time stands still, delta 0,
                                    frames are still drawn and counted;
                                    resumed: the time goes on from where it
                                    stood (the pause is not counted)
        clock.set_speed(s) / speed()   1 real time, 0.5 half, 2 double
        clock.seek(seconds)         move the clock (0 to start again)
        clock.elapsed() / frames()  the time and number of the last frame
    A SEEK may be made at any time, also before the canvas's first frame (a
    page opened at ?t=12): the next frame shows that time, on a running or a
    paused clock, and a running clock goes on from it; elapsed() returns the
    sought time at once. Pass the page's clock to the canvas (clock:
    clock.clone()): a canvas without one has a clock of its own, which the
    page's seeks never reach. The first frame, the frame after a seek and
    the frame after a resume have delta 0, so a layer that ADDS UP deltas
    into a time of its own never sees the jump: read frame.info.time (or
    seek a playback controller or animation player to it), and use delta
    only for what is meant to ignore seeks (smoothing, a simulation step,
    frame statistics).

    NAMED POINTS IN TIME. Define a point once, on the clock's timeline, and
    let every layer refer to it by name:
        clock.define_point("title in", 4.0)   // defines or moves it
        clock.remove_point("title in") -> bool
        clock.points() -> TimePoints
    In a layer, from the frame (the points as they were when it began):
        frame.point_time("title in")     Option<f64>: its time
        frame.since_point("title in")    Option<f64>: seconds since it
                                         (negative before it)
        frame.point_reached("title in")  bool: this frame's time >= it
        frame.point_crossed("title in")  bool: true in the ONE frame whose
                                         step of the clock reaches it (from
                                         the previous frame's time,
                                         exclusive, to this frame's,
                                         inclusive) - in every layer the
                                         same frame. A pause crosses
                                         nothing; a seek forward over a
                                         point crosses it; a seek back
                                         crosses nothing.
    A name that is not defined gives None and false; check
    frame.points.contains(name) where a misspelt name must not pass
    silently.

IMAGES: PNG AND JPEG FILES TO PIXELS (not in C#).
    decode_png(&bytes)   -> Result<DecodedImage, String>
    decode_jpeg(&bytes)  -> Result<DecodedImage, String>
    decode_image(&bytes) -> Result<DecodedImage, String>
        decode_image looks at the file's first bytes (never at a name or a
        content type) and calls one of the other two.
    DecodedImage { width, height, rgba: Vec<u8> }, size() -> (w, h);
    MAX_IMAGE_DIMENSION = 8192 (the largest width or height accepted).
    THE PIXEL CONTRACT - the one form in which every crate of these packages
    takes decoded pixels: tightly packed 8-bit RGBA, four bytes a pixel, red
    first, no padding between rows, STRAIGHT (not premultiplied) alpha, rows
    from the top of the picture to the bottom. The decoders are NEUTRAL
    about colour space: they return the values as the file stores them, and
    the CALLER says whether they are sRGB-encoded colour (a picture, a
    base-colour or emissive map) or linear data (a normal, occlusion,
    metallic or roughness map).
    THE FUNCTIONS ARE SEPARATE ON PURPOSE: unused code is dropped when the
    application is built, so an application that only calls decode_png
    carries no JPEG decoder, and the other way round; decode_image carries
    both. Call the one for the format that can occur.
    PNG: every colour type and bit depth becomes straight RGBA8 (grey ->
    equal red, green and blue; a palette and its transparency chunk; a tRNS
    colour key -> alpha 0; 1, 2 and 4-bit samples scaled to the full range);
    16-bit samples keep their HIGH byte (fine for pictures; a 16-bit normal
    map loses its low byte); Adam7 interlacing is undone; the file's gamma,
    chromaticity and ICC chunks are NOT applied; of an animated PNG only the
    default image is read.
    JPEG: baseline and progressive (Huffman coded, 8-bit samples), any
    chroma subsampling; colour becomes RGBA, greyscale equal red, green and
    blue, alpha 255. An ICC profile is NOT applied, and the EXIF orientation
    is NOT applied: the pixels are as stored (a photograph straight from a
    phone may rely on that tag to stand upright; glTF expects exactly the
    stored pixels). REFUSED, with a message that says so: CMYK and YCCK
    files (save them as RGB JPEG or PNG), 12-bit and 16-bit samples,
    lossless, arithmetic-coded and hierarchical JPEG, other numbers of
    colour components, and damaged or cut-short files (the decoder never
    fills a missing part with grey). A JPEG is a poor choice for a normal,
    metallic-roughness or occlusion map: its compression changes the data
    values, not only the look; use PNG for those.
    decode_image refuses every other format, NAMING it where it recognises
    it (GIF, WebP, KTX2, KTX, BMP, TIFF, AVIF, HEIF, JPEG XL, DDS, QOI,
    Radiance HDR); an application with a decoder of its own for another
    format hands over the pixels in the pixel contract instead. Every error
    is a String that says what is wrong with the file. Larger than 8192 on a
    side is refused for both formats.
    The graphics crate's create_texture_from_png / _jpeg / _image and its
    glTF loader, and the animation crate's pictures, decode through these
    functions. A Radiance .hdr panorama is not an image file in this sense:
    the graphics crate decodes it itself (create_environment_from_bytes).

THE MATHS: codebrix_webgpu::math is `glam` under its OWN names. There are no
aliases for System.Numerics names, because the multiplication order differs.
PORTING FROM System.Numerics:

    System.Numerics            codebrix_webgpu::math (glam)
    ---------------            ----------------------------
    Vector2                    Vec2
    Vector3                    Vec3
    Vector4                    Vec4
    Matrix4x4                  Mat4
    Quaternion                 Quat
    Vector3.Transform(v, m)    m.transform_point3(v)
    Matrix4x4.CreateTranslation(t)       Mat4::from_translation(t)
    Matrix4x4.CreateScale(s)             Mat4::from_scale(s)
    Matrix4x4.CreateFromQuaternion(q)    Mat4::from_quat(q)
    Quaternion.CreateFromAxisAngle(a, r) Quat::from_axis_angle(a, r)

    WARNING - THE ORDER IS REVERSED. System.Numerics uses row vectors: in
    `a * b` the matrix `a` is applied FIRST. glam uses column vectors: in
    `a * b` the matrix `b` is applied first. So the .NET
    `scale * rotation * translation` is glam's
    `translation * rotation * scale`, and a model-view-projection matrix is
    `projection * view * model`. glam's matrices are column-major; with the
    `bytemuck` feature (on) they upload to a WGSL uniform unchanged.


THE codebrix-webgpu-graphics CRATE
==================================

The Rust counterpart of the .NET library CodeBrix.WebGpu.Graphics, for a
browser frontend. Code written against the C# library ports to it nearly line
for line, by these rules:

    * TYPE NAMES ARE THE C# NAMES: GraphicsContext, GpuBuffer, GpuTexture,
      GpuSampler, SamplerDescription, GpuShader, BindingLayoutEntry,
      BindingEntry, GpuBindGroupLayout, GpuBindGroup, RenderPipelineBuilder,
      GpuRenderPipeline, BlendPreset, DepthPreset, CullPreset, VertexLayout,
      VertexElement, GraphicsFrame, RenderPass, RenderTargetView, Color4,
      GraphicsError, GraphicsException, ShaderCompilationException,
      PbrShader, PbrFeatures, SpriteBatch, RectangleF (System.Drawing's),
      Camera, ProjectionKind, OrbitCamera, MeshVertex, MeshData, MeshShapes,
      Mesh, MeshRenderer, LitMaterial, MaterialAlphaMode, SceneLighting,
      DirectionalLight, AmbientLighting, Model, ModelMesh, ModelPrimitive,
      ModelMorphTarget, ModelMaterial, ModelTextureSlot, ModelNode,
      NodeTransform, ModelSkin, ModelAnimation, ModelAnimationChannel,
      AnimationPath, AnimationInterpolation, ModelVertex, ModelSkinVertex,
      ModelInstance, ModelPoseEdit, ModelRenderer, ModelLoadOptions,
      AnimationPlayer, AnimationState, AnimationLoopMode, EnvironmentMap,
      EnvironmentOptions, ProceduralSkyOptions, FloatImage,
      RadianceHdrDecoder, SkyboxRenderer, ToneMapper, ToneMapOperator,
      HeadlessRenderTarget, AnimatedTexture, AnimatedTextureOptions,
      AnimatedTextureStorage, RenderBundleBuilder, GpuRenderBundle,
      ComputePipelineBuilder, GpuComputePipeline, GpuPipelineLayout,
      ComputePass, ComputePassOptions, RenderPassOptions, PassTimestamps,
      DrawIndirectArgs, DrawIndexedIndirectArgs, DispatchIndirectArgs,
      GpuQuerySet, GpuTimer, TextureRegion, TextureCopyLayout and
      BufferMapState (WebGPU's map state, as C# has it). NEW in Rust (C# leaves
      it to each viewer): OrbitController and OrbitLimits; the model-data
      READER (ModelDataSet, ModelData, ModelTextureData, ModelDataFormat);
      and, marked "Not in C#" in the crate's documentation, the glTF loader
      (GltfDocument, GltfModelSet, GltfLoadOptions), picking and labels (Ray,
      CanvasProjection), culling and level of detail (CullVolume,
      LodInstance, LodLevel, LodState, ModelRenderStats), local lights and
      shadows (PointLight, SpotLight, LocalLight, SunShadowSettings,
      SpotShadowSettings, ShadowMapSize, SunShadowMap), multisampling
      (CanvasFrameTarget, TargetFormats), the effects (FrameTimer, GlowLayer,
      BloomRenderer, ParticleSystem) and water (WaveField, OpenWaterWaves,
      WaterRenderer, WaterSurface, WaterLook).
    * MEMBERS ARE snake_case OF THE C# NAMES, WITH THE SAME ARGUMENT ORDER:
      CreateTextureFromRgba -> create_texture_from_rgba, SetBindGroup ->
      set_bind_group, Width -> width(). A property is a getter method.
    * C# DEFAULT ARGUMENTS: a lone defaulted `label` is an `Option<&str>`
      written out (None for C#'s null). Two or more defaulted arguments are
      ONE options struct with Default holding the C# defaults
      (Texture2DOptions, RgbaTextureOptions, RenderTextureOptions,
      DepthTextureOptions, ColorTargetOptions, DepthOptions, StencilMasks,
      DrawOptions, DrawIndexedOptions, BufferRange, DepthRange,
      VertexLayoutOptions, SpriteBatchBeginOptions, CircleOptions,
      MaterialOptions, SkyboxDrawOptions, HeadlessTargetOptions,
      PlayOptions, RenderBundleOptions). Defaulted arguments of a value factory
      (BindingLayoutEntry, BindingEntry) are `with_*` methods on the value.
      A lone defaulted number (a thickness, a segment count) is written out.
      The MeshShapes factories take EVERY argument written out (C#'s
      defaults are in their docs; a `None` colour is white).
    * NEW FIELDS COME WITHOUT A MAJOR VERSION. Public structs gain fields as
      the crates grow, and the crate's major version does not change for
      it. Write every struct literal of an options or settings type with
      `..Default::default()` (`SceneLighting { light_intensity: 3.0,
      ..SceneLighting::default() }`) and match enum variants with `..`, and
      such code keeps compiling. Types that have gained fields this way:
      SceneLighting (local_lights, sun_shadows, spot_shadows),
      ModelRenderStats (its shadow, level-of-detail and light counts),
      ModelMaterial (emissive_vertex_color; build it with
      ModelMaterial::new(index, name) and set fields), ModelInstance and
      OrbitController (made by their constructors, so only code that names
      every field is touched), and codebrix-webgpu's CanvasInput pointer
      events (button, buttons, kind, modifiers).
    * C# PROPERTIES WITH A SETTER: a pub field on a plain value (Camera,
      SceneLighting, OrbitCamera's target and yaw), or a getter and a
      `set_*` method where C# clamps or tracks changes (OrbitCamera's pitch and
      distance, LitMaterial's every property, SpriteBatch's
      use_nearest_filtering).
    * C# CLASSES THAT CODE SHARES BY REFERENCE stay shared: a LitMaterial is a
      handle (its clones are the same material; its setters take &self), and
      so are a Model and an EnvironmentMap.
    * C# OVERLOADS get a second name: CreateIndexBuffer(ushort[] / uint[]) ->
      create_index_buffer_u16 / _u32; BeginRenderPass(target, ...) ->
      begin_render_pass_to; CreateBuffer<T>(data, ...) -> create_buffer_from;
      WithVertexBuffer<T>() -> with_vertex_buffer_of::<T>(); Play(int / string)
      -> play / play_named (CrossFade and Blend the same); Stop() /
      Stop(layer) / Stop(state) -> stop / stop_layer / stop_state;
      CreateRenderBundle(target / pipeline / formats) ->
      create_render_bundle / _for_pipeline / _with_formats;
      CreateIndirectBuffer(args / size) -> create_indirect_buffer_from /
      create_indirect_buffer; CopyBufferToTexture, CopyTextureToBuffer (a
      region / a whole level) -> copy_buffer_to_texture / _level and
      copy_texture_to_buffer / _level; CopyTextureToTexture (a region / a
      level) -> copy_texture_region_to_texture / copy_texture_to_texture;
      ExecuteBundle(s) -> execute_bundle / execute_bundles;
      SpriteBatch.Draw(AnimatedTexture, ..) -> draw_animated /
      draw_animated_rotated.
    * IDisposable IS Drop: a resource is released when it is dropped; there
      is no Dispose call.
    * "unmanaged T" IS `bytemuck::Pod`: buffers take plain values
      (`codebrix_webgpu_graphics::bytemuck` is re-exported; glam's types are
      Pod).
    * MATHS: System.Numerics is glam, with the matrix order REVERSED (see THE
      MATHS above). The shaders' uniform layouts are unchanged: a glam Mat4
      built as projection * view * model has the same bytes as the C# matrix
      built as model * view * projection. Matrix3x2 (the sprite batch's and a
      material's 2D transforms) is glam's Affine2, reversed the same way:
      C#'s `CreateScale(s) * CreateTranslation(t)` is
      `Affine2::from_translation(t) * Affine2::from_scale(s)`. glam's view
      and projection constructors are in its `camera` module
      (glam::camera::rh::view::look_at_mat4,
      glam::camera::rh::proj::directx::perspective / orthographic: right-
      handed, depth 0..1 as WebGPU has it); Camera uses them for you.
    * VertexLayout.FromStruct<T> (reflection) becomes the VertexType trait,
      written by hand with VertexLayout::describe (no derive macro).

THE ONE DELIBERATE DIFFERENCE: A BROWSER CANNOT WAIT.
    The C# library checks every object it creates inside a WebGPU error scope
    and waits for the answer, so a C# factory either returns or throws with
    WebGPU's message. A browser answers only asynchronously, so:
    * Device creation is async: GraphicsContext::create(options).await.
    * Reading data back is async: buffer.read::<T>(count, offset).await,
      texture.read_pixels(mip) / read_rgba() / read_region(&region) .await,
      HeadlessRenderTarget's read_pixels / read_rgba / read_depth .await,
      buffer.map_async(mode, offset, size).await (C# Map waits; MapAsync is
      the same), timer.read_milliseconds().await, and
      ctx.when_submitted_work_done().await (C# WhenSubmittedWorkDoneAsync).
    * A shader's compiler messages are async:
      context.try_create_shader(wgsl, label).await (Ok: the shader and any
      warnings; Err: ShaderCompilationException with the errors, line and
      column), or shader.diagnostics().await.
    * A factory returns its object AT ONCE. A validation error WebGPU finds
      later is QUEUED: context.take_errors() hands out the queued
      GraphicsErrors (C#'s TakeErrors()). Read it once per frame, or after
      loading, and show or log what comes out.
    * Where C# code relies on "throw with WebGPU's message", await a checked
      form: builder.build_checked().await, or wrap any calls in
      context.validate(|c| ...).await, which returns the result or a
      GraphicsException with WebGPU's message.
    * Checks the library makes ITSELF before it asks WebGPU (a size of 0, a
      mip level that does not exist, a pipeline with a fragment stage but
      neither a colour target nor depth, a sample count other than 1 or 4)
      return Err(GraphicsException) at once, with a message
      that says what to do. The C# ArgumentException family maps to that.
    * Poll(), WaitIdle() and WaitForSubmission() have no counterpart: the
      browser drives the queue. An async read takes a CLONE of what it reads
      (the cheap handles: GpuTimer, an Rc of a GpuBuffer) into a spawned task,
      while the layer goes on drawing.

THE CONTEXT.
    GraphicsContext::wrap_gpu_context(&gpu)    SHARES the canvas host's
        device (the usual way in a page: one device for everything). C#:
        Wrap(IWebGpuDeviceSource).
    GraphicsContext::wrap(device, queue, instance, adapter)
    GraphicsContext::create(GraphicsContextOptions).await
        Its own device. Err is codebrix-webgpu's GpuUnavailable (show it with
        WebGpuUnavailable). GraphicsContextOptions: power_preference,
        force_fallback_adapter, label, required_features, required_limits,
        request_adapter_limits (C#'s browser-meaningful options).
    device() / queue() / instance() / adapter() / gpu_context()
        The wgpu objects: low-level wgpu stays available beside the wrapper.
    adapter_info(), limits(), features(), has_feature(f), is_lost(),
    lost_reason(), is_wrapped(), take_errors(), validate(..)
    take_errors() is a SUBSCRIPTION to the device's errors, which
    codebrix-webgpu collects through the device's one handler (WebGPU'S
    ERRORS above): every GraphicsContext - and any other library on the
    device - sees every error that arrives after it was made; clones of one
    GraphicsContext share its queue. Nothing installs a second handler.

RESOURCES (the context's factories).
    create_buffer(size, usage, label)             -> Result<GpuBuffer, ..>
    create_buffer_from(&data, usage, label)
    create_vertex_buffer(&vertices, label)
    create_index_buffer_u16(&indices, label) / create_index_buffer_u32(..)
    create_uniform_buffer_from(&value, label) / create_uniform_buffer(size, ..)
    create_storage_buffer_from(&data, label) / create_storage_buffer(size, ..)
        Sizes round up to 4 (uniforms to 16); COPY_SRC | COPY_DST are added
        unless the usage maps the buffer.
    buffer.write(&data, byte_offset) / write_value(&value, byte_offset)
    buffer.read::<T>(count, byte_offset).await    async readback
    create_texture_2d(w, h, format, usage, Texture2DOptions)
    create_texture_from_rgba(w, h, &rgba, RgbaTextureOptions { mipmaps, srgb, label })
        mipmaps: a full chain made on the CPU (2 x 2 box filter); srgb:
        Rgba8UnormSrgb for colour images.
    create_texture_from_png(&png_bytes, RgbaTextureOptions { .. })
    create_texture_from_jpeg(&jpeg_bytes, RgbaTextureOptions { .. })
    create_texture_from_image(&bytes, RgbaTextureOptions { .. })   (not in C#)
        The bytes of a PNG or JPEG FILE, decoded in the browser by
        codebrix-webgpu's decode_png / decode_jpeg / decode_image (IMAGES
        above: the rules, the refusals, the pixel contract), then exactly
        create_texture_from_rgba with the same options. The decoders do not
        know the colour space: say `srgb: true` for a picture or a colour map,
        false for data (normal, metallic-roughness, occlusion). Errors start
        with the call's name and give the decoder's message.
        create_texture_from_image carries both decoders; call _from_png
        where only PNG files occur.
    create_render_texture(w, h, RenderTextureOptions)
    create_depth_texture(w, h, DepthTextureOptions)
    texture.write(..) / write_rgba(..) / write_mip_level(..) / view() /
        as_render_target(); GpuTexture::full_mip_chain_length(w, h)
    create_sampler(&SamplerDescription::linear_clamp())  (also
        linear_repeat, nearest_clamp, nearest_repeat, or fields)
    create_shader(wgsl, label) / try_create_shader(wgsl, label).await
    create_bind_group_layout(label, &[BindingLayoutEntry::uniform(0, stages),
        BindingLayoutEntry::texture(1, stages), BindingLayoutEntry::sampler(2, stages)])
    create_bind_group(&layout, label, &[BindingEntry::for_buffer(0, &buffer),
        BindingEntry::for_texture(1, &texture), BindingEntry::for_sampler(2, &sampler)])
    wrap_buffer / wrap_texture / wrap_sampler / wrap_shader /
        wrap_bind_group_layout / wrap_bind_group(group, &layout, ..) /
        wrap_pipeline_layout / wrap_render_pipeline(p, formats, depth,
        samples, ..) / wrap_compute_pipeline / wrap_query_set /
        wrap_render_bundle            wgpu objects made elsewhere (C# Wrap*)
    Every type has handle() for its wgpu object.
    MAPPING (C# GpuBuffer.Mapping): ctx.create_mapped_buffer(size, usage, ..)
        (mapped from the start), buffer.map_async(MapMode::Read, 0, None)
        .await, buffer.read_mapped(&mut values, offset) /
        write_mapped(&values, offset), buffer.unmap(), buffer.map_state();
        ctx.create_initialized_buffer(&data, usage, ..).
    TEXTURE REGIONS (C# TextureRegion, TextureCopyLayout):
        TextureRegion::new(x, y, w, h).on_layer(l).on_mip_level(m),
        TextureRegion::whole(&texture, mip); texture.write_region(&region,
        &pixels); texture.read_region(&region).await;
        texture.copy_to(&other) (submitted at once);
        texture.create_layer_view(layer, ..) (C# CreateLayerView);
        TextureCopyLayout::aligned_bytes_per_row / buffer_size / pack_rows /
        unpack_rows (rows of buffer copies are 256-byte aligned).

PIPELINES.
    context.create_render_pipeline(&shader)       vs_main / fs_main of it
        .with_vertex_buffer(layout) / .with_vertex_buffer_of::<T>(VertexLayoutOptions::default())
        .with_color_target(format, ColorTargetOptions::blended(BlendPreset::PremultipliedAlpha))
        .with_depth(format, DepthOptions::default())       ReadWrite
        .with_culling(CullPreset::Back, wgpu::FrontFace::Ccw)
        .with_topology(..) / .with_sample_count(..) / .with_stencil(..)
        .with_bind_group_layouts(&[&group0, &group1]) / .with_pipeline_layout(..)
        .with_vertex_entry(name, None) / .with_fragment_entry(..) / .without_fragment()
        .with_label("..")
        .build()                -> Result<GpuRenderPipeline, GraphicsException>
        .build_checked().await  -> the same, with WebGPU's verdict
        with_sample_count takes 1 or 4 (SAMPLE_COUNTS; ANTI-ALIASING below).
        A pipeline of your own must match the target it draws into - colour
        format, depth format and sample count: keep one per
        pass.target().formats() (a TargetFormats, Hash + Eq) and build it
        with .with_sample_count(formats.sample_count), as the crate's own
        renderers do. A depth-only pipeline (with_depth, no colour target)
        may keep a fragment stage, for example one that only discards.
    VertexLayout::describe(VertexLayoutOptions::default())
        .add(wgpu::VertexFormat::Float32x2).skip(4).add_at(format, location)
        .build(0)               0 = the size of what was described

FRAMES, AND THE CANVAS: THE GRAPHICS ARE A LAYER.
    The canvas belongs to the codebrix-webgpu host (THE CANVAS HOST): it
    clears, owns the one encoder and the one clock, submits and presents. A
    graphics renderer is a CanvasRenderer that draws INTO the frame it is
    handed, over what is there:
        context.begin_canvas_frame(frame, depth)  -> GraphicsFrame
            (C#: BeginFrame(IWebGpuTextureTarget, depth)). Its target is the
            frame's view in the canvas's draw format; `depth` an optional
            depth texture of the frame's size that the layer keeps.
        context.begin_canvas_frame_on(frame, &mut canvas_target)?
            (not in C#) the same with a CanvasFrameTarget the layer keeps
            (made once with create_canvas_frame_target): it keeps a depth
            buffer at the canvas's size by itself (no resize code) and draws
            with 1 or 4 samples (ANTI-ALIASING below).
        graphics_frame.begin_render_pass(None, None)
            clear_color None LOADS (a layer never clears; C# null does the
            same); clear_depth None loads depth (C# defaults to 1: pass
            Some(1.0) with a depth texture of the layer's own).
        Pipelines use the frame's target.format and premultiplied blending
        (BlendPreset::PremultipliedAlpha, or Opaque for a layer that covers
        everything); shaders write linear colour (the view encodes).
        Animate from frame.info.time (the host's clock): never read a clock
        for anything that is drawn, so that pausing and speed apply to every
        layer. (The one clock reading in the crate is FrameTimer's CPU
        measurement, for read-outs; nothing drawn depends on it.)
    Outside the canvas: context.begin_frame() (an encoder of its own;
    frame.submit()), begin_frame_with_encoder(&mut encoder) (the caller
    submits), begin_render_pass_to(&target, ..) for a RenderTargetView of a
    texture (texture.as_render_target(), .with_depth(&depth)).
    RenderPass: set_pipeline, set_bind_group(index, &group),
    set_bind_group_with_offsets, set_vertex_buffer(slot, &buffer,
    BufferRange::default()), set_index_buffer(&buffer, format, range),
    draw(count, DrawOptions::default()), draw_indexed(count,
    DrawIndexedOptions::default()), draw_indirect(&args, offset)?,
    draw_indexed_indirect, multi_draw_indirect(&args, offset, count)?,
    multi_draw_indexed_indirect, execute_bundle(&bundle),
    execute_bundles(&[..]), set_blend_constant(color),
    set_stencil_reference(r), push_debug_group / pop_debug_group /
    insert_debug_marker, set_viewport, set_scissor_rect, end(); handle()
    for wgpu's own pass. set_pipeline CHECKS the pipeline against the pass's
    target (colour format, depth format, sample count) and PANICS with a
    message that names the difference and the fix (a mismatch would
    otherwise invalidate the whole frame); try_set_pipeline returns that
    message as an Err instead, check_pipeline only checks. execute_bundle(s)
    check a bundle the same way.
    The FRAME also records (C#: GraphicsFrame.Commands): begin_compute_pass
    (&ComputePassOptions), copy_buffer, copy_buffer_to_texture(_level),
    copy_texture_to_buffer(_level), copy_texture_region_to_texture,
    copy_texture_to_texture, upload_texture(&texture, &region, &pixels)
    (ordered with the frame's commands), clear_buffer(&buffer, offset,
    None), resolve_query_set(..), and the debug groups;
    begin_render_pass_with_options(target, clear, depth, &RenderPassOptions
    { label, timestamps, .. }) for the rarely used settings.

THE BUILT-IN SHADERS (codebrix_webgpu_graphics::shaders).
    The WGSL of the C# library, compiled into the crate: BuiltInShaders::
    SPRITE and LIT_MESH, MipmapShader::SOURCE, EnvironmentShaders::
    EQUIRECT, PROCEDURAL_SKY, IRRADIANCE, PREFILTER, BRDF_LUT, SKYBOX and
    TONE_MAP, and PbrShader::source(PbrFeatures) (one `const` per feature in
    front of one body; PbrFeatures::every_shader_variant() lists every
    combination - the number doubles with each shader feature, so compiling
    them all takes a while); ShadowDepthShader::source(features) for the
    shadow passes (its own few features, ShadowDepthShader::every_variant()).
    BuiltInShaderSource::all(variants) lists them by name (the named sources,
    the PBR variants given, then the shadow depth variants): compile them
    once in a new browser (each browser has its own WGSL compiler) - the
    GraphicsDemo sample's "Shaders" scene does that and shows the messages.
    The crate's own shaders beyond the C# library's are listed apart:
    EffectShaders::all() (glow, bloom, particles) and WaterShaders::all()
    (the wave field, the water surface, its scene capture); a page that
    compiles every shader compiles those too. Chrome refuses some WGSL that
    other compilers (naga) accept - a texture sample or a derivative inside
    a branch, a float literal at f32's limit - so prove a shader of your own
    in Chrome.

C# AND RUST SIDE BY SIDE (a textured quad):

    C# (CodeBrix.WebGpu.Graphics)                Rust (codebrix-webgpu-graphics)
    --------------------------------------------  ---------------------------------------------
    var ctx = GraphicsContext.Create();          let ctx = GraphicsContext::wrap_gpu_context(&gpu);
    var shader = ctx.CreateShader(wgsl, "quad"); let shader = ctx.create_shader(WGSL, Some("quad"));
    var vb = ctx.CreateVertexBuffer<QuadVertex>( let vb = ctx.create_vertex_buffer(&vertices,
        vertices, "quad");                           Some("quad"))?;
    var ib = ctx.CreateIndexBuffer(              let ib = ctx.create_index_buffer_u16(
        new ushort[] { 0, 1, 2, 0, 2, 3 });          &[0, 1, 2, 0, 2, 3], None)?;
    var tex = ctx.CreateTextureFromRgba(w, h,    let tex = ctx.create_texture_from_rgba(w, h,
        rgba, mipmaps: true, srgb: true);            &rgba, RgbaTextureOptions { mipmaps: true,
                                                     srgb: true, label: None })?;
    var smp = ctx.CreateSampler(                 let smp = ctx.create_sampler(
        SamplerDescription.LinearClamp);             &SamplerDescription::linear_clamp());
    var layout = ctx.CreateBindGroupLayout(      let layout = ctx.create_bind_group_layout(None, &[
        BindingLayoutEntry.Texture(0, Fragment),     BindingLayoutEntry::texture(0, ShaderStages::FRAGMENT),
        BindingLayoutEntry.Sampler(1, Fragment));    BindingLayoutEntry::sampler(1, ShaderStages::FRAGMENT)]);
    var group = ctx.CreateBindGroup(layout,      let group = ctx.create_bind_group(&layout, None, &[
        BindingEntry.ForTexture(0, tex),             BindingEntry::for_texture(0, &tex),
        BindingEntry.ForSampler(1, smp));            BindingEntry::for_sampler(1, &smp)]);
    var pipeline = ctx.CreateRenderPipeline(     let pipeline = ctx.create_render_pipeline(&shader)
        shader)                                      .with_vertex_buffer_of::<QuadVertex>(
        .WithVertexBuffer<QuadVertex>()                  VertexLayoutOptions::default())
        .WithColorTarget(format,                     .with_color_target(target.format,
            BlendPreset.PremultipliedAlpha)              ColorTargetOptions::blended(
                                                         BlendPreset::PremultipliedAlpha))
        .WithBindGroupLayouts(layout)                .with_bind_group_layouts(&[&layout])
        .Build();                                    .build()?;
    // every frame                               // every frame, in CanvasRenderer::render
    using var frame = ctx.BeginFrame(target,     let mut gf = ctx.begin_canvas_frame(frame, None);
        null);                                   {
    using (var pass = frame.BeginRenderPass(         let mut pass = gf.begin_render_pass(None, None)?;
        null, null))                                 pass.set_pipeline(&pipeline);
    {                                                pass.set_bind_group(0, &group);
        pass.SetPipeline(pipeline);                  pass.set_vertex_buffer(0, &vb, BufferRange::default());
        pass.SetBindGroup(0, group);                 pass.set_index_buffer(&ib, IndexFormat::Uint16,
        pass.SetVertexBuffer(0, vb);                     BufferRange::default());
        pass.SetIndexBuffer(ib,                      pass.draw_indexed(6, DrawIndexedOptions::default());
            IndexFormat.Uint16);                 }   // the pass ends here; the host submits
        pass.DrawIndexed(6);
    }
    frame.Submit();
    // errors were thrown at creation            for error in ctx.take_errors() { /* show it */ }

    The complete version (a layer on the host's clock, a texture made in
    code, the sprite shader) is samples/GraphicsDemo/frontends/graphics/
    src/quad.rs.

SPRITES AND SHAPES (SpriteBatch; C#: Drawing2D).
    SpriteBatch::new(&context)? / SpriteBatch::with_capacity(&context, quads)?
    / context.create_sprite_batch()?
        Pipelines are built when first needed (one per target format and
        blend). Coordinates are PIXELS of the pass's target, origin top
        left, y down; on a canvas that is device pixels, so pass
        Affine2::from_scale(Vec2::splat(frame.info.pixel_ratio as f32)) as
        the initial transform to draw in CSS pixels.
    batch.begin(&pass, SpriteBatchBeginOptions { transform, blend })?
        C#'s defaults: identity, BlendPreset::AlphaBlend (straight alpha -
        right for colours and textures whose alpha is not premultiplied; it
        writes the premultiplied result the canvas convention asks for).
    batch.draw(&texture, destination, tint)                whole texture
    batch.draw_region(&texture, destination, source, tint) an atlas cell
    batch.draw_rotated(&texture, position, source, tint, rotation, origin, scale)
        rotation in radians CLOCKWISE (y points down) about `origin`, given in
        texels from the region's top-left corner, which lands on `position`.
        C#'s three Draw overloads; destination and source are RectangleF.
    batch.fill_rectangle(rect, color) / draw_rectangle(rect, color, thickness)
    batch.draw_line(from, to, color, thickness) / fill_triangle(a, b, c, color)
    batch.fill_quad(a, b, c, d, color) / fill_circle(center, radius, color, segments)
    batch.draw_circle(center, radius, color, CircleOptions { thickness, segments })
        segments 0 = chosen from the radius (12 to 96), as in C#.
    batch.push_transform(Affine2) / pop_transform()?   the new one applies first
    batch.end(&mut pass)?
        Uploads and records ONE indexed draw per run of one texture - shapes
        never break a run. ORDER: items are drawn in the order they were
        added; a later item covers an earlier one (C# has no sort mode
        either). last_draw_call_count(), last_quad_count(),
        set_use_nearest_filtering(bool), forget(&texture), is_drawing().
    THE DIFFERENCES FROM C#, both from Rust's borrows: begin takes the pass
    to read its target and end takes THE SAME pass again to record into it
    (C#'s End() keeps the pass); and the drawing calls return nothing - a
    call outside begin/end is ignored and reported by the next end() as an
    Err that counts them (C# throws at the call). No text: the graphics crate
    draws none (the Animation crate's text layer will).

LIT MESHES, CAMERAS AND THE ORBIT CONTROLS (C#: Drawing3D).
    MeshShapes::cube(size, color) / box_shape(Vec3 size, color) /
    plane(width, depth, color, uv_repeat) / sphere(radius, segments, rings,
    color)   -> MeshData (counter-clockwise front faces; `box` is a Rust
             keyword, hence box_shape; color None = white)
    MeshData { vertices: Vec<MeshVertex>, indices: Vec<u32> }: get_bounds(),
        transformed(Mat4), MeshData::combine([&a, &b])
    MeshVertex { position, normal, tex_coord, color } (48 bytes, Pod)
    context.create_mesh(&data, label)? / create_mesh_from(&vertices, &indices, label)?
    MeshRenderer::new(&context)? / context.create_mesh_renderer()?
    renderer.create_material(base_color, MaterialOptions { base_color_texture,
        double_sided, label, sampler })?        -> LitMaterial (a shared handle)
        material.set_shininess(32.0) / set_specular_strength(0.25) /
        set_unlit / set_alpha_mode(MaterialAlphaMode::Auto|Opaque|Mask|Blend) /
        set_alpha_cutoff / set_texture_transform(Affine2) / set_base_color;
        is_translucent()
    renderer.begin(&pass, &camera, Some(&lighting))?   (None: the defaults)
    renderer.draw(&mesh, &material, world)? / draw_instanced(&mesh, &material, &worlds)?
    renderer.end(&mut pass)?
        Blinn-Phong with the key light and the ambient colour; the opaque
        draws first, then the translucent ones (alpha-blended, depth read
        only); consecutive draws of one mesh and material are ONE instanced
        draw; a mirroring world matrix draws with clockwise front faces.
        last_draw_call_count(), last_instance_count(). As with the sprite
        batch, begin takes the pass and end takes it again; draw returns an
        Err outside begin/end or for a material of another renderer.
    DEPTH on a canvas: the host has no depth buffer. A 3D layer keeps its own
        depth texture (create_depth_texture in resize), begins the frame with
        begin_canvas_frame(frame, Some(&depth)) and the pass with
        begin_render_pass(None, Some(1.0)): it LOADS the colour and clears
        only its own depth.
    SceneLighting { light_direction, light_color, light_intensity,
        ambient_color, ..., environment: Option<EnvironmentMap>, ... } with
        C#'s defaults (SceneLighting::default()); see IMAGE-BASED LIGHTING
        below for the environment.
    Camera { position, target, up, projection: ProjectionKind::Perspective |
        Orthographic, field_of_view, orthographic_height, near_plane,
        far_plane }: view_matrix(), forward(), get_projection_matrix(aspect),
        get_view_projection_matrix(aspect) (= projection * view),
        world_to_screen(world, width, height). Right-handed, y up, depth 0..1.
        Not in C#: world_to_canvas(world, css_w, css_h) and canvas_to_ray(x,
        y, css_w, css_h) (PICKING AND LABELS below), fit_distance(radius,
        aspect) and fit_orthographic_height(radius, aspect) (CAMERA CONTROL
        below).
    OrbitCamera: target, yaw (pub), pitch()/set_pitch (clamped to just under
        90 degrees), distance()/set_distance (at least min_distance),
        rotate(dyaw, dpitch), zoom(factor), position(), apply(&mut camera),
        OrbitCamera::orbit_position(target, distance, yaw, pitch).
    OrbitController::new(orbit) - the viewer's controls in one type
    (OrbitController::with_limits(orbit, limits) for limits of your own):
        controller.input(&input)   from the layer's CanvasRenderer::input
                                   (true when it changed the camera)
        controller.update(frame.info.time);
        controller.set_viewport(&camera, css_height);
        controller.apply(&mut camera)
                                   once per frame, in this order
        * drag with the left button, a pen's tip or ONE finger: orbit. THE
          CONTENT FOLLOWS THE POINTER: a drag to the right swings the scene
          to the right under the pointer (yaw falls by yaw_per_pixel = 0.008
          per CSS pixel; the camera moves the other way); a drag down tips
          the near side down (pitch grows by pitch_per_pixel = 0.006).
        * drag with the right or middle button, or with Shift held, or a
          pen with its side button: PAN (CAMERA CONTROL below).
        * wheel: a step towards the visitor zooms out (x1.1), away zooms in
          (x0.9). A trackpad pinch and a two-finger trackpad scroll arrive
          as wheel events and zoom.
        * TWO fingers: pinch - the distance follows the fingers' spread -
          and, while panning is on, the target follows their midpoint, both
          at once.
        * an ORTHOGRAPHIC camera zooms too: apply also sets its
          orthographic_height in proportion to the distance, so the wheel
          and the pinch scale the picture and the distance limits bound it.
        * auto-orbit: yaw = 0.7 + 0.18 * time until the first drag or pan
          (from the frame's time, so it stops with the clock);
          set_auto_orbit(true) starts it again from where the camera is.
        * OrbitLimits (default): pitch 0.05..1.35, distance 3..30 (the model
          viewer's on Android); every input stays within them. Negative
          pitch limits put the camera below the target, looking up.
        orbit, limits, the speeds, pan_enabled and pan_bounds are pub
        fields; is_auto_orbiting(), pressed_pointer_count(), frame_box(..).
    MeshRenderer shades with the key light and the ambient colour only: the
    local lights, shadows and exposure of SceneLighting are the model
    renderer's. It draws into 1- and 4-sample targets alike.

C# AND RUST SIDE BY SIDE (sprites):

    C# (CodeBrix.WebGpu.Graphics)                Rust (codebrix-webgpu-graphics)
    --------------------------------------------  ---------------------------------------------
    var batch = new SpriteBatch(ctx);            let mut batch = SpriteBatch::new(&ctx)?;
    // every frame, inside a render pass         // every frame, with the pass of a canvas frame
    batch.Begin(pass);                           batch.begin(&pass, SpriteBatchBeginOptions::default())?;
    batch.Draw(atlas,                            batch.draw_region(&atlas,
        new RectangleF(10, 10, 64, 64),              RectangleF::new(10.0, 10.0, 64.0, 64.0),
        new RectangleF(64, 0, 64, 64),               RectangleF::new(64.0, 0.0, 64.0, 64.0),
        Color4.Red);                                 Color4::RED);
    batch.Draw(atlas, new Vector2(200, 120),     batch.draw_rotated(&atlas, Vec2::new(200.0, 120.0),
        cell, Color4.White, angle,                   cell, Color4::WHITE, angle,
        new Vector2(32, 32), Vector2.One);           Vec2::splat(32.0), Vec2::ONE);
    batch.PushTransform(                         batch.push_transform(Affine2::from_translation(c)
        Matrix3x2.CreateRotation(a) *                * Affine2::from_angle(a));
        Matrix3x2.CreateTranslation(c));
    batch.FillCircle(Vector2.Zero, 8,            batch.fill_circle(Vec2::ZERO, 8.0,
        Color4.Yellow);                              Color4::YELLOW, 0);
    batch.PopTransform();                        batch.pop_transform()?;
    batch.DrawLine(a, b, Color4.White, 2);       batch.draw_line(a, b, Color4::WHITE, 2.0);
    batch.End();                                 batch.end(&mut pass)?;

C# AND RUST SIDE BY SIDE (a lit mesh under an orbit camera):

    C# (CodeBrix.WebGpu.Graphics)                Rust (codebrix-webgpu-graphics)
    --------------------------------------------  ---------------------------------------------
    var cube = ctx.CreateMesh(MeshShapes.Cube(2f)); let cube = ctx.create_mesh(&MeshShapes::cube(2.0, None), None)?;
    var meshes = new MeshRenderer(ctx);          let mut meshes = MeshRenderer::new(&ctx)?;
    var red = meshes.CreateMaterial(Color4.Red); let red = meshes.create_material(Color4::RED,
                                                     MaterialOptions::default())?;
    var orbit = new OrbitCamera { Distance = 8 }; let mut orbit = OrbitCamera::new(); orbit.set_distance(8.0);
    var camera = new Camera();                   let mut controls = OrbitController::new(orbit);
                                                 let mut camera = Camera::new();
    // input (the viewer's own code in C#)        // in CanvasRenderer::input
    userYaw -= dx * 0.008f;                      controls.input(input);
    orbit.Pitch += dy * 0.006f;
    orbit.Zoom(wheelUp ? 0.9f : 1.1f);
    // every frame                               // every frame, in CanvasRenderer::render
    orbit.Yaw = 0.7f + time * 0.18f + userYaw;   controls.update(frame.info.time);
                                                 controls.set_viewport(&camera, css_height);
    orbit.Apply(camera);                         controls.apply(&mut camera);
    using var frame = ctx.BeginFrame(target,     let mut gf = ctx.begin_canvas_frame(frame, Some(&depth));
        depth);                                  let mut pass = gf.begin_render_pass(None, Some(1.0))?;
    using var pass = frame.BeginRenderPass(
        Color4.Black, 1f);
    meshes.Begin(pass, camera,                   meshes.begin(&pass, &camera, Some(&SceneLighting::default()))?;
        new SceneLighting());
    meshes.Draw(cube, red,                       meshes.draw(&cube, &red,
        Matrix4x4.CreateRotationY(t));               Mat4::from_rotation_y(t))?;
    meshes.DrawInstanced(cube, red, worlds);     meshes.draw_instanced(&cube, &red, &worlds)?;
    meshes.End();                                meshes.end(&mut pass)?;

    The complete versions (layers on the host's clock, textures made in code,
    the legend of the controls in HTML over the canvas) are
    samples/GraphicsDemo/frontends/graphics/src/sprites.rs and lit_shapes.rs.

CAMERA CONTROL: PANNING AND FRAMING (not in C#).
    PANNING is on by default (controller.pan_enabled = true; false gives
    the controller without it: every drag orbits, two fingers only zoom).
    What pans: a drag with the right or the middle mouse button, a drag
    with Shift held (left button, pen tip or one finger), a pen with its
    side button pressed, and two fingers (the target follows their
    midpoint while their spread still zooms). The mode is decided at every
    move from that move's buttons and Shift, so pressing a second button or
    Shift in the middle of a drag switches between orbiting and panning
    without a jump. Ctrl, Alt and Meta change nothing. The right button
    reaches the layer because the canvas host suppresses the browser's
    context menu by default (WebGpuCanvas's suppress_context_menu).
    THE FEEL: the target moves in the screen plane, and the content at the
    target's depth follows the pointer exactly at any zoom - for that the
    controller needs the camera and the canvas's CSS height every frame:
        controller.update(frame.info.time);
        controller.set_viewport(&camera,
            frame.info.height as f32 / frame.info.pixel_ratio as f32);
        controller.apply(&mut camera);
    Until set_viewport is called it assumes Camera::default() on a canvas
    OrbitController::ASSUMED_CANVAS_HEIGHT (600) CSS pixels high. Only the
    camera's projection, field of view and orthographic height are used.
    A pan stops the auto-orbit, as a drag does (with panning on, nearly any
    two-finger gesture does); set_auto_orbit(true) resumes it around the
    panned target.
    pan_bounds: Option<(Vec3, Vec3)> (default None, anywhere): a box the
    target is held inside after every input.
    FRAMING A BOX:
        controller.frame_box(min, max, &camera, aspect_ratio) -> bool
            The target becomes the box's centre and pan_bounds the box. A
            PERSPECTIVE camera gets the distance at which the box's bounding
            sphere fits in view (camera.fit_distance(radius, aspect)), so the
            box stays in view from every direction the user can orbit to;
            the distance limits become that distance times
            FRAME_MIN_DISTANCE_FACTOR (0.3) and FRAME_MAX_DISTANCE_FACTOR (3).
            An ORTHOGRAPHIC camera gets a view height that fits the sphere
            (camera.fit_orthographic_height: 2 x radius / min(aspect, 1)),
            set by the next apply, and its zoom moves between the same
            factors. The yaw, the pitch and its limits and the auto-orbit are
            left alone. The corners may come in either order; false, and
            nothing changed, when a corner is not finite. The camera's
            planes are NOT changed: keep far_plane beyond the farthest
            distance plus the box's radius.
        controller.frame_box(model.bounds_min(), model.bounds_max(), &camera,
            width / height);        // a model drawn at the identity
    Pass a WORLD-space box for a model under a world matrix, and
    model.culling_bounds() for an animated model (its clips reach beyond the
    rest-pose bounds). The GraphicsDemo sample frames every model scene this
    way (frontends/graphics/src/model_layer.rs).
    THE ORTHOGRAPHIC ZOOM keeps the height in proportion to the distance;
    the proportion is taken from the camera at its first apply (no jump),
    set by frame_box, and taken again when the application changed
    orthographic_height itself since the last apply (its value wins, and
    the zoom goes on from it). Keep the Camera between frames: one rebuilt
    every frame with a fixed height keeps that height.

PICKING AND LABELS (not in C#).
    camera.world_to_canvas(world, css_w, css_h) -> CanvasProjection
        { x, y, depth, view_distance, in_front, in_view }: x and y in CSS
        pixels from the canvas's top-left corner, y down (meaningful only
        when in_front); depth the depth-buffer value (0 near, 1 far);
        view_distance the distance in front of the camera along its view
        (negative behind; sort or scale labels by it); in_view = in front,
        between the near and far planes and inside the canvas. There is NO
        occlusion test: a label of a thing behind a wall still shows.
        Pass the canvas size in CSS pixels (frame.info.width as f32 /
        frame.info.pixel_ratio as f32, the same for the height). To place
        an HTML label: absolutely positioned in a box laid over the canvas,
        `left: x px; top: y px`.
    camera.canvas_to_ray(x, y, css_w, css_h) -> Ray
        The ray through a canvas point (x and y as CanvasInput gives them,
        cast to f32): from the camera's position (perspective), or parallel
        rays from the plane through it (orthographic). Its direction is a
        unit vector, so hit distances are world units.
    Ray { origin, direction }: Ray::new(origin, direction) (normalizes),
        at(distance), intersect_box(min, max) -> Option<f32> (where it
        enters the box, or leaves it when it starts inside),
        intersect_transformed_box(min, max, world) -> Option<f32> (the box
        under any affine transform, tested exactly in its own space),
        nearest_box_hit(boxes) -> Option<(index, distance)> (the nearest of
        (min, max, Mat4) boxes, in the order given).
        let ray = camera.canvas_to_ray(x as f32, y as f32, css_w, css_h);
        if let Some((index, distance)) = ray.nearest_box_hit(instances.iter()
            .map(|i| (i.model().bounds_min(), i.model().bounds_max(), i.world))) {
            // instances[index] is under the pointer, `distance` away
        }
    It tests BOXES, not triangles: the rest-pose bounds miss an animated
    pose that reaches outside them, and culling_bounds() (every clip's
    reach) can be much larger than the model - pick with the one that suits
    the model. The GraphicsDemo sample's "Picking and labels" scene does both
    (frontends/graphics/src/demos/picking.rs).

MODELS: MODEL DATA, MODEL, INSTANCES AND THE MODEL RENDERER.
    The C# library loads a .glb/.gltf file, decodes its images and uploads
    everything in LoadModel(path). A browser has no file system and cannot
    wait for a fetch inside a call, so the crate takes BYTES that the
    application fetched, by one of two routes, both ending in the same
    Model:
    * GLTF FILES, served as they are by the backend (or packed into the
      frontend) and read IN THE BROWSER by the crate's glTF loader:
      GltfDocument::parse, then GltfModelSet::add and create_models (LOADING
      glTF MODELS below). Nothing is converted on the backend.
    * MODEL DATA, the crate's own binary format: the Graphics package's .NET
      types in CodeBrix.WebPlatform.WebGpu.Graphics.Models (ModelDataSet,
      ModelData, ModelVertex, ...; CORE API REFERENCE: MODEL DATA above),
      written into ONE binary blob by ModelDataWriter - for models an
      application MAKES in .NET (a ground, a shape built in code, data from
      another format). In the browser the crate READS the blob and uploads
      it: context.load_models(&bytes, &ModelLoadOptions::default())?. The
      blob's vertex, index and pixel ranges go to the GPU unchanged.
    The two loaders are separate entry points on purpose: an application
    that only loads blobs carries no glTF code in its WebAssembly file.
    THE BYTES COME FROM ANYWHERE; the crate never fetches. For a blob, two
    routes:
    * From the application's backend, through the platform's client: a
      [WireFile] GET action returns the blob, and the frontend calls the
      GENERATED <name>_bytes function and hands the bytes over:
          let bytes = generated::api::models::scene_data_bytes().await
              .map_err(|e| e.message())?;
          let models = ctx.load_models(&bytes, &ModelLoadOptions::default())?;
    * Packed into the frontend by the developer, for example
          const SCENE: &[u8] = include_bytes!("../assets/scene.cbmd");
          let models = ctx.load_models(SCENE, &ModelLoadOptions::default())?;
      (a file the developer wrote once with ModelDataWriter; a Dioxus asset
      fetched by the application's own code works the same way).
    THE FORMAT ("model data", file extension .cbmd): little-endian, a
    "CBMD" magic, a major and minor version and a section table; a reader
    refuses another MAJOR version and reads a higher minor one. It holds
    everything the C# loader's Model holds - nodes, meshes and primitives,
    the full metallic-roughness materials with every texture slot and
    KHR_texture_transform, textures as decoded RGBA8 with their sampler
    settings and colour space, skins, morph targets and animation clips -
    and a texture several models of one blob use is stored ONCE (C#'s
    ModelTextureCache). A blob holds one model or a set.
    THE RUST SIDE:
        ModelDataSet::read(&bytes)?      -> models: Vec<ModelData>, textures
            (borrows the bytes; every offset and reference is checked, and an
            error names the record: "model 0 'tree' mesh 0 primitive 2 names
            material 5, but there are 1")
        ctx.load_model(&bytes, &options)?   exactly one model (C# LoadModel)
        ctx.load_models(&bytes, &options)?  every model, textures shared
        ctx.create_models(&set, &options)?  from a set already read
        ModelLoadOptions { load_textures: true, mipmaps: true, label: None }
            (mipmaps are made on the CPU; sRGB colour maps, tangent
            generation and texture sizes are choices of whatever made the
            model data: GltfLoadOptions for a glTF file, the .NET side for a
            blob)
        ModelData (CPU only, right after ModelDataSet::read or from
            GltfModelSet::data_set): bounds() -> (min, max),
            culling_bounds(), has_blended_parts(), has_opaque_parts(),
            animation_names(), node_names() - facts without a device
    Model (a cheap handle: clones share it) - C#'s Model: name(), meshes(),
    materials(), default_material(), primitive_material(&prim), nodes(),
    root_nodes(), skins(), animations(), textures(), samplers(),
    slot_texture(&slot), vertex_buffer(), index_buffer(), index_format(),
    skin_buffer(), vertex_count(), index_count(), triangle_count(),
    bounds_min()/bounds_max()/bounds_center() (the rest pose, model space),
    warnings(), find_animation(name) / find_node(name) -> Option<usize>
    (C#'s -1 is None), create_instance() / create_instance_at(world). Not in
    C#: culling_bounds() -> (min, max), the bounds grown AT LOAD to hold every
    pose the model's clips reach (CULLING AND LEVEL OF DETAIL below);
    has_blended_parts(), has_opaque_parts() (by the materials of the parts it
    draws), animation_names() and node_names() (Vec<Option<&str>>, in clip
    and node order), animated_nodes() (the nodes any clip moves). CHANGING A
    LOADED MODEL (not in C#): material_mut(None | Some(i))? -> &mut
    ModelMaterial (factors, alpha mode, emissive_vertex_color - the
    emissive multiplied by the vertex colour, so one material carries many
    lamp colours) and set_material_texture(material, ModelTextureKind::
    BaseColor | MetallicRoughness | Normal | Occlusion | Emissive, texture)?
    (a GpuTexture of the application's own, moved, or an Rc<GpuTexture> to
    share one between slots and models; the slot's sampler and texture
    coordinates stay; base colour and emissive want an sRGB texture, the
    others a linear one; a normal texture switches normal mapping on; an ORM
    texture goes into both MetallicRoughness and Occlusion). Both need the
    ONLY HANDLE of the model: call them right after loading, before any
    create_instance, clone or draw - otherwise they return an error that says
    so (instances and the renderer would not see a later change). The data types
    keep the C# names: ModelMesh, ModelPrimitive (material: Option<usize>
    into materials(), None = the default material), ModelMorphTarget,
    ModelMaterial (MaterialAlphaMode Opaque/Mask/Blend), ModelTextureSlot
    (texture and sampler as Option<usize> into the model's lists; transform
    in C#'s Matrix3x2 order, transform_affine() for glam), ModelNode,
    NodeTransform, ModelSkin, ModelAnimation, ModelAnimationChannel,
    AnimationPath, AnimationInterpolation, ModelVertex (64 bytes),
    ModelSkinVertex (24 bytes). Their fields are public, read-only by
    convention (C#'s getters).
    ModelInstance - C#'s: pub world, pub visible, local_transforms(),
    get/set_local_transform(node, ..), node_matrices() (recomputed when the
    pose changed), get_node_world_matrix(node) (the node's CURRENT world
    matrix: read it after the frame's animation update; it panics on an
    index out of range), get/set_morph_weight(s), get_joint_palette(skin),
    reset_pose(), pose_version(); animation() (its AnimationPlayer, C#'s
    Animation), has_animation_player() and begin_pose_edit() (the next
    section). Not in C#, with defaults that change nothing:
        tint: Vec3 (ONE)          multiplies the base colour of every part
                                  (linear RGB; not the emissive)
        alpha: f32 (1)            multiplies the opacity; below 1 EVERY part
                                  is drawn among the blended parts (depth
                                  read, not written), so an opaque model
                                  can fade (it then shows its own inner
                                  faces where it overlaps itself); 0 or
                                  less skips the instance; a MASK material
                                  tests its cutoff first
        emissive_strength: f32 (1)  multiplies the emissive of every part
        never_cull: bool (false)  drawn even when outside the view
        casts_shadows, receives_shadows: bool (true)  LIGHTS AND SHADOWS
        get_node_world_matrix_named(name) -> Option<Mat4>   the first node
                                  of that name, to put a light, a glow or an
                                  emitter on a moving part:
            instance.get_node_world_matrix_named("lamp")
                .map(|m| m.transform_point3(Vec3::ZERO))
        set_node_visible(node, bool), set_node_visible_named(name, bool) ->
        bool (false: no such node), is_node_visible(node), show_all_nodes()
                                  a hidden node's parts are not drawn, nor
                                  those of its descendants (its pose is
                                  still computed); for a skinned mesh hide
                                  the node that carries the mesh, not a
                                  joint. Per instance; others unaffected.
    ModelRenderer (ctx.create_model_renderer()?) - C#'s: begin(&pass,
    &camera, Some(&lighting))?, draw(&instance)? any number of instances of
    any models (instances of one model become ONE instanced draw per
    primitive, per run of instances drawn alike), end(&mut pass)?;
    last_draw_call_count(), last_instance_count(), forget(&model).
    THE FINISHING STEP IN TWO HALVES (not in C#; C#'s End() is `end`):
        models.end_opaque(&mut pass)?    the OPAQUE and alpha-tested (MASK)
                                         parts of everything queued; the
                                         pair stays open (draw after it is
                                         an error)
        models.end_blended(&mut pass)?   the BLENDED parts (alpha mode
                                         BLEND, and every part of an
                                         instance whose alpha is below 1),
                                         back to front, and the pair ends
        models.end(&mut pass)?           both, in that order
    THE DRAWING ORDER of a scene with a sky: opaque, then the skybox, then
    blended - a blended part with only sky behind it would otherwise be
    covered by the sky:
        models.begin(&pass, &camera, Some(&lighting))?;
        for instance in &instances { models.draw(instance)?; }
        models.end_opaque(&mut pass)?;
        skybox.draw(&mut pass, &camera, Some(&lighting))?;
        models.end_blended(&mut pass)?;
    One model may mix opaque and blended parts. end_blended without
    end_opaque first draws only the blended parts. end_blended (and end) may
    be given a LATER pass of the same frame on the same target (the water
    surface needs that: WATER below); otherwise all of them take the pass
    begin was given. The blended parts are sorted by where each part is NOW:
    its centre moved by the instance's current morph weights and, for a
    skinned part, by the average of its skin's current joint matrices. It is
    a key per part and instance, not a per-triangle sort: interpenetrating
    translucent parts can still sort wrongly. A part that is both BLENDED
    and DOUBLE-SIDED is drawn twice, at its one place in that order - its
    back faces, then its front faces - so its far side blends under its
    near side (exact for convex shapes such as a glass ball or a bottle);
    set_two_pass_double_sided(false) draws it once, as C# does
    (two_pass_double_sided() reads it).
    CULLING is on by default (CULLING AND LEVEL OF DETAIL below): draw
    leaves out an instance entirely outside the camera's view.
    STATISTICS: last_draw_call_count() and last_instance_count() count both
    halves of the last pair; last_culled_instance_count(),
    last_shadow_draw_call_count(), last_local_light_count() and
    last_shadowed_spotlight_count() the same pair; frame_stats() ->
    ModelRenderStats { draw_calls, instances, culled_instances,
    shadow_passes, shadow_draw_calls, culled_lods, fading_lods,
    local_lights, shadowed_spotlights } over every pair of the current
    frame (the counts summed; the two light counts the most any pair
    chose). Read it after the frame's last pair.
    It shades with the crate's PBR shader: base colour, metallic-roughness,
    normal, occlusion and emissive maps, vertex colours, texture transforms,
    unlit, the alpha modes, double-sided, mirroring, GPU skinning and morph
    targets, up to eight directional lights, up to
    SceneLighting::MAX_LOCAL_LIGHTS local lights (point lights and
    spotlights) with shadows of the key light and of a few spotlights
    (LIGHTS AND SHADOWS below), and the ambient term: IMAGE-BASED
    by default (AmbientLighting::Environment, C#'s default: see the next
    section), or AmbientLighting::Hemisphere (SceneLighting's ambient_color as
    the sky, ambient_ground_color as the ground, around ambient_up).
    SKINNING AND MORPH TARGETS, as the data allows them: four joint
    influences per vertex (a glTF file's JOINTS_1 / WEIGHTS_1 are not read;
    the weights are divided by their sum), joint indices of 16 bits, morph
    targets applied BEFORE skinning (a part may have both). Skinned and
    morphing opaque instances batch as any others; blended parts are drawn
    one draw per part and instance, in their sorted order.

    C# AND RUST SIDE BY SIDE (loading and drawing a model):
    // C# (desktop)                              // Rust (browser layer)
    var model = ctx.LoadModel("tree.glb");      // backend: serves the .glb file as it is
                                                 let bytes = models::model_file_bytes("kit", "tree.glb")
                                                     .await.map_err(|e| e.message())?;
                                                 let document = GltfDocument::parse(&bytes)?;
                                                 // (fetch document.external_files(..) too)
                                                 let mut set = GltfModelSet::new();
                                                 set.add(&document, "tree", Some("kit"),
                                                     &|a| files.get(a).map(Vec::as_slice),
                                                     &GltfLoadOptions::default())?;
                                                 let model = set.create_models(&ctx,
                                                     &ModelLoadOptions::default())?.remove(0);
    var tree = model.CreateInstance(             let tree = model.create_instance_at(
        Matrix4x4.CreateScale(2.6f) *                Mat4::from_scale_rotation_translation(
        Matrix4x4.CreateTranslation(p));             Vec3::splat(2.6), Quat::IDENTITY, p));
    var models = ctx.CreateModelRenderer();      let mut models = ctx.create_model_renderer()?;
    // every frame                               // every frame, in CanvasRenderer::render
    using var pass = frame.BeginRenderPass(      let mut pass = gf.begin_render_pass(None, Some(1.0))?;
        sky, 1f);
    models.Begin(pass, camera, lighting);        models.begin(&pass, &camera, Some(&lighting))?;
    models.Draw(tree);                           models.draw(&tree)?;
    models.End();                                models.end(&mut pass)?;

    The complete version (the model files fetched once with a loading line
    and a failure message, one GltfModelSet whose models share their
    textures, the placements from the backend's JSON, the orbit camera, the
    order opaque - skybox - blended) is
    samples/GraphicsDemo/frontends/graphics/src/model_files.rs,
    viewer_scene.rs and viewer.rs; the .NET side (the files served by two
    [WireFile] actions, two models built in code as blobs) is
    samples/GraphicsDemo/backend.

LOADING glTF MODELS (not in C#: the desktop library reads the file itself).
    The crate reads glTF 2.0 in all three packagings - a .gltf with
    external files, a .gltf with data: URIs, a .glb - into the same model
    data a blob gives, so everything after it (Model, instances, the
    renderer) is the same. THE APPLICATION FETCHES; the crate does no
    networking. Two steps, because a browser cannot wait inside a call:
        // 1. parse the model file and ask which other files it names
        let document = GltfDocument::parse(&model_bytes)?;   // .gltf or .glb
        let wanted = document.external_files(Some("models/forest"), true);
        let mut files = HashMap::new();
        for file in &wanted {
            files.insert(file.address.clone(), fetch(&file.address).await?);
        }
        // 2. build the model data, the files handed over by address; upload
        let mut set = GltfModelSet::new();
        set.add(&document, "tree", Some("models/forest"),
            &|address| files.get(address).map(Vec::as_slice),
            &GltfLoadOptions::default())?;
        let models: Vec<Model> = set.create_models(&graphics,
            &ModelLoadOptions::default())?;
    GltfDocument::parse(&bytes)? checks the whole document and reads no
        buffer or image yet; is_binary(), warnings(), extensions_used(),
        extensions_required().
    document.external_files(base_address, images) -> Vec<GltfExternalFile
        { kind: GltfFileKind::Buffer | Image, index, uri, address }>: every
        buffer, then every image, whose URI is not a data: URI. `address` is
        the URI joined to the base address ('/' separators, percent escapes
        undone, "." and ".." resolved: "models/forest" + "../Textures/
        bark.jpg" is "models/Textures/bark.jpg"). The build asks its file
        function for EXACTLY these addresses: pass the same base address to
        both calls. images: false leaves the images out (for models loaded
        without textures).
    set.add(&document, name, base_address, &files, &options)? -> the model's
        index in the set; on an error the set is unchanged. model_count(),
        texture_count(), model_name(i), warnings(i), byte_size().
    set.create_models(&graphics, &ModelLoadOptions)? makes the GPU models of
        the models added SINCE THE LAST CALL and KEEPS their GPU textures,
        then releases its own copy of what it uploaded. set.data_set() is
        the set as a ModelDataSet, for graphics.create_models(&set.data_set(),
        ..) in one go. uploaded_texture(address, srgb, mipmaps).
    GltfLoadOptions { load_textures: true, mipmaps: true,
        color_textures_srgb: true, generate_tangents: true,
        max_texture_size: 0, share_vertex_data: true } (the defaults).
        max_texture_size 0 keeps every image's size; otherwise an image is
        halved until both sides fit. It is the main lever on memory: decoded
        textures are uncompressed RGBA8 on the GPU (2048 x 2048 is 16 MiB).
    SHARING. Within one GltfModelSet an external image (in one colour space)
    is fetched, decoded and stored ONCE however many models name it - by
    ADDRESS, so two models in sibling folders that name ../Textures/bark.jpg
    share it - and with set.create_models it is uploaded once ACROSS calls:
    keep ONE set for the models of a scene that are loaded at different
    times. Images inside a model file (data: URIs, .glb buffer views), and
    external images loaded with no base address, are not shared. One image
    used as colour and as data is two textures (sRGB and linear). Two
    primitives that name the same accessors for every attribute and morph
    target (one mesh with two materials) store their vertices once
    (share_vertex_data; never for primitives whose normals or tangents are
    generated). A blob still loads as before (ctx.load_models).
    ERRORS AND WARNINGS. A buffer that is not supplied is an error, and so
    is an image that is not supplied while textures are loaded. Everything
    else the builder cannot use is a WARNING and is left out
    (set.warnings(i), and Model::warnings() after the upload). A damaged
    file is refused with a message that names the place, never with a
    panic. The errors appear in the BROWSER: check the application's model
    files in its own tests.
    THE PIXELS are as the file stores them: no EXIF rotation, no ICC profile
    (IMAGES, in THE codebrix-webgpu CRATE); a 16-bit PNG keeps its high byte.
    Missing normals are generated as area-weighted smooth normals, missing
    tangents (for a normal map) per triangle from the texture coordinates,
    as the desktop C# library generates them (not MikkTSpace), so the
    browser and the desktop draw a model alike.
    REQUESTS: a .glb is one request, a .gltf with a .bin and images several.
    .glb, .gltf and .bin compress well; PNG and JPEG are compressed already.
    THE SUPPORT TABLE:
      .gltf with external files, with data: URIs; .glb      read
      triangles, triangle strips, triangle fans             read (strips and
                                                            fans become lists)
      POINTS, LINES, LINE_LOOP, LINE_STRIP                  REFUSED: the
                                                            primitive is left
                                                            out, with a warning
      every accessor component type, normalised integers,
        byte strides, interleaved and sparse accessors      read
      KHR_mesh_quantization                                 read (a normalised
                                                            integer divided by
                                                            its type's maximum)
      NORMAL missing                                        generated
      TANGENT missing, with a normal map                    generated
                                                            (generate_tangents)
      TEXCOORD_0, TEXCOORD_1, COLOR_0 (VEC3 or VEC4)        read
      TEXCOORD_2 and higher                                 samples TEXCOORD_0,
                                                            with a warning
      JOINTS_0 / WEIGHTS_0                                  read (weights
                                                            divided by their sum)
      JOINTS_1 / WEIGHTS_1                                  not read (four
                                                            influences)
      morph targets (POSITION, NORMAL, TANGENT), default
        weights on mesh and node                            read
      animations: translation, rotation, scale, weights;
        STEP, LINEAR, CUBICSPLINE                           read
      KHR_animation_pointer                                 a pointer to a
                                                            node's TRS or
                                                            weights is read;
                                                            others left out,
                                                            with a warning
      node TRS and matrices (a sheared matrix kept fixed)   read
      scenes                                                the default scene
                                                            (the first when
                                                            none is named)
      cameras, KHR_lights_punctual, KHR_materials_variants,
        KHR_xmp_json_ld                                     ignored
      materials: metallic-roughness, normal, occlusion,
        emissive, alpha modes, double sided                 read
      PNG and JPEG textures (external, data: URI, buffer
        view); samplers; KHR_texture_transform              read
      KHR_materials_emissive_strength, KHR_materials_unlit  read and drawn
      KHR_materials_ior, _specular, _clearcoat,
        _transmission, _sheen, _volume, _iridescence,
        _anisotropy, _diffuse_transmission,
        _pbrSpecularGlossiness                              captured,
                                                            APPROXIMATED when
                                                            drawn (a warning
                                                            names them)
      KHR_texture_basisu (KTX2), EXT_texture_webp,
        EXT_texture_avif, MSFT_texture_dds, any image that
        is neither PNG nor JPEG                             REFUSED: the
                                                            texture is left
                                                            out, with a
                                                            warning naming the
                                                            format (a PNG or
                                                            JPEG fallback
                                                            source is used
                                                            when the file has
                                                            one)
      a PNG or JPEG that does not decode (damaged, CMYK,
        larger than 8192)                                   the texture is
                                                            left out, with the
                                                            decoder's message
      KHR_draco_mesh_compression REQUIRED                   REFUSED: an error
                                                            from parse
      KHR_draco_mesh_compression used, not required         the uncompressed
                                                            fallback is read;
                                                            a primitive with
                                                            only compressed
                                                            data is left out
      EXT_meshopt_compression REQUIRED                      REFUSED: an error
                                                            from parse
      another REQUIRED extension the loader does not know   read without it,
                                                            with a warning
      EXT_mesh_gpu_instancing                               not expanded (the
                                                            mesh is drawn once)
      glTF 1.0, or a minVersion above 2.0                   REFUSED: an error
      a damaged or invalid document                         REFUSED: an error
                                                            with the place
      a damaged accessor                                    that primitive,
                                                            channel or skin is
                                                            left out, with a
                                                            warning
      a missing external buffer                             an error from add

LIGHTS AND SHADOWS (not in C#; the model renderer's - the MeshRenderer and
the skybox ignore them).
    LOCAL LIGHTS in SceneLighting::local_lights: Vec<LocalLight> (default
    empty):
        PointLight::new(position, color, intensity, range)
        SpotLight::new(position, direction, color, intensity, range,
            inner_angle, outer_angle)          .with_shadows(true)
        lighting.local_lights.push(PointLight::new(..).into());
    The light reaching a surface is color x intensity x falloff(distance,
    range) (x the cone for a spot), shaded like a directional light of that
    colour: at 1 m a local light of intensity i lights like a directional
    light of intensity i. The falloff is the inverse square, eased to
    EXACTLY 0 at `range` (glTF KHR_lights_punctual's window; f32::INFINITY
    for a plain inverse square). A spot's angles are radians from its axis:
    full within inner_angle, nothing from outer_angle, smooth between.
    LocalLight::falloff(d, range), spot.cone(angle), light.light_at(point)
    and light.importance(camera_position) are the same arithmetic on the
    CPU. A light with intensity 0, a black colour or range 0 lights nothing.
    THE CEILING: at most SceneLighting::MAX_LOCAL_LIGHTS are shaded in a
    pair (a fixed number, not a setting). When more are given, every begin
    picks, among the lights whose reach (a sphere of `range`) gets into the
    camera's view, those of highest importance (brightness over the squared
    distance to the camera, at least 1 m), and skips the rest for that pair.
    Lights are read at every begin, so they can move every frame. A pair
    without local lights uses shader variants without them and costs
    nothing more. Limits: no shadows from point lights, no light cookies or
    profiles, a point's highlight (no area lights).
    THE SUN'S SHADOW, SceneLighting::sun_shadows: SunShadowSettings
        { enabled: false, map_size: ShadowMapSize::Size2048, region_min,
          region_max: (-10, -10, -10) .. (10, 10, 10), softness: 9,
          strength: 1.0 }                                  (the defaults)
        SunShadowSettings::covering(min, max)   on, over that region
    "The sun" is the KEY light (light_direction, light_color,
    light_intensity); the additional directional lights never cast. ONE
    region, ONE map, NO cascades: outside the box nothing is in the sun's
    shadow, and a texel is the box's width over the map size, so keep the
    box as small as the scene allows (the box given to frame_box is a good
    one). Casters between the box and the sun still cast into it. Map
    sizes Size1024 / Size2048 / Size4096, at 4 bytes a texel. softness is
    the samples averaged per pixel (1 a hard edge, 4, 9, 16, 25); strength
    0..1 how much of the sun's light the shadow takes (ambient and
    image-based light are never shadowed).
    SPOTLIGHT SHADOWS: SpotLight::casts_shadows (with_shadows(true)) and
    SceneLighting::spot_shadows: SpotShadowSettings { map_size: Size1024,
    softness: 9, strength: 1.0 }. At most SceneLighting::
    MAX_SHADOWED_SPOTLIGHTS cast in a pair: the first casting spotlights
    among the chosen local lights, in importance order; the others still
    light, without shadows. A spot's shadow covers its cone out to its range
    (up to 85 degrees from its axis).
    PER INSTANCE: casts_shadows and receives_shadows (both true by default).
    What casts: the opaque and alpha-tested parts (with their cut-out) of an
    instance that casts, in its CURRENT pose (skins and morphs included);
    never blended parts, hidden nodes, or an instance whose alpha is below 1.
    What receives: every lit part of an instance that receives, blended and
    faded ones too. Each shadow pass culls against its own light's volume,
    so a caster behind the camera still casts.
    THE CALLS DO NOT CHANGE: switch shadows on in the SceneLighting and draw
    as always (begin, draw, end_opaque, skybox, end_blended). The pair's
    first finishing call draws the shadow passes on a command encoder of the
    renderer's own and submits it at once, before the frame the application
    records into, so the maps are finished before anything samples them;
    several pairs in one frame each keep their own maps. Submit each frame
    before the next frame's first begin (the canvas host does).
    models.sun_shadow_map() -> Option<SunShadowMap> (from the pair's first
    finishing call until the next begin) hands the sun's map and its
    numbers to another renderer (the water surface uses it).
    The bias against shadow acne is fixed, not a setting.

CULLING AND LEVEL OF DETAIL (not in C#).
    FRUSTUM CULLING is ON by default: draw(&instance) leaves out an instance
    whose culling bounds, placed by its world matrix, lie entirely outside
    the camera's view (behind it and beyond far_plane included) - nothing
    of it is uploaded or drawn. It was not visible anyway, so a scene looks
    exactly as without; only the counts change.
        models.set_frustum_culling(false)    every visible instance drawn
                                             (C#'s behaviour);
                                             frustum_culling() reads it
        instance.never_cull = true           drawn even when outside
        models.view_volume() -> Option<&CullVolume>   the view the current
                                             pair culls against
    THE BOUNDS: Model::culling_bounds() (and ModelData::culling_bounds()),
    grown AT LOAD to hold every pose the model's clips reach (every clip
    sampled over the rest pose, skinned parts per joint, morph targets at
    every weight from 0 to 1), with a small margin for a model with clips.
    A model without clips keeps its exact box. NOT covered: a pose set by
    hand beyond the clips, a vertex an application shader moves, and two
    blended clips that each move a different node of one chain far at once
    - use never_cull for such an instance. An instance culled for the camera
    that still casts into a shadow map is uploaded for its shadow pass.
    CullVolume (the same test for an application's own use):
    CullVolume::from_camera(&camera, aspect), from_view_projection(matrix)
    (a light's orthographic matrix gives a box), from_planes(..);
    contains_point(p), intersects_box(min, max),
    intersects_transformed_box(min, max, world) - false only when the box is
    certainly outside.
    LEVEL OF DETAIL: one object, several models, chosen by the camera's
    distance - and NOTHING POPS: inside a fade band both levels are drawn,
    each through a complementary pixel pattern, so one dissolves into the
    other.
        let mut tree = LodInstance::new(Mat4::from_translation(position), vec![
            LodLevel::single(detailed.create_instance(), 60.0),
            LodLevel::new(three_bushes, 250.0),       // a SET of instances
            LodLevel::single(card.create_instance(), 800.0),   // cut-off
        ])?;                                          // most detailed first
        tree.fade_band = 0.15;
        // every frame, between begin and end_opaque:
        if models.select_lod(&mut tree)? != LodState::Culled {
            for level in tree.shown_levels_mut() {
                AnimationPlayer::update_all_to(&mut level.instances, time);
            }
        }
        models.draw_lod(&mut tree)?;
    LodLevel::new(instances, max_distance) / LodLevel::single(instance,
    max_distance): a level is used out to its max_distance; f32::INFINITY
    (last level only) means no cut-off; an empty level is a gap. A level
    instance's own world is its place WITHIN the object: it is drawn at
    lod.world * instance.world, and distances are measured to lod.world's
    translation. LodInstance::new refuses no level, a distance not above 0,
    distances that do not grow, and an infinite distance before the last.
    LodInstance fields: world, fade_band (default
    LodInstance::DEFAULT_FADE_BAND = 0.1: the last part of each level's
    range, as a fraction; 0 is a hard switch), hysteresis (default
    DEFAULT_HYSTERESIS = 0.02: a slack against flicker when the camera
    hovers at a boundary), never_cull. Methods: levels(), level_mut(i),
    set_max_distance(i, d)?, shown_levels_mut() (the levels shown at the
    last choice: animate only these), state() -> LodState, position(),
    select(camera_position, scale) (the choice without a renderer),
    bounds().
    LodState: NotDrawn, Culled, Level(i), Fading { from, to, progress }
    (to None: the cut-off fade), BeyondCutOff; coverage(level),
    is_shown(level), is_fading(), shown_levels().
    models.draw_lod(&mut lod)? and models.select_lod(&mut lod)? (the choice
    without drawing; draw_lod then chooses the same) go between begin and
    end_opaque. models.set_lod_distance_scale(scale) multiplies every level
    distance (a quality setting: above 1 shows detail further out).
    frame_stats().culled_lods and .fading_lods count the objects.
    THE RULES: an object is culled whole first (state Culled, nothing
    measured) when its bounds are outside the camera's view and outside
    every shadow pass's volume; then each level instance goes through draw's
    own rules (visible, alpha, its own culling, the shadow switches). The
    fade is an 8 x 8 ordered dither per PIXEL, so it looks the same at 1 and
    4 samples and needs no sorting; BLENDED parts fade by alpha instead (no
    holes; a pair in mid-fade is a little less opaque than one part). A
    fading level casts through the same pattern, so its shadow thins out
    with it (while an object dissolves, its holes also show the part of its
    own shadow that lies under it). The first fade of an object compiles
    its dithered shader variant: a short stutter is possible then. ONE
    LodInstance follows ONE camera (two views of one object: two
    LodInstances). The application supplies the simpler models.


IMAGE-BASED LIGHTING, THE SKYBOX, THE HDR PATH AND TONE MAPPING.
    As in C#, an ENVIRONMENT is three RGBA16Float cube maps baked on the GPU
    (render passes, no compute): the radiance (the environment itself, full
    mip chain), the diffuse irradiance, and the GGX-prefiltered specular
    radiance; the context's split-sum BRDF LUT completes the image-based
    lighting. EnvironmentMap is a cheap HANDLE (clones are the same
    environment; dropping the last clone releases it - C#'s Dispose).
    MAKING ONE:
        ctx.create_procedural_sky(Some(&sky), Some(&options))?   // C# CreateProceduralSky
            ProceduralSkyOptions { sun_direction, sun_color, sun_intensity,
            sun_angular_radius, sun_glow, zenith_color, horizon_color,
            ground_color, turbidity, intensity, up } (C#'s defaults; the
            default sun matches the default key light); None = defaults
        ctx.create_environment(&float_image, Some(&options))?    // a linear
            equirectangular panorama in memory (C# CreateEnvironment(FloatImage))
        ctx.create_environment_from_bytes(&hdr_bytes, None)?     // the BYTES
            of a Radiance .hdr file (C# CreateEnvironment(stream) for a .hdr),
            decoded in the browser by RadianceHdrDecoder (ported from the C#
            library). Other formats (PNG, JPEG, JPEG XL, OpenEXR) are NOT
            made into an environment by the crate (an environment needs
            linear floats, and PNG and JPEG are 8-bit): decode them where
            they are (the backend) into linear floats and pass a FloatImage.
        ctx.default_environment()?   the procedural sky with the defaults,
            baked on first use and kept by the context (C# DefaultEnvironment)
        ctx.brdf_lut()?              the 128 x 128 Rg16Float LUT (C# BrdfLut)
        EnvironmentOptions { cube_size, irradiance_size,
            irradiance_source_size, prefiltered_size, prefiltered_mip_levels,
            specular_sample_count, max_source_width, label } (C#'s defaults,
            clamped as C# clamps them). cube_size is the size of the cube the
            SKYBOX shows (default 512, a power of two from 16 to 4096; the
            context's default_environment uses 256): raise it for a
            panorama with fine detail - a star field or a 4096-wide panorama
            wants 1024, an 8192-wide one 2048. Its GPU memory grows with the
            square (six RGBA16Float faces with mips); the lighting, sized by
            irradiance_size and prefiltered_size, does not change with it:
                ctx.create_environment_from_bytes(&hdr, Some(&EnvironmentOptions {
                    cube_size: 1024, ..EnvironmentOptions::default() }))?
        FloatImage::new(w, h, rgb)? / black / from_rgba; get_pixel(x, y) ->
            (r, g, b), set_pixel. RadianceHdrDecoder::is_radiance_hdr(&bytes),
            RadianceHdrDecoder::decode(&bytes)? (an error that says what is
            wrong with the file).
    The bytes of a panorama come from anywhere, as model data does (MODELS
    above): the generated <name>_bytes call of a [WireFile] action that
    returns the .hdr file, or include_bytes! in the frontend. Making an
    environment is synchronous for the caller: it records and submits the
    baking passes at once (call it in prepare, or in a frame - the bake is
    submitted before the host submits the frame). The environment's
    accessors (radiance(), irradiance(), prefiltered()) are wgpu cube views
    (C# wraps them as GpuTextures); size(), prefiltered_mip_levels(),
    is_procedural(), and the six-layer arrays behind them.
    LIGHTING WITH IT: SceneLighting's environment (None = the context's
    default environment), environment_intensity, environment_rotation (about
    +Y) and ambient: AmbientLighting::Environment (the default) - exactly C#'s
    properties. The model renderer then takes its diffuse and specular
    ambient from the environment and the LUT.
    THE SKYBOX: SkyboxRenderer (ctx.create_skybox_renderer()?) - draw(&mut
    pass, &camera, Some(&lighting))? draws the lighting's environment with its
    intensity, rotation and exposure (C# Draw(pass, camera, lighting));
    draw_environment(&mut pass, &camera, &env, SkyboxDrawOptions { intensity,
    rotation, exposure })? (C#'s other overload); set_blur(0..1) (C# Blur);
    draw_count(). Draw it AFTER the opaque parts and BEFORE the blended ones,
    in the same pass: models.end_opaque, skybox.draw, models.end_blended (it
    tests depth at the far plane and writes none; clear depth to 1). Drawn
    after all models, it would cover every blended part that has only sky
    behind it.
    WHICH EXPOSURE. Three settings scale brightness:
      * ToneMapper::exposure - on the HDR path, THE ONE for the picture's
        brightness: it scales everything in the HDR target before the curve
        (models, skybox, glows, particles, water, bloom alike) and changes
        nothing about what blooms (bloom reads the target's values before
        tone mapping).
      * SceneLighting::exposure (default 1) scales what the model renderer,
        the skybox (draw with a lighting) and the water surface output, not
        the MeshRenderer, glows or particles. Leave it at 1 on the HDR path,
        or use it to balance those against the rest; on the path without a
        tone mapper it is the scene's brightness. Raising it also makes more
        of the scene pass the bloom threshold.
      * SkyboxDrawOptions::exposure - only for draw_environment (a skybox
        drawn without a lighting); give it the lighting's exposure so the sky
        matches the models.
    THE HDR PATH: draw the scene into an RGBA16Float HeadlessRenderTarget
    (the C# name is kept; "headless" means "not the canvas"; its reads are
    async and it has no file members in the browser), then tone map it into the
    canvas frame:
        ctx.create_headless_target(w, h, HeadlessTargetOptions { format:
            Rgba16Float, with_depth: true, depth_format: Depth24Plus })?
            (C# CreateHeadlessTarget(w, h, format, withDepth, depthFormat);
            the defaults are C#'s: Rgba8Unorm, true, Depth32Float)
        target.view() / color() / depth() / width() / height() / format()
        ToneMapper (ctx.create_tone_mapper()?): pub operator: ToneMapOperator
            { AcesFitted (default), KhronosPbrNeutral, Reinhard, AcesNarkowicz,
            Clamp }, pub exposure (default 1); apply_in_pass(&mut pass,
            target.color())? (C# Apply(pass, hdr)) or apply(&mut frame,
            target.color(), &destination)? (C# Apply(frame, hdr, dest), a pass
            of its own that CLEARS the destination);
            ToneMapper::map(linear, operator, exposure) is the same curve on
            the CPU (C#'s static Map); pub preserve_alpha (default false, the
            C# behaviour; no C# equivalent): KEEPS THE SCENE'S ALPHA - the
            source is read as premultiplied colour, the curve is applied to
            the straight colour and the result premultiplied again, and it is
            BLENDED over the target (premultiplied-alpha blending) instead of
            replacing it; apply() then clears its destination to transparent.
            Without it the pass writes the source's alpha as it is but
            replaces the target and tone-maps premultiplied colour as if it
            were straight (bright or dark rims where alpha is partial).
    ON A CANVAS LAYER the HDR path is two passes in the frame: the scene into
    the HDR target (cleared, with its own depth), then a pass on the canvas
    frame that LOADS (begin_render_pass(None, None)) and tone maps over the
    whole canvas. apply_in_pass maps the source onto the target pixel for
    pixel and ignores a viewport: for an INSET (a small second view), tone
    map into an 8-bit texture of the inset's size with apply, then draw that
    texture with a SpriteBatch where it belongs. The tone-mapped scene is opaque, so layers BELOW it are
    covered; put 2D layers above it - or keep its alpha:
    DRAWING ON THE PAGE WITH THE GRAPHICS CRATE (models over the page, HDR on):
        WebGpuCanvas { transparent: true, .. } with no frame in the styling;
        no skybox; the HDR target cleared to TRANSPARENT; the tone mapper
        keeping the alpha:
            tone.preserve_alpha = true;
            let mut pass = gf.begin_render_pass_to(&hdr.view(),
                Some(Color4::TRANSPARENT), Some(1.0))?;
            models.begin(&pass, &camera, Some(&lighting))?; ...; models.end(&mut pass)?;
            // (no skybox.draw)
            let mut pass = gf.begin_render_pass(None, None)?;
            tone.apply_in_pass(&mut pass, hdr.color())?;
        Without HDR: draw the models straight into the frame (the host's
        clear is transparent) and leave the skybox out. The sample's "On the
        page" scene (frontends/graphics/src/page_scene.rs) does this. Without the HDR target (C#'s "LDR
    path"), draw the models and the skybox straight into the canvas frame:
    the canvas's sRGB view encodes them.
    C# GenerateMipmaps is ctx.generate_mipmaps(&texture)? (on the GPU; the
    texture needs TEXTURE_BINDING and RENDER_ATTACHMENT); textures of several
    layers are ctx.create_texture_array(w, h, layers, format, usage,
    Texture2DOptions { .. })? with texture.create_cube_view(label)? for six.

    C# AND RUST SIDE BY SIDE (an environment, the skybox, an HDR target and
    the tone mapper, as the model viewer uses them):
    // C# (desktop)                              // Rust (browser layer)
    var sky = ctx.CreateProceduralSky(           let sky = ctx.create_procedural_sky(Some(
        new ProceduralSkyOptions {                   &ProceduralSkyOptions { sun_direction: sun,
            SunDirection = sun,                          turbidity: 2.0,
            Turbidity = 2f });                           ..ProceduralSkyOptions::default() }), None)?;
    var hdri = ctx.CreateEnvironment(            let bytes = environments::small_cathedral_bytes()
        "small_cathedral_1k.hdr");                   .await.map_err(|e| e.message())?;
                                                 let hdri = ctx.create_environment_from_bytes(
                                                     &bytes, None)?;
    var lighting = new SceneLighting {           let lighting = SceneLighting {
        LightDirection = sun,                        light_direction: sun,
        Environment = sky };                         environment: Some(sky.clone()),
                                                     ..SceneLighting::default() };
    var skybox = ctx.CreateSkyboxRenderer();     let mut skybox = ctx.create_skybox_renderer()?;
    var tone = ctx.CreateToneMapper();           let mut tone = ctx.create_tone_mapper()?;
    tone.Operator =                              tone.operator =
        ToneMapOperator.KhronosPbrNeutral;           ToneMapOperator::KhronosPbrNeutral;
    var hdr = ctx.CreateHeadlessTarget(w, h,     let hdr = ctx.create_headless_target(w, h,
        TextureFormat.RGBA16Float, true,             HeadlessTargetOptions { format:
        TextureFormat.Depth24Plus);                  wgpu::TextureFormat::Rgba16Float,
                                                     with_depth: true, depth_format:
                                                     wgpu::TextureFormat::Depth24Plus })?;
    // every frame                               // every frame, in CanvasRenderer::render
    using (var pass = frame.BeginRenderPass(     let mut gf = ctx.begin_canvas_frame(frame, None);
        hdr.View, Color4.Black)) {               {
                                                     let mut pass = gf.begin_render_pass_to(
                                                         &hdr.view(), Some(Color4::BLACK), Some(1.0))?;
        models.Begin(pass, camera, lighting);        models.begin(&pass, &camera, Some(&lighting))?;
        models.Draw(tree);                           models.draw(&tree)?;
        models.End();                                models.end_opaque(&mut pass)?;
        skybox.Draw(pass, camera, lighting);         skybox.draw(&mut pass, &camera, Some(&lighting))?;
                                                     models.end_blended(&mut pass)?;
    }                                            }
                                                 bloom.apply(&mut gf, hdr.color())?;  // optional
    using (var pass = frame.BeginRenderPass(     let mut pass = gf.begin_render_pass(None, None)?;
        Color4.Black, null))
        tone.Apply(pass, hdr.Color);             tone.apply_in_pass(&mut pass, hdr.color())?;

    The complete version (the query switches ?ldr and ?no-skybox, the
    tone-map controls, the panorama fetched on demand) is
    samples/GraphicsDemo/frontends/graphics/src/viewer.rs and
    viewer_scene.rs; the backend's EnvironmentsController serves the .hdr.

ANIMATION PLAYBACK, SKINNING AND MORPH TARGETS.
    As in C#, the clips are part of the model (Model::animations(); each a
    ModelAnimation of ModelAnimationChannels driving a node's translation,
    rotation, scale or morph weights with STEP, LINEAR or CUBICSPLINE keys),
    and each ModelInstance has its own PLAYER: instance.animation() (C#'s
    instance.Animation) hands out the AnimationPlayer of that instance; its
    playback state is made on first use. The player writes the instance's
    POSE - local transforms and morph weights - and the model renderer draws
    that pose: a SKINNED primitive is skinned on the GPU from the instance's
    joint palette, a primitive with MORPH TARGETS is morphed on the GPU from
    its morph weights. Nothing else is needed for either: load the model,
    play a clip, draw the instance.
    PLAYING (C#'s names, an overload by name gets `_named`):
        player.play(clip, PlayOptions::default())?          by index
        player.play_named("walk", PlayOptions::default())?   by name
            replaces every clip on its layer, weight 1
        player.cross_fade_named("run", 0.4, PlayOptions::default())?
            fades the layer's clips out while the new one fades in
        player.blend_named("wave", 0.5, PlayOptions { layer: 1, .. })?
            adds a clip with a weight; a higher layer overrides the
            properties it animates by its weight
        PlayOptions { loop_mode: AnimationLoopMode::Loop (default) /
            PingPong / Once (holds the last frame), speed: 1.0, start_time:
            0.0, layer: 0 } - C#'s default arguments; PlayOptions::looping(m)
        Each returns the clip's AnimationState (C#'s): pub loop_mode, speed,
        paused; time()/set_time() (a seek), normalized_time(), weight()/
        set_weight(), fade_to(weight, seconds, stop_when_zero), layer(),
        loop_count(), is_finished(), finished_this_update(),
        looped_this_update(), is_fading(), name(), duration(), stop().
        player.stop() / stop_layer(layer) / stop_state(index): what a
        stopped clip animated returns to the rest pose.
        player.get_state(i) / get_state_mut(i) / find_state(clip) /
        state_count() / is_playing() / set_speed() / set_paused();
        at most max_states() clips at once (default
        AnimationPlayer::MAX_STATES = 8, C#'s MaxStates; set_max_states(n)
        raises or lowers it for that instance's player - starting a clip at
        the limit stops the faintest one fading out, or fails with a message
        that names the limit; lowering it stops nothing already playing).
        ModelAnimationChannel::sample(time, &mut values)? evaluates one
        channel by itself (C#'s Sample).
    TIME COMES FROM THE FRAME - the player never reads a clock. In a layer's
    render, give it the frame's time:
        instance.animation().update_to(frame.info.time);
            advances by the difference to the time of its previous
            update_to: pausing the host's clock holds the pose, resuming goes
            on from it, the clock's speed scales it, and a SEEK of the clock
            moves the clips to the matching place (a looping or ping-pong
            clip lands where playing up to that time puts it; a fade or a
            finished once-clip does not run backwards). Its FIRST call only
            records the time and applies the pose: a page whose clock does
            not start at 0 shows the clips at their start. For that:
        instance.animation().update_to_since_zero(frame.info.time);
            (not in C#) poses the clips as if they had been playing since
            clock time ZERO: its first call advances by the time itself
            (loops wrapped, once-clips finished and listed in
            finished_clips(), fades completed), every later call is
            update_to. Start the clips before its first call. The two may be
            mixed on one player; update_all_to_since_zero(&mut instances,
            time) does many.
    or the frame's delta, as C#'s Update(deltaSeconds):
        instance.animation().update(frame.info.delta as f32);
            (0 while the clock is paused, scaled by its speed; a seek moves
            nothing - re-pose for the new time yourself, as the sample's
            Viewer does)
        AnimationPlayer::update_all_to(&mut instances, frame.info.time) /
        update_all(&mut instances, delta) do many instances (those whose
        player was never used are skipped).
        player.evaluate() writes the pose without advancing (after
        set_time, for example).
    C#'s ClipFinished CALLBACK is a polled list: after an update,
    player.finished_clips() holds the clip indices of the once-clips that
    ended in it - start the next clip from there.
    A POSE BY HAND (C#'s BeginPoseEdit and its `using`):
        {
            let mut pose = instance.begin_pose_edit();
            pose.local_transforms()[arm].rotation = Quat::from_rotation_z(a);
            pose.get_morph_weights(face)[0] = 1.0;
        }   // the edit ends when `pose` is dropped; the pose has changed
    (C# throws when an instance is read, drawn or animated during an open
    edit; in Rust the edit borrows the instance, so such code does not
    compile.) set_local_transform and set_morph_weight(s) change one value.

    C# AND RUST SIDE BY SIDE (playing an animation):
    // C# (desktop)                              // Rust (browser layer)
    var walker = fox.CreateInstance(world);      let mut walker = fox.create_instance_at(world);
    walker.Animation.Play("walk",                walker.animation().play_named("walk", PlayOptions {
        AnimationLoopMode.Loop, 1f, 0.26f);          start_time: 0.26, ..PlayOptions::default() })?;
    // later: switch clips                       // later: switch clips
    walker.Animation.CrossFade("dance", 0.4f);   walker.animation().cross_fade_named("dance", 0.4,
                                                     PlayOptions::default())?;
    // every frame                               // every frame, in CanvasRenderer::render
    walker.Animation.Update(deltaSeconds);       walker.animation().update_to(frame.info.time);
    models.Draw(walker);                         models.draw(&walker)?;

    C# AND RUST SIDE BY SIDE (a skinned model):
    // C# (desktop)                              // Rust (browser layer)
    var archer = ctx.LoadModel(                  // the .glb read as in LOADING glTF MODELS
        "character-archer.glb");                 let archer = set.create_models(&ctx,
                                                     &ModelLoadOptions::default())?.remove(0);
    var hero = archer.CreateInstance();          let mut hero = archer.create_instance();
    hero.Animation.Play("walk");                 hero.animation().play_named("walk",
                                                     PlayOptions::default())?;
    int arm = archer.FindNode("arm-left");       let arm = archer.find_node("arm-left");
    // every frame                               // every frame
    hero.Animation.Update(deltaSeconds);         hero.animation().update_to(frame.info.time);
    Vector3 hand = hero.GetNodeWorldMatrix(      let hand = arm.map(|a| hero.get_node_world_matrix(a)
        arm).Translation;                            .w_axis.truncate());
    models.Begin(pass, camera, lighting);        models.begin(&pass, &camera, Some(&lighting))?;
    models.Draw(hero);   // skinned on the GPU   models.draw(&hero)?;   // skinned on the GPU
    models.End();                                models.end(&mut pass)?;
    A morph-target model is the same code: its weights clip (or
    set_morph_weight) moves the targets, the renderer blends them.

    The complete versions are in samples/GraphicsDemo/frontends/graphics:
    viewer_actors.rs (the model viewer's animals, chest and bird - looping
    clips with offsets, a routine of cross-fades, re-posing after a seek of
    the clock), model_layer.rs and model_scene.rs (the skinned archer with a
    clip list, and a morph-target shape the backend builds in code).

ANIMATED TEXTURES, RENDER BUNDLES, INDIRECT DRAWS, COMPUTE, QUERIES AND READBACK.
    ANIMATED TEXTURES (C# AnimatedTexture): frames with durations on the GPU.
        ctx.create_animated_texture(w, h, &frames, &AnimatedTextureOptions {
            loop_mode, srgb, mipmaps, premultiply_alpha,
            default_frame_duration, frame_durations, max_array_bytes, storage,
            streaming_slots, label })?
            `frames` are RGBA8 frames of w x h (red first, tight rows): C#'s
            CreateAnimatedTexture(frames) with decoded frames. The frames come
            from anywhere, as model data does: the backend decodes a GIF or an
            animated image and sends the pixels (one [WireFile] blob, split
            with bytes.chunks_exact(w * h * 4)), or the application packs or
            makes them. TIMES ARE SECONDS (C#'s TimeSpan), with C#'s defaults
            (0.1 s a frame, Loop, sRGB, 256 MiB, 3 streaming slots).
            AnimatedTextureStorage::TextureArray (every frame a layer, GPU
            mipmaps if asked) or Streaming (a ring of layers the current frame
            is uploaded into), chosen by size as C# chooses.
        anim.get_frame_texture(time) -> &GpuTexture   the frame at that time
            (C# GetFrameTexture); get_frame_index(time), get_frame_texture_at(i)?,
            get_layer(time), array_texture(), frame_count(), total_duration(),
            get_frame_duration(i)?, loop_mode() / set_loop_mode(..).
        batch.draw_animated(&anim, time, destination, tint) /
        batch.draw_animated_rotated(&anim, time, position, tint, rotation,
            origin, scale)   C#'s SpriteBatch.Draw(AnimatedTexture, ..).
        Pass the FRAME's time (or the time since the animation started): the
        texture reads no clock, so pausing the host's clock holds the frame.
    RENDER BUNDLES (C# RenderBundleBuilder, GpuRenderBundle): record once,
        replay every frame.
        let mut b = ctx.create_render_bundle_for_pipeline(&pipeline, label);
            (or create_render_bundle(&target_view, samples, label), or
            create_render_bundle_with_formats(&formats, &RenderBundleOptions))
        b.set_pipeline(&pipeline)?   (Err if its formats are not the bundle's)
        b.set_bind_group / set_vertex_buffer / set_index_buffer / draw /
            draw_indexed / draw_indirect(..)? / draw_indexed_indirect(..)?
        let bundle = b.finish(label);   or b.finish_checked(label).await?
        pass.execute_bundle(&bundle);   every frame
        The builder records the calls and replays them into WebGPU's bundle
        encoder when it finishes (as C# does); WebGPU's verdict comes through
        take_errors, or awaited by finish_checked (C# Finish throws). A bundle
        whose depth format has no stencil aspect is stencil-read-only, as the
        browser's passes are.
    INDIRECT DRAWS (C# DrawIndirectArgs, DrawIndexedIndirectArgs,
        DispatchIndirectArgs): Pod records of 16, 20 and 12 bytes.
        let args = ctx.create_indirect_buffer_from(&[DrawIndirectArgs::new(6)],
            label)?;   INDIRECT | STORAGE: a compute shader may write it
        pass.draw_indirect(&args, 0)?; pass.draw_indexed_indirect(..)?;
        pass.multi_draw_indirect(&args, 0, count)? (in a browser, `count`
        single indirect draws); compute_pass.dispatch_indirect(&args, 0)?
    COMPUTE (C# ComputePipelineBuilder, GpuComputePipeline, ComputePass):
        let pipeline = ctx.create_compute_pipeline(&shader)   entry "main"
            .with_entry("..").with_bind_group_layouts(&[&layout])
            .with_label("..").build()?;   (or .build_checked().await?)
        let mut cp = graphics_frame.begin_compute_pass(&ComputePassOptions {
            label, timestamps });
        cp.set_pipeline(&pipeline); cp.set_bind_group(0, &group);
        cp.dispatch(x, y, z);   (C#'s y and z default to 1: write them)
        A canvas LAYER may begin a compute pass on its frame before its render
        pass: what the compute pass writes (a storage buffer, indirect
        arguments), the render pass reads in the same frame.
        ctx.create_pipeline_layout(label, &[&layouts]) (C#
        CreatePipelineLayout, without immediates) is shared with
        .with_pipeline_layout(&layout).
    QUERIES (C# GpuQuerySet, GpuTimer): TIMESTAMPS where the browser grants
        the feature - DETECT, NEVER REQUIRE. Ask for the feature where the
        adapter has it, and check what the device got:
            GraphicsContext::create(GraphicsContextOptions {
                optional_features: wgpu::Features::TIMESTAMP_QUERY, .. })
            (then hand graphics.gpu_context() to the canvas host), and
            ctx.has_feature(wgpu::Features::TIMESTAMP_QUERY).
        let timer = ctx.create_timer(2, label)?;   two queries per slot
        RenderPassOptions { timestamps: Some(timer.pass(0)?), .. } /
        ComputePassOptions { timestamps: Some(timer.pass(1)?), .. }
        timer.resolve(&mut graphics_frame);   once per frame, outside passes
        timer.read_milliseconds().await? -> Option<Vec<f64>>   (C#
            ReadMilliseconds; async here; None until a frame was resolved,
            and while another read is in flight)
        ctx.create_timestamp_query_set(n, ..)? / create_query_resolve_buffer
        (&set, ..)? / graphics_frame.resolve_query_set(&set, first, count,
        &buffer, 0)? for timestamps of your own; ctx.timestamp_period().
        OCCLUSION QUERIES ARE REFUSED by this version (create_query_set and
        create_occlusion_query_set return an error that says why): the
        browser backend of the wgpu version this package carries does not
        pass a render pass's occlusion query set on, so a pass that began an
        occlusion query would fail validation and lose its frame. The API
        (RenderPassOptions::occlusion_query_set, begin/end_occlusion_query)
        keeps C#'s shape for the wgpu upgrade that passes the set.
    TEXTURE READBACK: texture.read_pixels(mip).await? (tight rows, the
        texture's byte order; Depth32Float as 4 bytes a texel),
        read_rgba().await? (RGBA8/BGRA8 as RGBA), read_region(&region).await?,
        target.read_pixels() / read_rgba() / read_depth() .await? of a
        HeadlessRenderTarget (read_depth: Depth32Float only, one f32 a pixel,
        and an Err while the target draws with 4 samples: WebGPU cannot copy
        a multisampled depth texture).
        The texture needs COPY_SRC (create_render_texture,
        create_texture_from_rgba and Depth32Float depth textures have it).

    C# AND RUST SIDE BY SIDE (an animated texture):
    // C# (desktop)                              // Rust (browser layer)
    var anim = ctx.CreateAnimatedTexture(        let anim = ctx.create_animated_texture(w, h,
        frames, new AnimatedTextureOptions {         &frames, &AnimatedTextureOptions {
            LoopMode = AnimationLoopMode.PingPong,       loop_mode: AnimationLoopMode::PingPong,
            FrameDurations = new[] {                     frame_durations: vec![0.08],
                TimeSpan.FromMilliseconds(80) } });      ..AnimatedTextureOptions::default() })?;
    // every frame                               // every frame, in CanvasRenderer::render
    batch.Draw(anim, elapsed,                    batch.draw_animated(&anim, frame.info.time,
        new RectangleF(16, 16, 96, 96),              RectangleF::new(16.0, 16.0, 96.0, 96.0),
        Color4.White);                               Color4::WHITE);

    C# AND RUST SIDE BY SIDE (a render bundle):
    // C# (desktop)                              // Rust (browser layer)
    var rec = ctx.CreateRenderBundle(pipeline);  let mut rec = ctx.create_render_bundle_for_pipeline(
                                                     &pipeline, Some("ring"));
    rec.SetPipeline(pipeline);                   rec.set_pipeline(&pipeline)?;
    rec.SetBindGroup(0, globals);                rec.set_bind_group(0, &globals);
    rec.SetVertexBuffer(0, vertices);            rec.set_vertex_buffer(0, &vertices, BufferRange::default());
    rec.SetIndexBuffer(indices,                  rec.set_index_buffer(&indices, IndexFormat::Uint32,
        IndexFormat.Uint32);                         BufferRange::default());
    rec.DrawIndexed(36, 12);                     rec.draw_indexed(36, DrawIndexedOptions {
                                                     instance_count: 12, ..DrawIndexedOptions::default() });
    var bundle = rec.Finish();                   let bundle = rec.finish(None);
    // every frame                               // every frame
    pass.ExecuteBundle(bundle);                  pass.execute_bundle(&bundle);

    C# AND RUST SIDE BY SIDE (compute writing an indirect draw):
    // C# (desktop)                              // Rust (browser layer)
    var args = ctx.CreateIndirectBuffer(         let args = ctx.create_indirect_buffer_from(
        new[] { new DrawIndirectArgs(6) });          &[DrawIndirectArgs::new(6)], None)?;
    var update = ctx.CreateComputePipeline(cs)   let update = ctx.create_compute_pipeline(&cs)
        .WithBindGroupLayouts(layout).Build();       .with_bind_group_layouts(&[&layout]).build()?;
    // every frame                               // every frame, on the layer's frame
    using (var cp = frame.BeginComputePass()) {  {
                                                     let mut cp = gf.begin_compute_pass(
                                                         &ComputePassOptions::default());
        cp.SetPipeline(update);                      cp.set_pipeline(&update);
        cp.SetBindGroup(0, group);                   cp.set_bind_group(0, &group);
        cp.Dispatch(64);                             cp.dispatch(64, 1, 1);
    }                                            }
    using (var pass = frame.BeginRenderPass(     let mut pass = gf.begin_render_pass(None, Some(1.0))?;
        null, 1f)) {
        pass.SetPipeline(particles);             pass.set_pipeline(&particles);
        pass.SetBindGroup(0, drawGroup);         pass.set_bind_group(0, &draw_group);
        pass.DrawIndirect(args);                 pass.draw_indirect(&args, 0)?;
    }

    C# AND RUST SIDE BY SIDE (GPU time and a texture read back):
    // C# (desktop)                              // Rust (browser layer)
    var timer = ctx.CreateTimer(1);              let timer = ctx.create_timer(1, None)?;  // if has_feature
    var options = new RenderPassOptions {        let options = RenderPassOptions {
        Timestamps = timer.Pass(0) };                timestamps: Some(timer.pass(0)?), ..RenderPassOptions::default() };
    using (var pass = frame.BeginRenderPass(     { let mut pass = gf.begin_render_pass_with_options(
        null, 1f, options)) { /* draw */ }           None, None, Some(1.0), &options)?; /* draw */ }
    timer.Resolve(frame);                        timer.resolve(&mut gf);
    double[] ms = timer.ReadMilliseconds();      // in a spawned task, with a clone of the timer:
                                                 let ms = timer.read_milliseconds().await?;
    byte[] rgba = target.ReadRgba();             let rgba = target.read_rgba().await?;

    The complete versions are samples/GraphicsDemo/frontends/graphics/src/
    more_layer.rs (the layer: a panorama made in code as the background,
    the bundle, the compute pass and the indirect draw, two animated
    textures, the timer) and more_scene.rs (the reads twice a second, and
    the one-off checks of the texture, region, mapping and awaited paths).

ANTI-ALIASING: MULTISAMPLING PER TARGET (not in C#).
    A target draws with 1 sample per pixel (the default everywhere) or 4
    (4x MSAA): SAMPLE_COUNTS = [1, 4], the two every WebGPU implementation
    supports; any other count is refused with a message. The setting is the
    TARGET's, so two canvases (or a canvas and an HDR target) may differ,
    and it may change between any two frames: the buffers of each count are
    made once and kept.
    ON THE CANVAS: a CanvasFrameTarget the layer keeps (one per canvas
    layer), in place of a depth texture of its own:
        // prepare:
        self.canvas = Some(graphics.create_canvas_frame_target(
            CanvasFrameTargetOptions::default())?);
            // { sample_count: 1, depth_format: Some(Depth32Float) };
            // None for a 2D layer without depth
        // render:
        let canvas = self.canvas.as_mut().unwrap();
        canvas.set_sample_count(4)?;          // or 1, from a control
        let mut gf = graphics.begin_canvas_frame_on(frame, canvas)?;
        {
            let mut pass = gf.begin_render_pass(None, Some(1.0))?;
            models.begin(&pass, &camera, Some(&lighting))?; /* draw */
            models.end_opaque(&mut pass)?;
            skybox.draw(&mut pass, &camera, Some(&lighting))?;
            models.end_blended(&mut pass)?;
        }
        // when gf is dropped, the layer's resolved picture is laid over
        // the canvas
    It keeps its buffers at the canvas frame's size and format (no resize
    code); sample_count(), depth_format(), depth(), buffer_sets_made(),
    release_unused_buffers() (frees the other count's set). With 1 sample it
    is exactly begin_canvas_frame with a depth texture. With 4, every pass
    on it resolves; the frame's first pass clears the multisampled colour
    (to transparent when the pass loads); and when the GraphicsFrame is
    DROPPED one more draw lays the resolved picture over the canvas with
    premultiplied blending - WebGPU's resolve would REPLACE what the layers
    below drew, so the layer contract (draw over what is there) is kept
    this way. A LIMIT OF THAT: on a 4-sample canvas layer, a blend that
    READS what lies below (BlendPreset::Multiply, or a custom blend that
    depends on the destination) sees the layer's own transparent start, not
    the layers below; "over", alpha, opaque and additive blending look as
    with 1 sample.
    OFFSCREEN AND THE HDR PATH:
        ctx.create_headless_target_with_samples(w, h, options, 4)?
            (create_headless_target is the same with 1;
            HeadlessTargetOptions has no sample count) and
            target.set_sample_count(n)? at any time; sample_count(),
            multisampled_color(), release_unused_buffers().
    color() is ALWAYS the single-sampled texture, which every pass resolves
    into: tone map it, bloom it and read it exactly as with 1 sample (the
    ToneMapper and BloomRenderer refuse a multisampled source with a
    message). view() is the multisampled colour and depth with color() as
    its resolve view. With 4 samples a pass that LOADS continues from the
    multisampled buffer, so anything written into color() another way (a
    copy, a compute pass, bloom) is not seen by a later pass on the target,
    and that pass's resolve overwrites it: bloom comes after the target's
    LAST scene pass. read_depth() is an Err at 4 samples. On the HDR path the
    canvas frame itself can stay at 1 sample (the tone-map pass has no
    geometric edges).
    PIPELINES follow the target: every renderer of the crate keys its
    pipelines by pass.target().formats() (TargetFormats { color, depth,
    sample_count }); RenderTargetView has sample_count(), is_multisampled(),
    resolve_view(), formats(), without_depth() and with_resolve(4, view)?
    for views of the application's own. A pipeline of the APPLICATION's
    must do the same (PIPELINES above), or set_pipeline panics with the
    mismatch when the control is switched. Shadow maps always draw with 1
    sample. On the HDR path the resolve averages BEFORE tone mapping, so
    very bright edges stay slightly rough. Only geometric edges are
    smoothed: texture shimmer is the mipmaps' job. Memory: about four times
    the target's colour and depth, plus a resolve texture for a canvas. The
    GraphicsDemo sample's "Anti-aliasing" scene shows both paths
    (frontends/graphics/src/demos/antialiasing.rs).

EFFECTS: FRAME TIMINGS, GLOW, BLOOM AND PARTICLES (not in C#).
    FRAME TIMINGS (a measurement; nothing drawn depends on it):
        let mut timer = graphics.create_frame_timer(FrameTimerOptions {
            max_passes: 16, smoothing_frames: 30, label: None })?; // defaults
        // every frame, in render:
        timer.begin_frame();
        let scene = RenderPassOptions { timestamps: timer.pass("scene"),
            ..RenderPassOptions::default() };
        // ... gf.begin_render_pass_with_options(.., &scene) ...
        timer.set_count("draw calls", models.last_draw_call_count() as u64);
        timer.end_frame(&mut gf);       // after the last timed pass
        let line = timer.timings().to_string();
            // "cpu 1.20 ms (1.15) | gpu 2.40 ms (2.35): scene 2.00, ..."
    timings() -> &FrameTimings { frame, cpu_ms, cpu_ms_smoothed,
    gpu_available, gpu_frame, gpu_total_ms, gpu_total_ms_smoothed, passes:
    Vec<PassTiming { name, gpu_ms, gpu_ms_smoothed }>, counts }. CPU time is
    from begin_frame to end_frame (the page's clock: the crate's one clock
    reading, for this read-out only). GPU total is the SUM of the marked
    passes; passes marked with one name in a frame add up; smoothed is the
    mean of the last smoothing_frames values. GPU values ARRIVE A FEW FRAMES
    LATE (gpu_frame says which frame they are from), and browsers coarsen
    timestamps. A device without timestamp-query still gets a timer: pass()
    returns None, the passes go untimed, gpu_available is false and the
    line says "gpu n/a" - ask for the feature with optional_features
    (QUERIES above), never require it. The model renderer's shadow passes
    and bloom's own passes cannot be marked one by one: they are not inside
    the application's marked pass.
    THE GLOW LAYER: soft, round, camera-facing light sprites in the 3D scene
    (lamps, sparks, a sun glint) - a look, not a light.
        let mut glows = graphics.create_glow_layer()?;
        // in the scene pass, after end_opaque and the skybox:
        glows.begin(&pass, &camera, frame.info.time)?;
        glows.add(&Glow::new(position, 0.6, Vec3::new(1.0, 0.7, 0.3), 3.0));
        glows.add_all(&lamps);
        glows.end(&mut pass)?;
    Glow { position, size (the width in scene units: it fades to exactly 0
    at half of it), color (LINEAR), intensity, pulse }; Glow::new(..)
    .with_pulse(GlowPulse { rate (cycles a second), depth (0..1), phase
    (cycles) }) (default 1, 0.5, 0). The pulse is evaluated on the GPU from
    the time given to begin (the pass carries none), so pausing the clock
    freezes it and a seek jumps to the right phase. All glows of a pair are
    ONE instanced draw. Depth is tested, never written; the blend is
    additive (GlowBlend::Additive) keeping the target's alpha; into an 8-bit
    target without sRGB the addition is only approximate (prefer an sRGB or
    float target). set_texture(Some(texture)) gives every glow of the layer
    one shape (its colour times its straight alpha, top up); None is the
    round falloff. Limits: a hard edge where a glow meets a surface (no
    soft-particle depth read). add outside begin/end is ignored and reported
    by the next end. On the HDR path a glow brighter than the bloom
    threshold blooms.
    BLOOM (the HDR path only): bright things get a soft, wide halo.
        let mut bloom = graphics.create_bloom_renderer()?;
        bloom.settings = BloomSettings { threshold: 1.2, intensity: 1.0,
            ..BloomSettings::default() };
            // defaults: threshold 1.5, soft_knee 0.25, intensity 0.8,
            // radius 6 (levels, at most BloomRenderer::MAX_LEVELS)
        // after the target's last scene pass, outside any pass, before the
        // tone mapper:
        bloom.apply(&mut gf, hdr.color())?;
    It adds the halo INTO the scene texture itself (its alpha kept), so the
    tone mapper maps it as before. Brightness is the largest channel of the
    linear scene value; the soft knee eases bloom in around the threshold;
    the default threshold is above a white surface under the default key
    light, so ordinary lit surfaces do not glow. The scene texture must be
    a float format with RENDER_ATTACHMENT and TEXTURE_BINDING (an error says
    so otherwise), and the resolved one at 4 samples. It makes bright
    things look bright; it lights no surface. With the tone mapper's
    preserve_alpha the halo shows only over what the scene drew.
    BloomRenderer::bright_pass(..) and chain_sizes(..) are its arithmetic
    on the CPU, for tuning. A scene that never calls it pays nothing.
    GPU PARTICLES: a system simulated by a compute pass and drawn in one
    instanced draw (the glow layer's look).
        let mut sparks = graphics.create_particle_system(&ParticleSystemOptions {
            capacity: 4096, seed: 7, ..Default::default() })?;
            // defaults: capacity 1024 (at most ParticleSystem::MAX_CAPACITY),
            // seed 1, blend GlowBlend::Additive, max_step 0.1, label None
        sparks.emitter.shape = EmitterShape::Cone { radius: 0.05, angle: 0.35 };
        sparks.emitter.rate = 400.0;
        sparks.settings = ParticleSettings { acceleration: Vec3::new(0.0,
            -9.81, 0.0), drag: 0.4, intensity: 4.0, ..Default::default() };
        // every frame, outside any pass, before the pass that draws:
        sparks.emitter.place(world);    // position and +Y direction
        sparks.update(&mut gf, frame.info.delta)?;
        // in the scene pass, after end_opaque and the skybox:
        sparks.draw(&mut pass, &camera)?;
    system.emitter (ParticleEmitter: shape Point | Sphere { radius } |
    Box { half_extents } | Cone { radius, angle }, position, direction,
    spread, rate per second, speed_min/max, lifetime_min/max, place(world))
    and system.settings (ParticleSettings: acceleration, drag, sizes,
    colours (linear) and opacities at birth and death, intensity,
    fade_out) are read at every update. burst(count), reset(&mut gf)?,
    set_texture(..), capacity(), blend(), spawned_count(),
    simulated_time(), alive_at_most() (an upper bound worked out on the
    CPU, no readback: exact when lifetime_min equals lifetime_max).
    THE CLOCK: a delta of 0 (paused, the first frame, the frame after a
    seek) simulates and spawns nothing; the speed scales the delta; a delta
    above max_step is simulated as max_step (a tab brought back does not
    spawn a minute of particles). The same seed and the same deltas, emitter
    changes and bursts give the same particles on one GPU. Spawns are
    spread through a step along the emitter's path from its last position,
    so a moving emitter leaves a trail - and a BURST right after moving the
    emitter is smeared along that path too: move it in one step and burst in
    the next. At capacity the oldest particles are replaced. Limits:
    particles are NOT sorted (transparent ones overlap in buffer order); no
    collisions; one update per frame; a seek does NOT rewind them (a page
    opened at ?t=12 shows them as just after loading); each frame draws the
    whole capacity, so choose it to fit about rate x the longest life.
    The effects' shaders are EffectShaders::all() (THE BUILT-IN SHADERS).
    The GraphicsDemo sample's "Glow and bloom" and "Particles" scenes and
    its frame-timings panel use all of this (frontends/graphics/src/demos/).

WATER (not in C#).
    Three parts, used together or alone: a simulated WAVE FIELD (rings that
    spread, bounce and fade - rain, a pointer, a wake), OPEN-WATER WAVES (a
    swell that is an exact function of time), and a WATER SURFACE that draws
    either or both in a 3D scene with refraction, depth tint, caustics,
    reflections and highlights.
    THE WAVE FIELD (simulated on the GPU, a picture any shader can read):
        let mut field = graphics.create_wave_field(&WaveFieldOptions {
            columns: 256, rows: 256, size: Vec2::new(10.0, 10.0),
            ..Default::default() })?;
        field.set_rain(Some(WaveRain::default()));
        // CanvasRenderer::input, with every event:
        field.stir(input, &WaveStir::default(), &|x, y| pond.field_point(
            &camera.canvas_to_ray(x as f32, y as f32, css_w, css_h)));
        // render, every frame, outside any pass, before the passes that
        // read it:
        field.update(&mut gf, frame.info.delta)?;
    WaveFieldOptions (defaults): columns 256, rows 256 (8 to 2048), size
    (10, 10) in scene units, wave_speed 1 unit a second, damping 0.6 (the
    part of a wave's height left after one second), edge WaveEdge::Reflect
    (or Absorb { width }, WaveEdge::ABSORB, or Wrap), steps_per_second 120,
    max_steps_per_frame 8, label. A speed the step rate cannot hold is
    refused, and the message names the highest speed and the lowest
    steps_per_second that would hold it (set_wave_speed refuses the same
    way). drop(WaveDrop { position, radius, depth }), set_wake(slot,
    Some(WaveWake { position, radius, strength, bob, bob_rate }))? (slots
    0..WaveField::MAX_WAKES; a wake presses the path travelled since the
    last update, so a moving thing drags a trough the same at any frame
    rate), set_rain(Some(WaveRain { rate, radius, depth, area, seed })),
    set_mask(Some(&WaveMask))? (per cell 0 water, 1 solid wall, between:
    shallows that damp waves; WaveMask::new / from_fn / from_picture /
    add_disc), set_damping, set_edge, reset(&mut gf)?. stir: a PRIMARY
    press drops and starts wake slot 7, a move while down drags it, the
    release stops it; other buttons and pointers are left alone (for
    orbiting); the function maps CSS pixels to field units, None off the
    water. stir does not say whether it used an event: a layer that stirs
    with the left button on the water and orbits with it elsewhere decides
    at the press (field_point of the press's ray) and hands that press's
    moves and release to stir, the others to the orbit controller. The field
    steps at a FIXED rate in CLOCK seconds: pause stops it,
    speed scales it, every display rate gives the same water, and the
    picture is interpolated between steps, so slow motion is smooth (it is
    up to one step behind the clock). IT CANNOT BE REWOUND: a seek shows the
    water as it is now. ONE update per frame (a second one is an Err).
    surface() is the picture: an Rgba16Float texture of columns x rows, R
    the height, G and B the slopes, A the Laplacian (positive in troughs) -
    filterable, read with a linear sampler and textureSampleLevel.
    CpuWaveField is the same simulation on the CPU (slow; for tests and
    read-outs), with height_difference(&bytes) against the GPU picture read
    back. LIMITS: a height field - no splashes, spray, breaking waves or
    flow; one clean ring per drop; fixed in place; heights reach the CPU
    only by reading the texture back.
    OPEN-WATER WAVES (an exact function of time, in world x/z):
        let sea = OpenWaterWaves::from_wind(direction, wind_speed, 8, seed);
            // or OpenWaterWaves::new(vec![GerstnerWave { .. }, ..])?
        let y = sea.height_at(x, z, frame.info.time);   // float things
        let n = sea.normal_at(x, z, frame.info.time);
    from_wind: waves within 60 degrees of the wind, from its spectrum
    (stronger wind, longer and higher waves; scene units are metres; up to
    OpenWaterWaves::MAX_WAVES). new refuses waves that would loop over
    themselves, with the factor to scale their steepness by. displacement_at,
    max_height(), records(time) (the per-frame data for a shader of your
    own). Pause freezes them; a seek shows the right waves at once. LIMITS:
    repetition with few waves, no breaking, foam or spray, deep-water waves
    only, the waves pass through islands and walls.
    THE WATER SURFACE (WaterRenderer, WaterSurface) NEEDS THE OFFSCREEN
    PATH: it reads the scene behind it, and the canvas picture cannot be
    read. Draw the scene into a HeadlessRenderTarget (the HDR target, its
    depth Depth32Float), then, in this order:
        // pass A: the solid scene and the sky
        let mut pass = gf.begin_render_pass_to(&hdr.view(),
            Some(Color4::BLACK), Some(1.0))?;
        models.begin(&pass, &camera, Some(&lighting))?;
        for instance in &instances { models.draw(instance)?; }
        models.end_opaque(&mut pass)?;
        skybox.draw(&mut pass, &camera, Some(&lighting))?;
        drop(pass);
        let sun = models.sun_shadow_map();      // None without its shadow
        water.capture_scene(&mut gf, &hdr)?;    // outside any pass
        // pass B: LOADS colour and depth
        let mut pass = gf.begin_render_pass_to(&hdr.view(), None, None)?;
        water.draw(&mut pass, &WaterFrame { camera: &camera, lighting:
            Some(&lighting), time: frame.info.time, sun_shadow: sun.as_ref() },
            &pond)?;
        models.end_blended(&mut pass)?;         // the same pair, a later pass
        // glows, particles ...
        drop(pass);
        bloom.apply(&mut gf, hdr.color())?;     // optional
        let canvas = gf.target().cloned().unwrap();   // the canvas frame
        tone.apply(&mut gf, hdr.color(), &canvas)?;   // or apply_in_pass
    graphics.create_water_renderer()?; capture_scene copies the resolved
    colour and the depth of the target (an Err names a target without depth
    or with another depth format); draw needs a capture of THIS frame on the
    SAME target, and several surfaces share one capture (they do not see
    each other). last_stats() -> WaterRenderStats { captures, surfaces,
    vertices, capture_size, local_lights }, pipeline_count().
    WaterSurface (defaults): center Vec3::ZERO (y = the water level), size
    (10, 10) along local X and Z, rotation 0 (about +Y), grid 64 (1 to
    WaterSurface::MAX_GRID), field None (Some(WaterField::of(&field)): view,
    the field's size, height_scale 1, displace true), open_waves None,
    detail (WaterDetail: fine ripples for the normals - strength,
    wavelength, speed, seed), look (WaterLook). field_point(&ray) (where a
    ray meets the still water, in field units - for stirring),
    to_field(..), to_world(..), height_at(x, z, t) and normal_at(x, z, t)
    (the water level plus the OPEN-WATER waves only; the wave field's
    heights stay on the GPU), grid_cell().
    WaterLook (defaults): absorption (0.45, 0.09, 0.05) per unit,
    scattering_color (0.01, 0.05, 0.08), turbidity 0.12, refraction 0.25,
    reflectivity 1, roughness 0.06, sun_glint 1, caustics
    Some(WaterCaustics::default()) (strength 0.7, scale 1.4, speed 0.35,
    focus 0.5, depth_fade 4), screen_reflections true,
    local_light_highlights true, receives_shadows true, soft_shore 0.12,
    sheen WaterSheen (strength 0 = OFF; above 0 a thin-film rainbow sheen
    on the slopes, with scale, drift, slope_gain and height_gain).
    WHAT A WATER PIXEL IS: the scene behind it, shifted by the waves'
    normal (refraction, falling back to the unshifted view where the shifted
    point lies in front of the water) and tinted by the real depth to the
    bottom (absorption and lit scattering); caustics on the bottom, brighter
    under crests, fading with depth and dark in the sun's shadow; a Fresnel
    reflection of the environment mixed with a screen-space reflection of
    the scene; the key light's glint (in its shadow, none) and up to
    SceneLighting::MAX_LOCAL_LIGHTS local lights' highlights, in HDR, so
    they bloom; the optional sheen; and a soft shore where the water is
    shallow. The water writes depth, so what is drawn after it sorts
    against it. Time-driven parts are exact functions of WaterFrame::time;
    the wave field follows the clock's delta. It works at 1 and 4 samples
    (the capture reads the resolved colour and depth sample 0).
    A WAVE FIELD ON TOP OF OPEN-WATER WAVES (a wake on a lake): a second,
    small WaterSurface over the same capture, its grid lying on the lake's
    grid lines with the same cell, WaterField { displace: false,
    ..WaterField::of(&field) }, a few millimetres above the lake; drawn after
    the lake in the same pass B. The two then compute the same open-wave
    heights at the same vertices, and the seam does not show.
    LIMITS: not on the plain canvas path; TRANSLUCENT THINGS UNDER THE WATER
    ARE HIDDEN (they are drawn after the capture and behind the water's
    depth: make them opaque or alpha-tested); no planar reflection (what is
    off-screen, behind the camera or hidden falls back to the environment;
    the water is not reflected in itself; a reflection can be cut off
    raggedly at an object's edge); no spotlight shadows on the water (only
    the sun's); a surface, not a fluid (no splashes, spray, foam, breaking
    waves or flow); the camera above the water; screen-space refraction
    (nothing beyond the screen's edge); caustics only through the water; a
    finite plane. The shaders are WaterShaders::all() (THE BUILT-IN
    SHADERS). The GraphicsDemo sample's "Water pond" and "Water lake" scenes
    are the worked examples (frontends/graphics/src/demos/water_pond.rs and
    water_lake.rs).


WHAT A DEVELOPER PORTING C# CODE WILL NOT FIND, AND WHAT TO USE INSTEAD.
    This is the one list of what the C# library CodeBrix.WebGpu.Graphics has
    and this crate does not, because a browser has no equivalent (or, where
    said, because of the wgpu version), with what to use instead.
    * NATIVE WINDOWS AND SURFACES: WindowSurface, NativeWindowSource,
      CreateWindowSurface / WrapWindowSurface, SurfaceAcquireResult,
      CompatibleWindow. Use: codebrix-webgpu's canvas host (WebGpuCanvas)
      and a layer (begin_canvas_frame); it configures, resizes, acquires
      and presents.
    * FILES AND DECODERS: reading from a path or a stream (LoadModel(path|
      stream), CreateTextureFromFile / Stream, Model.SourcePath: the
      application fetches the bytes), CreateTextureFromEncoded / Image and
      TextureLoadOptions for formats other than PNG and JPEG,
      CreateAnimatedTexture(path|stream|image), CreateEnvironment(path|
      stream|image) for PNG/JPEG/JPEG XL/OpenEXR, RadianceHdrDecoder.
      DecodeFile, ToImage, SaveAsPng, SaveAs, and the C# ModelLoadOptions'
      validation mode, parallel decoding and ModelTextureCache. Use: hand
      the crate BYTES, fetched with the generated <name>_bytes call or
      packed with include_bytes! - a glTF file and the files it names
      (GltfDocument::parse, GltfModelSet: LOADING glTF MODELS; one set
      shares textures as ModelTextureCache does; GltfLoadOptions holds the
      sRGB, tangent and texture-size choices), model data (ctx.load_models),
      a PNG or JPEG file (create_texture_from_png / _jpeg / _image),
      RGBA8 pixels decoded elsewhere (create_texture_from_rgba; premultiply
      there if wanted), RGBA8 frames (create_animated_texture), a FloatImage
      or Radiance .hdr bytes (create_environment / _from_bytes).
    * WAITING FOR THE GPU: Poll, WaitIdle, WaitForSubmission, the
      synchronous Read<T> / ReadPixels / ReadRgba / ReadRegion / ReadDepth /
      Map / ReadMilliseconds / TryReadMilliseconds, and Submit's
      SubmissionIndex for waiting. Use: the async forms (.await) and
      when_submitted_work_done().await.
    * EXCEPTIONS FROM WEBGPU AT THE CALL: C# throws WebGPU's validation
      message from each factory and from Submit / Finish. Use: take_errors()
      every frame, or validate(..) / build_checked() / finish_checked() /
      try_create_shader(..) .await where the verdict is needed at once.
    * EVENTS: UncapturedError and DeviceLost events, ClipFinished. Use:
      take_errors(), is_lost() / lost_reason(), finished_clips().
    * NATIVE FEATURES the browser does not have: SPIR-V, GLSL, passthrough
      and trusted shaders (WGSL only); immediates (WithImmediates,
      WithImmediateSize, SetImmediates, CreatePipelineLayout's immediate
      size: use a uniform buffer); ClearTexture (clear with a render pass);
      timestamps between commands or inside a pass (WriteTimestamp,
      GpuTimer.WriteStart / WriteEnd: use pass timestamps, GpuTimer.pass);
      pipeline-statistics queries; MultiDrawIndirectCount and
      MultiDrawIndexedIndirectCount (use multi_draw_indirect with a count
      known on the CPU, or one indirect draw whose instance count a compute
      pass writes); TransitionTexture; RequiredNativeFeatures,
      ExperimentalNativeFeatures, Backend / GraphicsBackend, AdapterSelector,
      EnumerateAdapters, InstanceFlags (a browser hands out one adapter per
      request: use power_preference and optional_features).
    * OCCLUSION QUERIES (the wgpu version, not the browser): refused with a
      message, see QUERIES above. Use: none with the wgpu version this
      package carries; a wgpu upgrade that passes the occlusion query set
      lifts the refusal.
    * NATIVE INTEROP AND DIAGNOSTICS: TryGetVulkanInfo / TryGetD3D12Info /
      MetalDevice, Adopt*Device, CreateTextureFrom{VulkanImage,
      D3D12Resource,MetalTexture}, IsExternal, GraphicsDiagnostics (engine
      version, log callback), GraphicsBuildInfo, GenerateReport,
      Start/StopGraphicsDebuggerCapture, DestroyDevice. Use: the browser's
      developer tools; device(), adapter() and adapter_info() for the rest.
    * SHARING CONTRACTS OF CodeBrix.WebGpu.Native (IWebGpuDeviceSource,
      IWebGpuTextureTarget, the Skia targets). Use: one GpuContext shared by
      every library on the page (wrap_gpu_context), and the canvas host's
      LAYERS to put Graphics and other renderers in one picture.
    * C#-ONLY SHAPES with a Rust equivalent: Dispose / IsDisposed /
      LiveResourceCount / GetLiveResources (Drop; the crate keeps no
      registry), GpuCommandBuffer (GraphicsFrame::finish returns wgpu's
      CommandBuffer; ctx.submit takes them), DebugGroupScope (push / pop),
      the Wrap*Pass and WrapRenderBundleEncoder calls (record with wgpu's own
      pass through handle() or encoder()), VertexLayout.FromStruct
      reflection (the VertexType trait), GraphicsCapabilities and
      GraphicsAdapterInfo (features(), limits(), adapter_features(),
      adapter_limits(), adapter_has_feature(), adapter_info(): wgpu's own
      types), GpuBuffer.GetMappedRange / MappedMode / MappedOffset (read_mapped,
      write_mapped, or handle().get_mapped_range for wgpu's view).
    * TEXT: the graphics crate draws none. Use: an Animation layer over the
      graphics (the Animation package draws text in fonts given as bytes).


THE codebrix-webgpu-animation CRATE
===================================

WHAT IT IS
  Real-time 2D animation in the browser with the names and concepts of the C# library
  CodeBrix.Imaging.Drawing.NoSkia: a drawing session of calibrated units, layers of shapes, text in
  fonts the application supplies as bytes, gradient fills and fades, the ANIMATION MODEL (keyframe
  tracks bound to element, layer and camera properties, transforms, groups, time windows, animated
  shapes, the camera with parallax, layout, combined bindings), PATHS (draw-on by trims, motion,
  rolling and trains along a path, morphs, combining and queries), TEXT BLOCKS (reveals, staggers,
  captions, text along a path), pictures and sprites (from decoded pixels, PNG or JPEG files), masks
  and effects, PIXEL PLANES (CPU calculation with NoSkia's values) with their whole-frame operations,
  transitions, noise and particles also drawn ON THE GPU, ANIMATED BRUSH STROKES and the painterly
  playback (ownership maps, colour pickup, hard-edged masks, keyframe players), formations, layer
  visibility, safe-area guides, SVG DOCUMENTS (parts animated under program control, and the
  document's own SMIL and CSS animations), easing curves, colour ramps, timelines, a playback
  controller and the debug overlay - drawn on the GPU by the crate's own renderer as a LAYER of
  the codebrix-webgpu canvas host. It draws over whatever the layers below drew
  (a 3D scene of the Graphics package, for example) and never clears the canvas.
  It also plays LOTTIE ANIMATIONS (Lottie JSON read through velato, played at the session's time as
  an element of a session; see LOTTIE ANIMATIONS).
  Beyond NoSkia (no C# equivalent), each a section below: RENDER-TIME LOOKS (a session's blend mode
  and opacity, its paint - as authored, a line drawing or flat colours -, effects on or off) and
  WIPES between any two renders; FIXED-WIDTH STROKES and SKETCHED LINES; PAINTERLY STROKE SETS
  (tens of thousands of brushstrokes in one GPU draw: brush tips, anchors, boil and flow; planned
  by direction fields, stroke layouts and painting timelines; colour rules and perceptual mixing;
  colours picked live from a picture, another session or an application's texture; a lit surface
  finish; checkpoints of a still painting; stroke-level transitions); and WATER (a ripple field as
  a function of time, refraction, reflection, the caustics noise kind).

  Register the package in the build program (AnimationCrates.Register() - it registers the WebGpu
  package's crates first) and name the crate in the frontend's Cargo.toml (beside codebrix-webgpu):
      codebrix-webgpu-animation = "..."
  The crate re-exports kurbo (curves, shapes, transforms) and peniko (brushes, gradients, images,
  blend modes, fill rules) as it was built with them: use codebrix_webgpu_animation::kurbo and
  ::peniko rather than naming those crates in the frontend's own Cargo.toml.

THE RULE: NOTHING DRAWN READS A CLOCK
  Time comes from the frame: the host's ONE clock (CanvasClock) hands every layer the same
  frame.info.time and frame.info.delta, pauses, seeks and carries named points in time. Set a
  session's time from it, seek a PlaybackController to it (or advance it by the delta); pausing the
  host's clock stops everything. Everything the crate draws is a function of the time and the values
  passed in (seeds included), so a seek shows exactly what playing to that time shows. The ONE clock
  reading in the crate is a MEASUREMENT: the CPU milliseconds of a frame in RenderStats (below),
  which nothing drawn ever reads.

PUTTING IT ON A CANVAS
      use std::cell::RefCell;
      use std::rc::Rc;
      use codebrix_webgpu::{CanvasLayers, RendererHandle, WebGpuCanvas};
      use codebrix_webgpu_animation::rendering::AnimationLayer;
      use codebrix_webgpu_animation::shapes::{CircleShape, ShapeOptions};
      use codebrix_webgpu_animation::{Color, DrawingSession, DrawingSessionOptions};

      let mut session = DrawingSession::new(DrawingSessionOptions { calibration_size: (1280, 720), layer_opacity: 255, ..Default::default() });
      let dots = session.add_layer("Dots", Color::from_rgb(255, 176, 72));
      dots.add_shape(CircleShape::new(640.0, 360.0, 40.0, ShapeOptions { is_filled: true, ..Default::default() })?);
      let session = Rc::new(RefCell::new(session));
      let layer = AnimationLayer::new({
          let session = session.clone();
          move |frame, scene| {
              let mut s = session.borrow_mut();
              s.set_time(frame.info.time);                       // the frame's time, never a clock
              s.render(scene, frame.info.width, frame.info.height);
          }
      });
      let layers = CanvasLayers::new(vec![/* a 3D layer, */ RendererHandle::new(layer)]);
      // rsx! { WebGpuCanvas { context, layers, clock, ... } }

  AnimationLayer::new takes a closure that records each frame's drawing into a VectorScene (emptied
  for it); AnimationLayer::for_session draws a shared session as it is; AnimationLayer::to_picture
  draws into a picture other layers read instead of onto the canvas (PICTURE TARGETS below).
  AnimationLayer::control().stats() gives the last frame's RenderStats (no C# equivalent): the
  renderer's counters (draws, culled, cache hits and misses, offscreen and folded groups, draw
  calls, passes, unsupported, effects and effect passes), what was uploaded this frame
  (image_uploads / image_upload_bytes for whole pictures, color_table_uploads /
  color_table_upload_bytes for ownership colour tables sent alone), the stroke sets (stroke_draws,
  strokes, stroke_uploads / stroke_upload_bytes, tip_uploads / tip_upload_bytes), their checkpoints
  (checkpoint_hits, checkpoint_bakes, checkpoint_misses, strokes_restored, checkpoints,
  checkpoint_bytes), the water (ripples_evaluated, ripples_skipped), and the CPU MILLISECONDS of the
  frame: record_ms (the layer's closure recording the scene - the application's own code, a
  session's render call for example), build_ms (the renderer turning the scene into geometry,
  paints and operations), encode_ms (encoding the GPU work) and cpu_ms() (their sum). The timings
  are a measurement only: the GPU's own time and the other layers of the canvas are not in them;
  browsers coarsen performance.now() (to 0.1 ms or more unless the page is cross-origin isolated),
  so read an average over many frames, not one frame; where performance is missing they read 0.
  Before the layer is prepared every counter is 0 (record_ms is still measured).

C# TO RUST, SIDE BY SIDE (the porting rule: C# type names kept; members snake_case; the same argument
order; C# default arguments are option structs with Default; where C# throws, Rust returns Result or
Option and the error says what to do; Drop for Dispose)
  C# (CodeBrix.Imaging.Drawing)                       Rust (codebrix_webgpu_animation)
  new DrawingSession(new DrawingSessionOptions {      DrawingSession::new(DrawingSessionOptions {
      CalibrationSize = new Size(1280, 720),              calibration_size: (1280, 720),
      LayerOpacity = 255,                                 layer_opacity: 255,
      BackgroundFillColor = c })                          background_fill_color: c, ..Default::default() })
  session.AddLayer("Dots", color)                     session.add_layer("Dots", color)  (-> &mut DrawingLayer)
  session.GetLayer("Dots")                            session.get_layer("Dots") / get_layer_mut("Dots")
  layer.AddShape(new RectangleShape(x, y, w, h,       layer.add_shape(RectangleShape::new(x, y, w, h,
      1, color, isFilled: true, cornerRadius: 6))         RectangleShapeOptions { stroke_thickness: 1.0, color: Some(color),
                                                            is_filled: true, corner_radius: 6.0 })?)
  new RectangleShape(x, y, w, h, fill, 0)             RectangleShape::with_fill(x, y, w, h, fill.into(), 0.0)?
  new CircleShape(cx, cy, r, 1, c, isFilled: true)    CircleShape::new(cx, cy, r, ShapeOptions { stroke_thickness: 1.0, color: Some(c), is_filled: true })?
  new EllipseShape(...), new LineShape(...),          EllipseShape::new(...), LineShape::new(..., LineShapeOptions {..}),
  new PolylineShape(points, 2, color)                 PolylineShape::new(&points, PolylineShapeOptions { stroke_thickness: 2.0, color: Some(color), ..Default::default() })?
  new PathShape(geometry, style, stroke, fill)        PathShape::with_style(geometry, style, Some(stroke), Some(fill), None)
  PathGeometry.Parse("M 0 0 L ..."),                  PathGeometry::parse("M 0 0 L ...")?, PathGeometryBuilder::new()
      new PathGeometryBuilder().MoveTo(..)...              .move_to(..).line_to(..).close().build()
  new PathStrokeStyle(4, cap, join, 4, dashes)        PathStrokeStyle::new(4.0, PathStrokeStyleOptions { cap, join, dash_pattern: Some(vec![..]), ..Default::default() })?
  (no C# equivalent: a fixed-width stroke)            PathStrokeStyleOptions { width_mode: StrokeWidthMode::Fixed, ..Default::default() }
  session.DrawCircle(cx, cy, r, ...)                  session.draw_circle(cx, cy, r, Some(thickness), Some(color), filled)?  (-> ElementId)
  session.DrawArrow(x1, y1, x2, y2, ...)              session.draw_arrow(x1, y1, x2, y2, Some(thickness), Some(color), head_length)?
  session.Time = t; session.Opacity = 0.5             session.set_time(t); session.set_opacity(0.5)
  session.Render(canvas, info)                        session.render(&mut scene, width, height)  (inside an AnimationLayer;
                                                          WITHOUT the background by default - render_with_background draws it)
  CanvasCalibration.GetDrawingRect(view, calib)       CanvasCalibration::get_drawing_rect(w, h, calib)
  Easing.CubicOut(t), Easing.Evaluate(kind, t)        Easing::cubic_out(t), Easing::evaluate(kind, t)
  Easing.GetFunction(kind)                            Easing::get_function(kind)  (-> fn(f64) -> f64)
  Easing.Steps(t, 4, StepPosition.Start)              Easing::steps(t, 4, StepPosition::Start)  (-> Option<f64>)
  Easing.BackOut(t, overshoot)                        Easing::back_out_with(t, overshoot)
  CubicBezierEasing.Ease.Evaluate(t)                  CubicBezierEasing::EASE.evaluate(t)
  new CubicBezierEasing(0.2, 0, 0, 1)                 const SNAP: CubicBezierEasing = CubicBezierEasing::new(0.2, 0.0, 0.0, 1.0).unwrap();
                                                          (new is a const fn: a curve outside its rule - x1 and x2 in 0..1,
                                                          every value finite - is then a BUILD error)
  new ColorRamp(c1, c2, c3); ramp.At(t)               ColorRamp::from_colors(&[c1, c2, c3])?; ramp.at(t)
  new ColorStop(0.5f, c)                              ColorStop::new(0.5, c)  (-> Option)
  new LinearGradientFill(start, end, stops)           LinearGradientFill::new(start, end, &stops, GradientSpread::Pad)?
  new RadialGradientFill(center, r, stops, spread)    RadialGradientFill::new(center, r, &stops, spread)?
  new AnimationTimeline(8, 30)                        AnimationTimeline::new(8.0, 30.0)?
  timeline.AddCue("move", 1, 2, EasingKind.QuadIn)    timeline.add_cue("move", 1.0, 2.0, EasingKind::QuadIn)?
  timeline.Progress("move", t)                        timeline.progress("move", t)  (-> Option<f64>)
  (no C# equivalent)                                  timeline.add_marker_at_point("title", &frame.points, "title in")?
  new PlaybackController(timeline)                    PlaybackController::new(timeline)
  playback.Advance(elapsed)                           playback.advance(frame.info.delta)   (or playback.seek(frame.info.time))
  Color.FromRgb(r, g, b) / Color.FromRgba(...)        Color::from_rgb(r, g, b) / Color::from_rgba(...)
  ImageSource.FromEncoded(pngBytes)                   ImageSource::from_png(&png_bytes)?   (PNG and JPEG only; no other format)
  ImageSource.FromEncoded(jpegBytes)                  ImageSource::from_jpeg(&jpeg_bytes)?   (the EXIF orientation is not applied)
  (backend: decode, then send the pixels)             backend PicturePixelsWriter.Write(rgba, w, h) -> frontend
                                                          ImageSource::from_picture_pixels(&bytes)?
  ImageSource.FromPlaneImage(image)                   ImageSource::from_plane_image(&image)?
  DrawingMask.FromPlane(plane, dest, invert)          DrawingMask::from_plane(&plane, dest, invert)?
  new DrawingPlane(w, h) / DrawingPlaneImage(w, h)    DrawingPlane::new(w, h)? / DrawingPlaneImage::new(w, h)?   (CPU, one thread)
  image.GaussianBlur / CreateBloom / ApplyVignette /  the same names on the CPU (exact, slow for whole frames); ON THE GPU over a
      ApplyToneCurve / ApplyGamma on a frame              layer: layer.set_frame_operations(Some(DrawingFrameOperations::NONE
                                                          .with_bloom(..)?.with_vignette(..)?.with_tone_curve(k, gain)))
  t.Apply(fromFrame, toFrame, dest, fraction)         CPU: t.apply_bytes(..)? / t.apply(..)?; ON THE GPU between two layers:
                                                          session.set_layer_transition("A", "B", t, fraction)?
  DrawingNoise.Fill(plane, kind, seed, f, ...)        CPU, exact: DrawingNoise::fill(&mut plane, ..)?; DRAWN on the GPU (same
                                                          parameters, not the same pattern): NoiseShape::new(x, y, w, h, kind,
                                                          seed, f, NoiseShapeOptions { fill, stops })?
  session.DrawParticles(system, t, softness)          session.draw_particles(&system, t, Some(softness))?  (one instanced draw)
  new BrushStrokeShape(cx, cy, l, w, angle, bend,     BrushStrokeShape::new(cx, cy, l, w, angle, BrushStrokeShapeOptions {
      profile, color)                                     bend, profile, color })?   (+ with_progress(p): painting it on)
  map.ApplyColors(image, colors) every frame          OwnershipShape::new(&map, &colors, dest, OwnershipShapeOptions { .. })?
                                                          (the colouring and seams on the GPU)
  (no C# equivalent: new colours, same map)           base.with_colors(&colors)?   (only the colour table is uploaded)

  Colours are straight-alpha sRGB bytes everywhere at the API (as in C#); the renderer works linear and
  premultiplied inside, as the canvas host requires. Gradient fills interpolate in straight sRGB, like
  NoSkia (so a LinearGradientFill made from a ColorRamp's stops matches ColorRamp::at), unless a
  perceptual space is chosen (COLOUR: RULES, PERCEPTUAL MIXING AND COLOUR AS A FUNCTION OF TIME).

  NEW FIELDS COME WITHOUT A MAJOR VERSION. Public structs gain fields as the crate grows, and its
  major version does not change for it. Write every struct literal of an options type with
  `..Default::default()` (`SessionRenderOptions { include_background: true, ..Default::default() }`)
  and such code keeps compiling. Types that have gained fields this way: SessionRenderOptions (blend,
  opacity, paint, effects, authored_layers, refraction - and it is no longer Copy: clone it to use it
  twice), PathStrokeStyleOptions (width_mode), the SVG scene's SvgStroke (non_scaling) and SvgPathNode
  (non_scaling_stroke), and RenderStats (its timings and its upload, stroke, checkpoint and ripple
  counters). Enums gain variants the same way: DrawingNoiseKind has a third kind, Caustics, so an
  exhaustive `match` on it needs a third arm.

TEXT: FONTS FROM BYTES, NEVER FROM THE SYSTEM
  Text is drawn as filled glyph outlines in fonts a FontRegistry holds. A font is registered from the
  BYTES of a TrueType or OpenType file - the crate never reads files, never fetches anything and
  never uses a system font (there is no fallback font either). Where the bytes come from is the
  application's choice:
    - FROM THE BACKEND: keep the font as a file or an embedded resource of the backend, serve it from
      a [WireFile] action, and fetch it with the generated `<name>_bytes` call (the platform's client
      does the talking; the GraphicsDemo sample does it this way):
          let bytes = generated::api::fonts::roboto_regular_bytes().await.map_err(|e| e.message())?;
          fonts.register_font(bytes, None)?;
    - PACKED INTO THE FRONTEND: `include_bytes!("../assets/MyFont.ttf").to_vec()`, or a Dioxus asset
      the frontend reads itself:
          fonts.register_font(include_bytes!("../assets/MyFont.ttf").to_vec(), Some("Body"))?;
  A FontRegistry is a SHARED HANDLE (clone() is the same registry, as a C# reference is): give one
  to every session through DrawingSessionOptions::fonts, register when the bytes arrive, and
  everything that draws text afterwards finds the font. Until a font is registered, making text
  fails with C#'s message ("No font is registered: ..."); the DebugOverlay draws nothing instead.
  Shaping is harfrust's (kerning, ligatures, contextual forms, marks); outlines are unhinted and read
  once per glyph; the renderer keeps each glyph's flattened geometry and draws it with a transform,
  so a line of text costs no flattening from frame to frame. TEXT ADDS about 300 KB (gzip) to the
  application's .wasm: the shaping tables for every script come with harfrust.

  Text positions (ShapedCluster::text_index, MissingCharacter::text_index) are UTF-8 BYTE offsets into
  the Rust string; C# counts UTF-16 units. This is the one deliberate difference in the text API.

  VARIABLE FONTS (no C# equivalent): the axis values are chosen WHEN THE FONT IS REGISTERED -
  fonts.register_font_with_variations(bytes, family_name_override, &[FontVariation::weight(700.0),
  FontVariation::width(87.5), FontVariation::new(*b"opsz", 14.0)])? (types in the text module; the
  values in each axis's own units, as CSS font-variation-settings; axes not named keep their
  defaults; an empty list is exactly register_font). ONE INSTANCE PER REGISTRATION: to use several
  weights of one variable font, register its bytes once per weight, each with its own family name
  override ("Body", "Body Bold") - without an override every instance answers to the font's own
  family name and the first one registered wins. Everything drawn from a registration uses its one
  instance: shaping, advances, metrics and outlines, so measure_text, text shapes, text blocks,
  captions, text along a path and the debug overlay agree. fonts.get_variation_axes(Some("Body"))?
  lists a font's axes (FontVariationAxis: tag, min_value, default_value, max_value, is_hidden;
  printed as "wght 100..900 (default 400)"); the list is empty for a font that is not variable.
  REFUSED, with a message that names the font's real axes and ranges, and nothing registered: an
  axis the font does not have, a value outside its range or not finite (refused, not clamped as CSS
  would), the same axis twice, and everything register_font refuses. SVG documents are NOT
  affected: their text uses the font bytes in DrawingSvgOptions, not the registry.

  DIRECTIONS: a line is shaped as ONE run, in the direction of its first strong character. Left-to-
  right text works as in NoSkia, and a line written entirely right to left (Arabic, Hebrew) comes out
  in the right order too. A line that MIXES directions (an Arabic sentence with a Latin word or a
  number inside) is NOT reordered: there is no Unicode bidirectional algorithm.

  CHARACTERS THE FONT LACKS - TextShapeOptions::missing_characters, a MissingCharacterMode:
    ShowTofu (the default)  a visible box, always: the font's .notdef glyph when it has ink (Roboto's
                            is a box), otherwise a hollow box the crate draws in that glyph's advance
                            (Open Sans's .notdef has no ink) - you see that a character is missing
    FontDefault             whatever the font's own .notdef glyph is, ink or not (what NoSkia draws)
    Blank                   nothing visible: a blank advance, like a space
  The advance is always sensible (the .notdef glyph's own, else the space's, else half an em), and
  the text REPORTS what was missing whatever the mode: TextShape::missing_characters() (character and
  byte offset, in text order), has_missing_characters(), each ShapedCluster's is_missing, and, with no
  shape made, FontRegistry::get_missing_characters(text, family) - to check a string before showing
  it. (NoSkia has no such option or report: it always draws .notdef.)

  C# (CodeBrix.Imaging.Drawing)                       Rust (codebrix_webgpu_animation)
  var fonts = new FontRegistry();                     let fonts = FontRegistry::new();
  fonts.RegisterFont(path) / (bytes) / (stream)       fonts.register_font(bytes, None)?   (bytes only)
  fonts.RegisterFont(bytes, "Body")                   fonts.register_font(bytes, Some("Body"))?
  (no C# equivalent: a variable font's instance)      fonts.register_font_with_variations(bytes, Some("Body Bold"), &[FontVariation::weight(700.0)])?
  fonts.Count, GetRegisteredFamilyNames()             fonts.count(), fonts.get_registered_family_names()
  fonts.Contains("Roboto")                            fonts.contains(Some("Roboto"))   (None: any font)
  fonts.MeasureText("Title", 48)                      fonts.measure_text("Title", 48.0, &MeasureTextOptions::default())?
  fonts.MeasureText(text, 48, 0.1f, "Body", false)    fonts.measure_text(text, 48.0, &MeasureTextOptions { tracking: 0.1,
                                                          family_name: Some("Body".into()), kerning: false })?
  (no C# equivalent)                                  fonts.get_missing_characters(text, None)?
  new DrawingSessionOptions { Fonts = fonts }         DrawingSessionOptions { fonts: Some(fonts.clone()), ..Default::default() }
  session.Fonts                                       session.fonts()
  session.DrawText("Title", 640, 100, 48,             session.draw_text("Title", 640.0, 100.0, 48.0, TextShapeOptions {
      Ink, TextAlign.Center)                              color: Some(ink), align: TextAlign::Center, ..Default::default() })?
  new TextShape(text, x, y, 30, fonts, Ink,           TextShape::new(text, x, y, 30.0, &fonts, TextShapeOptions { color: Some(ink),
      TextAlign.Right, 0.02f, "Body")                     align: TextAlign::Right, tracking: 0.02, family_name: Some("Body".into()),
                                                          ..Default::default() })?
  new TextShape(text, x, y, 30, fonts, Ink,           TextShape::new(text, x, y, 30.0, &fonts, TextShapeOptions { color: Some(ink),
      TextAlign.Left, 0, null, kerning: false)            kerning: false, ..Default::default() })?
  new TextShape("GRADIENT", x, y, 78, fonts,          TextShape::with_fill("GRADIENT", x, y, 78.0, &fonts, fill.into(),
      fill, TextAlign.Center, 0.02f)                      TextShapeOptions { align: TextAlign::Center, tracking: 0.02, ..Default::default() })?
  (no C# equivalent)                                  TextShapeOptions { missing_characters: MissingCharacterMode::Blank, .. }
  text.Width, text.Left, text.FamilyName              text.width(), text.left(), text.family_name()
  text.GetBounds().Value, text.GetOutline()           text.get_bounds(), text.get_outline()
  layer.AddShape(text)                                layer.add_shape(text)
  (x, y) is the anchor ON THE BASELINE and the size is the em size, both in calibrated units, as in
  C#. TextAlign::Justify is refused for a single line (as C# does). Blocks of text - wrapped,
  aligned, trimmed, revealed, split into pieces - are TEXT BLOCKS, REVEALS, STAGGERS, CAPTIONS, TEXT
  ALONG A PATH below.

THE DEBUG OVERLAY (C# DebugOverlay)
  A preview aid: the timecode, "frame N / COUNT" and, if asked, the active cues on a translucent plate
  in a corner, in output pixels, drawn LAST over the session; off until set. It fails soft (draws
  nothing) with no usable font.
  C#                                                  Rust
  var overlay = new DebugOverlay(fonts, timeline)     let mut overlay = DebugOverlay::new(Some(fonts.clone()), Some(timeline.clone()));
      { ShowCues = true,                              overlay.show_cues = true;
        Corner = DebugOverlayCorner.BottomRight };    overlay.corner = DebugOverlayCorner::BottomRight;
  overlay.TextSize = 14                               overlay.set_text_size(14.0)?;
  session.DebugOverlay = overlay; ... = null          session.set_debug_overlay(Some(overlay)); ... set_debug_overlay(None)
  overlay.GetLines(t), overlay.CanDrawText            overlay.get_lines(t), overlay.can_draw_text()
  To show it over a whole canvas of layers, keep an empty session that holds only the overlay and
  render it last in the frame (as the C# demo's live window and the GraphicsDemo sample do).

GRADIENTS AND FADES
  Gradient fills paint any filled shape - rectangles, circles, ellipses, polygons, paths and text:
  LinearGradientFill / RadialGradientFill with ColorStops and a GradientSpread (Pad, Repeat, Reflect),
  in calibrated units like the shapes. They interpolate straight sRGB colour AND straight alpha
  between the stops, as NoSkia does (by default: with_interpolation opts into a perceptual space,
  premultiplied - see COLOUR: RULES, PERCEPTUAL MIXING AND COLOUR AS A FUNCTION OF TIME), and a first
  stop past 0 (or a last before 1) holds its colour out to the end.
  C#                                                  Rust
  new LinearGradientFill(a, b, stops, spread)         LinearGradientFill::new(a, b, &stops, spread)?
  new RadialGradientFill(c, r, centre, edge)          RadialGradientFill::from_colors(c, r, centre, edge)?
  new CircleShape(cx, cy, r, fill)                    CircleShape::with_fill(cx, cy, r, fill.into())?
  new PolylineShape(points, fill)                     PolylineShape::with_fill(&points, fill.into())?
  FADES:
  session.Opacity = 0.4f                              session.set_opacity(0.4)
  layer.Opacity = 0.25f                               layer.set_opacity(0.25)
  shape.Opacity (DrawingElement.Opacity)              DrawingShape::with_opacity(0.5) / set_opacity(0.5)
  session.ExportPixels(.., includeBackground: false)  session.render_with(&mut scene, w, h, &SessionRenderOptions {
                                                          include_background: false, ..Default::default() })
  (draw a session into part of a frame)               SessionRenderOptions { target: Some(RectangleF::new(x, y, w, h)), .. }
  The session's opacity fades EVERY LAYER AS ONE GROUP over the background fill, which is not faded:
  over an opaque background that is a fade from and to that colour; without the background
  (include_background: false) what is below shows through - the fade is transparency. A layer's
  opacity fades that layer alone (as its own group); an element's opacity multiplies its paint.
  Where the renderer differs from NoSkia: groups composite in linear light (the canvas host's
  convention), so a half-faded colour over another looks a little lighter than NoSkia's sRGB
  arithmetic; the gradient's own interpolation is NoSkia's. A fade of the WHOLE session, background
  included, and a blend mode for it are render options (RENDER-TIME LOOKS below).

DRAWING ON THE PAGE (an animation that floats on the web page)
  A session drawn as a LAYER leaves its BACKGROUND OUT BY DEFAULT: DrawingSession::render and
  AnimationLayer::for_session draw the layers alone, so what is below shows - lower canvas layers,
  or the page itself behind a transparent canvas. THIS IS A DELIBERATE DIFFERENCE FROM C#, where
  Render and the exports draw the background unless told not to (includeBackground defaults to
  true). Ask for the background when a scene wants it:
      session.render_with_background(&mut scene, w, h);
      session.render_with(&mut scene, w, h, &SessionRenderOptions { include_background: true, ..Default::default() });
  The recipe: WebGpuCanvas { transparent: true, .. }; no frame (border, ring, rounded corners,
  background) on the canvas or its container in the page's styling; the session drawn with
  render(...) (no background); translucent shapes and text then blend with the page exactly (the
  canvas host's last pass makes the page see premultiplied sRGB, so there are no fringes). The
  sample's /animation/?scene=on-the-page does this.

RENDER-TIME LOOKS: BLEND, OPACITY, PAINT, EFFECTS, AND WIPES BETWEEN RENDERS (no C# equivalent)
  A look is chosen WHEN A SESSION IS RENDERED, on the SessionRenderOptions given to render_with: the
  scene is never changed, so the same session can be drawn with another look the next frame, or
  twice in one frame (two render_with calls, with different targets or looks). One look per render;
  a WIPE (below) joins two.
  - BLEND AND OPACITY: blend: SessionBlendMode (Normal - the default -, Multiply, Screen, Overlay,
    Darken, Lighten, Add) and opacity (1 by default; clamped to 0..1, NaN counts as 0). The
    session's WHOLE picture - its background when include_background draws it, and its layers - is
    combined as ONE group with what was drawn before it into the SAME scene (the same
    AnimationLayer: an earlier session, a picture, another render of the same session). Other
    canvas layers are not the backdrop: the animation layer is composited normally over them, so a
    3D scene on a lower canvas layer does not take part in a Multiply. Mix modes blend sRGB-encoded
    values (the W3C formulas, as every group of the crate); Add adds linear premultiplied values. The
    render opacity fades the background too, where set_opacity (C#'s rule) fades the layers over an
    unfaded background; the two multiply. Opacity 0 draws nothing of the session. The safe-area
    overlay and the debug overlay are drawn over the picture, never blended or faded. A blended or
    faded session is one offscreen group (one level of the nesting allowance, THE COST OF EFFECTS
    AND MASKS below); default options draw exactly what they drew before.
  - PAINT: paint: SessionPaint - AsAuthored (the default), Line(LinePaint), or Flat. LinePaint::new(
    stroke_style, color) is the line drawing's stroke and colour; .with_fill(paper) fills every shape
    with one colour UNDER its outline, so nearer shapes hide the lines behind them.
      LINE   every fill becomes its outline in the line style and colour (the fill left out, or the
             LinePaint's fill); every stroke is redrawn along its own path in the line style; text
             becomes outlined letters; pictures, sprites, noise, ownership maps, particles, stroke
             sets and pictures of a picture target are left out.
      FLAT   every fill becomes one solid colour from its own fill: a gradient its average along its
             length, a picture or sprite the average of its pixels (times its tint), noise its ramp's
             average, an ownership map its owners' colours weighted by the pixels each owns;
             outlines and particles stay as authored; a stroke set is drawn with every stroke in its
             own (or picked) colour WITHOUT its tip - exactly as with no tips; a picture of a picture
             target is drawn transparent (its pixels are on the GPU only, nothing to average).
    Shapes, text, SVG documents and Lottie animations all take the look. A shape's opacity and its
    fill's alpha are kept (a faded shape gives a faded line), and the line colour's alpha multiplies
    it. MASKS ARE NEVER REPAINTED: mattes, "outside a path" masks, plane masks, SVG masks and clips,
    a text block's outside-outline cut-out are coverage, so a masked element keeps its shape in every
    look. The background fill and the two overlays are never repainted. A line style with
    StrokeWidthMode::Fixed gives ONE line weight everywhere in drawing units, whatever the elements'
    scaling (FIXED-WIDTH STROKES below); with the default it scales with each element. A picture's
    average is worked out once per picture (one pass over its pixels, the first frame it is drawn
    flat) and kept.
  - EFFECTS: effects: false draws every element, group and layer WITHOUT its blur, glow and drop
    shadow (and an SVG document's blur), as an ordinary group at its opacity. KEPT: a layer's frame
    operations (tone curve, gamma, vignette, bloom), its layer transition and its water (refraction,
    reflection). DROPPED with the effects: a layer's surface finish.
  - EXCEPTIONS: authored_layers: a layer whose name is listed is drawn exactly as authored - paint,
    effects and finish - whatever paint and effects say.
  - refraction: the whole session seen through water (WATER below).
      let ink = LinePaint::new(PathStrokeStyle::new(2.0, PathStrokeStyleOptions { width_mode: StrokeWidthMode::Fixed,
          ..Default::default() })?, Color::from_rgb(30, 30, 40)).with_fill(Color::from_rgb(250, 248, 240));
      session.render_with(scene, w, h, &SessionRenderOptions { paint: SessionPaint::Line(ink.clone()), effects: false,
          authored_layers: vec!["Logo".into()], ..Default::default() });
      background.render_with_background(scene, w, h);
      ink_drawing.render_with(scene, w, h, &SessionRenderOptions { blend: SessionBlendMode::Multiply, opacity: 0.8,
          ..Default::default() });

  WIPES: SessionWipe { progress, edge, softness } (SessionWipe::new(progress, edge, softness);
  Default: left to right, progress 0, softness 48) joins ANY TWO RENDERS - two sessions, or one
  session rendered twice with different looks. At progress 0 only the first is drawn, at 1 only the
  second; in between the edge has travelled that fraction of the way across the area, with a soft
  band `softness` wide where the two blend (a smoothstep ramp, so the band shows no edge of its own).
    WipeEdge::Line { angle_degrees }  a straight edge travelling in that direction (0 left to right,
                                      90 top to bottom - canvas y points down -, 180 right to left,
                                      270 bottom to top, any angle); the second render shows where
                                      the edge has passed
    WipeEdge::Circle { center }       an iris: a circle growing from the centre (it may lie outside
                                      the area) until it covers the whole area, the second render
                                      inside; for an iris CLOSING, swap the renders and run the
                                      progress from 1 to 0
  The area, the centre and the softness are CANVAS PIXELS (the units of SessionRenderOptions::target):
  scale the softness with the canvas for the same look at every size. progress is clamped to 0..1
  (NaN counts as 0) and is the application's to animate from the frame's time - a function of the
  time like everything else, so pausing freezes the wipe and a seek shows the same frame.
    wipe.render(&mut scene, area, |scene| first, |scene| second)   any two renders, as closures that
        record into the scene (sessions with any options, or VectorScene content of your own)
    wipe.render_sessions(&mut scene, w, h, (&a, &options_a), (&b, &options_b))   two sessions (or one
        twice) over the whole canvas; the SECOND session's blend and opacity are put on the wipe's
        own group
  HOW IT IS DRAWN: the first render under, clipped to where it still shows (the band included); the
  second in ONE isolated group kept destination-in by the band's gradient - over opaque content an
  exact cross-fade. Softness 0 is a hard edge: both renders are only clipped, no group. Both renders
  are clipped to the area.
  WHAT A MULTIPLIED SECOND SESSION BLENDS WITH: with render_sessions, the second session's Multiply
  (or any mix mode) blends with what is UNDER THE WIPE - where the edge has passed the first render
  is cut away, so there it is not the backdrop. To multiply the second session over the FIRST, draw
  the first session once underneath as well, then wipe from it to the multiplied second (the
  GraphicsDemo sample's "wipe" scene does this; progress 0 and 1 then equal the two renders alone).
  Through render, a mix blend set inside the second render blends inside the wipe's isolated group,
  that is against transparency: use render_sessions for a blended second session.
  LIMITS: while 0 < progress < 1 BOTH renders are drawn in full; a soft edge uses ONE level of the
  nesting allowance for the second render (shared with that session's blend in render_sessions;
  the first render's depth is unchanged). The layer transitions of a session
  (set_layer_transition) are unchanged and stay the way to move between two layers of one session.
      let p = Easing::evaluate(EasingKind::CubicInOut, ((frame.info.time % 4.0) / 3.0).min(1.0)) as f32;
      SessionWipe::new(p, WipeEdge::Line { angle_degrees: 30.0 }, 0.06 * w as f32).render_sessions(scene, w, h,
          (&session, &SessionRenderOptions::default()),
          (&session, &SessionRenderOptions { paint: SessionPaint::Line(ink.clone()), ..Default::default() }));
      let iris = SessionWipe { progress: p, edge: WipeEdge::Circle { center: PointF::new(x, y) }, ..SessionWipe::default() };
      iris.render(scene, RectangleF::new(0.0, 0.0, w as f32, h as f32),
          |scene| day.render_with(scene, w, h, &day_options), |scene| night.render_with(scene, w, h, &night_options));

THE ANIMATION MODEL: TRACKS, BINDINGS, TRANSFORMS, GROUPS, TIME WINDOWS, CAMERA, LAYOUT
  A frame is a FUNCTION OF THE TIME: session.set_time(t) (C# session.Time = t) applies every binding
  in the order bound (a later binding of the same property wins), rebuilds every AnimatedShape for
  t and decides which elements are inside their time windows. Seeking back needs nothing more.
  - KeyframeTrack<T> with FloatTrack / PointTrack / ColorTrack / BoolTrack as its kinds: keys in time
    order, each with the EASING OF THE SEGMENT THAT ARRIVES AT IT (a named EasingKind, a custom curve,
    or a hold); loop_mode Hold / Loop / PingPong; evaluate(t) -> Option<T> (None for no keys, where
    C# throws). Colour tracks interpolate each straight sRGB channel and round as C#'s ColorRamp.
  - ELEMENTS HAVE IDS. Where C# code keeps the shape object to bind it, Rust keeps its ElementId:
    session.draw_*(..) and draw_shape(..) return it (C# returns the shape), layer.add_element(..) and
    group.add(..) return it, and session.find_element(id) / find_element_mut(id) reach the element on
    any layer or inside a group. A clone of a shape keeps its id (it is the same element).
  - BINDINGS: session.animations_mut().bind(target, property, track)? - target an ElementId, a
    BindingTarget::layer("Name") or BindingTarget::Camera; the track a FloatTrack, PointTrack,
    ColorTrack, BoolTrack, or BoundTrack::float_fn / point_fn (C#'s Func<double, T> overloads) /
    color_fn (no C# equivalent; it binds wherever a ColorTrack does - a layer's or group's Color,
    Tint, GlowColor, ShadowColor - and is evaluated at the binding's time on every set_time). The
    property and track are checked against the target when the binding is made (C#'s
    ArgumentException is the Err, saying what the target takes). bind_callback(|t| ..) is C#'s
    Bind(Action<double>); remove(&binding), clear(), apply(t) (C# Animations.Apply).
  - DrawingTransform (translation, rotation in degrees clockwise, scale_x / scale_y, anchor): scaled
    and rotated about the anchor, then moved - C#'s matrix. On an element (set_transform /
    with_transform) and on a layer (set_transform). get_bounds() / get_transformed_bounds() are C#'s
    bounds (the outline grown by half the stroke where stroked).
  - DrawingGroup: children in the group's coordinates; one transform, one opacity (below 1 the group
    is drawn whole, then faded - its children never show through each other), a content colour for
    children without their own (bindable as AnimatedProperty::Color).
  - TIME WINDOWS: with_time_window(Some(start), Some(end)) / set_time_window (C# StartTime / EndTime).
    AnimatedShape::new(|t| Some(shape), 0.0): an element made for each moment by a function of the
    time (None draws nothing).
  - DrawingCamera (pan, zoom, roll, pivot): session.set_camera(Some(DrawingCamera::new())); each
    layer follows it by layer.set_parallax(p): 1 with the scene, 0 fixed to the frame (a HUD), in
    between less (pan and roll times p, zoom to the power p). Matrix: C#'s.
  - LAYOUT: DrawingLayout::new(size) / DrawingLayout::from_guide(size, Some(&guide), options)? (SAFE
    AREAS below); place_at(anchor, reference, dx, dy),
    place(&LayoutPosition), place_box((w, h), &LayoutPosition); LayoutAnchor::TOP_LEFT .. BOTTOM_RIGHT;
    LayoutReference::Frame / SafeArea; short_side().
  C# (CodeBrix.Imaging.Drawing)                       Rust (codebrix_webgpu_animation)
  A KEYFRAME TRACK BOUND TO A PROPERTY:
  var ball = session.DrawCircle(0, 0, 11,             let ball = session.draw_circle(0.0, 0.0, 11.0, None,
      filled: true, color: accent);                       Some(accent), true)?;           // -> ElementId
  var lane = new PointTrack {                         let lane = PointTrack::from_keys(&[
      { 0, new PointF(220, 130) },                        (0.0, PointF::new(220.0, 130.0), EasingKind::Linear),
      { 2.5, new PointF(860, 130),                        (2.5, PointF::new(860.0, 130.0), EasingKind::BackOut)])?;
          EasingKind.BackOut } };
  lane.LoopMode = TrackLoopMode.PingPong;             let lane = lane.with_loop_mode(TrackLoopMode::PingPong);
  session.Animations.Bind(ball,                       session.animations_mut().bind(ball,
      AnimatedProperty.Position, lane);                   AnimatedProperty::Position, lane)?;
  session.Animations.Bind(layer,                      session.animations_mut().bind(BindingTarget::layer("Orbit"),
      AnimatedProperty.Rotation, spin);                   AnimatedProperty::Rotation, spin)?;
  session.Time = t;                                   session.set_time(frame.info.time);
  A CAMERA MOVE:
  session.Camera = new DrawingCamera();               session.set_camera(Some(DrawingCamera::new()));
  stars.Parallax = 0.15f; hud.Parallax = 0f;          stars.set_parallax(0.15)?; hud.set_parallax(0.0)?;
  session.Animations.Bind(session.Camera,             session.animations_mut().bind(BindingTarget::Camera,
      AnimatedProperty.Position, panTrack);               AnimatedProperty::Position, pan_track)?;
  session.Animations.Bind(session.Camera,             session.animations_mut().bind(BindingTarget::Camera,
      AnimatedProperty.Scale, zoomTrack);                 AnimatedProperty::Scale, zoom_track)?;   // zoom
  camera.ToView(p, size, parallax)                    camera.to_view(p, (w, h), parallax)
  ELEMENTS, GROUPS, TIME WINDOWS, LAYOUT:
  new DrawingTransform(new PointF(x, y), 45)          DrawingTransform::new(PointF::new(x, y), DrawingTransformOptions {
                                                          rotation: 45.0, ..Default::default() })?
  shape.Transform = t; shape.GetTransformedBounds()   e.set_transform(t); e.get_transformed_bounds()
  var g = new DrawingGroup(color); g.Add(shape);      let mut g = DrawingGroup::new(Some(color)); g.add(shape);
  line.StartTime = 1.5; line.EndTime = 2.7;           DrawingShape::from(line).with_time_window(Some(1.5), Some(2.7))
  new AnimatedShape(t => shape)                       AnimatedShape::new(|t| Some(shape.into()), 0.0)
  layout.Place(LayoutAnchor.Center,                   layout.place_at(LayoutAnchor::CENTER,
      LayoutReference.SafeArea)                           LayoutReference::SafeArea, 0.0, 0.0)
  new LayoutPosition(LayoutAnchor.TopCenter,          LayoutPosition::new(LayoutAnchor::TOP_CENTER, LayoutPositionOptions {
      LayoutReference.SafeArea, fractionY: 0.08f)         reference: LayoutReference::SafeArea, fraction_y: 0.08, ..Default::default() })?
  Differences: ids instead of object references (above). Bindings that COMBINE (C#'s BindingMode
  Add / Multiply) are COMBINED BINDINGS below; DrawingLayout's constructor that takes a
  SafeAreaGuide is DrawingLayout::from_guide (SAFE AREAS below). A property bound on an element that
  does not have it (RevealFraction on a circle, FrameIndex on a picture) is refused with a message.

PATHS: DRAW-ON, MOTION ALONG A PATH, MORPH
  - PathGeometry: parse (SVG path data), the builder, rectangle / rounded_rectangle / ellipse /
    circle / regular_polygon / star built AS C# BUILDS THEM (the same start point and direction, so a
    draw-on starts where NoSkia's does), get_length(), get_point_at(fraction) / get_point_at_distance
    (a PathPoint: position, unit tangent, distance, contour index; angle() in degrees), trim(start,
    end, offset)? (wrapping round with the offset, one piece across the start of a single closed
    contour), dash(pattern, offset)? (C#'s Dash), rounded_polygon(points, radius)? (C#'s
    RoundedPolygon), transformed(matrix).
  - PathShape trims (C# TrimStart / TrimEnd / TrimOffset, 0..1 fractions of the length; the fill is
    trimmed too) and morphs (PathShape::from_morph / with_morph, morph_fraction), all bindable.
  - PathMorph::new(from, to, 0)?: contours paired in order, resampled by length (one sample per two
    units, 16 to 4096), the second turned to run the same way and start at the best-matching sample;
    a missing contour grows from the other's mean point; get_path(fraction). C#'s arithmetic.
  - Motion along a path: session.animations_mut().bind_along_path(id, &path, progress,
    PathTravelOptions { orient_to_path, rotation_offset })? places the element's ANCHOR on the path at
    the progress fraction (draw the element around (0, 0)), turned to face along it when asked.
  C#                                                  Rust
  A DRAW-ON:
  var sig = new PathShape(PathGeometry.Parse(d),      let mut sig = PathShape::with_style(PathGeometry::parse(d)?,
      new PathStrokeStyle(7f), blue) { TrimEnd = 0 };     PathStrokeStyle::new(7.0, Default::default())?, Some(blue), None, None);
                                                      sig.set_trim_end(0.0)?;
  session.DrawShape(sig);                             let sig = session.draw_path(sig);
  session.Animations.Bind(sig,                        session.animations_mut().bind(sig,
      AnimatedProperty.TrimEnd, writing);                 AnimatedProperty::TrimEnd, writing.clone())?;
  dashes.WithDashOffset(-t * 46 % 23)                 dashes.with_dash_offset(((-t * 46.0) % 23.0) as f32)   // marching ants
  MOTION ALONG A PATH:
  session.Animations.BindAlongPath(dart, eight,       session.animations_mut().bind_along_path(dart, &eight,
      progress, orientToPath: true);                      progress, PathTravelOptions { orient_to_path: true, ..Default::default() })?;
  PathPoint p = eight.GetPointAt(0.25f);              let p = eight.get_point_at(0.25);
  p.Position, p.Tangent, p.Angle, p.Distance          p.position, p.tangent, p.angle(), p.distance
  A MORPH:
  var m = new PathMorph(circle, star);                let m = PathMorph::new(circle, star, 0)?;
  var s = new PathShape(m, 0f, style, ink, null, f);  let s = PathShape::with_morph(m, 0.0, style, Some(ink), None, Some(f.into()))?;
  session.Animations.Bind(s,                          session.animations_mut().bind(id,
      AnimatedProperty.MorphFraction, track);             AnimatedProperty::MorphFraction, track)?;
  text.GetOutline()                                   text.get_outline()   // glyph outlines, to morph
  Differences: arc lengths are kurbo's (to 1e-4 units), not C#'s flattening, so lengths and points
  on curves agree with NoSkia to that accuracy, on straight segments exactly. Path COMBINING and
  QUERIES, ROLLING and TRAINS along paths and TEXT ALONG A PATH are sections of their own below.
  DASHING AS GEOMETRY and ROUNDED POLYGONS are NoSkia's:
  geometry.Dash(new[] { 20f, 10f }, 5f)              geometry.dash(&[20.0, 10.0], 5.0)?   // open dashes; a 0 dash is a dot
  PathGeometry.RoundedPolygon(points, 12f)            PathGeometry::rounded_polygon(&points, 12.0)?
  (dashed STROKES need neither: a PathStrokeStyle's dash pattern dashes the stroke as it is drawn,
  and to_stroke_outline cuts the dashes first).

  FIXED-WIDTH STROKES (no C# equivalent). A stroke style's width_mode is
  StrokeWidthMode::ScalesWithElement (the default: C#'s behaviour) or StrokeWidthMode::Fixed. FIXED
  means fixed in the DRAWING'S OWN UNITS (the session's calibrated units), not in screen pixels:
  every scaling applied to the element INSIDE the drawing is ignored - its own transform (an
  animated scale included), the transforms of the groups it is in and its layer's transform - and
  an uneven scale gives an even line. The VIEW of the drawing still scales it, as it scales
  everything: the calibration (the canvas size) and the session's camera zoom. Dashes (pattern and
  offset) are in the same drawing units: they follow the width, not the element.
    let fixed = PathStrokeStyle::new(3.0, PathStrokeStyleOptions { width_mode: StrokeWidthMode::Fixed,
        dash_pattern: Some(vec![12.0, 8.0]), ..Default::default() })?;
    session.draw_shape(DrawingShape::from(PathShape::with_style(geometry, fixed.clone(), Some(ink), None, None))
        .with_transform(t));
    TextBlockShape::with_stroke(layout, x, y, Some(fixed), TextBlockStrokeOptions { stroke_color: Some(ink),
        ..Default::default() })?;   // outlined text, either stroke placement
  style.width_mode() / is_fixed_width() / with_width_mode(mode); with_width and with_dash_offset keep
  the mode. It applies to PathShape strokes and TextBlockShape outlines, and to SVG strokes marked
  vector-effect="non-scaling-stroke" (SVG DOCUMENTS below). NOT to the basic shapes' stroke_thickness
  (rectangle, circle, ellipse, line, arrow, polyline - they have no stroke style: use a PathShape for a
  fixed-width outline) and not to Lottie strokes. The low-level door is
  VectorScene::stroke_fixed_width(style, transform, reference, brush, brush_transform, shape) (reference
  takes the drawing's space to canvas pixels; a zero scale or a singular reference draws nothing).
  get_bounds still grows a shape by half the width in its own units, so under a strong scale the
  bounds of a fixed-width stroke are an approximation. COST: a fixed-width stroke is expanded in the
  drawing's space, so its cached outline belongs to the shape AS TRANSFORMED - a still element
  reuses it every frame, but one whose own transform changes (moving, turning, scaling) is stroked
  again every frame; a scaling stroke keeps reusing its outline while it moves.

  SKETCHED LINES (no C# equivalent). geometry.sketch(&SketchOptions)? takes a path and returns a
  ROUGHENED path - an ordinary PathGeometry, usable with everything (stroke it, fill it, trim it for
  a draw-on, dash it, morph it, mask with it); it keeps the input's fill rule. SketchOptions, lengths
  in the path's own units:
    roughness             how far the line strays, at most (0: in place, resampled and smoothed)
    wobble_length         the distance between wobbles along the path
    passes                the path drawn over itself that many times, each pass roughened
                          differently (each pass its own contours; the first equals the one-pass sketch)
    seed                  the same seed, settings and path give the same line, always
    corner_angle_degrees  a turn sharper than this is a corner and stays a sharp point; gentler turns
                          (curves) are smoothed through
  The defaults: roughness 2, wobble_length 30, passes 2, seed 1, corner angle 30. BOILING (the
  drawn-film look): options.boiled(time, rate) is the same options with the seed moved on `rate` times a second of the time passed in (8 to 12 a second looks like a drawn
  film); pass the FRAME's time, so pausing freezes the line and a seek always shows the same line (a
  rate of 0 or less, or a time that is not finite, leaves the seed alone). The stroke sets' boil uses
  the same timing (strokes::boil_epoch).
    let o = SketchOptions { roughness: 2.5, wobble_length: 24.0, passes: 2, seed: 7, corner_angle_degrees: 30.0 }
        .boiled(frame.info.time, 10.0);
    layer.add_shape(PathShape::with_style(house.sketch(&o)?, PathStrokeStyle::new(2.0, Default::default())?, Some(ink), None, None));
  Errors say what to change (a negative or non-finite roughness, a wobble length that is not
  positive, 0 passes, a negative corner angle); an empty path gives an empty path. LIMIT: it
  roughens the PATH, not the pen - stroked, the line has an even width and one colour (no pressure,
  no texture). COST: a CPU function of the path; while boiling, each new seed is new geometry for the
  renderer's cache, so sketch once per seed step (the result changes only `rate` times a second).

TEXT BLOCKS, REVEALS, STAGGERS, CAPTIONS, TEXT ALONG A PATH
  A block of text is laid out ONCE by TextLayoutEngine (or session.layout_text) into a
  TextLayoutResult - lines, clusters (the characters that shaped into glyphs together), words, boxes -
  shared as an Rc by everything drawn from it. The layout is NoSkia's, on the font and shaping layer
  above:
    - LINE BREAKING is NoSkia's OWN rule set, not the Unicode line-breaking algorithm: the text breaks
      at its line breaks (LF, CR, CR LF, NEL, LS, PS, VT, FF) and, with max_width, wraps after spaces
      (not no-break spaces), after a hyphen inside a word, after a zero width space and before or after
      a CJK ideograph; a word wider than the line breaks between characters. Spaces at a wrapped line's
      end hang past the width.
    - A line's height is line_height when set, else the tallest run's natural height times
      line_spacing; the baseline sits as NoSkia puts it.
    - Alignment Left / Center / Right / Justify, vertical alignment in max_height, max_lines /
      max_height cuts with an optional ellipsis after the last character or the last whole word.
    - Styled runs (TextRunDescriptor): their own family, size, tracking, colour or fill.
  Text positions are UTF-8 BYTE offsets, as everywhere in this crate's text (C# counts UTF-16 units).
  DIRECTIONS: lines are laid out left to right, as NoSkia. A run written right to left (Arabic,
  Hebrew) is shaped right to left and its clusters are placed right to left within each line - they
  stay in text order for counting, revealing and pieces. A line that MIXES directions is not
  reordered (not supported).
  MISSING CHARACTERS: TextLayoutOptions::missing_characters (default ShowTofu); the layout reports them
  (missing_characters(), has_missing_characters(), TextCluster::is_missing()).

  A TextBlockShape places a layout at (x, y) - its top-left - in a colour, a fill, or as an outline
  (a stroke style, with a fill or not; StrokePlacement Center or Outside). Its REVEAL shows the first
  units (characters, words or lines) by a fraction or a count, with an optional soft edge in units;
  bind RevealFraction or RevealCount to a float track and the reveal follows the time. A partly shown
  cluster is drawn with its paint's alpha multiplied (no offscreen layer per glyph). A block splits
  into PIECES (create_pieces, create_piece_group: characters, words, lines), each a block of its own
  over the same layout, to move, stagger or send along a path one by one. merge_overlaps merges each
  glyph's overlapping contours (path combining, below) before it is filled and outlined.
  A filled block draws every glyph from the renderer's geometry cache; an outlined or merged block
  draws its clusters' outlines as paths each frame.

  STAGGERS: session.animations_mut().bind_stagger(&ids, Some(&StaggerOptions {...}))? binds, per
  element, its opacity, position, scale and rotation tracks (C#'s order), each element starting a
  little after the one before in the options' order (StaggerOrder Forward / Reverse / CenterOut);
  the options are checked before anything is bound. StaggerTracks::create / state_at(t) give the same
  values without a session.

  CAPTIONS: CaptionTrack::parse reads SRT or WebVTT TEXT (tags removed, character references decoded;
  WebVTT's NOTE / STYLE / REGION blocks skipped; align:, line:N% and position:N% applied). The crate
  reads no files: fetch the caption text (from the backend through the platform's generated call, as
  the GraphicsDemo sample does, or carry it in the frontend) and parse it. session.draw_captions(track,
  layout, style)? shows the cues active at the session's time - wrapped, on a rounded box, stacked when
  they overlap - as one animated shape on the active layer, and returns its id. A CaptionPresenter
  answers the same for any time (get_placed_captions_at).
  CAPTIONS SIT AGAINST THE FRAME BY DEFAULT - a deliberate difference from NoSkia, where they sit in
  the layout's safe area: the caption area is the frame kept CaptionStyle::margin (in TEXT SIZES; one
  text size by default) from its edges. To place captions inside a safe-area guide instead - a
  preview of a video meant for a platform with unsafe areas - OPT IN: make the presenter with
  .with_placement(CaptionPlacement::from_guide(calibration_size, &guide, options)?) and draw it with
  session.draw_captions_with(presenter). WebVTT's line:N% is measured against the frame, as in C#.

  TEXT ALONG A PATH: layout.get_outline_along_path / get_cluster_outlines_along_path place each
  cluster's centre on the path at start_distance plus its distance from the block's left, turned to
  the path's direction; get_cluster_transforms_along_path gives one transform per cluster.
  ANIMATED: session.animations_mut().bind_text_along_path(&block, unit, "Layer", &path, progress,
  PathTrainOptions)? sends the block's pieces (white-space pieces left out) along the path as a TRAIN
  (ROLLING AND TRAINS below) that settles every piece in place; with roll_on_baseline each rolls on
  its own line's baseline. The pieces are added to the layer named (C#'s train.AddTo(layer)).
  C#                                                  Rust
  A TEXT BLOCK WITH A REVEAL:
  var layout = session.LayoutText(text, 30,           let layout = session.layout_text(text, 30.0, None,
      options: new TextLayoutOptions { MaxWidth =         Some(&TextLayoutOptions { max_width: Some(560.0),
      560, Align = TextAlign.Justify });                  align: TextAlign::Justify, ..Default::default() }))?;
  var typed = session.DrawTextBlock(layout, 40,       let typed = session.draw_text_block(layout.clone(), 40.0,
      150, Ink);                                          150.0, Some(ink))?;     // its ElementId
  session.Animations.Bind(typed,                      session.animations_mut().bind(typed,
      AnimatedProperty.RevealCount, typing);              AnimatedProperty::RevealCount, typing)?;
  var b = new TextBlockShape(layout, x, y, Ink)       let mut b = TextBlockShape::new(layout.clone(), x, y, Some(ink))?;
      { RevealUnit = TextUnit.Word,                   b.set_reveal_unit(TextUnit::Word);
        RevealSoftness = 2f };                        b.set_reveal_softness(2.0)?;
  new TextBlockShape(layout, x, y, stroke, Ink)       let mut o = TextBlockShape::with_stroke(layout, x, y, Some(stroke),
      { StrokePlacement = Outside };                      TextBlockStrokeOptions { stroke_color: Some(ink), ..Default::default() })?;
                                                      o.set_stroke_placement(TextStrokePlacement::Outside);
  var words = block.CreatePieceGroup(TextUnit.Word);  let words = block.create_piece_group(TextUnit::Word);
  session.DrawShape(words);                           let ids: Vec<_> = words.get_children().iter().map(|c| c.id()).collect();
  session.Animations.BindStagger(words.GetChildren(), session.draw_shape(words);
      new StaggerOptions { Interval = 0.18 });        session.animations_mut().bind_stagger(&ids, Some(&StaggerOptions {
                                                          interval: 0.18, ..Default::default() }))?;
  CAPTIONS:
  var track = CaptionTrack.ParseSrt(srt);             let track = CaptionTrack::parse_srt(&srt)?;   // text fetched from the backend
  session.DrawCaptions(track, layout,                 session.draw_captions(track, None, Some(CaptionStyle { size: 32.0,
      new CaptionStyle { Size = 32 });                    ..Default::default() }))?;               // AGAINST THE FRAME (default)
  (C#: inside the layout's safe area)                 let placement = CaptionPlacement::from_guide((1280, 624),
                                                          &SafeAreaGuides::broadcast_title_safe(), Default::default())?;
                                                      let p = CaptionPresenter::new(track, session.fonts(), layout, None)?
                                                          .with_placement(placement);              // THE OPT-IN
                                                      session.draw_captions_with(p);
  TEXT ALONG A PATH:
  session.Animations.BindTextAlongPath(block,         session.animations_mut().bind_text_along_path(&block,
      TextUnit.Character, route, progress, options)       TextUnit::Character, "Letters", &route, progress, options)?;
      .AddTo(letters);
  layout.GetOutlineAlongPath(wave, 20f)               layout.get_outline_along_path(&wave, 20.0, 0.0)?
  Also: layout.lines()[i].baseline() / left() / width(), clusters()[i].bounds(), get_pieces(unit),
  is_truncated(), get_rect_for_index(i)? - C#'s names in snake_case; "-1" results are None.
  Differences: a cluster draws the glyphs of its paragraph's shaping (C# re-shapes each cluster alone),
  so contextual forms stay; CaptionTrack.Load(path) has no counterpart.

PATH COMBINING AND PATH QUERIES
  COMBINING - the AREAS two or more geometries fill, each by its own fill rule, every contour taken as
  closed: a.union(&b, tol)?, intersect, subtract, exclusive_or, combine_with(&b, PathBooleanOperation,
  tol)?, PathGeometry::combine_all(&[..], operation, tol)?, simplify(tol)? (REMOVE OVERLAPS: one clean
  outline), to_stroke_outline(&style, tol)? (the area a stroke covers - caps, joins, miter limit, the
  dashes cut first - as a fillable outline), inflate(distance, PathInflateOptions { join, miter_limit,
  tolerance })? / deflate(..)?. tol None is 0.25 units, as C#.
  HOW (as NoSkia): curves are flattened with NoSkia's own flattening in double precision, every point
  is rounded once to a grid of 1/1024 unit, and the polygons are combined in exact 64-bit integer
  arithmetic - here by the i_overlay crate (NoSkia uses its own polygon engine). The STROKER of
  to_stroke_outline and inflate is NoSkia's own (its pieces, joins, caps and round chords), so the
  joins are C#'s. The result is closed polygons of lines on that grid, non-overlapping, with the
  non-zero rule; PathGeometry::empty() when nothing is left. Coordinates must stay within a million
  units (the error says so).
  DIFFERENCE FROM NoSkia: the same AREAS, but the contours' start points, directions and order come
  from i_overlay - a draw-on (trim) or a morph of a COMBINED path starts elsewhere than in NoSkia; the
  dashes of to_stroke_outline are cut by kurbo's dash iterator.
  QUERIES (C#'s arithmetic exactly, on the flattened path): contains(point, tol)?,
  distance_to_edge(point, tol)?, get_signed_distance(point, tol)? (negative inside),
  get_nearest_point(point, tol)? (an error for an empty geometry, where C# throws), distance(&other,
  tol)? and distance_with_offset(&other, offset, tol)? (0 when the areas touch or overlap),
  flatten(tol)? -> Vec<PathPolyline>; get_point_at_with(fraction, extend_beyond_ends) continues the
  path straight past its ends. Each query flattens the geometry again; for many questions of one area
  use a FormationArea, which flattens once.
  C#                                                  Rust
  A COMBINED PATH:
  var cut = star.Subtract(circle);                    let cut = star.subtract(&circle, None)?;
  var all = PathGeometry.CombineAll(parts,            let all = PathGeometry::combine_all(&parts,
      PathBooleanOperation.Union);                        PathBooleanOperation::Union, None)?;
  var outline = sig.ToStrokeOutline(new               let outline = sig.to_stroke_outline(&PathStrokeStyle::new(26.0,
      PathStrokeStyle(26f, PathStrokeCap.Round));         PathStrokeStyleOptions { cap: PathStrokeCap::Round, ..Default::default() })?, None)?;
  star.Inflate(10, PathStrokeJoin.Miter, 2f)          star.inflate(10.0, PathInflateOptions { join: PathStrokeJoin::Miter,
                                                          miter_limit: 2.0, ..Default::default() })?
  block.MergeOverlaps = true                          block.set_merge_overlaps(true)
  A QUERY:
  disc.Contains(p); disc.DistanceToEdge(p, 0.5f)      disc.contains(p, None)?; disc.distance_to_edge(p, Some(0.5))?
  a.Distance(b, new PointF(125, 0))                   a.distance_with_offset(&b, PointF::new(125.0, 0.0), None)?

COMBINED BINDINGS
  session.animations_mut().bind_with_mode(target, property, track, mode)? is C#'s Bind(..., BindingMode):
  BindingMode::Add adds the value to whatever the other bindings set (a sway on top of a turn, a shake
  on top of a path), Multiply multiplies by it (a pulse on top of a scale), Replace is the plain bind.
  Combinable: on an element or a layer Rotation, Scale (float: both axes; point: X and Y), ScaleX,
  ScaleY, Opacity (clamped to 0..1 after combining) and Position; on the camera Rotation, Scale (zoom)
  and Position (pan). A function of the time is BoundTrack::float_fn / point_fn. THE ORDER (C#'s):
  once a combining binding exists, every property it combines into is first put back to its resting
  value, then the Replace bindings apply in order, then every Multiply, then every Add - so the result
  does not depend on the order the bindings were made in. binding.mode() tells which.
  C#                                                  Rust
  session.Animations.Bind(pendulum, AnimatedProperty  session.animations_mut().bind_with_mode(pendulum,
      .Rotation, t => Flutter(t), BindingMode.Add);       AnimatedProperty::Rotation, BoundTrack::float_fn(flutter), BindingMode::Add)?;
  session.Animations.Bind(star, AnimatedProperty      session.animations_mut().bind_with_mode(star,
      .Scale, t => Pulse(t), BindingMode.Multiply);       AnimatedProperty::Scale, BoundTrack::float_fn(pulse), BindingMode::Multiply)?;
  Difference: a camera zoom the combination makes 0 or negative is refused by the camera (it keeps its
  last zoom) where C# throws.

ROLLING AND TRAINS ALONG PATHS
  - PathTravelOptions has C#'s every member: orient_to_path, rotation_offset, extend_beyond_ends,
    roll_radius (0 none; negative rolls the other way), upright_at_progress (1). Build it with
    `..PathTravelOptions::default()` (upright 1 is not a zero).
  - bind_along_path with a roll_radius ROLLS the element like a wheel (the roll ADDS to the facing
    rotation); with extend_beyond_ends a progress outside 0..1 carries it straight past the ends (an
    overshoot that settles). bind_roll_along_path(id, &path, progress, radius, upright, orient)? is the
    short form.
  - bind_train_along_path(&[ids], &path, progress, PathTrainOptions)? -> PathTrain sends several
    elements along ONE path as a train (order, stagger, spacing and spacing_scale, roll_radius /
    roll_on_baseline, scale_along_path, opacity_along_path, anchor_at_center, settle_in_place). The
    elements are on a layer before binding (C#'s AddTo has no counterpart); a car's gap, delay and
    roll change through train.set_car(i, car)?.
  C#                                                  Rust
  session.Animations.BindAlongPath(wheel, hills,      session.animations_mut().bind_along_path(wheel, &hills,
      progress, new PathTravelOptions {                   progress, PathTravelOptions { orient_to_path: true,
      OrientToPath = true, RollRadius = 30 });            roll_radius: 30.0, ..PathTravelOptions::default() })?;
  var train = session.Animations                      let train = session.animations_mut().bind_train_along_path(
      .BindTrainAlongPath(cars, road, progress,           &cars, &road, progress, PathTrainOptions { spacing: 30.0,
      new PathTrainOptions { Spacing = 30f });            ..PathTrainOptions::default() })?;
  Difference: on CURVED paths the travel distance is kurbo's arc length (to 1e-4 units), so a roll or
  a train on a curve agrees with NoSkia to that accuracy; on straight segments exactly.

FORMATIONS
  Many objects filling a shape and arriving over time, all functions of the time:
  FormationTargets::fill(&FormationArea::from_geometry(&shape, tol)?, options)? or
  fill_geometry(&shape, options)? -> targets on a jittered hexagonal grid ranked by an IArrivalOrder
  (RandomArrivalOrder, CenterOutArrivalOrder, DirectionalArrivalOrder, HeapArrivalOrder,
  CustomArrivalOrder, or a type of your own); the way in, an IArrivalPath (StraightArrival,
  CurvedArrival, SpiralArrival, SwayingFallArrival); when, an ArrivalSchedule (with an
  ArrivalRateProfile or a rate function); FormationMember::from_targets(&targets, &times, |target,
  time, k| path)?; then bind_formation(&members, "Rest", "Air", |k| resting_shape, Some(&mut |k|
  travelling_shapes))? -> FormationElements: resting elements on the first layer, shown from their
  arrival; travellers on the second, shown only while travelling, posed by one binding. DrawingRandom
  (seeded) and DrawingNoise::gradient come with them. The targets, orders (ties included), schedules
  and poses equal NoSkia's own output on the host; the calls into the maths library (sin, cos, exp,
  pow) may differ from C#'s in the last bits in a browser.
  C#                                                  Rust
  var targets = FormationTargets.Fill(shape,          let targets = FormationTargets::fill_geometry(&shape,
      new FormationTargetOptions { Spacing = 12f,         FormationTargetOptions { spacing: 12.0,
      Order = new HeapArrivalOrder(600f) });              order: Some(Box::new(HeapArrivalOrder::new(600.0)?)), ..Default::default() })?;
  var times = new ArrivalSchedule(3.0, 7.4)           let times = ArrivalSchedule::new(3.0, 7.4, None)?.get_times(targets.len());
      .GetTimes(targets.Count);
  var members = FormationMember.FromTargets(          let members = FormationMember::from_targets(&targets, &times,
      targets, times, (t, at, k) => new                   |t, at, _| -> Rc<dyn IArrivalPath> { Rc::new(SpiralArrival::new(
      SpiralArrival(new FormationPose(t.Position.X,           FormationPose::at(t.position.x, t.position.y), at - 1.5, at, 300.0,
      t.Position.Y), at - 1.5, at, 300f));                    SpiralArrivalOptions::default()).unwrap()) })?;
  session.Animations.BindFormation(members, rest,     session.animations_mut().bind_formation(&members, "Rest", "Air",
      air, k => Dot(k), k => new[] { Dot0() });           |k| dot(k), Some(&mut |_| vec![dot_at_origin()]))?;

LAYER VISIBILITY
  A bool track bound to a layer's IsVisible shows it while true and skips it while false (exact steps
  at its keys): bind(BindingTarget::layer("Hint"), AnimatedProperty::IsVisible, bools)?. A hidden
  layer is not drawn and not checked for safe areas; its bindings still apply. A HIDDEN MATTE STILL
  MASKS: a layer named by DrawingMask::from_layer cuts its masked layer whether it is shown or not.

SVG DOCUMENTS: PARTS, AND THE DOCUMENT'S OWN ANIMATION (SMIL, CSS)
  The crate takes an SVG document's TEXT or BYTES from anywhere - a call to the backend through the
  platform's generated client (the samples/GraphicsDemo way: a [WireFile] action serving the .svg),
  a file packed into the frontend, a string the program writes. It never reads a file, fetches a URL
  or reads a clock. usvg (default features off, its `text` feature on) parses the document, resolves
  CSS, use / defs / symbol / switch and paint servers into a tree; the crate turns it into a scene of
  PARTS (every element id) as plain data, and an SVG ELEMENT of a session draws that scene at the
  session's time through the crate's own renderer.
    let fonts = vec![Arc::new(roboto_bytes)];        // SVG <text> needs fonts: their BYTES
    let svg = Arc::new(DrawingSvg::from_svg(&markup, &DrawingSvgOptions { fonts, ..Default::default() })?);
    for line in svg.report() { /* what is not drawn or not played, one line each */ }
    let mut clock = DrawingSvgShape::new(svg.clone(), RectangleF::new(840.0, 170.0, 380.0, 380.0), DrawingSvgAspect::Meet);
    clock.get_part_mut("second")?.pivot = Some(PointF::new(150.0, 150.0));   // document coordinates
    clock.bind_part("second", AnimatedProperty::Rotation, FloatTrack::from_keys(&[(0.0, 0.0, EasingKind::Linear), (60.0, 360.0, EasingKind::Linear)])?)?;
    let id = session.draw_svg_shape(clock);          // or session.draw_svg(&svg, dest, aspect)
    session.animations_mut().bind(id, AnimatedProperty::Opacity, fade_in)?;   // a whole element, as any
  The element has the element properties of every shape (transform, opacity - the drawing fades as
  one -, time window, effects, bounds = the fitted box; Slice = the destination). Its part bindings
  and the document's own animations are evaluated at the SESSION'S TIME each frame; drawing never
  changes the shape (session.get_svg_mut(id) reaches it to change parts or bindings later).
  C#                                                  Rust
  var svg = new DrawingSvg(); svg.FromSvg(markup)     let svg = Arc::new(DrawingSvg::from_svg(&markup, &options)?);
      / svg.Load(stream)                                  / DrawingSvg::load(&bytes, &options)?   (-> Result; UTF-8, no .svgz)
  svg.Fonts.RegisterFont(path)                        DrawingSvgOptions { fonts: vec![Arc::new(bytes)], font_family: None, .. }
  svg.Bounds / svg.Warnings                           svg.bounds() / svg.warnings(); svg.report() ("Kind: message" lines)
  session.DrawSvg(svg, dest, aspect = Meet)           session.draw_svg(&svg, dest, DrawingSvgAspect::Meet) -> ElementId
  new DrawingSvgShape(svg, dest, aspect)              DrawingSvgShape::new(svg.clone(), dest, aspect); session.draw_svg_shape(shape)
  shape.GetPartIds() / HasPart / GetPart(id)          get_part_ids() / has_part(id) / get_part(id)? / get_part_mut(id)?
  part.Translation / Rotation / ScaleX / ScaleY /     the same fields, snake_case (pivot: Option<PointF>, document coordinates)
      Pivot / Opacity / IsVisible / TrimStart / TrimEnd
  part.Transform get / set, HasOverrides, Reset       transform() / set_transform(t), has_overrides(), reset()
  shape.ResetParts / MapToCalibrated / GetPartBounds  reset_parts() / map_to_calibrated(p) / get_part_bounds(id, time)
  shape.BindPart(animations, id, prop, FloatTrack)    shape.bind_part(id, prop, FloatTrack)?   (Rotation, Scale, ScaleX, ScaleY,
                                                          Opacity, TrimStart, TrimEnd; evaluated at the session's time)
  shape.BindPart(animations, id, prop, PointTrack)    shape.bind_part_point(id, prop, PointTrack)?   (Position, Anchor, Scale)
  DrawingSvgAspect Meet / Slice / Stretch / None      the same
  (no C# equivalent: the document's own animation)    plays by itself; shape.play_document_animations (default true),
                                                          shape.document_time_offset; svg.evaluate_animations(t)
  PICTURES IN A DOCUMENT: <image> is never decoded or fetched by the crate. shape.image_requests()
  lists what the document wants (SvgImageSource::Href(href) / Encoded { format, bytes }); hand over
  a picture with shape.set_image(&source, ImageSource) or set_image_href(href, ImageSource) - any
  ImageSource: decoded pixels (the picture-pixels form from the backend) or a PNG or JPEG file's
  bytes read with ImageSource::from_png / from_jpeg (see PICTURES AND SPRITES) - stretched over the
  image's box as the document places it. Without a picture it is not drawn (reported).
  SVG TEXT is laid out by usvg (anchors, tspans, textPath, letter and word spacing, decorations) in
  the fonts DrawingSvgOptions::fonts supplies and drawn as the paths usvg makes of the glyph outlines
  (unhinted). A family the fonts lack is drawn in DrawingSvgOptions::font_family (default: the first
  font's family) and reported (FontSubstituted); generic families (serif, sans-serif ...) are that
  family too; there is never a system font. Without fonts, text is reported (NoFontsRegistered) and
  not drawn.
  SPACE AND SEMANTICS (as C#): part overrides are in DOCUMENT coordinates (CSS px at 96 DPI, after the
  viewBox), applied as if the element were wrapped in an extra group; nested parts compose; opacities
  multiply; the nearest trimmed ancestor wins; trims cut strokes only, each path by the same
  fraction. Clip-path content is not affected; mask content is drawn with its parts' overrides. The
  pivot defaults to the centre of each owner node's box.
  THE DOCUMENT'S OWN ANIMATIONS REPLACE ATTRIBUTES (as SMIL / CSS say): an animated transform replaces
  the element's transform attribute (additive sums compose onto it), an animated opacity replaces the
  element's opacity, colours and stroke values replace the element's. INHERITED PROPERTIES BEHAVE AS
  IN A BROWSER: an animated fill, stroke, fill- / stroke-opacity, stroke width, dash array / offset or
  visibility on a GROUP reaches the elements inside it that do not set that property themselves (by
  an attribute, their style or a rule of the document's <style>); an element that sets its own keeps
  it. A program-driven part override applies on top (outside) of the document's own animation.
  DIFFERENCE FROM C#: an id inside content that a <use> instances (the <use>d element's own id
  included) does NOT reach the scene (usvg clears those ids), so it cannot be a part or be animated;
  report() names every animation that targets such an id (AnimationTargetInsideUse). Give each <use>
  element its own id and animate that. C# reaches every instance through the inner id.
  DRAWN: rect, circle, ellipse, line, polyline, polygon, path (arcs); transforms; groups; opacity;
    viewBox and preserveAspectRatio; use / defs / symbol / nested svg / switch; static CSS (<style>,
    style="", presentation attributes); fill and stroke with colours, fill-rule, fill- and
    stroke-opacity, stroke width, caps, joins (miter-clip drawn as miter), miter limit, dashes and
    dash offset; vector-effect="non-scaling-stroke" on rect, circle, ellipse, line, polyline, polygon
    and path (as an attribute, in style="" or from a rule of the document's <style>; not inherited, so
    on a <g> it does nothing; vector-effect: none switches it off): such a stroke is fixed in drawing
    units - neither the viewBox's scale, the document's transforms, the shape's placement nor the
    element's transform scale it (FIXED-WIDTH STROKES); paint-order; linear gradients and radial
    gradients with a focal point and focal radius (objectBoundingBox resolved), spread pad /
    reflect / repeat, gradientTransform; clip-path
    (one shape: a stencil clip; several shapes: their UNION); masks of mask-type="alpha";
    every mix-blend-mode value (plus-lighter too) and isolation; markers; a filter that is exactly ONE
    feGaussianBlur of the source graphic with one standard deviation (the crate's blur, radius =
    twice the standard deviation); <text> with supplied fonts; <image> with supplied pixels.
  APPROXIMATED (reported): luminance masks (the SVG default) are applied by their content's ALPHA; a
    mask on a mask is applied to the group as a second mask.
  NOT DRAWN (reported): <pattern> paints (the paint is drawn as nothing); every other filter (the
    element is drawn without it); images that are SVG documents; <image> without supplied pixels;
    <text> without fonts.
  NOT IN THE SCENE (usvg): display:none elements, unused <defs> content, zero-size shapes.
  NOT HONOURED: vector-effect="non-scaling-stroke" on an element inside <use>d content (usvg clears
    those ids) and on <text> (usvg draws text as paths of its own).
  IGNORED: shape-rendering crispEdges (the renderer always anti-aliases); clip paths on clip-path
    children.
  SMIL (time-based; evaluated for the session's time)
    SUPPORTED: <animate>, <set>, <animateTransform>, <animateMotion>; target = parent element or
      href="#id" (xlink:href too); elements without an id are given one (prefix codebrix-svg-part-).
      Attributes: opacity, fill-opacity, stroke-opacity, fill, stroke (colours incl. none /
      transparent / rgba), stroke-width, stroke-dashoffset, stroke-dasharray, visibility, display;
      animateTransform type translate, scale, rotate (with centre), skewX, skewY on `transform`.
      Timing: begin lists of offsets (every SMIL clock-value form, signed) and syncbases id.begin /
      id.end with offsets (chains that loop are followed); dur; end lists (offsets, syncbases);
      repeatCount (number or indefinite); repeatDur; min / max; restart always / whenNotActive /
      never; fill freeze / remove. Values: values, from-to, from-by, by, to, set to. calcMode discrete,
      linear, paced, spline with keySplines; keyTimes; additive replace / sum; accumulate sum; the
      sandwich (earlier begin first, then document order). animateMotion: path="...", <mpath href>,
      values / from / to / by points, keyPoints with keyTimes, rotate auto / auto-reverse / angle;
      applied on top of (outside) the element's transform.
    APPROXIMATED: animateMotion along a path with calcMode linear and no keyPoints plays paced
      (reported); a "to" animation of a colour starts from the first path's colour inside the target;
      mixed begin lists ("0s; click") play their time-based entries (the others reported).
    IGNORED (reported): event begins (click, mouseover, focusin, beginEvent, any "id.event"),
      accessKey(), wallclock(), id.repeat(n), begin="indefinite" alone; every other attribute (x, y,
      width, r, cx, d, points, stop-color, offset, gradient and pattern transforms ...); animateColor;
      discard; <script> and event attributes (scripts never run); targets that are not drawn (inside
      defs, display none, the root svg) and targets inside <use>d content (AnimationTargetInsideUse).
  THE CSS SUBSET (inside <style>, plus style="" on elements)
    SUPPORTED: @keyframes (and @-webkit-keyframes) with from / to / percentages and per-keyframe
      animation-timing-function; the animation shorthand (comma lists) and animation-name, -duration,
      -delay (negative too), -iteration-count (number, fractional, infinite), -direction (normal,
      reverse, alternate, alternate-reverse), -fill-mode (none, forwards, backwards, both),
      -timing-function (linear, ease, ease-in, ease-out, ease-in-out, cubic-bezier(), steps(n,
      start|end|jump-*), step-start, step-end), -play-state (paused = held at time 0); properties
      transform (translate/X/Y, scale/X/Y, rotate, skew/X/Y, matrix; px, %, deg, rad, grad, turn),
      opacity, fill, stroke, fill-opacity, stroke-opacity, stroke-width, stroke-dashoffset,
      stroke-dasharray, visibility; transform-origin (keywords, px, %) and transform-box (view-box,
      fill-box); missing 0% / 100% keyframes take the static value; selectors: type, #id, .class,
      compounds, descendant and '>' combinators, selector lists, by specificity then order; style=""
      wins. A CSS animation wins over SMIL on the same property. Transform lists of the same shape
      interpolate per function argument; lists of different shapes through CSS's 2D matrix
      decomposition.
    APPROXIMATED: fill-box is the element's loaded box.
    IGNORED (reported): other at-rules (@media, @supports, @import, @font-face ...) with everything
      inside them; rules with animations whose selectors use attributes, pseudo-classes or other
      combinators; animation names with no @keyframes; animations without a positive duration;
      keyframe selectors that are not from / to / a percentage.
    IGNORED (NOT reported): transitions, properties outside the list above inside @keyframes,
      @keyframes values that do not parse (the static value is used), animation-composition, var().
  No equivalent: RasterizeToPng / RasterizeToBitmap / Render(canvas) and NoSkia's DrawingPicture
  display list (the element draws the scene live; there is no CPU rasteriser), TextEmission, Dispose
  (Drop). NoSkia's SVG parser is not ported; usvg does that work here.

SAFE AREAS (a preview aid for animations that will be rendered to video)
  - SafeAreaGuides presets with NoSkia's figures and sources (instagram_reels_ad(), tiktok(),
    youtube_shorts_ad(), broadcast_title_safe(), youtube_end_screen() ...); your own with
    SafeAreaGuide::new(name, SafeAreaMargins::new(..)?, options)? and SafeAreaGuide::combine.
  - DrawingLayout::from_guide(calibration_size, Some(&guide), DrawingLayoutGuideOptions)? places against
    the guide's safe rectangle (C#'s DrawingLayout constructor that takes a guide).
  - session.check_safe_area(&guide, frame_size, SafeAreaCheckOptions) -> findings for the flagged
    elements (set_is_safe_area_checked on an element or a whole layer) that reach into unsafe zones.
  - session.set_safe_area_overlay(Some(SafeAreaOverlay::new(guide))) DRAWS the guide over the frame -
    a translucent mask over the unsafe zones (opacity scales it) and a thin line along the safe edge -
    after the layers and before the debug overlay, in the pixels of the frame as shown (the session's
    aspect-fitted rectangle), not faded with the session. Its edges are anti-aliased (C# draws them
    without anti-aliasing).
  - Captions sit against the frame by default; inside a guide only on the opt-in (CAPTIONS above).
  C#                                                  Rust
  new DrawingLayout(size, SafeAreaGuides              DrawingLayout::from_guide(size, Some(&SafeAreaGuides::instagram_reels_ad()),
      .InstagramReelsAd)                                  DrawingLayoutGuideOptions::default())?
  layer.IsSafeAreaChecked = true;                     layer.set_is_safe_area_checked(true);
  session.CheckSafeArea(SafeAreaGuides.TikTok)        session.check_safe_area(&SafeAreaGuides::tiktok(), None, SafeAreaCheckOptions::default())
  session.ExportPixels(px, size, true,                session.set_safe_area_overlay(Some(SafeAreaOverlay::new(guide)));
      new SafeAreaOverlay(guide))                         // drawn live; there are no exports

PICTURES AND SPRITES: DECODED PIXELS, THE PICTURE-PIXELS FORM, OR PNG AND JPEG FILES
  An ImageSource is a copy of straight-alpha RGBA pixels (sRGB-encoded, rows top to bottom),
  premultiplied on the GPU. THE WAYS TO SUPPLY A PICTURE:
    1. DECODED ON THE BACKEND (any format the backend can read): decode there (CodeBrix.Imaging:
       Image.Load<Rgba32>, CopyPixelDataTo), write the pixels with this package's .NET writer -
       PicturePixelsWriter.Write(rgba, width, height) (namespace
       CodeBrix.WebPlatform.WebGpu.Animation.Pictures; the form: "CBPX", a major and a minor version,
       width, height, then the RGBA bytes; PicturePixelsFormat holds its constants) - and send it from a
       [WireFile] action; the frontend fetches it with the generated `<name>_bytes` call and hands it
       to ImageSource::from_picture_pixels(&bytes)?. The package's .NET side has NO imaging
       dependency: it writes pixels the backend decoded.
    2. A PNG FILE, DECODED IN THE BROWSER: ImageSource::from_png(&bytes)? reads the bytes of a PNG
       file (fetched from the backend as a file answer, or carried in the frontend). PNG and JPEG
       are the ONLY file formats the crate reads (both through codebrix-webgpu's decoders). Every
       PNG colour type and bit depth is brought to straight 8-bit RGBA (grey to equal channels,
       palettes looked up with their transparency, colour keys to alpha 0, 16-bit samples to their
       high byte, interlacing undone); the values are taken as
       sRGB-encoded (the file's gamma, chromaticity and ICC chunks are not applied); of an animated
       PNG only the default image is read; at most 8192 x 8192 pixels.
    2b. A JPEG FILE, DECODED IN THE BROWSER: ImageSource::from_jpeg(&bytes)? (no C# equivalent)
       reads baseline and progressive JPEG with 8-bit samples and any chroma subsampling: colour
       becomes straight RGBA, greyscale equal channels, alpha 255; the values are taken as
       sRGB-encoded (an ICC profile is not applied). THE EXIF ORIENTATION IS NOT APPLIED: a phone
       photo that relies on the tag to stand upright is drawn on its side - rotate it before saving,
       or turn the ImageShape. Refused, with a message: CMYK and YCCK, 12-bit, lossless,
       arithmetic-coded and hierarchical files, damaged or cut-short files, and pictures larger than
       8192 x 8192.
    3. MADE IN THE FRONTEND: any RGBA bytes the application computes or carries
       (ImageSource::from_rgba), or a CPU plane image (ImageSource::from_plane_image - C#'s
       FromPlaneImage; the planes' float-to-byte rule).
  Answer picture requests with an ETag and "Cache-Control: public, no-cache": a reload then
  revalidates (304) instead of downloading again.
  A clone of an ImageSource or a SpriteSheet is the SAME picture (C# shares one reference): the
  pixels are kept once and uploaded to the GPU once (counted in RenderStats::image_uploads /
  image_upload_bytes). The GPU's textures are SWEPT EVERY FRAME by one rule with a byte budget
  (C#'s Dispose is Drop): a picture not drawn for a while goes, and while the pictures held but not
  drawn this frame weigh more than the budget, the least recently drawn go first (a picture drawn
  every frame, or every other frame, is never dropped). So a scene that makes a NEW picture every
  frame holds a few dead textures, not hundreds - but each new picture is still a whole upload; reuse
  the ImageSource when the pixels do not change. (The figures are in the repository's
  MAINTAINER-README.)
  - ImageShape::new(source, destination, ImageShapeOptions { source_rectangle, sampling, tint })?: the
    picture (or part of it, in its pixels) STRETCHED into the destination, as C# does (no fit or
    alignment modes: give a destination of the picture's aspect to keep it); ImageSampling::Linear
    (default) or Nearest; tint multiplies the straight values channel by channel, alpha included.
    Edges are anti-aliased; the element's transform, opacity, time window, groups and effects apply.
  - SpriteSheet::from_grid(&image, columns, rows, frame_count)? / from_rectangles / new(frames):
    cutting COPIES each cell, so cells never bleed.
  - SpriteShape::new(sheet, destination, SpriteShapeOptions { frames_per_second, playback, sampling })?
    with frame_index, playback_start_time and tint: the frame is a FUNCTION OF THE SESSION TIME (C#'s
    n = FrameIndex + floor((t - start) x fps + 1e-6), then Loop / PingPong / Hold); get_frame_at(t).
  - Bindable: AnimatedProperty::Tint (colour track; pictures and sprites), FrameIndex (float track,
    rounded down; sprites).
  C#                                                  Rust
  var src = ImageSource.FromRgba(rgba, w, h);         let src = ImageSource::from_rgba(&rgba, w, h)?;
  session.DrawImage(src, new RectangleF(60, 140,      session.draw_image(&src, RectangleF::new(60.0, 140.0,
      320, 200));                                         320.0, 200.0), ImageSampling::Linear)?;
  new ImageShape(src, dest, new Rectangle(x, y, w,    ImageShape::new(src.clone(), dest, ImageShapeOptions {
      h), ImageSampling.Nearest, tint);                   source_rectangle: Some(Rectangle::new(x, y, w, h)),
                                                          sampling: ImageSampling::Nearest, tint: Some(tint) })?
  A SPRITE:
  var sheet = SpriteSheet.FromGrid(img, 8, 1);        let sheet = SpriteSheet::from_grid(&img, 8, 1, 0)?;
  session.DrawSprite(sheet, dest, 12,                 session.draw_sprite(&sheet, dest, SpriteShapeOptions {
      SpritePlayback.PingPong);                           frames_per_second: 12.0, playback: SpritePlayback::PingPong,
                                                          ..Default::default() })?;
  session.Animations.Bind(sprite,                     session.animations_mut().bind(sprite,
      AnimatedProperty.FrameIndex, steps);                AnimatedProperty::FrameIndex, steps)?;
  ImageShape::from_picture_target(&target, destination, sampling)? draws a picture another session
  draws on the GPU every frame (PICTURE TARGETS below).
  No equivalent: ImageSource.FromFile / FromImagingImage / FromDrawingBitmap, and FromEncoded for any
  format but PNG and JPEG (decode on the backend and send the picture-pixels form instead).
  NoSkia's DrawingPicture
  (the display list its SVG renderer compiles to) is not a consumer type here: record shapes once
  into a group (or a VectorScene) - the renderer keeps their flattened geometry between frames.

MASKS AND EFFECTS
  - DrawingMask::inside(path) / outside(path) / from_layer(name, invert), set on a layer
    (layer.set_mask) or a group (group.set_mask); to mask one element, put it in a group. A mask is in
    the SAME coordinates as what it masks (a group's turns with the group; a layer's follows the layer
    and the camera). The content is drawn whole first and the mask multiplies its coverage once, so
    overlapping elements show no seam. Replace a layer's mask every frame to move it (a spotlight).
  - THE MATTE LAYER IS NAMED: from_layer names a layer OF THE SAME SESSION (C# takes the layer object,
    which may be outside the session). Hide that layer (set_is_visible(false)) to use it only as a
    matte; its elements' coverage, alpha included, at the session's time, is the mask. A name the
    session does not have covers nothing AND IS REPORTED, once: take the session's reports after
    drawing (session.take_reports()) and show or log them beside the device's errors.
  - DrawingMask::from_plane(&plane, destination, invert)? (C#'s FromPlane; from_plane_at_origin for
    the plane's own rectangle): the content kept in proportion to a plane's values stretched over
    the rectangle, 0 outside it (1 when inverted); the values are copied once as 8-bit alpha (C#'s
    UnitToByte) and sampled bilinear on the GPU.
  - DrawingEffects::blur(radius)? / drop_shadow(offset, blur_radius, color)? /
    drop_shadow_with_direction(..., ShadowDirection::RotatesWithElement)? / glow(radius, color,
    strength)?, combined with with_shadow / with_glow / with_blur_radius ...; set on any element
    (shape.set_effects / with_effects) or a layer (layer.set_effects). C#'s order and meaning: the
    content blurred, a glow from its coverage, a shadow from its coverage moved by the offset; content
    over glow over shadow; a radius is twice the Gaussian's standard deviation; sizes are in the
    element's own units and follow its scale, its groups', its layer's, the camera's zoom and the canvas
    size; a shadow FixedInScene keeps its direction however the element turns (only the camera's roll
    turns it), RotatesWithElement turns with the element.
  - Bindable (element or layer; from no effects they start from DrawingEffects::NONE, as in C#):
    BlurRadius, ShadowBlurRadius, GlowRadius (float tracks), ShadowOffset (point track), ShadowColor,
    GlowColor (colour tracks).
  C#                                                  Rust
  A MASK:
  group.Mask = DrawingMask.Inside(star);              group.set_mask(Some(DrawingMask::inside(star)));
  card.Mask = DrawingMask.Outside(keyhole);           card_layer.set_mask(Some(DrawingMask::outside(keyhole)));
  stripes.Mask = DrawingMask.FromLayer(matte);        stripes_layer.set_mask(Some(DrawingMask::from_layer("Matte", false)));
  AN EFFECT:
  star.Effects = DrawingEffects.Glow(16,              session.find_element_mut(star).unwrap().set_effects(Some(
      orange, 1.5f);                                      DrawingEffects::glow(16.0, orange, 1.5)?));
  card.Effects = DrawingEffects.DropShadow(           DrawingShape::from(card).with_effects(DrawingEffects::drop_shadow(
      new PointF(0, 4), 6, shadowColor);                  PointF::new(0.0, 4.0), 6.0, shadow_color)?)
  session.Animations.Bind(card,                       session.animations_mut().bind(card,
      AnimatedProperty.ShadowOffset, offsets);            AnimatedProperty::ShadowOffset, offsets)?;
  layer.Effects = DrawingEffects.DropShadow(...);     layer.set_effects(Some(DrawingEffects::drop_shadow(...)?));
  Differences and limits: the blur works on linear premultiplied colour (NoSkia's planes blur
  sRGB-encoded values; edges of colourful shapes over different colours can look a shade different),
  and at the canvas's edge the blur sees transparency (NoSkia mirrors). Mask edges are always
  anti-aliased (C#'s antialias: false has no effect). Of the MaskMeaning exports only HardEdged is
  computed (see PAINTERLY PLAYBACK). NoSkia has no luminance masks and no mask feathering, so
  neither has this crate.

PIXEL PLANES: CPU CALCULATION WITH NoSkia'S VALUES (module planes)
  - THE PLANES ARE CPU CALCULATION, SINGLE-THREADED. A browser gives a frontend ONE thread, so every
    plane operation runs on it, row after row (C# shares rows out across threads; its results never
    depend on that, so the values are the same - integer and plain float arithmetic bit for bit; the
    blur kernel's exp, the gamma's pow and the particles' cos and sin come from Rust's own math
    library in a browser and may differ from C# in the last bit). Use planes for masks, data, noise
    VALUES and small images; for whole frames in real time use THE GPU VERSIONS below. A whole
    1280 x 720 image costs milliseconds to tens of milliseconds per operation on the CPU.
  - DrawingPlane: one rectangle of f32 values (a channel, a mask, light, height, noise), row-major,
    never clamped; same-size operations change it IN PLACE and return &mut (or Result) so calls
    chain; resized / downsampled / create_bloom return a new plane. DrawingPlaneImage: four planes,
    STRAIGHT alpha; colour arithmetic leaves alpha alone, compositing is Porter-Duff on clamped
    alpha, mixing and every neighbourhood filter work premultiplied. A pixel is [r, g, b, a].
  - In-memory pixel access only (to_bytes / from_bytes, to_rgba / from_rgba): no file encoders and no
    CodeBrix.Imaging type; ImageSource::from_plane_image puts a plane image on screen,
    DrawingMask::from_plane uses a plane as a mask, FormationArea::from_mask fills a plane's shape.
  - TRANSITIONS (DrawingTransition: dissolve, wipe(direction, softness), slide, push,
    fade_through_color) are values applied at a FRACTION the caller computes from the frame's time;
    0 and 1 give the two frames exactly. C#'s "destination may be the outgoing frame" is
    apply_bytes_onto / apply_onto.
  - NOISE AND RANDOMNESS: DrawingRandom and DrawingNoise (value, gradient, 3D, fractal, plane fills)
    are pure functions of their seed and coordinates, bit for bit with C#. For noise that evolves,
    pass the time (times a speed) as z. A third kind, DrawingNoiseKind::Caustics (no C# equivalent),
    is the light net of a pool floor: bright curved lines over a dark floor (-1 the floor, 1 the
    brightest line) that drift and re-form as z grows, without a visible loop or seams;
    DrawingNoise::caustics(x, y, z, seed), and every call that takes a kind (sample, sample_3d,
    fractal, fractal_3d, fill) with the usual octave rule. It has no C# original to match, so it is
    computed with the GPU's 32-bit arithmetic on the CPU too: THE SAME PATTERN on the CPU and drawn by
    a NoiseShape (within float rounding). A suggested look: 2 octaves, lacunarity 1.7, gain 0.6, stops
    from dark blue through pale blue to white.
  - PARTICLES: DrawingParticleSystem is DATA (a seed and emitters); get_particles(t) is a pure
    function of the time - frames can be drawn in any order. MOTION BLUR: DrawingMotionBlur's
    sub-frame times and DrawingFrameAccumulator's averages (premultiplied, exact for 8-bit frames).
  C#                                                  Rust
  new DrawingPlane(w, h, 0.5f)                        DrawingPlane::with_value(w, h, 0.5)?
  plane.Add(0.25f).Multiply(mask).Clamp(0f, 1f)       plane.add(0.25).multiply_plane(&mask)?.clamp(0.0, 1.0)?
  plane.GaussianBlur(3f) / Resized(w, h)              plane.gaussian_blur(3.0)? / resized(w, h, None)?
  plane.CreateBloom(4, new[] { (3f, .8f) }, 0.6f)     plane.create_bloom(4, &[(3.0, 0.8)], DrawingPlaneBloomOptions { gain: 0.6, ..Default::default() })?
  image.Fill((x, y) => v4) / FillParallel(...)        image.fill_with(|x, y| [r, g, b, a])   (one thread; the same values)
  image.CompositeOver(src, 0.8f)                      image.composite_over(&src, Some(0.8))?
  DrawingTransition.Wipe(Down, 40f).Apply(a, b, f)    DrawingTransition::wipe(DrawingTransitionDirection::Down, Some(40.0))?.apply(&a, &b, f)?
  DrawingNoise.Fill(plane, Gradient, 42, 0.01,        DrawingNoise::fill(&mut plane, DrawingNoiseKind::Gradient, 42, 0.01,
      z: t * 0.5, octaves: 3)                             DrawingNoiseFillOptions { z: Some(t * 0.5), octaves: 3, ..Default::default() })?
  emitter.Rate = 30f; s.AddEmitter(emitter)           let e = s.add_emitter(DrawingParticleEmitter::new()); e.set_rate(30.0)?;
  s.GetParticles(t)                                   s.get_particles(t)?
  DrawingMotionBlur.GetSampleTimes(t, 1/60.0, 5)      DrawingMotionBlur::get_sample_times(t, 1.0 / 60.0, 5, None)?
  ToImagingImage / FromImagingImage                   no equivalent (bytes only)

THE GPU VERSIONS: FRAME OPERATIONS, LAYER TRANSITIONS, NOISE AND PARTICLES DRAWN ON THE GPU
  What a real-time scene uses instead of whole-frame CPU work (no C# types: C# does all of it on
  plane images on the CPU). The same formulas and parameters as the planes', values within float
  rounding - never bit for bit.
  - layer.set_frame_operations(Some(DrawingFrameOperations::NONE.with_tone_curve(k, gain)
    .with_gamma(e)?.with_vignette(FrameVignette { center_x, center_y, radius_x, radius_y, strength,
    minimum })?.with_bloom(sigma, gain)?)): the planes' tone curve, gamma and vignette on the layer's
    STRAIGHT sRGB colour (alpha left alone; vignette sizes in calibrated units), and a bloom (the
    layer's content blurred by sigma, times the gain, ADDED). The layer is drawn offscreen once, as
    for effects; on a layer that also has a glow, the bloom is not drawn.
  - session.set_layer_transition("Outgoing", "Incoming", transition, fraction)? (set it every frame;
    clear_layer_transition ends it): the INCOMING layer over the OUTGOING one - dissolve and wipe
    weigh the incoming layer per pixel, a fade through a colour shows the colour over the outgoing
    layer and then fades it into the incoming one, slides and pushes move the layers by whole device
    pixels inside the session's frame. EXACT FOR OPAQUE LAYERS (the planes transition whole frames);
    a translucent incoming layer is composited over the outgoing one rather than mixed with it. One
    transition per session at a time (use one session per panel for several).
  - NoiseShape::new(x, y, width, height, kind, seed, frequency, NoiseShapeOptions { fill, stops })?:
    noise COMPUTED PER PIXEL ON THE GPU (grain, clouds, dissolve patterns, textured fills) with the
    planes' parameters (kind, seed, frequency per calibrated unit, z, octaves, lacunarity, gain,
    offset) and character, mapped through colour stops (value -1 at position 0, 1 at position 1; two
    stops at one position make a hard threshold). For the value and gradient kinds, NOT THE SAME
    PATTERN as the CPU's for a seed (the GPU hashes with 32-bit integers in 32-bit floats; the CPU is
    NoSkia's exact 64-bit arithmetic) - the Caustics kind is the one that IS the same on both:
    a ported scene that DRAWS noise shows a different pattern, while anything POSITIONED by noise or
    DrawingRandom lands exactly where C# puts it. DrawingNoise::fill stays for exact pixels and is
    SLOW (a full 1280 x 720 fill takes a frame or more on one thread).
  - ParticlesShape::new(&system, t, softness)? / session.draw_particles(&system, t, softness)? (C#'s
    ParticlesShape / DrawParticles): the particles of THAT time as soft discs, ALL of them in ONE
    instanced GPU draw (softness 0 solid, 1 a radial fade; later particles over earlier ones). Make
    a new shape per frame (or use an AnimatedShape).
  - CPU plane images reach the screen as pictures (ImageSource::from_plane_image) - keep them small
    and let the renderer stretch them; a new picture each frame costs an upload each frame.
  - Motion blur by accumulation has no GPU version here (it needs several renders of one frame).

ANIMATED BRUSH STROKES AND PAINTERLY PLAYBACK (module painting, shapes::BrushStrokeShape)
  The crate PLAYS painterly animation: brushstrokes placed and animated by code, ownership maps that
  say which stroke owns each pixel, colour pickup, brush textures measured in each stroke's own
  axes, seams between strokes, hard-edged masks, and keyframe players that paint a costly picture
  every few seconds and make the frames between with a transition - calculation on the CPU with
  NoSkia's values, the drawing on the GPU. Nothing drawn reads a clock: pass the frame's time.
  - BrushStrokeShape is drawn by the renderer as an anti-aliased filled polygon in its colour or
    fill. PAINTING A STROKE ON: stroke.with_progress(p) (NO C# EQUIVALENT; at 1 it is exactly C#'s
    stroke): painted from its first tip to s = -0.5 + p along its axis - put stroke.clone()
    .with_progress(track(t)) in an AnimatedShape factory, or rebuild the layer each frame.
  - THE HARD-EDGED PIXEL RULE: a pixel belongs to an area when its CENTRE is inside it under the
    area's fill rule; a centre exactly on an edge is inside on a left or top edge, outside on a right
    or bottom edge. export_mask(size, MaskMeaning::HardEdged) and export_ownership_map(size) use it
    (the LAST element in painting order owns a pixel; a group or an animated shape is one element).
    The Coverage and VisibleAlpha meanings measure the drawn frame (a GPU readback, kept for tests):
    export_mask refuses them and says to use HardEdged.
  - COLOURING IN REAL TIME: re-colouring a whole map on the CPU every frame (apply_colors,
    create_boundary_plane, blend_color) does not fit a 60 fps frame at video sizes on one thread.
    Draw the map with OwnershipShape::new(&map, &colors, destination, OwnershipShapeOptions {
    unowned, seam_color, seam_strength })? (or from_float_colors with get_mean_colors' output): the
    map goes up as a picture of owner numbers with the colours beside it and the GPU colours every
    pixel and blends the seams in (the boundary softened over 3 x 3, about create_boundary_plane(0.8)).
    Each OwnershipShape::new is a NEW picture (a whole upload). For colours that change every frame,
    make the shape ONCE and re-colour it with base.with_colors(&colors)? (or with_float_colors; no C#
    equivalent): the same map with other colours, sharing the uploaded picture of owner numbers, so
    ONLY THE COLOUR TABLE travels to the GPU (the map's width x 4 bytes per row of colours; counted in
    RenderStats::color_table_uploads / color_table_upload_bytes); with_destination(rect)? moves it
    without a new picture. Two colourings of one map drawn in the SAME frame need two textures (the
    second is uploaded whole once, then nothing while its colours stay).
    For paintings that cost more than a frame, use a KeyframePlayer: prefetch(t) after each frame
    paints at most ONE keyframe ahead, and the frames between two cached keyframes can be dissolved
    ON THE GPU (keyframe k as a picture, keyframe k + 1 over it at the fraction) instead of by
    render_frame's CPU mix.
  - ONE THREAD: C#'s keyframe players paint on a background worker; here a keyframe is painted when
    first needed (render_frame) or earlier by prefetch. The paint function returns Result.
  C#                                                  Rust
  session.DrawBrushStroke(cx, cy, l, w, angle,        session.draw_brush_stroke(cx, cy, l, w, angle,
      bend, color)                                        BrushStrokeShapeOptions { bend, color: Some(c), ..Default::default() })?
  (none)                                              stroke.with_progress(p)
  session.ExportMask(size, MaskMeaning.HardEdged)     session.export_mask((w, h), MaskMeaning::HardEdged)?
  session.ExportOwnershipMap(layer, size)             session.export_layer_ownership_map("Name", (w, h))?
  map.FillUnowned() / CreateBoundaryPlane(0.8f)       map.fill_unowned() / map.create_boundary_plane(Some(0.8))?
  map.GetMeanColors(image) / ApplyColors(image, c)    map.get_mean_colors(&image) / map.apply_colors(&mut image, &c)?
  map.FillByElement(plane, p => .., unowned, axes)    map.fill_by_element(&mut plane, |p| .., Some(unowned), Some(&axes))?
  new KeyframePlayer(w, h, 1.0, paint, null, 6)       KeyframePlayer::new(w, h, 1.0, |k, t, rgba| { ..; Ok(()) },
                                                          KeyframePlayerOptions { cache_capacity: 6, ..Default::default() })?
  player.RenderFrame(t, rgba) / Prefetch(t)           player.render_frame(t, &mut rgba)? / player.prefetch(t)?
  map.ApplyColors(image, colors) every frame          let base = OwnershipShape::new(&map, &colors0, dest, opts)?;   // once
                                                      let shape = base.with_colors(&colors_at(t))?;   // per frame: the colour table only
  WHAT IS NOT HERE, AND WHY: interactive drawing - pointer-driven stroke capture (PointerPressed /
  PointerMoved / PointerReleased / PointerCanceled and their normalized forms, UndoLastStroke, the
  active stroke and ActiveStrokeOpacity, the freehand Stroke and StrokePoint, DrawingLayer.AddStroke):
  this crate plays animations, it is not a drawing tool (a recorded stroke plays as a PolylineShape
  or a PathShape with a trim); render-ahead on a worker thread; the Coverage and VisibleAlpha mask
  meanings; exports at a default export size (pass the size).

PAINTERLY STROKE SETS ON THE GPU (module strokes, shapes::BrushStrokesShape; no C# equivalent)
  A painting of TENS OF THOUSANDS of brushstrokes that paints itself on, travels with what it
  depicts and boils, drawn as ONE element in ONE instanced GPU draw - where a BrushStrokeShape per
  stroke costs the CPU work and a draw call per stroke every frame. THE CRATE SHIPS NO ARTWORK AND
  NO BRUSH STYLES: the pictures a painting is planned from, the brush tips, the palettes and every
  setting are the application's own (made in code, or pixels it supplies); the defaults below are
  starting points, not a style.
  - A RECORD is plain data: BrushStrokeRecord { center, length, width, angle (degrees clockwise
    from +x), bend, profile, color, load (0..1), tip, anchors: [u16; 2], anchor_blend (0..1), start,
    end (session seconds), seed }. BrushStrokeRecord::new(center, length, width, angle, color) gives
    bend 0, profile 4, load 1, no tip, anchors [0, 0], blend 0, a seed of 0 and start = end =
    BrushStrokeRecord::ALWAYS (always whole). The stroke paints itself on from start to end, from its
    first tip; progress_at(t), has_started(t), bounds(); to_shape(t)? is its CPU twin (a
    BrushStrokeShape, without the tip) and from_shape(&shape) the other way.
  - A SET: BrushStrokeSet::new(records)? checks every record (the error names it). A SET IS
    IMMUTABLE: its records are shared by its clones and uploaded to the GPU ONCE per set (64 bytes a
    stroke); to change strokes, make a new set (a new upload). Two sets made apart are never equal,
    even with equal records. id(), records(), len(), bounds(), anchor_bounds(), first_start(),
    started_count(t).
  - DRAWING: BrushStrokesShape::new(set, BrushStrokesOptions { .. })? drawn with
    session.draw_brush_strokes(shape) (-> ElementId; the active layer) or layer.add_shape(shape).
    BrushStrokesOptions: time_offset (0: the set's times are the session time minus it),
    head_softness (0..1; 0 a straight cut at the head, more fades the head out over that fraction of
    the length), tip_mirroring (true: a stroke's tip mirrored across by its seed), boil and flow
    (None). Records are drawn in RECORD ORDER (later over earlier), and only up to the last one that
    has started: sort a set by start (a painting timeline does) and the GPU draws only what has
    started. Per frame the CPU sends one small block per set (its placement, the time, the opacity,
    the anchors' transforms), so the CPU's work does not grow with the number of strokes. A set is
    culled before its first stroke starts and when it is off the canvas. The GPU draws the stroke's
    exact outline with a smooth edge (the CPU twin is a polygon of it: the tips differ slightly); a
    stroke narrower than one device pixel is drawn one pixel wide at a lower alpha. Below full
    opacity the set is drawn whole into a group that is then faded (strokes never show through each
    other) - one nesting level; at full opacity none.
  - BRUSH TIPS (the application's own pictures): BrushTip::new(coverage_plane, height_plane) or
    BrushTip::from_image(&picture, height_from_luminance)? (coverage = the picture's alpha; height =
    its luminance when asked, else flat); BrushTips::new((width, height), &[tips])? with each side a
    POWER OF TWO up to BrushTips::MAX_SIZE (2048) and at most BrushTips::MAX_TIPS (255) tips; the mip
    levels are made on the CPU; shape.with_tips(tips). A record's `tip` is 1-BASED into the tips (0 =
    no picture: a solid stroke). A tip's x runs ALONG the stroke (first tip to last) and its y
    ACROSS it, stretched over the whole stroke (it does not repeat). A tip's height is what the
    SURFACE FINISH lights.
  - ANCHORS (strokes that travel with what they depict): shape.with_anchors(vec![StrokeAnchor::Fixed(
    transform), StrokeAnchor::FollowElement(id)])? (at most BrushStrokesShape::MAX_ANCHORS, 256). A
    record's anchors index the list and anchor_blend mixes the two transforms linearly (a hem's strokes
    between the body and the hem; strong turns between the two narrow the stroke); an index past the
    list is no transform; with no anchors every stroke is in the set's own coordinates.
    FollowElement follows the element's OWN transform at the session's time (its bindings applied),
    whether the element is drawn or not (give it opacity 0 to show only the strokes): put the stroke
    shape at the same level as the element (the same layer or group). It follows the transform, not
    changes of shape inside the element, and not its parents' transforms. An id the session does not
    have is reported once (session.take_reports()) and acts as no transform.
  - BOIL: StrokeBoil { rate: 10, offset: 1, turn: 3, size: 0.05, seed: 0 } (Default): `rate` redraws a second (0: none), each stroke moved by up
    to `offset` set units per axis, turned by up to `turn` degrees, its length and width scaled by up
    to 1 +- `size`. Its timing is strokes::boil_epoch(seconds, rate) - the same as the sketch helper's
    boiled() - so pausing freezes it and a seek shows the same jitter.
  - FLOW: StrokeFlow { seed: 0, frequency: 0.01, speed: 0.2, turn: 30 } (Default): each stroke turned
    about its centre by up to `turn` degrees by a slowly moving noise (frequency per set unit, speed
    per second of the time): neighbouring strokes swirl together.
  - LOOKS (RENDER-TIME LOOKS): LINE leaves a set out; FLAT draws every stroke in its own (or picked)
    colour without its tip; an authored layer keeps its tips.
  - IN MASKS AND OWNERSHIP MAPS the whole set is ONE element; its area at a time is the union of its
    started strokes' CPU twins (shape.coverage_areas_at(t)), boil and flow left out - a CPU cost per
    stroke: for exports, not for every frame.
  - RenderStats: stroke_draws (one draw call each), strokes (instances drawn), stroke_uploads /
    stroke_upload_bytes, tip_uploads / tip_upload_bytes.
      let set = BrushStrokeSet::new(records)?;
      let tips = BrushTips::new((32, 16), &[BrushTip::new(DrawingPlane::from_data(32, 16, coverage)?, None)])?;
      let shape = BrushStrokesShape::new(set, BrushStrokesOptions { head_softness: 0.15,
              boil: Some(StrokeBoil { rate: 10.0, ..StrokeBoil::default() }), ..Default::default() })?
          .with_tips(tips)
          .with_anchors(vec![StrokeAnchor::Fixed(transform), StrokeAnchor::FollowElement(arm)])?;
      session.draw_brush_strokes(shape);
      // each frame, as for any session: session.set_time(frame.info.time); session.render(scene, w, h);

PLANNING A PAINTING: DIRECTION FIELDS, DETAIL MAPS, STROKE LAYOUT, THE PAINTING TIMELINE (no C#)
  Module painting: CPU work done ONCE, when a painting is made; seeded only (the same inputs give the
  same strokes).
  - DIRECTION FIELDS: a DirectionField is a grid of directions over a destination rectangle - a
    direction has no head, so angles are in (-90, 90] - with a strength 0..1, read bilinear between
    the cells' centres.
      DirectionField::from_picture(&picture, destination, PictureFieldOptions { field_size: None,
          smoothing: 3.0 })?   directions ALONG the picture's edges (its structure); the strength is
                               how clear the structure is there (weak in flat areas)
      DirectionField::from_noise(size, destination, kind, seed, frequency, NoiseFieldOptions { z: None,
          turns: 1.0, base_angle: 0.0, octaves: 1 })?   directions from noise (strength 1)
      DirectionField::from_guides(size, destination, &[guides])?   directions from GUIDES; the strength
                               is how much they agree:
          DirectionGuide::Along { path, reach, weight }            along a path (its nearest tangent)
          DirectionGuide::Around { center, reach, inward, weight } a swirl round a point, turned
                                   `inward` degrees towards it (0 circles; positive a spiral that
                                   comes in going clockwise on screen)
          DirectionGuide::Radial { center, reach, weight }         rays from a point
          DirectionGuide::Fixed { angle, center, reach, weight }   one direction (reach f32::INFINITY:
                                                                   everywhere)
    field.blend(&other, &weight_plane)? (the other where the weight is 1: noise where the
    structure is weak), rotated(degrees), angle_at(x, y), strength_at(x, y), stroke_pose(center,
    length) -> (angle, bend) (a stroke that curves with the field), trace(start, step, steps).
  - DETAIL MAPS: DetailMap::from_picture_edges(&picture, smoothing)? is a plain DrawingPlane of 0..1
    (how much edge there is); DetailMap::sample(&plane, destination, x, y).
  - STROKE LAYOUT: StrokeLayout::fill(&area, &field, Some((&detail, destination)), &|x, y| colour,
    &StrokeLayoutOptions { sizes, jitter: 0.33, angle_jitter: 6.0, seed: 1 })? -> one Vec of records
    per SIZE BAND, in each band's fill order. StrokeSize::new(length, width, spacing) (min_detail 0,
    variation 0): a band is placed only where the detail reaches its min_detail - small strokes crowd
    the edges, broad ones fill the flat areas; variation (0..1) scales length and width together.
    `sizes` is empty by default: give at least one. The records take their poses from the field and
    their colour from your function (the picture under each stroke's centre, through colour rules if
    you like), a seed each, and are always whole until a timeline schedules them.
    painting::element_area(&session, id, seconds)? is an element's area (its own coordinates) to fill
    one part of a figure - with that element as the strokes' anchor.
  - THE PAINTING TIMELINE: stages, each painting its strokes between two times in an order.
      let mut timeline = PaintingTimeline::new();
      timeline.add_stage(PaintingStage::new("wash", 0.0, 12.0, (0.6, 1.2)))?;     // seconds per stroke
      timeline.add_stage(PaintingStage::new("block-in", 8.0, 30.0, (0.4, 0.9))
          .with_order(StrokeOrder::Positional(Box::new(CenterOutArrivalOrder::new(false)))))?;
      timeline.add_stage(PaintingStage::new("detail", 25.0, 50.0, (0.2, 0.5)).with_order(StrokeOrder::DarkToLight))?;
      let set = timeline.schedule(vec![wash, block_in, detail], 11)?;   // one list per stage -> a set sorted by start
      timeline.add_cues(&mut cues)?;   // one cue per stage on an AnimationTimeline (a scrub bar's marks)
    StrokeOrder: AsGiven (the default), DarkToLight, LightToDark, Positional(any IArrivalOrder of the
    formations), Key(a function of the record; a NaN key drops the stroke). with_profile(..) shapes the
    pace of a stage (an ArrivalRateProfile). Each stage's strokes start between its first and last
    time in its order, each taking between the two stroke seconds to paint on; schedule takes
    `&mut self`. The painting is then a function of the time: a seek back shows exactly the earlier
    state, and a large still painting can be drawn from its checkpoints (below).

COLOUR: RULES, PERCEPTUAL MIXING AND COLOUR AS A FUNCTION OF TIME (module painting; no C# equivalent)
  - COLOUR RULES: ColorRules { palette: Option<PaintPalette>, broken: Option<BrokenColor> } (Default:
    neither). PaintPalette { pigments (1 to 32), snap (0..1), mode: PaletteMode::Nearest | MixOfTwo }
    (PaintPalette::new(pigments): snap 1, Nearest) pulls a colour, in Oklab, by `snap` towards the
    nearest pigment (Nearest posterises) or the nearest mix of two pigments (MixOfTwo keeps
    gradations); BrokenColor { hue: 12 (degrees), chroma: 0.1 (a fraction), lightness: 0 (Oklab L),
    seed: 0 } then varies each stroke's colour by up to that much either way, from the stroke's seed
    (broken colour: it vibrates from stroke to stroke without changing the overall value).
    rules.validate()?, rules.apply(color, stroke_seed) (alpha kept), rules.apply_float(..). The same
    rules run ON THE GPU for picked colours (StrokePickup::rules, below), agreeing within float
    rounding; there the cost of MixOfTwo grows with the number of pigment PAIRS - keep those
    palettes small.
  - PERCEPTUAL MIXING, AN OPT-IN: ColorInterpolation { Srgb (the default: NoSkia's straight rule),
    LinearRgb, Oklab, Oklch }; ColorMix::mix(a, b, t, space). The opt-in spaces mix premultiplied
    (CSS Color 4: a transparent end keeps the other's colour); Oklch takes the shorter way round the
    hue. Blue to yellow half way is grey in Srgb and stays colourful in Oklab.
    ColorRamp::with_interpolation(space), and LinearGradientFill / RadialGradientFill
    .with_interpolation(space) / set_interpolation(space) opt a ramp or a gradient fill in; a ramp
    and a fill made from its stops agree when both choose the SAME space. Nothing changes unless
    asked: every default is Srgb.
  - COLOUR AS A FUNCTION OF TIME: BoundTrack::color_fn binds a colour computed from the time:
      session.animations_mut().bind(BindingTarget::layer("Sky"), AnimatedProperty::Color,
          BoundTrack::color_fn(move |t| ColorMix::mix(night, day, ((t / 10.0) % 1.0) as f32, ColorInterpolation::Oklab)))?;

LIVE COLOUR SOURCES AND PICTURE TARGETS (strokes::StrokeColor, rendering::PictureTarget; no C#)
  A stroke set's colours can be PICKED EVERY FRAME on the GPU from a picture that changes - a
  painted version of a moving scene - instead of each record's own colour (StrokeColor::Own, the
  default):
      let pickup = StrokePickup { taps: 5, own_weight: 0.1, rules: Some(rules),
          ..StrokePickup::new(StrokeColorSource::Target(target.clone()), RectangleF::new(0.0, 0.0, 1280.0, 720.0)) };
      let shape = BrushStrokesShape::new(set, BrushStrokesOptions::default())?.with_tips(tips)
          .with_color(StrokeColor::Picked(pickup))?;
  StrokePickup: source; place (where the source lies in the set's coordinates, stretched over it);
  taps (1, 3 or 5 - the default 5 - points along the stroke, averaged); own_weight (0..1: 0 the picked
  colour, 1 the record's own, mixed in Oklab; 0 by default); rules (COLOUR RULES, applied to the
  picked colour before the own-colour mix; None). The taps lie on the stroke AS IT IS DRAWN THIS
  FRAME (after boil, flow and anchors), over its whole length even while it is being painted on, and
  outside `place` take the edge's colour. A stroke over a transparent part of the source paints
  nothing (unless own_weight is 1). Picking costs nothing per stroke on the CPU; on the GPU a few
  samples per stroke vertex. LIMITS: the mean of a few point samples, not the exact area mean of
  DrawingOwnershipMap::get_mean_colors; picked EVERY frame (to pick once and keep, use a still
  Picture); CPU and GPU agree within float rounding, not bit for bit. The sources:
  - StrokeColorSource::Picture(ImageSource): one of the frame's pictures (uploaded once per picture).
  - StrokeColorSource::Target(PictureTarget): another session's drawing, made THIS FRAME:
      let target = PictureTarget::new(640, 360)?;                 // 1..=8192 each side
      let picture_layer = AnimationLayer::to_picture(target.clone(), move |frame, scene| {
          scene_session.set_time(frame.info.time);
          scene_session.render_with_background(scene, frame.info.width, frame.info.height);   // the PICTURE's size
      });
      CanvasLayers::new(vec![RendererHandle::new(picture_layer), RendererHandle::new(reader_layer)])   // picture FIRST
    A to_picture layer draws NOTHING on the canvas: every frame it clears its picture to transparent
    and draws its closure's scene into it, at the picture's size (the closure's frame.info.width and
    height ARE the picture's). It is a whole second drawing with its own renderer: half the canvas
    size or less is plenty for a colour source. THE LAYER ORDER: list the picture's layer BELOW the
    layers that read it in CanvasLayers - they then read THIS frame's picture, exact under pause and
    seek; listed above them they read the previous frame's (transparent on the first), and a stroke
    pickup that does so is reported once (session.take_reports()). ImageShape::from_picture_target(
    &target, destination, ImageSampling::Linear)? shows the picture as any picture (the FLAT look
    draws it transparent and LINE leaves it out: its pixels are on the GPU only). A picture target is
    also a WATER source (below).
  - StrokeColorSource::Texture(ExternalPicture { view, size, encoding }): an APPLICATION'S texture -
    a 3D scene the Graphics package rendered (on the same device: GraphicsContext::wrap_gpu_context),
    a video frame the application copied in with wgpu's Queue::copy_external_image_to_texture (the
    crate never touches video). The view must be 2D, on the canvas host's device, with TEXTURE_BINDING
    usage and a filterable float format (8-bit unorm, sRGB or not, or Rgba16Float holding 0..1); a
    wrong view is a WebGPU validation error (the crate cannot check a view). Draw the texture BEFORE
    the stroke layer in the frame (an earlier canvas layer). Values are clamped to 0..1: TONE-MAP AN
    HDR SOURCE first (the graphics crate's ToneMapper into an 8-bit target). The ENCODING says how
    to read it: PictureEncoding::SrgbEncoded (sRGB-encoded values, straight alpha - an Rgba8Unorm
    target the ToneMapper wrote) or PictureEncoding::Linear (linear, premultiplied - what an sRGB
    texture's view decodes to, and what the canvas host's renderers write). WHEN THE PAGE ALSO SHOWS
    THAT TEXTURE ITSELF, hand over an Rgba8UnormSrgb texture with PictureEncoding::Linear: an
    Rgba8Unorm texture with SrgbEncoded is right for the strokes but is encoded twice, and looks
    washed out, when it is drawn as a picture. Such a source is only as exact under seeking as the
    application makes it (a video seeks as the application seeks it; a simulation cannot be rewound).
  The GraphicsDemo sample's "live-pickup" scene shows all three sources, a 3D scene among them.

THE SURFACE FINISH: CANVAS WEAVE AND PAINT THICKNESS, LIT (effects::SurfaceFinish; no C# equivalent)
  A layer drawn as a painted surface - the canvas's weave and the thickness of the paint, lit from
  a direction:
      layer.set_surface_finish(Some(SurfaceFinish { light_angle: 225.0, light_elevation: 35.0, relief: 0.6,
          impasto: 1.0, gloss: 0.15, weave: Some(CanvasWeave { spacing: 4.0, ..CanvasWeave::default() }) }))?;
  SurfaceFinish (Default): light_angle 225 (degrees clockwise on screen from +x, the way the light
  comes FROM: upper left), light_elevation 35 (above the canvas, up to 90), relief 0.6 (how strongly
  slopes shade), impasto 1 (0 a smooth glaze, 1 thick paint), gloss 0.15 (0..1, a small sheen on
  ridges), weave Some(CanvasWeave::default()); CanvasWeave: spacing 3 (layer units between threads),
  depth 0.35, irregularity 0.3 (0..1), angle 0, seed 0. An unusable finish is refused (the error says what to change) and the layer keeps its own;
  layer.surface_finish() reads it; set it again each frame to move the light (get_layer_mut).
  WHAT HAS THICKNESS: only the layer's STROKE SETS (each stroke's tip height x coverage x load x
  opacity x impasto, piling up where strokes overlap; a stroke without a tip is a flat plateau);
  shapes, pictures and ownership maps on a finished layer show only the weave. The weave is in the
  LAYER's coordinates (it stays put on the painting and scales with the zoom) and fades out where its
  threads would be narrower than about two device pixels; thick paint hides it. Flat areas keep their
  colour, to rounding. The finish shows only where the layer drew.
  HOW IT COMPOSES: it rides the layer's effect group - a layer with effects, frame operations, a
  transition or water takes no further nesting level; a plain layer takes one. The LINE and FLAT looks
  draw no finish (an authored layer keeps it), and effects off drops it. COST: the layer's stroke
  sets are drawn a second time each frame (into a height record of 2 bytes per canvas pixel), plus a
  few texture reads per pixel of the layer. LIMITS: the height ignores clips and masks (a stroke
  clipped by a mask still raises relief at the mask's edge); a finished layer inside a wipe inside a
  blended session uses all three nesting levels.

A STILL PAINTING FROM ITS CHECKPOINTS (strokes::StrokeCheckpoints; no C# equivalent)
  A large painting drawn whole every frame costs the GPU every stroke every frame, which tells at high
  pixel ratios. Its CHECKPOINTS are copies of the drawn picture taken at whole numbers of finished
  strokes: a frame starts from the nearest one and draws only the strokes after it.
      let shape = BrushStrokesShape::new(set, BrushStrokesOptions::default())?.with_tips(tips)
          .with_checkpoints(Some(StrokeCheckpoints::default()))?;   // spacing 2000, capacity 4
      if let Some(why) = shape.checkpoint_refusal() { /* show it: this shape can never use them */ }
  THE RULE IT KEEPS: every frame drawn with the cache is IDENTICAL, pixel for pixel on the same GPU, to
  the frame drawn with every stroke at its progress. OFF BY DEFAULT (None): each checkpoint costs 16
  bytes per canvas pixel of GPU memory (StrokeCheckpoints::bytes_at(width, height);
  BYTES_PER_PIXEL), memory the crate cannot weigh for the application, and it pays only for a large
  painting that holds still. StrokeCheckpoints { spacing: whole strokes between checkpoints, capacity:
  checkpoints kept per draw }; validate()?.
  WHEN THE CACHE IS USED - all of these, otherwise the set is simply drawn whole:
    1. the shape asked for it;
    2. nothing in the set moves with the time: no boil (a rate of 0 does not count), no flow, no
       anchor that follows an element (fixed anchors are fine), colours its own or picked from a still
       Picture (not a picture target or an application's texture), and not a stroke-level transition
       (checkpoint_refusal() says which; such a shape is reported once and counted in
       checkpoint_misses);
    3. the frame's KEY is the one the checkpoint was made under: the set, the tips, the colour source,
       the draw's settings apart from the time (where it is on screen, its opacity ...), the canvas
       size AND EVERYTHING DRAWN BELOW THE SET in the frame. So it is not used while the painting moves
       or scales on screen (a moving camera, a bound Position), after a resize, while anything below
       it changes, or - without a background - while the area the frame draws changes. Things drawn
       ABOVE the set do not matter: draw what changes (text, read-outs) in a session above the
       painting, use a background, and build the shape once per change of its settings;
    4. there are whole strokes to restore: a checkpoint holds the first strokes in paint order, all
       of them ENDED at the frame's time.
  Playing forward, each frame draws the strokes painted since the last checkpoint and those being
  painted; seeking back restores the nearest earlier checkpoint held; with none at or below the time
  the frame is a full redraw, which makes one there (checkpoints are made only while the key stays
  the same from one frame to the next). Checkpoints are kept by the renderer's sweep with a budget of
  their own (and `capacity` per draw); a resize drops them all. LIMITS: a finished layer's height
  (SURFACE FINISH) is still drawn from the whole set every frame; a seek further back than the
  checkpoints held is a full redraw. RenderStats: checkpoint_hits, checkpoint_bakes,
  checkpoint_misses, strokes_restored (not counted in `strokes`), checkpoints, checkpoint_bytes.

STROKE-LEVEL TRANSITIONS (judged by eye; no C# equivalent)
  One painting turning into another stroke by stroke - strokes sliding, turning and changing colour
  into their place in the second painting - instead of a dissolve:
      let morph = BrushStrokeMorph::pair(&noon, &dusk, StrokePairingOptions { search_radius: 40.0, color_weight: 0.5, seed: 3 })?;
      let spread = Box::new(DirectionalArrivalOrder::new(0.0, 0.0)?);   // a left-to-right wave
      let shape = BrushStrokesShape::morph(morph, BrushStrokesOptions::default(), Some((spread, 0.4)))?.with_tips(tips);
      let id = session.draw_brush_strokes(shape);
      session.animations_mut().bind(id, AnimatedProperty::MorphFraction, track)?;   // 0..1, scrubbable
  PAIRING (once, greedy and deterministic for a seed): each stroke of the first set takes the free
  stroke of the second within search_radius (set units; default 40) that costs least - distance, plus
  color_weight (default 0.5) times the colour difference in Oklab; the rest are left without a
  partner. morph.pairs() / len() / paired_count() / record_at(slot, fraction). AT A FRACTION (bind
  MorphFraction, or set_morph_fraction / with_morph_fraction; clamped 0..1): at 0 exactly the first
  painting, at 1 exactly the second; between, a pair slides, turns the shorter way (never more than 90
  degrees, a stroke being the same both ways round) and mixes its colour in Oklab; a stroke without a
  partner PAINTS ITSELF OFF, or ON. The SPREAD (an arrival order and an amount 0..<1) sends the change
  across the painting as a wave. Tips, anchors and colours work as on any set; checkpoints are
  refused. One draw; per frame only the fraction is sent. LIMITS: greedy, not optimal pairing;
  strokes may cross on long journeys; two very different paintings look like a shuffle; very
  different stroke counts paint many strokes off or on; a tip that is not symmetric along the stroke
  shows the switch at a pair's halfway point; the overlap order switches at fraction 0.5. Whether it
  reads as painting is for the eye to judge.

WATER: THE RIPPLE FIELD, CAUSTICS, REFRACTION AND REFLECTION (module water; no C# equivalent)
  THE RIPPLE FIELD is water as a FUNCTION OF THE TIME: rings from drops and seeded rain spreading and
  fading over a swell, inside a rectangle whose edges can reflect them. Nothing is remembered between
  frames: a time shows the same water whether it is played to, sought to or drawn after other times,
  and pausing freezes it.
      let field = RippleField::new(RectangleF::new(0.0, 0.0, 1280.0, 720.0), RippleOptions { walls: true, ..RippleOptions::default() })?
          .with_rain(RippleRain::new(2.0, 1))?                    // 2 drops a second, seed 1
          .with_swell(RippleSwell::gentle(4, 160.0, 0.6))?;       // four seeded waves
      // a click: a new field value with one more drop at the session's current time
      field = field.clone().with_drop(RippleDrop::new(PointF::new(x, y), session_time))?;
  - RippleOptions { speed: 90, wavelength: 16, rings: 3, decay: 1.8, height: 1, walls: false }.
    RippleDrop { center, time, strength: 1, size: 1 } (RippleDrop::new(center, time)). with_drops ADDS
    drops, with_drop adds one, with_rain and with_swell REPLACE; a new drop is a new field value (the
    old one is unchanged; clones share the drop list, so it is cheap) - scrubbing back before a
    click's time removes its ring and forward brings it back exactly.
  - THE RING of a drop of age a: its front is at speed x a (field.ring_radius(&drop, t)); the
    strongest feature is a TROUGH a quarter wavelength inside the front, a small leading CREST about
    0.15 wavelength outside it, and about `rings` crests behind it, fading away over
    field.drop_life() seconds (decay x ln 100); at the instant a drop lands it has no height yet.
  - RAIN: RippleRain { rate: 2 (drops a second, 0 to RippleRain::MAX_RATE = 200), start: 0, end: None,
    strength: (0.4, 1.0), size: (0.7, 1.3), area: None (the field's place), seed: 0 }
    (RippleRain::new(rate, seed)). Each second of the session (k <= t < k + 1) holds its own seeded
    drops, so rain is exact under seeking; start and end leave out drops outside them without moving
    the others; two fields with the same rain seed rain alike.
  - SWELL: RippleSwell { waves } (at most RippleSwell::MAX_WAVES, 8), SwellWave { angle: 0 (degrees
    clockwise from +x, the way it travels), wavelength: 160, height: 0.5, speed: 40, phase: 0 };
    RippleSwell::gentle(seed, wavelength, height) makes four waves that together never exceed the
    height.
  - WALLS: each drop also sounds from its mirror images across the place's edges - the first bounce
    of a rectangular pool, exactly (no second bounce).
  - THE CAP: at most RippleField::MAX_LIVE_DROPS (256) drops count at a time; beyond it the weakest at
    that time are left out (on the CPU too, so both agree), counted in RenderStats::ripples_skipped and
    reported once (session.take_reports()).
  - READ ON THE CPU (to float a boat, place a leaf): field.height_at(x, y, t), slope_at(x, y, t),
    focus_at(x, y, t) (the curvature - negative on crests, where light gathers), live_drops(t). Each
    call works out the live drops again (microseconds); positions are single precision, so keep them
    within a few thousand units of 0.
  CAUSTICS AS A NOISE KIND: DrawingNoiseKind::Caustics (PIXEL PLANES above) draws a pool floor's light
  net with a NoiseShape (z = the time times a speed: NoiseShapeOptions { fill: DrawingNoiseFillOptions
  { z: Some(t * 0.3), octaves: 2, lacunarity: 1.7, gain: 0.6, .. }, stops }), the same pattern on the
  CPU; a refraction's CausticLight (below) lights the floor with it.
  THE WATER SOURCE a refraction or a reflection is seen through (WaterSource):
    Ripples(field)                          the ripple field, evaluated per pixel on the GPU (the same
                                            arithmetic as height_at / slope_at): exact in time
    Picture { picture, place, height }      heights from an ImageSource's RED byte (128 still water,
                                            0 and 255 = -height and +height), stretched over place
    Target { picture, place, height }       the same from a PictureTarget drawn THIS FRAME (its layer
                                            BELOW the reader; draw it opaque); exact in time when
                                            it is drawn from the session's time
    Texture { picture, place, height }      an application's texture (an ExternalPicture; its RED
                                            channel as stored times height, 0 still): the Graphics
                                            package's simulated wave field, for example -
                                            ExternalPicture { view: field.surface().view().clone(),
                                            size: (columns, rows), encoding: PictureEncoding::Linear }
                                            (the encoding is not used for water), from a graphics
                                            layer BELOW the animation layer that calls field.update
  A SCENE WHOSE WATER IS A TEXTURE FED BY A SIMULATION CANNOT BE REWOUND: pausing or seeking the
  clock does not undo what the simulation went through (WaterSource::is_exact_in_time() is false
  only for Texture). THE WATER'S COORDINATES are the layer's DRAWING space - calibrated units under
  the camera at the layer's parallax; the layer's own transform does not move the water - so one
  field can serve one layer's refraction and another layer's reflection and both line up (a click's
  calibrated position is where its drop lands). source.validate()? checks it.
  REFRACTION - a layer (or a whole session) seen THROUGH the water:
      session.get_layer_mut("Floor").unwrap().set_refraction(Some(WaterRefraction {
          glints: Some(WaterGlints::default()), caustics: Some(CausticLight::default()),
          ..WaterRefraction::new(WaterSource::Ripples(field.clone())) }))?;
      // set it again whenever the field changes (a click): it holds the field by value
  WaterRefraction: source; strength 12 (water units the picture moves per unit of slope); shading 0.3;
  light_angle 225 and light_elevation 45 (where the light comes from, as for the surface finish);
  glints None (WaterGlints { amount: 0.8, sharpness: 80, color: (255, 245, 225) }: small sharp
  highlights on slopes facing the light); caustics None (CausticLight { strength: 0.5, frequency:
  0.02, speed: 0.25, focus: 0.5, seed: 0, color: (235, 248, 255) }: the caustic net, gathered under
  crests, LIGHTING the content's colour - black stays black). Each pixel of the layer reads its
  picture at a point moved by the slope (at most 128 device pixels), shaded by the slope towards the
  light; glints and caustics only where the layer drew. Strength 0 with no shading, glints or caustics
  draws exactly the layer's effect group without water. SessionRenderOptions::refraction does the
  same for a whole session's picture (its background included when drawn), in calibrated units
  under the camera.
  REFLECTION - a layer MIRRORED below a waterline:
      bank.set_reflection(Some(WaterReflection { streak: 3.0, water: Some(WaterSource::Ripples(field.clone())),
          ..WaterReflection::new(360.0) }))?;
  WaterReflection: line (the waterline's y in the layer's drawing space); gap 0 (the mirror sits this
  much lower); opacity 0.6 (at the line); fade_distance 300; tint (40, 70, 90, 90) multiplied in, its
  alpha saying how much; streak 3 (a vertical smear); water None (a still mirror) and water_strength
  10. Only below the line: the layer's CONTENT (not its glow or shadow) mirrored, fading with depth,
  tinted, smeared and broken up by the water, put UNDER the content. A camera roll tilts the line.
  When the reflection's water equals the refraction's source it is evaluated once.
  HOW IT COMPOSES: refraction and reflection ride the layer's ONE EFFECT GROUP - a layer with effects,
  frame operations, a transition or a surface finish takes no further nesting level, a plain layer
  takes one; refraction + reflection + a blur + a finish are still one group. A session's refraction
  is one group around its picture (inside the blend's group when the session is blended or faded).
  Water STAYS under the LINE and FLAT looks and with effects off. A checkpointed painting over water
  keeps its cache over still water, a ripple field or a still picture; water from a picture target
  or an application's texture makes the checkpoints above it unusable (they change every frame).
  LIMITS: a screen-space bend of the layer's own picture (nothing outside the layer is seen through
  it), not a lens; layers and sessions only (a group to bend goes on a layer of its own); a flat
  mirror of the layer's 2D picture (no parallax, no other layers); rings do not meet shapes; at most
  256 live drops per field; the offset is at most 128 device pixels.
  COST: what the water itself adds grows with the live drops every pixel of the group evaluates (walls
  evaluate each drop nine times) and with the pixels covered: over a full canvas a few dozen live
  drops are comfortable (a rain of a couple of drops a second keeps a dozen or two alive), and the cap
  of 256 with walls is the heavy end. The effect group it rides has a cost of its own, the same as any
  full-canvas effect (THE COST OF EFFECTS AND MASKS below), which grows with the canvas's device
  pixels: at high pixel ratios that group, not the water, is most of the cost - keep refracted layers
  to the part of the canvas that is water. RenderStats: ripples_evaluated (drops sent, per group that
  draws a field), ripples_skipped.

THE COST OF EFFECTS AND MASKS (one frame budget for the 3D and 2D layers)
  - Each element, group or layer WITH EFFECTS is drawn into an offscreen target, then: one resolve
    pass, then per blur (content blur, glow, soft shadow - each only if present) two passes (one per
    direction) plus one box-filter pass when the blur is wide, then the composite - so 3 to 10
    offscreen passes, each limited to the content's box grown by the effects' reach. A wide blur is
    computed at a coarser resolution (a pixel of 2, 4 ... 16 canvas pixels) so that no pass reads more
    than 17 texels per pixel; its cost then hardly grows with the radius. The fixed part - the
    offscreen target, the resolve and the composite at full resolution - is what a full-canvas
    effect costs most; small elements cost in proportion to their box. Measured costs of a
    full-canvas blur are in the repository's MAINTAINER-README (THE RENDERER'S EFFECT PASSES).
  - A mask "inside a path" is a stencil clip (nearly free); "outside a path" and a layer matte draw
    the content offscreen and composite the mask over it (one offscreen group each).
  - Effects nest at most three offscreen levels deep (with groups and masks); deeper ones are drawn
    without their effects and counted in RenderStats::unsupported. What takes a level: a blended or
    faded session (render options) one; a soft wipe one for its second render (shared with that
    session's blend in render_sessions); a stroke set below full opacity one; a layer's surface
    finish, refraction and reflection none of their own - they ride the layer's effect group (one
    level for a plain layer). So a finished, refracted layer of a blended session inside a wipe uses
    all three, and any further group inside it is drawn as a clip and counted.
  - THE CANVAS SIZE MULTIPLIES THE COST OF OFFSCREEN WORK. On a high-density screen (device pixel
    ratio 2: a 1280 x 720 canvas is 2560 x 1440 pixels) a frame with tens of offscreen groups - mix
    modes, masks, blurs, Plus-composited groups - can take 30 to 50 ms on an integrated GPU, where the
    same frame holds 60 fps at ratio 1 (the GraphicsDemo sample's MAINTAINER-README.txt names its
    scenes that do). Keep such groups few, each with the smallest shape that holds its content.

THE LOW-LEVEL DOOR: VectorScene
  What a session records itself into, and what any content the session API does not cover can draw
  with directly (peniko brushes, kurbo shapes): fill (non-zero or even-odd), stroke (kurbo's stroke: width,
  joins, caps, miter limit, dashes), push_layer (a group with a mix and compose mode and an alpha,
  inside a shape), push_clip_layer / push_clip_layer_with_rule, pop_layer, draw_image. Brushes: solid
  colours, linear / radial (two-point) / sweep gradients with pad, repeat and reflect, images (RGBA or
  BGRA, straight or premultiplied, nearest or linear sampling, pad / repeat / reflect per axis).

WHAT THE RENDERER DOES AND COSTS (so an application can plan)
  - Paths are filled by stencil-then-cover with 4x MSAA (five levels of edge coverage). Strokes are
    expanded by kurbo and filled; a stroke thinner than one device pixel is drawn one pixel wide at a
    proportionally lower alpha.
  - Flattened and stroked outlines are CACHED in the shape's own space: a shape that only moves,
    turns or scales a little costs no re-flattening; a shape whose geometry changes every frame (a
    morph, a trim, a boiling sketch) is re-flattened every frame, and so is a FIXED-WIDTH stroke whose
    element moves (it is stroked in the drawing's space).
  - Stroke sets are ONE instanced draw each, with their records uploaded once; brush tips and
    pictures are uploaded once. Everything uploaded is swept every frame by one rule with a byte
    budget (PICTURES AND SPRITES).
  - Shapes that do not overlap are drawn in one batch.
  - A group (push_layer) with normal blending and full alpha costs nothing (it is only its clip); a
    group of one draw puts its alpha into that draw; other groups are drawn offscreen and composited.
    A group with a MIX mode (Multiply, Screen, ... Luminosity) copies the canvas-sized target below it
    first: these are the expensive ones - use them on a few groups, not hundreds.
  - Mix modes blend sRGB-encoded values (as the W3C formulas and NoSkia do); normal blending happens
    in linear light (the canvas host's convention). Translucent overlaps therefore look a little
    lighter than NoSkia's.

LOTTIE ANIMATIONS (module lottie)
  NoSkia has no Lottie support, so there is no C# column here: the API uses the crate's own
  vocabulary and the same shape as a picture or a sprite - a document read from JSON, an ELEMENT of
  a session that plays it AT THE SESSION'S TIME, drawn by the crate's renderer. velato (Lottie
  JSON into a composition, evaluated for a frame) reads the document; the crate's own pass over the
  JSON runs first; the crate's renderer draws the result.

  A Lottie animation is read from its JSON TEXT or BYTES - from anywhere: the crate reads no file
  and fetches nothing (the sample's backend serves the JSON as a [WireFile] answer and the frontend
  fetches it with the generated `<name>_bytes` call). DOTLOTTIE (.lottie, a zip archive) IS NOT
  READ: unpack it on the backend and pass the animation JSON inside it (and its images as decoded
  pixels); DrawingLottie::load refuses a zip with an error that says so.

      use std::sync::Arc;
      use codebrix_webgpu_animation::lottie::{DrawingLottie, DrawingLottieAspect, DrawingLottieOptions,
          LottiePlayOptions, LottiePlayback, LottieRange};

      let lottie = Arc::new(DrawingLottie::load(&json_bytes, &DrawingLottieOptions::default())?);
      for line in lottie.report() { log(&line); }       // what this file uses that is not drawn as written
      let id = session.draw_lottie(lottie.clone(), RectangleF::new(40.0, 120.0, 400.0, 300.0),
          DrawingLottieAspect::Meet, LottiePlayOptions::default())?;   // loops from session time 0
      // every frame, as for any session: session.set_time(frame.info.time); session.render(scene, w, h);

  THE ELEMENT. session.draw_lottie(lottie, destination, aspect, options) puts a DrawingLottieShape
  on the active layer and returns its ElementId (DrawingShape::from(shape) puts one on any layer or
  in a group). It takes the element properties every element has - transform, opacity (multiplied
  into every draw, as Lottie's own opacities are), time window, effects, masks of its group or
  layer, the bindings of Opacity / Position / Rotation / Scale / Anchor. element.as_lottie() /
  as_lottie_mut() reach the shape: set_play_options, set_image, frame_at, marker_times.
  shape.get_bounds() is the fitted box (Slice: the destination).

  PLAYBACK IS A FUNCTION OF THE SESSION TIME. LottiePlayOptions:
      playback      LottiePlayback::Loop (default) | PingPong | Hold
      speed         1 by default; negative plays backwards; 0 holds start_offset
      range         LottieRange::Whole (default) | Frames { start, end } | Marker("name")
      begin_time    the session time at which playback begins (before it: the frame at start_offset)
      start_offset  where in the range playback begins, in seconds of the animation
      whole_frames  round down to whole frames (default: every time has its own in-between frame)
  The playhead at session time t is (t - begin_time) * speed + start_offset seconds into the range,
  then looped, ping-ponged or held. Hold (and the turn of a ping-pong) shows the range's last frame
  just inside its end: Lottie's out point is exclusive. Pause, speed and seek for the WHOLE page are
  the host clock's; to seek or restart ONE animation give it new options from the session time you
  are at (shape.set_play_options(LottiePlayOptions { begin_time: now, start_offset: 1.5, ..options })).
  A Hold animation that has finished (shape.is_finished(t)) draws the same list every frame.

  MARKERS ARE NAMED POINTS IN TIME. A marker (Lottie "markers": cm name, tm frame, dr duration) is a
  LottieMarker (lottie.markers(), lottie.marker(name)). LottieRange::Marker("wave") plays one
  marker's span (a marker without a duration plays to the end). To let other layers follow the
  animation's markers by name, define them on the host's clock:
      for (name, t) in shape.marker_times() { clock.define_point(&name, t); }
  marker_times() gives the session time at which each marker inside the range is first reached,
  playing forwards from begin_time; frame.points.reached("wave", t), since(..), crossed(..) then work
  as for every named point.

  IMAGE LAYERS take pictures from the application, per asset id: lottie.images() lists each
  asset (id, authored size, location = directory + file name or the data: URL, is_data_url); decode
  them on the backend (the picture-pixels form) or read a PNG or JPEG file's bytes with
  ImageSource::from_png / from_jpeg (see PICTURES AND SPRITES), and hand the ImageSource over with
      session.find_element_mut(id).and_then(|e| e.as_lottie_mut()).unwrap().set_image("image_0", source);
  They are placed as velato's own renderer places them (fitted, centred, into the authored box).
  Without pixels an image layer draws nothing (reported as ImageNeedsPixels).

  FITTING. DrawingLottieAspect: Meet (fit inside, centred - default), Slice (cover, centred, clipped
  to the destination), Stretch, None (one document unit per calibrated unit at the destination's
  corner). LOAD OPTIONS: DrawingLottieOptions { max_repeater_copies: 1000, draw_solid_layers: true }.

  THE DRAW LIST (for an application that draws Lottie content with its own code): shape.draw_list(t)
  gives a LottieDrawList of plain-data commands in drawing order, each with its final transform -
  PushGroup { alpha, blend, area, .. }, PushClip { path, .. }, Fill { path, paint, opacity, .. },
  Stroke { path, stroke, .. }, Image { asset_id, .. }, Pop - the commands the element maps one to one
  onto the renderer (groups to push_layer with their blend mode - mix modes, Plus for "add",
  SrcIn / SrcOut for track mattes; clips to push_clip_layer; fills non-zero; strokes; images to
  pictures). Groups that hold ONE draw are folded into it (shape.fold_groups, default true).

  VELATO'S PANICS AND REFUSALS, AND THE CRATE'S PASS. velato is used exactly as published. Read in
  its source: its importer PANICS on ordinary Lottie content (a layer transform without "r", 3D or
  split rotations, a split position in a shape group's transform, the blend modes add and hard
  mix, per-dimension easing handles on scalar, shape or gradient keyframes, a scalar keyframe with
  an empty value), and its reader REFUSES A WHOLE FILE for one text layer, any other unknown layer
  type, one unknown shape item, or a whole number written as a float in "ty", "ind", "w" .... In
  WebAssembly a panic aborts the page. So THE CRATE'S OWN PASS over the JSON runs BEFORE velato
  reads a document and turns every such case into what velato reads (or removes it), and reports
  it; a host test runs velato alone on three such documents and sees it panic. The pass also fixes
  what has an exact answer: hidden shape items removed (velato would draw them), masks of mode
  "none" removed (velato would clip by them), solid-colour layers drawn as rectangles (velato draws
  nothing for them), old-format keyframes completed; and the element applies TOP-LEVEL layer blend
  modes, which velato stores and never applies.

  WHAT IS DRAWN AND WHAT IS NOT (exact)
  SUPPORTED - drawn as written:
    Document     w, h, fr, ip, op; precomposition assets; image assets; markers.
    Layers       shape (ty 4); precomposition (ty 0) with its start time (st) and time stretch (sr);
                 null (ty 3) for parenting; image (ty 2) with the application's pixels; solid colour
                 (ty 1) as a rectangle of its size and colour; in and out points (ip / op);
                 parenting chains (parent); hidden layers (hd) not drawn.
    Transforms   anchor, position (also split X / Y on layers), scale, rotation, skew and skew axis,
                 opacity - fixed or keyframed - on layers and shape groups.
    Keyframes    linear, bezier easing (i / o), hold (h); sub-frame interpolation; old-format
                 keyframes whose last value is the previous keyframe's "e"; whole numbers written as
                 floats.
    Shapes       group (gr) with its transform; rectangle (rc) with corner radius; ellipse (el); star
                 and polygon (sr) with roundness; path (sh) including animated morphs; hidden items
                 (hd) removed.
    Paint        fill (fl) colour and opacity; stroke (st) colour, opacity, width, caps, joins, miter
                 limit (ml); gradient fill (gf) and gradient stroke (gs), linear and radial, animated
                 stops with stop opacity, animated start and end points.
    Modifiers    trim paths (tm: start, end, offset - "simultaneously"); repeater (rp: copies, offset,
                 transform, start and end opacity, composite above / below).
    Masks        one mask of mode add or intersect, further masks of mode intersect (nested clips);
                 mask paths animated; masks of mode none removed.
    Mattes       alpha (tt 1) and inverted alpha (tt 2) track mattes, the matte layer found by tp or
                 as the layer before.
    Blend modes  TOP-LEVEL layers' bm 1 to 16 (multiply ... luminosity, add).
  APPROXIMATED - drawn, not as written (each reported by report()):
    luma and inverted luma mattes (tt 3 / 4): by the matte's alpha ........ LumaMatteApproximated
    mask modes subtract, lighten, darken, difference, add after the first mask, inverted masks, mask
    opacity, mask expansion: every mask clips to the inside of its path, several to their
    intersection .......................................................... MaskApproximated
    even-odd fill rule (r 2): filled non-zero ............................. EvenOddFillApproximated
    trim paths "individually" (m 2): each path trimmed on its own ......... TrimIndividuallyApproximated
    curved motion paths (position keyframes' ti / to): straight lines ..... MotionPathApproximated
    an animated split position on a shape group's transform: the first keyframe
                                                                            SplitPositionApproximated
    per-dimension easing (different curves per dimension): the first curve EasingApproximated
    3D layers (ddd) and 3D rotation: drawn flat, Z rotation only .......... ThreeDimensionalIgnored
    repeaters with more copies than max_repeater_copies: limited .......... RepeaterCopiesLimited
  NOT DRAWN - ignored (each reported by report()):
    text layers (ty 5) - kept as null layers so that children still move .. TextLayerNotDrawn
    audio, video, image sequence, image and video placeholder, guide, adjustment, camera, light and
    data layers (ty 6 to 15) - null layers when they have a transform ..... LayerTypeNotDrawn
    image layers without pixels from the application ...................... ImageNeedsPixels
    blend modes of layers inside precompositions; hard mix (17) ........... BlendModeIgnored
    stroke dashes (d): drawn solid ........................................ StrokeDashesIgnored
    a gradient's own opacity (o on gf / gs): full opacity ................. GradientOpacityIgnored
    a radial gradient's highlight (h / a): centred ........................ GradientHighlightIgnored
    pucker/bloat, merge paths, offset path, twist, rounded corners, zig zag ShapeModifierIgnored
    shape items of unknown types: removed ................................. UnknownShapeRemoved
    expressions (x): keyframes or values used ............................. ExpressionIgnored
    layer effects (ef) .................................................... EffectIgnored
    layer styles (sy) ..................................................... LayerStyleIgnored
    time remapping (tm) of precomposition layers .......................... TimeRemapIgnored
    auto-orient (ao) ...................................................... AutoOrientIgnored
    motion blur (mb) ...................................................... MotionBlurIgnored
  IGNORED AND NOT REPORTED: precomposition layers are not clipped to their w / h; path direction (d on
    shapes); an animated miter limit (ml2) - the fixed ml is used; fonts, chars, slots (sid),
    metadata, the web-only names (cl, ln, tg).
  NOT READ AT ALL: dotLottie archives (.lottie) - unpack them on the backend.

  COST. A document is read once (the JSON into velato's composition; tens of milliseconds for a file
  with hundreds of layers - do it when the page loads, not in a frame). Each frame evaluates the
  document again and hands the renderer fresh paths: an animated path is re-flattened every frame,
  a still one is found in the renderer's geometry cache.

WHAT HAS NO EQUIVALENT (and what to do instead)
  - The freehand-stroke and pointer members of DrawingSession (PointerPressed, UndoLastStroke, the
    active stroke): not part of this package (interactive drawing tools are out of its scope).
  - ExportPng / ExportJpeg / ExportImage / FrameSequenceRenderer: none - the package draws in real time
    for the browser window only.
  - ExportPixels: none for applications. (The crate keeps an asynchronous readback for its own tests;
    it is not an application feature.)
  - FontRegistry.RegisterFont(path) and (stream): none - register the bytes (the crate reads no
    files); no system font is ever used, and there is no fallback font.
  - Mixed-direction text (bidirectional reordering): not supported - a line is shaped in the
    direction of its first strong character.
  - Element references: C# binds and finds the element object; Rust uses its ElementId (see THE
    ANIMATION MODEL).
  - ImageSource.FromFile, and FromEncoded for formats other than PNG and JPEG: none - decode on the
    backend (CodeBrix.Imaging) and send the picture-pixels form (PicturePixelsWriter), or hand the
    crate the PNG or JPEG file's bytes (see PICTURES AND SPRITES).
  - Noise DRAWN on the GPU (NoiseShape) has the same parameters and character as NoSkia's but not the
    same pattern for a seed; noise VALUES and DrawingRandom on the CPU are exact. (The Caustics kind,
    which C# does not have, is the same pattern on both.)
  - Whole-frame plane operations at video sizes on the CPU: they work, exactly, but too slowly for
    every frame on one thread - use the GPU versions (DrawingFrameOperations, set_layer_transition,
    NoiseShape, ParticlesShape, OwnershipShape).
  - The painting API's interactive capture (pointer strokes, UndoLastStroke, the freehand Stroke
    model) and render-ahead on a worker thread: none (see PAINTERLY PLAYBACK).
  - CaptionTrack.Load(path): none - the crate reads no files; fetch the caption text (from the
    backend, or carry it in the frontend) and parse it.
  - PathTrain.AddTo(layer): none - the elements of a train are on a layer before binding (and
    bind_text_along_path adds its pieces to the layer it names).
  - Lottie (no C# counterpart): see LOTTIE ANIMATIONS; dotLottie archives are not read - unpack them
    on the backend.

  THE LAYERING RULE: the Animation and Graphics crates never depend on each other. Only the
  APPLICATION puts an Animation layer over (or under) a Graphics layer, by listing both renderers
  in one CanvasLayers on one canvas with one clock; each draws into the frame it is handed and
  neither clears it.

  The layering rule holds for pictures too: a 3D scene of the Graphics package reaches the animation
  crate as a plain wgpu texture view (ExternalPicture: a colour source for stroke sets, or the water
  of a refraction), never as a type of the other crate.

  WHERE ITS DATA COMES FROM: the crate takes values and bytes that the application hands it (font
  files, decoded picture pixels, PNG and JPEG files, caption text, SVG documents, Lottie JSON,
  textures it made) and never fetches or reads anything itself. It ships no artwork and no brush
  styles: pictures, brush tips and palettes are the application's own.




NOTICES FOR A BUILT SITE
========================

The crates this package brings are compiled into the application's
WebAssembly file, so their notices belong with the site. The platform's
notices (PlatformCrates.ReadNotices) do NOT cover them. The package hands
them to the build program:

    string webGpuNotices = WebGpuCrates.ReadNotices();
    string graphicsNotices = GraphicsCrates.ReadNotices();   // with Graphics
    string animationNotices = AnimationCrates.ReadNotices(); // with Animation

Serve them with the site after the application's own notices and the
platform's, typically in the one THIRD-PARTY-NOTICES.txt beside the built
site. The file is BUILD OUTPUT: write it into the backend's frontend-built/
folder with the platform's WriteBuiltFile (it gives the file its compressed
copies) AFTER EVERY BUILD, with the build's configuration - never into
wwwroot/ (see NOTICES FOR A BUILT SITE in the platform's AGENT-README):

    frontend.WriteBuiltFile("backend/MyApp.Api", "THIRD-PARTY-NOTICES.txt",
        File.ReadAllText("THIRD-PARTY-NOTICES.txt") + "\n\n\n"
            + PlatformCrates.ReadNotices() + "\n\n\n"
            + WebGpuCrates.ReadNotices() + "\n\n\n"
            + GraphicsCrates.ReadNotices() + "\n\n\n"     // with Graphics
            + AnimationCrates.ReadNotices(),                // with Animation
        FrontendConfiguration.Release);


COMPLETE EXAMPLES
=================

1. A build program that registers the WebGpu crates, materializes, builds
   and writes the notices:

    using System;
    using System.IO;
    using System.Threading.Tasks;
    using CodeBrix.WebPlatform.FrontendBuilder;
    using CodeBrix.WebPlatform.FrontendBuilder.Crates;
    using CodeBrix.WebPlatform.WebGpu;

    public static class Program
    {
        public static async Task<int> Main()
        {
            WebGpuCrates.Register();

            var frontend = new PlatformFrontend(new PlatformFrontendOptions
            {
                FrontendDirectory = "frontend",
                BackendProjectPath = "backend/MyApp.Api/MyApp.Api.csproj",
            });

            var report = await frontend.CheckPrerequisitesAsync();
            if (!report.IsReady)
            {
                return 1;
            }

            var materialized = await frontend.MaterializeAsync();
            foreach (var bundle in materialized.CrateBundles)
            {
                Console.WriteLine(bundle.Origin + ": " + bundle.AddedCrates.Count + " crates added");
            }

            // the site, into the backend project's frontend-built/ folder
            await frontend.BuildAsync(FrontendConfiguration.Release, "backend/MyApp.Api");
            frontend.WriteBuiltFile("backend/MyApp.Api", "THIRD-PARTY-NOTICES.txt",
                File.ReadAllText("THIRD-PARTY-NOTICES.txt") + "\n\n\n"
                    + PlatformCrates.ReadNotices() + "\n\n\n"
                    + WebGpuCrates.ReadNotices(),
                FrontendConfiguration.Release);
            return 0;
        }
    }

2. The frontend: a canvas with two INDEPENDENT layers on one clock - an
   opaque scene, and a translucent overlay that appears at a named point -
   with a pause button. (`Scene` is written like `Overlay`, opaque, with
   BlendState::REPLACE.)

    [dependencies]
    codebrix-webplatform = { path = "platform/codebrix-webplatform" }
    dioxus = { version = "...", default-features = false,
               features = ["minimal", "web", "mounted"] }
    codebrix-webgpu = "..."

    use codebrix_webgpu::{wgpu, CanvasClock, CanvasFrame, CanvasLayers,
        CanvasRenderer, CanvasTarget, GpuContext, RendererHandle,
        WebGpuCanvas, WebGpuUnavailable};
    use dioxus::prelude::*;

    struct Overlay { pipeline: Option<wgpu::RenderPipeline> }

    impl CanvasRenderer for Overlay {
        fn prepare(&mut self, context: &GpuContext, target: &CanvasTarget) {
            let device = context.device();
            let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("overlay"),
                // no bindings; vs_main makes a quad of 6 vertices, fs_main
                // returns LINEAR colour PREMULTIPLIED by its alpha
                source: wgpu::ShaderSource::Wgsl(include_str!("overlay.wgsl").into()),
            });
            self.pipeline = Some(device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("overlay"),
                layout: None,
                vertex: wgpu::VertexState { module: &shader, entry_point: Some("vs_main"),
                    compilation_options: Default::default(), buffers: &[] },
                primitive: Default::default(),
                depth_stencil: None,
                multisample: Default::default(),
                fragment: Some(wgpu::FragmentState { module: &shader, entry_point: Some("fs_main"),
                    compilation_options: Default::default(),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: target.format,
                        blend: Some(wgpu::BlendState::PREMULTIPLIED_ALPHA_BLENDING),
                        write_mask: wgpu::ColorWrites::ALL,
                    })] }),
                multiview_mask: None,
                cache: None,
            }));
        }

        fn render(&mut self, frame: &mut CanvasFrame<'_>) {
            let (Some(pipeline), true) = (self.pipeline.as_ref(), frame.point_reached("title in")) else {
                return;
            };
            let mut pass = frame.encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("overlay"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: frame.view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations { load: wgpu::LoadOp::Load, store: wgpu::StoreOp::Store },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass.set_pipeline(pipeline);
            pass.draw(0..6, 0..1);
        }
    }

    #[component]
    fn Page() -> Element {
        let gpu = use_resource(GpuContext::request);
        let state = gpu.read();
        match &*state {
            None => rsx! { p { "Asking the browser for a WebGPU adapter..." } },
            Some(Err(reason)) => rsx! { WebGpuUnavailable { reason: reason.clone(), class: "notice" } },
            Some(Ok(context)) => rsx! { Stage { context: context.clone() } },
        }
    }

    #[component]
    fn Stage(context: GpuContext) -> Element {
        let clock = use_hook(|| {
            let clock = CanvasClock::new();
            clock.define_point("title in", 4.0);
            clock
        });
        let layers = use_hook(|| CanvasLayers::new(vec![
            RendererHandle::new(Scene::new()),               // bottom
            RendererHandle::new(Overlay { pipeline: None }), // over it
        ]));
        let pause = {
            let clock = clock.clone();
            move |_| { clock.toggle(); }
        };
        rsx! {
            WebGpuCanvas { context, layers, clock: clock.clone(),
                           class: "w-full aspect-video touch-none",
                           label: "The scene, with titles over it" }
            button { onclick: pause, "Pause or resume" }
        }
    }


MINIMUM VIABLE PROJECT TEMPLATE
===============================

The build program (shown for a Linux x64 machine; see the platform's
AGENT-README.txt for the host-tools package of other machines):

    <Project Sdk="Microsoft.NET.Sdk">
      <PropertyGroup>
        <OutputType>Exe</OutputType>
        <TargetFramework>net10.0</TargetFramework>
        <RollForward>Major</RollForward>
      </PropertyGroup>
      <ItemGroup>
        <PackageReference Include="CodeBrix.WebPlatform.FrontendBuilder.ApacheLicenseForever" Version="*" />
        <PackageReference Include="CodeBrix.WebPlatform.FrontendBuilder.LinuxX64.ApacheLicenseForever" Version="*" />
        <PackageReference Include="CodeBrix.WebPlatform.WebGpu.ApacheLicenseForever" Version="*" />
      </ItemGroup>
    </Project>

(Replace each `*` with a version: the FrontendBuilder packages at the platform
version the WebGpu package pins Core to.) Program.cs is example 1 above. The
frontend's Cargo.toml names `codebrix-webgpu` as in example 2.


PERFORMANCE TIPS
================

    * Ask for the adapter and the device ONCE (GpuContext::request) and
      share the context; every canvas of the page uses the same device.
    * Create pipelines and buffers in `prepare`, not in `render`.
    * Record into the frame's one encoder rather than submitting per layer.
    * The crates are unpacked into the frontend's vendor folder only when
      the set of archives changes; an unchanged set costs nothing on the next
      materialize.
    * The `wgpu` crate's browser backend leaves shader compilation and GPU
      work to the browser, so it adds little to the build time of a
      frontend.
    * Graphics: decoded textures are uncompressed RGBA8 on the GPU, so
      GltfLoadOptions::max_texture_size is the main lever on a scene's
      memory; one GltfModelSet per scene decodes and uploads a shared image
      once. Keep frustum culling on, and give distant objects a
      LodInstance; frame_stats() shows what each frame drew. Shader variants
      are compiled on first use, so the first frame of a new combination
      (a first shadow, a first fade) can stutter once.
    * Animation: many brushstrokes belong in ONE stroke set
      (BrushStrokesShape), not in a BrushStrokeShape each; make a set once
      and keep it (a new set is a new upload). Re-colour an ownership map
      with with_colors, not a new OwnershipShape. Reuse an ImageSource
      while its pixels stay the same. A large painting that holds still at
      a high pixel ratio is cheaper drawn from its checkpoints. Keep
      offscreen groups (blends, masks, effects, finishes, water) few and
      no larger than their content, and a picture target at half the
      canvas size or less. RenderStats (AnimationLayer::control().stats())
      shows what each frame drew, uploaded and spent on the CPU.


COMMON PITFALLS TO AVOID
========================

    * Referencing the package only from the backend or another project. The
      archive is copied beside the program that references it: reference it
      from the BUILD PROGRAM. Symptom: FileNotFoundException naming
      webgpu-crates/webgpu-crates.zip when the frontend is materialized.
    * Calling Register() AFTER MaterializeAsync. The bundle is then not part
      of that build, and the frontend's dependencies are "no matching
      package". Register first.
    * Using CrateMode.ApplicationAddsCrates with `codebrix-webgpu`. It is not
      on crates.io; cargo cannot find it in that mode.
    * Leaving out default-features = false on a `wgpu` line of the
      frontend's own. The default features need native-backend crates the
      bundle does not carry.
    * Calling use_gpu() in a component that is not inside a WebGpuBoundary.
      It panics: put the component inside the boundary (the boundary's own
      component cannot read it, only its children can).
    * Putting the content that needs WebGPU beside a WebGpuBoundary instead
      of inside it. Only the children are left out where WebGPU is missing.
    * Calling set_page_gpu_options() inside a component below the first
      WebGpuBoundary, or after it has appeared. The page has asked by then;
      call it at the top of the root component (use_hook).
    * Probing with wgpu::util::is_browser_webgpu_supported() before
      GpuContext::request(). The probe requests an adapter itself; the
      browser then asks twice. The one request's error is the answer.
    * A layer that clears (LoadOp::Clear) wipes the layers below it. The
      host clears; layers load.
    * sRGB-encoding colour in a shader, or writing colour that is not
      premultiplied. The view encodes; the convention is premultiplied.
    * Reading each layer's own time. Use frame.info (the one clock) and the
      clock's named points, so that pausing and speed apply to every layer.
    * Asking for a WebGL fallback. There is none; show the "WebGPU is not
      available in this browser" message instead.
    * Naming kurbo, peniko or another crate the Animation package carries
      (harfrust, i_overlay, usvg, velato ...) in the frontend's own
      Cargo.toml at another version than the package carries - or one the
      WebGpu package carries (png, zune-jpeg) or the Graphics package (gltf).
      The vendor folder holds only that version, and the platform's strict
      rule refuses another. Use the crates' re-exports
      (codebrix_webgpu_animation::kurbo, ::peniko) and their own API for the
      rest (codebrix_webgpu::decode_png, the graphics crate's glTF loader).
    * Reading a clock in an animation layer (a performance timer, a counter
      of frames). Set the session's time and seek the playback controller
      from frame.info.time, so that pausing the host's clock stops the
      animation with everything else. RenderStats' CPU milliseconds are a
      measurement to show, never a time to draw from.
    * Writing a SessionRenderOptions (or any options) literal without
      `..Default::default()`. The struct gains fields; it is not Copy
      either - clone it to use it twice.
    * Listing a picture target's AnimationLayer::to_picture layer ABOVE
      the layers that read it. They then see the previous frame's picture
      (a transparent one on the first frame); list it below them.
    * Expecting a Multiply second session of a wipe to multiply with the
      first render. It blends with what is under the wipe: draw the first
      session underneath as well, then wipe.
    * Drawing something that changes every frame BELOW a checkpointed
      stroke set (or moving the painting on screen). The cache then never
      holds: draw changing things above the painting.
    * Handing an HDR texture to a stroke pickup, or an Rgba8Unorm texture
      with SrgbEncoded that the page also draws as a picture. Tone-map HDR
      first; for a texture also shown, use Rgba8UnormSrgb with
      PictureEncoding::Linear.
    * A stroke set whose FollowElement anchors name elements on another
      layer or in another group. The strokes take the element's OWN
      transform only, not its parents': put the stroke shape at the same
      level as the elements it follows.
    * Using a MIX blend mode (Multiply, Screen, ... Luminosity) on many
      groups. Each such group copies the canvas-sized target below it; use
      them on a few groups, not hundreds.
    * Taking the WebGpu package with CodeBrix.WebPlatform packages of
      another version. The Core pin is exact: NuGet reports a conflict, or the
      strict rule stops the build. Use the platform version the package pins.
    * Porting System.Numerics matrix products in the same order. Reverse
      them (see THE MATHS).
    * Leaving the WebGpu crates' notices out of the site. Serve
      WebGpuCrates.ReadNotices() with the platform's (and
      GraphicsCrates.ReadNotices() / AnimationCrates.ReadNotices() when the
      graphics / animation crate is used).
    * Ending a block with `if let` / `match` on a RenderPass as its last
      expression. A pass borrows its GraphicsFrame, and the temporaries of a
      block's tail expression outlive the block (edition 2021): the compiler
      reports "`graphics_frame` does not live long enough". End that
      `if let` / `match` with a semicolon, or bind the pass with `let`.
    * Installing an uncaptured-error handler (device.on_uncaptured_error) or
      a device-lost callback yourself. WebGPU keeps ONE of each per device and
      codebrix-webgpu's is the one every reader depends on; subscribe
      instead (gpu.subscribe_errors(), or a GraphicsContext's take_errors()).
    * Reading an error at once after the call that caused it. The browser
      hands an uncaptured error over when the device next does work - a page
      that draws submits every frame, so it arrives a frame or so later. Code
      that must have the verdict at once awaits validate(..).
    * Being surprised by a console warning for an uncaptured error. Chrome
      also writes each uncaptured WebGPU error to the console as a warning,
      handler or not; the subscriptions still receive it.
    * Expecting an old Cargo.lock to follow a package update by itself. A new
      package release can carry a new VERSION of a crate of ours (or of a
      vendored crate); the frontend's Cargo.lock then names one that is no
      longer in the vendor folder. The platform handles this (its own
      AGENT-README states the rule): a lock the platform started carries a
      MARKER comment as its third line ("# CodeBrix.WebPlatform keeps this
      file in step ..."), and while the marker is there Materialize brings the
      moved entries in step by itself (it reports "Cargo.lock brought in step
      (codebrix-webgpu-graphics <old> -> <new>)"). A lock WITHOUT the marker is
      the developer's: Materialize leaves it alone and warns, and the build
      refuses with the exact command to run in the frontend's folder (cargo
      update --offline -p <crate> for each crate that moved).
    * Calling SpriteBatch::end or MeshRenderer::end with another pass than
      begin's, or drawing outside begin/end. end returns an Err that says so
      (a sprite draw outside begin/end is ignored and counted).
    * Drawing 3D into a canvas frame without a depth texture of the layer's
      own. The host has none: keep a CanvasFrameTarget and begin with
      begin_canvas_frame_on (it keeps the depth at the canvas's size), or
      make a depth texture in resize and pass it to begin_canvas_frame; and
      begin the pass with clear_depth Some(1.0).
    * Drawing the skybox after ALL the models. It covers every blended part
      that has only sky behind it: end_opaque, then the skybox, then
      end_blended.
    * Changing a model's materials or textures after making an instance of
      it (or cloning or drawing it). material_mut and set_material_texture
      need the model's only handle and return an error otherwise: change
      them right after loading.
    * A pipeline of your own kept for one target format only, on a target
      whose multisampling a control switches. set_pipeline panics with the
      mismatch: keep one pipeline per pass.target().formats(), built with
      .with_sample_count(formats.sample_count).
    * Blooming before the target's last scene pass, at 4 samples. The next
      pass's resolve overwrites the halo: bloom after the last scene pass,
      then tone map.
    * Drawing the water surface on the plain canvas path. It reads the scene
      behind it, which only an offscreen target allows: draw the scene into
      a HeadlessRenderTarget with Depth32Float depth (WATER).
    * Expecting update_to to pose a page opened at ?t=12 at second 12 on its
      first call. Its first call only records the time: use
      update_to_since_zero.
    * Expecting the skybox's cube to stay sharp for a detailed panorama at
      the default size. Raise EnvironmentOptions::cube_size (memory grows
      with its square).
    * Sprites at the wrong size on a high-density screen. The batch works in
      the target's (device) pixels: begin with
      Affine2::from_scale(Vec2::splat(pixel_ratio)) to draw in CSS pixels.
    * Expecting a factory to fail when WebGPU refuses its object. It returns
      the object; the error is queued (take_errors), or awaited with
      validate / build_checked / try_create_shader.
    * Beginning a layer's render pass with Some(clear colour). That wipes
      the layers below; pass None.
    * Handing load_model / load_models a .glb or .gltf file. They read MODEL
      DATA only (the error says so): read a glTF file with GltfDocument::parse
      and a GltfModelSet (LOADING glTF MODELS).
    * One GltfModelSet per model of a scene. Textures are shared only within
      one set: keep ONE set for the scene, and the same base address for
      external_files and add.
    * Writing model data with one release of the Graphics package and
      reading it with a crate of another MAJOR format version. The reader
      refuses it; use the same release on both sides.
    * Reading the joint palette or node matrices in C#'s multiplication
      order. ModelInstance's matrices are glam's (parent * local, node *
      inverse bind); their bytes are the same as C#'s.
    * Asking for occlusion queries. This version refuses them (the wgpu
      browser backend it carries drops a pass's occlusion query set, and the
      whole frame would fail): see QUERIES.
    * Requiring TIMESTAMP_QUERY. Not every browser or adapter grants it: ask
      for it with GraphicsContextOptions::optional_features, check
      has_feature, and leave GPU timing out where it is missing.
    * Copying into a buffer while it is being mapped. WebGPU refuses the
      copy (and the frame): check map_state() == BufferMapState::Unmapped
      before a frame copies into a read-back buffer (GpuTimer does this
      itself).
    * Asking for the same file again every time a scene is opened. The
      platform's client reads a file answer with the fetch cache mode
      "no-cache": the browser keeps it and REVALIDATES it with the backend
      on each request (a 304 with no body while the file is unchanged), but
      every request is still a round trip. Keep the bytes (or what was made
      from them) for the life of the page, as the GraphicsDemo sample does
      (frontends/graphics/src/page_cache.rs).
    * Driving an animated texture from a clock of its own. Pass the frame's
      time (frame.info.time), so that pausing the host's clock holds it.


WHAT THIS PACKAGE DOES NOT DO
=============================

    * No WebGL and no native (desktop) backends; the crates are for a
      WebGPU-capable browser only.
    * No JavaScript of its own and no pre-compiled WebAssembly: the crates
      are compiled into the application's own WebAssembly file.
    * `codebrix-webgpu` does not hide `wgpu`: a renderer draws with the
      `wgpu` API itself.
    * No code that talks to the application's backend: that is the
      platform's shared crate's job.
    * Nothing registers itself; nothing is downloaded at build time in the
      default crate mode.
    * The animation crate draws in real time for the browser window only:
      no export of images or frame sequences (see WHAT HAS NO EQUIVALENT in
      THE codebrix-webgpu-animation CRATE).
    * The graphics crate fetches nothing and reads no file system: glTF
      files, PNG and JPEG files, model data, pixels and panoramas are bytes
      the application hands it. It decodes PNG and JPEG (through
      codebrix-webgpu) and Radiance .hdr, and no other image format.
    * The animation crate reads no files and fetches nothing: fonts,
      pictures, caption text, SVG documents and Lottie JSON are bytes the
      application hands it. The only image file formats it decodes are PNG
      and JPEG (a JPEG's EXIF orientation is not applied); it never uses a
      system font. It ships no artwork and no brush styles.
    * Water that comes from a simulation (an application's texture, such as
      the Graphics package's wave field) cannot be rewound: only the ripple
      field and pictures drawn from the time are exact under seeking.


WORKING EXAMPLES ON GITHUB
==========================

    Registration, the paths, the missing-archive message and the archive's
    shape:
      https://github.com/ellisnet/CodeBrix.WebPlatform.WebGpu/blob/main/tests/CodeBrix.WebPlatform.WebGpu.Tests/WebGpuCratesTests.cs
    A frontend that uses codebrix-webgpu (two layers, one clock, a named
    point), compiled by the tests:
      https://github.com/ellisnet/CodeBrix.WebPlatform.WebGpu/blob/main/tests/CodeBrix.WebPlatform.WebGpu.Tests/Materialize/frontend-main.rs
    A frontend that draws through codebrix-webgpu-graphics as a layer and
    compiles every built-in shader - and uses the glTF loader, textures from
    PNG and JPEG files, culling, lights and shadows, multisampling, level of
    detail, the effects and water - compiled by the tests:
      https://github.com/ellisnet/CodeBrix.WebPlatform.WebGpu/blob/main/tests/CodeBrix.WebPlatform.WebGpu.Graphics.Tests/Materialize/frontend-main.rs
    GraphicsCrates (registration, both bundles, the archive's shape):
      https://github.com/ellisnet/CodeBrix.WebPlatform.WebGpu/blob/main/tests/CodeBrix.WebPlatform.WebGpu.Graphics.Tests/GraphicsCratesTests.cs
    The glTF loader checked against a C# glTF-to-model-data converter on
    real model files (the reference converter and its tests):
      https://github.com/ellisnet/CodeBrix.WebPlatform.WebGpu/tree/main/tests/CodeBrix.WebPlatform.WebGpu.Graphics.Tests.GltfReference
    The GraphicsDemo sample (its "graphics" application has a scene for
    every area of the graphics crate, the desktop model viewer's scene
    first, with its models served as glTF files and read by the crate's
    loader, and a row of feature demos - lights, shadows, culling,
    translucency, glow and bloom, particles, picking and labels,
    anti-aliasing, level of detail, a water pond and a water lake):
      https://github.com/ellisnet/CodeBrix.WebPlatform.WebGpu/tree/main/samples/GraphicsDemo
    The Graphics crate's names:
      https://github.com/ellisnet/CodeBrix.WebPlatform.WebGpu/blob/main/tests/CodeBrix.WebPlatform.WebGpu.Graphics.Tests/GraphicsRustCrateTests.cs
    The Animation crate's names:
      https://github.com/ellisnet/CodeBrix.WebPlatform.WebGpu/blob/main/tests/CodeBrix.WebPlatform.WebGpu.Animation.Tests/AnimationRustCrateTests.cs
    AnimationCrates (registration, both bundles, the archive's shape):
      https://github.com/ellisnet/CodeBrix.WebPlatform.WebGpu/blob/main/tests/CodeBrix.WebPlatform.WebGpu.Animation.Tests/AnimationCratesTests.cs
    A frontend that draws a session of shapes with easing, a colour ramp, a
    timeline and the playback controller through codebrix-webgpu-animation,
    and peniko brushes into the VectorScene directly - and uses the
    render-time looks and wipes, fixed-width and sketched strokes, stroke
    sets with tips, anchors, boil and flow, the painting's planning and
    colour rules, live colour sources and picture targets, the surface
    finish, checkpoints, stroke-level transitions, and the water (ripple
    field, caustics, refraction, reflection) - compiled by the tests:
      https://github.com/ellisnet/CodeBrix.WebPlatform.WebGpu/blob/main/tests/CodeBrix.WebPlatform.WebGpu.Animation.Tests/Materialize/frontend-main.rs
    A frontend that uses BOTH the graphics and the animation crates (a
    Graphics layer under an Animation layer), compiled by the tests:
      https://github.com/ellisnet/CodeBrix.WebPlatform.WebGpu/blob/main/tests/CodeBrix.WebPlatform.WebGpu.Animation.Tests/Materialize/frontend-both-main.rs
    The GraphicsDemo sample's "animation" application (the C# demo's scene
    catalogue with its keys, a scene for each area of the animation crate,
    SVG and Lottie included, and feature scenes with controls for the
    render-time looks and wipes, fixed-width strokes, the sketch helper's
    settings, ownership colours, stroke sets and their planning, live
    colour pickup, the surface finish, checkpoints and stroke-level
    transitions, and water scenes - ripples on a pool floor (a click drops
    a stone; scrubbing back removes its ring), a reflection below a
    waterline, a painted pond and a simulated pond that cannot be rewound -
    with the noise scene's caustics panel; the sample's MAINTAINER-README.txt
    lists them):
      https://github.com/ellisnet/CodeBrix.WebPlatform.WebGpu/tree/main/samples/GraphicsDemo/frontends/animation


QUICK REFERENCE CARD
====================

  BUILD PROGRAM (references FrontendBuilder, the machine's host-tools
  package and CodeBrix.WebPlatform.WebGpu[.Graphics|.Animation]
  .ApacheLicenseForever)
    WebGpuCrates.Register();                  // BEFORE MaterializeAsync
    GraphicsCrates.Register();                // instead, for the graphics
                                              //   crate (WebGpu's first)
    AnimationCrates.Register();               // for the animation crate
                                              //   (WebGpu's first; with
                                              //   Graphics too, or alone)
    await frontend.MaterializeAsync();        // writes the crates too
    result.CrateBundles                       // "webgpu": Added/SharedCrates
    WebGpuCrates.ReadNotices()                // serve with the site
    WebGpuCrates.GetArchivePath()             // <program>/webgpu-crates/
                                              //   webgpu-crates.zip

  FRONTEND Cargo.toml
    codebrix-webgpu = "..."                   // default crate mode only
    codebrix-webgpu-graphics = "..."          // with the Graphics package
    codebrix-webgpu-animation = "..."         // with the Animation package

  FRONTEND Rust (codebrix_webgpu)
    GpuContext::request().await               // Err: WebGpuUnavailable
    WebGpuBoundary { Content {} }             // Content only where WebGPU works
    WebGpuBoundary { fallback: |_| rsx!{..}, Content {} }  // else the fallback
    use_gpu()                                 // inside it: the GpuContext
    set_page_gpu_options(PageGpuOptions{..})  // once, in the root component
    CanvasLayers::new(vec![bottom, top])      // push/insert/remove live
    CanvasClock::new()                        // pause/resume/set_speed/seek
    clock.define_point("name", seconds)       // named points in time
    WebGpuCanvas { context, layers, clock, class: "..." }
    WebGpuCanvas { transparent: true, .. }    // on the page: no frame in CSS
    impl CanvasRenderer { prepare(target) render(frame) input(event) }
    frame.view / target.format / encoder / info.time / point_crossed(name)
    host clears; layers LoadOp::Load; linear, premultiplied colour
    gpu.subscribe_errors().take()             // every reader sees every error
    codebrix_webgpu::wgpu, codebrix_webgpu::math::{Vec3, Mat4, Quat}
    decode_png(&b)? / decode_jpeg(&b)? / decode_image(&b)?   // -> DecodedImage
        // { width, height, rgba }: straight RGBA8, rows top down
    CanvasInput::PointerMove { buttons, kind, modifiers, .. }  // match with ..
    WebGpuCanvas { suppress_context_menu: false, .. }  // keep the browser menu

  FRONTEND Rust (codebrix_webgpu_graphics)
    GraphicsContext::wrap_gpu_context(&gpu)   // share the host's device
    ctx.create_*(.., label) / options structs // C# names, snake_case
    ctx.create_render_pipeline(&shader).with_*(..).build()
    ctx.begin_canvas_frame(frame, None)       // a LAYER of the host
    gf.begin_render_pass(None, None)          // None: load, never clear
    ctx.take_errors()                         // C# TakeErrors(); async:
    validate / build_checked / try_create_shader / buffer.read  .await
    shaders::BuiltInShaderSource::all(PbrFeatures::every_shader_variant())
    SpriteBatch::new(&ctx)?; batch.begin(&pass, opts)?; draw*/fill*;
        batch.end(&mut pass)?                 // one draw per texture run
    ctx.create_mesh(&MeshShapes::sphere(..), None)?; MeshRenderer::new(&ctx)?
    renderer.begin(&pass, &camera, Some(&lighting))?; draw(&mesh, &mat,
        world)?; renderer.end(&mut pass)?     // own depth texture, Some(1.0)
    OrbitController::new(orbit) / with_limits(orbit, limits): input(&e) /
        update(time) / set_viewport(&cam, css_h) / apply(&mut cam)
                                              // right/middle/Shift drag pans
    controller.frame_box(min, max, &camera, aspect)   // aim at a box
    camera.canvas_to_ray(x, y, css_w, css_h).nearest_box_hit(boxes)  // picking
    camera.world_to_canvas(world, css_w, css_h)   // .x .y .in_view: labels
    ctx.create_texture_from_png(&png, RgbaTextureOptions { srgb: true, .. })?
    let doc = GltfDocument::parse(&glb)?; doc.external_files(Some(base), true)
    let mut set = GltfModelSet::new();        // ONE per scene: shared textures
    set.add(&doc, "tree", Some(base), &|a| files.get(a).map(Vec::as_slice),
        &GltfLoadOptions::default())?; let models = set.create_models(&ctx, &opts)?;
    let models = ctx.load_models(&bytes, &ModelLoadOptions::default())?;
        // a model-data blob: the generated <name>_bytes call, or include_bytes!
    model.set_material_texture(None, ModelTextureKind::BaseColor, tex)?
        // right after loading, before any instance
    let inst = models[0].create_instance_at(world);  // .tint .alpha .never_cull
    let mut mr = ctx.create_model_renderer()?;
    mr.begin(&pass, &camera, Some(&lighting))?; mr.draw(&inst)?;
    mr.end_opaque(&mut pass)?; skybox.draw(..)?; mr.end_blended(&mut pass)?;
        // or mr.end(&mut pass)? without a sky; mr.frame_stats()
    lighting.local_lights.push(PointLight::new(p, color, intensity, range).into());
    lighting.sun_shadows = SunShadowSettings::covering(min, max);
    let mut lod = LodInstance::new(world, vec![LodLevel::single(a, 60.0), ..])?;
    mr.draw_lod(&mut lod)?;                   // between begin and end_opaque
    let sky = ctx.create_procedural_sky(Some(&ProceduralSkyOptions { .. }), None)?;
    let env = ctx.create_environment_from_bytes(&hdr_bytes, None)?;  // .hdr
    SceneLighting { environment: Some(env.clone()), .. }   // None: default sky
    let mut skybox = ctx.create_skybox_renderer()?; skybox.draw(&mut pass,
        &camera, Some(&lighting))?            // after end_opaque, before
                                              //   end_blended
    let mut target = ctx.create_canvas_frame_target(CanvasFrameTargetOptions::default())?;
    target.set_sample_count(4)?; let mut gf = ctx.begin_canvas_frame_on(frame, &mut target)?;
    bloom.apply(&mut gf, hdr.color())?;       // after the last scene pass
    glows.begin(&pass, &camera, time)?; glows.add(&Glow::new(..)); glows.end(&mut pass)?
    particles.update(&mut gf, delta)?; particles.draw(&mut pass, &camera)?
    timer.begin_frame(); timer.pass("scene"); timer.end_frame(&mut gf); timer.timings()
    field.update(&mut gf, delta)?; water.capture_scene(&mut gf, &hdr)?;
    water.draw(&mut pass, &WaterFrame { .. }, &surface)?   // offscreen path only
    let hdr = ctx.create_headless_target(w, h, HeadlessTargetOptions {
        format: Rgba16Float, .. })?;          // scene into hdr.view()
    let mut tone = ctx.create_tone_mapper()?; tone.operator = ..; tone.exposure
    tone.apply_in_pass(&mut canvas_pass, hdr.color())?  // canvas pass LOADS
    tone.preserve_alpha = true; hdr cleared to Color4::TRANSPARENT; no skybox
                                              // models on the page, HDR on
    inst.animation().play_named("walk", PlayOptions::default())?;
    inst.animation().cross_fade_named("run", 0.4, PlayOptions::default())?;
    inst.animation().update_to(frame.info.time);   // or .update(delta)
    inst.animation().update_to_since_zero(frame.info.time);  // a page at ?t=12
    { let mut p = inst.begin_pose_edit(); p.local_transforms()[n] = t; }
        // skinning and morph targets: drawn by the model renderer
    let anim = ctx.create_animated_texture(w, h, &frames, &AnimatedTextureOptions::default())?;
    batch.draw_animated(&anim, frame.info.time, rect, Color4::WHITE);
    let mut b = ctx.create_render_bundle_for_pipeline(&p, None); ..; let bundle = b.finish(None);
    pass.execute_bundle(&bundle);
    let cp = ctx.create_compute_pipeline(&shader).with_bind_group_layouts(&[&l]).build()?;
    { let mut c = gf.begin_compute_pass(&ComputePassOptions::default()); c.dispatch(n, 1, 1); }
    let args = ctx.create_indirect_buffer_from(&[DrawIndirectArgs::new(6)], None)?;
    pass.draw_indirect(&args, 0)?;
    let timer = ctx.create_timer(2, None)?;   // only if has_feature(TIMESTAMP_QUERY)
    RenderPassOptions { timestamps: Some(timer.pass(0)?), .. }; timer.resolve(&mut gf);
    timer.read_milliseconds().await? / texture.read_rgba().await? / buffer.map_async(..).await?
    // what a browser has no equivalent for: WHAT A DEVELOPER PORTING C# CODE
    // WILL NOT FIND, AND WHAT TO USE INSTEAD

  FRONTEND Rust (codebrix_webgpu_animation)
    DrawingSession::new(DrawingSessionOptions { calibration_size: (1280, 720), ..Default::default() })
    session.add_layer("Name", color).add_shape(CircleShape::new(x, y, r, ShapeOptions { .. })?)
    RectangleShape / EllipseShape / LineShape / PolylineShape / PathShape
    PathGeometry::parse("M ..")? / PathGeometryBuilder::new().move_to(..).build()
    LinearGradientFill::new(start, end, &stops, GradientSpread::Pad)?.into()
    Easing::cubic_out(t) / Easing::evaluate(kind, t) / CubicBezierEasing::EASE
    ColorRamp::from_colors(&[a, b, c])?.at(t)
    AnimationTimeline::new(8.0, 30.0)?; timeline.add_cue("move", 1.0, 2.0, kind)?
    timeline.add_marker_at_point("title", &frame.points, "title in")?
    PlaybackController::new(timeline).seek(frame.info.time)   // or advance(delta)
    AnimationLayer::new(|frame, scene| { session.set_time(frame.info.time);
        session.render(scene, frame.info.width, frame.info.height); })
    scene.fill / stroke / push_layer(blend, alpha, ..) / push_clip_layer / pop_layer
    let fonts = FontRegistry::new(); fonts.register_font(bytes, None)?   // bytes only, no system fonts
    DrawingSessionOptions { fonts: Some(fonts.clone()), .. }; session.draw_text(t, x, baseline, size, TextShapeOptions { .. })?
    TextShape::with_fill(t, x, y, size, &fonts, fill.into(), opts)?; text.width() / left() / get_bounds()
    TextShapeOptions { missing_characters: MissingCharacterMode::ShowTofu (default) | FontDefault | Blank, .. }
    text.missing_characters(); fonts.get_missing_characters(t, None)?   // UTF-8 byte offsets
    session.set_debug_overlay(Some(DebugOverlay::new(Some(fonts), Some(timeline))))
    session.set_opacity(a) / layer.set_opacity(a); session.render_with(.., &SessionRenderOptions { include_background, target, ..Default::default() })
    SessionRenderOptions { blend: SessionBlendMode::Multiply, opacity, paint: SessionPaint::Line(LinePaint::new(style, ink).with_fill(paper)) | Flat, effects: false, authored_layers, refraction, .. }
    SessionWipe::new(progress, WipeEdge::Line { angle_degrees } | WipeEdge::Circle { center }, softness_px).render_sessions(scene, w, h, (&a, &oa), (&b, &ob)) / .render(scene, area, |s| .., |s| ..)
    control.stats(): record_ms / build_ms / encode_ms / cpu_ms(), image_uploads, color_table_uploads, stroke_draws, strokes, checkpoint_hits, ripples_evaluated ...
    session.render(..) draws NO background (a layer floats); render_with_background(..) draws it
    let id = session.draw_circle(..)?; session.animations_mut().bind(id, AnimatedProperty::Position, point_track)?
    FloatTrack::from_keys(&[(0.0, 0.0, EasingKind::Linear), (2.0, 1.0, EasingKind::CubicOut)])?.with_loop_mode(TrackLoopMode::Loop)
    bind(BindingTarget::layer("L") | BindingTarget::Camera, prop, track)?; session.set_camera(Some(DrawingCamera::new()))
    DrawingShape::from(shape).with_transform(DrawingTransform::translate(x, y)).with_time_window(Some(a), None)
    DrawingGroup::new(Some(c)).add(shape); AnimatedShape::new(|t| Some(shape.into()), 0.0); layer.set_parallax(0.5)?
    path.get_length() / get_point_at(f) / trim(a, b, offset)?; PathShape set_trim_end(0.0)? + bind TrimEnd
    bind_along_path(id, &path, progress, PathTravelOptions { orient_to_path: true, .. })?
    PathMorph::new(a, b, 0)? -> PathShape::with_morph(m, 0.0, style, ..)? + bind MorphFraction
    ImageSource::from_rgba(&rgba, w, h)?   // pixels made in the frontend
    ImageSource::from_picture_pixels(&bytes)?   // decoded on the backend, PicturePixelsWriter.Write(rgba, w, h)
    ImageSource::from_png(&png_bytes)? / ImageSource::from_jpeg(&jpeg_bytes)?   // decoded in the browser (the only two file formats; no EXIF orientation)
    session.draw_image(&src, dest, ImageSampling::Linear)?; ImageShape::new(src, dest, ImageShapeOptions { .. })?
    SpriteSheet::from_grid(&img, cols, rows, 0)? -> session.draw_sprite(&sheet, dest, SpriteShapeOptions { frames_per_second: 12.0, .. })?
    group.set_mask(Some(DrawingMask::inside(path))); layer.set_mask(Some(DrawingMask::from_layer("Matte", false)))
    shape.set_effects(Some(DrawingEffects::glow(16.0, c, 1.5)?.with_shadow(offset, 6.0, shadow)?)); layer.set_effects(..)
    bind BlurRadius / GlowRadius / ShadowBlurRadius (float), ShadowOffset (point), ShadowColor / GlowColor / Tint (colour), FrameIndex (float)
    DrawingLayout::from_guide(size, Some(&SafeAreaGuides::tiktok()), Default::default())?.place_at(LayoutAnchor::CENTER, LayoutReference::SafeArea, 0.0, 0.0)
    let layout = session.layout_text(text, 24.0, None, Some(&TextLayoutOptions { max_width: Some(400.0), .. }))?   // UTF-8 byte offsets
    let id = session.draw_text_block(layout, x, y, Some(ink))?; bind(id, AnimatedProperty::RevealFraction, track)?
    block.create_piece_group(TextUnit::Word); bind_stagger(&ids, Some(&StaggerOptions { interval: 0.1, .. }))?
    bind_text_along_path(&block, TextUnit::Character, "Layer", &path, progress, PathTrainOptions { roll_on_baseline: true, .. })?
    session.draw_captions(CaptionTrack::parse(&text)?, None, None)?   // against the frame; opt in: CaptionPlacement::from_guide
    a.union(&b, None)? / intersect / subtract / exclusive_or / simplify / to_stroke_outline(&style, None)? / inflate(d, opts)?
    path.contains(p, None)? / distance_to_edge / get_signed_distance / get_nearest_point / distance_with_offset
    bind_with_mode(id, AnimatedProperty::Rotation, BoundTrack::float_fn(f), BindingMode::Add)?
    bind_along_path(id, &path, p, PathTravelOptions { roll_radius: 30.0, .. })?; bind_train_along_path(&ids, &path, p, opts)?
    FormationTargets::fill_geometry(&shape, opts)? -> FormationMember::from_targets(..)? -> bind_formation(&m, "Rest", "Air", ..)?
    bind(BindingTarget::layer("L"), AnimatedProperty::IsVisible, bool_track)?   // a hidden matte still masks
    session.set_safe_area_overlay(Some(SafeAreaOverlay::new(guide))); session.check_safe_area(&guide, None, Default::default())
    let svg = Arc::new(DrawingSvg::from_svg(&markup, &DrawingSvgOptions { fonts, ..Default::default() })?); svg.report()
    let id = session.draw_svg(&svg, dest, DrawingSvgAspect::Meet)   // plays the document's own SMIL / CSS at the session's time
    shape.bind_part("hand", AnimatedProperty::Rotation, track)?; session.draw_svg_shape(shape); shape.set_image_href(href, pixels)
    let lottie = Arc::new(DrawingLottie::load(&json_bytes, &DrawingLottieOptions::default())?); lottie.report()   // dotLottie: unpack on the backend
    let id = session.draw_lottie(lottie, dest, DrawingLottieAspect::Meet, LottiePlayOptions { playback: LottiePlayback::PingPong, range: LottieRange::Marker("wave".into()), ..Default::default() })?
    for (name, t) in shape.marker_times() { clock.define_point(&name, t); }; shape.set_image("image_0", pixels)
    geometry.dash(&[20.0, 10.0], 0.0)? / PathGeometry::rounded_polygon(&points, 12.0)?
    layer.set_frame_operations(Some(DrawingFrameOperations::NONE.with_bloom(sigma, gain)?)); session.set_layer_transition("A", "B", t, f)?
    NoiseShape::new(x, y, w, h, kind, seed, freq, opts)?   // GPU: same character, not the same pattern (Caustics: the same); DrawingNoise::fill = exact (CPU, slow)
    session.draw_particles(&system, t, Some(softness))?; BrushStrokeShape::new(..)?.with_progress(p); OwnershipShape::new(&map, &colors, dest, opts)?
    base.with_colors(&colors)?   // an ownership map re-coloured: only the colour table is uploaded
    const SNAP: CubicBezierEasing = CubicBezierEasing::new(0.2, 0.0, 0.0, 1.0).unwrap();
    fonts.register_font_with_variations(bytes, Some("Body Bold"), &[FontVariation::weight(700.0)])?; fonts.get_variation_axes(Some("Body Bold"))?
    PathStrokeStyleOptions { width_mode: StrokeWidthMode::Fixed, .. }; geometry.sketch(&SketchOptions::default().boiled(frame.info.time, 10.0))?
    BoundTrack::color_fn(|t| ColorMix::mix(a, b, f(t), ColorInterpolation::Oklab)); ColorRamp / gradient fills .with_interpolation(ColorInterpolation::Oklab)
    let set = BrushStrokeSet::new(records)?; BrushStrokesShape::new(set, BrushStrokesOptions { boil, flow, .. })?.with_tips(tips).with_anchors(anchors)?; session.draw_brush_strokes(shape)
    DirectionField::from_picture / from_noise / from_guides; StrokeLayout::fill(..)?; PaintingTimeline::new().add_stage(..)?; timeline.schedule(bands, seed)?
    ColorRules { palette: Some(PaintPalette::new(pigments)), broken: Some(BrokenColor::default()) }.apply(color, seed)
    shape.with_color(StrokeColor::Picked(StrokePickup::new(StrokeColorSource::Picture(src) | Target(t) | Texture(ExternalPicture { .. }), place)))?
    AnimationLayer::to_picture(PictureTarget::new(w, h)?, |frame, scene| ..)   // listed BELOW its readers; ImageShape::from_picture_target(..)?
    layer.set_surface_finish(Some(SurfaceFinish::default()))?; shape.with_checkpoints(Some(StrokeCheckpoints::default()))?   // off by default
    BrushStrokesShape::morph(BrushStrokeMorph::pair(&a, &b, StrokePairingOptions::default())?, opts, None)? + bind MorphFraction
    RippleField::new(place, RippleOptions::default())?.with_rain(RippleRain::new(2.0, seed))?.with_drop(RippleDrop::new(p, t))?
    layer.set_refraction(Some(WaterRefraction::new(WaterSource::Ripples(field.clone()))))?; layer.set_reflection(Some(WaterReflection::new(line_y)))?
    session.take_reports()   // e.g. a matte layer name the session does not have; show beside the device's errors
    codebrix_webgpu_animation::{kurbo, peniko}   // re-exported; never name them yourself
    nothing drawn reads a clock: time is always frame.info.time / frame.info.delta (RenderStats' CPU timings are a measurement)

  .NET MODEL DATA (CodeBrix.WebPlatform.WebGpu.Graphics.Models)
    var set = new ModelDataSet(); set.Models.Add(model); set.Textures.Add(..);
    byte[] blob = ModelDataWriter.Write(set);  // serve from a [WireFile] GET

  CRATE NAMES
    GraphicsRustCrate.CrateName    "codebrix-webgpu-graphics"
    AnimationRustCrate.CrateName   "codebrix-webgpu-animation"

================================================================================
END OF AGENT-README
================================================================================
