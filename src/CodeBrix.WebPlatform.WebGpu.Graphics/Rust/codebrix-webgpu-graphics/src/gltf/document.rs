//! [`GltfDocument`]: a `.gltf` or `.glb` file parsed and checked, before any of its data is
//! read - the first of the loader's two steps. It says which other files the model names, so
//! that the application can fetch them (Not in C#: the reference converter's glTF library reads
//! the file and its satellite files in one call; a browser cannot wait for a fetch, so the two
//! steps are apart here).

use gltf::json;
use gltf::json::validation::{Error as ValidationError, Validate};

use crate::context::GraphicsException;
use crate::gltf::glb::{is_glb, split_glb};
use crate::gltf::uri::{combine_address, is_data_uri, unescape};

/// The extension names the loader refuses or reports by name.
pub(crate) const DRACO: &str = "KHR_draco_mesh_compression";
pub(crate) const MESHOPT: &str = "EXT_meshopt_compression";
pub(crate) const BASISU: &str = "KHR_texture_basisu";
pub(crate) const WEBP: &str = "EXT_texture_webp";
pub(crate) const DDS: &str = "MSFT_texture_dds";
pub(crate) const AVIF: &str = "EXT_texture_avif";
pub(crate) const ANIMATION_POINTER: &str = "KHR_animation_pointer";

/// What kind of file a glTF document names (Not in C#).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum GltfFileKind {
    /// A buffer: the model's vertex, index and animation data (usually a `.bin` file). The model
    /// cannot be built without it.
    Buffer,
    /// An image a texture uses (a PNG or JPEG file). Not needed when textures are not loaded.
    Image,
}

/// One file a glTF document names beside itself (Not in C#): a buffer or an image with a URI
/// that is not a `data:` URI. The application fetches it and hands its bytes to the build step
/// under [`GltfExternalFile::address`].
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct GltfExternalFile {
    /// What the file is.
    pub kind: GltfFileKind,
    /// The index of the buffer or image in the document.
    pub index: usize,
    /// The URI exactly as the document writes it (relative to the model file, percent-escaped).
    pub uri: String,
    /// The address the build step asks its file function for: the URI joined to the base address
    /// the list was asked with - '/' separators, percent escapes undone, "." and ".." resolved -
    /// or, without a base address, the URI with its percent escapes undone. Two models that name
    /// the same image get the same address, which is what lets them share one texture.
    pub address: String,
}

/// A glTF 2.0 document, parsed and checked (Not in C#; the first step of what the reference
/// converter's `GltfModelConverter.Convert` does in one call). Make one with
/// [`GltfDocument::parse`] from the bytes of a `.gltf` or `.glb` file, ask it for the files the
/// model names ([`GltfDocument::external_files`]), fetch them, and build the model data with
/// [`GltfModelSet::add`](crate::GltfModelSet::add).
///
/// Parsing checks the whole document: a file that is not glTF 2.0, is damaged, or names things
/// that are not there is refused with a message that says what is wrong - never a panic.
#[derive(Clone, Debug)]
pub struct GltfDocument {
    pub(crate) document: gltf::Document,
    /// The binary chunk of a `.glb` file (the buffer without a URI).
    pub(crate) blob: Option<Vec<u8>>,
    /// True when the bytes were a `.glb` container.
    pub(crate) binary: bool,
    /// What parsing had to leave out (the build step copies these into the model's warnings).
    pub(crate) warnings: Vec<String>,
    /// The materials that have NO `pbrMetallicRoughness` object. The `gltf` crate reads a missing
    /// object as one with glTF's defaults; the reference converter tells the two apart for a
    /// material with KHR_materials_pbrSpecularGlossiness, so only such documents are looked at.
    pub(crate) materials_without_pbr: Vec<usize>,
    /// For each animation, the number each of its channels has in the FILE; empty when no
    /// channel was taken out (the numbers are then the positions).
    pub(crate) channel_numbers: Vec<Vec<usize>>,
}

fn refuse(what: String) -> GraphicsException {
    GraphicsException::new(what)
}

impl GltfDocument {
    /// Parses the bytes of a `.gltf` (JSON) or `.glb` (binary) file and checks the document. No
    /// buffer or image data is read yet, and nothing is fetched: see
    /// [`GltfDocument::external_files`]. An error says what is wrong with the file and what to
    /// do about it. Refused by name: a model that REQUIRES Draco mesh compression
    /// (`KHR_draco_mesh_compression`) or meshopt compression (`EXT_meshopt_compression`).
    pub fn parse(bytes: &[u8]) -> Result<GltfDocument, GraphicsException> {
        if bytes.is_empty() {
            return Err(refuse("The glTF file is empty (0 bytes). Hand over the bytes of a .gltf or .glb file.".to_string()));
        }
        let binary = is_glb(bytes);
        let (json_bytes, blob) = if binary {
            let parts = split_glb(bytes)?;
            (parts.json, parts.bin.map(<[u8]>::to_vec))
        } else {
            (bytes.strip_prefix(&[0xEF, 0xBB, 0xBF]).unwrap_or(bytes), None)
        };
        if !binary && !json_bytes.iter().find(|b| !b.is_ascii_whitespace()).is_some_and(|b| *b == b'{') {
            return Err(refuse(format!("The {} bytes are neither a .glb file (which starts with \"glTF\") nor a .gltf file (JSON, which starts with '{{'). Hand over the bytes of the model file itself; model data made by the backend's ModelDataWriter (\"CBMD\") goes to load_model instead.", bytes.len())));
        }
        let mut warnings = Vec::new();
        let (root, channel_numbers) = read_root(json_bytes, &mut warnings)?;
        check_version(&root)?;
        check_required_extensions(&root, &mut warnings)?;
        check_validation(&root)?;
        check_references(&root)?;
        let materials_without_pbr = materials_without_pbr(&root, json_bytes);
        Ok(GltfDocument { document: gltf::Document::from_json_without_validation(root), blob, binary, warnings, materials_without_pbr, channel_numbers })
    }

    /// True when material `index` has a `pbrMetallicRoughness` object (see
    /// `materials_without_pbr`).
    pub(crate) fn has_pbr(&self, index: usize) -> bool {
        !self.materials_without_pbr.contains(&index)
    }

    /// The number a channel has in the file's animation (its position, unless parsing took
    /// channels out before it).
    pub(crate) fn channel_number(&self, animation: usize, channel: usize) -> usize {
        self.channel_numbers.get(animation).and_then(|numbers| numbers.get(channel)).copied().unwrap_or(channel)
    }

    /// The files the model names beside itself, in document order - every buffer, then every
    /// image, that has a URI which is not a `data:` URI; none for a self-contained `.glb` or a
    /// `.gltf` with only `data:` URIs. `base_address` is the model's own folder (for example
    /// `"models/forest"`, or a URL): each file's [`GltfExternalFile::address`] is its URI joined
    /// to it. Pass the SAME base address to [`GltfModelSet::add`](crate::GltfModelSet::add),
    /// whose file function is then asked for exactly these addresses. With `images` false the
    /// images are left out (for a build with
    /// [`GltfLoadOptions::load_textures`](crate::GltfLoadOptions::load_textures) off).
    pub fn external_files(&self, base_address: Option<&str>, images: bool) -> Vec<GltfExternalFile> {
        let root = self.document.as_json();
        let mut files = Vec::new();
        let mut add = |kind: GltfFileKind, index: usize, uri: &Option<String>| {
            if let Some(uri) = uri.as_deref().filter(|uri| !is_data_uri(uri)) {
                files.push(GltfExternalFile { kind, index, uri: uri.to_string(), address: address_of(base_address, uri) });
            }
        };
        for (index, buffer) in root.buffers.iter().enumerate() {
            add(GltfFileKind::Buffer, index, &buffer.uri);
        }
        if images {
            for (index, image) in root.images.iter().enumerate() {
                if image.buffer_view.is_none() {
                    add(GltfFileKind::Image, index, &image.uri);
                }
            }
        }
        files
    }

    /// True when the bytes were a binary `.glb` file, false for a `.gltf` (JSON) file.
    pub fn is_binary(&self) -> bool {
        self.binary
    }

    /// What parsing left out or could not use (for example animation channels of
    /// `KHR_animation_pointer`); the build step adds these to the model's warnings.
    pub fn warnings(&self) -> &[String] {
        &self.warnings
    }

    /// The extensions the document says it uses (`extensionsUsed`).
    pub fn extensions_used(&self) -> &[String] {
        &self.document.as_json().extensions_used
    }

    /// The extensions the document says it requires (`extensionsRequired`).
    pub fn extensions_required(&self) -> &[String] {
        &self.document.as_json().extensions_required
    }
}

/// The address of a URI: joined to the base address, or just unescaped without one.
pub(crate) fn address_of(base_address: Option<&str>, uri: &str) -> String {
    match base_address {
        Some(base) => combine_address(base, uri),
        None => unescape(uri).replace('\\', "/"),
    }
}

/// The node and property a KHR_animation_pointer pointer names when it is an ordinary node
/// channel in other words: "/nodes/<index>/translation", "/rotation", "/scale" or "/weights"
/// (C#: the glTF library's `AnimationPointer.TryParseNodeTransform`).
fn node_pointer(pointer: &str) -> Option<(u64, &str)> {
    let rest = pointer.strip_prefix("/nodes/")?;
    let (index, property) = rest.split_once('/')?;
    let index = index.trim().parse::<u64>().ok()?;
    matches!(property, "translation" | "rotation" | "scale" | "weights").then_some((index, property))
}

/// Reads the JSON into the `gltf` crate's typed document. A channel of KHR_animation_pointer does
/// not fit those types (it has the path "pointer", and often no node), so a document that uses
/// the extension - or that does not read at first - is gone through as plain JSON first: a
/// pointer that names a node's translation, rotation, scale or weights becomes the ordinary
/// channel it is, and every other channel without a node is taken out with a warning (C#: the
/// reference converter's glTF library reads the first kind as a node channel, and the builder
/// leaves the second kind out with this warning). Returns the document and, when channels were
/// taken out, each animation's remaining channels' numbers in the file.
fn read_root(json_bytes: &[u8], warnings: &mut Vec<String>) -> Result<(json::Root, Vec<Vec<usize>>), GraphicsException> {
    let first = match json::deserialize::from_slice::<json::Root>(json_bytes) {
        Ok(root) if !root.extensions_used.iter().any(|name| name == ANIMATION_POINTER) => return Ok((root, Vec::new())),
        Ok(_) => None,
        Err(error) => Some(error.to_string()),
    };
    let not_gltf = |error: &str| refuse(format!("The glTF document could not be read: {error}. The file is not valid glTF 2.0 JSON; export the model again, or check it with the Khronos glTF validator."));
    let Ok(mut value) = json::deserialize::from_slice::<json::Value>(json_bytes) else {
        return Err(not_gltf(first.as_deref().unwrap_or("it is not JSON")));
    };
    let mut removed = Vec::new();
    let mut changed = false;
    let mut channel_numbers: Vec<Vec<usize>> = Vec::new();
    if let Some(animations) = value.get_mut("animations").and_then(json::Value::as_array_mut) {
        for (animation_index, animation) in animations.iter_mut().enumerate() {
            let name = animation.get("name").and_then(json::Value::as_str).map(str::to_string);
            let mut numbers = Vec::new();
            if let Some(channels) = animation.get_mut("channels").and_then(json::Value::as_array_mut) {
                let mut channel_index = 0usize;
                channels.retain_mut(|channel| {
                    let index = channel_index;
                    channel_index += 1;
                    let target = channel.get("target");
                    let has_node = target.and_then(|t| t.get("node")).is_some_and(json::Value::is_u64);
                    let is_pointer = target.and_then(|t| t.get("path")).and_then(json::Value::as_str) == Some("pointer");
                    if has_node && !is_pointer {
                        numbers.push(index);
                        return true;
                    }
                    let pointer = target.and_then(|t| t.get("extensions")).and_then(|e| e.get(ANIMATION_POINTER)).and_then(|p| p.get("pointer")).and_then(json::Value::as_str).unwrap_or("").to_string();
                    if let (true, Some((node, property))) = (is_pointer, node_pointer(&pointer)) {
                        if let Some(target) = channel.get_mut("target").and_then(json::Value::as_object_mut) {
                            target.insert("node".to_string(), json::Value::from(node));
                            target.insert("path".to_string(), json::Value::from(property));
                            changed = true;
                            numbers.push(index);
                            return true;
                        }
                    }
                    removed.push(format!("Animation {animation_index}{} channel {index} targets '{pointer}' ({ANIMATION_POINTER}); only node channels are loaded.", name.as_ref().map(|n| format!(" '{n}'")).unwrap_or_default()));
                    false
                });
            }
            channel_numbers.push(numbers);
        }
    }
    if removed.is_empty() && !changed {
        if let Some(error) = first {
            return Err(not_gltf(&error));
        }
    }
    if removed.is_empty() {
        channel_numbers.clear();
    }
    match json::deserialize::from_value::<json::Root>(value) {
        Ok(root) => {
            warnings.append(&mut removed);
            Ok((root, channel_numbers))
        }
        Err(error) => Err(not_gltf(&error.to_string())),
    }
}

/// The materials without a `pbrMetallicRoughness` object, looked for only in a document that has
/// a KHR_materials_pbrSpecularGlossiness material (see `GltfDocument::materials_without_pbr`).
fn materials_without_pbr(root: &json::Root, json_bytes: &[u8]) -> Vec<usize> {
    if !root.materials.iter().any(|m| m.extensions.as_ref().is_some_and(|e| e.pbr_specular_glossiness.is_some())) {
        return Vec::new();
    }
    let Ok(value) = json::deserialize::from_slice::<json::Value>(json_bytes) else {
        return Vec::new();
    };
    let Some(materials) = value.get("materials").and_then(json::Value::as_array) else {
        return Vec::new();
    };
    materials.iter().enumerate().filter(|(_, material)| !material.get("pbrMetallicRoughness").is_some_and(json::Value::is_object)).map(|(index, _)| index).collect()
}

fn check_version(root: &json::Root) -> Result<(), GraphicsException> {
    let version = root.asset.version.trim();
    let major = version.split('.').next().unwrap_or("");
    if major != "2" {
        return Err(refuse(format!("The file is glTF version \"{version}\"; only glTF 2.0 is read. Convert the model to glTF 2.0.")));
    }
    if let Some(minimum) = root.asset.min_version.as_deref() {
        let mut parts = minimum.trim().split('.');
        let (major, minor) = (parts.next().unwrap_or(""), parts.next().unwrap_or("0"));
        if major != "2" || minor.parse::<u32>().map_or(true, |minor| minor > 0) {
            return Err(refuse(format!("The file needs a reader for glTF version \"{minimum}\" or later (its minVersion); this loader reads glTF 2.0. Export the model as plain glTF 2.0.")));
        }
    }
    Ok(())
}

/// Refuses the compression extensions by name when the document REQUIRES them, and notes every
/// other required extension the loader does not handle (the reference converter reads such a
/// file as if the extension were not there; so does the loader, and says so).
fn check_required_extensions(root: &json::Root, warnings: &mut Vec<String>) -> Result<(), GraphicsException> {
    /// What the loader reads, captures, approximates, reports per item, or can safely ignore.
    const HANDLED: [&str; 23] = [
        "KHR_materials_emissive_strength",
        "KHR_materials_ior",
        "KHR_materials_specular",
        "KHR_materials_transmission",
        "KHR_materials_unlit",
        "KHR_materials_pbrSpecularGlossiness",
        "KHR_materials_clearcoat",
        "KHR_materials_sheen",
        "KHR_materials_volume",
        "KHR_materials_iridescence",
        "KHR_materials_anisotropy",
        "KHR_materials_diffuse_transmission",
        "KHR_materials_dispersion",
        "KHR_materials_variants",
        "KHR_texture_transform",
        "KHR_mesh_quantization",
        "KHR_lights_punctual",
        "KHR_xmp_json_ld",
        BASISU,
        WEBP,
        DDS,
        AVIF,
        ANIMATION_POINTER,
    ];
    for extension in &root.extensions_required {
        match extension.as_str() {
            DRACO => return Err(refuse(format!("The model requires Draco mesh compression ({DRACO}), which this loader does not decode. Export the model without Draco compression."))),
            MESHOPT => return Err(refuse(format!("The model requires meshopt compression ({MESHOPT}), which this loader does not decode. Export the model without meshopt compression (for gltfpack: without -c / -cc)."))),
            known if HANDLED.contains(&known) => {}
            other => warnings.push(format!("The model requires the extension {other}, which this loader does not know; the model is read without it and may not look as intended.")),
        }
    }
    Ok(())
}

/// Runs the `gltf` crate's validation (every index in range, every enumeration a known value,
/// every size within this machine's limits) and turns what it finds into one message. Three kinds
/// of finding are NOT errors here, because the reference converter reads such files: a required
/// extension outside the crate's own list (the loader's list decided above), an attribute name
/// the crate does not know (such as an application's `_CUSTOM` attribute), and a POSITION
/// accessor without `min`/`max` or a primitive without POSITION (the builder leaves that
/// primitive out with a warning).
fn check_validation(root: &json::Root) -> Result<(), GraphicsException> {
    // The crate's validation itself looks up each primitive's POSITION accessor BEFORE it has
    // checked that index (gltf-json's primitive hook), so that one is checked here first.
    for (mesh_index, mesh) in root.meshes.iter().enumerate() {
        for (primitive_index, primitive) in mesh.primitives.iter().enumerate() {
            if let Some((_, accessor)) = primitive.attributes.iter().find(|(_, accessor)| accessor.value() >= root.accessors.len()) {
                return Err(refuse(format!("The glTF document is not valid: meshes[{mesh_index}].primitives[{primitive_index}] has an attribute that names accessor {}, but the document has {} accessor(s). Export the model again, or check it with the Khronos glTF validator.", accessor.value(), root.accessors.len())));
            }
        }
    }
    let mut problems: Vec<String> = Vec::new();
    let mut total = 0usize;
    root.validate(root, json::Path::new, &mut |path, error| {
        let path = path();
        let text = path.as_str();
        let tolerated = match error {
            ValidationError::Unsupported => true,
            ValidationError::Invalid | ValidationError::Missing => text.contains(".attributes["),
            _ => false,
        };
        if !tolerated {
            total += 1;
            if problems.len() < 5 {
                let what = match error {
                    ValidationError::IndexOutOfBounds => "names an item that is not in the document (index out of range)",
                    ValidationError::Invalid => "has a value glTF 2.0 does not allow",
                    ValidationError::Missing => "is missing",
                    ValidationError::Oversize => "is larger than this machine can address",
                    ValidationError::Unsupported => "is not supported",
                };
                problems.push(format!("{text} {what}"));
            }
        }
    });
    if total == 0 {
        return Ok(());
    }
    let more = if total > problems.len() { format!(" (and {} more)", total - problems.len()) } else { String::new() };
    Err(refuse(format!("The glTF document is not valid: {}{more}. Export the model again, or check it with the Khronos glTF validator.", problems.join("; "))))
}

/// The checks the `gltf` crate does not make and the loader (or the renderer after it) relies
/// on: animation channel targets, the declared byte ranges of buffer views, and a node hierarchy
/// that is a forest (no node with two parents, no node that is its own ancestor).
fn check_references(root: &json::Root) -> Result<(), GraphicsException> {
    let invalid = |what: String| refuse(format!("The glTF document is not valid: {what}. Export the model again, or check it with the Khronos glTF validator."));
    let node_count = root.nodes.len();
    for (animation_index, animation) in root.animations.iter().enumerate() {
        for (channel_index, channel) in animation.channels.iter().enumerate() {
            if channel.target.node.value() >= node_count {
                return Err(invalid(format!("animation {animation_index} channel {channel_index} targets node {}, but the document has {node_count} node(s)", channel.target.node.value())));
            }
            if matches!(channel.target.path, json::validation::Checked::Invalid) {
                return Err(invalid(format!("animation {animation_index} channel {channel_index} has a target path that is not translation, rotation, scale or weights")));
            }
        }
    }
    for (index, view) in root.buffer_views.iter().enumerate() {
        let buffer = &root.buffers[view.buffer.value()];
        let end = view.byte_offset.map_or(0, |offset| offset.0).checked_add(view.byte_length.0);
        if end.is_none_or(|end| end > buffer.byte_length.0) {
            return Err(invalid(format!("buffer view {index} covers bytes {} to {} of buffer {}, which is {} bytes long", view.byte_offset.map_or(0, |offset| offset.0), view.byte_offset.map_or(0, |offset| offset.0) as u128 + view.byte_length.0 as u128, view.buffer.value(), buffer.byte_length.0)));
        }
    }
    let mut parent: Vec<Option<usize>> = vec![None; node_count];
    for (index, node) in root.nodes.iter().enumerate() {
        for child in node.children.iter().flatten() {
            let child = child.value();
            if child == index {
                return Err(invalid(format!("node {index} is its own child")));
            }
            if let Some(other) = parent[child] {
                return Err(invalid(format!("node {child} is a child of both node {other} and node {index}; a glTF node hierarchy is a tree")));
            }
            parent[child] = Some(index);
        }
    }
    for start in 0..node_count {
        let mut current = parent[start];
        let mut steps = 0usize;
        while let Some(node) = current {
            steps += 1;
            if node == start || steps > node_count {
                return Err(invalid(format!("node {start} is its own ancestor; a glTF node hierarchy is a tree")));
            }
            current = parent[node];
        }
    }
    Ok(())
}
