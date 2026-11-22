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
            maths crate, and every crate they need, exactly as crates.io
            published them;
          * the crate `codebrix-webgpu` of this repository (see THE
            codebrix-webgpu CRATE): the WebGPU context, ONE canvas host
            component with its frame loop, layers and clock, the input, the
            "WebGPU is not available" result and message, and the maths
            types. It is not on crates.io; it travels only in this archive.
        Its ONE call, WebGpuCrates.Register(), registers them with the
        platform's frontend build; the build then writes them into the
        frontend beside the platform's own crates, so that the frontend
        still builds offline.

    CodeBrix.WebPlatform.WebGpu.Graphics
        Carries the Rust crate `codebrix-webgpu-graphics` of this repository
        (see THE codebrix-webgpu-graphics CRATE): 2D and 3D graphics in a
        platform frontend through WebGPU, with the type names, concepts and
        argument order of the .NET library CodeBrix.WebGpu.Graphics, drawing
        as a LAYER of the `codebrix-webgpu` canvas host. Its purpose and
        scope: what makes sense in a browser of 2D sprites and shapes, lit 3D
        meshes, models with physically based shading, skinning, morph targets
        and animation playback, image-based lighting with a skybox, tone
        mapping, animated textures, render bundles, indirect draws, compute,
        timestamp queries and texture readback; what a browser has no
        equivalent for is listed in that section in one place. Its ONE
        call, GraphicsCrates.Register(), registers the WebGpu package's
        crates FIRST and then its own; its assembly also holds the crate's
        names (GraphicsRustCrate).

    CodeBrix.WebPlatform.WebGpu.Animation
        Carries the Rust crate `codebrix-webgpu-animation` of this repository
        (see THE codebrix-webgpu-animation CRATE) and the third-party crates
        only it needs (`kurbo` for curves and strokes, `peniko` for brushes
        and blend modes, `harfrust` for text shaping and glyph outlines,
        `i_overlay` for path combining, `png` for PNG files, `usvg` for SVG
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
        queries), pictures and sprites (decoded pixels or PNG files), masks
        and effects, pixel planes with their real-time GPU versions
        (whole-frame operations, transitions, noise, particles), animated
        brush strokes and painterly playback, formations, safe-area guides,
        easing curves, colour ramps, timelines, a playback controller and the
        debug overlay - and, beyond NoSkia, SVG documents that animate
        themselves (SMIL and CSS) and Lottie animations; all drawn by the
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
WebGpu and Animation packages carry keep their own permissive licences; they
are listed with their licence texts in THIRD-PARTY-NOTICES-WebGpuCrates.txt
and THIRD-PARTY-NOTICES-WebGpuAnimationCrates.txt (see NOTICES FOR A BUILT
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
    use codebrix_webgpu_graphics::{GraphicsContext, Color4, BindingEntry,
        BindingLayoutEntry, BlendPreset, ColorTargetOptions, DrawOptions,
        RgbaTextureOptions, SamplerDescription};  // the graphics crate
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
            // "webgpu-graphics" (ONLY codebrix-webgpu-graphics: the wgpu
            // crates and codebrix-webgpu come from the WebGpu bundle). Calling
            // it again changes nothing; calling WebGpuCrates.Register() as
            // well changes nothing either.

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
            // harfrust, i_overlay, png, usvg, velato and the crates they
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
.NET tool) that converts models; the package itself reads no glTF and
decodes no image - the application fills the types (see
samples/GraphicsDemo/backend/GraphicsDemo.Assets for a glTF converter built
on CodeBrix.Graphics3D.Gltf2 and CodeBrix.Imaging). The types mirror what
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
generated <action>_bytes function. The platform's backend does not compress
answers yet: a blob with decoded textures is large (four bytes per texel)
but compresses very well.


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
       the Graphics package, "webgpu-graphics" (codebrix-webgpu-graphics).

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
                   class, style, id, label }
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
        keys, the space bar and the wheel do not scroll the page.
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

    CanvasInput: PointerDown { x, y, pointer_id }, PointerMove { x, y, dx,
    dy, pressed, pointer_id }, PointerUp { x, y, pointer_id }, Wheel {
    delta_x, delta_y } (CSS pixels, positive y towards the visitor), KeyDown
    { key }, KeyUp { key } (the browser's key names). Coordinates are the
    canvas's CSS pixels. Every pointer (a mouse button, a pen, each finger on
    a touch screen) has its own pointer_id, and each one's dx and dy are its
    own distances, so a layer can follow two fingers at once (a pinch: the
    graphics crate's OrbitController does). Match with `..`
    (`CanvasInput::PointerMove { dx, pressed: true, .. }`) so that a field
    added later does not break the match.

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
      it to each viewer): OrbitController and OrbitLimits; and the model-data
      READER (ModelDataSet, ModelData, ModelTextureData, ModelDataFormat),
      the browser's replacement for reading a glTF file.
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
      mip level that does not exist, a pipeline with a fragment stage but no
      colour target) return Err(GraphicsException) at once, with a message
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
        graphics_frame.begin_render_pass(None, None)
            clear_color None LOADS (a layer never clears; C# null does the
            same); clear_depth None loads depth (C# defaults to 1: pass
            Some(1.0) with a depth texture of the layer's own).
        Pipelines use the frame's target.format and premultiplied blending
        (BlendPreset::PremultipliedAlpha, or Opaque for a layer that covers
        everything); shaders write linear colour (the view encodes).
        Animate from frame.info.time (the host's clock): never read a clock,
        so that pausing and speed apply to every layer.
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
    for wgpu's own pass.
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
    combination).
    BuiltInShaderSource::all(variants) lists them by name: compile them once
    in a new browser (each browser has its own WGSL compiler) - the
    GraphicsDemo sample's "Shaders" scene does that and shows the messages.

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
    OrbitCamera: target, yaw (pub), pitch()/set_pitch (clamped to just under
        90 degrees), distance()/set_distance (at least min_distance),
        rotate(dyaw, dpitch), zoom(factor), position(), apply(&mut camera),
        OrbitCamera::orbit_position(target, distance, yaw, pitch).
    OrbitController::new(orbit) - the viewer's controls in one type:
        controller.input(&input)   from the layer's CanvasRenderer::input
        controller.update(frame.info.time); controller.apply(&mut camera)
                                   once per frame
        * drag (mouse, pen, ONE finger): orbit. THE CONTENT FOLLOWS THE
          POINTER: a drag to the right swings the scene to the right under
          the pointer (yaw falls by yaw_per_pixel = 0.008 per CSS pixel; the
          camera moves the other way); a drag down tips the near side down
          (pitch grows by pitch_per_pixel = 0.006).
        * wheel: a step towards the visitor zooms out (x1.1), away zooms in
          (x0.9).
        * TWO fingers: pinch - the distance follows the fingers' spread.
        * auto-orbit: yaw = 0.7 + 0.18 * time until the first drag (from
          the frame's time, so it stops with the clock);
          set_auto_orbit(true) starts it again from where the camera is.
        * OrbitLimits (default): pitch 0.05..1.35, distance 3..30 (the model
          viewer's on Android); every input stays within them.
        orbit, limits and the speeds are pub fields; is_auto_orbiting(),
        pressed_pointer_count().

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

MODELS: MODEL DATA, MODEL, INSTANCES AND THE MODEL RENDERER.
    The C# library loads a .glb/.gltf file, decodes its images and uploads
    everything in LoadModel(path). A browser has no file system and should
    not carry a glTF parser or image decoders, so the work is SPLIT:
    * In .NET (wherever the application likes - typically its backend),
      the glTF file is parsed and its images decoded into MODEL DATA: the
      Graphics package's types in CodeBrix.WebPlatform.WebGpu.Graphics.Models
      (ModelDataSet, ModelData, ModelVertex, ...; CORE API REFERENCE:
      MODEL DATA below), written into ONE binary blob by ModelDataWriter.
      The conversion from glTF is the application's (the package depends on
      Core and WebGpu only, never on a glTF or imaging library);
      samples/GraphicsDemo/backend/GraphicsDemo.Assets is a complete one,
      a port of the C# loader's CPU half.
    * In the browser, the crate READS the blob and uploads it:
      context.load_models(&bytes, &ModelLoadOptions::default())?. The blob's
      vertex, index and pixel ranges go to the GPU unchanged.
    THE BYTES COME FROM ANYWHERE; the crate never fetches. Two routes:
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
            (mipmaps are made on the CPU; glTF validation, sRGB colour maps
            and tangent generation are choices of the .NET side)
    Model (a cheap handle: clones share it) - C#'s Model: name(), meshes(),
    materials(), default_material(), primitive_material(&prim), nodes(),
    root_nodes(), skins(), animations(), textures(), samplers(),
    slot_texture(&slot), vertex_buffer(), index_buffer(), index_format(),
    skin_buffer(), vertex_count(), index_count(), triangle_count(),
    bounds_min()/bounds_max()/bounds_center(), warnings(),
    find_animation(name) / find_node(name) -> Option<usize> (C#'s -1 is
    None), create_instance() / create_instance_at(world). The data types
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
    pose changed), get_node_world_matrix(node), get/set_morph_weight(s),
    get_joint_palette(skin), reset_pose(), pose_version(); animation() (its
    AnimationPlayer, C#'s Animation), has_animation_player() and
    begin_pose_edit() (the next section).
    ModelRenderer (ctx.create_model_renderer()?) - C#'s: begin(&pass,
    &camera, Some(&lighting))?, draw(&instance)? any number of instances of
    any models (instances of one model become ONE instanced draw per
    primitive; blended primitives after the rest, back to front), end(&mut
    pass)?; last_draw_call_count(), last_instance_count(), forget(&model).
    It shades with the crate's PBR shader: base colour, metallic-roughness,
    normal, occlusion and emissive maps, vertex colours, texture transforms,
    unlit, the alpha modes, double-sided, mirroring, GPU skinning and morph
    targets, up to eight directional lights, and the ambient term: IMAGE-BASED
    by default (AmbientLighting::Environment, C#'s default: see the next
    section), or AmbientLighting::Hemisphere (SceneLighting's ambient_color as
    the sky, ambient_ground_color as the ground, around ambient_up).

    C# AND RUST SIDE BY SIDE (loading and drawing a model):
    // C# (desktop)                              // Rust (browser layer)
    var model = ctx.LoadModel("tree.glb");      // backend: ModelDataWriter.Write(set)
                                                 let bytes = models::model_data_bytes("tree")
                                                     .await.map_err(|e| e.message())?;
                                                 let model = ctx.load_model(&bytes,
                                                     &ModelLoadOptions::default())?;
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

    The complete version (the fetch with a loading line and a failure
    message, the placements from the backend's JSON, the orbit camera) is
    samples/GraphicsDemo/frontends/graphics/src/viewer_scene.rs and
    viewer.rs; the .NET side is samples/GraphicsDemo/backend.

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
            decoded in the browser: decode them where they are (the backend)
            into linear floats and pass a FloatImage.
        ctx.default_environment()?   the procedural sky with the defaults,
            baked on first use and kept by the context (C# DefaultEnvironment)
        ctx.brdf_lut()?              the 128 x 128 Rg16Float LUT (C# BrdfLut)
        EnvironmentOptions { cube_size, irradiance_size,
            irradiance_source_size, prefiltered_size, prefiltered_mip_levels,
            specular_sample_count, max_source_width, label } (C#'s defaults,
            clamped as C# clamps them)
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
    draw_count(). Draw it AFTER the opaque models in the same pass (it tests
    depth at the far plane and writes none; clear depth to 1).
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
    whole canvas. The tone-mapped scene is opaque, so layers BELOW it are
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
        models.End();                                models.end(&mut pass)?;
        skybox.Draw(pass, camera, lighting);         skybox.draw(&mut pass, &camera, Some(&lighting))?;
    }                                            }
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
        at most AnimationPlayer::MAX_STATES clips at once (C#'s MaxStates).
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
            finished once-clip does not run backwards)
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
    var archer = ctx.LoadModel(                  let bytes = models::model_data_bytes("character-archer")
        "character-archer.glb");                     .await.map_err(|e| e.message())?;
                                                 let archer = ctx.load_model(&bytes,
                                                     &ModelLoadOptions::default())?;
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
        HeadlessRenderTarget (read_depth: Depth32Float only, one f32 a pixel).
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


WHAT A DEVELOPER PORTING C# CODE WILL NOT FIND, AND WHAT TO USE INSTEAD.
    This is the one list of what the C# library CodeBrix.WebGpu.Graphics has
    and this crate does not, because a browser has no equivalent (or, where
    said, because of the wgpu version), with what to use instead.
    * NATIVE WINDOWS AND SURFACES: WindowSurface, NativeWindowSource,
      CreateWindowSurface / WrapWindowSurface, SurfaceAcquireResult,
      CompatibleWindow. Use: codebrix-webgpu's canvas host (WebGpuCanvas)
      and a layer (begin_canvas_frame); it configures, resizes, acquires
      and presents.
    * FILES AND DECODERS: LoadModel(path|stream), CreateTextureFromFile /
      Stream / Encoded / Image, TextureLoadOptions (decoding, ExtraUsage),
      CreateAnimatedTexture(path|stream|image), CreateEnvironment(path|
      stream|image) for PNG/JPEG/JPEG XL/OpenEXR, RadianceHdrDecoder.
      DecodeFile, ToImage, SaveAsPng, SaveAs, Model.SourcePath, and the
      .NET-side ModelLoadOptions (validation, sRGB colour maps, tangent
      generation, parallel decoding, ModelTextureCache). Use: decode in .NET
      (the backend) and hand the crate BYTES - model data
      (ctx.load_models), RGBA8 pixels (create_texture_from_rgba; premultiply
      there if wanted), RGBA8 frames (create_animated_texture), a FloatImage
      or Radiance .hdr bytes (create_environment / _from_bytes) - fetched
      with the generated <name>_bytes call or packed with include_bytes!.
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
  captions, text along a path), pictures and sprites (from decoded pixels or PNG files), masks and
  effects, PIXEL PLANES (CPU calculation with NoSkia's values) with their whole-frame operations,
  transitions, noise and particles also drawn ON THE GPU, ANIMATED BRUSH STROKES and the painterly
  playback (ownership maps, colour pickup, hard-edged masks, keyframe players), formations, layer
  visibility, safe-area guides, SVG DOCUMENTS (parts animated under program control, and the
  document's own SMIL and CSS animations), easing curves, colour ramps, timelines, a playback
  controller and the debug overlay - drawn on the GPU by the crate's own renderer as a LAYER of
  the codebrix-webgpu canvas host. It draws over whatever the layers below drew
  (a 3D scene of the Graphics package, for example) and never clears the canvas.
  It also plays LOTTIE ANIMATIONS (Lottie JSON read through velato, played at the session's time as
  an element of a session; see LOTTIE ANIMATIONS).

  Register the package in the build program (AnimationCrates.Register() - it registers the WebGpu
  package's crates first) and name the crate in the frontend's Cargo.toml (beside codebrix-webgpu):
      codebrix-webgpu-animation = "..."
  The crate re-exports kurbo (curves, shapes, transforms) and peniko (brushes, gradients, images,
  blend modes, fill rules) as it was built with them: use codebrix_webgpu_animation::kurbo and
  ::peniko rather than naming those crates in the frontend's own Cargo.toml.

THE RULE: NOTHING READS A CLOCK
  Time comes from the frame: the host's ONE clock (CanvasClock) hands every layer the same
  frame.info.time and frame.info.delta, pauses, seeks and carries named points in time. Set a
  session's time from it, seek a PlaybackController to it (or advance it by the delta); pausing the
  host's clock stops everything.

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
  for it); AnimationLayer::for_session draws a shared session as it is. AnimationLayer::control()
  gives the last frame's counters (RenderStats).

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
  ImageSource.FromEncoded(pngBytes)                   ImageSource::from_png(&png_bytes)?   (PNG only; no other format)
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

  Colours are straight-alpha sRGB bytes everywhere at the API (as in C#); the renderer works linear and
  premultiplied inside, as the canvas host requires. Gradient fills interpolate in straight sRGB, like
  NoSkia (so a LinearGradientFill made from a ColorRamp's stops matches ColorRamp::at).

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
  between the stops, as NoSkia does, and a first stop past 0 (or a last before 1) holds its colour out
  to the end.
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
                                                          include_background: false, target: None })
  (draw a session into part of a frame)               SessionRenderOptions { target: Some(RectangleF::new(x, y, w, h)), .. }
  The session's opacity fades EVERY LAYER AS ONE GROUP over the background fill, which is not faded:
  over an opaque background that is a fade from and to that colour; without the background
  (include_background: false) what is below shows through - the fade is transparency. A layer's
  opacity fades that layer alone (as its own group); an element's opacity multiplies its paint.
  Where the renderer differs from NoSkia: groups composite in linear light (the canvas host's
  convention), so a half-faded colour over another looks a little lighter than NoSkia's sRGB
  arithmetic; the gradient's own interpolation is NoSkia's.

DRAWING ON THE PAGE (an animation that floats on the web page)
  A session drawn as a LAYER leaves its BACKGROUND OUT BY DEFAULT: DrawingSession::render and
  AnimationLayer::for_session draw the layers alone, so what is below shows - lower canvas layers,
  or the page itself behind a transparent canvas. THIS IS A DELIBERATE DIFFERENCE FROM C#, where
  Render and the exports draw the background unless told not to (includeBackground defaults to
  true). Ask for the background when a scene wants it:
      session.render_with_background(&mut scene, w, h);
      session.render_with(&mut scene, w, h, &SessionRenderOptions { include_background: true, target: None });
  The recipe: WebGpuCanvas { transparent: true, .. }; no frame (border, ring, rounded corners,
  background) on the canvas or its container in the page's styling; the session drawn with
  render(...) (no background); translucent shapes and text then blend with the page exactly (the
  canvas host's last pass makes the page see premultiplied sRGB, so there are no fringes). The
  sample's /animation/?scene=on-the-page does this.

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
    ColorTrack, BoolTrack, or BoundTrack::float_fn / point_fn (C#'s Func<double, T> overloads). The
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
  ImageSource: decoded pixels (the picture-pixels form from the backend) or a PNG file's bytes read
  with ImageSource::from_png (see PICTURES AND SPRITES) - stretched over the image's box as the
  document places it. Without a picture it is not drawn (reported).
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
    dash offset; paint-order; linear gradients and radial gradients with a focal point and focal
    radius (objectBoundingBox resolved), spread pad / reflect / repeat, gradientTransform; clip-path
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

PICTURES AND SPRITES: DECODED PIXELS, THE PICTURE-PIXELS FORM, OR PNG FILES
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
       file (fetched from the backend as a file answer, or carried in the frontend). PNG is the ONLY
       file format the crate reads. Every PNG colour type and bit depth is brought to straight 8-bit
       RGBA (grey to equal channels, palettes looked up with their transparency, colour keys to
       alpha 0, 16-bit samples to their high byte, interlacing undone); the values are taken as
       sRGB-encoded (the file's gamma, chromaticity and ICC chunks are not applied); of an animated
       PNG only the default image is read; at most 8192 x 8192 pixels.
    3. MADE IN THE FRONTEND: any RGBA bytes the application computes or carries
       (ImageSource::from_rgba), or a CPU plane image (ImageSource::from_plane_image - C#'s
       FromPlaneImage; the planes' float-to-byte rule).
  Answer picture requests with an ETag and "Cache-Control: public, no-cache": a reload then
  revalidates (304) instead of downloading again.
  A clone of an ImageSource or a SpriteSheet is the SAME picture (C# shares one reference): the
  pixels are kept once and uploaded to the GPU once; the texture is dropped once no frame has used
  it for a while (C#'s Dispose is Drop).
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
  No equivalent: ImageSource.FromFile / FromImagingImage / FromDrawingBitmap, and FromEncoded for any
  format but PNG (decode on the backend and send the picture-pixels form instead). NoSkia's DrawingPicture
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
    pass the time (times a speed) as z.
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
    stops at one position make a hard threshold). NOT THE SAME PATTERN as the CPU's for a seed (the
    GPU hashes with 32-bit integers in 32-bit floats; the CPU is NoSkia's exact 64-bit arithmetic):
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
  NoSkia's values, the drawing on the GPU. Nothing reads a clock: pass the frame's time.
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
  WHAT IS NOT HERE, AND WHY: interactive drawing - pointer-driven stroke capture (PointerPressed /
  PointerMoved / PointerReleased / PointerCanceled and their normalized forms, UndoLastStroke, the
  active stroke and ActiveStrokeOpacity, the freehand Stroke and StrokePoint, DrawingLayer.AddStroke):
  this crate plays animations, it is not a drawing tool (a recorded stroke plays as a PolylineShape
  or a PathShape with a trim); render-ahead on a worker thread; the Coverage and VisibleAlpha mask
  meanings; exports at a default export size (pass the size).

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
    without their effects and counted in RenderStats::unsupported.
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
    morph, a trim) is re-flattened every frame.
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
  them on the backend (the picture-pixels form) or read a PNG file's bytes with
  ImageSource::from_png (see PICTURES AND SPRITES), and hand the ImageSource over with
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
  - ImageSource.FromFile, and FromEncoded for formats other than PNG: none - decode on the backend
    (CodeBrix.Imaging) and send the picture-pixels form (PicturePixelsWriter), or hand the crate the
    PNG file's bytes (see PICTURES AND SPRITES).
  - Noise DRAWN on the GPU (NoiseShape) has the same parameters and character as NoSkia's but not the
    same pattern for a seed; noise VALUES and DrawingRandom on the CPU are exact.
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

  WHERE ITS DATA COMES FROM: the crate takes values and bytes that the application hands it (font
  files, decoded picture pixels, PNG files, caption text, SVG documents, Lottie JSON) and never
  fetches or reads anything itself.




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
platform's, typically in the one THIRD-PARTY-NOTICES.txt in the backend's
wwwroot, written AFTER BuildAsync:

    File.WriteAllText("backend/MyApp.Api/wwwroot/THIRD-PARTY-NOTICES.txt",
        File.ReadAllText("THIRD-PARTY-NOTICES.txt") + "\n\n\n"
            + PlatformCrates.ReadNotices() + "\n\n\n"
            + WebGpuCrates.ReadNotices() + "\n\n\n"
            + GraphicsCrates.ReadNotices() + "\n\n\n"     // with Graphics
            + AnimationCrates.ReadNotices());               // with Animation


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

            await frontend.BuildAsync(FrontendConfiguration.Release, "backend/MyApp.Api/wwwroot");
            File.WriteAllText("backend/MyApp.Api/wwwroot/THIRD-PARTY-NOTICES.txt",
                File.ReadAllText("THIRD-PARTY-NOTICES.txt") + "\n\n\n"
                    + PlatformCrates.ReadNotices() + "\n\n\n"
                    + WebGpuCrates.ReadNotices());
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
      (harfrust, i_overlay, png, usvg, velato ...) in the frontend's own
      Cargo.toml at another version than the package carries. The vendor
      folder holds only that version, and the platform's strict rule refuses
      another. Use the crate's re-exports (codebrix_webgpu_animation::kurbo,
      ::peniko) and its own API for the rest.
    * Reading a clock in an animation layer (a performance timer, a counter
      of frames). Set the session's time and seek the playback controller
      from frame.info.time, so that pausing the host's clock stops the
      animation with everything else.
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
      own. The host has none: make one in resize, pass it to
      begin_canvas_frame, and begin the pass with clear_depth Some(1.0).
    * Sprites at the wrong size on a high-density screen. The batch works in
      the target's (device) pixels: begin with
      Affine2::from_scale(Vec2::splat(pixel_ratio)) to draw in CSS pixels.
    * Expecting a factory to fail when WebGPU refuses its object. It returns
      the object; the error is queued (take_errors), or awaited with
      validate / build_checked / try_create_shader.
    * Beginning a layer's render pass with Some(clear colour). That wipes
      the layers below; pass None.
    * Handing the crate a .glb or .gltf file. It reads MODEL DATA only (the
      error says so): convert the glTF in .NET and write it with
      ModelDataWriter.
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
    * Expecting the browser's HTTP cache to keep what a <name>_bytes call
      fetched. The platform's client asks with the cache switched off, so a
      blob is fetched in full each time it is asked for: keep the bytes (or
      what was made from them) for the life of the page, as the GraphicsDemo
      sample does (frontends/graphics/src/page_cache.rs).
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
    * The graphics crate reads no files and decodes no images: it takes
      pixels and data that the application hands it.
    * The animation crate reads no files and fetches nothing: fonts,
      pictures, caption text, SVG documents and Lottie JSON are bytes the
      application hands it. The only image file format it decodes is PNG;
      it never uses a system font.


WORKING EXAMPLES ON GITHUB
==========================

    Registration, the paths, the missing-archive message and the archive's
    shape:
      https://github.com/ellisnet/CodeBrix.WebPlatform.WebGpu/blob/main/tests/CodeBrix.WebPlatform.WebGpu.Tests/WebGpuCratesTests.cs
    A frontend that uses codebrix-webgpu (two layers, one clock, a named
    point), compiled by the tests:
      https://github.com/ellisnet/CodeBrix.WebPlatform.WebGpu/blob/main/tests/CodeBrix.WebPlatform.WebGpu.Tests/Materialize/frontend-main.rs
    A frontend that draws through codebrix-webgpu-graphics as a layer and
    compiles every built-in shader, compiled by the tests:
      https://github.com/ellisnet/CodeBrix.WebPlatform.WebGpu/blob/main/tests/CodeBrix.WebPlatform.WebGpu.Graphics.Tests/Materialize/frontend-main.rs
    GraphicsCrates (registration, both bundles, the archive's shape):
      https://github.com/ellisnet/CodeBrix.WebPlatform.WebGpu/blob/main/tests/CodeBrix.WebPlatform.WebGpu.Graphics.Tests/GraphicsCratesTests.cs
    The GraphicsDemo sample (its "graphics" application has a scene for
    every area of the graphics crate, the desktop model viewer's scene
    first):
      https://github.com/ellisnet/CodeBrix.WebPlatform.WebGpu/tree/main/samples/GraphicsDemo
    The Graphics crate's names:
      https://github.com/ellisnet/CodeBrix.WebPlatform.WebGpu/blob/main/tests/CodeBrix.WebPlatform.WebGpu.Graphics.Tests/GraphicsRustCrateTests.cs
    The Animation crate's names:
      https://github.com/ellisnet/CodeBrix.WebPlatform.WebGpu/blob/main/tests/CodeBrix.WebPlatform.WebGpu.Animation.Tests/AnimationRustCrateTests.cs
    AnimationCrates (registration, both bundles, the archive's shape):
      https://github.com/ellisnet/CodeBrix.WebPlatform.WebGpu/blob/main/tests/CodeBrix.WebPlatform.WebGpu.Animation.Tests/AnimationCratesTests.cs
    A frontend that draws a session of shapes with easing, a colour ramp, a
    timeline and the playback controller through codebrix-webgpu-animation,
    and peniko brushes into the VectorScene directly, compiled by the tests:
      https://github.com/ellisnet/CodeBrix.WebPlatform.WebGpu/blob/main/tests/CodeBrix.WebPlatform.WebGpu.Animation.Tests/Materialize/frontend-main.rs
    A frontend that uses BOTH the graphics and the animation crates (a
    Graphics layer under an Animation layer), compiled by the tests:
      https://github.com/ellisnet/CodeBrix.WebPlatform.WebGpu/blob/main/tests/CodeBrix.WebPlatform.WebGpu.Animation.Tests/Materialize/frontend-both-main.rs
    The GraphicsDemo sample's "animation" application (the C# demo's scene
    catalogue with its keys, a scene for each area of the animation crate,
    SVG and Lottie included; the sample's MAINTAINER-README.txt lists them):
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
    OrbitController::new(orbit): input(&e) / update(time) / apply(&mut cam)
    let models = ctx.load_models(&bytes, &ModelLoadOptions::default())?;
        // bytes from the generated <name>_bytes call, or include_bytes!
    let inst = models[0].create_instance_at(world);
    let mut mr = ctx.create_model_renderer()?;
    mr.begin(&pass, &camera, Some(&lighting))?; mr.draw(&inst)?; mr.end(&mut pass)?
    let sky = ctx.create_procedural_sky(Some(&ProceduralSkyOptions { .. }), None)?;
    let env = ctx.create_environment_from_bytes(&hdr_bytes, None)?;  // .hdr
    SceneLighting { environment: Some(env.clone()), .. }   // None: default sky
    let mut skybox = ctx.create_skybox_renderer()?; skybox.draw(&mut pass,
        &camera, Some(&lighting))?            // after the opaque models
    let hdr = ctx.create_headless_target(w, h, HeadlessTargetOptions {
        format: Rgba16Float, .. })?;          // scene into hdr.view()
    let mut tone = ctx.create_tone_mapper()?; tone.operator = ..; tone.exposure
    tone.apply_in_pass(&mut canvas_pass, hdr.color())?  // canvas pass LOADS
    tone.preserve_alpha = true; hdr cleared to Color4::TRANSPARENT; no skybox
                                              // models on the page, HDR on
    inst.animation().play_named("walk", PlayOptions::default())?;
    inst.animation().cross_fade_named("run", 0.4, PlayOptions::default())?;
    inst.animation().update_to(frame.info.time);   // or .update(delta)
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
    session.set_opacity(a) / layer.set_opacity(a); session.render_with(.., &SessionRenderOptions { include_background, target })
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
    ImageSource::from_png(&png_bytes)?   // a PNG file decoded in the browser (PNG is the only file format)
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
    NoiseShape::new(x, y, w, h, kind, seed, freq, opts)?   // GPU: same character, not the same pattern; DrawingNoise::fill = exact (CPU, slow)
    session.draw_particles(&system, t, Some(softness))?; BrushStrokeShape::new(..)?.with_progress(p); OwnershipShape::new(&map, &colors, dest, opts)?
    session.take_reports()   // e.g. a matte layer name the session does not have; show beside the device's errors
    codebrix_webgpu_animation::{kurbo, peniko}   // re-exported; never name them yourself
    nothing reads a clock: time is always frame.info.time / frame.info.delta

  .NET MODEL DATA (CodeBrix.WebPlatform.WebGpu.Graphics.Models)
    var set = new ModelDataSet(); set.Models.Add(model); set.Textures.Add(..);
    byte[] blob = ModelDataWriter.Write(set);  // serve from a [WireFile] GET

  CRATE NAMES
    GraphicsRustCrate.CrateName    "codebrix-webgpu-graphics"
    AnimationRustCrate.CrateName   "codebrix-webgpu-animation"

================================================================================
END OF AGENT-README
================================================================================
