================================================================================
UPGRADE NOTES: velato (Lottie documents in the Animation package's crate set)
================================================================================

Read the generic procedure, UPGRADING A VENDORED LIBRARY in the
MAINTAINER-README.txt at the repository root, and the notes for the rest of
the Animation crate set (../kurbo-peniko/UPGRADE-README.txt), first: the set
is ONE set with one Cargo.lock, derived and re-derived exactly as that file
says. This file holds what is particular to velato.

WHERE IT COMES FROM
-------------------
The `velato` crate (https://crates.io/crates/velato: Lottie animations read
into a composition and evaluated for a frame through its RenderSink trait;
the Linebender project), "Apache-2.0 OR MIT" (Apache-2.0 is used). It brings
`serde_repr` ("MIT OR Apache-2.0", a procedural macro that runs at build
time), which the platform and the WebGpu package do not carry; it also uses
kurbo and peniko (this set's), serde and serde_json (the platform's) at the
versions they carry. velato was approved by the owner BY NAME AT 0.12.0 WITH
DEFAULT FEATURES OFF; a newer version is a new approval. Both crates are in
../../vendor exactly as crates.io published them, never patched.

WHAT IS PINNED
--------------
../../animation-crates/Cargo.toml names `velato = { version = "=0.12.0",
default-features = false }`. NEVER TURN ITS DEFAULT FEATURES ON: its default
`vello` feature brings vello 0.10 and its tree (a second read-fonts, image
decoders) - none of which this repository uses. The first-party crate names
the same version, with default features off, and serde_json at the version
the platform carries (it reads the JSON itself before velato does).

HOW THE CRATE USES IT
---------------------
codebrix-webgpu-animation's src/lottie/: document_check.rs (the crate's OWN
pass over the JSON before velato reads it), drawing_lottie.rs
(velato::Composition::from_json), recording_sink.rs (velato's RenderSink onto
the crate's plain-data draw list), lottie_element.rs (the list mapped onto the
crate's renderer). velato's Renderer is used only as an evaluator; its vello
output is not compiled.

THINGS TO WATCH
---------------
  - velato 0.12.0 PANICS on ordinary Lottie content and REFUSES whole files
    for one unsupported item (the list is in the Lottie section of the
    repository's AGENT-README.txt); in WebAssembly a panic aborts the page.
    document_check.rs steps in before each case. On an upgrade, read velato's
    importer (src/import/) and schema (src/schema/) again for new todo!,
    unimplemented!, assert! and indexing, run the crate's host tests
    (src/lottie/tests_*.rs, among them the test that runs velato ALONE on the
    known panicking documents - if velato stops panicking that test says so,
    and the pass can say less), and the sample's "lottie-files" and
    "lottie-features" scenes.
  - velato stores layer blend modes and never applies them, ignores mask
    modes, mask opacity and inversion, and drops stroke dashes; the crate's
    sink applies top-level blend modes, and report() names the rest. A newer
    velato that applies any of them changes what the sink must do.
  - serde_repr carries a GitHub workflow file and a funding file of its own
    project (.github/) and compile-failure test expectations (tests/ui): part
    of the crate as published, never compiled or run.
  - The three large trial files the Lottie tests use are the GraphicsDemo
    sample's assets (samples/GraphicsDemo/backend/GraphicsDemo.Assets/Lottie),
    read through CODEBRIX_ANIMATION_TEST_LOTTIE; the crate carries only small
    hand-written test files.
