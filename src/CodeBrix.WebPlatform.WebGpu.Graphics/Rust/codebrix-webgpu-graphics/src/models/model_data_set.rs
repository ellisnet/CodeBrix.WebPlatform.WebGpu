//! [`ModelDataSet`], [`ModelData`] and [`ModelTextureData`]: the READER of the model-data format,
//! the binary blob the Graphics package's .NET writer (`ModelDataWriter`) makes from glTF models
//! on the backend. The blob's vertex, index, skin and pixel ranges are borrowed as they are, ready
//! for the GPU; everything else is decoded into the model types. See [`ModelDataFormat`].

use glam::{Quat, Vec3};

use crate::context::GraphicsException;
use crate::drawing3d::MaterialAlphaMode;
use crate::models::data_reader::{read_u32, Pools, Record, Section};
use crate::models::model_layout::ModelLayout;
use crate::models::{AnimationInterpolation, AnimationPath, ModelAnimation, ModelAnimationChannel, ModelMaterial, ModelMesh, ModelMorphTarget, ModelNode, ModelPrimitive, ModelSkin, ModelTextureSlot, NodeTransform};
use crate::resources::SamplerDescription;

/// The model-data format's constants (C#: the .NET writer's `ModelDataFormat`). A blob starts
/// with [`ModelDataFormat::MAGIC`] and a major and minor version; a reader refuses another major
/// version and accepts a higher minor one (it skips what it does not know).
pub struct ModelDataFormat;

impl ModelDataFormat {
    /// The four bytes a blob starts with: "CBMD".
    pub const MAGIC: [u8; 4] = *b"CBMD";
    /// The major version this reader reads.
    pub const MAJOR_VERSION: u16 = 1;
    /// The minor version this reader knows (higher minor versions are read too).
    pub const MINOR_VERSION: u16 = 0;
    /// The file extension the sample's backend uses: ".cbmd".
    pub const FILE_EXTENSION: &'static str = ".cbmd";
    /// The bytes of the header.
    pub const HEADER_SIZE: usize = 32;
    /// The bytes of one section-table entry.
    pub const SECTION_ENTRY_SIZE: usize = 24;
}

/// (tag, record size of version 1.0): every section a 1.0 blob has.
const SECTIONS: [(&str, usize); 18] = [
    ("STRG", 1),
    ("U32S", 4),
    ("F32S", 4),
    ("MODL", 120),
    ("MESH", 24),
    ("PRIM", 64),
    ("MTGT", 24),
    ("MTRL", 336),
    ("TXTR", 40),
    ("SMPL", 24),
    ("NODE", 148),
    ("SKIN", 28),
    ("ANIM", 16),
    ("CHAN", 32),
    ("VTX0", 1),
    ("IDX0", 1),
    ("SKV0", 1),
    ("PIX0", 1),
];

/// A texture of a [`ModelDataSet`]: decoded RGBA8 pixels, level 0, with the colour space the
/// texture is sampled in (C#: the texture the loader decodes; the .NET writer's `ModelTextureData`).
#[derive(Clone, Debug, PartialEq)]
pub struct ModelTextureData<'a> {
    /// A description (the image's URI, say), or None.
    pub name: Option<String>,
    /// The width in pixels.
    pub width: u32,
    /// The height in pixels.
    pub height: u32,
    /// True for an sRGB texture (base colour, emissive), false for a linear one (data maps).
    pub srgb: bool,
    /// True when the loader should generate the mip chain.
    pub mipmaps: bool,
    /// The pixels: RGBA8, rows top to bottom, `width * height * 4` bytes, borrowed from the blob.
    pub pixels: &'a [u8],
}

/// One model of a [`ModelDataSet`]: everything the C# loader produces, on the CPU, with the
/// vertex, index and skin bytes borrowed from the blob (the .NET writer's `ModelData`). Make the
/// GPU [`Model`](crate::Model) with
/// [`GraphicsContext::create_models`](crate::GraphicsContext::create_models).
#[derive(Clone, Debug, PartialEq)]
pub struct ModelData<'a> {
    /// The model's name (C#: the file name without its extension).
    pub name: String,
    /// The [`ModelVertex`](crate::ModelVertex) bytes, 64 per vertex.
    pub vertex_bytes: &'a [u8],
    /// The number of vertices.
    pub vertex_count: u32,
    /// The index bytes (u16 or u32 each, see [`ModelData::index_format`]).
    pub index_bytes: &'a [u8],
    /// The number of indices.
    pub index_count: u32,
    /// The index format: 16-bit when every primitive has at most 65 536 vertices, else 32-bit.
    pub index_format: wgpu::IndexFormat,
    /// The [`ModelSkinVertex`](crate::ModelSkinVertex) bytes, 24 per vertex, or empty.
    pub skin_bytes: &'a [u8],
    /// The number of skin vertices (0 = no skin buffer).
    pub skin_vertex_count: u32,
    /// The meshes, glTF order.
    pub meshes: Vec<ModelMesh>,
    /// The materials, glTF order.
    pub materials: Vec<ModelMaterial>,
    /// The glTF default material, used by primitives that name none.
    pub default_material: ModelMaterial,
    /// Every node, glTF order.
    pub nodes: Vec<ModelNode>,
    /// The root nodes of the model's scene.
    pub root_nodes: Vec<usize>,
    /// The skins.
    pub skins: Vec<ModelSkin>,
    /// The animation clips.
    pub animations: Vec<ModelAnimation>,
    /// The model's distinct textures, as indices into [`ModelDataSet::textures`]; a material
    /// slot's `texture` indexes this list.
    pub textures: Vec<usize>,
    /// The model's distinct samplers; a material slot's `sampler` indexes this list.
    pub samplers: Vec<SamplerDescription>,
    /// What was skipped or approximated while the model data was made.
    pub warnings: Vec<String>,
}

impl ModelData<'_> {
    /// The model's facts without a GPU (Not in C#; wish: per-model facts readable without
    /// drawing): the rest-pose model-space bounds (minimum, maximum) - what
    /// [`Model::bounds_min`](crate::Model::bounds_min) / `bounds_max` report once loaded; (0, 0, 0)
    /// twice for a model that draws nothing.
    pub fn bounds(&self) -> (Vec3, Vec3) {
        let layout = ModelLayout::new(&self.nodes, &self.meshes, &self.skins);
        (layout.bounds_min, layout.bounds_max)
    }

    /// True when at least one part the model draws has an alpha-blended material (as
    /// [`Model::has_blended_parts`](crate::Model::has_blended_parts)). Not in C#.
    pub fn has_blended_parts(&self) -> bool {
        self.parts(true)
    }

    /// True when at least one part the model draws is opaque or alpha-tested (as
    /// [`Model::has_opaque_parts`](crate::Model::has_opaque_parts)). Not in C#.
    pub fn has_opaque_parts(&self) -> bool {
        self.parts(false)
    }

    /// The name of every animation clip, in clip order; None for an unnamed clip (as
    /// [`Model::animation_names`](crate::Model::animation_names)). Not in C#.
    pub fn animation_names(&self) -> Vec<Option<&str>> {
        self.animations.iter().map(|a| a.name.as_deref()).collect()
    }

    /// The name of every node, in node order; None for an unnamed node (as
    /// [`Model::node_names`](crate::Model::node_names)). Not in C#.
    pub fn node_names(&self) -> Vec<Option<&str>> {
        self.nodes.iter().map(|n| n.name.as_deref()).collect()
    }

    /// True when a drawn part's material is (`translucent`) or is not alpha-blended.
    fn parts(&self, translucent: bool) -> bool {
        let layout = ModelLayout::new(&self.nodes, &self.meshes, &self.skins);
        layout.draw_items.iter().any(|item| {
            let primitive = &self.meshes[item.mesh].primitives[item.primitive];
            let material = primitive.material.and_then(|m| self.materials.get(m)).unwrap_or(&self.default_material);
            material.is_translucent() == translucent
        })
    }
}

/// A blob of the model-data format, read: one or more models and the textures they use, each
/// texture once however many models use it (C#: what a `ModelTextureCache` shares between loads).
/// Read it with [`ModelDataSet::read`]; it borrows the blob's bytes.
#[derive(Clone, Debug, PartialEq)]
pub struct ModelDataSet<'a> {
    /// The models, in the blob's order.
    pub models: Vec<ModelData<'a>>,
    /// The textures the models use.
    pub textures: Vec<ModelTextureData<'a>>,
    /// The blob's minor version.
    pub minor_version: u16,
}

impl<'a> ModelDataSet<'a> {
    /// Reads a blob. The bytes may come from anywhere: the generated `<name>_bytes` call of a
    /// `[WireFile]` action of the application's backend, `include_bytes!`, a Dioxus asset.
    /// Every offset, size and reference is checked; an error says what is wrong.
    pub fn read(bytes: &'a [u8]) -> Result<ModelDataSet<'a>, GraphicsException> {
        let sections = read_sections(bytes)?;
        let get = |tag: &str| sections.iter().find(|s| s.tag == tag).copied().expect("every known section was checked");
        let pools = Pools { strings: get("STRG"), u32s: get("U32S"), f32s: get("F32S") };
        let tables = Tables { models: get("MODL"), meshes: get("MESH"), primitives: get("PRIM"), targets: get("MTGT"), materials: get("MTRL"), textures: get("TXTR"), samplers: get("SMPL"), nodes: get("NODE"), skins: get("SKIN"), animations: get("ANIM"), channels: get("CHAN") };
        let (vertices, indices, skins, pixels) = (get("VTX0"), get("IDX0"), get("SKV0"), get("PIX0"));

        let mut textures = Vec::with_capacity(tables.textures.count);
        for index in 0..tables.textures.count {
            textures.push(read_texture(&tables.textures.record(index), index, &pools, &pixels)?);
        }
        let mut models = Vec::with_capacity(tables.models.count);
        for index in 0..tables.models.count {
            models.push(read_model(index, &tables, &pools, (&vertices, &indices, &skins), textures.len())?);
        }
        let minor_version = u16::from_le_bytes([bytes[6], bytes[7]]);
        Ok(ModelDataSet { models, textures, minor_version })
    }
}

/// The record tables of a blob.
struct Tables<'a> {
    models: Section<'a>,
    meshes: Section<'a>,
    primitives: Section<'a>,
    targets: Section<'a>,
    materials: Section<'a>,
    textures: Section<'a>,
    samplers: Section<'a>,
    nodes: Section<'a>,
    skins: Section<'a>,
    animations: Section<'a>,
    channels: Section<'a>,
}

fn damaged(message: String) -> GraphicsException {
    GraphicsException::new(format!("{message}; the model data is damaged or not made by ModelDataWriter - write it again with the Graphics package's ModelDataWriter."))
}

/// Checks the header and the section table and returns every known section.
fn read_sections(bytes: &[u8]) -> Result<Vec<Section<'_>>, GraphicsException> {
    if bytes.len() < ModelDataFormat::HEADER_SIZE || bytes[0..4] != ModelDataFormat::MAGIC {
        return Err(GraphicsException::new(format!("The {} bytes are not model data: model data starts with \"CBMD\". Hand over the bytes the backend's ModelDataWriter wrote (for example from the generated <name>_bytes call), not a .glb or .gltf file.", bytes.len())));
    }
    let major = u16::from_le_bytes([bytes[4], bytes[5]]);
    if major != ModelDataFormat::MAJOR_VERSION {
        return Err(GraphicsException::new(format!("The model data has format version {major}, but this crate reads version {}; use the same release of the Graphics package on the backend (the writer) and in the frontend (this reader).", ModelDataFormat::MAJOR_VERSION)));
    }
    let count = read_u32(bytes, 8) as usize;
    if read_u32(bytes, 12) != 0 {
        return Err(damaged(format!("The model data's header flags are {:#x}, but version 1 has none", read_u32(bytes, 12))));
    }
    let length = read_u32(bytes, 16) as usize;
    if length != bytes.len() {
        return Err(GraphicsException::new(format!("The model data says it is {length} bytes long, but {} bytes were handed over; the download was cut short or padded - fetch it again.", bytes.len())));
    }
    let table_end = count.checked_mul(ModelDataFormat::SECTION_ENTRY_SIZE).and_then(|n| n.checked_add(ModelDataFormat::HEADER_SIZE));
    if table_end.is_none_or(|end| end > bytes.len()) {
        return Err(damaged(format!("The model data names {count} sections, more than its {} bytes can hold", bytes.len())));
    }
    let mut found: Vec<Section<'_>> = Vec::new();
    for entry in 0..count {
        let at = ModelDataFormat::HEADER_SIZE + entry * ModelDataFormat::SECTION_ENTRY_SIZE;
        let tag_bytes = &bytes[at..at + 4];
        let record_size = read_u32(bytes, at + 4) as usize;
        let record_count = read_u32(bytes, at + 8) as usize;
        let offset = read_u32(bytes, at + 16) as usize;
        let byte_length = read_u32(bytes, at + 20) as usize;
        let Some(&(tag, known_size)) = SECTIONS.iter().find(|(tag, _)| tag.as_bytes() == tag_bytes) else {
            continue;
        };
        if found.iter().any(|s| s.tag == tag) {
            return Err(damaged(format!("The model data has section {tag} twice")));
        }
        if record_size < known_size || record_size.checked_mul(record_count) != Some(byte_length) {
            return Err(damaged(format!("Section {tag} has {record_count} records of {record_size} bytes in {byte_length} bytes, but its records are at least {known_size} bytes")));
        }
        if offset % 16 != 0 || offset.checked_add(byte_length).is_none_or(|end| end > bytes.len()) {
            return Err(damaged(format!("Section {tag} lies at bytes {offset} to {} of {}, not inside the blob at a multiple of 16", offset as u64 + byte_length as u64, bytes.len())));
        }
        found.push(Section { tag, record_size, count: record_count, bytes: &bytes[offset..offset + byte_length] });
    }
    if let Some((missing, _)) = SECTIONS.iter().find(|(tag, _)| !found.iter().any(|s| s.tag == *tag)) {
        return Err(damaged(format!("The model data has no section {missing}")));
    }
    Ok(found)
}

/// An index in `0..count`, or an error naming the record and what it names.
fn in_range(value: u32, count: usize, what: &dyn Fn() -> String, kind: &str) -> Result<usize, GraphicsException> {
    if (value as usize) < count {
        Ok(value as usize)
    } else {
        Err(damaged(format!("{} names {kind} {value}, but there are {count}", what())))
    }
}

/// An optional index (-1 = None) in `0..count`.
fn optional(value: i32, count: usize, what: &dyn Fn() -> String, kind: &str) -> Result<Option<usize>, GraphicsException> {
    if value < 0 {
        return Ok(None);
    }
    in_range(value as u32, count, what, kind).map(Some)
}

/// A (first, count) range of a table, checked against the table's length.
fn table_range(record: &Record<'_>, at: usize, table: &Section<'_>, what: &dyn Fn() -> String) -> Result<std::ops::Range<usize>, GraphicsException> {
    let (first, count) = record.list(at);
    let end = first as u64 + count as u64;
    if end > table.count as u64 {
        return Err(damaged(format!("{} names records {first} to {end} of section {}, which has {}", what(), table.tag, table.count)));
    }
    Ok(first as usize..end as usize)
}

fn read_texture<'a>(record: &Record<'_>, index: usize, pools: &Pools<'a>, pixels: &Section<'a>) -> Result<ModelTextureData<'a>, GraphicsException> {
    let what = || format!("texture {index}");
    let name = pools.string(record, 0, &what)?;
    let (width, height) = (record.u32(8), record.u32(12));
    if !(1..=8192).contains(&width) || !(1..=8192).contains(&height) {
        return Err(damaged(format!("Texture {index} is {width} x {height} pixels; a texture is 1 to 8192 pixels on each side")));
    }
    if record.u32(16) != 1 {
        return Err(damaged(format!("Texture {index} has pixel format {}, but version 1 knows only 1 (RGBA8)", record.u32(16))));
    }
    let length = record.u32(32) as usize;
    if length as u64 != width as u64 * height as u64 * 4 {
        return Err(damaged(format!("Texture {index} has {length} pixel bytes, but {width} x {height} RGBA8 pixels take {}", width as u64 * height as u64 * 4)));
    }
    let bytes = pixels.range(record.u32(28), length, &what)?;
    Ok(ModelTextureData { name, width, height, srgb: record.u32(20) == 1, mipmaps: record.bool(24), pixels: bytes })
}

fn read_model<'a>(index: usize, tables: &Tables<'a>, pools: &Pools<'a>, buffers: (&Section<'a>, &Section<'a>, &Section<'a>), texture_count: usize) -> Result<ModelData<'a>, GraphicsException> {
    let record = tables.models.record(index);
    let model_what = || format!("model {index}");
    let name = pools.string(&record, 0, &model_what)?.unwrap_or_default();
    let what = || format!("model {index} '{name}'");
    let (vertices, indices, skins) = buffers;
    let vertex_count = record.u32(12);
    let vertex_bytes = vertices.range(record.u32(8), vertex_count as usize * 64, &|| format!("the vertices of {}", what()))?;
    let index_count = record.u32(20);
    let (index_format, index_size) = match record.u32(24) {
        2 => (wgpu::IndexFormat::Uint16, 2),
        4 => (wgpu::IndexFormat::Uint32, 4),
        other => return Err(damaged(format!("{} has index format {other}; it is 2 (u16) or 4 (u32)", what()))),
    };
    let index_bytes = indices.range(record.u32(16), index_count as usize * index_size, &|| format!("the indices of {}", what()))?;
    let skin_vertex_count = record.u32(32);
    let skin_bytes = skins.range(record.u32(28), skin_vertex_count as usize * 24, &|| format!("the skin vertices of {}", what()))?;

    let node_range = table_range(&record, 56, &tables.nodes, &what)?;
    let node_count = node_range.len();
    let material_range = table_range(&record, 44, &tables.materials, &what)?;
    let material_count = material_range.len();
    let mesh_range = table_range(&record, 36, &tables.meshes, &what)?;
    let mesh_count = mesh_range.len();
    let skin_range = table_range(&record, 72, &tables.skins, &what)?;
    let skin_count = skin_range.len();
    let sampler_range = table_range(&record, 96, &tables.samplers, &what)?;
    let model_textures: Vec<usize> = pools.u32_list(&record, 88, &what)?.into_iter().map(|t| in_range(t, texture_count, &what, "texture")).collect::<Result<_, _>>()?;

    let mut samplers = Vec::with_capacity(sampler_range.len());
    for (local, absolute) in sampler_range.clone().enumerate() {
        samplers.push(read_sampler(&tables.samplers.record(absolute), &|| format!("{} sampler {local}", what()))?);
    }
    let slot_counts = (model_textures.len(), samplers.len());
    let mut materials = Vec::with_capacity(material_count);
    for (local, absolute) in material_range.clone().enumerate() {
        materials.push(read_material(&tables.materials.record(absolute), pools, slot_counts, &|| format!("{} material {local}", what()))?);
    }
    let default_material_index = in_range(record.u32(52), tables.materials.count, &what, "default material record")?;
    let default_material = read_material(&tables.materials.record(default_material_index), pools, slot_counts, &|| format!("{} default material", what()))?;

    let mut meshes = Vec::with_capacity(mesh_count);
    for (local, absolute) in mesh_range.clone().enumerate() {
        meshes.push(read_mesh(local, absolute, tables, pools, (material_count, vertex_count, index_count, skin_vertex_count), &|| format!("{} mesh {local}", what()))?);
    }
    let mut nodes = Vec::with_capacity(node_count);
    for (local, absolute) in node_range.clone().enumerate() {
        nodes.push(read_node(local, &tables.nodes.record(absolute), pools, (node_count, mesh_count, skin_count), &|| format!("{} node {local}", what()))?);
    }
    let root_nodes: Vec<usize> = pools.u32_list(&record, 64, &what)?.into_iter().map(|n| in_range(n, node_count, &what, "root node")).collect::<Result<_, _>>()?;
    let mut model_skins = Vec::with_capacity(skin_count);
    for (local, absolute) in skin_range.clone().enumerate() {
        model_skins.push(read_skin(local, &tables.skins.record(absolute), pools, node_count, &|| format!("{} skin {local}", what()))?);
    }
    let animation_range = table_range(&record, 80, &tables.animations, &what)?;
    let mut animations = Vec::with_capacity(animation_range.len());
    for (local, absolute) in animation_range.enumerate() {
        animations.push(read_animation(local, &tables.animations.record(absolute), tables, pools, node_count, &|| format!("{} animation {local}", what()))?);
    }
    let warnings = pools.string_list(&record, 104, &what)?;
    Ok(ModelData { name, vertex_bytes, vertex_count, index_bytes, index_count, index_format, skin_bytes, skin_vertex_count, meshes, materials, default_material, nodes, root_nodes, skins: model_skins, animations, textures: model_textures, samplers, warnings })
}

fn read_sampler(record: &Record<'_>, what: &dyn Fn() -> String) -> Result<SamplerDescription, GraphicsException> {
    let address = |value: u32| match value {
        0 => Ok(wgpu::AddressMode::Repeat),
        1 => Ok(wgpu::AddressMode::ClampToEdge),
        2 => Ok(wgpu::AddressMode::MirrorRepeat),
        other => Err(damaged(format!("{} has address mode {other}; it is 0 to 2", what()))),
    };
    let filter = |value: u32| match value {
        0 => Ok(wgpu::FilterMode::Nearest),
        1 => Ok(wgpu::FilterMode::Linear),
        other => Err(damaged(format!("{} has filter {other}; it is 0 or 1", what()))),
    };
    let mipmap_filter = match record.u32(16) {
        0 => wgpu::MipmapFilterMode::Nearest,
        1 => wgpu::MipmapFilterMode::Linear,
        other => return Err(damaged(format!("{} has mipmap filter {other}; it is 0 or 1", what()))),
    };
    let u = address(record.u32(0))?;
    Ok(SamplerDescription { address_mode: u, address_mode_u: Some(u), address_mode_v: Some(address(record.u32(4))?), mag_filter: filter(record.u32(8))?, min_filter: filter(record.u32(12))?, mipmap_filter, max_lod: record.f32(20), ..SamplerDescription::default() })
}

fn read_slot(record: &Record<'_>, at: usize, counts: (usize, usize), what: &dyn Fn() -> String) -> Result<ModelTextureSlot, GraphicsException> {
    let texture = optional(record.i32(at), counts.0, what, "model texture")?;
    let sampler = optional(record.i32(at + 4), counts.1, what, "model sampler")?;
    let mut transform = [0.0f32; 6];
    for (index, value) in transform.iter_mut().enumerate() {
        *value = record.f32(at + 16 + index * 4);
    }
    Ok(ModelTextureSlot { texture, sampler, tex_coord: record.u32(at + 8).min(1), transform, has_transform: record.bool(at + 12), texture_index: record.i32(at + 40) })
}

fn read_material(record: &Record<'_>, pools: &Pools<'_>, counts: (usize, usize), what: &dyn Fn() -> String) -> Result<ModelMaterial, GraphicsException> {
    let mut material = ModelMaterial::new(record.i32(8), pools.string(record, 0, what)?);
    material.base_color_factor = record.vec4(12);
    material.base_color_texture = read_slot(record, 28, counts, &|| format!("{}'s base colour texture", what()))?;
    material.metallic_factor = record.f32(72);
    material.roughness_factor = record.f32(76);
    material.metallic_roughness_texture = read_slot(record, 80, counts, &|| format!("{}'s metallic-roughness texture", what()))?;
    material.normal_texture = read_slot(record, 124, counts, &|| format!("{}'s normal texture", what()))?;
    material.normal_scale = record.f32(168);
    material.occlusion_texture = read_slot(record, 172, counts, &|| format!("{}'s occlusion texture", what()))?;
    material.occlusion_strength = record.f32(216);
    material.emissive_factor = record.vec3(220);
    material.emissive_strength = record.f32(232);
    material.emissive_texture = read_slot(record, 236, counts, &|| format!("{}'s emissive texture", what()))?;
    material.alpha_mode = match record.u32(280) {
        0 => MaterialAlphaMode::Opaque,
        1 => MaterialAlphaMode::Mask,
        2 => MaterialAlphaMode::Blend,
        other => return Err(damaged(format!("{} has alpha mode {other}; it is 0 (opaque), 1 (mask) or 2 (blend)", what()))),
    };
    material.alpha_cutoff = record.f32(284);
    material.double_sided = record.bool(288);
    material.unlit = record.bool(292);
    material.index_of_refraction = record.f32(296);
    material.clear_coat_factor = record.f32(300);
    material.clear_coat_roughness = record.f32(304);
    material.specular_factor = record.f32(308);
    material.specular_color = record.vec3(312);
    material.transmission_factor = record.f32(324);
    material.approximated_features = pools.string_list(record, 328, what)?;
    Ok(material)
}

/// `counts` = (materials, vertices, indices, skin vertices) of the model.
fn read_mesh(local: usize, absolute: usize, tables: &Tables<'_>, pools: &Pools<'_>, counts: (usize, u32, u32, u32), what: &dyn Fn() -> String) -> Result<ModelMesh, GraphicsException> {
    let record = tables.meshes.record(absolute);
    let name = pools.string(&record, 0, what)?;
    let primitive_range = table_range(&record, 8, &tables.primitives, what)?;
    let default_morph_weights = pools.f32_list(&record, 16, what)?;
    let mut primitives = Vec::with_capacity(primitive_range.len());
    for (index, primitive) in primitive_range.enumerate() {
        let p = tables.primitives.record(primitive);
        let what = || format!("{} primitive {index}", what());
        let material = optional(p.i32(0), counts.0, &what, "material")?;
        let (first_index, index_count, base_vertex, vertex_count) = (p.u32(4), p.u32(8), p.u32(12), p.u32(16));
        if index_count % 3 != 0 || first_index as u64 + index_count as u64 > counts.2 as u64 || base_vertex as u64 + vertex_count as u64 > counts.1 as u64 {
            return Err(damaged(format!("{} draws indices {first_index} + {index_count} from vertex {base_vertex} + {vertex_count}, outside the model's {} indices and {} vertices (or not whole triangles)", what(), counts.2, counts.1)));
        }
        let skin_base_vertex = match p.i32(20) {
            value if value < 0 => None,
            value if value as u64 + vertex_count as u64 <= counts.3 as u64 => Some(value as u32),
            value => return Err(damaged(format!("{} names skin vertices {value} + {vertex_count}, but the model has {}", what(), counts.3))),
        };
        let flags = p.u32(48);
        let target_range = table_range(&p, 52, &tables.targets, &what)?;
        let mut morph_targets = Vec::with_capacity(target_range.len());
        for target in target_range {
            let t = tables.targets.record(target);
            let deltas = |at: usize| -> Result<Vec<Vec3>, GraphicsException> {
                let values = pools.f32_list(&t, at, &what)?;
                if !values.is_empty() && values.len() != vertex_count as usize * 3 {
                    return Err(damaged(format!("A morph target of {} has {} delta floats, neither 0 nor 3 per vertex ({})", what(), values.len(), vertex_count as usize * 3)));
                }
                Ok(values.chunks_exact(3).map(|v| Vec3::new(v[0], v[1], v[2])).collect())
            };
            morph_targets.push(ModelMorphTarget { position_deltas: deltas(0)?, normal_deltas: deltas(8)?, tangent_deltas: deltas(16)? });
        }
        primitives.push(ModelPrimitive {
            index,
            material,
            first_index,
            index_count,
            base_vertex,
            vertex_count,
            skin_base_vertex,
            bounds_min: p.vec3(24),
            bounds_max: p.vec3(36),
            has_vertex_colors: flags & 1 != 0,
            has_tex_coord1: flags & 2 != 0,
            has_tangents: flags & 4 != 0,
            tangents_generated: flags & 8 != 0,
            normals_generated: flags & 16 != 0,
            morph_targets,
        });
    }
    Ok(ModelMesh { index: local, name, primitives, default_morph_weights })
}

/// `counts` = (nodes, meshes, skins) of the model.
fn read_node(local: usize, record: &Record<'_>, pools: &Pools<'_>, counts: (usize, usize, usize), what: &dyn Fn() -> String) -> Result<ModelNode, GraphicsException> {
    let children = pools.u32_list(record, 12, what)?.into_iter().map(|c| in_range(c, counts.0, what, "child node")).collect::<Result<Vec<_>, _>>()?;
    let rotation = record.vec4(40);
    Ok(ModelNode {
        index: local,
        name: pools.string(record, 0, what)?,
        parent: optional(record.i32(8), counts.0, what, "parent node")?,
        children,
        mesh: optional(record.i32(20), counts.1, what, "mesh")?,
        skin: optional(record.i32(24), counts.2, what, "skin")?,
        default_transform: NodeTransform::new(record.vec3(28), Quat::from_xyzw(rotation.x, rotation.y, rotation.z, rotation.w), record.vec3(56)),
        has_fixed_matrix: record.bool(68),
        local_matrix: record.mat4(72),
        default_morph_weights: pools.f32_list(record, 136, what)?,
        in_scene: record.bool(144),
    })
}

fn read_skin(local: usize, record: &Record<'_>, pools: &Pools<'_>, node_count: usize, what: &dyn Fn() -> String) -> Result<ModelSkin, GraphicsException> {
    let joints = pools.u32_list(record, 8, what)?.into_iter().map(|j| in_range(j, node_count, what, "joint node")).collect::<Result<Vec<_>, _>>()?;
    let floats = pools.f32_list(record, 16, what)?;
    if floats.len() != joints.len() * 16 {
        return Err(damaged(format!("{} has {} joints but {} inverse-bind floats (16 per joint)", what(), joints.len(), floats.len())));
    }
    let inverse_bind_matrices = floats.chunks_exact(16).map(|m| glam::Mat4::from_cols_slice(m)).collect();
    Ok(ModelSkin { index: local, name: pools.string(record, 0, what)?, joints, inverse_bind_matrices, skeleton: optional(record.i32(24), node_count, what, "skeleton node")? })
}

fn read_animation(local: usize, record: &Record<'_>, tables: &Tables<'_>, pools: &Pools<'_>, node_count: usize, what: &dyn Fn() -> String) -> Result<ModelAnimation, GraphicsException> {
    let range = table_range(record, 8, &tables.channels, what)?;
    let mut channels = Vec::with_capacity(range.len());
    for (index, channel) in range.enumerate() {
        let c = tables.channels.record(channel);
        let what = || format!("{} channel {index}", what());
        let path = match c.u32(4) {
            0 => AnimationPath::Translation,
            1 => AnimationPath::Rotation,
            2 => AnimationPath::Scale,
            3 => AnimationPath::Weights,
            other => return Err(damaged(format!("{} has path {other}; it is 0 to 3", what()))),
        };
        let interpolation = match c.u32(8) {
            0 => AnimationInterpolation::Step,
            1 => AnimationInterpolation::Linear,
            2 => AnimationInterpolation::CubicSpline,
            other => return Err(damaged(format!("{} has interpolation {other}; it is 0 to 2", what()))),
        };
        let component_count = c.u32(12) as usize;
        let times = pools.f32_list(&c, 16, &what)?;
        let values = pools.f32_list(&c, 24, &what)?;
        let per_key = component_count * if interpolation == AnimationInterpolation::CubicSpline { 3 } else { 1 };
        if values.len() != times.len() * per_key {
            return Err(damaged(format!("{} has {} keys and {} values, not {per_key} per key", what(), times.len(), values.len())));
        }
        channels.push(ModelAnimationChannel { node: in_range(c.u32(0), node_count, &what, "node")?, path, interpolation, times, values, component_count });
    }
    Ok(ModelAnimation { index: local, name: pools.string(record, 0, what)?, channels })
}
