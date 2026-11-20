================================================================================
UPGRADE NOTES: wgpu (the WebGPU API for Rust)
================================================================================

Read the generic procedure, UPGRADING A VENDORED LIBRARY in the
MAINTAINER-README.txt at the repository root, first. This file holds what is
particular to this library.

WHERE IT COMES FROM
-------------------
The `wgpu` crate on crates.io (https://crates.io/crates/wgpu), from the wgpu
project (https://github.com/gfx-rs/wgpu). Licensed MIT OR Apache-2.0. It is
NOT patched: its source, and that of every crate its resolution names, is in
../../vendor exactly as crates.io published it (`cargo vendor --versioned-dirs`
form, one name-version folder per crate, each with its .cargo-checksum.json).

Only the crates.io packages are vendored, never the upstream repository: the
repository's examples, tests and artwork carry files under other terms or
under no stated licence, and none of that is in the published crates.

Only stable releases are used. Never take an alpha, beta or release
candidate. Take the latest released version when upgrading.

WHAT IS PINNED
--------------
../../webgpu-crates/Cargo.toml (the crate set) names `wgpu` at an exact
version (`=x.y.z`) with the default features off and the features `std`,
`webgpu` and `wgsl` on. The comment in that file says why each feature is on
or off. The target is a browser build (wasm32-unknown-unknown) with the WebGPU
backend only: no WebGL, no native backend. With the default features off no
native graphics API crate enters the lock.

../../webgpu-crates/Cargo.lock fixes the version of every other crate.

HOW THE VENDORED SET IS DERIVED
-------------------------------
The vendored set is EVERY package the crate set's Cargo.lock names, which is
exactly what `cargo vendor --locked --versioned-dirs` writes for that lock.
It is not reduced to what a browser build compiles, and it must not be:

  - A platform frontend resolves its own Cargo.lock offline from ONE vendor
    folder (the platform's crates plus every registered bundle's). Cargo
    resolves a lock for all targets and all optional dependencies that the
    features in use can reach, so it needs crates a browser build never
    compiles (for wgpu: wgpu-core, wgpu-hal, naga and their dependencies,
    which wgpu depends on for native targets). Without them cargo stops with
    "no matching package named ... found", both when the frontend has no
    Cargo.lock yet and when it has one. This was established by experiment
    when the set was first vendored: removing any single one of the crates
    that the browser build does not compile broke both cases.
  - So the folder holds more than is compiled into an application. Which
    crates ARE compiled into a wasm32 application is what
    `cargo tree --target wasm32-unknown-unknown -e normal,build` lists for the
    crate set; the notices say which crates are compiled in and which are
    carried only.

THE LOCK IS RESOLVED AGAINST THE PLATFORM'S LOCK
------------------------------------------------
The CodeBrix.WebPlatform platform's strict rule for crate bundles: a crate
this bundle shares with the platform must be at the platform's exact version
with identical content (.cargo-checksum.json), and the bundle must not carry
any version of a crate that the platform carries in a different version. A
consumer's frontend build otherwise stops with a FrontendBuildException.

So the lock is never resolved fresh. It is seeded with the platform's lock:

 1. In a scratch folder outside the repository, put a copy of
    ../../webgpu-crates/Cargo.toml, its src/lib.rs, and a copy of the
    platform's Cargo.lock (the one of the platform version that the package's
    exact CodeBrix.WebPlatform.Core pin names; in the CodeBrix.WebPlatform
    repository it is
    src/CodeBrix.WebPlatform.FrontendBuilder/Rust/platform-crates/Cargo.lock)
    as that folder's Cargo.lock. Do not copy the .cargo folder: this step
    needs crates.io.
 2. Let cargo resolve there (`cargo metadata --format-version 1`, or any
    command that resolves). Cargo keeps every locked version that still
    satisfies the requirements, adds only what is missing, and drops the
    platform's packages the crate set does not use.
 3. `cargo vendor --locked --versioned-dirs <empty folder>` there, and
    replace ../../vendor and ../../webgpu-crates/Cargo.lock with the result.
 4. Compare with the platform: every vendored crate whose name the
    platform's vendor folder holds must be there at the same version with an
    identical .cargo-checksum.json, and no vendored crate may be at a version
    of a name the platform holds in other versions only. The maintainer tool
    checks this (`LibraryChecker`, check group `platform`).
 5. Check that the lock resolves offline from the vendored folder: in
    ../../webgpu-crates, `cargo metadata --offline --locked --format-version 1`
    and `cargo check --offline --locked --target wasm32-unknown-unknown`
    (with CARGO_TARGET_DIR outside the repository).

If step 4 fails, a requirement of the new wgpu cannot be met by the version
the platform carries. Do not vendor: the wgpu upgrade has to wait for a
platform release whose crates satisfy it, or the owner decides otherwise.

WHAT MUST MOVE TOGETHER
-----------------------
  - THE PLATFORM. The package pins one exact CodeBrix.WebPlatform.Core
    version, and the crate set's lock is resolved against that platform
    version's lock. EVERY republish of the platform means: move the Core pin,
    re-resolve from the new platform lock (above), re-vendor, run the gate,
    republish. A shared crate the platform moved (wasm-bindgen, web-sys,
    js-sys, hashbrown, ...) otherwise breaks the strict rule in every
    consumer's build.
  - The wgpu project's own crates are released together at one version
    (wgpu, wgpu-types, wgpu-core, wgpu-hal, naga, naga-types and their
    helpers). Let cargo resolve them; never pin one of them separately.
  - wasm-bindgen, web-sys, js-sys and wasm-bindgen-futures are the
    platform's. wgpu states minimum versions of them; if a new wgpu needs a
    newer one than the platform carries, see "If step 4 fails" above.
  - The first-party crate codebrix-webgpu (../../codebrix-webgpu) names
    `wgpu` at an exact version: move it with the crate set, change the
    crate's own version, and build it (the materialize tests, the
    GraphicsDemo sample in a browser). The Graphics crate
    (src/CodeBrix.WebPlatform.WebGpu.Graphics/Rust/codebrix-webgpu-graphics)
    names `wgpu` and codebrix-webgpu at exact versions too: move both, change
    its own version, run its unit tests and the GraphicsDemo "Shaders" and
    "Textured quad" scenes in a browser (a new wgpu may change the WebGPU
    API it wraps). The Animation crate, when it exists, the same.
  - glam, the other crate chosen by name, has its own notes
    (../glam/UPGRADE-README.txt); it is resolved in the same crate set.

WHAT TO CHECK
-------------
  - The release notes of every version between the old and the new one, for
    API changes that the Graphics and Animation crates use, and for features
    that were renamed, added or removed. Re-read the feature list in the new
    wgpu's Cargo.toml and keep the crate set's comment true: a new feature
    that a WebGPU-only browser build can use is a question for the owner (a
    feature that brings new crates is a new crate set).
  - Any crate new to the lock: read the licence files inside it, not only
    its declared licence, before it is vendored. The gate judges declared
    licences and bundled files; a licence that is not approved goes to the
    repository owner.
  - Files in the wgpu project's crates that state other terms or none (the
    upstream repository has such files in its examples and tests; check that
    the published crates still do not carry them).
  - The declared minimum Rust version of the new wgpu against the Rust
    toolchain the platform's host tools use.
  - The size of the vendor folder and of the package's crate archive.
  - Run the gate and regenerate the notices (`LibraryChecker notices
    --write`), and read the difference.

THINGS TO WATCH
---------------
  - Some vendored crates carry upstream CI files (.github/workflows), editor
    settings or shell scripts of their own. They are part of the crate as
    published and stay; they are never run.
  - The `profiling` crate as published carries no licence file; its declared
    licence and its README say MIT OR Apache-2.0.
  - naga is vendored but not compiled into a browser build with the features
    the crate set names; switching on spirv, glsl or naga-ir in an
    application changes that (and spirv and glsl need crates that are not
    vendored).
  - OCCLUSION QUERIES in the browser: wgpu 30.0.2's WebGPU backend
    (src/backend/webgpu.rs, begin_render_pass) passes a render pass's
    timestamp writes to the browser but never its occlusion query set, so a
    pass that begins an occlusion query is rejected and its frame lost. The
    graphics crate (codebrix-webgpu-graphics) therefore refuses occlusion
    query sets. On an upgrade, look for set_occlusion_query_set in that
    function; when it is there, lift the refusal (check_query_set in the
    crate's src/queries/graphics_context_queries.rs) and prove an occlusion
    query in the GraphicsDemo "More" scene.

================================================================================
END OF UPGRADE NOTES
================================================================================
