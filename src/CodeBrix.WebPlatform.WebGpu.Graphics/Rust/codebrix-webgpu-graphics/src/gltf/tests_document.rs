//! Host tests of parsing: the three packagings, the files a document names, the refusals by name,
//! and - above all - that a damaged or hostile file is answered with a message and never a panic.

use std::cell::Cell;
use std::collections::HashMap;

use crate::gltf::test_support::{build, glb, png, triangle_document, Buffer};
use crate::gltf::{GltfDocument, GltfFileKind, GltfLoadOptions, GltfModelSet};

fn parse_error(json: &str) -> String {
    GltfDocument::parse(json.as_bytes()).expect_err("the document must be refused").message().to_string()
}

/// A triangle with one textured material: the buffer and the image are whatever the caller
/// writes for them.
fn textured_triangle(buffer_members: &str, image: &str) -> (String, Vec<u8>) {
    let mut buffer = Buffer::default();
    let positions = buffer.floats(&[0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0], "VEC3");
    let uvs = buffer.floats(&[0.0, 0.0, 1.0, 0.0, 0.0, 1.0], "VEC2");
    let indices = buffer.u16s(&[0, 1, 2]);
    let rest = format!(
        "\"meshes\":[{{\"primitives\":[{{\"attributes\":{{\"POSITION\":{positions},\"TEXCOORD_0\":{uvs}}},\"indices\":{indices},\"material\":0}}]}}],\"nodes\":[{{\"mesh\":0}}],\"scenes\":[{{\"nodes\":[0]}}],\"materials\":[{{\"pbrMetallicRoughness\":{{\"baseColorTexture\":{{\"index\":0}}}}}}],\"textures\":[{{\"source\":0}}],\"images\":[{image}]"
    );
    (buffer.document_with_buffer(buffer_members, &rest), buffer.bytes)
}

#[test]
fn a_gltf_with_data_uris_names_no_file_and_builds() {
    let document = GltfDocument::parse(triangle_document("", "").as_bytes()).unwrap();
    assert!(!document.is_binary());
    assert!(document.external_files(Some("models"), true).is_empty());
    assert!(document.warnings().is_empty());
    let set = build(&triangle_document("", ""));
    assert_eq!((set.model_count(), set.texture_count()), (1, 0));
    assert_eq!(set.model_name(0), Some("test"));
    assert!(set.warnings(0).is_empty());
    assert_eq!(set.data_set().models[0].vertex_count, 3);
}

#[test]
fn a_gltf_with_external_files_lists_them_and_asks_for_exactly_those_addresses() {
    let pixels = png(1, 1, &[255, 0, 0, 255]);
    let (json, bytes) = textured_triangle(",\"uri\":\"data%20files/model.bin\"", "{\"uri\":\"../Textures/colour.png\"}");
    let document = GltfDocument::parse(json.as_bytes()).unwrap();
    let files = document.external_files(Some("packs/kit/models"), true);
    assert_eq!(files.len(), 2);
    assert_eq!((files[0].kind, files[0].index, files[0].uri.as_str(), files[0].address.as_str()), (GltfFileKind::Buffer, 0, "data%20files/model.bin", "packs/kit/models/data files/model.bin"));
    assert_eq!((files[1].kind, files[1].uri.as_str(), files[1].address.as_str()), (GltfFileKind::Image, "../Textures/colour.png", "packs/kit/Textures/colour.png"));
    // Without images (a build with textures off) only the buffer is listed; without a base
    // address the address is the URI with its escapes undone.
    assert_eq!(document.external_files(Some("packs/kit/models"), false).len(), 1);
    assert_eq!(document.external_files(None, true)[0].address, "data files/model.bin");

    let supplied: HashMap<String, Vec<u8>> = HashMap::from([(files[0].address.clone(), bytes), (files[1].address.clone(), pixels)]);
    let asked = std::cell::RefCell::new(Vec::new());
    let mut set = GltfModelSet::new();
    let resolve = |address: &str| {
        asked.borrow_mut().push(address.to_string());
        supplied.get(address).map(Vec::as_slice)
    };
    set.add(&document, "tri", Some("packs/kit/models"), &resolve, &GltfLoadOptions::default()).unwrap();
    assert_eq!(*asked.borrow(), vec!["packs/kit/models/data files/model.bin".to_string(), "packs/kit/Textures/colour.png".to_string()]);
    let data = set.data_set();
    assert_eq!(data.textures.len(), 1);
    assert_eq!(data.textures[0].name.as_deref(), Some("packs/kit/Textures/colour.png"));
    assert_eq!((data.textures[0].width, data.textures[0].height, data.textures[0].srgb, data.textures[0].mipmaps), (1, 1, true, true));
    assert_eq!(data.textures[0].pixels, &[255, 0, 0, 255]);
}

#[test]
fn a_glb_carries_its_buffer_and_its_image() {
    let mut buffer = Buffer::default();
    let positions = buffer.floats(&[0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0], "VEC3");
    let uvs = buffer.floats(&[0.0, 0.0, 1.0, 0.0, 0.0, 1.0], "VEC2");
    let image_view = buffer.view(&png(2, 1, &[1, 2, 3, 255, 4, 5, 6, 255]));
    let rest = format!(
        "\"meshes\":[{{\"name\":\"m\",\"primitives\":[{{\"attributes\":{{\"POSITION\":{positions},\"TEXCOORD_0\":{uvs}}},\"material\":0}}]}}],\"nodes\":[{{\"mesh\":0}}],\"materials\":[{{\"pbrMetallicRoughness\":{{\"baseColorTexture\":{{\"index\":0}}}}}}],\"textures\":[{{\"source\":0}}],\"images\":[{{\"bufferView\":{image_view},\"mimeType\":\"image/png\",\"name\":\"pic\"}}]"
    );
    let file = glb(&buffer.document_with_buffer("", &rest), Some(&buffer.bytes));
    let document = GltfDocument::parse(&file).unwrap();
    assert!(document.is_binary());
    assert!(document.external_files(None, true).is_empty());
    let mut set = GltfModelSet::new();
    set.add(&document, "boxed", Some("anywhere"), &|_| None, &GltfLoadOptions::default()).unwrap();
    let data = set.data_set();
    assert_eq!(data.textures[0].pixels, &[1, 2, 3, 255, 4, 5, 6, 255]);
    // An image inside the file is named after the model and the image, and is not shared.
    assert_eq!(data.textures[0].name.as_deref(), Some("boxed image 0 'pic'"));
    assert!(set.shared.is_empty());
    // No scene: the parentless node is the root.
    assert_eq!(data.models[0].root_nodes, vec![0]);
    assert!(data.models[0].nodes[0].in_scene);
}

#[test]
fn a_file_that_was_not_supplied_is_an_error_that_names_it_and_leaves_the_set_as_it_was() {
    let (json, bytes) = textured_triangle(",\"uri\":\"model.bin\"", "{\"uri\":\"colour.png\"}");
    let document = GltfDocument::parse(json.as_bytes()).unwrap();
    let mut set = GltfModelSet::new();
    let error = set.add(&document, "tri", Some("m"), &|_| None, &GltfLoadOptions::default()).unwrap_err();
    assert!(error.message().contains("'model.bin'") && error.message().contains("'m/model.bin'") && error.message().contains("was not supplied"), "{}", error.message());
    // The buffer is there, the image is not: an error too, and nothing stays in the set.
    let error = set.add(&document, "tri", Some("m"), &|address| (address == "m/model.bin").then_some(bytes.as_slice()), &GltfLoadOptions::default()).unwrap_err();
    assert!(error.message().contains("'colour.png'"), "{}", error.message());
    assert_eq!((set.model_count(), set.texture_count()), (0, 0));
    // With textures off the image is not asked for.
    let asked = Cell::new(0);
    let resolve = |address: &str| {
        asked.set(asked.get() + 1);
        (address == "m/model.bin").then_some(bytes.as_slice())
    };
    set.add(&document, "tri", Some("m"), &resolve, &GltfLoadOptions { load_textures: false, ..GltfLoadOptions::default() }).unwrap();
    assert_eq!((asked.get(), set.texture_count()), (1, 0));
    // A buffer shorter than the document says is refused too.
    let error = set.add(&document, "tri", Some("m"), &|_| Some(&bytes[..10]), &GltfLoadOptions { load_textures: false, ..GltfLoadOptions::default() }).unwrap_err();
    assert!(error.message().contains("10 bytes long"), "{}", error.message());
}

#[test]
fn draco_and_meshopt_are_refused_by_name_when_required() {
    let draco = triangle_document("", ",\"extensionsUsed\":[\"KHR_draco_mesh_compression\"],\"extensionsRequired\":[\"KHR_draco_mesh_compression\"]");
    let message = parse_error(&draco);
    assert!(message.contains("Draco") && message.contains("KHR_draco_mesh_compression") && message.contains("Export the model without Draco"), "{message}");
    let meshopt = triangle_document("", ",\"extensionsUsed\":[\"EXT_meshopt_compression\"],\"extensionsRequired\":[\"EXT_meshopt_compression\"]");
    let message = parse_error(&meshopt);
    assert!(message.contains("meshopt") && message.contains("EXT_meshopt_compression"), "{message}");
    // Another required extension the loader does not know: read, with a warning that names it.
    let other = triangle_document("", ",\"extensionsUsed\":[\"VENDOR_thing\"],\"extensionsRequired\":[\"VENDOR_thing\"]");
    let document = GltfDocument::parse(other.as_bytes()).unwrap();
    assert!(document.warnings()[0].contains("VENDOR_thing"));
    assert_eq!(document.extensions_required(), ["VENDOR_thing".to_string()]);
    // Extensions the loader handles are no warning, required or not.
    let handled = triangle_document("", ",\"extensionsUsed\":[\"KHR_mesh_quantization\",\"KHR_texture_transform\"],\"extensionsRequired\":[\"KHR_mesh_quantization\",\"KHR_texture_transform\"]");
    assert!(GltfDocument::parse(handled.as_bytes()).unwrap().warnings().is_empty());
}

#[test]
fn a_draco_primitive_without_plain_data_is_left_out_by_name() {
    // Not required: the document parses; the primitive has only compressed data.
    let json = "{\"asset\":{\"version\":\"2.0\"},\"extensionsUsed\":[\"KHR_draco_mesh_compression\"],\"buffers\":[{\"byteLength\":4,\"uri\":\"data:application/octet-stream;base64,AAAAAA==\"}],\"bufferViews\":[{\"buffer\":0,\"byteLength\":4}],\"accessors\":[{\"componentType\":5126,\"count\":3,\"type\":\"VEC3\",\"min\":[0,0,0],\"max\":[1,1,1],\"sparse\":{\"count\":1,\"indices\":{\"bufferView\":0,\"componentType\":5121},\"values\":{\"bufferView\":0}}}],\"meshes\":[{\"primitives\":[{\"attributes\":{\"POSITION\":0},\"extensions\":{\"KHR_draco_mesh_compression\":{\"bufferView\":0,\"attributes\":{\"POSITION\":0}}}}]}]}";
    let set = build(json);
    // The accessor has sparse data, so it counts as plain data here - and is damaged: the
    // primitive is left out with the accessor's message, not a panic.
    assert_eq!(set.data_set().models[0].meshes[0].primitives.len(), 0);
    assert_eq!(set.warnings(0).len(), 1);
    let no_data = json.replace(",\"sparse\":{\"count\":1,\"indices\":{\"bufferView\":0,\"componentType\":5121},\"values\":{\"bufferView\":0}}", "");
    // An accessor with neither a buffer view nor sparse data does not pass the document check.
    assert!(parse_error(&no_data).contains("accessors[0].bufferView is missing"));
}

#[test]
fn a_file_that_is_not_gltf_2_is_refused_with_a_message() {
    assert!(parse_error("").contains("empty"));
    assert!(parse_error("hello").contains("neither a .glb file"));
    assert!(GltfDocument::parse(b"CBMD\x01\x00\x00\x00").unwrap_err().message().contains("load_model"));
    assert!(parse_error("{").contains("could not be read"));
    assert!(parse_error("{}").contains("could not be read"));
    assert!(parse_error("{\"asset\":{\"version\":\"1.0\"}}").contains("glTF version \"1.0\""));
    assert!(parse_error("{\"asset\":{\"version\":\"2.0\",\"minVersion\":\"2.1\"}}").contains("minVersion"));
    // A UTF-8 byte order mark is tolerated; so is nothing but an asset.
    assert!(GltfDocument::parse("\u{feff}{\"asset\":{\"version\":\"2.0\"}}".as_bytes()).is_ok());
    let empty = build("{\"asset\":{\"version\":\"2.0\"}}");
    assert_eq!((empty.data_set().models[0].vertex_count, empty.data_set().models[0].index_format), (0, wgpu::IndexFormat::Uint32));
}

#[test]
fn a_document_that_names_things_that_are_not_there_is_refused_with_the_place() {
    let message = parse_error(&triangle_document("", ",\"skins\":[{\"joints\":[7]}]"));
    assert!(message.contains("skins[0].joints[0]") && message.contains("index out of range"), "{message}");
    let message = parse_error(&triangle_document(",\"material\":3", ""));
    assert!(message.contains("meshes[0].primitives[0].material"), "{message}");
    let message = parse_error(&triangle_document("", ",\"animations\":[{\"channels\":[{\"sampler\":0,\"target\":{\"node\":9,\"path\":\"translation\"}}],\"samplers\":[{\"input\":0,\"output\":0}]}]"));
    assert!(message.contains("animation 0 channel 0 targets node 9"), "{message}");
    let message = parse_error(&triangle_document("", ",\"animations\":[{\"channels\":[{\"sampler\":0,\"target\":{\"node\":0,\"path\":\"colour\"}}],\"samplers\":[{\"input\":0,\"output\":0}]}]"));
    assert!(message.contains("target path"), "{message}");
    // A buffer view that runs past its buffer.
    let json = triangle_document("", "").replace("\"byteLength\":36}", "\"byteLength\":4000}");
    assert!(parse_error(&json).contains("buffer view 0 covers bytes 0 to 4000"), "{}", parse_error(&json));
    // A node hierarchy that is not a tree.
    let cycle = "{\"asset\":{\"version\":\"2.0\"},\"nodes\":[{\"children\":[1]},{\"children\":[0]}]}";
    assert!(parse_error(cycle).contains("its own ancestor"));
    let two_parents = "{\"asset\":{\"version\":\"2.0\"},\"nodes\":[{\"children\":[2]},{\"children\":[2]},{}]}";
    assert!(parse_error(two_parents).contains("child of both node 0 and node 1"));
    assert!(parse_error("{\"asset\":{\"version\":\"2.0\"},\"nodes\":[{\"children\":[0]}]}").contains("its own child"));
}

#[test]
fn what_the_reference_reads_is_read_an_unknown_attribute_and_a_position_without_bounds() {
    let mut buffer = Buffer::default();
    let positions = buffer.accessor(&[0u8; 36], 5126, 3, "VEC3", "");
    let custom = buffer.accessor(&[0u8; 12], 5126, 3, "SCALAR", "");
    let json = buffer.document(&format!("\"meshes\":[{{\"primitives\":[{{\"attributes\":{{\"POSITION\":{positions},\"_CUSTOM\":{custom},\"COLOR_x\":{custom}}}}}]}}]"));
    let set = build(&json);
    assert_eq!(set.data_set().models[0].vertex_count, 3);
    // A primitive without POSITION is left out with the reference's warning.
    let none = buffer.document(&format!("\"meshes\":[{{\"name\":\"m\",\"primitives\":[{{\"attributes\":{{\"NORMAL\":{positions}}}}}]}}]"));
    let set = build(&none);
    assert_eq!(set.warnings(0), ["Mesh 0 'm' primitive 0 has no POSITION; it is left out.".to_string()]);
}

#[test]
fn animation_pointer_channels_become_node_channels_or_are_left_out_with_a_warning() {
    let mut buffer = Buffer::default();
    let times = buffer.floats(&[0.0, 1.0], "SCALAR");
    let values = buffer.floats(&[0.0, 0.0, 0.0, 1.0, 2.0, 3.0], "VEC3");
    let channel = |target: &str| format!("{{\"sampler\":0,\"target\":{target}}}");
    let pointer = |path: &str| format!("{{\"path\":\"pointer\",\"extensions\":{{\"KHR_animation_pointer\":{{\"pointer\":\"{path}\"}}}}}}");
    let channels = [channel(&pointer("/materials/0/emissiveFactor")), channel("{\"node\":0,\"path\":\"scale\"}"), channel(&pointer("/nodes/0/translation")), channel(&pointer("/nodes/0/extensions/x"))].join(",");
    let json = buffer.document(&format!("\"extensionsUsed\":[\"KHR_animation_pointer\"],\"nodes\":[{{}}],\"animations\":[{{\"name\":\"a\",\"channels\":[{channels}],\"samplers\":[{{\"input\":{times},\"output\":{values}}}]}}]"));
    let document = GltfDocument::parse(json.as_bytes()).unwrap();
    assert_eq!(document.warnings(), ["Animation 0 'a' channel 0 targets '/materials/0/emissiveFactor' (KHR_animation_pointer); only node channels are loaded.".to_string(), "Animation 0 'a' channel 3 targets '/nodes/0/extensions/x' (KHR_animation_pointer); only node channels are loaded.".to_string()]);
    let set = build(&json);
    let data = set.data_set();
    let kept = &data.models[0].animations[0].channels;
    assert_eq!(kept.len(), 2);
    assert_eq!((kept[0].path, kept[1].path), (crate::AnimationPath::Scale, crate::AnimationPath::Translation));
    assert_eq!(kept[1].values, vec![0.0, 0.0, 0.0, 1.0, 2.0, 3.0]);
    // The file's channel numbers are kept for messages.
    assert_eq!((document.channel_number(0, 0), document.channel_number(0, 1)), (1, 2));
    // A channel without a node and without the extension is left out the same way.
    let plain = buffer.document(&format!("\"nodes\":[{{}}],\"animations\":[{{\"channels\":[{{\"sampler\":0,\"target\":{{\"path\":\"scale\"}}}}],\"samplers\":[{{\"input\":{times},\"output\":{values}}}]}}]"));
    assert_eq!(GltfDocument::parse(plain.as_bytes()).unwrap().warnings(), ["Animation 0 channel 0 targets '' (KHR_animation_pointer); only node channels are loaded.".to_string()]);
}

/// A document that exercises most of the loader: a skinned, morphed, textured, animated mesh in
/// strips, with a sparse accessor, a matrix node and material extensions.
fn rich_document() -> String {
    let mut buffer = Buffer::default();
    let positions = buffer.floats(&[0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 1.0, 1.0, 0.0], "VEC3");
    let normals = buffer.floats(&[0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0], "VEC3");
    let uvs = buffer.floats(&[0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 1.0, 1.0], "VEC2");
    let joints = buffer.accessor(&[0, 1, 0, 0, 0, 1, 0, 0, 1, 0, 0, 0, 1, 0, 0, 0], 5121, 4, "VEC4", "");
    let weights = buffer.accessor(&[255, 0, 0, 0, 128, 127, 0, 0, 255, 0, 0, 0, 255, 0, 0, 0], 5121, 4, "VEC4", "\"normalized\":true");
    let indices = buffer.u16s(&[0, 1, 2, 3]);
    let deltas = buffer.floats(&[0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0], "VEC3");
    let sparse_indices = buffer.view(&[1, 0, 3, 0]);
    let sparse_values = buffer.view(&[0u8; 24]);
    let sparse = buffer.raw_accessor(format!("{{\"componentType\":5126,\"count\":4,\"type\":\"VEC3\",\"sparse\":{{\"count\":2,\"indices\":{{\"bufferView\":{sparse_indices},\"componentType\":5123}},\"values\":{{\"bufferView\":{sparse_values}}}}}}}"));
    let matrices = buffer.floats(&[1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0], "MAT4");
    let times = buffer.floats(&[0.0, 1.0], "SCALAR");
    let rotations = buffer.floats(&[0.0, 0.0, 0.0, 2.0, 0.0, 3.0, 0.0, 4.0], "VEC4");
    let weight_keys = buffer.floats(&[0.0, 1.0], "SCALAR");
    let image = buffer.view(&png(1, 1, &[10, 20, 30, 255]));
    buffer.document(&format!(
        "\"meshes\":[{{\"name\":\"quad\",\"weights\":[0.5],\"primitives\":[{{\"mode\":5,\"attributes\":{{\"POSITION\":{positions},\"NORMAL\":{normals},\"TEXCOORD_0\":{uvs},\"JOINTS_0\":{joints},\"WEIGHTS_0\":{weights}}},\"indices\":{indices},\"material\":0,\"targets\":[{{\"POSITION\":{deltas},\"NORMAL\":{sparse}}}]}}]}}],\
         \"nodes\":[{{\"name\":\"root\",\"children\":[1,2],\"matrix\":[2,0,0,0,0,2,0,0,0,0,2,0,1,2,3,1]}},{{\"mesh\":0,\"skin\":0,\"rotation\":[0,0,0,3]}},{{\"translation\":[1,0,0]}}],\
         \"scenes\":[{{\"nodes\":[0]}}],\"scene\":0,\
         \"skins\":[{{\"joints\":[1,2],\"inverseBindMatrices\":{matrices},\"skeleton\":0}}],\
         \"animations\":[{{\"name\":\"spin\",\"channels\":[{{\"sampler\":0,\"target\":{{\"node\":1,\"path\":\"rotation\"}}}},{{\"sampler\":1,\"target\":{{\"node\":1,\"path\":\"weights\"}}}}],\"samplers\":[{{\"input\":{times},\"output\":{rotations},\"interpolation\":\"STEP\"}},{{\"input\":{times},\"output\":{weight_keys}}}]}}],\
         \"materials\":[{{\"name\":\"shiny\",\"alphaMode\":\"BLEND\",\"doubleSided\":true,\"pbrMetallicRoughness\":{{\"baseColorFactor\":[1,0.5,0.25,1],\"baseColorTexture\":{{\"index\":0}},\"metallicFactor\":0.25}},\"normalTexture\":{{\"index\":0,\"scale\":2}},\"emissiveFactor\":[1,1,1],\"extensions\":{{\"KHR_materials_emissive_strength\":{{\"emissiveStrength\":4}},\"KHR_materials_clearcoat\":{{\"clearcoatFactor\":0.5}}}}}}],\
         \"samplers\":[{{\"magFilter\":9728,\"wrapS\":33071}}],\"textures\":[{{\"source\":0,\"sampler\":0}}],\"images\":[{{\"bufferView\":{image},\"mimeType\":\"image/png\"}}]"
    ))
}

#[test]
fn the_rich_document_builds_what_it_describes() {
    let set = build(&rich_document());
    let data = set.data_set();
    let model = &data.models[0];
    let primitive = &model.meshes[0].primitives[0];
    // A strip of 4 indices is 2 triangles.
    assert_eq!((model.vertex_count, model.index_count, primitive.index_count, primitive.skin_base_vertex), (4, 6, 6, Some(0)));
    assert_eq!(model.index_bytes, &[0, 0, 1, 0, 2, 0, 1, 0, 3, 0, 2, 0]);
    // The default morph weight 0.5 is baked into the positions.
    let vertices: Vec<crate::ModelVertex> = model.vertex_bytes.chunks_exact(64).map(bytemuck::pod_read_unaligned).collect();
    assert_eq!(vertices[0].position, glam::Vec3::new(0.0, 0.0, 0.5));
    assert_eq!(primitive.morph_targets[0].position_deltas.len(), 4);
    assert_eq!(primitive.morph_targets[0].normal_deltas, vec![glam::Vec3::ZERO; 4]);
    assert_eq!((primitive.bounds_min.z, primitive.bounds_max.z), (0.5, 0.5));
    assert_eq!(model.nodes[1].default_morph_weights, vec![0.5]);
    // Skin weights of bytes: 128 / 255 and 127 / 255, divided by their sum.
    let skin: Vec<crate::ModelSkinVertex> = model.skin_bytes.chunks_exact(24).map(bytemuck::pod_read_unaligned).collect();
    let (a, b) = (128.0f32 / 255.0, 127.0f32 / 255.0);
    assert_eq!((skin[1].joints, skin[1].weights), ([0, 1, 0, 0], [a / (a + b), b / (a + b), 0.0, 0.0]));
    // The matrix node decomposes; the rotation of node 1 is normalised.
    assert_eq!(model.nodes[0].default_transform.scale, glam::Vec3::splat(2.0));
    assert_eq!(model.nodes[0].default_transform.translation, glam::Vec3::new(1.0, 2.0, 3.0));
    assert!(!model.nodes[0].has_fixed_matrix && model.nodes.iter().all(|n| n.in_scene));
    assert_eq!(model.nodes[1].default_transform.rotation, glam::Quat::IDENTITY);
    assert_eq!((model.nodes[1].parent, model.nodes[0].children.clone(), model.root_nodes.clone()), (Some(0), vec![1, 2], vec![0]));
    assert_eq!((model.skins[0].joints.clone(), model.skins[0].skeleton, model.skins[0].inverse_bind_matrices.len()), (vec![1, 2], Some(0), 2));
    // Animation: rotation keys normalised, STEP kept; weights with one target per key.
    let channels = &model.animations[0].channels;
    assert_eq!((channels[0].interpolation, channels[0].component_count), (crate::AnimationInterpolation::Step, 4));
    assert_eq!(channels[0].values, vec![0.0, 0.0, 0.0, 1.0, 0.0, 0.6, 0.0, 0.8]);
    assert_eq!((channels[1].path, channels[1].component_count, channels[1].values.clone()), (crate::AnimationPath::Weights, 1, vec![0.0, 1.0]));
    // The material, its two slots on one image in two colour spaces, and its sampler.
    let material = &model.materials[0];
    assert_eq!((material.name.as_deref(), material.alpha_mode, material.double_sided), (Some("shiny"), crate::MaterialAlphaMode::Blend, true));
    assert_eq!((material.base_color_factor, material.metallic_factor, material.roughness_factor, material.normal_scale, material.emissive_strength, material.clear_coat_factor), (glam::Vec4::new(1.0, 0.5, 0.25, 1.0), 0.25, 1.0, 2.0, 4.0, 0.5));
    assert_eq!(material.approximated_features, vec!["KHR_materials_clearcoat".to_string()]);
    assert_eq!(set.warnings(0), ["Material 0 'shiny' uses KHR_materials_clearcoat (captured, approximated when drawn).".to_string()]);
    assert_eq!((data.textures.len(), model.textures.clone()), (2, vec![0, 1]));
    assert_eq!((data.textures[0].srgb, data.textures[1].srgb), (true, false));
    assert_eq!((material.base_color_texture.texture, material.normal_texture.texture, material.base_color_texture.sampler), (Some(0), Some(1), Some(0)));
    assert_eq!((model.samplers.len(), model.samplers[0].address_mode_u, model.samplers[0].address_mode_v, model.samplers[0].mag_filter), (1, Some(wgpu::AddressMode::ClampToEdge), Some(wgpu::AddressMode::Repeat), wgpu::FilterMode::Nearest));
    // The file has NORMAL and no normal-map tangents are possible without TANGENT... the normal
    // map has TEXCOORD_0, so tangents are generated.
    assert!(primitive.tangents_generated && primitive.has_tangents && !primitive.normals_generated);
    // And the whole thing reads back through the model-data types the renderer takes.
    assert_eq!(model.default_material.name.as_deref(), Some("default"));
}

/// Every prefix of a file, and the file with each of its numbers replaced by hostile values,
/// must come back as Ok or as an error - the test fails only by a panic, an abort or a hang.
fn must_not_panic(json: &str) {
    let try_it = |bytes: &[u8]| {
        if let Ok(document) = GltfDocument::parse(bytes) {
            let _ = document.external_files(Some("base"), true);
            let mut set = GltfModelSet::new();
            let _ = set.add(&document, "fuzz", Some("base"), &|_| None, &GltfLoadOptions::default());
            let _ = set.data_set();
        }
    };
    let bytes = json.as_bytes();
    for cut in (0..bytes.len()).step_by(7) {
        try_it(&bytes[..cut]);
    }
    // Each number in the JSON, replaced in turn.
    let mut starts = Vec::new();
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index].is_ascii_digit() && (index == 0 || matches!(bytes[index - 1], b':' | b'[' | b',')) {
            let mut end = index;
            while end < bytes.len() && (bytes[end].is_ascii_digit() || bytes[end] == b'.') {
                end += 1;
            }
            starts.push((index, end));
            index = end;
        } else {
            index += 1;
        }
    }
    for (start, end) in starts {
        for hostile in ["0", "1", "7", "255", "65535", "65536", "4294967295", "4294967296", "18446744073709551615", "99999999999999999999999", "-1", "1e30", "0.5"] {
            let mut changed = bytes[..start].to_vec();
            changed.extend_from_slice(hostile.as_bytes());
            changed.extend_from_slice(&bytes[end..]);
            try_it(&changed);
        }
    }
}

#[test]
fn a_damaged_document_is_never_a_panic() {
    must_not_panic(&triangle_document("", ""));
    must_not_panic(&rich_document());
    let (json, _) = textured_triangle(",\"uri\":\"model.bin\"", "{\"uri\":\"colour.png\"}");
    must_not_panic(&json);
}

#[test]
fn a_damaged_glb_is_never_a_panic() {
    let json = rich_document();
    let file = glb(&json, Some(&[0u8; 64]));
    for cut in 0..file.len().min(400) {
        let _ = GltfDocument::parse(&file[..cut]);
    }
    for at in (0..file.len().min(64)).chain((file.len() - 80..file.len()).step_by(3)) {
        for value in [0u8, 1, 0x7F, 0xFF] {
            let mut changed = file.clone();
            changed[at] = value;
            if let Ok(document) = GltfDocument::parse(&changed) {
                let _ = GltfModelSet::new().add(&document, "fuzz", None, &|_| None, &GltfLoadOptions::default());
            }
        }
    }
}

#[test]
fn an_accessor_that_runs_past_its_data_leaves_its_primitive_out_with_a_message() {
    // 3 positions, but the accessor says 30.
    let json = triangle_document("", "").replace("\"count\":3,\"type\":\"VEC3\"", "\"count\":30,\"type\":\"VEC3\"");
    let set = build(&json);
    assert_eq!(set.data_set().models[0].meshes[0].primitives.len(), 0);
    assert!(set.warnings(0)[0].contains("needs 30 elements of 12 bytes") && set.warnings(0)[0].ends_with("it is left out."), "{:?}", set.warnings(0));
    // An index beyond the vertices: the reference's warning.
    let mut buffer = Buffer::default();
    let positions = buffer.floats(&[0.0; 9], "VEC3");
    let indices = buffer.u16s(&[0, 1, 5]);
    let json = buffer.document(&format!("\"meshes\":[{{\"primitives\":[{{\"attributes\":{{\"POSITION\":{positions}}},\"indices\":{indices}}}]}}]"));
    assert_eq!(build(&json).warnings(0), ["Mesh 0 primitive 0 has index 5 for 3 vertices; it is left out.".to_string()]);
    // A POSITION accessor of the wrong shape.
    let json = triangle_document("", "").replace("\"count\":3,\"type\":\"VEC3\"", "\"count\":3,\"type\":\"VEC2\"");
    assert!(build(&json).warnings(0)[0].contains("POSITION accessor with 2 components"));
}
