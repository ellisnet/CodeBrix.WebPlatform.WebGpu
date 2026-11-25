//! [`GltfModelBuilder`]: the conversion of one glTF document into one model of a
//! [`GltfModelSet`] (C#: the reference converter's `GltfModelBuilder`, which is the CPU half of
//! the desktop library's glTF loader, step for step - the same steps in the same order here, so
//! that the two give the same model data). Nodes with the in-scene flags, materials with every
//! channel and extension the C# library reads (its list of approximated features and its
//! warnings), texture slots with KHR_texture_transform and the texture-coordinate override,
//! samplers shared by content, textures per (image, colour space), every primitive packed into
//! one vertex and one index array (strips and fans turned into lists, sparse accessors read,
//! default morph weights baked into positions, normals and tangents generated as the C# library
//! generates them, 16-bit indices when every primitive has at most 65 536 vertices), skins
//! (identity inverse bind matrices when the file has none) and animations decoded into packed
//! per-channel arrays.
//!
//! WHERE THIS DIFFERS FROM THE C# BUILDER, each on purpose: the document is checked before it is
//! read and a damaged accessor leaves its primitive or channel out with a warning (the C# glTF
//! library reads past the end or stops with an exception); only PNG and JPEG images are decoded;
//! only the textures a material SLOT uses are decoded (the C# builder also decodes the textures of
//! extensions it only notes); primitives may share vertices
//! ([`GltfLoadOptions::share_vertex_data`]); nothing in the file's data is "repaired" (the C#
//! glTF library, in its default mode, renormalises normals, tangents and skin weights of a file
//! that passes all of its checks).

use std::collections::HashMap;

use glam::{Mat4, Quat, Vec2, Vec3, Vec4};
use gltf::json;
use gltf::json::validation::Checked;

use crate::context::GraphicsException;
use crate::drawing3d::MaterialAlphaMode;
use crate::gltf::accessors::{read_floats, read_indices, BufferData, FloatElements};
use crate::gltf::document::{address_of, GltfDocument, AVIF, BASISU, DDS, DRACO, WEBP};
use crate::gltf::geometry::{from_fan, from_strip, generate_normals, generate_tangents};
use crate::gltf::model_set::{GltfLoadOptions, GltfModelSet, OwnedModel, OwnedTexture};
use crate::gltf::numerics::{add3, decompose, length_squared3, normalize3, normalize_or_identity, scale3, texture_transform_matrix, trs_matrix};
use crate::gltf::textures::{fitted_size, reduce, ImageFormat};
use crate::gltf::uri::{decode_data_uri, is_data_uri};
use crate::models::{AnimationInterpolation, AnimationPath, ModelAnimation, ModelAnimationChannel, ModelMaterial, ModelMesh, ModelMorphTarget, ModelNode, ModelPrimitive, ModelSkin, ModelSkinVertex, ModelTextureSlot, ModelVertex, NodeTransform};
use crate::resources::SamplerDescription;

/// The largest texture side the model data holds (C#: `ModelDataFormat.MaxTextureDimension`).
const MAX_TEXTURE_DIMENSION: u32 = 8192;

/// The bytes of one glTF buffer: from the `.glb` file, from the application's file function, or
/// decoded from a `data:` URI.
enum BufferBytes<'d, 'r> {
    Blob(&'d [u8]),
    File(&'r [u8]),
    Decoded(Vec<u8>),
}

impl BufferBytes<'_, '_> {
    fn as_slice(&self) -> &[u8] {
        match self {
            BufferBytes::Blob(bytes) => bytes,
            BufferBytes::File(bytes) => bytes,
            BufferBytes::Decoded(bytes) => bytes,
        }
    }
}

/// A material channel's reference to a texture, from whichever JSON shape the channel has.
struct SlotSource {
    texture: usize,
    tex_coord: u32,
    /// KHR_texture_transform: (offset, rotation, scale, texCoord override).
    transform: Option<([f32; 2], f32, [f32; 2], Option<u32>)>,
}

/// The vertex attributes of a primitive that decide whether two primitives can share vertices:
/// every attribute accessor, every morph-target accessor, and the default morph weights baked in.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct VertexKey {
    attributes: [Option<usize>; 8],
    targets: Vec<[Option<usize>; 3]>,
    weights: Vec<u32>,
}

/// What a set of shared vertices is: where it lies and what was found out while reading it.
#[derive(Clone, Debug)]
struct SharedVertices {
    base_vertex: u32,
    vertex_count: u32,
    skin_base_vertex: Option<u32>,
    bounds_min: Vec3,
    bounds_max: Vec3,
    has_vertex_colors: bool,
    has_tex_coord1: bool,
    has_tangents: bool,
    morph_targets: Vec<ModelMorphTarget>,
}

/// Builds one model (see the module documentation).
pub(crate) struct GltfModelBuilder<'s, 'd, 'r> {
    set: &'s mut GltfModelSet,
    document: &'d GltfDocument,
    root: &'d json::Root,
    name: &'d str,
    base_address: Option<&'d str>,
    files: &'d dyn Fn(&str) -> Option<&'r [u8]>,
    options: &'d GltfLoadOptions,
    warnings: Vec<String>,
    /// (glTF texture, sRGB) -> the texture's index in the set.
    textures: HashMap<(usize, bool), usize>,
    /// The set textures this model uses, in the order they were first wanted.
    distinct_textures: Vec<usize>,
    samplers: HashMap<[u8; 6], usize>,
    distinct_samplers: Vec<SamplerDescription>,
    vertices: Vec<ModelVertex>,
    indices: Vec<u32>,
    skin_vertices: Vec<ModelSkinVertex>,
    largest_primitive: usize,
    shared_vertices: HashMap<VertexKey, SharedVertices>,
}

impl<'s, 'd, 'r> GltfModelBuilder<'s, 'd, 'r> {
    /// Creates the builder (C#: the constructor).
    pub(crate) fn new(set: &'s mut GltfModelSet, document: &'d GltfDocument, name: &'d str, base_address: Option<&'d str>, files: &'d dyn Fn(&str) -> Option<&'r [u8]>, options: &'d GltfLoadOptions) -> GltfModelBuilder<'s, 'd, 'r> {
        GltfModelBuilder {
            set,
            document,
            root: document.document.as_json(),
            name,
            base_address,
            files,
            options,
            warnings: document.warnings.clone(),
            textures: HashMap::new(),
            distinct_textures: Vec::new(),
            samplers: HashMap::new(),
            distinct_samplers: Vec::new(),
            vertices: Vec::new(),
            indices: Vec::new(),
            skin_vertices: Vec::new(),
            largest_primitive: 0,
            shared_vertices: HashMap::new(),
        }
    }

    /// Builds the model (C#: `Build`). The textures it needs that the set does not hold yet are
    /// added to the set; the model itself is returned, not yet added.
    pub(crate) fn build(mut self) -> Result<OwnedModel, GraphicsException> {
        let buffer_bytes = self.resolve_buffers()?;
        let buffers: Vec<Option<&[u8]>> = buffer_bytes.iter().map(|b| b.as_ref().map(BufferBytes::as_slice)).collect();

        let mut nodes = self.read_nodes();
        let root_nodes = self.read_scene(&mut nodes);

        if self.options.load_textures {
            self.load_textures(&buffers)?;
        }
        let materials: Vec<ModelMaterial> = (0..self.root.materials.len()).map(|index| self.read_material(index)).collect();
        let default_material = ModelMaterial::new(-1, Some("default".to_string()));

        let meshes: Vec<ModelMesh> = (0..self.root.meshes.len()).map(|index| self.read_mesh(index, &materials, &buffers)).collect();
        let skins = self.read_skins(&buffers);
        let animations = self.read_animations(&buffers);

        for (index, node) in nodes.iter_mut().enumerate() {
            let Some(mesh) = node.mesh.and_then(|mesh| meshes.get(mesh)) else {
                continue;
            };
            let morph_count = mesh.default_morph_weights.len();
            if morph_count > 0 {
                node.default_morph_weights = self.node_morph_weights(index, morph_count);
            }
        }

        // C#: Pack. 16-bit indices when every primitive has at most 65 536 vertices; a model with
        // no vertices at all says 32-bit, as the C# builder does.
        let index_format = if !self.vertices.is_empty() && self.largest_primitive <= 65536 { wgpu::IndexFormat::Uint16 } else { wgpu::IndexFormat::Uint32 };
        let index_bytes: Vec<u8> = match index_format {
            wgpu::IndexFormat::Uint16 => self.indices.iter().flat_map(|index| (*index as u16).to_le_bytes()).collect(),
            wgpu::IndexFormat::Uint32 => self.indices.iter().flat_map(|index| index.to_le_bytes()).collect(),
        };
        Ok(OwnedModel {
            name: self.name.to_string(),
            vertices: self.vertices,
            index_bytes,
            index_count: self.indices.len() as u32,
            index_format,
            skin_vertices: self.skin_vertices,
            meshes,
            materials,
            default_material,
            nodes,
            root_nodes,
            skins,
            animations,
            textures: self.distinct_textures,
            samplers: self.distinct_samplers,
            warnings: self.warnings,
        })
    }

    /// The bytes of every buffer: the `.glb` chunk for the first buffer without a URI, a decoded
    /// `data:` URI, or the application's file. A buffer that cannot be had is an error - the
    /// model's geometry is in it.
    fn resolve_buffers(&self) -> Result<Vec<Option<BufferBytes<'d, 'r>>>, GraphicsException> {
        let mut resolved = Vec::with_capacity(self.root.buffers.len());
        for (index, buffer) in self.root.buffers.iter().enumerate() {
            let bytes = match buffer.uri.as_deref() {
                None => match (&self.document.blob, index) {
                    (Some(blob), 0) => Some(BufferBytes::Blob(blob.as_slice())),
                    // Another buffer without a URI has no bytes (a compression extension's
                    // fallback buffer, say): an accessor that reads it is reported then.
                    _ => None,
                },
                Some(uri) if is_data_uri(uri) => match decode_data_uri(uri) {
                    Ok(bytes) => Some(BufferBytes::Decoded(bytes)),
                    Err(error) => return Err(GraphicsException::new(format!("Buffer {index} of the glTF model '{}' is a data: URI that could not be decoded: {error}. Export the model again.", self.name))),
                },
                Some(uri) => {
                    let address = address_of(self.base_address, uri);
                    match (self.files)(&address) {
                        Some(bytes) => Some(BufferBytes::File(bytes)),
                        None => return Err(GraphicsException::new(format!("The glTF model '{}' refers to the external file '{uri}' (asked for as '{address}'), which was not supplied. Fetch every file GltfDocument::external_files lists and hand them to the file function.", self.name))),
                    }
                }
            };
            if let Some(bytes) = &bytes {
                let (have, want) = (bytes.as_slice().len() as u64, buffer.byte_length.0);
                if have < want {
                    return Err(GraphicsException::new(format!("Buffer {index} of the glTF model '{}' is {have} bytes long, but the document says {want}; the file was cut short or is not the model's buffer - fetch it again.", self.name)));
                }
            }
            resolved.push(bytes);
        }
        Ok(resolved)
    }

    // ---------------------------------------------------------------------------------- nodes

    /// C#: `ReadNodes`, the first two loops.
    fn read_nodes(&mut self) -> Vec<ModelNode> {
        let mut nodes: Vec<ModelNode> = Vec::with_capacity(self.root.nodes.len());
        for (index, node) in self.root.nodes.iter().enumerate() {
            let mut data = ModelNode {
                index,
                name: node.name.clone(),
                parent: None,
                children: Vec::new(),
                mesh: node.mesh.map(|mesh| mesh.value()),
                skin: node.skin.map(|skin| skin.value()),
                default_transform: NodeTransform::IDENTITY,
                has_fixed_matrix: false,
                local_matrix: Mat4::IDENTITY,
                default_morph_weights: Vec::new(),
                in_scene: false,
            };
            let decomposed = match &node.matrix {
                // A node given as a matrix: its fourth column is taken as 0, 0, 0, 1 (C#: the
                // glTF library's AffineTransform keeps only the upper 4 x 3 part).
                Some(matrix) => {
                    let mut m = *matrix;
                    (m[3], m[7], m[11], m[15]) = (0.0, 0.0, 0.0, 1.0);
                    let off_diagonal_all_set = [m[1], m[2], m[4], m[6], m[8], m[9]].iter().all(|v| *v != 0.0);
                    if m[0] == 0.0 && m[5] == 0.0 && m[10] == 0.0 && off_diagonal_all_set {
                        // C#: the glTF library's `IsLosslessDecomposable` answers true for exactly
                        // this shape (a zero diagonal) and hands out a zero scale; kept, so that
                        // the two builders agree on such a file.
                        Some(([m[12], m[13], m[14]], [0.0, 0.0, 0.0, 1.0], [0.0, 0.0, 0.0]))
                    } else {
                        match decompose(&m) {
                            Some(parts) => Some(parts),
                            None => {
                                data.has_fixed_matrix = true;
                                data.local_matrix = Mat4::from_cols_array(&m);
                                None
                            }
                        }
                    }
                }
                None => {
                    let mut rotation = node.rotation.map_or([0.0, 0.0, 0.0, 1.0], |r| r.0);
                    // C#: the glTF library hands out a rotation that is further than 0.00769 from
                    // unit length already normalised; the builder then normalises what it gets.
                    let length = crate::gltf::numerics::length4(rotation);
                    if (length - 1.0).abs() > 0.00769 {
                        rotation = rotation.map(|component| component / length);
                    }
                    Some((node.translation.unwrap_or([0.0; 3]), rotation, node.scale.unwrap_or([1.0; 3])))
                }
            };
            if let Some((translation, rotation, scale)) = decomposed {
                let rotation = normalize_or_identity(rotation);
                data.default_transform = NodeTransform::new(Vec3::from(translation), Quat::from_xyzw(rotation[0], rotation[1], rotation[2], rotation[3]), Vec3::from(scale));
                data.local_matrix = Mat4::from_cols_array(&trs_matrix(translation, rotation, scale));
            }
            nodes.push(data);
        }
        for (index, node) in self.root.nodes.iter().enumerate() {
            for child in node.children.iter().flatten() {
                let child = child.value();
                if child < nodes.len() {
                    nodes[index].children.push(child);
                    nodes[child].parent = Some(index);
                }
            }
        }
        nodes
    }

    /// C#: the rest of `ReadNodes`: the root nodes of the default scene (the first scene when
    /// the file names none; every parentless node when it has no scene), and the in-scene flags.
    fn read_scene(&mut self, nodes: &mut [ModelNode]) -> Vec<usize> {
        let scene = match self.root.scene {
            Some(index) => self.root.scenes.get(index.value()),
            None => self.root.scenes.first(),
        };
        let root_nodes: Vec<usize> = match scene {
            Some(scene) => scene.nodes.iter().map(|node| node.value()).filter(|node| *node < nodes.len()).collect(),
            None => nodes.iter().filter(|node| node.parent.is_none()).map(|node| node.index).collect(),
        };
        let mut pending = root_nodes.clone();
        while let Some(index) = pending.pop() {
            if nodes[index].in_scene {
                continue;
            }
            nodes[index].in_scene = true;
            pending.extend(nodes[index].children.iter().copied());
        }
        root_nodes
    }

    /// C#: `NodeMorphWeights` over the glTF library's `Node.MorphWeights` - the node's own
    /// weights, else its mesh's - cut or padded with zeros to the mesh's morph target count.
    fn node_morph_weights(&self, node: usize, count: usize) -> Vec<f32> {
        let node = &self.root.nodes[node];
        let own = node.weights.as_deref().filter(|weights| !weights.is_empty());
        let source = own.or_else(|| node.mesh.and_then(|mesh| self.root.meshes.get(mesh.value())).and_then(|mesh| mesh.weights.as_deref())).unwrap_or(&[]);
        let mut weights = vec![0.0; count];
        for (target, value) in weights.iter_mut().zip(source) {
            *target = *value;
        }
        weights
    }

    // ------------------------------------------------------------------------------- textures

    /// The texture reference of each of a material's five slots, in the order the C# builder
    /// reads them: base colour (or the specular-glossiness diffuse texture), metallic-roughness,
    /// normal, occlusion, emissive; with whether the slot is a colour texture.
    fn slot_sources(&self, material: usize) -> [(Option<SlotSource>, bool); 5] {
        let json = &self.root.materials[material];
        let info = |info: &Option<json::texture::Info>| -> Option<SlotSource> {
            info.as_ref().map(|info| SlotSource {
                texture: info.index.value(),
                tex_coord: info.tex_coord,
                transform: info.extensions.as_ref().and_then(|e| e.texture_transform.as_ref()).map(|t| (t.offset.0, t.rotation.0, t.scale.0, t.tex_coord)),
            })
        };
        let base = if self.has_base_color_channel(material) {
            info(&json.pbr_metallic_roughness.base_color_texture)
        } else {
            json.extensions.as_ref().and_then(|e| e.pbr_specular_glossiness.as_ref()).and_then(|sg| info(&sg.diffuse_texture))
        };
        let metallic_roughness = if self.document.has_pbr(material) { info(&json.pbr_metallic_roughness.metallic_roughness_texture) } else { None };
        let normal = json.normal_texture.as_ref().map(|t| SlotSource { texture: t.index.value(), tex_coord: t.tex_coord, transform: t.extensions.as_ref().and_then(|e| transform_of(e.others.get("KHR_texture_transform"))) });
        let occlusion = json.occlusion_texture.as_ref().map(|t| SlotSource { texture: t.index.value(), tex_coord: t.tex_coord, transform: t.extensions.as_ref().and_then(|e| transform_of(e.others.get("KHR_texture_transform"))) });
        [(base, true), (metallic_roughness, false), (normal, false), (occlusion, false), (info(&json.emissive_texture), true)]
    }

    /// True when the material's base colour comes from `pbrMetallicRoughness` (C#:
    /// `material.FindChannel("BaseColor").HasValue`): always, except for a material that has a
    /// KHR_materials_pbrSpecularGlossiness extension and no `pbrMetallicRoughness` object.
    fn has_base_color_channel(&self, material: usize) -> bool {
        self.document.has_pbr(material)
    }

    /// C#: `LoadTextures`. Decodes the image of every texture a material slot uses, once per
    /// colour space, and enters it in the set - or finds it there, when another model of the set
    /// already named the same external image.
    fn load_textures(&mut self, buffers: &BufferData<'_>) -> Result<(), GraphicsException> {
        let color_srgb = self.options.color_textures_srgb;
        let mut wanted: Vec<(usize, bool)> = Vec::new();
        for material in 0..self.root.materials.len() {
            for (source, color) in self.slot_sources(material) {
                let Some(source) = source else {
                    continue;
                };
                let key = (source.texture, color && color_srgb);
                if !wanted.contains(&key) {
                    wanted.push(key);
                }
            }
        }
        // (image, sRGB) -> the texture's index in the set, for the images of this model.
        let mut image_textures: HashMap<(usize, bool), usize> = HashMap::new();
        let mut texture_images: Vec<((usize, bool), (usize, bool))> = Vec::new();
        for (texture, srgb) in wanted {
            let Some(image) = self.choose_image(texture, srgb, buffers, &mut image_textures)? else {
                continue;
            };
            texture_images.push(((texture, srgb), (image, srgb)));
        }
        for (key, image_key) in texture_images {
            let Some(texture) = image_textures.get(&image_key).copied() else {
                continue;
            };
            self.textures.insert(key, texture);
            if !self.distinct_textures.contains(&texture) {
                self.distinct_textures.push(texture);
            }
        }
        Ok(())
    }

    /// The images a texture names, best first (C#: `Texture.PrimaryImage` then `FallbackImage`):
    /// the image of the first texture extension it carries, then its core `source`.
    fn image_candidates(&self, texture: usize) -> Vec<usize> {
        let Some(json) = self.root.textures.get(texture) else {
            return Vec::new();
        };
        let mut candidates = Vec::new();
        if let Some(extensions) = &json.extensions {
            for name in [DDS, "MSFT_texture_xnb", WEBP, BASISU, "EXT_texture_astc", AVIF] {
                let source = extensions.others.get(name).and_then(|e| e.get("source")).and_then(json::Value::as_u64);
                if let Some(image) = source.and_then(|s| usize::try_from(s).ok()).filter(|s| *s < self.root.images.len()) {
                    candidates.push(image);
                    break;
                }
            }
        }
        let core = json.source.value();
        if core < self.root.images.len() && !candidates.contains(&core) {
            candidates.push(core);
        }
        candidates
    }

    /// How a message names an image (C#: `Describe`): its URI in quotes, or "image N 'name'".
    fn describe_image(&self, image: usize) -> String {
        let json = &self.root.images[image];
        match json.uri.as_deref().filter(|uri| !is_data_uri(uri) && json.buffer_view.is_none()) {
            Some(uri) => format!("'{uri}'"),
            None => format!("image {image}{}", json.name.as_deref().filter(|n| !n.is_empty()).map(|n| format!(" '{n}'")).unwrap_or_default()),
        }
    }

    /// The address an external image is shared under (C#: `CacheKey`), or None for an image
    /// inside the model file, and for every image when the build has no base address.
    fn image_address(&self, image: usize) -> Option<String> {
        let json = &self.root.images[image];
        let uri = json.uri.as_deref().filter(|uri| !uri.is_empty() && !is_data_uri(uri) && json.buffer_view.is_none())?;
        self.base_address.map(|base| crate::gltf::uri::combine_address(base, uri))
    }

    /// Picks the image of a texture (C#: `ChooseImage`), decodes it unless the set already has
    /// it, and enters it in `image_textures`. Ok(None): the texture is left out, with a warning.
    fn choose_image(&mut self, texture: usize, srgb: bool, buffers: &BufferData<'_>, image_textures: &mut HashMap<(usize, bool), usize>) -> Result<Option<usize>, GraphicsException> {
        let candidates = self.image_candidates(texture);
        let mut refused: Option<(usize, ImageFormat)> = None;
        for image in candidates {
            if image_textures.contains_key(&(image, srgb)) {
                return Ok(Some(image));
            }
            // An external image another model of the set already brought: no bytes are asked for.
            let address = self.image_address(image);
            if let Some(shared) = address.as_ref().and_then(|a| self.set.shared.get(&(a.clone(), srgb, self.options.mipmaps))) {
                image_textures.insert((image, srgb), *shared);
                return Ok(Some(image));
            }
            let bytes = self.image_bytes(image, buffers)?;
            let format = ImageFormat::of(&bytes);
            if !format.is_decodable() {
                refused.get_or_insert((image, format));
                continue;
            }
            let what = self.describe_image(image);
            match codebrix_webgpu::decode_image(&bytes) {
                Ok(decoded) => {
                    let (mut width, mut height, mut pixels) = (decoded.width, decoded.height, decoded.rgba);
                    let fitted = fitted_size(width, height, self.options.max_texture_size);
                    if fitted != (width, height) {
                        pixels = reduce(&pixels, width, height, fitted);
                        (width, height) = fitted;
                    }
                    if width > MAX_TEXTURE_DIMENSION || height > MAX_TEXTURE_DIMENSION {
                        self.warnings.push(format!("The image {what} is {width} x {height} pixels, larger than the model data format's {MAX_TEXTURE_DIMENSION} x {MAX_TEXTURE_DIMENSION}; the texture is left out. Scale the image down."));
                        return Ok(None);
                    }
                    let index = self.set.textures.len();
                    self.set.textures.push(OwnedTexture { name: Some(address.clone().unwrap_or_else(|| format!("{} {what}", self.name))), width, height, srgb, mipmaps: self.options.mipmaps, pixels, address: address.clone(), format });
                    if let Some(address) = address {
                        self.set.shared.insert((address, srgb, self.options.mipmaps), index);
                    }
                    image_textures.insert((image, srgb), index);
                    return Ok(Some(image));
                }
                Err(error) => {
                    self.warnings.push(format!("The image {what} could not be decoded: {error} The texture is left out."));
                    return Ok(None);
                }
            }
        }
        self.warnings.push(match refused {
            None => format!("Texture {texture} has no image."),
            Some((image, format)) => format!("Texture {texture} uses {}, {}; this loader reads PNG and JPEG images only, so the texture is left out. Convert the image to PNG or JPEG.", self.describe_image(image), format.phrase()),
        });
        Ok(None)
    }

    /// The bytes of an image: its buffer view, its decoded `data:` URI, or the application's file.
    fn image_bytes(&self, image: usize, buffers: &BufferData<'_>) -> Result<std::borrow::Cow<'r, [u8]>, GraphicsException> {
        use std::borrow::Cow;
        let json = &self.root.images[image];
        if let Some(view) = json.buffer_view {
            let Some(view_json) = self.root.buffer_views.get(view.value()) else {
                return Ok(Cow::Owned(Vec::new()));
            };
            let data = buffers.get(view_json.buffer.value()).copied().flatten().unwrap_or(&[]);
            let start = view_json.byte_offset.map_or(0, |o| o.0);
            let range = usize::try_from(start).ok().zip(usize::try_from(start.saturating_add(view_json.byte_length.0)).ok());
            return Ok(Cow::Owned(range.and_then(|(start, end)| data.get(start..end)).unwrap_or(&[]).to_vec()));
        }
        match json.uri.as_deref() {
            None => Ok(Cow::Owned(Vec::new())),
            Some(uri) if is_data_uri(uri) => Ok(Cow::Owned(decode_data_uri(uri).unwrap_or_default())),
            Some(uri) => {
                let address = address_of(self.base_address, uri);
                match (self.files)(&address) {
                    Some(bytes) => Ok(Cow::Borrowed(bytes)),
                    None => Err(GraphicsException::new(format!("The glTF model '{}' refers to the external file '{uri}' (asked for as '{address}'), which was not supplied. Fetch every file GltfDocument::external_files lists and hand them to the file function, or build with load_textures off.", self.name))),
                }
            }
        }
    }

    // ------------------------------------------------------------------------------ materials

    /// C#: `ReadMaterial`.
    fn read_material(&mut self, index: usize) -> ModelMaterial {
        let json = &self.root.materials[index];
        let mut result = ModelMaterial::new(index as i32, json.name.clone());
        let mut approximated: Vec<&str> = Vec::new();
        result.alpha_mode = match json.alpha_mode {
            Checked::Valid(json::material::AlphaMode::Mask) => MaterialAlphaMode::Mask,
            Checked::Valid(json::material::AlphaMode::Blend) => MaterialAlphaMode::Blend,
            _ => MaterialAlphaMode::Opaque,
        };
        result.alpha_cutoff = json.alpha_cutoff.map_or(0.5, |cutoff| cutoff.0);
        result.double_sided = json.double_sided;
        let extensions = json.extensions.as_ref();
        result.unlit = extensions.is_some_and(|e| e.unlit.is_some());
        let [base, metallic_roughness, normal, occlusion, emissive] = self.slot_sources(index);

        let pbr = &json.pbr_metallic_roughness;
        if self.has_base_color_channel(index) {
            result.base_color_factor = Vec4::from(pbr.base_color_factor.0);
            result.base_color_texture = self.slot(base.0, true);
        } else if let Some(specular_glossiness) = extensions.and_then(|e| e.pbr_specular_glossiness.as_ref()) {
            result.base_color_factor = Vec4::from(specular_glossiness.diffuse_factor.0);
            result.base_color_texture = self.slot(base.0, true);
            result.metallic_factor = 0.0;
            approximated.push("KHR_materials_pbrSpecularGlossiness");
        }
        if self.document.has_pbr(index) {
            result.metallic_factor = pbr.metallic_factor.0;
            result.roughness_factor = pbr.roughness_factor.0;
            result.metallic_roughness_texture = self.slot(metallic_roughness.0, false);
        }
        result.normal_texture = self.slot(normal.0, false);
        result.normal_scale = json.normal_texture.as_ref().map_or(1.0, |t| t.scale);
        result.occlusion_texture = self.slot(occlusion.0, false);
        result.occlusion_strength = json.occlusion_texture.as_ref().map_or(1.0, |t| t.strength.0);
        result.emissive_factor = Vec3::from(json.emissive_factor.0);
        result.emissive_strength = extensions.and_then(|e| e.emissive_strength.as_ref()).map_or(1.0, |e| e.emissive_strength.0);
        result.emissive_texture = self.slot(emissive.0, true);

        result.index_of_refraction = extensions.and_then(|e| e.ior.as_ref()).map_or(1.5, |ior| ior.ior.0);
        if (result.index_of_refraction - 1.5).abs() > 1e-6 {
            approximated.push("KHR_materials_ior");
        }
        let other = |name: &str| extensions.and_then(|e| e.others.get(name)).filter(|value| value.is_object());
        let number = |value: &json::Value, member: &str, fallback: f32| value.get(member).and_then(json::Value::as_f64).map_or(fallback, |v| v as f32);
        if let Some(clear_coat) = other("KHR_materials_clearcoat") {
            result.clear_coat_factor = number(clear_coat, "clearcoatFactor", 0.0);
            result.clear_coat_roughness = number(clear_coat, "clearcoatRoughnessFactor", 0.0);
            approximated.push("KHR_materials_clearcoat");
        }
        if let Some(specular) = extensions.and_then(|e| e.specular.as_ref()) {
            result.specular_factor = specular.specular_factor.0;
            result.specular_color = Vec3::from(specular.specular_color_factor.0);
            approximated.push("KHR_materials_specular");
        }
        if let Some(transmission) = extensions.and_then(|e| e.transmission.as_ref()) {
            result.transmission_factor = transmission.transmission_factor.0;
            approximated.push("KHR_materials_transmission");
        }
        for name in ["KHR_materials_sheen", "KHR_materials_volume", "KHR_materials_iridescence", "KHR_materials_anisotropy", "KHR_materials_diffuse_transmission"] {
            if other(name).is_some() {
                approximated.push(name);
            }
        }
        if !approximated.is_empty() {
            result.approximated_features = approximated.iter().map(|name| name.to_string()).collect();
            self.warnings.push(format!("Material {index}{} uses {} (captured, approximated when drawn).", json.name.as_ref().map(|name| format!(" '{name}'")).unwrap_or_default(), approximated.join(", ")));
        }
        result
    }

    /// C#: `Slot`.
    fn slot(&mut self, source: Option<SlotSource>, color: bool) -> ModelTextureSlot {
        let Some(source) = source else {
            return ModelTextureSlot::EMPTY;
        };
        let srgb = color && self.options.color_textures_srgb;
        let local = self.textures.get(&(source.texture, srgb)).and_then(|absolute| self.distinct_textures.iter().position(|t| t == absolute));
        let mut tex_coord = source.tex_coord;
        let mut transform = ModelTextureSlot::IDENTITY_TRANSFORM;
        if let Some((offset, rotation, scale, tex_coord_override)) = source.transform {
            transform = texture_transform_matrix(offset, rotation, scale);
            if let Some(overridden) = tex_coord_override {
                tex_coord = overridden;
            }
        }
        if tex_coord > 1 {
            self.warnings.push(format!("A texture uses TEXCOORD_{tex_coord}; only TEXCOORD_0 and TEXCOORD_1 are loaded, so it samples TEXCOORD_0."));
        }
        let sampler = local.map(|_| self.sampler_for(source.texture));
        ModelTextureSlot { texture: local, sampler, tex_coord: if tex_coord > 1 { 0 } else { tex_coord }, transform, has_transform: source.transform.is_some(), texture_index: source.texture as i32 }
    }

    /// C#: `SamplerFor`: the model's sampler for a texture's glTF sampler (repeat and linear
    /// filtering with mipmaps when the texture has none), one per distinct content.
    fn sampler_for(&mut self, texture: usize) -> usize {
        use json::texture::{MagFilter, MinFilter, WrappingMode};
        let sampler = self.root.textures.get(texture).and_then(|t| t.sampler).and_then(|s| self.root.samplers.get(s.value()));
        let wrap = |mode: Checked<WrappingMode>| match mode {
            Checked::Valid(WrappingMode::ClampToEdge) => wgpu::AddressMode::ClampToEdge,
            Checked::Valid(WrappingMode::MirroredRepeat) => wgpu::AddressMode::MirrorRepeat,
            _ => wgpu::AddressMode::Repeat,
        };
        let (u, v) = sampler.map_or((wgpu::AddressMode::Repeat, wgpu::AddressMode::Repeat), |s| (wrap(s.wrap_s), wrap(s.wrap_t)));
        let mag = match sampler.and_then(|s| s.mag_filter) {
            Some(Checked::Valid(MagFilter::Nearest)) => wgpu::FilterMode::Nearest,
            _ => wgpu::FilterMode::Linear,
        };
        let (min, mip, no_mips) = match sampler.and_then(|s| s.min_filter) {
            Some(Checked::Valid(MinFilter::Nearest)) => (wgpu::FilterMode::Nearest, wgpu::MipmapFilterMode::Nearest, true),
            Some(Checked::Valid(MinFilter::Linear)) => (wgpu::FilterMode::Linear, wgpu::MipmapFilterMode::Nearest, true),
            Some(Checked::Valid(MinFilter::NearestMipmapNearest)) => (wgpu::FilterMode::Nearest, wgpu::MipmapFilterMode::Nearest, false),
            Some(Checked::Valid(MinFilter::LinearMipmapNearest)) => (wgpu::FilterMode::Linear, wgpu::MipmapFilterMode::Nearest, false),
            Some(Checked::Valid(MinFilter::NearestMipmapLinear)) => (wgpu::FilterMode::Nearest, wgpu::MipmapFilterMode::Linear, false),
            _ => (wgpu::FilterMode::Linear, wgpu::MipmapFilterMode::Linear, false),
        };
        let address = |mode: wgpu::AddressMode| match mode {
            wgpu::AddressMode::ClampToEdge => 1u8,
            wgpu::AddressMode::MirrorRepeat => 2,
            _ => 0,
        };
        let key = [address(u), address(v), (mag == wgpu::FilterMode::Linear) as u8, (min == wgpu::FilterMode::Linear) as u8, (mip == wgpu::MipmapFilterMode::Linear) as u8, no_mips as u8];
        if let Some(existing) = self.samplers.get(&key) {
            return *existing;
        }
        let created = self.distinct_samplers.len();
        self.distinct_samplers.push(SamplerDescription { address_mode: u, address_mode_u: Some(u), address_mode_v: Some(v), mag_filter: mag, min_filter: min, mipmap_filter: mip, max_lod: if no_mips { 0.0 } else { 32.0 }, ..SamplerDescription::default() });
        self.samplers.insert(key, created);
        created
    }

    // --------------------------------------------------------------------------------- meshes

    /// C#: `ReadMesh`.
    fn read_mesh(&mut self, index: usize, materials: &[ModelMaterial], buffers: &BufferData<'_>) -> ModelMesh {
        let mesh = &self.root.meshes[index];
        let morph_count = mesh.primitives.iter().map(|p| p.targets.as_ref().map_or(0, Vec::len)).max().unwrap_or(0);
        let mut default_weights = vec![0.0f32; morph_count];
        for (target, value) in default_weights.iter_mut().zip(mesh.weights.iter().flatten()) {
            *target = *value;
        }
        let mut primitives = Vec::new();
        for primitive in 0..mesh.primitives.len() {
            if let Some(mut loaded) = self.read_primitive(index, primitive, materials, &default_weights, buffers) {
                loaded.index = primitives.len();
                primitives.push(loaded);
            }
        }
        ModelMesh { index, name: mesh.name.clone(), primitives, default_morph_weights: default_weights }
    }

    /// Reads an attribute as floats; a problem leaves the primitive out with a warning.
    fn attribute(&mut self, accessor: Option<usize>, components: &[usize], name: &str, place: &str, buffers: &BufferData<'_>) -> Result<Option<FloatElements>, ()> {
        let Some(accessor) = accessor else {
            return Ok(None);
        };
        match read_floats(&self.document.document, buffers, accessor, &format!("{place} {name}")) {
            Ok(elements) if components.contains(&elements.components) => Ok(Some(elements)),
            Ok(elements) => {
                self.warnings.push(format!("{place} has a {name} accessor with {} components per element, which {name} cannot have; it is left out.", elements.components));
                Err(())
            }
            Err(problem) => {
                self.warnings.push(format!("{problem}; it is left out."));
                Err(())
            }
        }
    }

    /// C#: `ReadPrimitive`. None when the primitive is left out (a warning says why).
    fn read_primitive(&mut self, mesh_index: usize, primitive_index: usize, materials: &[ModelMaterial], default_weights: &[f32], buffers: &BufferData<'_>) -> Option<ModelPrimitive> {
        use json::mesh::{Mode, Semantic};
        let mesh = &self.root.meshes[mesh_index];
        let primitive = &mesh.primitives[primitive_index];
        let place = format!("Mesh {mesh_index}{} primitive {primitive_index}", mesh.name.as_ref().map(|name| format!(" '{name}'")).unwrap_or_default());
        let mode = match primitive.mode {
            Checked::Valid(mode) => mode,
            Checked::Invalid => Mode::Triangles,
        };
        if !matches!(mode, Mode::Triangles | Mode::TriangleStrip | Mode::TriangleFan) {
            let drawn = match mode {
                Mode::Points => "POINTS",
                Mode::Lines => "LINES",
                Mode::LineLoop => "LINE_LOOP",
                _ => "LINE_STRIP",
            };
            self.warnings.push(format!("{place} draws {drawn}; only triangles are supported, so it is left out."));
            return None;
        }
        let attribute = |semantic: Semantic| primitive.attributes.get(&Checked::Valid(semantic)).map(|accessor| accessor.value());
        let accessors = [attribute(Semantic::Positions), attribute(Semantic::Normals), attribute(Semantic::Tangents), attribute(Semantic::TexCoords(0)), attribute(Semantic::TexCoords(1)), attribute(Semantic::Colors(0)), attribute(Semantic::Joints(0)), attribute(Semantic::Weights(0))];
        let compressed = primitive.extensions.as_ref().is_some_and(|e| e.others.contains_key(DRACO));
        let has_view = |accessor: Option<usize>| accessor.and_then(|a| self.root.accessors.get(a)).is_some_and(|a| a.buffer_view.is_some() || a.sparse.is_some());
        if compressed && !has_view(accessors[0]) {
            self.warnings.push(format!("{place} is compressed with Draco ({DRACO}), which this loader does not decode; it is left out. Export the model without Draco compression."));
            return None;
        }
        let position_count = accessors[0].and_then(|a| self.root.accessors.get(a)).map_or(0, |a| a.count.0);
        if position_count == 0 {
            self.warnings.push(format!("{place} has no POSITION; it is left out."));
            return None;
        }
        let positions = self.attribute(accessors[0], &[3], "POSITION", &place, buffers).ok()??;
        let count = positions.count();
        let normals = self.attribute(accessors[1], &[3], "NORMAL", &place, buffers).ok()?;
        let tangents = self.attribute(accessors[2], &[3, 4], "TANGENT", &place, buffers).ok()?;
        let uv0 = self.attribute(accessors[3], &[2], "TEXCOORD_0", &place, buffers).ok()?;
        let uv1 = self.attribute(accessors[4], &[2], "TEXCOORD_1", &place, buffers).ok()?;
        let colors = self.attribute(accessors[5], &[3, 4], "COLOR_0", &place, buffers).ok()?;
        let joints = self.attribute(accessors[6], &[3, 4], "JOINTS_0", &place, buffers).ok()?;
        let weights = self.attribute(accessors[7], &[3, 4], "WEIGHTS_0", &place, buffers).ok()?;

        let local = self.read_primitive_indices(primitive, mode, count, &place, buffers)?;

        let material = primitive.material.map(|m| m.value()).filter(|m| *m < materials.len());
        let normal_texture = material.map(|m| materials[m].normal_texture);
        let has_normal_map = normal_texture.is_some_and(|slot| slot.texture_index >= 0);
        let normals_generated = normals.is_none();
        let mut tangents_generated = false;
        let mut tangent_set = 0;
        if tangents.is_none() && self.options.generate_tangents && has_normal_map {
            tangent_set = normal_texture.map_or(0, |slot| slot.tex_coord);
            if (tangent_set == 0 && uv0.is_some()) || (tangent_set == 1 && uv1.is_some()) {
                tangents_generated = true;
            } else {
                self.warnings.push(format!("{place} has a normal map but no TEXCOORD_{tangent_set}; no tangents were generated."));
            }
        }

        // Not in C#: vertices another primitive already stored are used again instead of copied.
        let target_accessors: Vec<[Option<usize>; 3]> = primitive.targets.iter().flatten().map(|t| [t.positions.map(|a| a.value()), t.normals.map(|a| a.value()), t.tangents.map(|a| a.value())]).collect();
        let shareable = self.options.share_vertex_data && !normals_generated && !tangents_generated;
        let key = VertexKey { attributes: accessors, targets: target_accessors.clone(), weights: default_weights.iter().map(|w| w.to_bits()).collect() };
        let shared = if shareable { self.shared_vertices.get(&key).cloned() } else { None };
        let stored = match shared {
            Some(stored) => stored,
            None => {
                let base_vertex = self.vertices.len();
                let mut vertices: Vec<ModelVertex> = Vec::with_capacity(count);
                for index in 0..count {
                    let mut vertex = ModelVertex { position: Vec3::from(positions.get::<3>(index, [0.0; 3]).unwrap_or([0.0; 3])), ..ModelVertex::default() };
                    vertex.normal = Vec3::from(normals.as_ref().and_then(|n| n.get::<3>(index, [0.0; 3])).unwrap_or([0.0; 3]));
                    vertex.tangent = tangents.as_ref().and_then(|t| t.get::<4>(index, [0.0, 0.0, 0.0, 1.0])).unwrap_or([0.0; 4]);
                    vertex.tex_coord0 = Vec2::from(uv0.as_ref().and_then(|t| t.get::<2>(index, [0.0; 2])).unwrap_or([0.0; 2]));
                    vertex.tex_coord1 = Vec2::from(uv1.as_ref().and_then(|t| t.get::<2>(index, [0.0; 2])).unwrap_or([0.0; 2]));
                    vertex.set_color_vector(Vec4::from(colors.as_ref().and_then(|c| c.get::<4>(index, [0.0, 0.0, 0.0, 1.0])).unwrap_or([1.0; 4])));
                    vertices.push(vertex);
                }
                let morph_targets = self.read_morph_targets(&target_accessors, count, default_weights, &mut vertices, &place, buffers)?;
                let (mut min, mut max) = ([f32::MAX; 3], [f32::MIN; 3]);
                for vertex in &vertices {
                    let position = vertex.position.to_array();
                    for axis in 0..3 {
                        // C#: Vector3.Min / Vector3.Max, component by component.
                        min[axis] = if min[axis] < position[axis] { min[axis] } else { position[axis] };
                        max[axis] = if max[axis] > position[axis] { max[axis] } else { position[axis] };
                    }
                }
                if normals_generated {
                    generate_normals(&mut vertices, &local);
                }
                if tangents_generated {
                    generate_tangents(&mut vertices, &local, tangent_set);
                }
                let mut skin_base_vertex = None;
                if let (Some(joints), Some(weights)) = (&joints, &weights) {
                    skin_base_vertex = Some(self.skin_vertices.len() as u32);
                    for index in 0..count {
                        let joint = joints.get::<4>(index, [0.0, 0.0, 0.0, 1.0]).unwrap_or([0.0; 4]);
                        let mut weight = weights.get::<4>(index, [0.0, 0.0, 0.0, 1.0]).unwrap_or([1.0, 0.0, 0.0, 0.0]);
                        let sum = weight[0] + weight[1] + weight[2] + weight[3];
                        weight = if sum > 1e-8 { weight.map(|w| w / sum) } else { [1.0, 0.0, 0.0, 0.0] };
                        self.skin_vertices.push(ModelSkinVertex { joints: joint.map(|j| j.clamp(0.0, 65535.0) as u16), weights: weight });
                    }
                }
                self.vertices.extend_from_slice(&vertices);
                let stored = SharedVertices {
                    base_vertex: base_vertex as u32,
                    vertex_count: count as u32,
                    skin_base_vertex,
                    bounds_min: Vec3::from(min),
                    bounds_max: Vec3::from(max),
                    has_vertex_colors: colors.is_some(),
                    has_tex_coord1: uv1.is_some(),
                    has_tangents: tangents.is_some() || tangents_generated,
                    morph_targets,
                };
                if shareable {
                    self.shared_vertices.insert(key, stored.clone());
                }
                stored
            }
        };

        let first_index = self.indices.len() as u32;
        self.indices.extend_from_slice(&local);
        self.largest_primitive = self.largest_primitive.max(count);
        Some(ModelPrimitive {
            index: primitive_index,
            material,
            first_index,
            index_count: local.len() as u32,
            base_vertex: stored.base_vertex,
            vertex_count: stored.vertex_count,
            skin_base_vertex: stored.skin_base_vertex,
            bounds_min: stored.bounds_min,
            bounds_max: stored.bounds_max,
            has_vertex_colors: stored.has_vertex_colors,
            has_tex_coord1: stored.has_tex_coord1,
            has_tangents: stored.has_tangents,
            tangents_generated,
            normals_generated,
            morph_targets: stored.morph_targets,
        })
    }

    /// C#: `ReadIndices`: the primitive's triangle list, relative to its own vertices.
    fn read_primitive_indices(&mut self, primitive: &json::mesh::Primitive, mode: json::mesh::Mode, count: usize, place: &str, buffers: &BufferData<'_>) -> Option<Vec<u32>> {
        use json::mesh::Mode;
        let raw: Vec<u32> = match primitive.indices {
            None => (0..count as u32).collect(),
            Some(accessor) => match read_indices(&self.document.document, buffers, accessor.value(), &format!("{place} indices")) {
                Ok(indices) => indices,
                Err(problem) => {
                    self.warnings.push(format!("{problem}; it is left out."));
                    return None;
                }
            },
        };
        let mut list = match mode {
            Mode::TriangleStrip => from_strip(&raw),
            Mode::TriangleFan => from_fan(&raw),
            _ => raw,
        };
        let usable = list.len() - (list.len() % 3);
        if usable != list.len() {
            self.warnings.push(format!("{place} has {} indices (not a multiple of 3); the incomplete triangle is dropped.", list.len()));
            list.truncate(usable);
        }
        if let Some(bad) = list.iter().find(|index| **index as usize >= count) {
            self.warnings.push(format!("{place} has index {bad} for {count} vertices; it is left out."));
            return None;
        }
        if list.is_empty() {
            self.warnings.push(format!("{place} has no triangles; it is left out."));
            return None;
        }
        Some(list)
    }

    /// C#: `ReadMorphTargets` and `Deltas`: the deltas of every morph target, with the default
    /// weights baked into the vertices. None when a target's accessor cannot be read.
    fn read_morph_targets(&mut self, targets: &[[Option<usize>; 3]], count: usize, default_weights: &[f32], vertices: &mut [ModelVertex], place: &str, buffers: &BufferData<'_>) -> Option<Vec<ModelMorphTarget>> {
        let mut result = Vec::with_capacity(targets.len());
        let mut changed_normals = false;
        for (target, accessors) in targets.iter().enumerate() {
            let mut deltas: [Vec<Vec3>; 3] = [Vec::new(), Vec::new(), Vec::new()];
            for (kind, name) in ["POSITION", "NORMAL", "TANGENT"].iter().enumerate() {
                let Some(elements) = self.attribute(accessors[kind], &[3], &format!("morph target {target} {name}"), place, buffers).ok()? else {
                    continue;
                };
                deltas[kind] = (0..count).map(|index| Vec3::from(elements.get::<3>(index, [0.0; 3]).unwrap_or([0.0; 3]))).collect();
            }
            let [positions, normals, tangents] = deltas;
            let weight = default_weights.get(target).copied().unwrap_or(0.0);
            if weight != 0.0 {
                for (index, vertex) in vertices.iter_mut().enumerate() {
                    if !positions.is_empty() {
                        vertex.position = Vec3::from(add3(vertex.position.to_array(), scale3(positions[index].to_array(), weight)));
                    }
                    if !normals.is_empty() {
                        vertex.normal = Vec3::from(add3(vertex.normal.to_array(), scale3(normals[index].to_array(), weight)));
                        changed_normals = true;
                    }
                    if !tangents.is_empty() {
                        let delta = scale3(tangents[index].to_array(), weight);
                        vertex.tangent = [vertex.tangent[0] + delta[0], vertex.tangent[1] + delta[1], vertex.tangent[2] + delta[2], vertex.tangent[3] + 0.0];
                    }
                }
            }
            result.push(ModelMorphTarget { position_deltas: positions, normal_deltas: normals, tangent_deltas: tangents });
        }
        if changed_normals {
            for vertex in vertices.iter_mut() {
                let normal = vertex.normal.to_array();
                if length_squared3(normal) > 0.0 {
                    vertex.normal = Vec3::from(normalize3(normal));
                }
            }
        }
        Some(result)
    }

    // ------------------------------------------------------------------- skins and animations

    /// C#: `ReadSkins`: the joints and one inverse bind matrix per joint - the identity when the
    /// skin has no matrices (or fewer than joints, or matrices that cannot be read, with a
    /// warning).
    fn read_skins(&mut self, buffers: &BufferData<'_>) -> Vec<ModelSkin> {
        let mut skins = Vec::with_capacity(self.root.skins.len());
        for (index, skin) in self.root.skins.iter().enumerate() {
            let joints: Vec<usize> = skin.joints.iter().map(|joint| joint.value()).collect();
            let mut matrices = vec![Mat4::IDENTITY; joints.len()];
            if let Some(accessor) = skin.inverse_bind_matrices {
                let place = format!("Skin {index}{}", skin.name.as_ref().map(|name| format!(" '{name}'")).unwrap_or_default());
                match read_floats(&self.document.document, buffers, accessor.value(), &format!("{place} inverse bind matrices")) {
                    Ok(elements) if elements.components == 16 => {
                        if elements.count() < joints.len() {
                            self.warnings.push(format!("{place} has {} joints but {} inverse bind matrices; the missing ones are the identity.", joints.len(), elements.count()));
                        }
                        for (matrix, values) in matrices.iter_mut().zip(elements.values.chunks_exact(16)) {
                            *matrix = Mat4::from_cols_slice(values);
                        }
                    }
                    Ok(_) => self.warnings.push(format!("{place} has inverse bind matrices that are not MAT4; the identity is used.")),
                    Err(problem) => self.warnings.push(format!("{problem}; the identity is used.")),
                }
            }
            skins.push(ModelSkin { index, name: skin.name.clone(), joints, inverse_bind_matrices: matrices, skeleton: skin.skeleton.map(|node| node.value()) });
        }
        skins
    }

    /// C#: `ReadAnimations`.
    fn read_animations(&mut self, buffers: &BufferData<'_>) -> Vec<ModelAnimation> {
        let mut animations = Vec::with_capacity(self.root.animations.len());
        for (index, animation) in self.root.animations.iter().enumerate() {
            let mut channels = Vec::new();
            for channel in 0..animation.channels.len() {
                if let Some(decoded) = self.read_channel(index, channel, buffers) {
                    channels.push(decoded);
                }
            }
            animations.push(ModelAnimation { index, name: animation.name.clone(), channels });
        }
        animations
    }

    /// C#: `ReadChannel`: the key times and the packed key values of one channel - a key per
    /// input time that has an output value (three per key for a cubic spline: in-tangent, value,
    /// out-tangent). Rotation values are normalised; nothing else is changed.
    fn read_channel(&mut self, animation_index: usize, channel_index: usize, buffers: &BufferData<'_>) -> Option<ModelAnimationChannel> {
        use json::animation::{Interpolation, Property};
        let animation = &self.root.animations[animation_index];
        let channel = &animation.channels[channel_index];
        let place = format!("Animation {animation_index}{} channel {}", animation.name.as_ref().map(|name| format!(" '{name}'")).unwrap_or_default(), self.document.channel_number(animation_index, channel_index));
        let Some(sampler) = animation.samplers.get(channel.sampler.value()) else {
            self.warnings.push(format!("{place} has no sampler; it is left out."));
            return None;
        };
        let interpolation = match sampler.interpolation {
            Checked::Valid(Interpolation::Step) => AnimationInterpolation::Step,
            Checked::Valid(Interpolation::CubicSpline) => AnimationInterpolation::CubicSpline,
            _ => AnimationInterpolation::Linear,
        };
        let per_key = if interpolation == AnimationInterpolation::CubicSpline { 3 } else { 1 };
        let (path, expected) = match channel.target.path {
            Checked::Valid(Property::Translation) => (AnimationPath::Translation, 3),
            Checked::Valid(Property::Scale) => (AnimationPath::Scale, 3),
            Checked::Valid(Property::Rotation) => (AnimationPath::Rotation, 4),
            _ => (AnimationPath::Weights, 1),
        };
        let read = |builder: &mut Self, accessor: usize, what: &str| match read_floats(&builder.document.document, buffers, accessor, &format!("{place} {what}")) {
            Ok(elements) => Some(elements),
            Err(problem) => {
                builder.warnings.push(format!("{problem}; it is left out."));
                None
            }
        };
        let input = read(self, sampler.input.value(), "input")?;
        let output = read(self, sampler.output.value(), "output")?;
        if input.components != 1 || output.components != expected {
            self.warnings.push(format!("{place} has an input with {} components per key and an output with {}, where 1 and {expected} are needed; it is left out.", input.components, output.components));
            return None;
        }
        // Morph weights: one float per morph target per key, the number of targets worked out
        // from the two accessors' counts (C#: the glTF library's `Output.Count / Input.Count`).
        let components = if path == AnimationPath::Weights {
            let (inputs, outputs) = (self.root.accessors[sampler.input.value()].count.0, self.root.accessors[sampler.output.value()].count.0);
            match inputs.checked_mul(per_key as u64).filter(|n| *n > 0).map(|n| outputs / n) {
                Some(dimensions) if dimensions > 0 => dimensions as usize,
                _ => 0,
            }
        } else {
            expected
        };
        let keys = if components == 0 { 0 } else { input.count().min(output.values.len() / (components * per_key)) };
        if keys == 0 {
            self.warnings.push(format!("{place} has {} output values for {} keys; it is left out.", output.values.len(), input.count()));
            return None;
        }
        let times = input.values[..keys].to_vec();
        let mut values = output.values[..keys * per_key * components].to_vec();
        if path == AnimationPath::Rotation {
            // Of a cubic key (in-tangent, value, out-tangent) only the value is normalised.
            for (item, quaternion) in values.chunks_exact_mut(4).enumerate() {
                if per_key == 1 || item % 3 == 1 {
                    let unit = normalize_or_identity([quaternion[0], quaternion[1], quaternion[2], quaternion[3]]);
                    quaternion.copy_from_slice(&unit);
                }
            }
        }
        Some(ModelAnimationChannel { node: channel.target.node.value(), path, interpolation, times, values, component_count: components })
    }
}

/// The KHR_texture_transform of a normal or occlusion texture reference, whose extensions the
/// `gltf` crate keeps as JSON: (offset, rotation, scale, texCoord override).
fn transform_of(transform: Option<&json::Value>) -> Option<([f32; 2], f32, [f32; 2], Option<u32>)> {
    let transform = transform.filter(|value| value.is_object())?;
    let pair = |member: &str, fallback: [f32; 2]| -> [f32; 2] {
        match transform.get(member).and_then(json::Value::as_array) {
            Some(values) if values.len() >= 2 => [values[0].as_f64().map_or(fallback[0], |v| v as f32), values[1].as_f64().map_or(fallback[1], |v| v as f32)],
            _ => fallback,
        }
    };
    let rotation = transform.get("rotation").and_then(json::Value::as_f64).map_or(0.0, |v| v as f32);
    let tex_coord = transform.get("texCoord").and_then(json::Value::as_u64).and_then(|v| u32::try_from(v).ok());
    Some((pair("offset", [0.0, 0.0]), rotation, pair("scale", [1.0, 1.0]), tex_coord))
}
