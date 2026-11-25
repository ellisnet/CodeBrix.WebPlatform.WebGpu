//! glTF 2.0 loading: a `.gltf` or `.glb` file served as it is becomes the crate's model data in
//! the browser, with no conversion on the backend (Not in C# as a module: the desktop library's
//! glTF loader reads files and uploads in one call; its CPU half, as the reference converter
//! `GltfModelBuilder` holds it, is ported here step for step).
//!
//! TWO STEPS, because a browser cannot wait for a file inside a call:
//! 1. [`GltfDocument::parse`] reads the model file and checks it;
//!    [`GltfDocument::external_files`] says which other files it names (its `.bin` buffers, its
//!    images). THE APPLICATION FETCHES them - this crate does no networking.
//! 2. [`GltfModelSet::add`] builds the model data, asking a function the application supplies
//!    for each file's bytes by its address. The set is the crate's ordinary model data: hand
//!    [`GltfModelSet::data_set`] to [`GraphicsContext::create_models`](crate::GraphicsContext::create_models),
//!    or call [`GltfModelSet::create_models`], which shares GPU textures between models loaded
//!    at different times.
//!
//! These entry points are apart from `load_model` on purpose: an application that only loads
//! model-data blobs never reaches this module, and the build leaves it (and the `gltf` crate)
//! out of the `.wasm`.
//!
//! WHAT IS READ. Core glTF 2.0 in all three packagings (`.gltf` with external files, `.gltf`
//! with `data:` URIs, `.glb`): triangles, strips and fans; every accessor type, normalised
//! integers (KHR_mesh_quantization), byte strides and sparse accessors; two texture-coordinate
//! sets, vertex colours, generated normals and tangents; skins; morph targets with default
//! weights; animations with STEP, LINEAR and CUBICSPLINE; PNG and JPEG textures (decoded by
//! `codebrix_webgpu::decode_image`), samplers and KHR_texture_transform; and the material
//! extensions the C# library reads - drawn: KHR_materials_emissive_strength, KHR_materials_unlit;
//! captured and approximated when drawn, with a warning: KHR_materials_ior, _specular,
//! _clearcoat, _transmission, _sheen, _volume, _iridescence, _anisotropy, _diffuse_transmission
//! and _pbrSpecularGlossiness.
//!
//! WHAT IS REFUSED, each with a message that names it: a model that requires Draco
//! (KHR_draco_mesh_compression) or meshopt (EXT_meshopt_compression) compression - an error; a
//! texture in KTX2/Basis (KHR_texture_basisu), WebP (EXT_texture_webp), AVIF or DDS - that
//! texture is left out, with a warning (a PNG or JPEG fallback image is used when the file has
//! one); points and lines - that primitive is left out, with a warning. A damaged file is
//! refused with a message that says what is wrong: never a panic.
//!
//! Layering: the `gltf` crate parses and validates the JSON and iterates accessors; everything
//! else is this module (`builder.rs` is the port; `numerics.rs` holds the arithmetic that must
//! round as the reference converter's does). `reference_tests.rs` checks the whole against the
//! C# reference converter on real model files.

mod accessors;
mod builder;
mod document;
mod geometry;
mod glb;
mod model_set;
mod numerics;
mod textures;
mod uri;

#[cfg(test)]
mod reference_tests;
#[cfg(test)]
mod test_support;
#[cfg(test)]
mod tests_builder;
#[cfg(test)]
mod tests_document;

pub use document::{GltfDocument, GltfExternalFile, GltfFileKind};
pub use model_set::{GltfLoadOptions, GltfModelSet};
