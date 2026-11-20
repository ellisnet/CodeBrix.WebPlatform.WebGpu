================================================================================
UPGRADE NOTES: png (PNG files in the Animation package's crate set)
================================================================================

Read the generic procedure, UPGRADING A VENDORED LIBRARY in the
MAINTAINER-README.txt at the repository root, and the notes for the rest of
the Animation crate set (../kurbo-peniko/UPGRADE-README.txt), first: the set
is ONE set with one Cargo.lock, derived and re-derived exactly as that file
says. This file holds what is particular to png.

WHERE IT COMES FROM
-------------------
The `png` crate (https://crates.io/crates/png, the image-rs project: a PNG
decoder and encoder in safe Rust) and what it brings that the platform and
the WebGpu package do not carry: `fdeflate`, `flate2`, `miniz_oxide` (two
versions: png's own requirement and flate2's), `crc32fast`, `adler2`,
`simd-adler32`, and `zlib-rs`, which the lock names as an optional
dependency of flate2 but which nothing compiles. png, fdeflate, flate2 and
crc32fast are "MIT OR Apache-2.0", miniz_oxide "MIT OR Zlib OR Apache-2.0",
adler2 "0BSD OR MIT OR Apache-2.0" (Apache-2.0 is used for all of them),
simd-adler32 "MIT", zlib-rs "Zlib". png was approved by the owner BY NAME
with this tree; a newer version is a new approval. They are in ../../vendor
exactly as crates.io published them, never patched. bitflags and cfg-if,
which png and crc32fast use, are carried by the WebGpu package.

WHAT IS PINNED
--------------
../../animation-crates/Cargo.toml names `png` exactly, with its default
features, which are none (`benchmarks`, `unstable` and `zlib-rs` stay off).
flate2 is built with its own default, `rust_backend` (miniz_oxide). The
first-party crate names the same version exactly.

HOW THE CRATE USES IT
---------------------
codebrix-webgpu-animation's src/images/image_source_png.rs:
ImageSource::from_png decodes the bytes of one PNG file with
Transformations::EXPAND | STRIP_16 and brings every output to straight 8-bit
RGBA. Only the decoder is used; the encoder is compiled in only for the
crate's own tests (they make their PNG files with it). PNG is the only file
format the crate reads.

THINGS TO WATCH
---------------
  - zlib-rs carries src/deflate/test-data/paper-100k.pdf, a third-party PDF
    document without terms of its own, used only by zlib-rs's tests; carried,
    never compiled (the owner's ruling for uncompiled test data covers it).
  - crc32fast and simd-adler32 carry GitHub workflow files of their own
    projects (.github/workflows); part of the crates as published, never run.
  - On an upgrade: if png's tree adds a crate or moves flate2's backend, the
    re-resolve shows it; a new crate needs the owner's approval first. Run the
    crate's host tests (images/tests_png.rs) and the "images" scene of the
    sample (the PNG file beside the backend-decoded picture) after it.
  - What PNG reading adds to an application's release .wasm is measured in
    MAINTAINER-README.txt (the Animation crate's .wasm cost).
