//! Host tests of the builder's steps on small documents made in memory: primitives (modes,
//! generated normals and tangents, shared vertices), materials and their extensions, textures
//! (the formats refused by name, sharing between models, the size limit), nodes, skins and
//! animation packing. The comparison with the reference converter on real model files is in
//! `reference_tests.rs`.

use std::cell::Cell;
use std::collections::HashMap;

use glam::{Quat, Vec3, Vec4};

use crate::gltf::test_support::{build, build_with, data_uri, png, triangle_document, Buffer};
use crate::gltf::{GltfDocument, GltfLoadOptions, GltfModelSet};
use crate::models::ModelVertex;
use crate::{AnimationInterpolation, MaterialAlphaMode, ModelTextureSlot};

fn vertices(set: &GltfModelSet) -> Vec<ModelVertex> {
    set.data_set().models[0].vertex_bytes.chunks_exact(64).map(bytemuck::pod_read_unaligned).collect()
}

/// A document with one mesh whose primitives are written by the caller from the accessors of a
/// quad: positions 0, normals 1, texture coordinates 2, and index accessors 3 (0 1 2) and 4
/// (2 1 3).
fn quad_document(primitives: &str, rest: &str) -> String {
    let mut buffer = Buffer::default();
    buffer.floats(&[0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 1.0, 1.0, 0.0], "VEC3");
    buffer.floats(&[0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0], "VEC3");
    buffer.floats(&[0.0, 1.0, 1.0, 1.0, 0.0, 0.0, 1.0, 0.0], "VEC2");
    buffer.u16s(&[0, 1, 2]);
    buffer.u16s(&[2, 1, 3]);
    buffer.document(&format!("\"meshes\":[{{\"primitives\":[{primitives}]}}],\"nodes\":[{{\"mesh\":0}}]{rest}"))
}

#[test]
fn a_triangle_without_normals_gets_generated_ones_and_16_bit_indices() {
    let set = build(&triangle_document("", ""));
    let data = set.data_set();
    let model = &data.models[0];
    let primitive = &model.meshes[0].primitives[0];
    assert!(primitive.normals_generated && !primitive.has_tangents && !primitive.has_vertex_colors);
    assert_eq!((primitive.material, primitive.first_index, primitive.base_vertex, primitive.vertex_count), (None, 0, 0, 3));
    assert_eq!((primitive.bounds_min, primitive.bounds_max), (Vec3::ZERO, Vec3::new(1.0, 1.0, 0.0)));
    assert_eq!((model.index_format, model.index_bytes), (wgpu::IndexFormat::Uint16, &[0u8, 0, 1, 0, 2, 0][..]));
    let vertices = vertices(&set);
    assert!(vertices.iter().all(|v| v.normal == Vec3::Z && v.color == [65535; 4] && v.tangent == [0.0; 4]));
}

#[test]
fn points_and_lines_are_left_out_by_name_and_strips_and_fans_become_lists() {
    let json = quad_document(
        "{\"attributes\":{\"POSITION\":0},\"mode\":0},{\"attributes\":{\"POSITION\":0},\"mode\":1},{\"attributes\":{\"POSITION\":0},\"mode\":2},{\"attributes\":{\"POSITION\":0},\"mode\":3},{\"attributes\":{\"POSITION\":0,\"NORMAL\":1},\"mode\":5},{\"attributes\":{\"POSITION\":0,\"NORMAL\":1},\"mode\":6}",
        "",
    );
    let set = build_with(&json, &GltfLoadOptions { share_vertex_data: false, ..GltfLoadOptions::default() });
    assert_eq!(
        set.warnings(0),
        [
            "Mesh 0 primitive 0 draws POINTS; only triangles are supported, so it is left out.".to_string(),
            "Mesh 0 primitive 1 draws LINES; only triangles are supported, so it is left out.".to_string(),
            "Mesh 0 primitive 2 draws LINE_LOOP; only triangles are supported, so it is left out.".to_string(),
            "Mesh 0 primitive 3 draws LINE_STRIP; only triangles are supported, so it is left out.".to_string(),
        ]
    );
    let data = set.data_set();
    let primitives = &data.models[0].meshes[0].primitives;
    assert_eq!(primitives.len(), 2);
    // The loaded primitives are numbered as they stand in the mesh's list.
    assert_eq!((primitives[0].index, primitives[1].index), (0, 1));
    // Without indices the vertices are taken in order: a strip 0 1 2 3, then a fan 0 1 2 3.
    let indices: Vec<u16> = data.models[0].index_bytes.chunks_exact(2).map(|b| u16::from_le_bytes([b[0], b[1]])).collect();
    assert_eq!(indices, vec![0, 1, 2, 1, 3, 2, 1, 2, 0, 2, 3, 0]);
    assert_eq!((primitives[1].first_index, primitives[1].base_vertex), (6, 4));
}

#[test]
fn an_incomplete_triangle_is_dropped_and_a_primitive_without_triangles_is_left_out() {
    let mut buffer = Buffer::default();
    let positions = buffer.floats(&[0.0; 12], "VEC3");
    let four = buffer.u16s(&[0, 1, 2, 3]);
    let two = buffer.u16s(&[0, 1]);
    let json = buffer.document(&format!("\"meshes\":[{{\"primitives\":[{{\"attributes\":{{\"POSITION\":{positions}}},\"indices\":{four}}},{{\"attributes\":{{\"POSITION\":{positions}}},\"indices\":{two}}}]}}]"));
    let set = build(&json);
    assert_eq!(
        set.warnings(0),
        [
            "Mesh 0 primitive 0 has 4 indices (not a multiple of 3); the incomplete triangle is dropped.".to_string(),
            "Mesh 0 primitive 1 has 2 indices (not a multiple of 3); the incomplete triangle is dropped.".to_string(),
            "Mesh 0 primitive 1 has no triangles; it is left out.".to_string(),
        ]
    );
    assert_eq!(set.data_set().models[0].index_count, 3);
}

#[test]
fn primitives_that_name_the_same_accessors_share_their_vertices() {
    let primitives = "{\"attributes\":{\"POSITION\":0,\"NORMAL\":1,\"TEXCOORD_0\":2},\"indices\":3},{\"attributes\":{\"POSITION\":0,\"NORMAL\":1,\"TEXCOORD_0\":2},\"indices\":4}";
    let shared = build(&quad_document(primitives, ""));
    let copied = build_with(&quad_document(primitives, ""), &GltfLoadOptions { share_vertex_data: false, ..GltfLoadOptions::default() });
    let (shared_data, copied_data) = (shared.data_set(), copied.data_set());
    assert_eq!((shared_data.models[0].vertex_count, copied_data.models[0].vertex_count), (4, 8));
    let (a, b) = (&shared_data.models[0].meshes[0].primitives[0], &shared_data.models[0].meshes[0].primitives[1]);
    assert_eq!((a.base_vertex, b.base_vertex, a.vertex_count, b.vertex_count), (0, 0, 4, 4));
    assert_eq!((a.first_index, b.first_index, b.index_count), (0, 3, 3));
    assert_eq!((a.bounds_min, a.bounds_max), (b.bounds_min, b.bounds_max));
    let (c, d) = (&copied_data.models[0].meshes[0].primitives[0], &copied_data.models[0].meshes[0].primitives[1]);
    assert_eq!((c.base_vertex, d.base_vertex), (0, 4));
    // The same indices and, per primitive, the same vertices either way.
    assert_eq!(shared_data.models[0].index_bytes, copied_data.models[0].index_bytes);
    assert_eq!(&copied_data.models[0].vertex_bytes[..256], shared_data.models[0].vertex_bytes);
    assert_eq!(&copied_data.models[0].vertex_bytes[256..], shared_data.models[0].vertex_bytes);
    assert!(shared.byte_size() < copied.byte_size());
}

#[test]
fn primitives_whose_normals_are_generated_never_share() {
    // No NORMAL: each primitive's normals come from its own triangles.
    let primitives = "{\"attributes\":{\"POSITION\":0},\"indices\":3},{\"attributes\":{\"POSITION\":0},\"indices\":4}";
    let set = build(&quad_document(primitives, ""));
    assert_eq!(set.data_set().models[0].vertex_count, 8);
    // Different attribute sets do not share either.
    let primitives = "{\"attributes\":{\"POSITION\":0,\"NORMAL\":1},\"indices\":3},{\"attributes\":{\"POSITION\":0,\"NORMAL\":1,\"TEXCOORD_0\":2},\"indices\":4}";
    assert_eq!(build(&quad_document(primitives, "")).data_set().models[0].vertex_count, 8);
}

#[test]
fn tangents_are_generated_for_a_normal_map_and_only_then() {
    let image = data_uri(&png(1, 1, &[128, 128, 255, 255]));
    let rest = format!(",\"materials\":[{{\"normalTexture\":{{\"index\":0}}}},{{\"normalTexture\":{{\"index\":0,\"texCoord\":1}}}}],\"textures\":[{{\"source\":0}}],\"images\":[{{\"uri\":\"{image}\"}}]");
    let primitives = "{\"attributes\":{\"POSITION\":0,\"NORMAL\":1,\"TEXCOORD_0\":2},\"indices\":3,\"material\":0},{\"attributes\":{\"POSITION\":0,\"NORMAL\":1,\"TEXCOORD_0\":2},\"indices\":3},{\"attributes\":{\"POSITION\":0,\"NORMAL\":1,\"TEXCOORD_0\":2},\"indices\":3,\"material\":1}";
    let set = build(&quad_document(primitives, &rest));
    let data = set.data_set();
    let primitives_built = &data.models[0].meshes[0].primitives;
    assert_eq!((primitives_built[0].tangents_generated, primitives_built[1].tangents_generated, primitives_built[2].tangents_generated), (true, false, false));
    assert_eq!(set.warnings(0), ["Mesh 0 primitive 2 has a normal map but no TEXCOORD_1; no tangents were generated.".to_string()]);
    // U along +X with V growing downwards: tangent +X, handedness +1.
    assert_eq!(vertices(&set)[0].tangent, [1.0, 0.0, 0.0, 1.0]);
    assert!(!data.textures[0].srgb);
    // With the option off nothing is generated and nothing is said.
    let off = build_with(&quad_document(primitives, &rest), &GltfLoadOptions { generate_tangents: false, ..GltfLoadOptions::default() });
    assert!(off.warnings(0).is_empty() && off.data_set().models[0].meshes[0].primitives.iter().all(|p| !p.tangents_generated));
    // With textures off the slot still names its texture, so tangents are still generated.
    let untextured = build_with(&quad_document(primitives, &rest), &GltfLoadOptions { load_textures: false, ..GltfLoadOptions::default() });
    let untextured_data = untextured.data_set();
    assert!(untextured_data.models[0].meshes[0].primitives[0].tangents_generated);
    let slot = untextured_data.models[0].materials[0].normal_texture;
    assert_eq!((slot.texture, slot.sampler, slot.texture_index), (None, None, 0));
}

#[test]
fn vertex_colours_and_a_second_texture_coordinate_set_are_read() {
    let mut buffer = Buffer::default();
    let positions = buffer.floats(&[0.0; 9], "VEC3");
    let colours = buffer.accessor(&[255, 0, 0, 0, 255, 0, 0, 0, 255], 5121, 3, "VEC3", "\"normalized\":true");
    let uv1 = buffer.accessor(&[0, 0, 255, 255, 0, 128, 0, 0, 0, 0, 0, 0], 5123, 3, "VEC2", "\"normalized\":true");
    let json = buffer.document(&format!("\"meshes\":[{{\"primitives\":[{{\"attributes\":{{\"POSITION\":{positions},\"COLOR_0\":{colours},\"TEXCOORD_1\":{uv1}}}}}]}}]"));
    let set = build(&json);
    let data = set.data_set();
    let primitive = &data.models[0].meshes[0].primitives[0];
    assert!(primitive.has_vertex_colors && primitive.has_tex_coord1);
    let vertices = vertices(&set);
    // A VEC3 colour has alpha 1.
    assert_eq!((vertices[0].color, vertices[1].color), ([65535, 0, 0, 65535], [0, 65535, 0, 65535]));
    assert_eq!(vertices[0].tex_coord1, glam::Vec2::new(0.0, 1.0));
    assert_eq!(vertices[1].tex_coord1, glam::Vec2::new(32768.0 / 65535.0, 0.0));
}

#[test]
fn a_material_s_core_values_and_extensions_are_captured() {
    let materials = "\"materials\":[\
        {\"name\":\"all\",\"alphaMode\":\"MASK\",\"alphaCutoff\":0.25,\"emissiveFactor\":[0.5,0.25,0.125],\"occlusionTexture\":{\"index\":0,\"strength\":0.5},\
         \"extensions\":{\"KHR_materials_unlit\":{},\"KHR_materials_ior\":{\"ior\":1.33},\"KHR_materials_specular\":{\"specularFactor\":0.5,\"specularColorFactor\":[1,0,0]},\"KHR_materials_transmission\":{\"transmissionFactor\":0.75},\
         \"KHR_materials_clearcoat\":{\"clearcoatFactor\":1,\"clearcoatRoughnessFactor\":0.125},\"KHR_materials_sheen\":{},\"KHR_materials_volume\":{},\"KHR_materials_iridescence\":{},\"KHR_materials_anisotropy\":{},\"KHR_materials_diffuse_transmission\":{}}},\
        {},\
        {\"extensions\":{\"KHR_materials_pbrSpecularGlossiness\":{\"diffuseFactor\":[0.5,0.5,0.5,1]}}},\
        {\"pbrMetallicRoughness\":{\"metallicFactor\":0.5},\"extensions\":{\"KHR_materials_pbrSpecularGlossiness\":{\"diffuseFactor\":[0,0,0,1]},\"KHR_materials_ior\":{}}}]";
    let json = triangle_document("", &format!(",{materials},\"textures\":[{{}}]"));
    let set = build_with(&json, &GltfLoadOptions { load_textures: false, ..GltfLoadOptions::default() });
    let data = set.data_set();
    let materials = &data.models[0].materials;
    let all = &materials[0];
    assert_eq!((all.alpha_mode, all.alpha_cutoff, all.unlit, all.double_sided), (MaterialAlphaMode::Mask, 0.25, true, false));
    assert_eq!((all.emissive_factor, all.occlusion_strength, all.index_of_refraction), (Vec3::new(0.5, 0.25, 0.125), 0.5, 1.33));
    assert_eq!((all.specular_factor, all.specular_color, all.transmission_factor, all.clear_coat_factor, all.clear_coat_roughness), (0.5, Vec3::X, 0.75, 1.0, 0.125));
    // The reference's order of the approximated features, and its warning.
    let expected = ["KHR_materials_ior", "KHR_materials_clearcoat", "KHR_materials_specular", "KHR_materials_transmission", "KHR_materials_sheen", "KHR_materials_volume", "KHR_materials_iridescence", "KHR_materials_anisotropy", "KHR_materials_diffuse_transmission"];
    assert_eq!(all.approximated_features, expected.map(str::to_string));
    assert_eq!(set.warnings(0)[0], format!("Material 0 'all' uses {} (captured, approximated when drawn).", expected.join(", ")));
    // An empty material has glTF's defaults and no warning.
    assert_eq!(materials[1], crate::ModelMaterial::new(1, None));
    // Specular-glossiness WITHOUT pbrMetallicRoughness: the diffuse colour, metallic 0.
    assert_eq!((materials[2].base_color_factor, materials[2].metallic_factor, materials[2].approximated_features.clone()), (Vec4::new(0.5, 0.5, 0.5, 1.0), 0.0, vec!["KHR_materials_pbrSpecularGlossiness".to_string()]));
    // WITH pbrMetallicRoughness the core values win and nothing is approximated; an ior
    // extension without a value is the default 1.5.
    assert_eq!((materials[3].base_color_factor, materials[3].metallic_factor, materials[3].index_of_refraction), (Vec4::ONE, 0.5, 1.5));
    assert!(materials[3].approximated_features.is_empty());
    assert_eq!(set.warnings(0).len(), 2);
    assert_eq!(data.models[0].default_material, crate::ModelMaterial::new(-1, Some("default".to_string())));
}

#[test]
fn a_texture_transform_and_its_texture_coordinate_override_are_applied_to_the_slot() {
    let image = data_uri(&png(1, 1, &[1, 2, 3, 4]));
    let transform = "\"extensions\":{\"KHR_texture_transform\":{\"offset\":[0.5,0.25],\"scale\":[2,3],\"texCoord\":1}}";
    let materials = format!(
        "\"materials\":[{{\"pbrMetallicRoughness\":{{\"baseColorTexture\":{{\"index\":0,{transform}}},\"metallicRoughnessTexture\":{{\"index\":0,\"texCoord\":2}}}},\"normalTexture\":{{\"index\":0,{transform}}},\"occlusionTexture\":{{\"index\":0,\"texCoord\":1}},\"emissiveTexture\":{{\"index\":0,\"extensions\":{{\"KHR_texture_transform\":{{\"texCoord\":5}}}}}}}}],\"textures\":[{{\"source\":0}}],\"images\":[{{\"uri\":\"{image}\"}}]"
    );
    let set = build(&triangle_document("", &format!(",{materials}")));
    let data = set.data_set();
    let material = &data.models[0].materials[0];
    let expected = [2.0, 0.0, 0.0, 3.0, 0.5, 0.25];
    assert_eq!((material.base_color_texture.transform, material.base_color_texture.has_transform, material.base_color_texture.tex_coord), (expected, true, 1));
    assert_eq!((material.normal_texture.transform, material.normal_texture.has_transform, material.normal_texture.tex_coord), (expected, true, 1));
    assert_eq!((material.occlusion_texture.transform, material.occlusion_texture.has_transform, material.occlusion_texture.tex_coord), (ModelTextureSlot::IDENTITY_TRANSFORM, false, 1));
    // TEXCOORD_2 and beyond sample set 0, with the reference's warning.
    assert_eq!((material.metallic_roughness_texture.tex_coord, material.emissive_texture.tex_coord, material.emissive_texture.has_transform), (0, 0, true));
    assert_eq!(set.warnings(0), ["A texture uses TEXCOORD_2; only TEXCOORD_0 and TEXCOORD_1 are loaded, so it samples TEXCOORD_0.".to_string(), "A texture uses TEXCOORD_5; only TEXCOORD_0 and TEXCOORD_1 are loaded, so it samples TEXCOORD_0.".to_string()]);
    // One image, used as colour (base colour, emissive) and as data: two textures, one sampler.
    assert_eq!((data.textures.len(), data.textures[0].srgb, data.textures[1].srgb, data.models[0].samplers.len()), (2, true, false, 1));
    assert_eq!((material.base_color_texture.texture, material.emissive_texture.texture, material.normal_texture.texture, material.occlusion_texture.texture), (Some(0), Some(0), Some(1), Some(1)));
    // Colour textures marked linear on request: then it is one texture.
    let linear = build_with(&triangle_document("", &format!(",{materials}")), &GltfLoadOptions { color_textures_srgb: false, mipmaps: false, ..GltfLoadOptions::default() });
    assert_eq!((linear.data_set().textures.len(), linear.data_set().textures[0].srgb, linear.data_set().textures[0].mipmaps), (1, false, false));
}

/// A textured triangle whose one texture is written by the caller.
fn texture_document(texture: &str, images: &str) -> String {
    triangle_document(",\"material\":0", &format!(",\"materials\":[{{\"name\":\"m\",\"pbrMetallicRoughness\":{{\"baseColorTexture\":{{\"index\":0}}}}}}],\"textures\":[{texture}],\"images\":[{images}]"))
}

#[test]
fn ktx2_webp_and_other_formats_are_refused_by_name_and_a_png_fallback_is_used() {
    let ktx2 = data_uri(&[0xAB, b'K', b'T', b'X', b' ', b'2', b'0', 0xBB, 0x0D, 0x0A, 0x1A, 0x0A, 0, 0, 0, 0]);
    let webp = data_uri(b"RIFF\x10\x00\x00\x00WEBPVP8 ");
    let gif = data_uri(b"GIF89a\x01\x00\x01\x00");
    let good = data_uri(&png(1, 1, &[9, 9, 9, 255]));
    let refused = |texture: &str, images: &str| -> (usize, Vec<String>) {
        let set = build(&texture_document(texture, images));
        assert_eq!(set.data_set().models[0].materials[0].base_color_texture.texture, None);
        // The slot still says which glTF texture it names.
        assert_eq!(set.data_set().models[0].materials[0].base_color_texture.texture_index, 0);
        (set.texture_count(), set.warnings(0).to_vec())
    };
    let (count, warnings) = refused("{\"extensions\":{\"KHR_texture_basisu\":{\"source\":0}}}", &format!("{{\"uri\":\"{ktx2}\",\"name\":\"compressed\"}}"));
    assert_eq!((count, warnings), (0, vec!["Texture 0 uses image 0 'compressed', a KTX2 (Basis Universal) image; this loader reads PNG and JPEG images only, so the texture is left out. Convert the image to PNG or JPEG.".to_string()]));
    let (_, warnings) = refused("{\"extensions\":{\"EXT_texture_webp\":{\"source\":0}}}", &format!("{{\"uri\":\"{webp}\"}}"));
    assert!(warnings[0].contains("a WebP image") && warnings[0].contains("PNG and JPEG images only"), "{warnings:?}");
    let (_, warnings) = refused("{\"source\":0}", &format!("{{\"uri\":\"{gif}\"}}"));
    assert!(warnings[0].contains("neither PNG nor JPEG"), "{warnings:?}");
    let (_, warnings) = refused("{}", "{\"uri\":\"data:image/png;base64,\"}");
    assert_eq!(warnings, vec!["Texture 0 has no image.".to_string()]);
    // A KTX2 image with a PNG fallback: the PNG is used and nothing is said.
    let set = build(&texture_document("{\"source\":1,\"extensions\":{\"KHR_texture_basisu\":{\"source\":0}}}", &format!("{{\"uri\":\"{ktx2}\"}},{{\"uri\":\"{good}\"}}")));
    assert!(set.warnings(0).is_empty());
    assert_eq!(set.data_set().textures[0].pixels, &[9, 9, 9, 255]);
    // A PNG that is damaged: the decoder's message, and the texture is left out.
    let mut broken = png(2, 2, &[7; 16]);
    broken.truncate(40);
    let (_, warnings) = refused("{\"source\":0}", &format!("{{\"uri\":\"{}\"}}", data_uri(&broken)));
    assert!(warnings[0].starts_with("The image image 0 could not be decoded: ") && warnings[0].ends_with("The texture is left out."), "{warnings:?}");
}

#[test]
fn an_image_several_models_name_is_decoded_once_per_set_and_asked_for_once() {
    let json = texture_document("{\"source\":0}", "{\"uri\":\"../Textures/shared.png\"}");
    let document = GltfDocument::parse(json.as_bytes()).unwrap();
    let files: HashMap<String, Vec<u8>> = HashMap::from([("kit/Textures/shared.png".to_string(), png(2, 2, &[5; 16]))]);
    let asked = Cell::new(0);
    let resolve = |address: &str| {
        asked.set(asked.get() + 1);
        files.get(address).map(Vec::as_slice)
    };
    let mut set = GltfModelSet::new();
    // Two models in sibling folders: "../Textures/shared.png" is one address.
    set.add(&document, "first", Some("kit/trees"), &resolve, &GltfLoadOptions::default()).unwrap();
    set.add(&document, "second", Some("kit/rocks"), &resolve, &GltfLoadOptions::default()).unwrap();
    assert_eq!((set.model_count(), set.texture_count(), asked.get()), (2, 1, 1));
    let data = set.data_set();
    assert_eq!((data.models[0].textures.clone(), data.models[1].textures.clone()), (vec![0], vec![0]));
    assert_eq!(data.models[1].materials[0].base_color_texture.texture, Some(0));
    // Another colour space or mipmap choice is another texture; no base address means no sharing.
    set.add(&document, "third", Some("kit/rocks"), &resolve, &GltfLoadOptions { color_textures_srgb: false, ..GltfLoadOptions::default() }).unwrap();
    assert_eq!(set.texture_count(), 2);
    let unshared: HashMap<String, Vec<u8>> = HashMap::from([("../Textures/shared.png".to_string(), png(2, 2, &[5; 16]))]);
    let mut alone = GltfModelSet::new();
    alone.add(&document, "a", None, &|address| unshared.get(address).map(Vec::as_slice), &GltfLoadOptions::default()).unwrap();
    alone.add(&document, "b", None, &|address| unshared.get(address).map(Vec::as_slice), &GltfLoadOptions::default()).unwrap();
    assert_eq!(alone.texture_count(), 2);
    assert_eq!(alone.data_set().textures[0].name.as_deref(), Some("a '../Textures/shared.png'"));
}

#[test]
fn a_texture_larger_than_the_limit_is_halved_until_it_fits() {
    let json = texture_document("{\"source\":0}", &format!("{{\"uri\":\"{}\"}}", data_uri(&png(8, 4, &[200; 128]))));
    let set = build_with(&json, &GltfLoadOptions { max_texture_size: 3, ..GltfLoadOptions::default() });
    let data = set.data_set();
    assert_eq!((data.textures[0].width, data.textures[0].height, data.textures[0].pixels.len()), (2, 1, 8));
    assert!(data.textures[0].pixels.iter().all(|p| *p == 200));
    assert_eq!(build(&json).data_set().textures[0].width, 8);
}

#[test]
fn nodes_take_their_transform_from_trs_or_from_a_matrix() {
    let nodes = "\"nodes\":[{\"name\":\"trs\",\"translation\":[1,2,3],\"rotation\":[0,0,0,2],\"scale\":[2,2,2],\"children\":[1]},{\"matrix\":[1,0,0,0,1,1,0,0,0,0,1,0,5,6,7,1]},{\"rotation\":[0,0,0,0]},{}],\"scenes\":[{\"nodes\":[3]},{\"nodes\":[0]}],\"scene\":1";
    let set = build(&format!("{{\"asset\":{{\"version\":\"2.0\"}},{nodes}}}"));
    let data = set.data_set();
    let nodes = &data.models[0].nodes;
    assert_eq!((nodes[0].name.as_deref(), nodes[0].default_transform.translation, nodes[0].default_transform.rotation, nodes[0].default_transform.scale), (Some("trs"), Vec3::new(1.0, 2.0, 3.0), Quat::IDENTITY, Vec3::splat(2.0)));
    assert_eq!(nodes[0].local_matrix.to_cols_array(), [2.0, 0.0, 0.0, 0.0, 0.0, 2.0, 0.0, 0.0, 0.0, 0.0, 2.0, 0.0, 1.0, 2.0, 3.0, 1.0]);
    // A sheared matrix does not decompose: it is kept as it is, and the transform is the identity.
    assert!(nodes[1].has_fixed_matrix);
    assert_eq!(nodes[1].local_matrix.to_cols_array(), [1.0, 0.0, 0.0, 0.0, 1.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 5.0, 6.0, 7.0, 1.0]);
    assert_eq!(nodes[1].default_transform, crate::NodeTransform::IDENTITY);
    // A rotation of no length becomes the identity instead of a division by zero.
    assert_eq!(nodes[2].default_transform.rotation, Quat::IDENTITY);
    // The named scene decides the roots and what is in the scene.
    assert_eq!(data.models[0].root_nodes, vec![0]);
    assert_eq!(nodes.iter().map(|n| n.in_scene).collect::<Vec<_>>(), vec![true, true, false, false]);
    assert_eq!((nodes[1].parent, nodes[0].parent), (Some(0), None));
}

#[test]
fn morph_weights_come_from_the_node_or_its_mesh() {
    let mut buffer = Buffer::default();
    let positions = buffer.floats(&[0.0; 9], "VEC3");
    let up = buffer.floats(&[0.0, 1.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 0.0], "VEC3");
    let json = buffer.document(&format!(
        "\"meshes\":[{{\"weights\":[0.25],\"primitives\":[{{\"attributes\":{{\"POSITION\":{positions}}},\"targets\":[{{\"POSITION\":{up}}},{{\"POSITION\":{up}}}]}}]}}],\"nodes\":[{{\"mesh\":0}},{{\"mesh\":0,\"weights\":[1,0.5,0.75]}},{{}}]"
    ));
    let set = build(&json);
    let data = set.data_set();
    let model = &data.models[0];
    // Two targets, one weight in the file: the second is 0.
    assert_eq!(model.meshes[0].default_morph_weights, vec![0.25, 0.0]);
    assert_eq!(model.nodes[0].default_morph_weights, vec![0.25, 0.0]);
    // The node's own weights, cut to the target count.
    assert_eq!(model.nodes[1].default_morph_weights, vec![1.0, 0.5]);
    assert!(model.nodes[2].default_morph_weights.is_empty());
    assert_eq!(vertices(&set)[0].position, Vec3::new(0.0, 0.25, 0.0));
}

#[test]
fn a_skin_without_matrices_has_identity_ones_and_weights_are_divided_by_their_sum() {
    let mut buffer = Buffer::default();
    let positions = buffer.floats(&[0.0; 9], "VEC3");
    let joints = buffer.accessor(&[2, 0, 1, 0, 0, 0, 0, 0, 3, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0], 5123, 3, "VEC4", "");
    let weights = buffer.floats(&[2.0, 2.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.5, 0.25, 0.25, 0.0], "VEC4");
    let json = buffer.document(&format!("\"meshes\":[{{\"primitives\":[{{\"attributes\":{{\"POSITION\":{positions},\"JOINTS_0\":{joints},\"WEIGHTS_0\":{weights}}}}}]}}],\"nodes\":[{{\"mesh\":0,\"skin\":0}},{{}}],\"skins\":[{{\"name\":\"s\",\"joints\":[1,0]}}]"));
    let set = build(&json);
    let data = set.data_set();
    assert_eq!((data.models[0].skins[0].name.as_deref(), data.models[0].skins[0].joints.clone(), data.models[0].skins[0].skeleton), (Some("s"), vec![1, 0], None));
    assert_eq!(data.models[0].skins[0].inverse_bind_matrices, vec![glam::Mat4::IDENTITY; 2]);
    let skin: Vec<crate::ModelSkinVertex> = data.models[0].skin_bytes.chunks_exact(24).map(bytemuck::pod_read_unaligned).collect();
    assert_eq!((skin[0].joints, skin[0].weights), ([2, 1, 0, 0], [0.5, 0.5, 0.0, 0.0]));
    // Weights that add up to nothing: all on the first joint.
    assert_eq!(skin[1].weights, [1.0, 0.0, 0.0, 0.0]);
    assert_eq!(skin[2].weights, [0.5, 0.25, 0.25, 0.0]);
    assert_eq!(data.models[0].nodes[0].skin, Some(0));
}

#[test]
fn animation_keys_are_packed_per_interpolation() {
    let mut buffer = Buffer::default();
    let times = buffer.floats(&[0.0, 0.5, 1.0], "SCALAR");
    let linear = buffer.floats(&[0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 2.0, 0.0, 0.0], "VEC3");
    // Cubic: in-tangent, value, out-tangent per key; only the VALUE of a rotation is normalised.
    let two_times = buffer.floats(&[0.0, 1.0], "SCALAR");
    let cubic = buffer.floats(&[9.0, 9.0, 9.0, 9.0, 0.0, 0.0, 0.0, 2.0, 8.0, 8.0, 8.0, 8.0, 7.0, 7.0, 7.0, 7.0, 0.0, 0.0, 3.0, 4.0, 6.0, 6.0, 6.0, 6.0], "VEC4");
    let short = buffer.floats(&[1.0, 1.0, 1.0], "VEC3");
    let wrong = buffer.floats(&[1.0, 1.0, 1.0, 1.0], "VEC4");
    let channel = |sampler: usize, path: &str| format!("{{\"sampler\":{sampler},\"target\":{{\"node\":0,\"path\":\"{path}\"}}}}");
    let json = buffer.document(&format!(
        "\"nodes\":[{{}}],\"animations\":[{{\"name\":\"clip\",\"channels\":[{},{},{},{}],\"samplers\":[{{\"input\":{times},\"output\":{linear}}},{{\"input\":{two_times},\"output\":{cubic},\"interpolation\":\"CUBICSPLINE\"}},{{\"input\":{times},\"output\":{short}}},{{\"input\":{times},\"output\":{wrong}}}]}}]",
        channel(0, "translation"),
        channel(1, "rotation"),
        channel(2, "scale"),
        channel(3, "scale")
    ));
    let set = build(&json);
    let data = set.data_set();
    let channels = &data.models[0].animations[0].channels;
    assert_eq!(channels.len(), 3);
    assert_eq!((channels[0].interpolation, channels[0].times.clone(), channels[0].values.len(), channels[0].component_count), (AnimationInterpolation::Linear, vec![0.0, 0.5, 1.0], 9, 3));
    assert_eq!(channels[1].interpolation, AnimationInterpolation::CubicSpline);
    assert_eq!(channels[1].values, vec![9.0, 9.0, 9.0, 9.0, 0.0, 0.0, 0.0, 1.0, 8.0, 8.0, 8.0, 8.0, 7.0, 7.0, 7.0, 7.0, 0.0, 0.0, 0.6, 0.8, 6.0, 6.0, 6.0, 6.0]);
    // Fewer values than times: as many keys as have a value (the reference pairs them the same way).
    assert_eq!((channels[2].times.clone(), channels[2].values.clone()), (vec![0.0], vec![1.0, 1.0, 1.0]));
    // A scale channel with a VEC4 output is left out with a message.
    assert_eq!(set.warnings(0).len(), 1);
    assert!(set.warnings(0)[0].starts_with("Animation 0 'clip' channel 3 has an input with 1 components per key and an output with 4"), "{:?}", set.warnings(0));
    assert_eq!(data.models[0].animations[0].duration(), 1.0);
}

#[test]
fn a_later_upload_takes_the_textures_that_are_on_the_gpu_already_and_uploads_only_the_rest() {
    // Two models: the first uses "shared.png"; the second uses it too (base colour) and an image
    // of its own (emissive), with a sampler of its own on the shared texture's slot.
    let first = texture_document("{\"source\":0}", "{\"uri\":\"shared.png\"}");
    let second = triangle_document(
        ",\"material\":0",
        ",\"materials\":[{\"pbrMetallicRoughness\":{\"baseColorTexture\":{\"index\":0}},\"emissiveTexture\":{\"index\":1}}],\"samplers\":[{\"wrapS\":33071}],\"textures\":[{\"source\":0,\"sampler\":0},{\"source\":1}],\"images\":[{\"uri\":\"shared.png\"},{\"uri\":\"own.png\"}]",
    );
    let files: HashMap<String, Vec<u8>> = HashMap::from([("kit/shared.png".to_string(), png(1, 1, &[1, 1, 1, 255])), ("kit/own.png".to_string(), png(1, 1, &[2, 2, 2, 255]))]);
    let resolve = |address: &str| files.get(address).map(Vec::as_slice);
    let mut set = GltfModelSet::new();
    set.add(&GltfDocument::parse(first.as_bytes()).unwrap(), "first", Some("kit"), &resolve, &GltfLoadOptions::default()).unwrap();
    set.add(&GltfDocument::parse(second.as_bytes()).unwrap(), "second", Some("kit"), &resolve, &GltfLoadOptions::default()).unwrap();
    assert_eq!(set.texture_count(), 2);

    // Nothing on the GPU yet: the plan is the whole set, and so is data_set().
    let plan = set.upload_plan(&|_| false);
    assert_eq!((plan.view.models.len(), plan.view.textures.len(), plan.view_textures.clone(), plan.reused.len()), (2, 2, vec![0, 1], 0));
    assert_eq!(plan.view, set.data_set());
    assert_eq!(plan.view.models[1].materials[0].base_color_texture.texture, Some(0));

    // The shared texture is on the GPU (an earlier upload): only "own.png" is left to upload, the
    // second model's base colour slot is put right after creation, with its own sampler.
    let plan = set.upload_plan(&|texture| texture == 0);
    assert_eq!((plan.view.textures.len(), plan.view_textures.clone()), (1, vec![1]));
    assert_eq!(plan.view.textures[0].name.as_deref(), Some("kit/own.png"));
    assert!(plan.view.models[0].textures.is_empty() && plan.view.models[0].materials[0].base_color_texture.texture.is_none());
    assert_eq!(plan.view.models[1].textures, vec![0]);
    assert_eq!((plan.view.models[1].materials[0].base_color_texture.texture, plan.view.models[1].materials[0].emissive_texture.texture), (None, Some(0)));
    assert_eq!(plan.reused, vec![(0, Some(0), crate::ModelTextureKind::BaseColor, 0, Some(0)), (1, Some(0), crate::ModelTextureKind::BaseColor, 0, Some(0))]);
    // The slot keeps what it says about the glTF texture, its coordinates and its transform.
    assert_eq!(plan.view.models[1].materials[0].base_color_texture.texture_index, 0);
    assert!(set.uploaded_texture("kit/shared.png", true, true).is_none());
    assert!(format!("{set:?}").contains("\"first\", \"second\""));
}
