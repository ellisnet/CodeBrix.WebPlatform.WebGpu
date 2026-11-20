================================================================================
UPGRADE NOTES: glam (vectors, matrices and quaternions for Rust)
================================================================================

Read the generic procedure, UPGRADING A VENDORED LIBRARY in the
MAINTAINER-README.txt at the repository root, and the wgpu notes beside this
folder (../wgpu/UPGRADE-README.txt: how the crate set's lock is resolved),
first. This file holds what is particular to glam.

WHERE IT COMES FROM
-------------------
The `glam` crate on crates.io (https://crates.io/crates/glam), from the
glam-rs project. Licensed MIT OR Apache-2.0, with both licence files in the
crate. It is NOT patched: its source is in ../../vendor exactly as crates.io
published it. It was approved by the repository owner by name, as the maths
of the first-party crate codebrix-webgpu.

WHAT IS PINNED
--------------
../../webgpu-crates/Cargo.toml names `glam` at an exact version with its
default features and `bytemuck` (so that its types upload to the GPU as plain
bytes; bytemuck is already in the set for wgpu). Its other optional
dependencies (approx, mint, rand, serde, rkyv, speedy, encase, zerocopy,
libm as a maths backend, arbitrary) stay off and are NOT vendored: a
consumer's offline build resolves glam without them (the materialize tests
prove it). Switching one of them on is a new crate set and needs the owner's
approval. ../../codebrix-webgpu/Cargo.toml names the same exact version and
features, and so does the Graphics crate's Cargo.toml
(src/CodeBrix.WebPlatform.WebGpu.Graphics/Rust/codebrix-webgpu-graphics).

WHAT TO CHECK WHEN UPGRADING
----------------------------
  - Stable releases only; read the changelog (CHANGELOG.md in the crate) for
    API changes that codebrix-webgpu or its users depend on, and for
    changes to the meaning of the multiplication order or the matrix layout
    (AGENT-README.txt has a porting table from System.Numerics that must
    stay true).
  - Resolve with the crate set's lock as the seed: the only change in
    Cargo.lock must be glam (and any crate the new glam needs, each a
    proposal to the owner). Vendor it, run the gate, regenerate the notices.
  - ATTRIBUTION.md. glam carries code ported from other libraries, named in
    its ATTRIBUTION.md (each with its notice). The notices generator does not
    read that file, so its notices are carried verbatim in
    ../../notices/notices-header-WebGpuCrates.txt: compare the new
    ATTRIBUTION.md with the header and bring the header up to date.
  - The declared minimum Rust version (rust-version) and edition of the new
    glam against the Rust toolchain the platform's host tools use.
  - codebrix-webgpu's own version, and codebrix-webgpu-graphics': change
    them with the glam version.

================================================================================
END OF UPGRADE NOTES
================================================================================
