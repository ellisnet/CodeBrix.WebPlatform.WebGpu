================================================================================
UPGRADE NOTES: usvg (SVG documents in the Animation package's crate set)
================================================================================

Read the generic procedure, UPGRADING A VENDORED LIBRARY in the
MAINTAINER-README.txt at the repository root, and the notes for the rest of
the Animation crate set (../kurbo-peniko/UPGRADE-README.txt), first: the set
is ONE set with one Cargo.lock, derived and re-derived exactly as that file
says. This file holds what is particular to usvg.

WHERE IT COMES FROM
-------------------
The `usvg` crate (https://crates.io/crates/usvg: SVG documents parsed, CSS
resolved, use / defs / symbol / switch and paint servers resolved into a
tree; the resvg project of Linebender) and what it brings that the platform
and the WebGpu package do not carry: roxmltree, simplecss, svgtypes,
tiny-skia-path, siphasher, data-url, imagesize, pico-args, float-cmp,
strict-num and arrayref, and with its `text` feature fontdb, skrifa,
unicode-bidi, unicode-script, unicode-vo and xmlwriter (harfrust 0.15.0,
read-fonts and font-types are already in the set at the versions usvg asks
for). usvg is "Apache-2.0 OR MIT" (Apache-2.0 is used). usvg was approved by
the owner BY NAME AT 0.50.0 with its default features off; its `text` feature
was approved in advance for when the demo's SVGs contain text (they do). A
newer version is a new approval. They are in ../../vendor exactly as
crates.io published them, never patched.

WHAT IS PINNED
--------------
../../animation-crates/Cargo.toml names `usvg = { version = "=0.50.0",
default-features = false, features = ["text"] }` and `svgtypes = "=0.16.2"`
(the version usvg brings; the crate's animation reader uses it directly).
usvg's other features stay off: `system-fonts` and `memmap-fonts` (the crate
never looks for a system font or maps a file), `svgz` (it would bring flate2)
and the writer. The first-party crate names the same two versions exactly.

HOW THE CRATE USES IT
---------------------
codebrix-webgpu-animation's src/svg/: drawing_svg.rs reads the document with
usvg::Tree::from_str, with fonts loaded into usvg's font database from the
bytes the application supplies (every generic family set to the
application's family) and its own image href resolver (usvg's default one
reads files and is never called); the SMIL and CSS animations are read from
the XML through usvg's roxmltree re-export (usvg drops them); scene_builder.rs
turns the tree into the crate's scene (text through its flattened paths);
svg_encode.rs draws it through the crate's renderer. Nothing of usvg's
rendering side exists (that is resvg, which is not used).

THINGS TO WATCH
---------------
  - usvg CLEARS IDS inside content a <use> instances (the documented
    difference from NoSkia); the crate reports animations that target them.
    On an upgrade, check whether usvg still does so (tests_smil.rs names it).
  - usvg reads `isolation` only from a style attribute or CSS: the crate's
    isolation step before parsing (drawing_svg.rs) relies on it.
  - Without fonts usvg drops <text> silently: the crate reports it itself.
  - fontdb carries tests/fonts/Tuffy.ttf (public domain, with its
    LICENSE.txt), owner-approved, carried and never compiled; skrifa's
    hinting code follows FreeType by its own comments (its MIT-or-Apache
    declaration is relied on, and the crate does not use hinting).
  - data-url's src/mime.rs holds a table copied from hyperium/mime (MIT or
    Apache-2.0); the notices header (part G) credits it.
  - GitHub workflow files ship inside arrayref, fontdb, imagesize, pico-args,
    roxmltree, siphasher, strict-num, unicode-bidi and unicode-script: part of
    the crates as published, never run (the gate reports them).
  - usvg's API moves with each release (0.x): run the crate's host tests
    (src/svg/tests_*.rs) and the sample's svg-parts, svg-smil, svg-css and
    svg-features scenes after an upgrade.
