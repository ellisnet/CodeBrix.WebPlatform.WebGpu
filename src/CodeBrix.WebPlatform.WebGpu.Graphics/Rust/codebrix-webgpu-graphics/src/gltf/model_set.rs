//! [`GltfModelSet`] and [`GltfLoadOptions`]: the model data built from glTF documents, owned
//! (the blob reader's [`ModelDataSet`] borrows a blob; this is the same data with its own
//! storage), with the texture table that lets models share an image (C#: the reference
//! converter's `ModelDataSet` filled by `GltfModelConverter.Convert` with one
//! `ModelTextureCache`, and its `ModelConversionOptions`).

use std::collections::HashMap;
use std::rc::Rc;

use crate::context::{GraphicsContext, GraphicsException};
use crate::gltf::builder::GltfModelBuilder;
use crate::gltf::document::GltfDocument;
use crate::models::{Model, ModelAnimation, ModelData, ModelDataSet, ModelLoadOptions, ModelMaterial, ModelMesh, ModelNode, ModelSkin, ModelSkinVertex, ModelTextureData, ModelTextureKind, ModelVertex};
use crate::resources::{GpuTexture, SamplerDescription};

/// Options for building model data from a glTF document (C#: the reference converter's
/// `ModelConversionOptions`, whose defaults are the desktop library's `ModelLoadOptions`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GltfLoadOptions {
    /// True (default) to decode the material textures; false leaves every texture out, and no
    /// image is asked for (C#: `LoadTextures`).
    pub load_textures: bool,
    /// True (default) to mark every texture for a mip chain, which
    /// [`GraphicsContext::create_models`](crate::GraphicsContext::create_models) makes when it
    /// uploads the texture (C#: `Mipmaps`).
    pub mipmaps: bool,
    /// True (default) to mark colour textures (base colour, emissive) as sRGB, as glTF specifies;
    /// false marks them linear. Data textures (normal, metallic-roughness, occlusion) are always
    /// linear (C#: `ColorTexturesSrgb`).
    pub color_textures_srgb: bool,
    /// True (default) to generate tangents for primitives whose material has a normal map and
    /// whose file has no TANGENT (C#: `GenerateTangents`).
    pub generate_tangents: bool,
    /// The largest side a texture may have, in pixels; 0 (default) keeps every image at its own
    /// size. A larger image is halved (both sides) until both sides fit, before it is stored.
    /// Decoded textures sit in memory and on the GPU as uncompressed RGBA8, so this is the main
    /// lever on a model's size: 2048 x 2048 is 16 MiB, 1024 x 1024 is 4 MiB (C#:
    /// `MaxTextureSize`; the reference converter resamples with its imaging library's default
    /// filter, this loader averages the pixels each new pixel covers, so the reduced pictures
    /// are alike but not equal).
    pub max_texture_size: u32,
    /// True (default) to store ONE copy of the vertices that several primitives share: when two
    /// primitives of a model name the same accessors for every vertex attribute and morph target
    /// (the usual way to give one mesh two materials), the second uses the first's vertices
    /// instead of a copy of them (Not in C#: the reference converter copies them for every
    /// primitive; set false for exactly its layout). Primitives whose normals or tangents are
    /// generated never share, because those depend on each primitive's own triangles.
    pub share_vertex_data: bool,
}

impl Default for GltfLoadOptions {
    fn default() -> GltfLoadOptions {
        GltfLoadOptions { load_textures: true, mipmaps: true, color_textures_srgb: true, generate_tangents: true, max_texture_size: 0, share_vertex_data: true }
    }
}

/// One texture of a [`GltfModelSet`], decoded: the owned form of [`ModelTextureData`].
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct OwnedTexture {
    pub(crate) name: Option<String>,
    pub(crate) width: u32,
    pub(crate) height: u32,
    pub(crate) srgb: bool,
    pub(crate) mipmaps: bool,
    pub(crate) pixels: Vec<u8>,
    /// The address the image came from (the key it is shared under), or None for an image inside
    /// its model file, which is never shared.
    pub(crate) address: Option<String>,
    /// The format the image file had (PNG or JPEG).
    pub(crate) format: crate::gltf::textures::ImageFormat,
}

/// One model of a [`GltfModelSet`]: the owned form of [`ModelData`].
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct OwnedModel {
    pub(crate) name: String,
    pub(crate) vertices: Vec<ModelVertex>,
    pub(crate) index_bytes: Vec<u8>,
    pub(crate) index_count: u32,
    pub(crate) index_format: wgpu::IndexFormat,
    pub(crate) skin_vertices: Vec<ModelSkinVertex>,
    pub(crate) meshes: Vec<ModelMesh>,
    pub(crate) materials: Vec<ModelMaterial>,
    pub(crate) default_material: ModelMaterial,
    pub(crate) nodes: Vec<ModelNode>,
    pub(crate) root_nodes: Vec<usize>,
    pub(crate) skins: Vec<ModelSkin>,
    pub(crate) animations: Vec<ModelAnimation>,
    pub(crate) textures: Vec<usize>,
    pub(crate) samplers: Vec<SamplerDescription>,
    pub(crate) warnings: Vec<String>,
}

/// Model data built from glTF documents (Not in C# as a type; it is the reference converter's
/// `ModelDataSet` together with its `ModelTextureCache`, and the desktop library's
/// `ModelTextureCache` of GPU textures): one or more models and the textures they use, each
/// image ONCE however many models name it. Add models with [`GltfModelSet::add`], then make the
/// GPU models - either all at once with
/// [`GraphicsContext::create_models`](crate::GraphicsContext::create_models) from
/// [`GltfModelSet::data_set`] (the same call, and the same [`ModelDataSet`], as for a model-data
/// blob), or with [`GltfModelSet::create_models`], which can be called again after more models
/// were added and then SHARES the GPU textures of the earlier ones.
///
/// ```ignore
/// let document = GltfDocument::parse(&model_bytes)?;
/// let mut files = std::collections::HashMap::new();
/// for file in document.external_files(Some("models/forest"), true) {
///     files.insert(file.address.clone(), fetch(&file.address).await?); // the application fetches
/// }
/// let mut set = GltfModelSet::new();
/// set.add(&document, "tree", Some("models/forest"), &|address| files.get(address).map(Vec::as_slice), &GltfLoadOptions::default())?;
/// let models = set.create_models(&graphics, &ModelLoadOptions::default())?;
/// ```
#[derive(Clone, Default)]
pub struct GltfModelSet {
    pub(crate) models: Vec<OwnedModel>,
    pub(crate) textures: Vec<OwnedTexture>,
    /// (address, sRGB, mipmaps) -> index in `textures` (C#: `ModelTextureCache`'s dictionary).
    pub(crate) shared: HashMap<(String, bool, bool), usize>,
    /// The GPU texture of each texture [`GltfModelSet::create_models`] has uploaded (the same
    /// length as `textures` once it has run; shorter or empty before).
    gpu_textures: Vec<Option<Rc<GpuTexture>>>,
    /// How many of `models` [`GltfModelSet::create_models`] has made GPU models of.
    uploaded_models: usize,
}

impl std::fmt::Debug for GltfModelSet {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GltfModelSet").field("models", &self.models.iter().map(|m| m.name.as_str()).collect::<Vec<_>>()).field("textures", &self.textures.len()).field("uploaded_models", &self.uploaded_models).field("byte_size", &self.byte_size()).finish()
    }
}

/// What one call of [`GltfModelSet::create_models`] has to do, worked out without a device: the
/// model data to hand to `create_models`, and what to put right afterwards.
pub(crate) struct UploadPlan<'a> {
    /// The models not uploaded yet and the textures they need that are not on the GPU yet.
    pub(crate) view: ModelDataSet<'a>,
    /// For each texture of `view`, its index in the set.
    pub(crate) view_textures: Vec<usize>,
    /// The slots whose texture IS on the GPU already: (model in `view`, material or None for the
    /// default material, slot, the texture's index in the set, the slot's sampler).
    pub(crate) reused: Vec<(usize, Option<usize>, ModelTextureKind, usize, Option<usize>)>,
}

const SLOT_KINDS: [ModelTextureKind; 5] = [ModelTextureKind::BaseColor, ModelTextureKind::MetallicRoughness, ModelTextureKind::Normal, ModelTextureKind::Occlusion, ModelTextureKind::Emissive];

impl GltfModelSet {
    /// An empty set.
    pub fn new() -> GltfModelSet {
        GltfModelSet::default()
    }

    /// Builds the model data of a parsed document and adds it to the set as its last model (C#:
    /// `GltfModelConverter.Convert`, the builder half). Returns the model's index in the set.
    ///
    /// `name` is the model's name (C#: the file name without its extension). `base_address` is
    /// the model's own folder, the one [`GltfDocument::external_files`] was asked with; `files`
    /// is asked for each external buffer and image the build needs, by the
    /// [`address`](crate::GltfExternalFile::address) that list reports, and returns the file's
    /// bytes or None. THE APPLICATION FETCHES: this crate does no networking; fetch the files
    /// first (a browser cannot wait inside this call) and hand them over. A buffer that is not
    /// supplied is an error; so is an image that is not supplied while textures are loaded.
    ///
    /// An external image in one colour space is decoded ONCE per set: every model added to this
    /// set that names the same address shares the one texture, and `files` is not asked for it
    /// again. Images inside a model file (`data:` URIs, buffer views) are not shared. With no
    /// base address, external images are not shared either (C#: a null base key).
    ///
    /// Textures are PNG or JPEG (decoded by `codebrix_webgpu::decode_image`); a texture in
    /// another format - KTX2/Basis, WebP, AVIF, DDS - is left out with a warning that names the
    /// format, as is anything else the builder cannot use (points and lines, a primitive without
    /// POSITION, an animation channel it cannot read): see [`GltfModelSet::warnings`]. An error
    /// is returned when the model cannot be built at all (a buffer that is missing or too
    /// short, Draco or meshopt data); the set is then unchanged.
    pub fn add<'r>(&mut self, document: &GltfDocument, name: &str, base_address: Option<&str>, files: &dyn Fn(&str) -> Option<&'r [u8]>, options: &GltfLoadOptions) -> Result<usize, GraphicsException> {
        let textures_before = self.textures.len();
        let shared_before = self.shared.clone();
        match GltfModelBuilder::new(self, document, name, base_address, files, options).build() {
            Ok(model) => {
                self.models.push(model);
                Ok(self.models.len() - 1)
            }
            Err(error) => {
                self.textures.truncate(textures_before);
                self.shared = shared_before;
                Err(error)
            }
        }
    }

    /// The number of models.
    pub fn model_count(&self) -> usize {
        self.models.len()
    }

    /// The number of textures (each image once per colour space, however many models use it).
    pub fn texture_count(&self) -> usize {
        self.textures.len()
    }

    /// A model's name.
    pub fn model_name(&self, model: usize) -> Option<&str> {
        self.models.get(model).map(|m| m.name.as_str())
    }

    /// What was skipped or approximated while a model was built (C#: `ModelData.Warnings`); empty
    /// for an index the set does not have.
    pub fn warnings(&self, model: usize) -> &[String] {
        self.models.get(model).map_or(&[], |m| m.warnings.as_slice())
    }

    /// The bytes the set holds for the GPU: vertices, indices, skin vertices and decoded pixels
    /// (before mip chains).
    pub fn byte_size(&self) -> usize {
        let models: usize = self.models.iter().map(|m| m.vertices.len() * ModelVertex::SIZE + m.index_bytes.len() + m.skin_vertices.len() * ModelSkinVertex::SIZE).sum();
        models + self.textures.iter().map(|t| t.pixels.len()).sum::<usize>()
    }

    /// The set as the [`ModelDataSet`] that
    /// [`GraphicsContext::create_models`](crate::GraphicsContext::create_models) takes: the
    /// vertex, index, skin and pixel bytes are borrowed from this set, the rest is copied. Once
    /// [`GltfModelSet::create_models`] has run, this holds only the models added since and the
    /// textures they still need (what that call will upload next); a slot whose texture is on
    /// the GPU already is empty here.
    pub fn data_set(&self) -> ModelDataSet<'_> {
        self.upload_plan(&|texture| self.gpu_textures.get(texture).is_some_and(Option::is_some)).view
    }

    /// Makes the GPU models of the models added since the last call (all of them the first
    /// time), in the order they were added (Not in C#; the desktop library shares GPU textures
    /// between loads through its `ModelTextureCache`, which this set stands in for). A texture
    /// is uploaded ONCE per set: models added later that name the same image use the texture an
    /// earlier call made - so keep ONE set for the models of a scene that are loaded at different
    /// times, and their shared images are fetched, decoded and uploaded once.
    ///
    /// After the upload the set RELEASES its own copy of what went to the GPU (vertices, indices
    /// and pixels - most of its memory); names, warnings and the texture table stay. With
    /// [`ModelLoadOptions::load_textures`] off nothing is uploaded or released for textures.
    /// Errors are those of [`GraphicsContext::create_models`]; after an error the same models
    /// are tried again by the next call.
    pub fn create_models(&mut self, graphics: &GraphicsContext, options: &ModelLoadOptions) -> Result<Vec<Model>, GraphicsException> {
        self.gpu_textures.resize(self.textures.len(), None);
        let plan = self.upload_plan(&|texture| self.gpu_textures[texture].is_some());
        let mut models = graphics.create_models(&plan.view, options)?;
        let mut uploaded: Vec<(usize, Rc<GpuTexture>)> = Vec::new();
        if options.load_textures {
            for (model, data) in models.iter().zip(&plan.view.models) {
                for (local, view_texture) in data.textures.iter().enumerate() {
                    if let Some(texture) = model.textures().get(local) {
                        uploaded.push((plan.view_textures[*view_texture], texture.clone()));
                    }
                }
            }
            for (model, material, kind, texture, sampler) in &plan.reused {
                let Some(shared) = self.gpu_textures[*texture].clone() else {
                    continue;
                };
                models[*model].set_material_texture(*material, *kind, shared)?;
                models[*model].material_mut(*material)?.slot_mut(*kind).sampler = *sampler;
            }
        }
        drop(plan);
        for (texture, gpu) in uploaded {
            self.gpu_textures[texture] = Some(gpu);
            self.textures[texture].pixels = Vec::new();
        }
        for model in &mut self.models[self.uploaded_models..] {
            model.vertices = Vec::new();
            model.index_bytes = Vec::new();
            model.skin_vertices = Vec::new();
            model.meshes = Vec::new();
            model.animations = Vec::new();
        }
        self.uploaded_models = self.models.len();
        Ok(models)
    }

    /// The GPU texture [`GltfModelSet::create_models`] made for the image at an address (the
    /// address the build asked the file function for), in a colour space, or None.
    pub fn uploaded_texture(&self, address: &str, srgb: bool, mipmaps: bool) -> Option<&Rc<GpuTexture>> {
        let index = *self.shared.get(&(address.to_string(), srgb, mipmaps))?;
        self.gpu_textures.get(index)?.as_ref()
    }

    /// See [`UploadPlan`]. `on_gpu` says whether a texture of the set has been uploaded.
    pub(crate) fn upload_plan(&self, on_gpu: &dyn Fn(usize) -> bool) -> UploadPlan<'_> {
        let mut view_textures: Vec<usize> = Vec::new();
        let mut reused = Vec::new();
        let mut models = Vec::new();
        for (view_model, m) in self.models[self.uploaded_models.min(self.models.len())..].iter().enumerate() {
            // The model's texture list without the textures that are on the GPU already.
            let mut textures = Vec::new();
            let mut new_local: Vec<Option<usize>> = Vec::with_capacity(m.textures.len());
            for &texture in &m.textures {
                if on_gpu(texture) {
                    new_local.push(None);
                } else {
                    let view_index = view_textures.iter().position(|t| *t == texture).unwrap_or_else(|| {
                        view_textures.push(texture);
                        view_textures.len() - 1
                    });
                    new_local.push(Some(textures.len()));
                    textures.push(view_index);
                }
            }
            let mut adjust = |material: &ModelMaterial, index: Option<usize>| -> ModelMaterial {
                let mut material = material.clone();
                for kind in SLOT_KINDS {
                    let slot = material.slot_mut(kind);
                    let Some(local) = slot.texture else {
                        continue;
                    };
                    match new_local.get(local).copied().flatten() {
                        Some(kept) => slot.texture = Some(kept),
                        None => {
                            if let Some(texture) = m.textures.get(local) {
                                reused.push((view_model, index, kind, *texture, slot.sampler));
                            }
                            slot.texture = None;
                        }
                    }
                }
                material
            };
            let materials: Vec<ModelMaterial> = m.materials.iter().enumerate().map(|(index, material)| adjust(material, Some(index))).collect();
            let default_material = adjust(&m.default_material, None);
            models.push(ModelData {
                name: m.name.clone(),
                vertex_bytes: bytemuck::cast_slice(&m.vertices),
                vertex_count: m.vertices.len() as u32,
                index_bytes: &m.index_bytes,
                index_count: m.index_count,
                index_format: m.index_format,
                skin_bytes: bytemuck::cast_slice(&m.skin_vertices),
                skin_vertex_count: m.skin_vertices.len() as u32,
                meshes: m.meshes.clone(),
                materials,
                default_material,
                nodes: m.nodes.clone(),
                root_nodes: m.root_nodes.clone(),
                skins: m.skins.clone(),
                animations: m.animations.clone(),
                textures,
                samplers: m.samplers.clone(),
                warnings: m.warnings.clone(),
            });
        }
        let textures = view_textures.iter().map(|&index| &self.textures[index]).map(|t| ModelTextureData { name: t.name.clone(), width: t.width, height: t.height, srgb: t.srgb, mipmaps: t.mipmaps, pixels: &t.pixels }).collect();
        UploadPlan { view: ModelDataSet { models, textures, minor_version: crate::models::ModelDataFormat::MINOR_VERSION }, view_textures, reused }
    }
}
