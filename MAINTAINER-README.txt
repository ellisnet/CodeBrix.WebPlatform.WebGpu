================================================================================
MAINTAINER-README: CodeBrix.WebPlatform.WebGpu
Notes for people and agents MAINTAINING this repository - not for package
consumers
================================================================================

If you are CONSUMING the NuGet packages, stop reading and open AGENT-README.txt
instead. Everything below is about the repository itself: how it is laid out,
how it builds, how it is tested, how it is packaged, how the third-party code
it carries is kept and checked, and the conventions the source follows.

This repository follows the model of CodeBrix.WebPlatform (its own
MAINTAINER-README.txt is the fuller account of crate bundles, vendoring and
the licence gate); what is particular to this repository is written here.


PURPOSE AND SCOPE
=================
This repository produces three NuGet packages, each a package that brings a
crate bundle to a CodeBrix.WebPlatform frontend:

    PackageId:  CodeBrix.WebPlatform.WebGpu.ApacheLicenseForever
    Project:    src/CodeBrix.WebPlatform.WebGpu/
                CodeBrix.WebPlatform.WebGpu.csproj
    Carries:    the vendored crates (wgpu, glam, png and zune-jpeg and their
                resolution) and the first-party crate codebrix-webgpu, in
                one archive, and the generated notices of the vendored
                crates; WebGpuCrates.Register() registers them.

    PackageId:  CodeBrix.WebPlatform.WebGpu.Graphics.ApacheLicenseForever
    Project:    src/CodeBrix.WebPlatform.WebGpu.Graphics/
                CodeBrix.WebPlatform.WebGpu.Graphics.csproj
    Carries:    the first-party crate codebrix-webgpu-graphics and the
                third-party crates ONLY it needs (gltf, gltf-json,
                gltf-derive, lazy_static and inflections: what gltf brings
                beyond the platform's and the WebGpu package's crates) in
                its own archive, and their notices (a hand-written header
                and a generated part);
                GraphicsCrates.Register() registers the WebGpu package's
                bundle first, then its own.

    PackageId:  CodeBrix.WebPlatform.WebGpu.Animation.ApacheLicenseForever
    Project:    src/CodeBrix.WebPlatform.WebGpu.Animation/
                CodeBrix.WebPlatform.WebGpu.Animation.csproj
    Carries:    the first-party crate codebrix-webgpu-animation and the
                third-party crates ONLY it needs (kurbo, peniko, harfrust,
                i_overlay, usvg, velato and what they bring beyond the
                platform's and the WebGpu package's crates) in its own archive, and their notices (a
                hand-written header and a generated part);
                AnimationCrates.Register() registers the WebGpu package's
                bundle first, then its own.

Assembly names and root namespaces are the package ids without the
".ApacheLicenseForever" suffix. License: Apache-2.0 (LICENSE at the root) for
the code written here; everything third-party keeps its own licence (see
NOTICES). Consumer documentation: AGENT-README.txt, ONE file shared by the
three packages and packed into each.

THE DEPENDENCY RULE (the owner's): WebGpu -> Core (exact pin). Graphics ->
Core (exact pin) + WebGpu (exact pin). Animation -> Core (exact pin) + WebGpu
(exact pin). Never Graphics <-> Animation, in the projects or in the crates
(the gate's "dependency-rule" group checks both). They do NOT carry the wgpu
crates: Graphics' and Animation's Register() call WebGpuCrates.Register()
first and add only their own bundles (codebrix-webgpu-graphics and
codebrix-webgpu-animation, each with the third-party crates only it needs -
never a crate the WebGpu package or the platform carries). Everything both
of them need (the context, the canvas host, layers, the clock, the maths, PNG
and JPEG decoding) is in codebrix-webgpu; only the APPLICATION puts their
renderers together.

THE FAMILY RULE (the owner's): the three packages (WebGpu, Graphics,
Animation) are built and published together from ONE build of the root
solution, with one version, and each pins the packages it depends on
EXACTLY: all three pin Core as Version="[x]", and Graphics and Animation pin
the WebGpu package of the same build as "[x]". Graphics and Animation reach
WebGpu through a ProjectReference, which NuGet would normally pack as a
minimum (version="x", meaning ">= x"). The shared file
src/CodeBrix.WebPlatform.WebGpu/ExactProjectReferencePins.targets, imported
by both projects, runs right after NuGet's _GetProjectReferenceVersions
target and rewrites the ProjectVersion metadata of
_ProjectReferencesWithVersions as "[x]" before the nuspec is generated, so
the nuspec reads version="[1.0.y.z]". The gate's "platform" group checks that
the target is there and that both projects import it. The pinned version is
the WebGpu project's own, evaluated from the clock in the same build: build
EARLY IN A UTC MINUTE and check the Release nuspecs before publishing (a
build that crosses a minute could pin a WebGpu version one minute off).


THE RULES OF THE PACKAGES
=========================
  - CORE, EXACT PIN. Each library references
    CodeBrix.WebPlatform.Core.ApacheLicenseForever at an EXACT version
    (Version="[x.y.z.w]"): its crates are chosen against one platform
    release. Graphics and Animation also reference the WebGpu project,
    packed as an exact pin (THE DEPENDENCY RULE and THE FAMILY RULE above),
    and nothing else. Never a reference to
    FrontendBuilder from a library.
  - THE COUPLING. Family versions are date stamps, and the pin is exact:
    EVERY REPUBLISH OF CORE FORCES A REPUBLISH OF THESE PACKAGES, rebuilt
    against that exact Core (and the WebGpu crate set re-resolved against
    that release's crates), or an application that updates the platform
    gets a NuGet restore error until they are republished.
  - THE STRICT RULE is checked HERE, so that a consumer never meets it:
    every vendored crate the platform's archive also holds is at the
    platform's version with the same content; none is at a version the
    platform does not carry (LibraryChecker, group "platform").
  - WebGPU ONLY. The crate set turns on no WebGL and no native backend.
  - LICENCE RULE. Everything a developer clones from this repository is
    available under Apache-2.0 or a more permissive licence (MIT, BSD, CC0,
    ISC; Zlib is approved for this repository by the owner). The allowlist
    in tools/licensing/license-allowlist.json is the owner's file: only he
    edits it.
  - NOTHING IS DOWNLOADED at build or test time (nuget.org restore
    excepted). No Node.js, no JavaScript of ours, no shell
    scripts, no GitHub workflow files - in vendored trees either (report one
    found in a crate; never delete a file from a vendored crate).


REPOSITORY LAYOUT
=================
    CodeBrix.WebPlatform.WebGpu.slnx
        The solution: the three libraries, the maintainer tool and, in the
        Tests folder, every test project and the test-side libraries they
        use (the materialize tests' sample backend, the glTF reference
        converter). Its "Solution Items" are the
        canonical files: .gitignore, AGENT-README.txt, EXTRAS-README.txt,
        global.json, icon-codebrix-128.png, LICENSE, MAINTAINER-README.txt,
        README-INDEX.txt, README.md, THIRD-PARTY-NOTICES.txt.
    global.json
        Pins the SDK (the family's current standard, with latestFeature
        roll-forward and no prerelease SDKs) and selects the
        Microsoft.Testing.Platform test runner. Copied from
        CodeBrix.WebPlatform; keep the two the same.
    .gitattributes, .gitignore
        See GIT AND THE VENDORED FOLDER.
    icon-codebrix-128.png
        The package icon.
    AGENTS.md, CLAUDE.md, .clinerules, .cursorrules, .windsurfrules,
    .cursor/rules/agent-readme.mdc, .github/copilot-instructions.md,
    .junie/guidelines.md
        The family's AI-agent pointer files, byte for byte the family's
        canonical copies. They only point at README-INDEX.txt; never edit
        them per repository. (.github holds that one pointer file and no
        workflow.)
    README.md, AGENT-README.txt, MAINTAINER-README.txt, EXTRAS-README.txt,
    README-INDEX.txt, THIRD-PARTY-NOTICES.txt
        The document set (see CODING CONVENTIONS, Documentation).
    src/CodeBrix.WebPlatform.WebGpu/
        WebGpuCrates.cs           the registration call, the paths, the
                                  notices
        InternalsVisibleTo.cs     names the test project
        buildTransitive/CodeBrix.WebPlatform.WebGpu.ApacheLicenseForever
        .targets                  copies webgpu-crates/ into a consuming
                                  project's output folder
        Rust/webgpu-crates/       THE CRATE SET: Cargo.toml (names wgpu,
                                  glam, png and zune-jpeg and their
                                  features; never built into anything) and
                                  Cargo.lock (the pins)
        Rust/vendor/              the vendored crates, one name-version
                                  folder per crate, exactly as crates.io
                                  published them
        Rust/codebrix-webgpu/     THE FIRST-PARTY CRATE (Cargo.toml, src/,
                                  the JPEG test files under fixtures/):
                                  ours, carried in the archive in vendored
                                  form (see THE FIRST-PARTY CRATE)
        WebGpuCratesArchive.targets
                                  the archive writer (an inline MSBuild task;
                                  never packed)
        Rust/vendoring/wgpu/, Rust/vendoring/glam/, Rust/vendoring/png/,
        Rust/vendoring/zune-jpeg/
                                  UPGRADE-README.txt: what is particular to
                                  upgrading each library chosen by name
        Rust/notices/             notices-header-WebGpuCrates.txt (hand-
                                  written) and THIRD-PARTY-NOTICES-
                                  WebGpuCrates.txt (generated by the gate)
    src/CodeBrix.WebPlatform.WebGpu/ExactProjectReferencePins.targets
                                  the exact WebGpu pin of Graphics and
                                  Animation (THE FAMILY RULE); never packed
    src/CodeBrix.WebPlatform.WebGpu.Graphics/
        GraphicsCrates.cs         the registration call (WebGpu's first),
                                  the paths, the notices
        GraphicsRustCrate.cs      the crate's names
        buildTransitive/...targets  copies webgpu-graphics-crates/ into a
                                  consuming project's output folder
        Rust/codebrix-webgpu-graphics/
                                  THE GRAPHICS CRATE (Cargo.toml, src/, the
                                  WGSL under src/shaders/wgsl/, the test
                                  fixtures under fixtures/); see THE
                                  GRAPHICS CRATE
        Rust/graphics-crates/     THE GRAPHICS CRATE SET: Cargo.toml (names
                                  gltf and its features; never built into
                                  anything) and Cargo.lock (resolved against
                                  the platform's and the WebGpu set's locks)
        Rust/vendor/              the third-party crates ONLY this package
                                  carries (the glTF crates), exactly as
                                  crates.io published them (see VENDORED
                                  SNAPSHOTS)
        Rust/vendoring/gltf/UPGRADE-README.txt
                                  what is particular to upgrading gltf and
                                  the set
        Rust/notices/             notices-header-WebGpuGraphicsCrates.txt
                                  (hand-written: the crate is ours; the
                                  three tone-mapping sources of its shaders
                                  and the .hdr decoder's; the glTF crates'
                                  part) and THIRD-PARTY-NOTICES-
                                  WebGpuGraphicsCrates.txt (generated by the
                                  gate)
        The archive is written by the WebGpu project's
        WebGpuCratesArchive.targets (the ONE place the writer lives; the
        Graphics project imports it) and holds the vendored crates of
        Rust/vendor and codebrix-webgpu-graphics-<version>/.
    src/CodeBrix.WebPlatform.WebGpu.Animation/
        AnimationCrates.cs        the registration call (WebGpu's first),
                                  the paths, the notices
        AnimationRustCrate.cs     the crate's names
        buildTransitive/...targets  copies webgpu-animation-crates/ into a
                                  consuming project's output folder
        Rust/codebrix-webgpu-animation/
                                  THE ANIMATION CRATE (Cargo.toml, src/, the
                                  WGSL under src/rendering/core/); see THE
                                  ANIMATION CRATE
        Rust/animation-crates/    THE ANIMATION CRATE SET: Cargo.toml (names
                                  kurbo, peniko, harfrust, i_overlay, usvg, svgtypes and velato; never built into
                                  anything) and Cargo.lock (resolved against
                                  the platform's and the WebGpu set's locks)
        Rust/vendor/              the third-party crates ONLY this package
                                  carries, exactly as crates.io published
                                  them (see VENDORED SNAPSHOTS)
        Rust/vendoring/kurbo-peniko/UPGRADE-README.txt
                                  what is particular to upgrading the set
        Rust/vendoring/harfrust/UPGRADE-README.txt
                                  what is particular to harfrust (text)
        Rust/vendoring/i_overlay/UPGRADE-README.txt
                                  what is particular to i_overlay (path
                                  combining)
        Rust/vendoring/usvg/UPGRADE-README.txt
                                  what is particular to usvg (SVG
                                  documents, its text feature)
        Rust/vendoring/velato/UPGRADE-README.txt
                                  what is particular to velato (Lottie)
        Rust/notices/             notices-header-WebGpuAnimationCrates.txt
                                  (hand-written: the vello and Skia material
                                  of the crate) and THIRD-PARTY-NOTICES-
                                  WebGpuAnimationCrates.txt (generated by the
                                  gate)
        The archive is written by the WebGpu project's
        WebGpuCratesArchive.targets and holds the vendored crates of
        Rust/vendor and codebrix-webgpu-animation-<version>/.
    libs/CodeBrix.WebPlatform.WebGpu.LibraryChecker/
        The maintainer tool: the licence and vendoring gate. Not packable.
    tests/<project>.Tests/
        One test project per project above.
    tests/CodeBrix.WebPlatform.WebGpu.Tests.SampleBackend/
        The fixture backend the materialize tests read (never run).
    tests/CodeBrix.WebPlatform.WebGpu.Graphics.Tests.GltfReference/
        THE REFERENCE CONVERTER (never packed): the C# glTF-to-model-data
        converter the Graphics crate's glTF loader is checked against, and
        its test model packs under Models/ (TESTING, THE glTF REFERENCE).
    tools/licensing/license-allowlist.json
        The approved licences (the owner's file).
    samples/
        Sample applications, each with its own .slnx, NOT in the root
        solution.
    samples/GraphicsDemo/
        A CodeBrix.WebPlatform application with two named frontend
        applications ("graphics" at /graphics, "animation" at /animation)
        and the developer's own page at "/". It consumes this repository's
        three packages as PACKAGES from nuget.org, never as
        projects, and its build program registers the Graphics and the
        Animation bundles (and through them the WebGpu one, once); both
        applications draw with codebrix-webgpu; animation shows the desktop
        AnimationDemoApp's scene catalogue as far as the animation crate
        reaches, drawn by codebrix-webgpu-animation, and graphics shows the
        desktop model viewer's
        complete scene (the default, its models served as their glTF files
        and read by the crate's loader) and, on buttons, a scene for each
        further feature of codebrix-webgpu-graphics that the viewer does not
        show (its MAINTAINER-README lists them). It
        has its own MAINTAINER-README.txt (building, running, checking it in
        a browser, what is build output).


BUILDING
========
From the repository root:

    dotnet restore CodeBrix.WebPlatform.WebGpu.slnx
    dotnet build   CodeBrix.WebPlatform.WebGpu.slnx

Target framework: net10.0 only. 0 warnings and 0 errors in Debug and Release.
GenerateDocumentationFile is ON in every library, so every public member
carries an XML doc comment; fix CS1591 by writing the comment, never with
<NoWarn>.

THE CRATES ARCHIVE. Rust/vendor and Rust/codebrix-webgpu are NOT embedded
and NOT packed as loose files. The target _CreateWebGpuCratesArchive (in the
WebGpu project file) writes obj/webgpu-crates/webgpu-crates.zip (the
project's intermediate folder; under the artifacts path when a build passes
--artifacts-path) with the task WriteWebGpuCratesArchive of
WebGpuCratesArchive.targets - incrementally: only when a file of
Rust/vendor or Rust/codebrix-webgpu, the crate set's Cargo.lock, the project
file or the task's file is newer than the archive. (Removing a crate makes
no file newer; the lock file changes with every change of the set, which is
why it is an input.) The archive holds the name-version folders at its top
and nothing else: every vendored crate byte for byte, and codebrix-webgpu in
VENDORED FORM - the folder codebrix-webgpu-<version> (name and version from
its Cargo.toml), its files except target/, .cargo/ and Cargo.lock, and a
.cargo-checksum.json GENERATED at that moment ({"files":{path:sha256},
"package":null}, what cargo's directory sources need). THE ARCHIVE IS
DETERMINISTIC: entries in ordinal path order, one fixed time stamp, no file
permissions, so the same files give the same bytes on any machine and
checkout (the platform's build compares the archive's hash to decide whether
to rewrite an application's vendor folder). WHY AN INLINE TASK: MSBuild's
ZipDirectory records file times and the file system's order and cannot add a
generated file; the inline task (RoslynCodeTaskFactory, part of MSBuild, so
no package and no script) can. It compiles against .NET Standard 2.0, so it
uses only what that offers. A committed checksum file was the alternative;
generating it at build time means it can never be stale. With no
Rust/vendor folder the archive holds codebrix-webgpu alone, so that the
project still builds; the tests then skip what needs the vendored crates.
The archive and
Rust/notices/THIRD-PARTY-NOTICES-WebGpuCrates.txt are None items that are
packed under tools/webgpu-crates/ and copied to the output folder as
webgpu-crates/ (so a project reference - the tests - gets them as well); in
a consuming project, the package's buildTransitive targets file does that
copy. The notices item is taken only when the file exists (the gate writes
it). WebGpuCrates looks for
<application base folder>/webgpu-crates/webgpu-crates.zip: that folder name
is a contract. Rust/vendor and Rust/webgpu-crates/target are in
DefaultItemExcludes, and nothing under Rust/ is a project item.

THE GRAPHICS AND ANIMATION ARCHIVES are written the same way, by the same
task (the Graphics and Animation projects import WebGpuCratesArchive.targets):
obj/webgpu-graphics-crates/webgpu-graphics-crates.zip holds the crates of the
Graphics library's Rust/vendor byte for byte and
codebrix-webgpu-graphics-<version>/ in vendored form;
obj/webgpu-animation-crates/webgpu-animation-crates.zip holds the crates of
the Animation library's Rust/vendor byte for byte and
codebrix-webgpu-animation-<version>/ in vendored form. Each is packed with
its notices under tools/webgpu-graphics-crates/ and
tools/webgpu-animation-crates/, and copied beside a consuming program by that
package's buildTransitive targets.

Building the libraries does NOT compile any Rust.


TESTING
=======
    dotnet test --solution CodeBrix.WebPlatform.WebGpu.slnx

or one project:

    dotnet test --project tests/CodeBrix.WebPlatform.WebGpu.Tests/CodeBrix.WebPlatform.WebGpu.Tests.csproj

THE TEST RUNNER IS Microsoft.Testing.Platform, selected by global.json at the
repo root. Do not delete the "test" section: without it `dotnet test` falls
back to the VSTest bridge, which xUnit v3 no longer supports on this SDK. The
output ends in a "Test run summary:" block. If `dotnet test --solution` ever
reports zero tests, run a test assembly directly and read its counts.

The test projects are xUnit v3 with SilverAssertions. No coverage collector is
referenced. No network is needed.

WebGpu.Tests covers the registration (the bundle's name, package and version,
and that registering again changes nothing), the paths, the messages for a
missing archive and missing notices, and the archive: only name-version
folders at the top, each with its Cargo.toml and .cargo-checksum.json, the
wgpu crates among them; the archive is the vendor folder byte for byte plus
codebrix-webgpu and nothing else; codebrix-webgpu is its source without
build output or lock file, with a checksum file that lists every other file
with its sha256 and no package checksum; the entries are in path order with
one time stamp. The tests that need vendored crates are SKIPPED, with the
reason, when there is no Rust/vendor; the tests that compare with the source
find the repository from their own source file's folder.

THE REAL-MATERIALIZE TESTS (WebGpu.Tests, folder Materialize/) run the
platform's own PlatformFrontend.MaterializeAsync, as a build program does,
with WebGpuCrates.Register() called, against the REAL platform archive and
this package's real archive. The project references
CodeBrix.WebPlatform.FrontendBuilder.ApacheLicenseForever at the Core pin's
version (its buildTransitive targets put the platform's archive beside the
test assembly) and the fixture backend
tests/CodeBrix.WebPlatform.WebGpu.Tests.SampleBackend (one small controller,
read and never run). The crate set's Cargo.toml is copied beside the test
assembly, and the test frontend names wgpu with exactly the crate set's
`wgpu = ...` line, so a change of the crate set is tested without editing a
test. ONE frontend is materialized in a temporary folder for the whole
collection (MaterializedFrontend); the tests then check:
  - MaterializeResult.CrateBundles reports the "webgpu" bundle with this
    package's id and version, and only the registered bundles;
  - its AddedCrates and SharedCrates partition the archive's crates, wgpu,
    wgpu-types and naga-types are added, every shared crate is one the
    platform's archive holds, and no added crate's name is one the platform
    carries (the strict rule, passed for real);
  - the vendor folder holds wgpu's folder with its .cargo-checksum.json and
    exactly one name-version folder per crate of the two archives, and the
    cargo configuration points at it and keeps cargo offline;
  - registering again changes nothing, and a second Materialize of the
    unchanged set does not rewrite the vendor folder (the stamp);
  - codebrix-webgpu is among the added crates, with its checksum file in the
    vendor folder;
  - `cargo metadata --offline` resolves the frontend from the vendor folder
    alone, codebrix-webgpu included, with glam resolved to bytemuck only
    (none of glam's other optional dependencies is needed - or carried);
  - `cargo check --offline --target wasm32-unknown-unknown` compiles the
    whole test frontend (Materialize/frontend-main.rs, which uses the public
    API of codebrix-webgpu: two layers, one clock, a named point, the maths)
    with no warning from the crate or the frontend;
  - `cargo test --offline` runs the crate's own unit tests ON THE HOST, from
    a copy of the codebrix-webgpu folder of the materialized vendor folder
    (what ships), resolved from that vendor folder.
    The cargo tests are SKIPPED, with the reason, on a machine without cargo
    (or, for the check, without the wasm32 target); nothing needs the
    network, dx, a browser or a container.
COSTS: the one Materialize unpacks both archives (a whole vendor folder) into
the temporary folder; the cargo check (a cold check of the frontend for
wasm32) and the cargo test (a cold host build of the crate's dependencies)
are by far the slowest tests; the cargo metadata run and every other test
take a moment. The whole project stays well under a minute on a
developer machine. The
crate-bundle registry is process-wide: the only bundle any test in the
project registers is this package's own, which is what keeps the tests
independent of their order.

GRAPHICS.TESTS has the same shape for the Graphics package: the registration
(both bundles, WebGpu's first, registering again changes nothing), the paths
and messages, the archive's shape (exactly the Graphics library's Rust/vendor
byte for byte - the glTF crates - plus codebrix-webgpu-graphics-<version>/,
its source without build output or lock file, a checksum file listing every
other file with its sha256 and package null, path order, one time stamp,
none of the WebGpu archive's crates), the
dependency rule and the exact pins, and a REAL Materialize (folder
Materialize/) of a frontend that names codebrix-webgpu-graphics: cargo
metadata resolves it offline, `cargo check --offline --target
wasm32-unknown-unknown` compiles Materialize/frontend-main.rs (a layer that
draws through the GraphicsContext, its resources and its pipeline builder,
and the compile of every built-in shader, a second layer that draws a
sprite batch over lit meshes with a depth texture, under a camera driven by
the orbit controller from the canvas's input, a third that loads models
from model-data bytes and draws instances of them with the model renderer,
and a fourth on the HDR path: an environment from the procedural sky or from
.hdr bytes, image-based lighting, the skybox, an RGBA16Float headless target
and the tone mapper; and animation playback on the model instances - play,
cross-fade and blend, the frame-time and frame-delta updates, a pose edit,
channel sampling; and a fifth layer with the rest - a compute pass that
writes indirect arguments, an indirect dispatch, a render bundle, an indirect
draw, pass timestamps and the GPU timer, an animated texture drawn by the
sprite batch, a query resolve and a buffer clear - with the async checks of
buffer mapping, texture regions, copies and readback, the headless target's
reads, the awaited compute pipeline and bundle, and the context options with
optional_features; and the additions beyond the C# library, each in a block
of its own, marked with BEGIN / END comments or in the layer it extends:
the glTF loader's two steps and a second model sharing a set's textures,
textures from PNG and JPEG bytes put into a loaded model's materials, the
model facts, the per-instance look and node visibility, culling and its
statistics, local lights and shadows, the frame timer, the glow layer,
bloom and particles, multisampling on a canvas frame target and an HDR
target, level of detail, the wave field and open-water waves, and the water
surface)
with no warning from the crate or the frontend, and `cargo test --offline` runs the graphics crate's unit
tests on the host from the copy in the materialized vendor folder. The same
skip rule as WebGpu.Tests. A new feature of the crate gets its block in
frontend-main.rs, so that the wasm32 check compiles real use of it.
THE glTF REFERENCE. The same materialize test writes a reference folder
first (GltfReferenceFolder.Write of the test-side library
tests/CodeBrix.WebPlatform.WebGpu.Graphics.Tests.GltfReference: cases.txt,
the model files of every pack, and one ModelDataWriter blob per case made by
THE REFERENCE CONVERTER - the C# glTF-to-model-data converter on the
CodeBrix.Graphics3D.Gltf2 and CodeBrix.Imaging packages, which follows the
desktop library's loader step for step) and runs the crate's tests with
CODEBRIX_GRAPHICS_TEST_GLTF_REFERENCE=<folder>: src/gltf/reference_tests.rs
loads every model with the Rust loader and compares it with the blob, value
by value - exactly, bit for bit (counts, indices, names, vertices with read
or generated normals and tangents, skins, morph deltas, node matrices,
animation keys, materials, samplers, PNG pixels, the warnings word for
word), with a tolerance only for JPEG pixels (two different decoders).
Without the variable those tests print SKIPPED and pass. The packs: the
GraphicsDemo sample's Models/** (LINKED by the library's project file: move
or rename the sample's model folder and change the link) and the library's
own Models/<pack>/ - Khronos glTF Sample Assets, CC0-1.0 and CC-BY-4.0
only, each pack with SOURCE.txt (commit, date, per model its licence and
attribution), each model with its LICENSE.md and metadata.json, and the
licence texts under LICENSES/ (Models/README.txt is the rule for adding a
pack; .gitattributes marks the packs -text). The Draco variants are kept
to test the refusal. The converter's own tests are in
tests/CodeBrix.WebPlatform.WebGpu.Graphics.Tests/GltfReference/. To write a
reference folder by hand:
    CODEBRIX_GLTF_REFERENCE_FOLDER=<folder> dotnet test --project tests/CodeBrix.WebPlatform.WebGpu.Graphics.Tests/CodeBrix.WebPlatform.WebGpu.Graphics.Tests.csproj --filter-method "CodeBrix.WebPlatform.WebGpu.Graphics.Tests.GltfReference.GltfReferenceFolderTests.Write_to_the_folder_named_by_the_environment_variable"
GRAPHICS.TESTS ALSO TESTS THE MODEL-DATA WRITER (folder Models/): golden
bytes of a tiny model, every section, the 16-byte alignment of every section
and every range, every check of the writer and its message, and a round trip
through a small reader that lives in the TEST project only (the library has
no reader: the Rust crate reads). THE FIXTURES: ModelDataFixturesTests
writes three model sets (ModelDataFixtures.cs describes every value they
hold) and compares them byte for byte with the files in the crate's
fixtures/ folder, which the crate's own unit tests read - so the writer and
the Rust reader cannot drift apart. After a deliberate change of the format
or of a fixture, regenerate them by running that test class with the
environment variable CODEBRIX_REGENERATE_FIXTURES=1, then update the Rust
tests (src/models/fixture_tests.rs) to the new values. The fixtures travel in
the crate's archive (they are small), so the materialized crate's own tests
read the same files.

ANIMATION.TESTS has the same shape for the Animation package: the
registration (both bundles, WebGpu's first, registering again changes
nothing), the paths and messages (the notices name the vello and Skia
material, the kurbo and peniko crates, harfrust, i_overlay, usvg and velato), the archive's shape (exactly the
Animation library's Rust/vendor byte for byte plus
codebrix-webgpu-animation-<version>/ in vendored form, a checksum file
listing every other file with its sha256 and package null, path order, one
time stamp, none of the WebGpu archive's crates), the dependency rule, and
TWO real Materialize runs (folder Materialize/), one after the other in one
fixture because the bundle registry is process-wide:
  - with AnimationCrates.Register() only: the bundles are webgpu then
    webgpu-animation, the Animation bundle adds its crate and its own
    third-party crates and shares nothing, the vendor folder holds one
    folder per crate of the three archives, `cargo metadata --offline`
    resolves codebrix-webgpu-animation with kurbo, peniko, harfrust, i_overlay, usvg and velato from the vendor
    folder and no Graphics crate, `cargo check --offline --target
    wasm32-unknown-unknown` compiles Materialize/frontend-main.rs (a session
    of every basic shape with a gradient fill and a dashed path, easing,
    a colour ramp, a timeline with a marker at a named point, the playback
    controller, text - a registry fed with font bytes, a line with a
    gradient fill, the missing-character choice and report, a measured
    width, the debug overlay, and a session drawn without its background
    into part of the canvas - and a second AnimationLayer drawing peniko brushes and kurbo
    shapes into the VectorScene directly - a sweep gradient, a group with a
    mix mode, a clip - and, in marked blocks, every API added since: from_jpeg,
    a const CubicBezierEasing, the RenderStats timings and counters,
    register_font_with_variations / get_variation_axes, fixed-width strokes on
    a path, a text block's outline and an SVG with vector-effect, the sketch
    helper boiling, OwnershipShape::with_colors, the render options' paint,
    effects, authored layers, blend and opacity, wipes by line and iris,
    stroke sets with tips, anchors, boil and flow, the painting's planning
    and colour, picked colours from a picture, a picture target listed below
    its reader and an application texture, the surface finish, checkpoints,
    a stroke-level transition, the ripple field and caustics, and refraction
    and reflection with every water source) with no warning from the crate or the frontend, and
    `cargo test --offline` runs the crate's unit tests on the host from the
    copy in the materialized vendor folder;
  - then with GraphicsCrates.Register() as well: the three bundles are
    reported, the vendor folder holds one folder per crate of the four
    archives, and `cargo check` compiles Materialize/frontend-both-main.rs, a
    frontend that names BOTH crates (a Graphics layer under an Animation
    layer) - the proof that an application can pull in either package, or
    both. Only this TEST project references the Graphics project; the
    Animation library never does.
The same skip rule as WebGpu.Tests.

The LibraryChecker tests run the gate's checks on small made-up trees and
the whole gate against this repository (they need cargo, offline).

WHAT THE UNIT TESTS OF codebrix-webgpu COVER, AND WHAT CANNOT BE TESTED ON THE
HOST. The crate's `#[cfg(test)]` tests (run by the materialize test above)
cover its CPU-only logic: the clock (pause, resume, speed, seek, a seek
before the first frame on a running and on a paused clock, the same
tick for every canvas in one animation frame), the IMAGE DECODERS (PNG: every
colour type and bit depth, palette and tRNS, 16-bit keeping the high byte,
an APNG's default image, the size limit; JPEG: baseline, progressive,
greyscale, 4:2:0 at an odd size, and every refusal - CMYK, YCCK, 12-bit,
lossless, arithmetic, hierarchical, oversize, cut short in the header and in
the data; decode_image's sniffing and named refusals - on small JPEG files
in the crate's fixtures/jpeg/, made for the tests from synthetic pixels as
the test file's `//!` says, and PNG files the tests encode themselves), the
pointer events' buttons, button, kind and modifiers, the named points in time
(reached, since, crossed across pause, speed and seeks), the layer list
(order, add, move, remove) and when a layer is prepared or resized, the
surface size from the displayed size and pixel ratio, the drag deltas (each
pointer id on its own), the wheel units, the keys the canvas keeps, the
renderer handle, the messages, the maths order (glam applies the right-hand
matrix first; the camera module projects into depth 0..1), and the queues
of the shared error handler (every subscriber sees every error in order,
taking from one leaves the others', a late subscriber sees only later
errors, a dropped subscription ends, the capacity and its count, the shared
lost reason). The installation itself (once per device, through a registry
of the page's devices) needs a device and is proved in the browser (the
"Sprites" scene's check). Whatever
needs a browser cannot run on the host: there is no canvas surface outside a
browser build (the crate compiles for the host with a stub that says so),
no requestAnimationFrame, no adapter. Those parts are proved by the
GraphicsDemo sample in headless Chrome (its MAINTAINER-README.txt says how).

WHAT THE UNIT TESTS OF codebrix-webgpu-graphics COVER: Color4 (hex, bytes,
packing, lerp), the error and exception messages, the C# defaults of every
options struct, the buffer write checks (alignment, overrun, padding), the
factories' usage rules, texture formats, the full mip chain length and the CPU
box filter, sampler presets, binding-layout entries and their `with_*`
changes, the blend/depth/cull presets, vertex layouts (offsets, locations,
strides), the one-shot future behind async readback, the shader diagnostics, and
the remaining parts (the animated texture's frame index in every loop mode, its durations,
storage choice and premultiply; texture regions and the 256-byte row packing;
the indirect, clear and query-set checks; the GPU timer's milliseconds; the
context options' feature choice),
the device errors as GraphicsErrors, RectangleF, the sprite batch's CPU half
(texture coordinates of a region, runs that break only on a new texture and
never on a shape, drawing order, rotation about an origin, the transform
stack, every shape's geometry and C#'s segment rule, the quad indices, the
C# defaults of its options), the camera (C#'s CameraTests ported: the
target in the middle, depth 0..1, orthographic sizes; a degenerate camera),
the orbit camera's clamps, the ORBIT CONTROLLER (auto-orbit from the frame
time and its stop on a drag, a rightward drag lowering the yaw - the scene
following the pointer -, the pitch and distance limits, with_limits and
negative pitch limits, wheel steps, a pinch that zooms without orbiting, one
finger after a pinch; PANNING by button, Shift, pen and two fingers, the mode
switching mid-drag, the bounds, the content following the pointer checked
through world_to_canvas in both projections; frame_box; the orthographic
zoom - wheel, pinch, limits, the application's own height, pan after zoom,
perspective unchanged), world_to_canvas and canvas_to_ray round trips and
the ray-box tests (drawing3d/ray.rs), the mesh primitives
(C#'s MeshShapesTests ported: counts, bounds, outward winding; transformed,
combine), the mesh checks, the lit material's defaults, translucency, alpha
parameter and texture-transform rows, the mesh renderer's CPU half
(mirroring, instance data and its normal matrix, consecutive draws merged,
mirrored runs apart, the frame uniforms), the scene lighting's defaults,
the MODEL DATA READER on the .NET writer's fixture blobs (every value of
every section, the shared texture, the derived node order, draw items,
morph-weight and palette layout and rest-pose matrices) and on damaged data
(its messages), the model types' defaults, ModelVertex's layout, the PBR
renderer's CPU half (frame uniforms with the lights in C#'s order, material
uniforms and texture-transform rows, material features, morph-delta packing,
mirroring runs, shader sRGB encoding by format), ANIMATION PLAYBACK (C#'s
AnimationSamplingTests and AnimationPlayerTests ported with C#'s test model
and values - STEP, LINEAR and CUBICSPLINE keys, slerp on the shorter arc, the
key cursor against a plain search, loop / once / ping-pong, pause, speed,
seeks, cross-fade weights, layered and weighted blending with the quaternion
hemisphere correction, morph-weight channels, stopping to the rest pose, the
finished clips, the state limit - plus the frame-time update through pause,
speed and seeks; the C# tests load their model on a GPU, the Rust tests make
the same nodes and clips as data, since the player needs no GPU),
and the shaders: the PBR source is one constant per feature followed by the
body, MIRRORED changes nothing, every variant is listed once, no variant
declares an array of matrices (the C# library's guard), the bakers start with
their common part, the tone map has its three named curves. And the
additions beyond the C# library, each beside its code: the GLTF LOADER
(src/gltf/: the .glb container, data: URIs, percent escapes and address
joining, .NET's arithmetic in numerics.rs, strips and fans, generated
normals and tangents, texture format sniffing and reduction, the three
packagings, every refusal by name, damaged documents with every byte
changed or cut short - never a panic -, materials and extensions, texture
and vertex sharing, skins, animation packing, the upload plan of
GltfModelSet::create_models; and the reference comparison above), the model
facts, the translucent sort that follows skins and morphs, the per-instance
look and node visibility, update_to_since_zero and the per-player clip
limit, CULLING (the bounds grown at load: models/model_bounds_tests.rs; the
volume test: visibility/), two-pass double-sided parts, LOCAL LIGHTS AND
SHADOWS (falloff, cone, importance and choice, the light views and their
volumes, the bias numbers, the shadow passes of each pair), MULTISAMPLING
(the buffer plans, the pipeline check), LEVEL OF DETAIL (the choice, bands,
hysteresis and culling: models/lod_tests.rs; the dither arithmetic, checked
against the WGSL copies in pbr_body.wgsl and shadow_depth.wgsl), the
EFFECTS (the frame timer's arithmetic, the glow layer's packing, bloom's
chain and bright pass, the particles' spawning and stepping, mirrored in
effects/particle_math.rs), and WATER (water/tests.rs and
water/tests_surface.rs: the wave field's CPU twin, its stability rule,
drops, wakes, rain, masks and stirring, the open-water waves, the water
surface's uniform record and its CPU twins). What needs a
browser - creating anything on a device, compiling, drawing, readback - is
proved in the GraphicsDemo sample's scenes in headless Chrome ("Shaders",
"Textured quad", the Viewer and the feature demos; its MAINTAINER-README.txt
says how).


THE MAINTAINER TOOL
===================
libs/CodeBrix.WebPlatform.WebGpu.LibraryChecker (assembly LibraryChecker) is
this repository's licence and vendoring gate, ported from
CodeBrix.WebPlatform's tool and reduced to what this repository holds. It
finds the repository root by the .slnx and the allowlist. Its check groups:

    no-node    nothing of ours uses Node.js
    vendor     Rust/vendor holds exactly the crates the crate set derives,
               every file as crates.io published it; the Graphics and the
               Animation library's Rust/vendor each hold exactly their crate
               set's OWN crates (its lock minus what the WebGpu package or the
               platform carries), every file as published; git ignores no
               file of the three vendor trees; no GitHub workflow files
               outside them
    platform   THE STRICT RULE against the platform's archive (from the
               FrontendBuilder package at the Core pin's version); and for
               the Graphics and the Animation crate sets: every crate one
               shares is at the WebGpu package's or the platform's version
               with the same checksum, and every crate it vendors is a name
               neither of them carries in any version; and the two bundles
               side by side (an application may register both): no crate is
               vendored by both packages in another version or with other
               content (the same crate in both is listed as a note)
    platform   ALSO THE FAMILY RULE: Graphics and Animation import the
               exact-pin target and pin the WebGpu project, never a WebGpu
               package
    first-party
               codebrix-webgpu, codebrix-webgpu-graphics and
               codebrix-webgpu-animation are ours (license Apache-2.0,
               publish = false, a name no third-party crate has), and every
               dependency is at an EXACT version the vendored or the
               platform's crates carry (the Graphics and Animation crates may
               also name codebrix-webgpu, at its own version, and each the
               crates its own vendor folder holds); they are
               exempt from the licence audit and absent from the generated
               third-party notices (that is what "ours" means)
    dependency-rule
               Graphics and Animation, as projects and as crates, both depend
               on WebGpu (the project reference; codebrix-webgpu in the
               crate's [dependencies]) and never name each other (no
               reference, no crate name, no Rust name in the sources)
    licenses   every vendored crate, and the crates compiled into a wasm32
               application, against the allowlist; bundled files scanned;
               the same for the Graphics and the Animation crate sets, each
               audited offline in a temporary copy whose vendor folder merges
               its own crates with the shared ones (cargo cannot resolve it
               from its own vendor folder alone)
    notices    THIRD-PARTY-NOTICES-WebGpuCrates.txt is the header plus what
               the audit produces; `notices --write` regenerates it, and
               THIRD-PARTY-NOTICES-WebGpuGraphicsCrates.txt and
               THIRD-PARTY-NOTICES-WebGpuAnimationCrates.txt the same way
               from their own headers and each package's OWN crates. The
               Graphics header has the crate's own-code statement, its four
               entries (the three tone-mapping functions, with the MIT text,
               and the .hdr decoder's source) and the glTF crates' part, and
               the Graphics project packs the file; the Animation header has
               the vello and Skia entries (Skia's licence
               text verbatim) and harfrust's Unicode data (the Unicode
               License V3 text verbatim) and the Animation project packs the file.
               The GraphicsDemo sample's notices name every model,
               panorama and font pack (Models/, Environments/ and Fonts/ of
               GraphicsDemo.Assets; a font's licence file is its original
               OFL.txt),
               its licence file and its files - except the packs that are
               the owner's own work, named one by one in
               SampleNoticesCheck.OwnersOwnPacks (today crystals-and-avian,
               the Avian): his rule is that his own work needs no
               provenance, attribution or notice, so the check demands
               neither a licence file nor an entry for them and lists them
               as notes. Add a pack there only on the owner's word

Run it before every commit that touches the crate set, the vendor tree, the
first-party crate, the notices or the Core pin.


PACKAGING AND PUBLISHING
========================
GeneratePackageOnBuild is true in every library, so every build emits fresh
.nupkg files (under bin/<Configuration>/, or the artifacts path's package
folder).

Versioning is the CodeBrix date-stamped scheme, computed in each csproj from
System.DateTime.UtcNow as 1.<years-since-base>.<day-of-year>.<minute-of-day>.
It is monotonically increasing but is NOT SemVer. Two builds inside the same
UTC minute produce the SAME version - never publish two packages from one
minute. Do not replace the version block with a literal <Version>.
WebGpuCrates reads the package version it registers FROM THE ASSEMBLY (the
informational version, without build metadata, written as NuGet writes a
package version), so the bundle always names the real package version.

Publish the three packages from ONE build, after the Core they pin is on
nuget.org. Check that the .nupkg files carry the same version.

What ships inside each nupkg:
    the assembly and its XML documentation
    icon-codebrix-128.png    (PackageIcon)
    README.md                (PackageReadmeFile)
    AGENT-README.txt         (the consumer guide, shared, from the repo root)
    THIRD-PARTY-NOTICES.txt  (the ROOT file - see NOTICES)
  and, in the WebGpu package:
    tools/webgpu-crates/webgpu-crates.zip   (vendored crates + codebrix-webgpu)
    tools/webgpu-crates/THIRD-PARTY-NOTICES-WebGpuCrates.txt
    buildTransitive/CodeBrix.WebPlatform.WebGpu.ApacheLicenseForever.targets
  and, in the Graphics package:
    tools/webgpu-graphics-crates/webgpu-graphics-crates.zip
                                            (its own vendored crates +
                                             codebrix-webgpu-graphics)
    tools/webgpu-graphics-crates/THIRD-PARTY-NOTICES-WebGpuGraphicsCrates.txt
    buildTransitive/CodeBrix.WebPlatform.WebGpu.Graphics.ApacheLicenseForever.targets
  and, in the Animation package:
    tools/webgpu-animation-crates/webgpu-animation-crates.zip
                                            (its own vendored crates +
                                             codebrix-webgpu-animation)
    tools/webgpu-animation-crates/THIRD-PARTY-NOTICES-WebGpuAnimationCrates.txt
    buildTransitive/CodeBrix.WebPlatform.WebGpu.Animation.ApacheLicenseForever.targets
The folder names webgpu-crates/, webgpu-graphics-crates/ and
webgpu-animation-crates/ are a contract
(the registration calls open the archives there). Look in each nuspec: Core
"[x]", and in Graphics and Animation the WebGpu package "[same version]".

Never publish a WebGpu package built without its crates or without the
generated notices: run the gate first, and look inside the .nupkg.
Jeremy publishes.


VENDORED SNAPSHOTS
==================
Rust/vendor holds point-in-time snapshots of crates, exactly as crates.io
published them (`cargo vendor --versioned-dirs` form, each folder with its
.cargo-checksum.json). They are never edited:
  - cargo records a checksum for every file and refuses to build from a
    crate whose files no longer match;
  - the gate's vendor check reports any modified, missing or added file.
Do not put notes or patches inside a vendored crate folder. What we know
about wgpu lives in Rust/vendoring/wgpu/UPGRADE-README.txt instead; read it
before changing the crate set.

THE CRATE SET decides the crates: Rust/webgpu-crates/Cargo.toml names wgpu,
at an exact version, with default features off and the features a
WebGPU-only browser build needs; its Cargo.lock fixes the versions, seeded
from the platform's lock so that every shared crate is at the platform's
version. EVERY crate the lock names is vendored (what
`cargo vendor --locked --versioned-dirs` writes for it): more than a browser
build compiles, because a consumer's frontend resolves its own lock offline
for every target and fails when one of them is missing. The UPGRADE-README
says exactly how the set is derived and why; the generated notices say which
crates are compiled into an application and which are only carried. Adding a
crate or a
feature is a new crate set: the owner approves it first, and the licence
files inside every new crate are READ before it comes in.

ALL the crates of the set are vendored here, also those the platform
carries: this repository is self-contained, and the gate keeps the shared
ones identical to the platform's. In an application's vendor folder cargo
keeps one copy of each.

THE CRATES CHOSEN BY NAME: wgpu (its features in the crate set's comment),
glam (the maths of codebrix-webgpu; default features plus bytemuck, its
other optional dependencies off and not vendored), png (PNG files for
codebrix-webgpu's decode_png, with its tree; zlib-rs carried but not
compiled, the test PDF it carries) and zune-jpeg (JPEG files for
decode_jpeg; DEFAULT FEATURES OFF, `std` on; it brings zune-core). Each has
an UPGRADE-README.txt under Rust/vendoring/.

THE SECOND CRATE SET: THE ANIMATION PACKAGE'S. src/CodeBrix.WebPlatform.WebGpu
.Animation/Rust/animation-crates/Cargo.toml names kurbo, peniko, harfrust, i_overlay, usvg,
svgtypes and velato
(exact versions, default features); its Cargo.lock is resolved against the
platform's lock AND the WebGpu crate set's, so that every crate it shares
with them is at their exact version. Its vendor folder
(src/CodeBrix.WebPlatform.WebGpu.Animation/Rust/vendor) holds ONLY THE CRATES
THE PACKAGE ADDS - the lock's crates that neither the platform nor the WebGpu
package carries - because the Graphics and Animation bundles hold only what
they add (the owner's rule). That is the one difference from the WebGpu set
(and the Graphics set below is built the same way):
the folder cannot resolve the set by itself, so the gate builds a merged
source for cargo (MAINTAINER TOOL, licenses), as an application's vendor
folder is after Materialize. The same rules otherwise: exactly as crates.io
published them, never edited, every new crate's licence files read before it
comes in, the owner approves every new crate.
Rust/vendoring/kurbo-peniko/UPGRADE-README.txt says how the set is derived
and re-derived; Rust/vendoring/harfrust/UPGRADE-README.txt what is particular
to harfrust (its Unicode data, read-fonts' Adobe Glyph List, the .wasm cost);
Rust/vendoring/i_overlay/UPGRADE-README.txt what is particular to i_overlay
(two crates without a licence file, libm compiled in, how the crate uses it).
The set also names usvg (default features off, its `text` feature on) and
svgtypes at the version usvg brings; Rust/vendoring/usvg/UPGRADE-README.txt
says what is particular to them (the public-domain test font fontdb carries,
the ids usvg clears inside <use>d content, how the crate uses usvg). And it
names velato (DEFAULT FEATURES OFF); Rust/vendoring/velato/UPGRADE-README.txt
says what is particular to it (default features off, its panics and the
crate's pass in front of them).

THE THIRD CRATE SET: THE GRAPHICS PACKAGE'S. src/CodeBrix.WebPlatform.WebGpu
.Graphics/Rust/graphics-crates/Cargo.toml names gltf (approved by the owner
by name; DEFAULT FEATURES OFF - its `import` feature reads files and decodes
images through the `image` crate, and is never turned on - with the
features the crate's glTF loader reads, listed in that file's comment, and
the same line in the Graphics crate's Cargo.toml). It is resolved and
vendored exactly as the Animation set: its lock against the platform's and
the WebGpu set's locks, its vendor folder
(src/CodeBrix.WebPlatform.WebGpu.Graphics/Rust/vendor) holding ONLY the
crates the package adds (gltf, gltf-json, gltf-derive, lazy_static,
inflections); everything else gltf uses (serde, serde_json - whose
`raw_value` feature gltf-json switches on, a feature and not a crate -,
byteorder, proc-macro2, quote, syn and the rest) is the platform's, at its
version. gltf-derive is a procedural macro (build time only). inflections
carries a documentation deploy script of its author: part of the crate as
published, never run, never deleted. Rust/vendoring/gltf/UPGRADE-README.txt
says how the set is derived and what is particular to gltf. The Graphics
and the Animation sets never share a crate at different versions or
contents (the gate's platform group).

UPGRADING A VENDORED LIBRARY (the generic procedure every UPGRADE-README.txt
under Rust/vendoring/ points at; each file holds what is particular to its
library):
  1. The owner approves a new crate, a new feature or a new crate set
     first. Only stable releases from crates.io, never the upstream
     repository; take the latest release when upgrading.
  2. Read the release notes of every version in between, and the new
     version's Cargo.toml (its features and dependencies, its minimum Rust
     version against the host tools' toolchain). Keep the crate set's
     comment true.
  3. Change the exact version (`=x.y.z`) in the crate set's Cargo.toml and
     in every first-party crate that names the library.
  4. Re-resolve AGAINST THE PLATFORM'S LOCK (Rust/vendoring/wgpu/
     UPGRADE-README.txt, THE LOCK IS RESOLVED AGAINST THE PLATFORM'S LOCK,
     has the steps; the Graphics and Animation sets also against the WebGpu
     set's lock). `LibraryChecker revendor` does it for the WebGpu set from
     the platform's lock ALONE, which lets cargo move every crate the
     platform does not carry to its newest compatible version: when one
     crate is meant to change, seed the scratch resolve with the set's
     current Cargo.lock instead, and compare the new lock with the old one
     (only the intended entries may differ).
  5. `cargo vendor --locked --versioned-dirs` into a fresh folder and
     replace the package's Rust/vendor (the Graphics and Animation folders:
     only the crates the package adds) and the set's Cargo.lock. Never edit
     a vendored file.
  6. READ THE LICENCE FILES inside every crate new to the lock, not only its
     declared licence; a licence that is not approved goes to the owner.
  7. Bump the version of every first-party crate whose dependency moved
     (THE FIRST-PARTY CRATE, ITS VERSION).
  8. Run the gate (`LibraryChecker`) and regenerate the notices
     (`LibraryChecker notices --write`); read the difference. Then the
     tests of the touched packages (the materialize tests build and test the
     crates offline from the vendored folders) and the GraphicsDemo sample's
     scenes that use the library, in a browser.


THE FIRST-PARTY CRATE
=====================
Rust/codebrix-webgpu is the crate `codebrix-webgpu`, written here: the WebGPU
context that owns or shares the device (with the device's ONE uncaptured-error
handler and its subscriptions), ONE canvas host component (frame
loop, input, layers, the host's clear, the surface configuration and the
blending convention), the clock with pause, speed, seek and named points in
time, the "WebGPU is not available" result and message, glam's maths
re-exported under its own names, and PNG and JPEG decoding (src/images/:
decode_png through png, decode_jpeg through zune-jpeg, decode_image picking
one by the first bytes - separate functions, so that an application carries
only the decoder it calls; the PNG code moved here unchanged from the
Animation crate). Graphics and Animation build on it. The consumer's view of
it is in AGENT-README.txt.

  - NOT ON crates.io. It travels only inside webgpu-crates.zip, in vendored
    form (THE CRATES ARCHIVE above), and reaches a frontend through
    Materialize; the frontend names it like any crate. A frontend in
    CrateMode.ApplicationAddsCrates cannot reach it (a stated limit).
  - OURS: Apache-2.0 under the repository's LICENSE, publish = false. It is
    not third-party: no notices entry, no licence audit (the gate's
    first-party group checks the manifest instead).
  - EVERY DEPENDENCY AT AN EXACT VERSION that the vendored or the platform's
    crates carry (the gate checks it): dioxus, web-sys and wasm-bindgen are
    the PLATFORM's (dioxus is its =0.7.10 line today), wgpu, glam, png and
    zune-jpeg the crate set's. THIS TIES THE CRATE TO ONE PLATFORM
    RELEASE: every platform republish that moves one of those crates means
    editing the crate's Cargo.toml to the new versions, building it against
    them (the materialize tests), and republishing with the new Core pin. Each
    dependency is on one line of its table (the gate reads it so).
  - ITS VERSION. cargo treats a name and version from a vendor folder as
    unchanging (it does not notice changed content under the same version),
    so change `version` in its Cargo.toml whenever its content changes in a
    package that ships; the archive folder's name follows. A change of the
    PUBLIC API is a minor version (or a major one if it breaks a caller);
    codebrix-webgpu-graphics names codebrix-webgpu's exact version, so bump
    both together. A NEW VERSION MOVES AN ENTRY OF EVERY FRONTEND'S LOCK: its
    Cargo.lock names the old version, which the materialized vendor folder no
    longer holds. The PLATFORM handles that (its AGENT-README is the
    authority on the rule): a lock that carries the platform's MARKER comment
    as its third line ("# CodeBrix.WebPlatform keeps this file in step ...")
    is brought in step by Materialize itself - exactly the moved entries,
    reported as "Cargo.lock brought in step (codebrix-webgpu-graphics <old>
    -> <new>)"; GraphicsDemo's two frontend locks carry the marker. A lock
    WITHOUT the marker is the developer's: Materialize leaves it alone and
    warns, and the build refuses with the exact `cargo update --offline -p
    <crate>` command to run. The consumer's side of this is a pitfall in
    AGENT-README.
  - NO BACKEND CODE: nothing in it talks to the application's backend (no
    reqwest, no fetch); that is the platform's shared crate's job.
  - NO STYLESHEET: no CSS class names inside it; the application passes its
    classes in (class, heading_class, text_class).
  - It compiles for the host as well as for wasm32 (the surface creation has
    a host stub), so that its unit tests run on the host.
  - THE TRANSPARENT CANVAS (src/page_compose.rs, internal). The page
    composites a canvas as colour premultiplied in its sRGB values; the
    layers write linear premultiplied colour through the sRGB view, which
    stores sRGB(colour x alpha) - too bright wherever alpha is partial. So
    on a canvas with `transparent: true` the host gives the layers a texture
    of its own (target.format, the surface's size; CanvasFrame.texture and
    .view are that texture's) and, after the last layer, one full-canvas pass
    writes it onto the surface (in the surface's own, non-sRGB format):
    un-premultiplied, sRGB-encoded, premultiplied again. The layer contract
    does not change; opaque canvases are untouched. compose_pixel is the
    pass's arithmetic on the CPU, for the unit test. Proved in a browser by
    the sample's two "on the page" scenes (a protocol screenshot of the page
    as composited: the page unchanged where nothing is drawn, a
    half-transparent square exactly the sRGB blend, no fringe at an edge).
  - Do not run cargo inside its folder; if you do, its target/ and
    Cargo.lock are git-ignored and never enter the archive. To check it by
    hand, copy it to a scratch folder with a .cargo/config.toml that points
    at a vendor folder holding the platform's crates and ours (a
    materialized frontend's).


THE GRAPHICS CRATE
==================
Rust/codebrix-webgpu-graphics (in src/CodeBrix.WebPlatform.WebGpu.Graphics)
is the crate `codebrix-webgpu-graphics`, written here: the Rust counterpart of
the .NET library CodeBrix.WebGpu.Graphics (repository CodeBrix.WebGpu), built
on codebrix-webgpu. The consumer's view, with the porting rules and the C#
and Rust example side by side, is in AGENT-README.txt (THE
codebrix-webgpu-graphics CRATE). Everything THE FIRST-PARTY CRATE says above
holds for it too (not on crates.io; ours; exact dependencies; bump `version`
when its content changes; no backend code; host build for its unit tests; do
not run cargo in its folder). In addition:

  - DEPENDENCIES: codebrix-webgpu at its own exact version, wgpu, glam
    and bytemuck at the WebGpu crate set's exact versions, and gltf at the
    Graphics crate set's exact version, with the set's features (THE THIRD
    CRATE SET in VENDORED SNAPSHOTS). NEVER anything of the Animation
    package. No dioxus and no web-sys of its own: the canvas host is
    codebrix-webgpu's, and PNG and JPEG are decoded by codebrix-webgpu. The
    crate itself derives nothing (bytemuck's derive is not used); the one
    procedural macro in its build is gltf-derive, which gltf-json uses.
  - THE PARITY RULE (the owner's): the same FEATURES and the same SHAPE of
    API as the C# library - type names kept, members in snake_case, the same
    argument order, an options struct (with Default holding the C#
    defaults) where C# has two or more default arguments, a written-out
    Option<&str> where the only default is the label, Drop for Dispose,
    polled queues for events - for everything that makes sense in a browser.
    THE ONE DELIBERATE DIFFERENCE: a browser cannot wait, so device creation,
    readback and shader diagnostics are async, a validation error is queued
    (take_errors, C#'s TakeErrors), and the C# "throw with WebGPU's message"
    is an awaited form (validate, build_checked, try_create_shader).
    Existing licence-compatible Rust code is preferred over porting C# for
    what does the work INSIDE (nothing new is approved: propose a crate to
    the owner by name first).
  - A LAYER, NEVER ALONE: the crate draws into the frame the canvas host
    hands it (begin_canvas_frame), loads instead of clearing, and takes its
    time from the frame: NOTHING IT DRAWS READS A CLOCK. The one exception
    (the owner's) is measurement-only frame timing: FrameTimer reads the
    page's performance.now() (std::time::Instant on the host;
    src/queries/cpu_clock.rs) to measure CPU time for a read-out, and only
    when an application makes one; nothing drawn depends on it. No text of
    its own (text is the Animation crate's, as a layer).
  - THE ORDER OF ITS FEATURES (the owner's, agreed): 1 context, canvas
    frames, hosting, every built-in shader compiled in the browser; 2
    resources and the pipeline builder; 3 SpriteBatch; 4 the lit-mesh
    renderer, cameras and input; 5 the model-data API and static PBR models;
    6 sky, image-based lighting, skybox; 7 the HDR path and tone mapping; 8
    animation playback; 9 skinning and morph targets; 10 the rest. Each step
    ends visible in the GraphicsDemo "graphics" application, and the owner
    judges the look himself: no programmatic image comparison with the C#
    library, ever.
  - ONE DEVICE, ONE ERROR HANDLER (settled): WebGPU has one uncaptured-error
    handler and one device-lost callback per device. codebrix-webgpu installs
    both, ONCE per device (a thread-local registry of the page's devices,
    keyed by wgpu::Device, holding each device's hub weakly; every
    GpuContext constructor goes through it), and hands the errors to
    SUBSCRIPTIONS: every subscription sees every error that arrives while it
    exists; reading one never takes errors from another; each keeps at most
    a fixed number unread and counts the rest. GraphicsContext::take_errors
    is one subscription per context (clones share it); the Animation crate
    takes its own the same way. Neither crate, and no sample, calls
    on_uncaptured_error or set_device_lost_callback. The browser hands an
    error over when the device does work (a submission), which is why the
    sample's check submits empty work while it waits.
  - THE BATCHING RENDERERS (SpriteBatch, MeshRenderer) keep their CPU half
    in a type of its own (SpriteBuilder, MeshBatch), generic over the texture
    or geometry key, so that the batching is unit-tested on the host. Their
    begin takes the RenderPass and end takes it again (Rust's borrows); each
    starts its buffers over when RenderPass::frame_id changes. Buffers that
    grow are replaced at once: dropping a wgpu object in the browser does not
    destroy it (the web backend's Drop does nothing), so work already
    recorded in the frame stays valid.
  - MODELS (src/models/): the model-data READER (ModelDataSet::read), the
    model types, Model (an Rc handle), ModelInstance and ModelRenderer - a
    port of the C# library's Model, ModelInstance, ModelRenderer and
    PbrModelRenderPath (same uniform and storage layouts, same packing of
    instances, node matrices, joint palettes and morph weights, same
    batching, mirroring and back-to-front blending). The C# GltfModelLoader's
    CPU half is src/gltf/ (below), its upload half
    GraphicsContext::create_models.
    ADDED (not in C#), in the same files and beside them: the finishing
    step in two halves (end_opaque, end_blended; end does both); the
    translucent sort key moved by the current morph weights and skin (the
    morph targets' centre shifts are worked out at load); the per-instance
    tint, alpha and emissive strength and node visibility (instances that
    use them are planned after the plain ones, so plain instances still
    batch as before); node world matrices by name; the model facts on Model
    and ModelData; material_mut and set_material_texture (only on the sole
    handle of a model); FRUSTUM CULLING with bounds grown at load
    (models/model_bounds.rs; the volume test in src/visibility/; the one
    rule in model_draw_plan::is_culled); two-pass drawing of blended
    double-sided parts; LOCAL LIGHTS and SHADOWS (src/lighting/
    local_lights.rs and shadows.rs: the CPU half; models/model_shadows.rs:
    the maps, the depth pipelines and the passes, on an encoder of the
    renderer's own, submitted at the pair's first finishing call; each pair
    of a frame its own set of maps); LEVEL OF DETAIL (models/
    lod_instance.rs: the types and the CPU choice; models/lod_dither.rs: the
    dither code and the host mirror of the shaders' dither arithmetic,
    whose WGSL copies in pbr_body.wgsl and shadow_depth.wgsl must stay
    identical text - a host test checks both); ModelRenderStats. The
    PbrFeatures SHADER features are the low contiguous bits and the
    pipeline-only flags sit above them (every_shader_variant maps the low
    bits straight to variants): a new shader feature goes at the low end
    and moves the pipeline-only flags up, with from_bits_truncate's mask and
    the tests that spell the masks. Prefer a shader of its own (as the water
    and effects have) to a new PBR feature: each one doubles the variants.
  - THE glTF LOADER (src/gltf/, not in C#): parsing through the gltf crate,
    and a BUILDER that is a port of the C# GltfModelBuilder (step for step
    the desktop library's loader: its normal and tangent generation, not
    MikkTSpace, and its warnings, word for word), producing the crate's
    ModelData / ModelDataSet, so that create_models and the renderer did not
    change. numerics.rs reproduces .NET's arithmetic where the result
    depends on it (normalising by division, the TRS matrix,
    Matrix4x4.Decompose, Matrix3x2.CreateRotation's snapping). Its own entry
    points, never inside load_model, so that an application that loads only
    blobs carries no glTF code. Images through codebrix-webgpu's decoders.
    Checked against THE REFERENCE CONVERTER on the test side (TESTING, THE
    glTF REFERENCE): keep the two in step when either changes. Differences
    from the C# converter, by decision: only PNG and JPEG are decoded; only
    the textures a material slot uses are decoded; vertex sharing
    (share_vertex_data; off gives the C# layout); nothing is repaired; an
    accessor that runs past its buffer view leaves its primitive out;
    max_texture_size averages pixels itself (alike, not equal to the C#
    resampler).
  - ANTI-ALIASING (src/presentation/: canvas_frame_target.rs,
    multisampling.rs, the sample count in HeadlessRenderTarget and
    RenderTargetView): 1 or 4 samples per target; every renderer keys its
    pipelines by TargetFormats (colour, depth, sample count);
    RenderPass::set_pipeline checks a pipeline against its target. A
    4-sample canvas layer resolves into a texture of its own and lays it
    over the canvas when its GraphicsFrame is dropped (WebGPU's resolve
    would replace the layers below). A multisampled DEPTH texture has
    TEXTURE_BINDING usage (the water's capture reads it; probed in Chrome).
    Shadow maps stay at 1 sample (a comparison sampler cannot read a
    multisampled texture).
  - THE EFFECTS (src/effects/: glow layer, bloom, particles;
    src/queries/frame_timer.rs) and WATER (src/water/: the wave field, its
    CPU twin WaveCore shared by WaveField and the public CpuWaveField,
    open-water waves, the water surface, its scene capture) are new files
    with OUR OWN shaders (glow, bloom, particles, wave_field,
    water_surface, water_capture .wgsl, listed by EffectShaders::all() and
    WaterShaders::all(), not by BuiltInShaderSource::all): THE WGSL COPY
    rules below do not apply to them. Each shader has a host-tested twin in
    Rust that must stay in step with it: particles.wgsl with
    effects/particle_math.rs, wave_field.wgsl with water/wave_math.rs,
    water_surface.wgsl with water/water_math.rs and water/gerstner_math.rs
    (the uniform record's offsets are asserted against the WGSL struct's
    field order). The wave field damps the VELOCITY, not the height (a
    damped height acts as a spring that reflects waves at an absorbing
    edge); it follows the canvas clock's DELTA, at a fixed step rate. Prove
    every new or changed shader in Chrome, not only in naga: Chrome refuses
    a texture sample or a derivative inside a branch, and a float literal at
    f32's limit, that naga accepts.
    The frame bind group binds the chosen environment's irradiance and
    prefiltered cubes and the context's BRDF LUT (AmbientLighting::
    Environment, C#'s SelectEnvironment: the lighting's environment or the
    context's default sky), or 1 x 1 placeholders for the hemisphere; the
    renderer keeps the groups of the last few environments.
    THE FORMAT is the Graphics package's: the .NET writer
    (src/CodeBrix.WebPlatform.WebGpu.Graphics/Models, ModelDataWriter and its
    Internal/ encoder) and this reader must change TOGETHER. A change that
    an older reader can skip (a new section, longer records) raises the
    MINOR version; anything else raises the MAJOR version, which an older
    reader refuses with a message. Keep the record layouts documented in
    the writer's code and in the reader's offsets in step, regenerate the
    fixtures (TESTING), and bump the crate's version.
  - ANIMATION (src/animation/, and ModelInstance's player and pose edit): a
    port of the C# library's Animation/ (AnimationPlayer, AnimationState,
    the internal AnimationSampling; AnimationLoopMode) and of
    ModelInstance.Animation / BeginPoseEdit / ModelPoseEdit - the same
    sampling, the same per-layer blending with its scratch accumulators, the
    same fade and loop arithmetic, the same rest-pose return on stop.
    Differences, all forced by Rust's ownership or by the browser: the
    playback state (PlayerData) lives in the instance and
    ModelInstance::animation() hands out an AnimationPlayer that BORROWS the
    instance (C# hands out an object that refers back to it); the player's
    core works on slices of the model's nodes and clips and of the
    instance's pose (PoseSource, PoseTarget), so it is unit-tested on the
    host without a Model (which needs a device); a state cannot remove
    itself, so AnimationState::stop and a fade to 0 over no time with
    stop_when_zero MARK it and the player removes it at its next call; C#'s
    ClipFinished callback is finished_clips() after an update; overloads by
    name are the `_named` twins; C#'s throws are Err(GraphicsException);
    the pose edit is a guard that ends on drop (Rust's borrow forbids
    reading, drawing or animating during it, which C# checks at run time).
    ADDED (no C# counterpart): update_to(time) / update_all_to, which take
    the frame's TIME and advance by the difference to the previous one, so
    the host's clock - pause, speed and seeks - drives playback (THE
    LAYERING RULE: a player never reads a clock). Skinning and morph targets
    need nothing here: the model renderer reads the instance's joint
    palettes and morph weights (its data path is the C# PbrModelRenderPath's).
  - LIGHTING (src/lighting/, src/images/, src/presentation/): a port of the
    C# library's Lighting/ (EnvironmentMap, EnvironmentOptions,
    ProceduralSkyOptions, FloatImage, RadianceHdrDecoder, SkyboxRenderer,
    ToneMapper, the internal EnvironmentBaker, GraphicsContext.Environments),
    of its GPU MipmapGenerator and GenerateMipmaps, and of
    HeadlessRenderTarget - same uniform layouts (BakeUniforms 80 bytes in
    256-byte slots, SkyUniforms 112, SkyboxUniforms 96, ToneMapUniforms 32),
    same passes in the same order, same option clamping. Differences: no
    Parallel.For (the CPU halving of over-wide panoramas and the half-float
    conversion run on one thread; the half conversion is our own small
    round-to-nearest-even function, no crate); for environments no image
    decoding but the .hdr decoder (CreateEnvironment(path/stream/Image) become
    create_environment(&FloatImage) and create_environment_from_bytes(.hdr);
    textures from PNG and JPEG files go through codebrix-webgpu's decoders,
    create_texture_from_png / _jpeg / _image in src/context/);
    EnvironmentMap is an Rc handle holding only wgpu objects, and the
    context's EnvironmentCache (baker pipelines, LUT, default environment)
    and MipmapGenerator hold no context, so nothing forms a reference cycle;
    the skybox, tone mapper and model renderer key their bind-group caches
    by environment id / texture and keep the last few (C# drops disposed
    ones). The CPU parts (option resolution, sky packing, half conversion,
    halving, the decoder, ToneMapper::map) are unit-tested on the host, the
    decoder and the curves with the C# library's own test values.
  - THE REST (src/images/animated_texture.rs, src/bundles/,
    src/frames/{compute_pass, graphics_frame_commands, indirect_args}.rs,
    src/pipelines/compute_pipeline_builder.rs, src/queries/,
    src/resources/{gpu_buffer_mapping, gpu_texture_readback,
    texture_region}.rs): ports of the C# library's AnimatedTexture (C#'s
    frame arithmetic on 100-nanosecond ticks, its storage choice, its
    premultiply rounding; frames as RGBA8 slices, times as seconds), the
    Bundles/ (RenderBundleBuilder RECORDS the calls and replays them into
    wgpu's bundle encoder at finish, as C# replays them into a rehearsal;
    finish_checked awaits WebGPU's verdict where C# throws), the indirect
    argument records and their checks, ComputePipelineBuilder and
    ComputePass, the queries (GpuQuerySet, PassTimestamps, GpuTimer),
    buffer mapping, TextureRegion and TextureCopyLayout, the frame's
    copies, clears and resolves, the async texture and target reads, and
    the Wrap* factories. Differences: every read that waits in C# is async;
    GpuTimer is an Rc handle so a spawned task can read while the layer
    resolves, and it skips the copy into its read-back buffer while a read
    maps it (WebGPU refuses that copy); a bundle on a depth format without
    stencil is made stencil-read-only (the browser's passes are, and a
    bundle must agree); SpriteBatch keys its texture bind groups by the VIEW
    (an animated texture's frames are layer views of one texture);
    GraphicsContextOptions gained optional_features (no C# counterpart: a
    feature asked for only where the adapter has it - how a page detects
    timestamp-query without requiring it) and request_all_adapter_features
    (C#'s RequestAllAdapterFeatures). REFUSED, with a message:
    OCCLUSION query sets - wgpu 30.0.2's WebGPU backend
    (src/backend/webgpu.rs, begin_render_pass) never sets the descriptor's
    occlusionQuerySet, so the browser rejects BeginOcclusionQuery and the
    whole frame's command buffer is invalid (seen in Chrome); re-check on
    every wgpu upgrade (vendoring/wgpu/UPGRADE-README.txt) and lift the
    refusal in check_query_set when the backend passes the set. Not ported,
    because a browser has none: encoder and in-pass timestamps,
    pipeline-statistics queries, ClearTexture, MultiDraw*IndirectCount,
    immediates (wgpu's browser backend panics on most of these, so the
    crate never calls them). The CPU parts are unit-tested on the host (the
    animated texture's frame index, durations, storage choice and
    premultiply; region checks and row packing; indirect and clear checks;
    query-set checks; the timer's millisecond arithmetic; the options'
    feature choice); the GPU parts are proved by the sample's "More" scene.
    THE C#-TO-RUST PARITY AUDIT of the public surface (type by type, with
    what is not in the browser and why) is the basis of AGENT-README's list
    WHAT A DEVELOPER PORTING C# CODE WILL NOT FIND: keep that list in step
    when either library changes.
  - THE ORBIT CONTROLLER is the one type with no C# counterpart: the C#
    library leaves input to each viewer (the desktop and Android hosts of
    WebGpuModelViewer). Its numbers are theirs (drag 0.008 / 0.006 per pixel,
    wheel 0.9 / 1.1, auto-orbit 0.7 + 0.18 t, the Android limits pitch
    0.05..1.35 and distance 3..30). THE DRAG CONVENTION (the owner's): the
    content follows the pointer - a rightward drag lowers the yaw, so the
    scene swings right under the pointer; a PAN moves the target so that the
    content at the target's depth follows the pointer exactly (set_viewport
    gives it the camera and the canvas height). Also not in C#: with_limits,
    panning (pan_enabled, pan_bounds), frame_box, and the orthographic zoom
    (apply sets orthographic_height in proportion to the distance; the
    ratio lives in a Cell because apply takes &self, so the controller is
    not Sync). Camera::world_to_canvas / canvas_to_ray / fit_distance /
    fit_orthographic_height and Ray (drawing3d/ray.rs) are the picking and
    framing helpers.

THE WGSL COPY. The crate's shaders (src/shaders/wgsl/) are a ONE-WAY copy of
the WGSL string constants of the C# library: Shaders/BuiltInShaders.cs (Sprite,
LitMesh), Shaders/PbrShader.cs (Body), Lighting/EnvironmentShaders.cs (Common,
Equirect, ProceduralSky, Irradiance, Prefilter, BrdfLut, Skybox, ToneMap) and
Images/MipmapGenerator.cs (Wgsl), one .wgsl file each, compiled in with
include_str!. No "copied from" header and no attribution (the owner's rule for
his own code; that repository says its WGSL is its own). How the copy is
made, so that the next one is the same:
  - each file is the text of the C# verbatim string exactly, WITHOUT the
    newline that opens the string (the line after @"), up to the closing ";
  - the bakers that C# writes as `Common + @"..."` are
    concat!(environment_common.wgsl, "\n", <its file>) in Rust: the same text
    as C#'s but for that first newline;
  - PbrShader::source(features) writes, as C#'s Source(PbrFeatures), one
    `const NAME: bool = true|false;` line per shader feature in C#'s order,
    then a newline and the body: C#'s output for C#'s features.
OUR ADDITIONS TO THE COPY. pbr_body.wgsl carries additions of this
repository that the C# library does not have - the local lights, the
shadows, the level-of-detail dither, the per-instance tint, alpha and
emissive strength, emissive times vertex colour - each marked "Not in C#"
in the file, and PbrFeatures has shader features of ours after C#'s
(LOCAL_LIGHTS, SHADOWS, DITHER_FADE), so the PBR source is C#'s plus them.
shadow_depth.wgsl is ours entirely. A re-sync keeps every addition.
CHANGES FORCED BY THE BROWSER: NONE. Every built-in shader, and every one of
the PBR shader's feature combinations, compiled in Chrome (on the laptop's
Intel adapter, through Vulkan) with no error and no warning, unchanged. When a
browser's compiler ever refuses one, fix OUR copy with the smallest change,
record it HERE (file, line, the browser's message, the change), and tell the
owner (the C# copy is not edited from this repository).
TO RE-SYNC after the C# library's WGSL changes: repeat the copy above, keep
every change recorded here, run the crate's unit tests and the GraphicsDemo
"Shaders" scene in the browsers that matter, and bump the crate's version.
THE TONE-MAPPING CURVES in tone_map.wgsl (and again in Rust, ToneMapper::map)
follow three outside sources (aces_fitted: BakingLab's ACES.hlsl, MIT;
aces_narkowicz: Narkowicz's post, CC0 elected; pbr_neutral:
KhronosGroup/ToneMapping, Apache-2.0), and the RADIANCE .hdr DECODER follows
the logic of stb_image.h's HDR loader (nothings/stb, MIT or public domain at
the user's choice; public domain elected). All four are listed, with the MIT
text of BakingLab, in the hand-written header of the Graphics package's
notices (the gate requires the four entries); keep it in step if the curves
or the decoder change.


THE ANIMATION CRATE
===================
Rust/codebrix-webgpu-animation (in src/CodeBrix.WebPlatform.WebGpu.Animation)
is the crate `codebrix-webgpu-animation`, written here: real-time 2D animation
with the names and concepts of the .NET library CodeBrix.Imaging.Drawing.NoSkia
(repository CodeBrix.Imaging.Drawing), drawn by the crate's OWN renderer as a
layer of the codebrix-webgpu canvas host. The consumer's view, with the
porting rule and the C# and Rust example side by side, is in AGENT-README.txt
(THE codebrix-webgpu-animation CRATE). Everything THE FIRST-PARTY CRATE says
above holds for it too (not on crates.io; ours; exact dependencies; bump
`version` when its content changes; no backend code; host build for its unit
tests; do not run cargo in its folder). In addition:

  - DEPENDENCIES: codebrix-webgpu at its own exact version (and through it
    wgpu, glam, dioxus, web-sys, and the PNG and JPEG decoders behind
    ImageSource::from_png and from_jpeg - decode_png / decode_jpeg; png and
    zune-jpeg are the WebGpu package's crates, not this crate's), kurbo,
    peniko, harfrust, i_overlay, usvg, svgtypes and velato at the Animation
    crate set's exact versions (THE SECOND CRATE SET above), and serde_json
    at the platform's (velato's JSON reader, which the Lottie module also
    uses). The `color` crate (the Oklab / Oklch mixing of painting/) is
    reached through peniko's re-export (peniko::color), not named. NEVER
    anything of the Graphics package: a 3D scene or a wave field reaches the
    crate as a plain wgpu::TextureView. No dioxus and no web-sys of its own
    (the one browser call, performance.now() in rendering/cpu_clock.rs, goes
    through codebrix_webgpu::wgpu::web_sys::js_sys by reflection).
  - NEW FIELDS AND THE VERSION (the owner's decision): a public struct that
    gains fields, or an enum that gains a variant (DrawingNoiseKind::Caustics),
    is released as a MINOR version, not a major one. AGENT-README tells
    consumers to write struct literals with `..Default::default()` and lists
    the types that gained fields (NEW FIELDS COME WITHOUT A MAJOR VERSION):
    keep that list current, and give every new field a Default that draws
    exactly what was drawn before.
  - THE PARITY RULE (the owner's, as for Graphics): C# type names kept,
    members in snake_case, the same argument order, option structs with
    Default for C# default arguments, Result or Option with a message that
    says what to do where C# throws, named constructors for overloads, Drop
    for Dispose. Existing licence-compatible Rust code is preferred over
    porting C# for what does the work inside (kurbo flattens and strokes;
    vello's code is taken where it fits).
  - THE RULE OF THE CRATE: nothing it DRAWS depends on a clock. Every time
    comes from the frame (the host's one clock: pause, speed, seeks, named
    points), and every value drawn is a function of the time and of what is
    passed in (seeds included: no randomness from anywhere else), so a seek
    shows exactly what playing shows - the ripple field, the rain, the boil,
    the sketch helper, the painting timeline and the checkpoint cache all
    keep it. THE ONE EXCEPTION (the owner's amendment): measurement-only
    frame timing - RenderStats' record_ms / build_ms / encode_ms, read
    through the internal rendering/cpu_clock.rs (performance.now() in a
    browser, std::time::Instant on the host). Nothing drawn may ever read
    it. The one place the rule cannot hold is an application's texture fed
    by a simulation or a video (WaterSource::Texture, a texture colour
    source): the rustdoc says plainly that such a scene cannot be rewound.
  - HOW A FEATURE IS PROVED: every feature ends visible in a scene of the
    GraphicsDemo "animation" application, and the owner judges the look
    himself: no programmatic image comparison with the C# library, ever.
    A default he chooses by eye is stated plainly in the rustdoc and in
    AGENT-README ("default 2.0"); a change of it changes both. The crate
    ships NO artwork and NO brush styles; demo pictures and tips are made in
    code in the sample.
  - EVERY SHADER CHANGE IS PROVED IN A BROWSER, NOT ONLY VALIDATED. The WGSL
    validator used on the host (naga, in a scratch checker outside the
    repository - it is not a dependency) ACCEPTS two things Chrome's WGSL
    compiler REJECTS, both found this way: a texture sample with implicit
    derivatives (textureSample) inside a branch - keep every such sample and
    derivative outside branches and pick coordinates with select(); and a
    float literal at the limit of f32 (3.4028235e38) - use a large ordinary
    value (1.0e9) instead. So a WGSL change is done only when a page that
    uses it has run in Chrome with no WebGPU validation error (subscribe to
    the device's errors and read them every frame).
    The areas with notes of their own: PIXEL PLANES, THE GPU HALVES AND THE
    PAINTERLY PLAYBACK; the SVG DOCUMENTS bullet below; LOTTIE IN THE
    ANIMATION CRATE.
  - LAYOUT: src/lib.rs (the crate's documentation and exports),
    src/color.rs and src/geometry.rs (Color as straight sRGB bytes, PointF,
    RectangleF), src/animation/ (Easing, EasingKind, StepPosition,
    CubicBezierEasing, ColorRamp, AnimationTimeline, TimelineCue,
    PlaybackController, and their tests), src/fills/, src/paths/ (with
    stroke_width_mode.rs - fixed-width strokes, tests_fixed_width.rs - and
    path_sketch.rs - the sketch helper, tests_sketch.rs),
    src/shapes/ (TextShape and TextBlockShape among them, and
    brush_strokes_shape.rs: BrushStrokesShape), src/text/
    (FontRegistry, FontVariation and FontVariationAxis, TextAlign,
    MissingCharacterMode, MissingCharacter,
    ShapedCluster, the layout engine TextLayoutEngine and its results;
    INTERNAL: the font face with its caches, the shaping step, and the text
    and layout tests), src/captions/ (CaptionTrack and its SRT / WebVTT
    parser, CaptionStyle, CaptionPresenter, CaptionPlacement, PlacedCaption),
    src/formations/, src/planes/ (DrawingPlane, DrawingPlaneImage, the
    filters, blends, transitions, DrawingRandom, DrawingNoise - with
    drawing_caustics.rs, the caustics kind, the CPU mirror of noise.wgsl's
    caustics_once -, particles and motion blur), src/painting/ (the
    painterly playback, and the painting's planning and colour:
    direction_field.rs, direction_guide.rs, detail.rs, stroke_layout.rs,
    painting_timeline.rs, color_mix.rs, color_rules.rs - with the CPU
    mirrors of the stroke shader's colour arithmetic -, tests_fields.rs,
    tests_timeline.rs, tests_color.rs), src/strokes/ (the stroke sets: the
    records and sets, BrushTips, StrokeAnchor, StrokeBoil and boil_epoch,
    StrokeFlow, the colour sources of stroke_color.rs, StrokeCheckpoints and
    the cache's CPU half, BrushStrokeMorph; INTERNAL: stroke_gpu.rs and
    stroke_color_gpu.rs - the 64-byte record and the draw block, their
    layouts in the header comments, CPU mirrors of the shader -; tests.rs,
    tests_gpu.rs, tests_color_gpu.rs, tests_checkpoints.rs, tests_morph.rs),
    src/water/ (RippleField, RippleDrop, RippleOptions, RippleRain,
    RippleSwell, SwellWave, WaterSource, WaterRefraction, WaterGlints,
    CausticLight, WaterReflection; INTERNAL: ripple_math.rs - the GPU records
    and the per-point evaluation in f32, the reference the composite
    shader's ripple loop transcribes -; tests.rs, tests_water_gpu.rs),
    src/images/ (ImageSource, SpriteSheet, from_png and from_jpeg -
    image_source_png.rs and image_source_jpeg.rs, thin calls to
    codebrix-webgpu's decode_png / decode_jpeg - and the picture-pixels form),
    src/effects/ (DrawingMask, DrawingEffects, DrawingFrameOperations, and
    surface_finish.rs: SurfaceFinish, CanvasWeave, the finish's pass data
    and CPU mirrors of the weave and the shading),
    src/svg/ and src/lottie/ (below), src/mask_meaning.rs, src/safe_areas/,
    src/models/ (DrawingLayer, ElementId, DrawingTransform, DrawingCamera,
    AnimatedProperty, BindingTarget, BoundTrack, AnimationBinding,
    DrawingAnimations and its editor DrawingAnimationsMut - its path,
    train and formation bindings in drawing_animations_motion.rs, its text bindings in
    drawing_animations_text.rs, the combining order in binding_combine.rs -
    PathTravelOptions, PathTrain, StaggerOrder / StaggerOptions /
    StaggerTracks, and the model tests), src/layout/ (DrawingLayout, LayoutAnchor,
    LayoutPosition, LayoutReference), src/drawing_session*.rs (with
    SessionRenderOptions, the painting exports and the layer transitions;
    drawing_session_blend.rs - SessionBlendMode -, drawing_session_paint.rs -
    SessionPaint, LinePaint -, drawing_session_wipe.rs - SessionWipe,
    WipeEdge -, and their tests drawing_session_looks_tests.rs),
    and src/rendering/ (CanvasCalibration;
    VectorScene, the low-level door the session records into; AnimationLayer
    with its control handle and RenderStats; picture_target.rs -
    PictureTarget, the canvas frame number every AnimationLayer publishes and
    the late-read rule; DebugOverlay; INTERNAL: scene_look.rs - where the
    looks are applied as the session records -, cpu_clock.rs - the one
    measuring clock) with the INTERNAL renderer core in src/rendering/core/
    (texture_ledger.rs among it: what is uploaded and the residency sweep).

TEXT BLOCKS, PATH QUERIES, COMBINED BINDINGS, FORMATIONS AND SAFE AREAS HAVE
TWO HALVES: their CALCULATION (the layout engine, reveals, staggers, captions,
text along a path; path queries, combined bindings, rolling and trains,
formations, safe areas) is checked value for value against NoSkia's own
output, and its drawing half sits beside it (the TextBlock shape kind and
its encoding, the session's layout_text / draw_text_block / draw_captions,
the reveal and stagger bindings, the text train, the safe-area overlay's
drawing) with path combining.
Keep the calculation as it is unless NoSkia changes; its tests pin NoSkia's
numbers.
  - TEXT BLOCKS (shapes/text_block_shape.rs): a filled block draws each glyph
    from the renderer's geometry cache under the same key as TextShape's
    glyphs; a partly revealed cluster draws with its alpha multiplied (no
    offscreen layer); an outlined or merged block draws its clusters' outlines
    as paths (OUTSIDE placement: the stroke in a layer of its own, the glyphs
    cut out destination-out, then the fill). merge_overlaps keeps each
    cluster's simplified outline in a cache shared by clones and pieces.
  - CAPTIONS: draw_captions is an AnimatedShape rebuilt from the presenter's
    placed captions at each time (the presenter caches its layouts and
    placements). Against the frame by default; the safe-area guide is the
    opt-in CaptionPlacement::from_guide (the owner's rule).
  - THE SAFE-AREA OVERLAY is drawn by DrawingSession::render_with after the
    layers and before the debug overlay, in the pixels of the aspect-fitted
    frame, each part (mask, ring) in one group at its alpha so overlaps never
    darken; MSAA edges (C# draws it without anti-aliasing).
  - PATH COMBINING (paths/path_clipper.rs, internal; the public calls in
    paths/path_geometry_combine.rs): NoSkia's PathClipper - its flattening in
    double precision (path_area_query.rs flatten_f64), its 1/1024 grid and
    range check, its Region / Combine / CombineAll rules and its STROKER (the
    owner's decision: NoSkia's own joins, not i_overlay's stroke or offset) -
    with i_overlay's integer overlay (i64) where NoSkia calls its polygon
    engine. i_overlay takes one fill rule for both operands, so operands of
    different fill rules are reduced to their regions first (the same area).
    Dashes are cut by kurbo's dash iterator before stroking (C# has its own
    Dash). The polygons cover NoSkia's areas; their start vertices and order
    are i_overlay's.
  - SVG DOCUMENTS (src/svg). Two halves, each its own files:
    THE CALCULATION (drawing_svg.rs, scene_builder.rs, scene_walker.rs,
    animation/): the document's text is prepared (ids for animated elements
    without one, `isolation:isolate` on animated shapes so that usvg gives
    them a group of their own - dropped again in the scene - and ids for the
    elements inside an animated element that set an inherited property
    themselves), read by usvg into a tree (fonts from the application's
    bytes; an href resolver of our own that never reads a file), turned into
    the crate's scene of parts, and walked per time with the document's own
    SMIL / CSS animation frame and the program's part overrides into a flat
    draw list. INHERITED PROPERTIES (fill, stroke, their opacities, stroke
    width, dashes, visibility) animated on a group stop at an element that
    sets the property itself (SvgScene::own_style, Inherited::stop in the
    walker) - what browsers do. Display hides a subtree; visibility is
    inherited and can be set back by a descendant. Animations whose target
    id usvg cleared (inside <use>d content) are named in the report
    (AnimationTargetInsideUse). THE DRAWING (svg_encode.rs,
    drawing_svg_session.rs): ShapeKind::Svg holds a DrawingSvgShape; its
    encode takes the draw list of the session's time (bindings applied to a
    copy of the parts) and maps each command onto the VectorScene: groups to
    push_layer (blend modes; the renderer keeps a group offscreen when a blend
    or a mask inside it needs that), the one plain blur to push_effects
    (radius = 2 x stdDeviation), a one-shape clip to a stencil clip, a
    several-shape clip to a group whose content is then kept destination-in
    by the shapes drawn together (their UNION; nesting one clip per shape
    would be their intersection), masks to a destination-in group clipped to
    the mask's region, fills and strokes with solid and gradient brushes
    (two-point radial for the focal point), images through fill_picture when
    the application supplied pixels. Text is usvg's: laid out by usvg and
    drawn as the paths of its flattened glyph outlines (the crate's own text
    engine is not used for SVG text, so SVG's layout rules - anchors, tspans,
    textPath, spacing - are usvg's). Host tests: svg/tests_scene.rs,
    tests_smil.rs, tests_css.rs, tests_draw.rs (the draw tests' text test
    reads the test project's Roboto).

TEXT, IN A PAGE. src/text and src/shapes/text_shape.rs.
  - FONTS FROM BYTES ONLY. FontRegistry::register_font takes the bytes of a
    font file (from the backend through the generated `_bytes` call, or
    packed into the frontend); there is no path or stream overload, no
    system font and no fallback font. The registry is a shared handle
    (Rc), so a session, its shapes and an overlay see a font registered
    later. Family names come from the font's name table (or an override);
    None means the first registered font; the errors are C#'s messages.
  - THE FACE (text/font_face.rs, internal): the read-fonts model harfrust
    re-exports (harfrust::font), which owns the bytes; the metrics the text
    needs; and two caches that live as long as the face - glyph outlines in
    FONT UNITS (y up, drawn once per glyph by the font reader, unhinted) and
    shape plans per direction, script and kerning switch. Each face has a
    program-wide number, the renderer's key for its glyphs.
  - SHAPING (text/text_shaping.rs, internal): ONE run per line, in the
    direction and script harfrust guesses from the first strong character
    (no bidi; mixed directions are not reordered). Kerning off = the `kern`
    feature off. Glyphs are grouped into clusters by harfrust's cluster value
    (a UTF-8 byte offset) - not NoSkia's heuristic of matching glyphs back to
    characters. Tracking is added per cluster, as NoSkia's TextOutline does.
  - MISSING CHARACTERS (the owner's rule; NoSkia has none of it): glyph 0
    marks a cluster as missing; its advance is the .notdef glyph's own, else
    the space's, else half an em; the characters the character map lacks
    (default-ignorable ones never count) are reported in text order.
    MissingCharacterMode: ShowTofu (default) draws glyph 0 when it has ink,
    otherwise the face's hollow box (two opposite contours inside the
    advance, baseline to cap height, kept in the outline cache under a
    reserved key); FontDefault draws glyph 0 as it is; Blank draws nothing.
  - DRAWING: a TextShape keeps (outline key, position) pairs; each glyph is
    recorded with VectorScene::fill_cached under a key made from the face's
    number and the glyph, with the transform that places and scales it (y
    flipped). So the renderer's geometry cache holds each glyph's flattened
    fan once per scale step and the batching joins glyphs that do not
    overlap; text costs no flattening from frame to frame. A gradient fill is
    in calibrated units: each glyph's brush transform places it back.
  - THE DEBUG OVERLAY (rendering/debug_overlay.rs): C#'s lines and plate
    geometry, in output pixels, drawn last by DrawingSession::render_with;
    it fails soft without a usable font.
  - FADES: DrawingSession::render_with draws the background fill OUTSIDE
    the session's opacity group (C#'s rule: the background is not faded),
    can leave the background out, and can draw into a part of the canvas
    (clipped to it), which is how one session is placed in another's frame.

THE ANIMATION MODEL AND PATHS, IN A PAGE. src/animation (KeyframeTrack and
Keyframe, TrackValue, TrackLoopMode), src/models, src/layout, src/paths.
  - IDS FOR REFERENCES. C# binds the element OBJECT; Rust shapes are values
    owned by their layer, so every DrawingShape carries an ElementId (a
    program-wide counter; a clone keeps it). A binding names its target by
    id, layer name or "the camera"; DrawingSession::set_time applies the
    bindings by finding each target (layers, then groups, depth first) -
    fine for the hundreds of elements a scene has.
  - THE BINDING EDITOR. session.animations_mut() returns
    DrawingAnimationsMut, which borrows the session, so a binding is checked
    against the element, layer or camera when it is made (C# checks the
    object it is handed). Replace, Add and Multiply bindings exist (C#'s
    BindingMode; the combining order is in binding_combine.rs); a property
    the target does not have is refused with a message. Callbacks (bind_callback) cannot reach the session.
  - set_time = C#'s Time setter: bindings in order, then every AnimatedShape
    rebuilt (layers and groups), then time windows apply when drawing.
  - THE BACKGROUND DEFAULT. DrawingSession::render draws NO background
    (SessionRenderOptions::default() has include_background false) - the
    owner's "on the page" rule; C# draws it by default. The demo's scenes
    call render_with_background.
  - DRAWING: an element's transform is multiplied in when it is encoded; a
    layer's matrix is calibration x camera(parallax) x layer transform (C#'s
    order). A group or animated shape below full opacity is pushed as one
    layer (the renderer folds a one-draw group into its paint).
  - PATHS. paths/path_measure.rs (internal) splits a kurbo BezPath into
    contours of segments with kurbo arc lengths (accuracy 1e-4) and gives the
    point and tangent at a distance and the segments between two distances;
    PathGeometry's get_point_at and trim, PathMorph's sampling and
    bind_along_path stand on it. The builder's rectangle, rounded rectangle
    and ellipse are written as C#'s PathGeometryBuilder writes them (start
    point, direction, the kappa cubics), so trims and morphs start where
    NoSkia's do. PathMorph is a line-by-line port of C#'s (pairing, sampling,
    shoelace direction, best shift). A PathShape with a morph keeps the
    in-between of its fraction as its geometry; the trim is applied when it
    is drawn.

THE RENDERER, IN A PAGE. src/rendering/core: build.rs (the CPU half),
paint.rs (paints, gradient ramps, the radial set-up), gpu.rs (pipelines,
targets, encoding, the testing readback), texture_ledger.rs (what the GPU half
uploads and keeps), and the WGSL modules gpu.rs assembles from pieces:
common.wgsl + noise.wgsl + paint.wgsl (the stencil and cover module),
common.wgsl + noise.wgsl + strokes.wgsl (the stroke sets), common.wgsl +
composite.wgsl (the compositing module) and effects.wgsl (the effect passes,
split by its `//@part` markers; the COMPOSITE part is built with noise.wgsl
too, for the caustic light). noise.wgsl holds the noise functions every
module shares (the 32-bit hash, the value and gradient noise, caustics_once):
one copy, so the paint, the strokes and the water draw the same noise.
  - STENCIL-THEN-COVER WITH 4x MSAA. kurbo flattens fills and expands strokes
    (caps, joins, miter limit, dashes) into a triangle fan; the fan's winding
    goes into the stencil's low 5 bits (wrapping), then a cover draw (the
    transformed local bounds) paints where the stencil says: non-zero by the
    count, even-odd by the parity bit. The top 3 stencil bits hold the clip
    depth (7 nested clips). A stroke thinner than one device pixel is drawn
    one pixel wide at a proportionally lower alpha.
  - CACHING. Outlines are flattened in the SHAPE'S OWN SPACE at a tolerance
    from the transform's scale (rounded up to a quarter octave) and cached
    under the elements' hash, the stroke style and that scale step, so a
    shape that moves, turns or scales a little reuses its fan. A draw that
    misses two frames running (a morph, a trim) is no longer stored. A cached
    fan unused for 90 frames is evicted, or at once when the cache holds
    three times one frame's use. A FIXED-WIDTH stroke
    (VectorScene::stroke_fixed_width) is expanded in the reference space (the
    session's calibration x camera), so its fan is keyed by the shape AS
    TRANSFORMED: a still element reuses it, a moving one is stroked every
    frame (and the two-frames rule then stops storing it).
  - BATCHING. Consecutive fills whose device boxes do not overlap share one
    stencil draw and one cover draw.
  - GROUPS (push_layer). Normal blending at full alpha is only a clip (no
    offscreen target), unless the group holds a non-normal group (an
    isolation pre-pass keeps those offscreen); a group of one draw puts its
    alpha into the paint; any other group is drawn into an offscreen 4x
    target (canvas-sized, one per nesting level, made on first use and again
    after a resize, cleared only where used; groups nest 3 deep, deeper ones
    are drawn as clips and counted "unsupported" in RenderStats) and
    composited per pixel (the mean of the 4 samples): fixed-function
    blending for the compose modes, and for a MIX mode a copy of the whole
    target below (WebGPU allows no partial MSAA copy) and vello's
    blend_mix_compose in sRGB-encoded values. Mix modes blend sRGB-encoded
    values (the W3C formulas, as NoSkia); normal blending is linear and
    premultiplied (the canvas host's convention).
  - PAINTS. Solid colours; gradients through a 512-sample ramp in a storage
    buffer (vello's make_ramp; straight interpolation for the session's
    gradient fills, as NoSkia; peniko's own choice, premultiplied by
    default, for peniko gradients; a fill's opt-in interpolation -
    with_interpolation - arrives as peniko's interpolation_cs, LinearSrgb /
    Oklab / Oklch, which make_ramp mixes premultiplied through the same
    function as painting::ColorMix, and the ramp cache's key includes it),
    at most 256 distinct ramps a frame, the
    two-point radial set-up from vello's draw_leaf.wgsl and the sweep from
    its fine.wgsl; images as Rgba8Unorm textures (sRGB bytes decoded in the
    shader, straight or premultiplied, BGRA swapped once), nearest or linear
    sampling, pad / repeat / reflect per axis.
  - THE FINAL STEP. The main 4x target is averaged onto the host's view over
    the drawn area. The layer never clears the view and draws over what the
    layers below drew. Every pipeline is made once per target format
    (CanvasRenderer::prepare).
  - PICTURES. An ImageShape or SpriteShape is a rectangle filled with an image
    paint placed by its source rectangle (pad extend), with an optional TINT
    (paint flag 4: four factors multiplying the straight sRGB values, alpha
    included, before premultiplying). A picture's texture is keyed by its
    pixels' blob, so clones share one upload.
  - THE RESIDENCY SWEEP (texture_ledger.rs, RESIDENCY; the decision is the
    host-tested sweep_choice). Pictures (ownership maps included), stroke
    sets' record buffers (and transitions'), and brush tips are swept EVERY
    frame by one policy: anything not drawn for more than 120 frames goes,
    and while what is held but not drawn this frame weighs more than 16 MiB
    (per kind), the least recently drawn of the entries not drawn in the
    last two frames go first. So a scene that makes a new 1 MiB picture
    every frame holds about 17 textures, not the 120 to 180 the old rule
    (unused over 120 frames, swept every 60) kept; a picture drawn every
    frame or every other frame is never dropped. Stroke CHECKPOINTS follow
    the same rule with their own budget (CHECKPOINT_RESIDENCY: 120 frames,
    128 MiB). The ledger also decides an ownership map's uploads (below).
  - LOOKS (scene_look.rs, set by DrawingSession::render_with): the paint
    override, the effects switch and the authored layers are applied while
    the session records into the VectorScene - at its fill, stroke, glyph,
    picture, noise, ownership, particle and stroke-set calls - so the scene
    itself never changes, and cleared after the render. Anything drawn
    inside a group composited destination-in / -out / -atop, source-in /
    -out / -atop, xor or clear keeps its colours (coverage, not paint). A
    picture's average colour (FLAT) is computed once per pixel blob and kept
    by the VectorScene (at most 512). A session's blend and opacity are one
    push_layer around the background and the layers; a soft wipe is the
    first render clipped plus one isolated group for the second, kept
    destination-in by a 9-stop smoothstep gradient (drawing_session_wipe.rs).
  - STROKE SETS (Op::Strokes, their own pipeline; strokes/stroke_gpu.rs's
    header comment holds the record and draw-block layouts - change the Rust
    and the WGSL structs together). One instanced draw per set: a triangle
    strip of 8 vertices per instance (the stroke's hull), instances 0..k
    where k is one past the last started record (a binary search on the
    CPU), stencil compared with the clip depth, premultiplied blending, 4
    samples, the empty hull fragments discarded. The records are uploaded
    once per set id (64 bytes each); per frame only a 256-byte-aligned draw
    block (the element's device matrix, the time, opacity, flags, boil, flow,
    the picked colour's parameters and the anchors' transforms) at a
    dynamic offset; its binding size follows from the header and the anchor
    limit and must stay within WebGPU's default uniform binding limit. The
    bind groups: 0 the frame's (view, paints, ramps); 1 the draw's textures
    (the tips array with its trilinear sampler, the colour source with its
    clamp-to-edge sampler; a 1 x 1 blank where none), made per (tips,
    source) pair and dropped after two unused frames; 2 the records (and a
    transition's target records); 3 the draw block. The picked colour and
    the colour rules run in the VERTEX shader, once per stroke vertex. A
    stroke's progress is exactly 1 from its end on (by a comparison, not the
    division), which is what lets the CPU know which strokes the GPU draws
    whole - the checkpoint cache depends on it.
  - THE CHECKPOINT CACHE (strokes/stroke_checkpoints.rs plans each frame,
    host-tested; gpu.rs splits the stroke draw): a checkpoint is a copy of
    the draw's own MULTISAMPLED target (all 4 samples; the targets have
    COPY_DST for it) at the point of the stroke draw, holding a prefix of
    the set whose every stroke has ended; restoring it and drawing the
    strokes after it repeats exactly the blends of the whole draw. The KEY,
    worked out after the build, covers the set, tips, colour source, the
    block apart from the time, the target level, the canvas size and
    everything encoded before the set in the frame (a picture target's or
    an application texture's water makes it unique to the frame). The rule
    (pixel for pixel the same frame) was proved in Chrome at top level, in
    a faded group and on a finished layer: change nothing here without
    proving it again the same way (two layers drawing the same session,
    with and without the cache, their pixels compared through the testing
    readback over a schedule of plays, holds and seeks).
  - PICTURE TARGETS (rendering/picture_target.rs; gpu.rs's picture final
    pass): a to_picture layer has its OWN renderer and buffers (two scenes
    in one frame must not share write_buffer'd buffers), clears its picture
    and draws into it at the picture's size; the picture is an Rgba8Unorm
    texture of premultiplied sRGB-encoded colour made on first use and
    shared by every reader. All canvas layers share one encoder, so a
    picture layer below its readers gives them the same frame's picture;
    every AnimationLayer publishes the canvas frame number so that a late
    read is detected and reported once.
  - THE SURFACE FINISH, THE WATER AND THE EFFECT GROUP: a finished or
    watery layer is drawn through its effect group (the group its effects,
    frame operations and transition ride on). Just before a finished
    layer's composite, ONE height pass per finished layer (single-sampled,
    no stencil, additive, into a canvas-size R16Float record) draws its
    stroke sets again (strokes.wgsl fs_stroke_height); the composite
    (effects.wgsl fs_composite) then applies, in order, the refraction's
    read offset, the finish, the water's light (shading, caustics, glints),
    the reflection, the frame operations, bloom, glow, shadow, transition
    and alpha. The group's region grows by the refraction's reach and by
    the reflection's mirrored part. The ripple fields of a frame go into ONE
    storage buffer written once a frame (each field's live drops and swell
    waves), each group's header into its pass data; the shader's
    ripple_ring / ripple_water are a line-for-line transcription of
    water/ripple_math.rs `evaluate`, and the host test runs a CPU mirror of
    the WGSL over the frame's real records - keep the three in step. The
    pass-data layout (FxPassData in build.rs and FxPass in effects.wgsl) and
    the composite's bindings grow with each part (frame operations, finish,
    water): their layout is stated beside the structs; change both sides
    together.
  - MASKS are made by the session's encoding, not by the renderer: "inside a
    path" is a stencil clip; "outside a path" and a layer matte are the
    content in a group followed by a nested group composited destination-out
    (or destination-in) holding the path or the matte's elements - the
    isolation pre-pass keeps the content group offscreen. A layer's (or an
    effect's, or a faded group's) own group is used as the content group, so
    a mask costs one level, not two.
  - THE RENDERER'S EFFECT PASSES (effects.wgsl; build.rs plans them, gpu.rs
    runs them). An element, group or layer with DrawingEffects is a group
    (Item::PushEffects) always drawn into an offscreen 4x level target. At its
    pop, build.rs plans, over the REGION (the content's device box grown by the
    effects' reach, C#'s Reach rule, in whole pixels): RESOLVE (the 4 samples
    averaged into an Rgba16Float texture, transparent outside the content),
    then for the content blur, the glow and the soft shadow - each only when
    present - a BLUR at the LEVEL its sigma asks for (1, 2, 4 ... 16 canvas
    pixels per texel, so that the kernel reads at most 8 texels a side; a
    box-filter DOWN pass first when the level differs from its input's), as
    a horizontal and a vertical pass with the planes' Gaussian kernel (C#'s
    PlaneKernels.GaussianKernel, uploaded per frame), then the COMPOSITE into
    the target below (shadow at the offset, glow times its strength capped at
    full, content over both, times the element's opacity; premultiplied
    blending; the stencil clip of the target applies). Glow and shadow are
    made from the BLURRED content's coverage, as in C#. Every pass reads only
    texels it knows to be valid (zero elsewhere): the textures are shared by
    every effect of the frame, one per role and level, made on first use.
    The pass parameters live in one storage buffer indexed by the quads'
    paint field. RenderStats counts `effects` and `effect_passes`.
    MEASURED COST (Intel UHD 770, headless Chrome, timestamp queries around
    the 2D layer, a full-canvas group of 41 shapes; "none" is the same group
    drawn directly, "faded" the same group at 50 % opacity - an offscreen
    group without effects; radii in calibrated units, so twice as many pixels
    at 2560 x 1440; the GPU's clock as sampled during each case):
                     1280 x 720 (300-1,400 MHz)  2560 x 1440 (950-1,450 MHz)
      none            3.0 - 3.5 ms                10.7 - 11.4 ms
      faded           8.0 - 9.4                   12.3 - 12.4
      blur 4          11.5 - 12.9                 14.0 - 14.1
      blur 8          10.6 - 12.1                 12.8 - 13.1
      blur 24         10.0 - 11.8                 13.1 - 13.2
      blur 60         11.4 - 11.8                 12.7 - 13.1
      glow 24         11.8 - 12.1                 13.0 - 13.2
      shadow 24       10.4 - 10.9                 12.7 - 12.8
      blur 6 + glow 24 + shadow 24
                      12.1 - 12.4                 12.9 - 13.1
    READING IT: a full-canvas effect costs about what a full-canvas faded
    group costs plus 1 to 4 ms; the radius hardly matters (wide blurs run
    coarse); the 1280 x 720 figures are at the low clock the GPU keeps for a
    light load, so they are larger than the work (the same work at 2560 x
    1440, four times the pixels, takes about as long at a high clock). With
    the earlier limit of 24 taps a side (a blur of a few pixels at full
    resolution) a 2560 x 1440 blur 4 took 40 ms and all three effects 65 ms:
    that is why the limit is 8. Effects on small elements cost in proportion
    to their region. Passes per effect: resolve 1, each blur 2 (+1 box filter
    when it runs coarser than its input), composite 1 - a blur alone 3 to 4,
    glow or soft shadow alone 3 to 4, all three 9 to 10 (RenderStats
    effect_passes).
  - THE TESTING READBACK (AnimationLayerControl::read_pixels_for_testing):
    the layer's own pixels of a coming frame, asynchronously; it exists only
    for the sample's run-checks (the owner's ruling: kept for testing only).

WHAT IS TAKEN FROM vello, AND HOW TO REFRESH IT. The crate takes code from
the vello crates vello_shaders 0.12.0 and vello_encoding 0.12.0 (Apache-2.0
OR MIT, used under Apache-2.0): blend.wgsl's mix and compose code COPIED
VERBATIM into composite.wgsl (from "// Color mixing modes" to the end, below
vello's copyright and SPDX lines); fine.wgsl's extend modes and its radial
and sweep gradient parameters TRANSLATED into paint.wgsl; draw_leaf.wgsl's
gradient set-up TRANSLATED into paint.rs; ramp_cache.rs's make_ramp ADAPTED
into paint.rs (f32 output instead of packed bytes). Inside fine.wgsl's sweep
code about ten lines are vello's "xy_to_unit_angle from Skia" (BSD-3-Clause).
Each file of ours says in its header what it holds from vello, and the
package's notices header lists every piece with its licence; keep the two in
step. vello is NOT a dependency and is not vendored: nothing waits for vello
releases. To take a newer vello's version of a piece (only on purpose, for a
fix or a feature): fetch that version's crates as crates.io publishes them
into a scratch folder outside the repository (never into the repository),
read their licence files and the file headers again, compare the pieces
above with ours, carry the change over (copied text stays verbatim; the
translations are ours), update the version in the file headers and in the
notices header, run the crate's unit tests, and check the "shapes" scene of
the GraphicsDemo "animation" application in a browser (its sample points
cover every fill rule, stroke, gradient, mix and compose mode, clip, group
and image).

WHAT THE UNIT TESTS OF codebrix-webgpu-animation COVER (on the host, run by
Animation.Tests from the materialized copy): every easing curve at several
points against values computed from NoSkia's formulas, Steps and the CSS
keyword curves, the cubic-bezier solver, ColorRamp's rounding, the
timeline's frames, timecodes, cues and markers (one from a named point of a
CanvasClock), playback (play, pause, seek, loop, speed, max step), the
canvas calibration, the paints and ramps and the radial set-up, the session
gradient fills against NoSkia's interpolation formula (straight sRGB and
straight alpha, a first stop past 0), the session's opacity group and the
render options and the background default, the renderer's CPU half:
batching, caching, folding of groups,
isolation, the mix backdrop, thin strokes, culling and the scale steps; and
TEXT with real fonts - the registry's rules and messages, kerning on and off,
a ligature as one cluster, UTF-8 byte offsets, tracking, alignment and bounds,
missing characters (the report, the advance, the three modes with Roboto,
whose .notdef is a box, and Open Sans, whose .notdef has no ink), a
right-to-left line, the glyph geometry key, and the debug overlay's lines and
its fail-soft rule; THE ANIMATION MODEL - float, point, colour and bool
tracks (per-segment easing by the later key, custom curves, holds, Loop and
PingPong time mapping, NaN and empty tracks), the transform matrix and
transformed bounds, the camera's matrix with parallax and its inverse,
bindings applied to elements, layers and the camera (and re-applied the same
when the time returns), the checks a binding gets when it is made, groups
(bounds, content colour, fading as one), time windows and animated shapes,
the layout's placement against a safe rectangle; and PATHS - lengths and
points of rectangles, circles, stars and several contours from C#'s start
points, trims (reversed, empty, wrapping on an open path and round a closed
one), trimmed and morphing path shapes, the morph's pairing, start and
direction matching and growing of a missing contour, and motion along a
path with and without facing along it; PICTURES AND SPRITES - buffers and
the BGRA swap, crops, grids (remainders, partial last rows, refusals), the
three playback modes' frame mapping and a sprite's frame as a function of the
time (C#'s 1e-6 nudge, a delayed hold, a fixed index); MASKS AND EFFECTS -
the Gaussian kernel against C#'s formula, the reach, the level a wide blur
runs at, Measure's radius scale and shadow offsets (fixed in the scene,
turning with the element, a stretch, the camera's roll), the device
parameters, the planned passes of an effect group and its region, and the
effect, frame-index and tint bindings (a negative radius from a track counts
as 0; effects start from NONE); TEXT BLOCKS, CAPTIONS AND STAGGERS
(text/layout_tests.rs, captions/tests.rs, models/stagger_tests.rs: NoSkia's own
tests' values and values from its formulas with the shaped advances - line
breaking, wrapping, alignment, justification, trimming and ellipses, runs,
pieces, lookups, text along a path, reveals, the SRT and WebVTT parsers, the
presenter's placement against the frame and in a guide, the stagger tracks);
COMBINED BINDINGS, FORMATIONS, QUERIES AND SAFE AREAS
(models/tests_step12.rs, formations/tests.rs, planes/tests.rs, safe_areas/tests.rs, paths/tests_queries.rs: pinned to NoSkia's own output -
formations, orders and ties, schedules, falls and spirals, mask distances,
noise, the flattening, the queries, combined bindings and their order, rolls
and trains, safe-area guides, the check and the overlay's plan); PATH
COMBINING (paths/tests_combine.rs: areas worked out by hand from NoSkia's
rules - the four operations, differing fill rules, combine_all, simplify, stroke
outlines with each cap and join and NoSkia's chord counts, dashes, dots,
inflate and deflate, the grid, the messages); and THE DRAWING HALF
(shapes/tests_text_block.rs: what a block, a reveal, an outline, merged
overlaps, captions and the overlay record, and the reveal, stagger and
text-along-a-path bindings); and, beyond NoSkia (no C# reference: values
from the design's formulas, the counts and layouts the GPU half is handed,
and CPU mirrors of the shaders' arithmetic), the const easing curves,
RenderStats' timings, variable fonts with the real variable Roboto (its
axes, advances and outlines per weight, the refusals), fixed-width strokes
and the SVG reader's vector-effect, the session's blend and opacity, the
looks, the effects switch, the exceptions and the coverage rule, wipes,
ownership uploads through the texture ledger, the sketch helper, and the
painterly and water areas listed under PIXEL PLANES, THE GPU HALVES AND THE
PAINTERLY PLAYBACK below. THE TEST FONTS are not in the crate (OFL fonts may never
be in a package or a crate): they are
tests/CodeBrix.WebPlatform.WebGpu.Animation.Tests/Fonts/ (Roboto - the
static Roboto-Regular.ttf and the variable Roboto.ttf, which the
variable-font tests use, beside the one Roboto OFL.txt - and Open Sans with
its OFL.txt; both families unchanged from the owner's
CodeBrix.Platform.Fonts.* repositories), copied beside the test assembly; Animation.Tests
passes their folder to `cargo test` in the environment variable
CODEBRIX_ANIMATION_TEST_FONTS and checks that the text tests ran. Without the
variable the tests look for that folder from the crate's place in the
repository and skip (with a line saying so) when it is not there. THE LOTTIE
TRIAL FILES are not in the crate either (the crate travels inside the package;
only small hand-written files sit in src/lottie/test_files): the three large
invented trial files and the sample's own features file are the GraphicsDemo
sample's assets (samples/GraphicsDemo/backend/GraphicsDemo.Assets/Lottie),
which Animation.Tests copies beside its assembly and passes to `cargo test`
in CODEBRIX_ANIMATION_TEST_LOTTIE, checking that the Lottie tests ran;
without the variable those tests print "SKIPPED: no Lottie trial file" and
pass. What needs a GPU -
pipelines, drawing, the readback - is proved by the sample's "animation"
application in headless Chrome (each scene drawn, moving and still while the
clock is paused, and the "shapes" scene's sample points through the testing
readback; its MAINTAINER-README.txt says how) - and so, for this round's
GPU work, are the stroke sets, the colour pickup, the finish's height pass,
the checkpoint copies (pixel equality with the cache on and off) and the
water, each first run in headless Chrome with the device's WebGPU errors
read every frame (THE ANIMATION CRATE: every shader change is proved in a
browser).


PIXEL PLANES, THE GPU HALVES AND THE PAINTERLY PLAYBACK
  THE CPU HALVES are checked value for value against NoSkia's
  own output (planes/tests_reference.rs and painting/tests_reference.rs assert hashes of what a
  reference program running the NoSkia package printed; planes/tests_planes.rs and
  painting/tests.rs carry NoSkia's own tests with their stated values). Do not "tidy" that code:
  a changed loop order changes the hashes. Everything there is ONE thread.
  THE STROKE BAND of the hard-edged rule is path_clipper::stroke_band_pieces: NoSkia's stroker's
  pieces WITHOUT their union (filled non-zero they cover the union's area; the union would round
  crossing points differently). The Gaussian kernel of the effects is the planes' one
  (effect_sizes::gaussian_kernel forwards to plane_kernels::gaussian_kernel).
  THE GPU HALVES (rendering/core), each a small addition to the existing renderer:
    - FRAME OPERATIONS AND TRANSITIONS ride on the EFFECT GROUP of a layer: EffectParams gained a
      `frame` part (FrameParams) and FxPassData vectors of its own (as later the surface finish's
      `finish` part and the water's `water` part did; the layout is stated beside FxPassData in
      build.rs and FxPass in effects.wgsl); the effect composite
      (effects.wgsl fs_composite) applies the tone curve, gamma and vignette to the straight sRGB
      colour, adds a bloom (a blur of the content planned into the glow's texture - so a layer with
      a glow draws no bloom), and weighs the result by the transition (dissolve, wipe, fade through
      a colour). A layer in a transition composites over its whole frame (the region is grown to
      the frame). Slides and pushes are not passes: the session shifts the layers by whole device
      pixels inside a clip of the frame (drawing_session_transitions.rs).
    - NOISE is a paint kind (KIND_NOISE, paint.wgsl noise_value): evaluated per fragment in the
      shape's calibrated space, mapped through a gradient ramp slot. 32-bit hashing, so NOT the CPU
      pattern for the value and gradient kinds; the planes' parameters and octave rule. THE CAUSTICS
      KIND (DrawingNoiseKind::Caustics, paint mode bit 4, noise.wgsl caustics_once /
      caustics_cell_point) has no C# original, so its CPU code (planes/drawing_caustics.rs) IS the
      GPU arithmetic - f32, the shared 32-bit hash painting::color_rules::noise_hash, the seed's LOW
      32 bits (NoiseShape passes `seed as u32` for this kind; the other kinds keep their xor fold) -
      and both draw the same pattern. Its constants are shared between the two files: change them
      together.
    - PARTICLES are one operation (Op::Particles) drawn by their own pipeline: instance data
      (centre, the disc's two device axes, colour, softness; 48 bytes), a quad per instance from
      the vertex index, the stencil compared with the clip depth as every composite is, the fade
      and the edge's anti-aliasing in the fragment shader (fwidth).
    - OWNERSHIP MAPS are a paint kind (KIND_OWNERS) over an ordinary picture: the map's owner
      numbers (index + 1 in R G B, 0 = nobody) and, in the rows below, the colours; the shader
      looks the colour up with textureLoad and softens the boundary over 3 x 3 (weights of a
      sigma-0.8 Gaussian cut at radius 1). OwnershipShape::new makes a new picture (a whole
      upload); OwnershipShape::with_colors shares the picture of owner numbers and sends ONLY ITS
      COLOUR TABLE (the map's width x ceil(elements / width) rows x 4 bytes, written into the
      existing texture at row = map height). The rules are decided by the CPU-side ledger
      (texture_ledger.rs) that gpu.rs follows; two colourings of one map in one frame need two
      textures (all of a frame's uploads land before its draws). Measured before the change: every
      colours-only change of the sample's 560 x 420 map uploaded 943,040 bytes, where its colour
      table is 2,240.
    - Plane masks are a picture (white, alpha = the plane's bytes) composited destination-in (or
      -out) over the masked content, the same place as a layer matte.
  THE REPORTS: an unknown matte layer name goes into the session's report list once
  (DrawingSession::take_reports). The device's shared WebGPU error queue cannot take it: its
  publish is crate-internal to codebrix-webgpu, and a new public method there would mean a new
  codebrix-webgpu version, which the Graphics crate's exact pin would then conflict
  with. An application shows or logs the reports beside the device's errors.
  WHAT THE TESTS COVER: rendering/core/tests_gpu_half.rs (noise paint, the particles operation,
  frame parameters, transitions' recorded items, brushstrokes painting on, the ownership
  picture's layout), effects/tests_step9.rs (plane masks, the matte report), images/tests_png.rs
  (one small PNG constant through from_png, and the picture-pixels form - the PNG rules
  themselves are tested in codebrix-webgpu, whose decoder from_png calls), images/tests_jpeg.rs
  (from_jpeg on a 336-byte greyscale JPEG constant, codebrix-webgpu's fixture made from
  synthetic pixels), rendering/core/tests_ownership_upload.rs (a colours-only change uploads the
  colour table and nothing else, through the session and the renderer's CPU half over several
  frames; the ledger keeps and evicts), rendering/core/tests_strokes.rs and strokes/tests_gpu.rs
  (the stroke operation, the record and draw-block packing, CPU mirrors of the shader's outline,
  tip, boil and flow arithmetic, the sweep policy), strokes/tests.rs (records and sets),
  strokes/tests_color_gpu.rs (CPU mirrors of the stroke shader's pickup and colour rules against
  ColorRules::apply_float and the Oklab mixing, within float rounding), strokes/tests_checkpoints.rs
  and rendering/core/tests_checkpoints.rs (the plan of each frame, the refusals, the keys),
  strokes/tests_morph.rs (pairing, slots, the interpolation, the two orders),
  painting/tests_fields.rs, tests_timeline.rs and tests_color.rs (direction fields - a stroke's
  two tips and middle on one circle on a circular field -, guides, detail maps, layouts, the
  timeline's schedule and cues, colour mixing and the rules), effects/tests_surface_finish.rs (the
  finish's checks, pass data and CPU mirrors of the weave and shading), water/tests.rs (the ripple
  field: the front, trough and crest each at its true place, rain windows, walls, the cap, the
  exact Laplacian, the same water however a time is reached), planes/drawing_caustics.rs's own
  tests (the range, no seams, 3 x 3 cells against 5 x 5, the seed's low 32 bits, the shader's
  values), water/tests_water_gpu.rs (a CPU
  mirror of effects.wgsl's water code over the frame's real records, EQUAL to
  RippleField::height_at / slope_at / focus_at; region growth, nesting, the reflection),
  paths/tests_fixed_width.rs and paths/tests_sketch.rs, drawing_session_looks_tests.rs (blend and
  opacity, the looks, the effects switch, the coverage rule, wipes and their nesting cost).
  WHAT ONLY A BROWSER PROVES: everything the GPU does - every pipeline and shader actually
  compiling in a browser, the drawn pixels, the colour-table write into an existing texture, the
  stroke draw, the height pass, the checkpoint copies and their pixel equality, the water's per
  pixel evaluation, a picture target read the same frame, an application's texture bound as a
  source. The host tests prove the decisions, the counts, the layouts and the arithmetic (through
  the CPU mirrors); they cannot run a GPU. So every WGSL change is run in Chrome as well as
  validated by naga (in a scratch checker): naga accepts things Chrome rejects (THE ANIMATION
  CRATE above).
  CPU TIMINGS (a native release build, one thread, 1280 x 720; a browser is
  slower): plane arithmetic 0.2-0.7 ms; an image blur 22-30 ms; an image bloom 14-26 ms; noise
  fills 15 ms (2D, one octave) to 107 ms (3D, four octaves); transitions of images 1-4 ms; an
  ownership scene re-coloured every frame 27 ms (5.7 ms at 560 x 420); one keyframe painting
  14.5 ms; a dissolve between keyframes 3.9 ms. Hence the GPU halves.
  MEASURED IN THE BROWSER (headless Chrome on this laptop's integrated GPU, the sample's scenes at
  1280 x 720, 120 frames each from requestAnimationFrame timestamps): every pixel-plane and
  painterly scene holds 60 fps (mean 16.7 ms, worst 16.8 ms, none over 20 ms). Before the colouring moved to the
  GPU the "ownership" scene (two 560 x 420 maps made and coloured on the CPU every frame) measured
  mean 26.9 ms, p95 50 ms, 49 of 120 frames over 20 ms; with OwnershipShape 16.7 / 16.7 / 0. The
  "motion-blur" scene's 32 Plus-composited groups measured mean 49 ms while each group's shape was
  the whole canvas (a compose other than source-over composites its WHOLE shape) and 16.7 ms with
  the groups kept to their panels - give such groups the smallest shape that holds them.


LOTTIE IN THE ANIMATION CRATE
=============================
src/lottie/ of codebrix-webgpu-animation reads a Lottie document's
JSON through velato 0.12.0 (DEFAULT FEATURES OFF: its `vello` feature is never
turned on) and plays it as an element of a session. It is built in two halves,
as the other areas are: THE DATA HALF (document_check.rs - the crate's OWN pass
over the JSON before velato reads it; drawing_lottie.rs; recording_sink.rs -
velato's RenderSink onto a plain-data LottieDrawList; group_folding.rs; the
playback options, ranges and markers) and THE DRAWING HALF (lottie_element.rs:
DrawingSession::draw_lottie, ShapeKind::Lottie, DrawingShape::as_lottie, and
the list mapped command by command onto VectorScene - groups with their blend
mode, clips, non-zero fills, strokes, pictures for image layers whose pixels
the application handed over with DrawingLottieShape::set_image). The element
evaluates its document at the session's time in every frame; velato's
Renderer is scratch storage only (a RefCell inside the shape; a clone gets its
own). Markers become the clock's named points through
DrawingLottieShape::marker_times.

velato is third-party and used EXACTLY as published. Its importer panics, and
its reader refuses whole files, on ordinary content (the list is in
AGENT-README.txt, LOTTIE ANIMATIONS); the pass prevents every case found and
reports it; tests_load.rs runs velato ALONE on three such documents and sees
it panic - if a newer velato stops panicking, that test says so. What the
module draws, approximates and leaves out is listed exactly in AGENT-README;
report() names what a given file uses that is not drawn as written. dotLottie
archives are not read (the backend unpacks them). The tests: tests_load.rs,
tests_playback.rs, tests_draw.rs (the data half, values computed by hand from
the JSON) and tests_element.rs (the drawing half: one renderer call per
command, the element's transform and opacity, image pixels, Hold, markers on
a CanvasClock, the sample's features file).


NOTICES
=======
THE ROOT THIRD-PARTY-NOTICES.txt is packed into every package, so it is short
and written by hand: what the LICENSE covers, an entry for each set of
vendored crates a package carries (wgpu, glam, png and zune-jpeg in the
WebGpu package; the gltf crates in the Graphics package; kurbo and peniko,
harfrust, i_overlay, usvg and velato in the Animation package) with a
pointer to the GENERATED notices that travel with them, an
entry for each outside source of a first-party crate (vello and Skia in the
Animation crate's renderer; the tone-mapping curves and the .hdr decoder in
the Graphics crate), the fonts of the tests and the sample, and the glTF
sample models of the reference converter's tests. Never paste generated
text into it. Test material that is never packed (fonts, models) is listed
there too, with its licence.

THE GENERATED FILE Rust/notices/THIRD-PARTY-NOTICES-WebGpuCrates.txt is the
hand-written header notices-header-WebGpuCrates.txt plus what the gate's
licence audit produces (`LibraryChecker notices --write`); edit only the
header. It is packed into the WebGpu package and handed to applications by
WebGpuCrates.ReadNotices. After regenerating, read the difference.

GLAM'S ATTRIBUTION. glam's ATTRIBUTION.md names code ported from DirectXMath,
Realtime Math and GLM, each under the MIT License. The notices generator does
not read ATTRIBUTION.md, so those three notices are carried, verbatim, in the
hand-written header; keep them there while glam is vendored, and re-read the
file when glam is upgraded. codebrix-webgpu is named in the header as
first-party and is not listed. The same header carries the paragraphs PNG
FILES (png and its tree, png's public-domain Paeth predictor, zlib-rs carried
but not compiled and the test PDF it carries) and JPEG FILES (zune-jpeg and
zune-core, and the stb_image material zune-jpeg names).

THE ANIMATION PACKAGE'S NOTICES,
src/CodeBrix.WebPlatform.WebGpu.Animation/Rust/notices/
THIRD-PARTY-NOTICES-WebGpuAnimationCrates.txt, are GENERATED like the WebGpu
package's: the hand-written header notices-header-WebGpuAnimationCrates.txt
(the crate is ours; the code its renderer takes from vello, used under
Apache-2.0, and the ten lines inside it that vello marks as Skia's, with
Skia's BSD-3-Clause text verbatim; a credit for kurbo's cu2qu port; credits
for HarfBuzz and the note on read-fonts' Adobe Glyph List, compiled only with
a feature kept off; part D, the Unicode License V3 text verbatim for the
Unicode data in harfrust's tables - the gate demands both licence texts; and
parts E to G on path combining (i_overlay and its crates, the licence texts
of the two that publish no licence file, libm's source notices), SVG
documents (usvg and its tree) and Lottie documents (velato and serde_repr))
plus
the licence audit of the package's OWN third-party crates (`LibraryChecker
notices --write`); the crates it shares are listed in the WebGpu package's
or the platform's notices, not here. Edit only the header. It is handed to
applications by AnimationCrates.ReadNotices.

THE GRAPHICS PACKAGE'S NOTICES,
src/CodeBrix.WebPlatform.WebGpu.Graphics/Rust/notices/
THIRD-PARTY-NOTICES-WebGpuGraphicsCrates.txt, are GENERATED like the
Animation package's: the hand-written header
notices-header-WebGpuGraphicsCrates.txt (the crate is this repository's
own; its dependencies' notices come with the WebGpu package and the
platform; the three tone-mapping sources and the .hdr decoder's source with
their licences - the BakingLab MIT text verbatim, as that repository's
LICENSE has it; and part 5 on the glTF crates: gltf, gltf-json and
gltf-derive, gltf-derive's validator_derive notice, lazy_static, inflections
and its carried script) plus the licence audit of the package's OWN
third-party crates (`LibraryChecker notices --write`). Edit only the
header. It is packed beside the archive and handed to applications by
GraphicsCrates.ReadNotices. Nothing taken from another CodeBrix repository
gets an entry (the owner's rule), only what that repository took from
outside.

The root LICENSE covers the code written here. Do not describe the
repository as "Apache-2.0" without that qualification anywhere.


GIT AND THE VENDORED FOLDER
===========================
Two things would silently break a clone, so the repository guards against
both:
  - LINE ENDINGS. .gitattributes keeps text files LF everywhere (so a
    Windows clone is no problem), declares the binary types, and marks the
    three vendor trees (src/CodeBrix.WebPlatform.WebGpu/Rust/vendor,
    src/CodeBrix.WebPlatform.WebGpu.Graphics/Rust/vendor and
    src/CodeBrix.WebPlatform.WebGpu.Animation/Rust/vendor) `-text`, so git
    never touches a byte in them (some upstream files have Windows line
    endings, and their checksums cover the exact bytes), and the reference
    converter's test model packs
    (tests/CodeBrix.WebPlatform.WebGpu.Graphics.Tests.GltfReference/Models)
    the same way (model files kept exactly as obtained).
  - IGNORED FILES. The last lines of the root .gitignore un-ignore
    everything inside the three vendor trees, one line each, because a
    general ignore rule (bin/, debug/, *.log ...) would otherwise hide a
    file a vendored crate needs. The gate asks git and fails if any such
    file is ignored. Check by hand with `git status --ignored --porcelain`:
    nothing inside a vendor tree may be listed. Keep those lines LAST (a new
    vendor tree adds its line there).

Never commit: any `target` folder (the three crate sets', codebrix-webgpu's,
codebrix-webgpu-graphics' and codebrix-webgpu-animation's), the three
first-party crates' Cargo.lock, bin/ and obj/; the .gitignore at the
repository root covers them. What the GraphicsDemo sample's builds write
needs no rule there: every folder the platform's build writes into a
frontend (platform/vendor, platform/codebrix-webplatform, src/generated,
.cargo, assets/generated, target) and the backend's frontend-built/ holds a
.gitignore of its own that ignores the folder (a `cargo clean` removes
target/'s, and the next materialize writes it back); the root .gitignore has
no GraphicsDemo lines (the sample's MAINTAINER-README.txt says what is
build output).


THE PLATFORM'S PACKAGES
=======================
The CodeBrix.WebPlatform packages are PUBLISHED and come from nuget.org, like
every other package; this repository never builds against unpublished
platform code. Their version is set in:
  - the Core pin of the three library projects (exact: Version="[...]");
  - every FrontendBuilder reference: the maintainer tool's project and the
    test projects that reference it;
  - WebPlatformPackageVersion in samples/GraphicsDemo/build/
    GraphicsDemo.BuildFrontend/GraphicsDemo.BuildFrontend.csproj and
    samples/GraphicsDemo/backend/GraphicsDemo.Api/GraphicsDemo.Api.csproj.
All of them name the ONE version the platform's packages were published at.

WHEN THE PLATFORM IS REPUBLISHED: set the new version in all of those,
re-resolve the crate set against the new platform lock, run the gate, and
rebuild; then publish this repository's three packages again (their Core
pin is exact) and set their new version in the sample (THE SAMPLE'S
PACKAGES, below).


THE SAMPLE'S PACKAGES
=====================
samples/GraphicsDemo takes this repository's three packages, like the
platform's, as PUBLISHED packages from nuget.org. There is no local package
feed: no nuget.config and no temp-packages/, here or in the sample. Their
ONE version is set in WebGpuPackageVersion in
samples/GraphicsDemo/build/GraphicsDemo.BuildFrontend/GraphicsDemo
.BuildFrontend.csproj and in
samples/GraphicsDemo/backend/GraphicsDemo.Assets/GraphicsDemo.Assets.csproj
(the two must name the same version).

WHEN A LIBRARY OF THIS REPOSITORY CHANGES, the sample does not see it until
the three packages are published again: publish them from ONE build
(PACKAGING AND PUBLISHING), wait until nuget.org serves all three, then set
the new version in both of the sample's project files, and build and test
the sample. If a crate's version changed, the sample's next build brings
its two marked Cargo.lock files in step by itself and says so (THE
FIRST-PARTY CRATE, ITS VERSION); check that line in the build's output.
A crate published AGAIN under the SAME crate version (its content changed
between two releases of the packages while its version stayed) is not
rebuilt by cargo: it treats a vendored name and version as unchanging and
keeps the earlier build in the frontend's target folder, so the sample's
build then fails on what the new code added (or, worse, runs the old code).
Bump the crate's version when its compiled content changes; otherwise
delete that crate's build output - for the Animation crate
samples/GraphicsDemo/frontends/animation/target/
wasm32-unknown-unknown/wasm-release/build/codebrix-webgpu-animation (the
frontend's target/ ignores itself in git; a `cargo clean` removes its
.gitignore and the next materialize writes it back) - and build again.


PROVENANCE AND VENDORED SOURCES
===============================
The assemblies contain no third-party source. The repository CARRIES
third-party code outside the assemblies: the vendored crates in
src/CodeBrix.WebPlatform.WebGpu/Rust/vendor, shipped in the WebGpu package
as an archive, in src/CodeBrix.WebPlatform.WebGpu.Graphics/Rust/vendor,
shipped in the Graphics package's archive, and in
src/CodeBrix.WebPlatform.WebGpu.Animation/Rust/vendor, shipped in the
Animation package's archive, each recorded in its package's generated
notices (NOTICES above). The test material that is never packed - the
fonts of the Animation tests and the glTF sample models of the reference
converter, each with its licence file - is listed in the root
THIRD-PARTY-NOTICES.txt. codebrix-webgpu-animation, shipped in the
Animation archive, is first-party; the code its renderer takes from vello
(and the few lines of it that vello took from Skia) is listed in the
Animation package's notices header.
codebrix-webgpu, shipped in the same archive, is first-party, and so is
codebrix-webgpu-graphics (shipped in the Graphics package's archive), whose
WGSL is copied from the owner's own C# library (THE WGSL COPY) with our own
additions and shaders, and whose three tone-mapping curves and .hdr decoder
are listed in the Graphics package's notices.


CODING CONVENTIONS
==================
.NET
  - Target net10.0. Nullable reference types are OFF: never '?' on a
    reference type, never the '!' operator. Value-type nullables are fine.
  - No <ImplicitUsings>, no global usings. Every file lists its own usings at
    the top, System.* first, then the rest alphabetically; no usings below
    the namespace line.
  - File-scoped namespaces. One type per file. Sources in sub-folders that
    match their namespaces; Internal/ for internal helpers.
  - XML doc comments on every public and protected member.
  - Each packable library has InternalsVisibleTo.cs at its project root
    naming its test project.
  - Tests: files named <ClassUnderTest>Tests.cs; methods
    MemberName_snake_case_description or snake_case_description;
    multi-statement tests carry //Arrange //Act //Assert comments and
    single-statement tests are expression-bodied; assertions in the
    SilverAssertions fluent form; TestContext.Current.CancellationToken
    passed wherever xUnit asks for it.

Rust (codebrix-webgpu)
  - A `//!` comment at the top of every file saying what it is; a doc
    comment on every public item; errors and messages that say what to do.
  - Edition 2021; every dependency at an exact version (THE FIRST-PARTY
    CRATE); no JavaScript, no wasm-bindgen snippets, no CSS class names.
  - 0 warnings from `cargo check --target wasm32-unknown-unknown` and from
    its unit tests on the host; CPU-only logic in its own module with
    `#[cfg(test)]` tests, so that it is tested on the host.

Documentation
  - AGENT-README.txt is consumer-only and carries NO version numbers and no
    counts.
  - README.md follows the family template: no upstream references, no
    versions, sizes or dates that go stale, no history.
  - Describe policy, not counts that change.
  - Plans and status records do not belong in the repository.


NOTES
=====
  - The family's document collector reads README-INDEX.txt; keep every path
    it lists real.

================================================================================
END OF MAINTAINER-README
================================================================================
