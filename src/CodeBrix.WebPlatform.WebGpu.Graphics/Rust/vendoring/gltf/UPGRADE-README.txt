================================================================================
UPGRADE NOTES: gltf (glTF documents; the Graphics package's crate set)
================================================================================

Read the generic procedure in the MAINTAINER-README.txt at the repository root
(VENDORED SNAPSHOTS), the wgpu notes
(src/CodeBrix.WebPlatform.WebGpu/Rust/vendoring/wgpu/UPGRADE-README.txt) and
the Animation package's notes
(src/CodeBrix.WebPlatform.WebGpu.Animation/Rust/vendoring/kurbo-peniko/
UPGRADE-README.txt: the other crate set that holds only what its package
adds), first. This file holds what is particular to the Graphics package's
crate set.

WHERE THEY COME FROM
--------------------
The `gltf` crate (https://crates.io/crates/gltf, the gltf-rs project,
https://github.com/gltf-rs/gltf: the glTF 2.0 JSON and the binary .glb
container read into typed structures, validated, with readers for accessors
and animation samplers) and its two companions from the same project at the
same version, `gltf-json` (the JSON structures and their validation) and
`gltf-derive` (a procedural macro that writes the validation code; it runs at
build time and none of its code is in an application). All three are "MIT OR
Apache-2.0" (Apache-2.0 is used), each with LICENSE-APACHE and LICENSE-MIT
(copyright "The gltf Library Developers"). gltf-derive's LICENSE-MIT also says
that it "was adapted from `validator_derive`
(https://github.com/Keats/validator)" and carries that project's MIT notice
(Copyright (c) 2016 Vincent Prouillet); the notices header credits it.

They bring two further crates:
  - `lazy_static` (https://crates.io/crates/lazy_static, "MIT OR Apache-2.0",
    Apache-2.0 is used; LICENSE-APACHE and LICENSE-MIT): used by gltf.
  - `inflections` (https://crates.io/crates/inflections, MIT, "Copyright (c)
    2016 Caleb Meredith", one LICENSE file; no dependencies): used by
    gltf-derive at build time.

gltf was approved by the owner BY NAME at 1.4.1, with those four; a newer
version is a new approval. Only stable releases are taken. lazy_static and
inflections are the versions cargo resolves for gltf's requirements. They are
NOT patched: each crate is in ../../vendor exactly as crates.io published it
(`cargo vendor --versioned-dirs` form, one name-version folder per crate,
each with its .cargo-checksum.json).

Every other crate gltf needs is one the PLATFORM carries, at the platform's
version: byteorder, serde (with serde_core and serde_derive), serde_json (with
itoa, memchr and zmij), proc-macro2, quote, syn and unicode-ident. gltf-json
asks for serde_json's `raw_value` feature; a feature needs no approval of its
own and no further crate (the code is in the serde_json the platform carries),
and cargo unifies it with whatever the application and the platform ask for.

WHAT IS PINNED
--------------
../../graphics-crates/Cargo.toml (the crate set) names `gltf` exactly, with
DEFAULT FEATURES OFF and these features on: utils, names, extensions,
KHR_materials_emissive_strength, KHR_texture_transform, KHR_materials_ior,
KHR_materials_pbrSpecularGlossiness, KHR_materials_specular,
KHR_materials_transmission, KHR_materials_unlit, allow_empty_texture (the
manifest's comment says what each is for). Its Cargo.lock fixes every other
crate. The first-party crate ../../codebrix-webgpu-graphics names the same
version with THE SAME FEATURES: keep the two lines the same.

NEVER TURN ON `import` (gltf's default), `image`, `base64`, `urlencoding` or
`guess_mime_type`: `import` reads the file system and decodes pictures through
the `image` crate and its tree, none of which is approved or vendored. The
Graphics crate is handed every byte by the application and decodes PNG and
JPEG through codebrix-webgpu. Never turn on `extras` either without reading
the loader first: it changes the type of every `extras` field.

HOW THE VENDORED SET IS DERIVED
-------------------------------
As the Animation package's: ../../vendor holds ONLY THE CRATES THE PACKAGE
ADDS - the crates of the crate set's Cargo.lock that neither the platform's
crates nor the WebGpu package's vendor folder carries; every other crate of
the lock must be carried by one of them at the same version with the same
checksum. The gate derives the set that way (`LibraryChecker`, groups
`vendor` and `platform`). The Graphics and the Animation packages never
depend on each other, so this set is resolved against the platform's and the
WebGpu set's locks ONLY; it must nevertheless add no crate NAME the Animation
package also adds (an application may register both bundles).

THE LOCK IS RESOLVED AGAINST THE PLATFORM'S AND THE WebGpu SET'S LOCKS
----------------------------------------------------------------------
 1. In a scratch folder outside the repository, put a copy of
    ../../graphics-crates/Cargo.toml and its src/lib.rs, and, as that
    folder's Cargo.lock, the platform's Cargo.lock (of the platform version
    the packages' exact CodeBrix.WebPlatform.Core pin names; in the
    CodeBrix.WebPlatform repository it is
    src/CodeBrix.WebPlatform.FrontendBuilder/Rust/platform-crates/Cargo.lock
    at that version's tag) with the [[package]] entries of the WebGpu crate
    set's Cargo.lock (src/CodeBrix.WebPlatform.WebGpu/Rust/webgpu-crates/
    Cargo.lock) that it does not already hold appended (one entry per name
    and version). Do not copy a .cargo folder: this step needs crates.io.
 2. Let cargo resolve there (`cargo metadata --format-version 1`). Cargo
    keeps every locked version that still satisfies the requirements, adds
    only what is missing, and drops what the crate set does not use. For a
    change of ONE crate, seed with the crate set's CURRENT Cargo.lock entries
    as well, so that nothing else moves.
 3. `cargo vendor --locked --versioned-dirs <empty folder>` there. Copy the
    Cargo.lock to ../../graphics-crates/Cargo.lock, and into ../../vendor
    ONLY the folders of the crates that neither the platform's crates nor
    the WebGpu package's vendor folder holds (remove the old ones first). The
    others must be byte-identical to the copies the platform or the WebGpu
    package carries (compare their .cargo-checksum.json files).
 4. Check that each new crate's .cargo-checksum.json "package" value is the
    crates.io sha256 of that version, and read the licence files of every
    crate that is new to the set.
 5. Run the gate and regenerate the notices (`LibraryChecker notices
    --write`), read the difference, and run the Graphics tests (their real
    Materialize resolves and checks the crate offline and runs its unit
    tests, the glTF loader's among them).

If a requirement of a new gltf cannot be met by the version the platform
carries (serde, serde_json, syn ...), do not vendor: the upgrade waits for a
platform release that satisfies it, or the owner decides otherwise.

HOW THE CRATE USES IT
---------------------
codebrix-webgpu-graphics' src/gltf/ is the loader; the gltf crate is its
PARSING layer and nothing more:
  - the .glb container is split by the loader's own code (the crate's
    Glb::from_slice subtracts the header size from the file's stated length
    without a check, which overflows on a damaged header);
  - the JSON is read into gltf's typed structures (gltf::json::Root);
  - gltf's validation is run through its public Validate trait and its
    findings are FILTERED: the loader accepts what the reference converter
    accepts (a required extension the loader handles itself, an attribute
    name the crate does not know, a POSITION accessor without min/max) and
    turns everything else into its own error message; then the loader runs
    its own checks of everything the crate does not check and would panic on
    (byte ranges of buffer views and accessors, sparse ranges, animation
    channel targets);
  - typed elements are read with gltf's accessor iterator
    (gltf::accessor::Iter, feature `utils`: byte strides, interleaved views,
    sparse accessors), always with the element type the accessor itself
    declares; the conversion to floats (normalised integers) is the loader's
    own, written to give the same numbers as the reference converter.
The crate's mesh and animation `Reader` types are NOT used: they stop with
`unreachable!` or a debug assertion on an accessor whose type does not suit
its use, and the loader must refuse a malformed file with a message, never
stop the page.

THINGS TO WATCH (re-check on an upgrade)
----------------------------------------
  - The list of extensions gltf-json accepts in "extensionsRequired"
    (ENABLED_EXTENSIONS in gltf-json's src/extensions/mod.rs) and the
    loader's own list (src/gltf/): the loader decides, not the crate.
  - Every place the loader relies on validation to make an index safe:
    gltf-json's Validate derive checks every Index<T> it holds, EXCEPT an
    animation channel's target node and path (Channel has no derived
    validation) - the loader checks those itself.
  - gltf-json's own validation is not panic-free: its primitive hook looks up
    the POSITION accessor (root.accessors[index]) before that index has been
    validated. The loader checks every attribute's accessor index before it
    runs the validation (src/gltf/document.rs, check_validation); the test
    a_damaged_document_is_never_a_panic finds this again if it is removed.
    The crate's Glb::from_slice, mesh Reader and Document::validate (private)
    are not called at all.
  - gltf-json reads "pbrMetallicRoughness", texture "sampler" defaults and
    the material defaults with serde defaults: a missing object and an empty
    one look the same. Where the reference converter tells them apart the
    loader says so in its code.
  - An animation channel without "node" (KHR_animation_pointer) does not
    deserialize; the loader goes through such a document as plain JSON first
    (src/gltf/document.rs, read_root).
  - Files inside the crates that are never used: gltf's examples/ and tests/,
    lazy_static's tests/ and examples/, the crates' own Cargo.lock files, and
    inflections' scripts/deploy-docs.sh (its author's documentation deploy
    script: part of the crate as published, never run - the repository runs
    no shell script, and a file of a vendored crate is never deleted).
  - gltf-derive is a procedural macro on syn 2: it compiles for the HOST at
    build time. The Graphics crate itself uses no derive of its own.
  - WHAT THE LOADER ADDS TO A .wasm, measured with gltf 1.4.1 (small wasm32
    libraries that each call one thing, opt-level "z", fat LTO, one codegen
    unit, panic = abort, stripped; no wasm-opt; the difference to a library
    that links the crates and calls nothing; bytes):
        the model-data (blob) reader alone            42 156
        decode_png alone                             113 599
        decode_jpeg alone                            153 345
        decode_image (both decoders)                 260 668
        GltfDocument::parse alone                    497 579
        parse + GltfModelSet::add (the whole loader,
        with both image decoders)                    888 779
    Of the whole loader, gzip leaves about 295 000 bytes. The largest part is
    gltf-json's typed JSON reading (the serde code of every glTF structure):
    about 500 000 bytes before the builder and the decoders. A library that
    calls only the blob reader holds no glTF code at all (checked by looking
    for the loader's messages in the file). Measure again after an upgrade,
    the same way (the figures move with the Rust toolchain too).

================================================================================
END OF UPGRADE NOTES
================================================================================
