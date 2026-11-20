================================================================================
UPGRADE NOTES: kurbo and peniko (the Animation package's crate set)
================================================================================

Read the generic procedure, UPGRADING A VENDORED LIBRARY in the
MAINTAINER-README.txt at the repository root, and the wgpu notes
(src/CodeBrix.WebPlatform.WebGpu/Rust/vendoring/wgpu/UPGRADE-README.txt), first.
This file holds what is particular to the Animation package's crate set.

WHERE THEY COME FROM
--------------------
The `kurbo` crate (https://crates.io/crates/kurbo: curves, flattening, stroke
expansion with joins, caps and dashes) and the `peniko` crate
(https://crates.io/crates/peniko: brushes, gradients, images, blend modes,
fill rules), both from the Linebender project, both "Apache-2.0 OR MIT"
(Apache-2.0 is used). kurbo was approved by the owner by name; peniko is the
version velato brings (it is inside velato's approved tree, and the owner was
told that our renderer uses it directly). They are NOT patched: each crate is
in ../../vendor exactly as crates.io published it (`cargo vendor
--versioned-dirs` form, one name-version folder per crate, each with its
.cargo-checksum.json).

Only stable releases are used. A new version of either is a new crate set:
the owner approves it first, and the licence files of every crate new to the
set are READ before it is vendored.

WHAT IS PINNED
--------------
../../animation-crates/Cargo.toml (the crate set) names kurbo and peniko at
exact versions with their default features (std). Its Cargo.lock fixes every
other crate. The first-party crate ../../codebrix-webgpu-animation names the
same two versions exactly (the gate's first-party group checks that each is a
version the vendored crates carry).

HOW THE VENDORED SET IS DERIVED
-------------------------------
Unlike the WebGpu package, the Animation package vendors ONLY THE CRATES IT
ADDS: its bundle holds only what it adds (the owner's rule for the Graphics
and Animation packages). So ../../vendor holds the crates of the crate set's
Cargo.lock that neither the platform's crates nor the WebGpu package's
vendor folder carries; every other crate of the lock must be carried by one
of them at the same version with the same checksum. The gate derives the set
that way (`LibraryChecker`, groups `vendor` and `platform`).

Because the folder holds only those crates, cargo cannot resolve the crate
set from it alone. The gate's licence audit builds a temporary merged source
(this package's crates, the shared ones copied from the WebGpu package's
vendor folder or unpacked from the platform's archive) - which is also what
an application's vendor folder holds after Materialize.

THE LOCK IS RESOLVED AGAINST THE PLATFORM'S AND THE WebGpu SET'S LOCKS
----------------------------------------------------------------------
 1. In a scratch folder outside the repository, put a copy of
    ../../animation-crates/Cargo.toml and its src/lib.rs, and, as that
    folder's Cargo.lock, the platform's Cargo.lock (of the platform version
    the packages' exact CodeBrix.WebPlatform.Core pin names; in the
    CodeBrix.WebPlatform repository it is
    src/CodeBrix.WebPlatform.FrontendBuilder/Rust/platform-crates/Cargo.lock)
    with the [[package]] entries of the WebGpu crate set's Cargo.lock
    (src/CodeBrix.WebPlatform.WebGpu/Rust/webgpu-crates/Cargo.lock) that it
    does not already hold appended (one entry per name and version). Do not
    copy a .cargo folder: this step needs crates.io.
 2. Let cargo resolve there (`cargo metadata --format-version 1`). Cargo
    keeps every locked version that still satisfies the requirements, adds
    only what is missing, and drops what the crate set does not use.
 3. `cargo vendor --locked --versioned-dirs <empty folder>` there. Copy the
    Cargo.lock to ../../animation-crates/Cargo.lock, and into ../../vendor
    ONLY the folders of the crates that neither the platform's crates nor
    the WebGpu package's vendor folder holds (remove the old ones first). The
    others must be byte-identical to the copies the platform or the WebGpu
    package carries (compare their .cargo-checksum.json files).
 4. Check that each new crate's .cargo-checksum.json "package" value is the
    crates.io sha256 of that version, and read the licence files of every
    crate that is new to the set.
 5. Run the gate and regenerate the notices (`LibraryChecker notices
    --write`), read the difference, and run the Animation tests (their
    real Materialize resolves and checks the crate offline and runs its unit
    tests).

If a requirement of a new kurbo or peniko cannot be met by the version the
platform or the WebGpu package carries, do not vendor: the upgrade waits for
a platform release (or a WebGpu crate set) that satisfies it, or the owner
decides otherwise.

WHAT MUST MOVE TOGETHER
-----------------------
  - THE PLATFORM AND THE WebGpu CRATE SET. Every republish of the platform
    (and every change of the WebGpu crate set) means: re-resolve this crate
    set as above, re-vendor, run the gate.
  - kurbo and peniko are released by the same project; peniko re-exports
    kurbo (and the `color` crate), so take versions that agree (peniko's
    dependency decides kurbo's version).
  - The first-party crate codebrix-webgpu-animation names both at exact
    versions: move them with the crate set, change the crate's own version,
    and run its unit tests and the GraphicsDemo "animation" application in a
    browser.
  - The renderer takes code from vello (the blend and compose shader code,
    the gradient and ramp code; see the package's notices header). vello is
    NOT a dependency and nothing waits for its releases; refresh that code
    only on purpose, as MAINTAINER-README.txt (THE ANIMATION CRATE) says.

THINGS TO WATCH
---------------
  - kurbo's src/cubicbez.rs names three functions as a port of fontTools'
    cu2qu; the notices header credits it.
  - color carries a generator script of its own (make_x11_colors.py); it is
    part of the crate as published and is never run.
  - euclid is named in the lock through kurbo's weak `euclid?/std` feature
    although it is not compiled in; it is the platform's.
