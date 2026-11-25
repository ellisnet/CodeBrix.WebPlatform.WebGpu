================================================================================
UPGRADE NOTES: zune-jpeg (JPEG files in the WebGpu package's crate set)
================================================================================

Read the generic procedure in the MAINTAINER-README.txt at the repository root
(VENDORED SNAPSHOTS) and the wgpu notes beside this folder
(../wgpu/UPGRADE-README.txt: how the crate set's lock is resolved against the
platform's lock), first. This file holds what is particular to zune-jpeg.

WHERE IT COMES FROM
-------------------
The `zune-jpeg` crate (https://crates.io/crates/zune-jpeg, the zune-image
project: a JPEG decoder in pure Rust, the one the `image` crate uses) and its
one companion, `zune-core` (the project's shared types: the byte reader, the
colour spaces, the decoder options), which zune-jpeg re-exports. Both are "MIT
OR Apache-2.0 OR Zlib" (Apache-2.0 is used), each with LICENSE-APACHE,
LICENSE-MIT and LICENSE-ZLIB (copyright "zune-image developers"); every source
file says the same in its header. zune-jpeg was approved by the owner BY NAME
(with zune-core); a newer version is a new approval. Only stable releases are
taken: the newest stable 0.5 release whose declared rust-version the Rust
toolchain the platform's host tools use meets (0.5.15 declares 1.75.0; later
0.5.16 release candidates are not taken). zune-core is the version cargo
resolves for zune-jpeg's requirement (^0.5.1). They are in ../../vendor
exactly as crates.io published them, never patched.

WHAT IS PINNED
--------------
../../webgpu-crates/Cargo.toml names `zune-jpeg` exactly, with DEFAULT
FEATURES OFF and `std` on (which turns on zune-core's `std`). Off, and not
vendored: `x86` and `neon` (native SIMD paths; nothing on WebAssembly),
`portable_simd` (needs a nightly compiler), `log` (zune-core's logging, which
would bring the `log` macros into the decoder), and zune-core's `serde`. So
the two crates bring no other crate. The first-party crate codebrix-webgpu
names the same version and features exactly, and uses zune-core through
zune-jpeg's re-export (zune_jpeg::zune_core), so it names no second crate.

HOW THE CRATE USES IT
---------------------
codebrix-webgpu's src/images/jpeg_decode.rs: decode_jpeg first walks the file's
segments to its frame header and refuses, with its own messages, lossless,
12-bit (any precision but 8), arithmetic-coded and hierarchical files; then it
runs the decoder in its STRICT mode (a damaged or cut-short file is an error,
never a picture with a grey part) with RGBA output and the size limits moved
out of the way (the function checks 8192 x 8192 itself, with its message);
CMYK and YCCK input is refused. The crate's host tests
(src/images/tests_jpeg.rs) read the small JPEG files in the crate's
fixtures/jpeg/ folder, made for them from synthetic pixels (the test file
says how).

THINGS TO WATCH
---------------
  - Credits inside zune-jpeg: src/bitstream.rs says its accelerated AC table
    decoding is "borrowed from stb_image.h written by Fabian Gissen", and
    src/worker.rs and src/mcu.rs name small parts "borrowed from" / "derived
    from" stb (stb_image, by Sean Barrett: public domain, or MIT, at the
    user's choice). Listed as a credit in the notices header.
  - The crates carry their own Cargo.lock and benchmark notes (Benches.md);
    part of the crates as published, never used.
  - The behaviour decode_jpeg relies on, to re-check on an upgrade (its tests
    do): strict mode turns "premature end of buffer" into an error;
    input_colorspace reports CMYK and YCCK (the Adobe segment's transform
    byte); the output colour space RGBA works from YCbCr, RGB and Luma; a
    12-bit or lossless file is refused before decoding (by our own walk, so a
    new version that accepts them changes nothing until the rules change).
  - A version that adds arithmetic decoding (the 0.5.16 candidates have an
    `arith` feature) changes nothing while the feature stays off.
  - What JPEG reading adds to an application's release .wasm has not been
    measured.

================================================================================
END OF UPGRADE NOTES
================================================================================
