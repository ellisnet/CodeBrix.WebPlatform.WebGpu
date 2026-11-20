================================================================================
EXTRAS-README: CodeBrix.WebPlatform.WebGpu
Samples, tools and other content in this repository that is not part of a NuGet
package
================================================================================

The packable projects of this repository are src/CodeBrix.WebPlatform.WebGpu,
src/CodeBrix.WebPlatform.WebGpu.Graphics and
src/CodeBrix.WebPlatform.WebGpu.Animation. Everything listed below exists to
build, check, test or document them. None of it is included in a package.

For runnable, compilable usage of the libraries, read the test projects: the
"WORKING EXAMPLES ON GITHUB" section of AGENT-README.txt maps each feature
area to the test file that exercises it.


TEST PROJECTS
=============
    tests/CodeBrix.WebPlatform.WebGpu.Tests/
    tests/CodeBrix.WebPlatform.WebGpu.Graphics.Tests/
    tests/CodeBrix.WebPlatform.WebGpu.Animation.Tests/
    tests/CodeBrix.WebPlatform.WebGpu.LibraryChecker.Tests/

xUnit v3; run them as described in MAINTAINER-README.txt (TESTING).


THE MAINTAINER TOOL
===================
    libs/CodeBrix.WebPlatform.WebGpu.LibraryChecker/

The licence and vendoring gate of this repository (assembly name
LibraryChecker). It checks the vendored crates against what crates.io
published, against the platform's crates (the strict rule for crate bundles)
and against the licence allowlist in tools/licensing/license-allowlist.json,
and it writes the generated notices. It is no package. See
MAINTAINER-README.txt (THE MAINTAINER TOOL).


THE SAMPLE
==========
    samples/GraphicsDemo/

A CodeBrix.WebPlatform application that uses the WebGpu, Graphics and
Animation packages the way any application does: a small backend, a build
program that registers the crate bundles, and an assets project
(GraphicsDemo.Assets) whose resources the backend serves - the desktop model
viewer's glTF models and a skinned character converted in .NET into model
data, a morph-target shape built in code, one HDR panorama as its Radiance
.hdr file, two fonts, a picture both as decoded pixels (the picture-pixels
form) and as its PNG file, a caption file, SVG documents and Lottie files.
It has two named frontend applications:

  - "graphics", with a scene for every area of the graphics crate: "Viewer"
    (the first, shown when the page opens: the desktop model viewer's
    complete scene - its ground and props, and its animated actors, the
    chest opening and closing, the animals walking or cross-fading through
    their routine and the bird circling overhead, played on the page's
    clock - drawn with physically based shading under its orbit camera, lit
    by image-based light from a procedural sky or the HDR panorama, with
    the sky drawn behind them, rendered through an HDR target and tone
    mapped, with the desktop viewer's switches as the query parameters ?ldr
    and ?no-skybox and controls for the tone-map operator and the
    exposure), "Layers" (a triangle drawn with wgpu every frame, turned and
    scaled with the pointer, the wheel and the keys, and over it, as a
    second independent layer on the same clock, a translucent or solid
    square; the clock pauses and resumes both, and both react to one named
    point in time), "Shaders" (every built-in shader of
    codebrix-webgpu-graphics compiled by the browser, with the compiler's
    messages), "Textured quad", "Sprites" (the sprite batch), "Lit shapes"
    (lit primitives under an orbit camera driven by drag, wheel and touch),
    "Skinned model", "Morph targets", "More" (a panorama made in code, a
    render bundle, particles moved by a compute pass and drawn by an
    indirect draw, two animated textures, GPU timer results) and "On the
    page" (models drawn on a transparent canvas over the page);
  - "animation", the scene catalogue of the desktop AnimationDemoApp (its
    scenes ported, then scenes of the sample's own for SVG documents that
    animate themselves and for Lottie), drawn by codebrix-webgpu-animation
    on the page's clock, with its keys - Space pauses the clock, Left and
    Right seek, Shift with Left or Right moves between scenes, Home starts
    again, D shows the debug overlay - and a page of its own for drawing on
    the page (?scene=on-the-page).

A plain page of the developer's own at the site root links to both and to
the site's two notices files. Without WebGPU each application shows "WebGPU
is not available in this browser". The sample has its own solution
(GraphicsDemo.slnx, not in the root solution) and its own
MAINTAINER-README.txt, which lists every scene.


THE RUST MATERIAL OF THE WebGpu LIBRARY
=======================================
    src/CodeBrix.WebPlatform.WebGpu/Rust/

The crate set (webgpu-crates/Cargo.toml and its Cargo.lock), the vendored
crates (vendor/), the first-party crate codebrix-webgpu (codebrix-webgpu/),
the notes on upgrading the vendored crates (vendoring/) and the notices
(notices/). Only the archive made from vendor/ and codebrix-webgpu/ and the
generated notices travel in the package; the rest is repository material. See
MAINTAINER-README.txt (VENDORED SNAPSHOTS).


THE RUST MATERIAL OF THE Graphics LIBRARY
=========================================
    src/CodeBrix.WebPlatform.WebGpu.Graphics/Rust/

The first-party crate codebrix-webgpu-graphics (codebrix-webgpu-graphics/,
its WGSL under src/shaders/wgsl/) and its hand-written notices (notices/).
The archive made from the crate and the notices travel in the package. See
MAINTAINER-README.txt (THE GRAPHICS CRATE).


THE RUST MATERIAL OF THE Animation LIBRARY
==========================================
    src/CodeBrix.WebPlatform.WebGpu.Animation/Rust/

The first-party crate codebrix-webgpu-animation (codebrix-webgpu-animation/,
its WGSL under src/rendering/core/), the crate set of the third-party crates
only it needs (animation-crates/Cargo.toml and its Cargo.lock), those crates
(vendor/), the notes on upgrading them (vendoring/) and the notices (notices/:
the hand-written header and the generated file). Only the archive made from
vendor/ and codebrix-webgpu-animation/ and the generated notices travel in
the package. See MAINTAINER-README.txt (THE ANIMATION CRATE).
