================================================================================
UPGRADE NOTES: i_overlay (path combining in the Animation package's crate set)
================================================================================

Read the generic procedure, UPGRADING A VENDORED LIBRARY in the
MAINTAINER-README.txt at the repository root, and the notes for the rest of
the Animation crate set (../kurbo-peniko/UPGRADE-README.txt), first: the set
is ONE set with one Cargo.lock, derived and re-derived exactly as that file
says. This file holds what is particular to i_overlay.

WHERE IT COMES FROM
-------------------
The `i_overlay` crate (https://crates.io/crates/i_overlay: boolean operations
on polygons in exact integer arithmetic) and what it brings that the platform
and the WebGpu package do not carry: `i_float`, `i_shape`, `i_tree` and
`i_key_sort`, all of the iShape-Rust project. i_overlay is "MIT OR
Apache-2.0" (Apache-2.0 is used); the other four are "MIT". i_overlay was
approved by the owner BY NAME AT 9.2.0; a newer version is a new approval.
They are in ../../vendor exactly as crates.io published them, never patched.
i_float also brings `libm`, which the WebGpu package carries (same version,
same checksum - the gate checks it).

WHAT IS PINNED
--------------
../../animation-crates/Cargo.toml names `i_overlay = "=9.2.0"` with its
default features, which are none: `allow_multithreading` (it would bring
rayon), `serde` and `glam` stay off, and their crates are not vendored. The
first-party crate names the same version exactly.

HOW THE CRATE USES IT
---------------------
codebrix-webgpu-animation's src/paths/path_clipper.rs: NoSkia's flattening and
1/1024 grid, then Overlay::<i64> (the INTEGER overlay, so the grid is the input
exactly) with OverlayRule and FillRule; NoSkia's own stroker builds the pieces
of stroke outlines and offsets, which the overlay unites. Nothing else of the
crate is used (not its float API, not its own stroke or offset outlines).

THINGS TO WATCH
---------------
  - NO LICENSE FILE in i_overlay and i_key_sort as published (i_overlay's
    README states its two licenses; i_key_sort only declares "MIT" in its
    Cargo.toml). The owner ruled that a declared license is enough; the notices
    header (../../notices/notices-header-WebGpuAnimationCrates.txt, part E)
    supplies the texts. On an upgrade, look again: if the crates ship license
    files, the generated notices pick them up and part E can say less.
  - libm is COMPILED into an application that uses this crate (through
    i_float); part E reproduces the notices of its source files.
  - i_float carries a GitHub workflow file (.github/workflows/ci.yml) of its
    own project, i_overlay JSON test fixtures and SVG diagrams, i_float CSV
    benchmark results: part of the crates as published, never compiled or run.
  - The API moves quickly (majors 8 -> 9 in a few months): an upgrade may
    rename Overlay's constructors or rules; run the crate's host tests
    (paths/tests_combine.rs) and the "path-combine" scene after it.
  - Results cover the same areas as NoSkia's own engine, but contours start at
    other vertices: a draw-on or morph of a combined path differs from NoSkia.
