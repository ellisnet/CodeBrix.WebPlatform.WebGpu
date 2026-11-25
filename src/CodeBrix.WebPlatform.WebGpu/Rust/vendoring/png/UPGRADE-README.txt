================================================================================
UPGRADE NOTES: png (PNG files in the WebGpu package's crate set)
================================================================================

Read the generic procedure in the MAINTAINER-README.txt at the repository root
(VENDORED SNAPSHOTS) and the wgpu notes beside this folder
(../wgpu/UPGRADE-README.txt: how the crate set's lock is resolved against the
platform's lock), first. This file holds what is particular to png.

WHERE IT COMES FROM
-------------------
The `png` crate (https://crates.io/crates/png, the image-rs project: a PNG
decoder and encoder in safe Rust) and what it brings that the platform does
not carry: `fdeflate`, `flate2`, `miniz_oxide` (two versions: png's own
requirement and flate2's), `crc32fast`, `adler2`, `simd-adler32`, and
`zlib-rs`, which the lock names as an optional dependency of flate2 but which
nothing compiles. png, fdeflate, flate2 and crc32fast are "MIT OR
Apache-2.0", miniz_oxide "MIT OR Zlib OR Apache-2.0", adler2 "0BSD OR MIT OR
Apache-2.0" (Apache-2.0 is used for all of them), simd-adler32 "MIT", zlib-rs
"Zlib". png was approved by the owner BY NAME with this tree; a newer version
is a new approval. They are in ../../vendor exactly as crates.io published
them, never patched. bitflags and cfg-if, which png and crc32fast use, are in
the set for wgpu as well.

The tree was first vendored for the Animation package's crate set; it moved
into this set unchanged (the same versions, byte for byte the same folders)
when PNG decoding moved into codebrix-webgpu, so that the Graphics and the
Animation crates both build on it.

WHAT IS PINNED
--------------
../../webgpu-crates/Cargo.toml names `png` exactly, with its default
features, which are none (`benchmarks`, `unstable` and `zlib-rs` stay off).
flate2 is built with its own default, `rust_backend` (miniz_oxide). The
first-party crate codebrix-webgpu names the same version exactly.

HOW THE CRATE USES IT
---------------------
codebrix-webgpu's src/images/png_decode.rs: decode_png decodes the bytes of one
PNG file with Transformations::EXPAND | STRIP_16 and brings every output to
straight 8-bit RGBA. Only the decoder is used; the encoder is compiled in only
for the crate's own tests (they make their PNG files with it). The Animation
crate's ImageSource::from_png is a call to decode_png.

RE-RESOLVING
------------
The set's lock holds the png tree at the versions above. Re-resolving the set
from the platform's lock ALONE (`LibraryChecker revendor`, the wgpu notes' step
1) lets cargo take the newest compatible version of every crate the platform
does not carry - the png tree included. When only one crate of the set is
meant to change, seed the scratch resolve with the set's current Cargo.lock
(it was resolved from the platform's lock, so it keeps every platform version)
instead, so that nothing else moves; then compare the new lock with the old
one: only the intended entries may differ.

THINGS TO WATCH
---------------
  - zlib-rs carries src/deflate/test-data/paper-100k.pdf, a third-party PDF
    document without terms of its own, used only by zlib-rs's tests; carried,
    never compiled (the owner's ruling for uncompiled test data covers it).
  - crc32fast and simd-adler32 carry GitHub workflow files of their own
    projects (.github/workflows); part of the crates as published, never run.
  - On an upgrade: if png's tree adds a crate or moves flate2's backend, the
    re-resolve shows it; a new crate needs the owner's approval first. Run the
    crate's host tests (src/images/tests_png.rs, through the materialize
    tests) and the GraphicsDemo sample's "images" scene (the PNG file beside
    the backend-decoded picture) after it.

================================================================================
END OF UPGRADE NOTES
================================================================================
