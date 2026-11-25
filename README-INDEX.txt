================================================================================
README-INDEX: CodeBrix.WebPlatform.WebGpu
Map of the README files in this repository
================================================================================

If you are an AI coding agent: find the NuGet package you are consuming below
and read its AGENT-README file in full. Read MAINTAINER-README.txt only if you
are changing this repository itself.

AGENT-README FILES (consumer documentation, one per NuGet package)
------------------------------------------------------------------
  AGENT-README.txt
      One file for every package of this repository:
      CodeBrix.WebPlatform.WebGpu.ApacheLicenseForever - the Rust wgpu,
      glam, png and zune-jpeg crates and the crate codebrix-webgpu (the
      WebGPU context, the canvas host with layers and one clock, the maths,
      PNG and JPEG decoding) for WebGPU in the browser, carried as one
      archive with their notices, and the one call a build program makes to
      register them as a crate bundle for a CodeBrix.WebPlatform frontend.
      CodeBrix.WebPlatform.WebGpu.Graphics.ApacheLicenseForever - 2D and 3D
      graphics for a platform frontend, through the Rust crate
      codebrix-webgpu-graphics (the counterpart of the .NET library
      CodeBrix.WebGpu.Graphics, drawing as a layer of the canvas host, with
      glTF loading, lights and shadows, culling and level of detail,
      anti-aliasing, effects and water), carried with the gltf crates it
      needs, and the one call that registers them.
      CodeBrix.WebPlatform.WebGpu.Animation.ApacheLicenseForever - real-time
      2D animation for a platform frontend, through the Rust crate
      codebrix-webgpu-animation (the names and concepts of the .NET library
      CodeBrix.Imaging.Drawing.NoSkia - a drawing session, shapes, text,
      the animation model, paths, pictures, masks and effects, pixel planes,
      painterly playback, easing, colour ramps, timelines - with SVG
      documents and Lottie animations, render-time looks and wipes,
      painterly stroke sets drawn on the GPU, and water, drawn by its own
      renderer as a layer of the canvas host), carried with the third-party crates it needs
      (kurbo, peniko, harfrust, i_overlay, usvg, velato and what they
      bring), and the one call that registers them.

MAINTAINER AND EXTRAS
---------------------
  MAINTAINER-README.txt
      Building, testing, packaging, versioning and provenance notes for
      maintainers.
  EXTRAS-README.txt
      Samples, tools and other non-package content in this repository.
  src/CodeBrix.WebPlatform.WebGpu/Rust/vendoring/wgpu/UPGRADE-README.txt
      What is particular to upgrading the wgpu crates.
  src/CodeBrix.WebPlatform.WebGpu/Rust/vendoring/glam/UPGRADE-README.txt
      What is particular to upgrading the glam crate.
  src/CodeBrix.WebPlatform.WebGpu/Rust/vendoring/png/UPGRADE-README.txt
      What is particular to upgrading png (PNG files) and its tree in the
      WebGpu crate set.
  src/CodeBrix.WebPlatform.WebGpu/Rust/vendoring/zune-jpeg/UPGRADE-README.txt
      What is particular to upgrading zune-jpeg (JPEG files) and zune-core.
  src/CodeBrix.WebPlatform.WebGpu.Graphics/Rust/vendoring/gltf/UPGRADE-README.txt
      What is particular to upgrading gltf (glTF documents) and the
      Graphics package's crate set.
  src/CodeBrix.WebPlatform.WebGpu.Animation/Rust/vendoring/kurbo-peniko/UPGRADE-README.txt
      What is particular to upgrading the Animation package's crate set:
      kurbo and peniko, and how the whole set is resolved and vendored.
  src/CodeBrix.WebPlatform.WebGpu.Animation/Rust/vendoring/harfrust/UPGRADE-README.txt
      What is particular to upgrading harfrust (text shaping) in that set.
  src/CodeBrix.WebPlatform.WebGpu.Animation/Rust/vendoring/i_overlay/UPGRADE-README.txt
      What is particular to upgrading i_overlay (path combining) in that set.
  src/CodeBrix.WebPlatform.WebGpu.Animation/Rust/vendoring/usvg/UPGRADE-README.txt
      What is particular to upgrading usvg (SVG documents, with its text feature) in that set.
  src/CodeBrix.WebPlatform.WebGpu.Animation/Rust/vendoring/velato/UPGRADE-README.txt
      What is particular to upgrading velato (Lottie documents) in that set.
  tests/CodeBrix.WebPlatform.WebGpu.Graphics.Tests.GltfReference/Models/README.txt
      The rule for the test model packs of the glTF reference converter
      (CC0 or CC-BY only, each with its licence file and SOURCE.txt).
  samples/GraphicsDemo/MAINTAINER-README.txt
      The GraphicsDemo sample: building, running and checking it in a
      browser.
  samples/GraphicsDemo/THIRD-PARTY-NOTICES.txt
      The model packs, the HDR panorama and the fonts the GraphicsDemo
      sample's backend carries: source, licence and files of each (the site also serves it, as
      /SAMPLE-THIRD-PARTY-NOTICES.txt).

GENERAL
-------
  README.md
      Human-facing overview shown on GitHub and nuget.org.
  README-INDEX.txt
      This file.
  THIRD-PARTY-NOTICES.txt
      What came from where, and under which licences, for the packages;
      it points to the generated notices below.
  src/CodeBrix.WebPlatform.WebGpu/Rust/notices/THIRD-PARTY-NOTICES-WebGpuCrates.txt
      The third-party crates the WebGpu package carries (generated; packed
      into that package).
  src/CodeBrix.WebPlatform.WebGpu.Animation/Rust/notices/THIRD-PARTY-NOTICES-WebGpuAnimationCrates.txt
      The third-party crates the Animation package carries and the outside
      sources of its crate's renderer (a hand-written header and a generated
      part; packed into that package).
  src/CodeBrix.WebPlatform.WebGpu.Graphics/Rust/notices/THIRD-PARTY-NOTICES-WebGpuGraphicsCrates.txt
      The third-party crates the Graphics package carries and the outside
      sources of parts of its crate (a hand-written header and a generated
      part; packed into that package).

================================================================================
