//! Host tests of the reader on the blobs the .NET writer made (fixtures/, which the .NET test
//! suite regenerates and compares byte for byte, so the writer and this reader cannot drift).
//! The values asserted here are the ones the .NET ModelDataFixtures describes.

use glam::{Mat4, Quat, Vec3, Vec4};

use crate::drawing3d::MaterialAlphaMode;
use crate::models::model_layout::{morph_center_shifts, rest_matrices, ModelDrawItem, ModelLayout};
use crate::models::{AnimationInterpolation, AnimationPath, ModelDataSet, ModelVertex};

const TRIANGLE: &[u8] = include_bytes!("../../fixtures/triangle.cbmd");
const EVERY_SECTION: &[u8] = include_bytes!("../../fixtures/every-section.cbmd");
const TWO_MODELS: &[u8] = include_bytes!("../../fixtures/two-models-shared-texture.cbmd");

fn vertices(bytes: &[u8]) -> Vec<ModelVertex> {
    bytes.chunks_exact(64).map(bytemuck::pod_read_unaligned).collect()
}

#[test]
fn the_triangle_is_read_as_the_writer_wrote_it() {
    let set = ModelDataSet::read(TRIANGLE).unwrap();
    assert_eq!(TRIANGLE.len(), 1792);
    assert_eq!(set.minor_version, 0);
    assert!(set.textures.is_empty());
    let model = &set.models[0];
    assert_eq!(set.models.len(), 1);
    assert_eq!(model.name, "triangle");
    assert_eq!((model.vertex_count, model.index_count, model.index_format), (3, 3, wgpu::IndexFormat::Uint16));
    assert_eq!(model.index_bytes, &[0, 0, 1, 0, 2, 0]);
    let v = vertices(model.vertex_bytes);
    assert_eq!(v[1].position, Vec3::X);
    assert_eq!(v[2].tex_coord0, glam::Vec2::new(0.0, 1.0));
    assert_eq!(v[0].normal, Vec3::Z);
    assert_eq!(v[0].color, [65535; 4]);
    let primitive = &model.meshes[0].primitives[0];
    assert_eq!(model.meshes[0].name.as_deref(), Some("triangle"));
    assert_eq!((primitive.material, primitive.first_index, primitive.index_count, primitive.base_vertex, primitive.vertex_count, primitive.skin_base_vertex), (Some(0), 0, 3, 0, 3, None));
    assert_eq!(primitive.bounds_max, Vec3::new(1.0, 1.0, 0.0));
    let red = &model.materials[0];
    assert_eq!((red.name.as_deref(), red.index, red.base_color_factor, red.metallic_factor, red.roughness_factor), (Some("red"), 0, Vec4::new(1.0, 0.0, 0.0, 1.0), 0.0, 0.5));
    assert!(!red.base_color_texture.is_present() && red.base_color_texture.texture.is_none());
    assert_eq!((red.index_of_refraction, red.specular_color, red.alpha_mode), (1.5, Vec3::ONE, MaterialAlphaMode::Opaque));
    assert_eq!((model.default_material.index, model.default_material.name.as_deref()), (-1, Some("default")));
    let node = &model.nodes[0];
    assert_eq!((node.name.as_deref(), node.parent, node.mesh, node.skin, node.in_scene, node.has_fixed_matrix), (Some("triangle"), None, Some(0), None, true, false));
    assert_eq!(node.local_matrix, Mat4::IDENTITY);
    assert_eq!(model.root_nodes, vec![0]);
    assert!(model.skins.is_empty() && model.animations.is_empty() && model.warnings.is_empty() && model.skin_bytes.is_empty());
}

#[test]
fn every_section_is_read() {
    let set = ModelDataSet::read(EVERY_SECTION).unwrap();
    let texture = &set.textures[0];
    assert_eq!((texture.name.as_deref(), texture.width, texture.height, texture.srgb, texture.mipmaps), (Some("checker"), 2, 2, true, true));
    assert_eq!(texture.pixels, &[255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255, 255, 255, 255, 255]);
    let model = &set.models[0];
    assert_eq!(model.textures, vec![0]);
    let sampler = &model.samplers[0];
    assert_eq!((sampler.address_mode_u, sampler.address_mode_v, sampler.mag_filter, sampler.min_filter, sampler.mipmap_filter, sampler.max_lod), (Some(wgpu::AddressMode::ClampToEdge), Some(wgpu::AddressMode::MirrorRepeat), wgpu::FilterMode::Nearest, wgpu::FilterMode::Linear, wgpu::MipmapFilterMode::Nearest, 0.0));
    assert_eq!(model.warnings, vec!["points primitive skipped".to_string(), "image 1 could not be decoded".to_string()]);
    assert_eq!((model.vertex_count, model.index_count, model.index_format), (7, 9, wgpu::IndexFormat::Uint32));
    let v = vertices(model.vertex_bytes);
    assert_eq!((v[0].position, v[0].tangent, v[0].color), (Vec3::new(-1.0, -1.0, 0.0), [1.0, 0.0, 0.0, 1.0], [65535, 32768, 0, 65535]));
    assert_eq!(v[5].tex_coord1, glam::Vec2::splat(0.5));
    let indices: Vec<u32> = model.index_bytes.chunks_exact(4).map(|c| u32::from_le_bytes([c[0], c[1], c[2], c[3]])).collect();
    assert_eq!(indices, vec![0, 1, 2, 0, 2, 3, 0, 1, 2]);

    let quad = &model.meshes[0].primitives[0];
    assert_eq!((quad.material, quad.index_count, quad.vertex_count, quad.has_vertex_colors, quad.has_tangents, quad.tangents_generated, quad.has_tex_coord1), (Some(0), 6, 4, true, true, true, false));
    let skinned = &model.meshes[1].primitives[0];
    assert_eq!((skinned.material, skinned.first_index, skinned.base_vertex, skinned.skin_base_vertex, skinned.has_tex_coord1, skinned.normals_generated), (None, 6, 4, Some(0), true, true));
    assert_eq!(skinned.bounds_max, Vec3::new(1.0, 1.0, 0.5));
    assert_eq!(skinned.morph_targets[0].position_deltas, vec![Vec3::new(0.0, 0.0, 0.5); 3]);
    assert!(skinned.morph_targets[0].normal_deltas.is_empty() && skinned.morph_targets[0].tangent_deltas.is_empty());
    assert_eq!(model.meshes[1].default_morph_weights, vec![0.25]);

    let skin_vertices: Vec<crate::models::ModelSkinVertex> = model.skin_bytes.chunks_exact(24).map(bytemuck::pod_read_unaligned).collect();
    assert_eq!(skin_vertices.len(), 3);
    assert_eq!((skin_vertices[1].joints, skin_vertices[1].weights), ([0, 1, 0, 0], [0.5, 0.5, 0.0, 0.0]));

    let textured = &model.materials[0];
    let slot = textured.base_color_texture;
    assert_eq!((slot.texture, slot.sampler, slot.tex_coord, slot.has_transform, slot.texture_index), (Some(0), Some(0), 0, true, 0));
    assert_eq!(slot.transform, [2.0, 0.0, 0.0, 2.0, 0.5, 0.25]);
    assert_eq!((textured.alpha_mode, textured.alpha_cutoff, textured.double_sided), (MaterialAlphaMode::Mask, 0.25, true));
    let coat = &model.materials[1];
    assert_eq!((coat.metallic_factor, coat.roughness_factor, coat.normal_scale, coat.occlusion_strength), (0.25, 0.75, 0.5, 0.875));
    assert_eq!((coat.emissive_factor, coat.emissive_strength, coat.alpha_mode, coat.unlit), (Vec3::new(1.0, 0.5, 0.25), 2.0, MaterialAlphaMode::Blend, true));
    assert_eq!((coat.index_of_refraction, coat.clear_coat_factor, coat.clear_coat_roughness, coat.specular_factor, coat.transmission_factor), (1.25, 0.75, 0.125, 0.5, 0.375));
    assert_eq!(coat.specular_color, Vec3::new(1.0, 0.5, 0.25));
    assert_eq!(coat.approximated_features, vec!["KHR_materials_clearcoat".to_string(), "KHR_materials_transmission".to_string()]);
    assert_eq!((coat.occlusion_texture.texture, coat.occlusion_texture.sampler, coat.occlusion_texture.tex_coord), (Some(0), None, 1));

    let nodes = &model.nodes;
    assert_eq!(nodes.len(), 5);
    assert_eq!((nodes[0].children.clone(), nodes[0].default_transform.translation, nodes[0].default_transform.scale), (vec![1, 2, 3], Vec3::new(1.0, 2.0, 3.0), Vec3::splat(2.0)));
    assert_eq!(nodes[0].local_matrix, Mat4::from_scale_rotation_translation(Vec3::splat(2.0), Quat::IDENTITY, Vec3::new(1.0, 2.0, 3.0)));
    assert!(nodes[1].has_fixed_matrix);
    // C#'s M21 = 0.5 (row 2, column 1) and M41 = 4 are glam's y_axis.x and w_axis.x.
    assert_eq!((nodes[1].local_matrix.y_axis.x, nodes[1].local_matrix.w_axis.x), (0.5, 4.0));
    assert_eq!((nodes[2].mesh, nodes[2].skin, nodes[2].default_morph_weights.clone()), (Some(1), Some(0), vec![0.25]));
    assert_eq!((nodes[4].parent, nodes[4].default_transform.rotation), (Some(3), Quat::from_xyzw(0.0, 0.0, 0.70710677, 0.70710677)));
    assert!(nodes.iter().all(|n| n.in_scene));

    let skin = &model.skins[0];
    assert_eq!((skin.name.as_deref(), skin.joints.clone(), skin.skeleton), (Some("skin"), vec![3, 4], Some(3)));
    assert_eq!(skin.inverse_bind_matrices[1], Mat4::from_translation(Vec3::new(0.0, -2.0, 0.0)));

    let clip = &model.animations[0];
    assert_eq!((clip.name.as_deref(), clip.channels.len(), clip.duration()), (Some("move"), 4, 2.0));
    let paths: Vec<(usize, AnimationPath, AnimationInterpolation, usize)> = clip.channels.iter().map(|c| (c.node, c.path, c.interpolation, c.component_count)).collect();
    assert_eq!(
        paths,
        vec![(0, AnimationPath::Translation, AnimationInterpolation::Step, 3), (3, AnimationPath::Rotation, AnimationInterpolation::Linear, 4), (4, AnimationPath::Scale, AnimationInterpolation::CubicSpline, 3), (2, AnimationPath::Weights, AnimationInterpolation::Linear, 1)]
    );
    assert_eq!(clip.channels[2].values.len(), 18);
    assert_eq!(clip.channels[3].values, vec![0.25, 1.0]);
    assert_eq!((model.animations[1].name.as_deref(), model.animations[1].channels.len()), (Some("idle"), 0));
}

#[test]
fn a_texture_two_models_use_is_stored_once() {
    let set = ModelDataSet::read(TWO_MODELS).unwrap();
    assert_eq!(set.textures.len(), 2);
    assert_eq!((set.textures[0].name.as_deref(), set.textures[0].pixels), (Some("shared"), &[10u8, 20, 30, 255, 40, 50, 60, 255][..]));
    assert_eq!((set.textures[1].srgb, set.textures[1].mipmaps, set.textures[1].pixels), (false, false, &[1u8, 2, 3, 4][..]));
    let (first, second) = (&set.models[0], &set.models[1]);
    assert_eq!((first.name.as_str(), first.textures.clone(), second.name.as_str(), second.textures.clone()), ("first", vec![0], "second", vec![1, 0]));
    assert_eq!(second.samplers[0].address_mode_u, Some(wgpu::AddressMode::ClampToEdge));
    assert_eq!(first.samplers[0].max_lod, 32.0);
    // Local texture 1 of "second" is set texture 0, the one "first" uses.
    assert_eq!(second.textures[second.materials[0].base_color_texture.texture.unwrap()], 0);
    assert_eq!(second.textures[second.materials[0].emissive_texture.texture.unwrap()], 1);
    assert_eq!(vertices(second.vertex_bytes)[2].position, Vec3::ONE);
    assert_eq!(second.index_bytes, &[0, 0, 1, 0, 2, 0, 0, 0, 2, 0, 3, 0]);
    assert_eq!((second.meshes[0].name.as_deref(), second.nodes[0].name.as_deref()), (Some("second-mesh"), Some("second-node")));
}

#[test]
fn the_layout_is_derived_as_csharp_derives_it() {
    let set = ModelDataSet::read(EVERY_SECTION).unwrap();
    let model = &set.models[0];
    let layout = ModelLayout::new(&model.nodes, &model.meshes, &model.skins);
    assert_eq!(layout.node_order, vec![0, 1, 2, 3, 4]);
    assert_eq!(layout.draw_items, vec![ModelDrawItem { node: 1, mesh: 0, primitive: 0 }, ModelDrawItem { node: 2, mesh: 1, primitive: 0 }]);
    assert_eq!((layout.morph_weight_offsets.clone(), layout.morph_weight_count, layout.baked_morph_weights.clone()), (vec![None, None, Some(0), None, None], 1, vec![0.25]));
    assert_eq!((layout.skin_palette_offsets.clone(), layout.palette_size, layout.triangle_count), (vec![0], 2, 3));
    let matrices = rest_matrices(&model.nodes, &layout.node_order);
    // joint-1 = root * joint-0 * joint-1: translation (1,2,3) + 2 * ((0,1,0) + (0,1,0)).
    assert!(matrices[4].transform_point3(Vec3::ZERO).distance(Vec3::new(1.0, 6.0, 3.0)) < 1e-5);
    // The skinned primitive's bounds count untransformed; the quad's through root * shear.
    assert!(layout.bounds_max.z >= 0.5);
}

#[test]
fn the_model_facts_are_readable_without_a_gpu() {
    let set = ModelDataSet::read(EVERY_SECTION).unwrap();
    let model = &set.models[0];
    let layout = ModelLayout::new(&model.nodes, &model.meshes, &model.skins);
    assert_eq!(model.bounds(), (layout.bounds_min, layout.bounds_max));
    // The quad's material is MASK and the skinned primitive uses the default material: the
    // blended material "coat" is not used by a drawn part.
    assert!(model.has_opaque_parts() && !model.has_blended_parts());
    assert_eq!(model.animation_names(), vec![Some("move"), Some("idle")]);
    assert_eq!(model.node_names().len(), 5);
    let mut blended = model.clone();
    blended.default_material.alpha_mode = MaterialAlphaMode::Blend;
    assert!(blended.has_opaque_parts() && blended.has_blended_parts());
    let triangle = &ModelDataSet::read(TRIANGLE).unwrap().models[0];
    assert_eq!(triangle.bounds(), (Vec3::ZERO, Vec3::new(1.0, 1.0, 0.0)));
    assert_eq!(triangle.node_names(), vec![Some("triangle")]);
    assert!(triangle.animation_names().is_empty());
}

#[test]
fn the_morph_centre_shift_is_derived_from_the_blob_s_vertices() {
    // The skinned primitive's one target moves its three vertices 0.5 along z.
    let set = ModelDataSet::read(EVERY_SECTION).unwrap();
    let model = &set.models[0];
    let shifts = morph_center_shifts(&model.meshes, model.vertex_bytes);
    assert_eq!(shifts[0], vec![Vec::<Vec3>::new()]);
    assert!(shifts[1][0][0].distance(Vec3::new(0.0, 0.0, 0.5)) < 1e-6, "{:?}", shifts[1][0]);
}

#[test]
fn damaged_data_is_refused_with_a_message_that_says_what_to_do() {
    let error = ModelDataSet::read(b"glTF....").unwrap_err();
    assert!(error.message().contains("not model data"), "{error}");
    let mut wrong_version = TRIANGLE.to_vec();
    wrong_version[4] = 2;
    assert!(ModelDataSet::read(&wrong_version).unwrap_err().message().contains("format version 2"));
    let cut = &TRIANGLE[..TRIANGLE.len() - 16];
    assert!(ModelDataSet::read(cut).unwrap_err().message().contains("cut short"));
    let mut bad_material = TRIANGLE.to_vec();
    let entry = (0..18).map(|i| 32 + i * 24).find(|&at| &bad_material[at..at + 4] == b"PRIM").unwrap();
    let offset = u32::from_le_bytes(bad_material[entry + 16..entry + 20].try_into().unwrap()) as usize;
    bad_material[offset..offset + 4].copy_from_slice(&7i32.to_le_bytes());
    let message = ModelDataSet::read(&bad_material).unwrap_err().to_string();
    assert!(message.contains("model 0 'triangle' mesh 0 primitive 0 names material 7, but there are 1"), "{message}");
}
