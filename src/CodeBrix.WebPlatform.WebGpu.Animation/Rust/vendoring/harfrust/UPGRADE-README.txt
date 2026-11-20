================================================================================
UPGRADE NOTES: harfrust (text in the Animation package's crate set)
================================================================================

Read the generic procedure, UPGRADING A VENDORED LIBRARY in the
MAINTAINER-README.txt at the repository root, and the notes for the rest of
the Animation crate set (../kurbo-peniko/UPGRADE-README.txt), first: the set
is ONE set with one Cargo.lock, derived and re-derived exactly as that file
says. This file holds what is particular to harfrust.

WHERE IT COMES FROM
-------------------
The `harfrust` crate (https://crates.io/crates/harfrust: text shaping, a Rust
port of HarfBuzz, and glyph outlines through its own font reader) and what it
brings that the platform and the WebGpu package do not carry: `read-fonts` and
`font-types` of the Fontations project. harfrust is "MIT"; read-fonts and
font-types are "MIT OR Apache-2.0" (Apache-2.0 is used). harfrust was approved
by the owner BY NAME AT 0.15.0 - not a later version, because 0.15.0 shares
read-fonts 0.46 with the crates the later SVG-text and Lottie steps bring
(skrifa 0.49, vello 0.12). A newer harfrust is a new approval. They are in
../../vendor exactly as crates.io published them, never patched.

WHAT IS PINNED
--------------
../../animation-crates/Cargo.toml names `harfrust = "=0.15.0"` with its
default features (`std`): NOT `icu` (harfrust carries its own Unicode tables,
and its ICU versions would clash with the platform's), NOT `libm` (it would
bring core_maths and spin), NOT `tracing`. read-fonts and font-types come with
harfrust's own settings; read-fonts' `agl` feature stays OFF. The first-party
crate names the same version exactly.

THINGS TO WATCH
---------------
  - THE UNICODE DATA. harfrust's tables (src/unicode/ucd_table.rs,
    emoji_table.rs, src/ot/shaper/*_table.rs, arabic_pua.rs) are generated
    from Unicode Character Database files, and their headers quote the data
    files' own copyright lines and point to unicode.org's terms. The notices
    header (../../notices/notices-header-WebGpuAnimationCrates.txt, part D)
    carries the Unicode License V3 text for them, and the gate demands it
    (AnimationNoticesCheck). On an upgrade, read the new tables' headers
    again (the Unicode version and the copyright line) and keep part D true.
  - THE ADOBE GLYPH LIST. read-fonts carries data/glyphlist.txt and
    data/zapfdingbats.txt ("Copyright 2002-2019 Adobe", BSD-3-Clause) and a
    table generated from them, compiled only with read-fonts' `agl` feature.
    Keep that feature OFF; if it is ever turned on, the Adobe notice must be
    added to the notices header.
  - HARFBUZZ. harfrust's README calls it a port of HarfBuzz, "licensed under
    the Old MIT"; HarfBuzz's own COPYING is not in the crate. The notices
    header lists it as a credit.
  - FONTS ARE NEVER IN A CRATE. The crate's own tests read fonts from the
    test project (tests/CodeBrix.WebPlatform.WebGpu.Animation.Tests/Fonts,
    OFL fonts with their OFL.txt); nothing font-shaped may go into
    codebrix-webgpu-animation or the vendor folder. harfrust's own tests need
    fonts that are not in the published crate; they are never run from the
    vendored copy.
  - THE .wasm COST. Text adds about 800 KB raw (about 300 KB with gzip) to an
    application's WebAssembly file: the shaping tables for every script come
    with harfrust, and no feature removes them. Measure again on an upgrade
    (the GraphicsDemo "animation" application's .wasm).
  - THE API USED. codebrix-webgpu-animation uses harfrust's ShaperFont,
    Buffer, ShapePlan, Feature, harfrust::shape and its re-exported font
    model (harfrust::font: charmap, metrics, names, glyph metrics,
    outline_glyphs with DrawOptions and a PathSink). After an upgrade, run
    the crate's unit tests (Animation.Tests runs them with the test fonts):
    they hold shaping results for fixed strings (kerning, a ligature, byte
    offsets, the missing-character modes) computed with those fonts.
