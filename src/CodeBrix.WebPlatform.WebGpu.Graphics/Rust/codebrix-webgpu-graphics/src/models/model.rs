//! [`Model`] and [`ModelLoadOptions`]: a model on the GPU, made from model data (C#: `Model`,
//! `ModelLoadOptions` and the context's `LoadModel`).

use std::rc::Rc;
use std::sync::atomic::{AtomicU64, Ordering};

use glam::{Mat4, Vec3};

use crate::context::{GraphicsContext, GraphicsException, RgbaTextureOptions};
use crate::models::model_layout::ModelLayout;
use crate::models::{ModelAnimation, ModelData, ModelDataSet, ModelInstance, ModelMaterial, ModelMesh, ModelNode, ModelPrimitive, ModelSkin, ModelTextureSlot};
use crate::resources::{GpuBuffer, GpuSampler, GpuTexture, SamplerDescription};

/// Numbers the models, so that a renderer can keep GPU state per model.
static NEXT_MODEL_ID: AtomicU64 = AtomicU64::new(1);

/// Options for loading models (C#: `ModelLoadOptions`, the members that still apply when the
/// model data was made on the backend; glTF validation, sRGB colour textures and tangent
/// generation are choices of the .NET side that makes the data).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ModelLoadOptions {
    /// True (default) to upload the material textures (C#: `LoadTextures`). False leaves every
    /// slot without its texture, as when a texture could not be decoded.
    pub load_textures: bool,
    /// True (default) for mipmaps on every texture the data asks mipmaps for (C#: `Mipmaps`; here
    /// the mip chain is made on the CPU when the texture is created).
    pub mipmaps: bool,
    /// A label for the models' GPU objects, or None for each model's name (C#: `Label`).
    pub label: Option<String>,
}

impl Default for ModelLoadOptions {
    fn default() -> ModelLoadOptions {
        ModelLoadOptions { load_textures: true, mipmaps: true, label: None }
    }
}

/// What a [`Model`] handle shares.
pub(crate) struct ModelInner {
    pub(crate) id: u64,
    pub(crate) name: String,
    pub(crate) label: String,
    pub(crate) meshes: Vec<ModelMesh>,
    pub(crate) materials: Vec<ModelMaterial>,
    pub(crate) default_material: ModelMaterial,
    pub(crate) nodes: Vec<ModelNode>,
    pub(crate) root_nodes: Vec<usize>,
    pub(crate) skins: Vec<ModelSkin>,
    pub(crate) animations: Vec<ModelAnimation>,
    pub(crate) textures: Vec<Rc<GpuTexture>>,
    pub(crate) samplers: Vec<GpuSampler>,
    pub(crate) vertex_buffer: GpuBuffer,
    pub(crate) index_buffer: GpuBuffer,
    pub(crate) index_format: wgpu::IndexFormat,
    pub(crate) skin_buffer: Option<GpuBuffer>,
    pub(crate) vertex_count: u32,
    pub(crate) index_count: u32,
    pub(crate) warnings: Vec<String>,
    pub(crate) layout: ModelLayout,
}

/// A glTF 2.0 model on the GPU (C#: `Model`): immutable data shared by any number of
/// [`ModelInstance`]s. Every primitive's vertices live in ONE vertex buffer of
/// [`ModelVertex`](crate::ModelVertex), every index in ONE index buffer, skinning data in one skin
/// buffer. The model also holds its materials, textures and samplers, node hierarchy, skins,
/// morph targets and animation clips. Draw instances with a
/// [`ModelRenderer`](crate::ModelRenderer).
///
/// A cheap handle: clones share the model; it is released when the last clone (and the last
/// instance of it) is dropped (C#: `Dispose`). Textures shared with other models of the same
/// [`ModelDataSet`] are released with the last model that uses them.
///
/// THE DIFFERENCE FROM C#: there is no file reading in a browser. The model is made from MODEL
/// DATA - a blob the Graphics package's .NET `ModelDataWriter` wrote from the glTF file, with the
/// glTF parsed and the images decoded on the backend - through
/// [`GraphicsContext::load_model`] / [`GraphicsContext::load_models`].
#[derive(Clone)]
pub struct Model {
    pub(crate) inner: Rc<ModelInner>,
}

impl std::fmt::Debug for Model {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Model").field("name", &self.inner.name).field("vertex_count", &self.inner.vertex_count).field("index_count", &self.inner.index_count).finish()
    }
}

impl PartialEq for Model {
    fn eq(&self, other: &Model) -> bool {
        Rc::ptr_eq(&self.inner, &other.inner)
    }
}

impl Model {
    /// The model's name (C#: `Name`, the file name without its extension).
    pub fn name(&self) -> &str {
        &self.inner.name
    }

    /// The label of the model's GPU objects (C#: `Label`).
    pub fn label(&self) -> &str {
        &self.inner.label
    }

    /// The meshes, glTF order (C#: `Meshes`).
    pub fn meshes(&self) -> &[ModelMesh] {
        &self.inner.meshes
    }

    /// The materials, glTF order (C#: `Materials`).
    pub fn materials(&self) -> &[ModelMaterial] {
        &self.inner.materials
    }

    /// The glTF default material, used by primitives that name none (C#: `DefaultMaterial`).
    pub fn default_material(&self) -> &ModelMaterial {
        &self.inner.default_material
    }

    /// A primitive's material: the one it names, else the default material (C#:
    /// `ModelPrimitive.Material`).
    pub fn primitive_material(&self, primitive: &ModelPrimitive) -> &ModelMaterial {
        primitive.material.and_then(|m| self.inner.materials.get(m)).unwrap_or(&self.inner.default_material)
    }

    /// Every node, glTF order (C#: `Nodes`).
    pub fn nodes(&self) -> &[ModelNode] {
        &self.inner.nodes
    }

    /// The root nodes of the model's scene (C#: `RootNodes`).
    pub fn root_nodes(&self) -> &[usize] {
        &self.inner.root_nodes
    }

    /// The skins (C#: `Skins`).
    pub fn skins(&self) -> &[ModelSkin] {
        &self.inner.skins
    }

    /// The animation clips (C#: `Animations`).
    pub fn animations(&self) -> &[ModelAnimation] {
        &self.inner.animations
    }

    /// The distinct GPU textures the materials use (C#: `Textures`).
    pub fn textures(&self) -> &[Rc<GpuTexture>] {
        &self.inner.textures
    }

    /// A material slot's texture, or None (C#: `ModelTextureSlot.Texture`).
    pub fn slot_texture(&self, slot: &ModelTextureSlot) -> Option<&GpuTexture> {
        slot.texture.and_then(|t| self.inner.textures.get(t)).map(|t| t.as_ref())
    }

    /// A material slot's sampler, or None (C#: `ModelTextureSlot.Sampler`).
    pub fn slot_sampler(&self, slot: &ModelTextureSlot) -> Option<&GpuSampler> {
        slot.sampler.and_then(|s| self.inner.samplers.get(s))
    }

    /// The distinct samplers the materials use (C#: `Samplers`).
    pub fn samplers(&self) -> &[GpuSampler] {
        &self.inner.samplers
    }

    /// The shared vertex buffer of [`ModelVertex`](crate::ModelVertex) (C#: `VertexBuffer`).
    pub fn vertex_buffer(&self) -> &GpuBuffer {
        &self.inner.vertex_buffer
    }

    /// The shared index buffer (C#: `IndexBuffer`).
    pub fn index_buffer(&self) -> &GpuBuffer {
        &self.inner.index_buffer
    }

    /// The index format, 16 or 32 bits (C#: `IndexFormat`).
    pub fn index_format(&self) -> wgpu::IndexFormat {
        self.inner.index_format
    }

    /// The shared buffer of [`ModelSkinVertex`](crate::ModelSkinVertex), or None (C#: `SkinBuffer`).
    pub fn skin_buffer(&self) -> Option<&GpuBuffer> {
        self.inner.skin_buffer.as_ref()
    }

    /// The number of vertices in the vertex buffer (C#: `VertexCount`).
    pub fn vertex_count(&self) -> u32 {
        self.inner.vertex_count
    }

    /// The number of indices in the index buffer (C#: `IndexCount`).
    pub fn index_count(&self) -> u32 {
        self.inner.index_count
    }

    /// The number of triangles one instance draws (C#: `TriangleCount`).
    pub fn triangle_count(&self) -> u32 {
        self.inner.layout.triangle_count
    }

    /// The minimum corner of the rest-pose model-space bounds (C#: `BoundsMin`).
    pub fn bounds_min(&self) -> Vec3 {
        self.inner.layout.bounds_min
    }

    /// The maximum corner of the rest-pose bounds (C#: `BoundsMax`).
    pub fn bounds_max(&self) -> Vec3 {
        self.inner.layout.bounds_max
    }

    /// The centre of the rest-pose bounds (C#: `BoundsCenter`).
    pub fn bounds_center(&self) -> Vec3 {
        (self.inner.layout.bounds_min + self.inner.layout.bounds_max) * 0.5
    }

    /// What was skipped or approximated when the model data was made (C#: `Warnings`).
    pub fn warnings(&self) -> &[String] {
        &self.inner.warnings
    }

    /// An instance in the rest pose (C#: `CreateInstance()`).
    pub fn create_instance(&self) -> ModelInstance {
        ModelInstance::new(self.clone())
    }

    /// An instance in the rest pose with a world matrix (C#: `CreateInstance(world)`).
    pub fn create_instance_at(&self, world: Mat4) -> ModelInstance {
        let mut instance = self.create_instance();
        instance.world = world;
        instance
    }

    /// The index of the first animation with this name, or None (C#: `FindAnimation`, -1 for none).
    pub fn find_animation(&self, name: &str) -> Option<usize> {
        self.inner.animations.iter().position(|a| a.name.as_deref() == Some(name))
    }

    /// The index of the first node with this name, or None (C#: `FindNode`, -1 for none).
    pub fn find_node(&self, name: &str) -> Option<usize> {
        self.inner.nodes.iter().position(|n| n.name.as_deref() == Some(name))
    }

    pub(crate) fn id(&self) -> u64 {
        self.inner.id
    }

    pub(crate) fn layout(&self) -> &ModelLayout {
        &self.inner.layout
    }
}

impl GraphicsContext {
    /// Loads the ONE model of a model-data blob onto the GPU (C#: `LoadModel(path|stream,
    /// options)`; the blob is what the backend's `ModelDataWriter` made of the glTF file). An
    /// error when the bytes are not model data, are damaged, or hold more or fewer than one model
    /// (use [`GraphicsContext::load_models`] for a set).
    pub fn load_model(&self, bytes: &[u8], options: &ModelLoadOptions) -> Result<Model, GraphicsException> {
        let set = ModelDataSet::read(bytes)?;
        if set.models.len() != 1 {
            return Err(GraphicsException::new(format!("The model data holds {} models; load_model takes exactly one - use load_models for a set.", set.models.len())));
        }
        Ok(self.create_models(&set, options)?.remove(0))
    }

    /// Loads every model of a model-data blob onto the GPU, in the blob's order. A texture
    /// several models use is uploaded ONCE and shared (C#: loading with one `ModelTextureCache`).
    pub fn load_models(&self, bytes: &[u8], options: &ModelLoadOptions) -> Result<Vec<Model>, GraphicsException> {
        let set = ModelDataSet::read(bytes)?;
        self.create_models(&set, options)
    }

    /// Makes the GPU models of a model-data set that has already been read (C#: the loader's
    /// upload step). Buffers and textures are created at once; WebGPU's own validation errors, if
    /// any, arrive through [`GraphicsContext::take_errors`] (or wrap the call in
    /// [`GraphicsContext::validate`]).
    pub fn create_models(&self, set: &ModelDataSet<'_>, options: &ModelLoadOptions) -> Result<Vec<Model>, GraphicsException> {
        let mut textures: Vec<Option<Rc<GpuTexture>>> = vec![None; set.textures.len()];
        let mut models = Vec::with_capacity(set.models.len());
        for data in &set.models {
            if options.load_textures {
                for &texture in &data.textures {
                    if textures[texture].is_none() {
                        let source = &set.textures[texture];
                        let label = format!("{} {}", options.label.as_deref().unwrap_or(&data.name), source.name.as_deref().unwrap_or("texture"));
                        let created = self.create_texture_from_rgba(source.width, source.height, source.pixels, RgbaTextureOptions { mipmaps: source.mipmaps && options.mipmaps, srgb: source.srgb, label: Some(label) })?;
                        textures[texture] = Some(Rc::new(created));
                    }
                }
            }
            models.push(self.create_model(data, &textures, options)?);
        }
        Ok(models)
    }

    fn create_model(&self, data: &ModelData<'_>, textures: &[Option<Rc<GpuTexture>>], options: &ModelLoadOptions) -> Result<Model, GraphicsException> {
        let label = options.label.clone().unwrap_or_else(|| data.name.clone());
        let vertex_buffer = self.create_buffer_from(data.vertex_bytes, wgpu::BufferUsages::VERTEX, Some(&format!("{label} vertices")))?;
        let index_buffer = self.create_buffer_from(data.index_bytes, wgpu::BufferUsages::INDEX, Some(&format!("{label} indices")))?;
        let skin_buffer = if data.skin_vertex_count > 0 { Some(self.create_buffer_from(data.skin_bytes, wgpu::BufferUsages::STORAGE, Some(&format!("{label} skin")))?) } else { None };
        let samplers: Vec<GpuSampler> = data.samplers.iter().map(|s| self.create_sampler(&SamplerDescription { label: Some(format!("{label} sampler")), ..s.clone() })).collect();

        // The model's own texture list: the set's textures it uses. A texture that is not loaded
        // leaves its slots empty, as C# does for an image it could not decode.
        let mut model_textures = Vec::with_capacity(data.textures.len());
        let mut local_of = vec![None; data.textures.len()];
        for (local, &texture) in data.textures.iter().enumerate() {
            if let Some(created) = &textures[texture] {
                local_of[local] = Some(model_textures.len());
                model_textures.push(created.clone());
            }
        }
        let remap = |slot: &ModelTextureSlot| -> ModelTextureSlot {
            let texture = slot.texture.and_then(|t| local_of[t]);
            ModelTextureSlot { texture, sampler: if texture.is_some() { slot.sampler } else { None }, ..*slot }
        };
        let remap_material = |source: &ModelMaterial| -> ModelMaterial {
            let mut material = source.clone();
            material.base_color_texture = remap(&source.base_color_texture);
            material.metallic_roughness_texture = remap(&source.metallic_roughness_texture);
            material.normal_texture = remap(&source.normal_texture);
            material.occlusion_texture = remap(&source.occlusion_texture);
            material.emissive_texture = remap(&source.emissive_texture);
            material
        };
        let layout = ModelLayout::new(&data.nodes, &data.meshes, &data.skins);
        Ok(Model {
            inner: Rc::new(ModelInner {
                id: NEXT_MODEL_ID.fetch_add(1, Ordering::Relaxed),
                name: data.name.clone(),
                label,
                meshes: data.meshes.clone(),
                materials: data.materials.iter().map(remap_material).collect(),
                default_material: remap_material(&data.default_material),
                nodes: data.nodes.clone(),
                root_nodes: data.root_nodes.clone(),
                skins: data.skins.clone(),
                animations: data.animations.clone(),
                textures: model_textures,
                samplers,
                vertex_buffer,
                index_buffer,
                index_format: data.index_format,
                skin_buffer,
                vertex_count: data.vertex_count,
                index_count: data.index_count,
                warnings: data.warnings.clone(),
                layout,
            }),
        })
    }
}
