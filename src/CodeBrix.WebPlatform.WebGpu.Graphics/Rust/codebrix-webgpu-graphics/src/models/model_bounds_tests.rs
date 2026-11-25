//! Host tests of the culling bounds: they hold EVERY pose an independent poser makes (the
//! renderer's vertex arithmetic: morph, then the node matrix or the weighted joint palette) at
//! many more times than the bounds were sampled at, for the repository's fixture model (a node
//! hierarchy, a skin, a morph target and clips with every interpolation), for made-up models
//! that move far (a long turn, a bending skin, a morph) and - when the folder is there - for the
//! GraphicsDemo sample's glTF models.

use glam::{Mat4, Quat, Vec3};

use super::{apply_clip, pose_matrices, sample_times, Bounds, MAX_SAMPLE_TURN};
use crate::models::model_layout::ModelLayout;
use crate::models::{AnimationInterpolation, AnimationPath, ModelAnimation, ModelAnimationChannel, ModelData, ModelDataSet, ModelMaterial, ModelMesh, ModelMorphTarget, ModelNode, ModelPrimitive, ModelSkin, ModelSkinVertex, ModelVertex, NodeTransform};

const TRIANGLE: &[u8] = include_bytes!("../../fixtures/triangle.cbmd");
const EVERY_SECTION: &[u8] = include_bytes!("../../fixtures/every-section.cbmd");

/// The instance's morph weights at rest (each node's defaults), laid out as an instance does.
fn default_weights(data: &ModelData<'_>, layout: &ModelLayout) -> Vec<f32> {
    let mut weights = vec![0.0; layout.morph_weight_count];
    for (index, node) in data.nodes.iter().enumerate() {
        if let Some(offset) = layout.morph_weight_offsets[index] {
            for (target, weight) in node.default_morph_weights.iter().enumerate() {
                if let Some(slot) = weights.get_mut(offset + target) {
                    *slot = *weight;
                }
            }
        }
    }
    weights
}

/// The defaults with a clip's weight channels sampled at `time` over them.
fn clip_weights(data: &ModelData<'_>, layout: &ModelLayout, clip: &ModelAnimation, time: f32) -> Vec<f32> {
    let mut weights = default_weights(data, layout);
    for channel in clip.channels.iter().filter(|c| c.path == AnimationPath::Weights) {
        if let Some(offset) = layout.morph_weight_offsets[channel.node] {
            let mut value = vec![0.0; channel.component_count];
            channel.sample(time, &mut value).unwrap();
            for (target, weight) in value.iter().enumerate() {
                if let Some(slot) = weights.get_mut(offset + target) {
                    *slot = *weight;
                }
            }
        }
    }
    weights
}

/// Every vertex the renderer draws for these local transforms and morph weights, in model space:
/// position + (weight - baked) x delta per target, then the node's matrix, or for a skinned
/// primitive the weighted sum of its joints' palette matrices.
fn posed_vertices(data: &ModelData<'_>, layout: &ModelLayout, locals: &[NodeTransform], weights: &[f32]) -> Vec<Vec3> {
    let matrices = pose_matrices(&data.nodes, &layout.node_order, locals);
    let mut palette = vec![Mat4::IDENTITY; layout.palette_size];
    for (skin, data) in data.skins.iter().enumerate() {
        for (joint, &node) in data.joints.iter().enumerate() {
            palette[layout.skin_palette_offsets[skin] + joint] = matrices[node] * data.inverse_bind_matrices.get(joint).copied().unwrap_or(Mat4::IDENTITY);
        }
    }
    let vertices: Vec<ModelVertex> = data.vertex_bytes.chunks_exact(ModelVertex::SIZE).map(bytemuck::pod_read_unaligned).collect();
    let influences: Vec<ModelSkinVertex> = data.skin_bytes.chunks_exact(ModelSkinVertex::SIZE).map(bytemuck::pod_read_unaligned).collect();
    let mut points = Vec::new();
    for item in &layout.draw_items {
        let primitive = &data.meshes[item.mesh].primitives[item.primitive];
        let node = &data.nodes[item.node];
        for vertex in 0..primitive.vertex_count as usize {
            let mut point = vertices[primitive.base_vertex as usize + vertex].position;
            if let Some(offset) = layout.morph_weight_offsets[item.node] {
                for target in 0..primitive.morph_targets.len().min(node.default_morph_weights.len()) {
                    if let Some(delta) = primitive.morph_targets[target].position_deltas.get(vertex) {
                        point += *delta * (weights[offset + target] - layout.baked_morph_weights[offset + target]);
                    }
                }
            }
            match node.skin.filter(|&s| s < data.skins.len()) {
                Some(skin) if primitive.is_skinned() => {
                    let influence = influences[primitive.skin_base_vertex.unwrap() as usize + vertex];
                    let offset = layout.skin_palette_offsets[skin];
                    point = influence.joints.iter().zip(&influence.weights).filter(|(_, &w)| w > 0.0).map(|(&j, &w)| palette[offset + j as usize].transform_point3(point) * w).sum();
                }
                _ => point = matrices[item.node].transform_point3(point),
            }
            points.push(point);
        }
    }
    points
}

/// Asserts that the model's culling bounds hold every vertex of the rest pose and of every clip
/// sampled `samples` times evenly (and at every key), each with the clip's own weights, every
/// weight 0 and every weight 1. Returns the bounds and the box of everything posed.
fn assert_holds_every_pose(data: &ModelData<'_>, samples: usize) -> (Bounds, Bounds) {
    let layout = ModelLayout::new(&data.nodes, &data.meshes, &data.skins);
    let (min, max) = data.culling_bounds();
    let tolerance = 1e-4 * (1.0 + (max - min).length());
    let mut reached = Bounds::EMPTY;
    let mut check = |points: Vec<Vec3>, what: &dyn Fn() -> String| {
        for point in points {
            reached.add_point(point);
            assert!(point.cmpge(min - Vec3::splat(tolerance)).all() && point.cmple(max + Vec3::splat(tolerance)).all(), "model '{}': {} puts a vertex at {point}, outside the culling bounds {min}..{max}", data.name, what());
        }
    };
    let rest: Vec<NodeTransform> = data.nodes.iter().map(|n| n.default_transform).collect();
    let ones = vec![1.0; layout.morph_weight_count];
    let zeros = vec![0.0; layout.morph_weight_count];
    for weights in [default_weights(data, &layout), ones.clone(), zeros.clone()] {
        check(posed_vertices(data, &layout, &rest, &weights), &|| format!("the rest pose with weights {weights:?}"));
    }
    for clip in &data.animations {
        let duration = clip.duration();
        let mut times: Vec<f32> = (0..=samples).map(|i| duration * i as f32 / samples as f32).collect();
        times.extend(clip.channels.iter().flat_map(|c| c.times.iter().copied()));
        for time in times {
            let mut locals = rest.clone();
            apply_clip(clip, time, &mut locals);
            for weights in [clip_weights(data, &layout, clip, time), ones.clone(), zeros.clone()] {
                check(posed_vertices(data, &layout, &locals, &weights), &|| format!("clip {} at {time} s with weights {weights:?}", clip.index));
            }
        }
    }
    (Bounds::new(min, max), reached)
}

#[test]
fn a_model_without_clips_or_morph_targets_keeps_its_rest_bounds() {
    let set = ModelDataSet::read(TRIANGLE).unwrap();
    let triangle = &set.models[0];
    assert_eq!(triangle.culling_bounds(), triangle.bounds());
    assert_holds_every_pose(triangle, 1);
}

#[test]
fn the_fixture_s_bounds_hold_every_pose_of_its_hierarchy_skin_morph_and_clips() {
    let set = ModelDataSet::read(EVERY_SECTION).unwrap();
    let model = &set.models[0];
    let (bounds, reached) = assert_holds_every_pose(model, 400);
    // They hold the rest-pose bounds too, and grow beyond them (the root moves, a joint turns).
    let (rest_min, rest_max) = model.bounds();
    assert!(bounds.min.cmple(rest_min).all() && bounds.max.cmpge(rest_max).all());
    assert!(bounds.min != rest_min || bounds.max != rest_max);
    // Conservative, not absurd: no wider than twice what the poses reach, per axis.
    let size = reached.max - reached.min;
    assert!((bounds.max - bounds.min).cmple(size * 2.0 + Vec3::splat(1e-3)).all(), "{bounds:?} against {reached:?}");
}

// ---- made-up models --------------------------------------------------------------------------

fn vertex(position: Vec3) -> ModelVertex {
    ModelVertex { position, ..ModelVertex::default() }
}

fn primitive(base_vertex: u32, positions: &[Vec3], skinned: bool, morph_targets: Vec<ModelMorphTarget>) -> ModelPrimitive {
    let mut bounds = Bounds::EMPTY;
    positions.iter().for_each(|p| bounds.add_point(*p));
    ModelPrimitive { index: 0, material: None, first_index: 0, index_count: 3, base_vertex, vertex_count: positions.len() as u32, skin_base_vertex: skinned.then_some(0), bounds_min: bounds.min, bounds_max: bounds.max, has_vertex_colors: false, has_tex_coord1: false, has_tangents: false, tangents_generated: false, normals_generated: false, morph_targets }
}

fn node(index: usize, parent: Option<usize>, children: Vec<usize>, mesh: Option<usize>, skin: Option<usize>, transform: NodeTransform, morph_weights: Vec<f32>) -> ModelNode {
    ModelNode { index, name: None, parent, children, mesh, skin, default_transform: transform, has_fixed_matrix: false, local_matrix: transform.to_matrix(), default_morph_weights: morph_weights, in_scene: true }
}

fn model<'a>(name: &str, vertex_bytes: &'a [u8], skin_bytes: &'a [u8], meshes: Vec<ModelMesh>, nodes: Vec<ModelNode>, skins: Vec<ModelSkin>, animations: Vec<ModelAnimation>) -> ModelData<'a> {
    let roots = nodes.iter().filter(|n| n.parent.is_none()).map(|n| n.index).collect();
    ModelData {
        name: name.to_string(),
        vertex_bytes,
        vertex_count: (vertex_bytes.len() / ModelVertex::SIZE) as u32,
        index_bytes: &[],
        index_count: 3,
        index_format: wgpu::IndexFormat::Uint16,
        skin_bytes,
        skin_vertex_count: (skin_bytes.len() / ModelSkinVertex::SIZE) as u32,
        meshes,
        materials: Vec::new(),
        default_material: ModelMaterial::new(-1, None),
        nodes,
        root_nodes: roots,
        skins,
        animations,
        textures: Vec::new(),
        samplers: Vec::new(),
        warnings: Vec::new(),
    }
}

fn rotation_channel(node: usize, interpolation: AnimationInterpolation, times: Vec<f32>, rotations: &[Quat]) -> ModelAnimationChannel {
    let values = match interpolation {
        AnimationInterpolation::CubicSpline => rotations.iter().flat_map(|q| [0.0, 0.0, 0.0, 0.0].into_iter().chain(q.to_array()).chain([0.0; 4])).collect(),
        _ => rotations.iter().flat_map(|q| q.to_array()).collect(),
    };
    ModelAnimationChannel { node, path: AnimationPath::Rotation, interpolation, times, values, component_count: 4 }
}

fn cube(center: Vec3, half: f32) -> Vec<Vec3> {
    (0..8).map(|c| center + Vec3::new(if c & 1 == 0 { -half } else { half }, if c & 2 == 0 { -half } else { half }, if c & 4 == 0 { -half } else { half })).collect()
}

#[test]
fn a_long_turn_between_two_keys_is_followed_round_its_arc() {
    // A root that turns 170 degrees about z between two keys carries a cube 2 m out on x round a
    // half circle: the cube passes y = 2.5 at 90 degrees, which neither key reaches.
    let positions = cube(Vec3::ZERO, 0.5);
    let vertices: Vec<ModelVertex> = positions.iter().copied().map(vertex).collect();
    let bytes: Vec<u8> = bytemuck::cast_slice(&vertices).to_vec();
    let mesh = ModelMesh { index: 0, name: None, primitives: vec![primitive(0, &positions, false, Vec::new())], default_morph_weights: Vec::new() };
    let nodes = vec![node(0, None, vec![1], None, None, NodeTransform::IDENTITY, Vec::new()), node(1, Some(0), Vec::new(), Some(0), None, NodeTransform::new(Vec3::new(2.0, 0.0, 0.0), Quat::IDENTITY, Vec3::ONE), Vec::new())];
    for interpolation in [AnimationInterpolation::Linear, AnimationInterpolation::CubicSpline] {
        let turn = rotation_channel(0, interpolation, vec![0.0, 1.0], &[Quat::IDENTITY, Quat::from_rotation_z(170f32.to_radians())]);
        let clip = ModelAnimation { index: 0, name: Some("turn".to_string()), channels: vec![turn] };
        // The interval is split so that no step turns more than the limit.
        assert!(sample_times(&clip).len() as f32 >= 170f32.to_radians() / MAX_SAMPLE_TURN);
        let data = model("arm", &bytes, &[], vec![mesh.clone()], nodes.clone(), Vec::new(), vec![clip]);
        let (bounds, reached) = assert_holds_every_pose(&data, 1000);
        assert!(reached.max.y > 2.49 && bounds.max.y < 2.5 * 1.1, "{bounds:?}");
    }
}

#[test]
fn a_skin_that_bends_is_covered_round_its_moving_joints() {
    // A column of vertices from y = 0 to y = 2 on a two-joint skin: joint 0 (node 1) at the
    // origin, joint 1 (node 2) at y = 1 under it. The lower vertices follow joint 0, the upper
    // ones joint 1, the middle ones both. Clip 0 bends joint 1 by 150 degrees; clip 1 moves the
    // whole skeleton 3 m along x by STEP.
    let positions: Vec<Vec3> = (0..=8).flat_map(|i| [Vec3::new(-0.1, i as f32 * 0.25, 0.0), Vec3::new(0.1, i as f32 * 0.25, 0.0)]).collect();
    let vertices: Vec<ModelVertex> = positions.iter().copied().map(vertex).collect();
    let influences: Vec<ModelSkinVertex> = positions
        .iter()
        .map(|p| match p.y {
            y if y < 0.9 => ModelSkinVertex { joints: [0, 0, 0, 0], weights: [1.0, 0.0, 0.0, 0.0] },
            y if y < 1.1 => ModelSkinVertex { joints: [0, 1, 0, 0], weights: [0.5, 0.5, 0.0, 0.0] },
            _ => ModelSkinVertex { joints: [1, 0, 0, 0], weights: [1.0, 0.0, 0.0, 0.0] },
        })
        .collect();
    let vertex_bytes: Vec<u8> = bytemuck::cast_slice(&vertices).to_vec();
    let skin_bytes: Vec<u8> = bytemuck::cast_slice(&influences).to_vec();
    let mesh = ModelMesh { index: 0, name: None, primitives: vec![primitive(0, &positions, true, Vec::new())], default_morph_weights: Vec::new() };
    let up = NodeTransform::new(Vec3::Y, Quat::IDENTITY, Vec3::ONE);
    let nodes = vec![
        node(0, None, Vec::new(), Some(0), Some(0), NodeTransform::IDENTITY, Vec::new()),
        node(1, None, vec![2], None, None, NodeTransform::IDENTITY, Vec::new()),
        node(2, Some(1), Vec::new(), None, None, up, Vec::new()),
    ];
    let skin = ModelSkin { index: 0, name: None, joints: vec![1, 2], inverse_bind_matrices: vec![Mat4::IDENTITY, Mat4::from_translation(Vec3::NEG_Y)], skeleton: Some(1) };
    let bend = ModelAnimation { index: 0, name: Some("bend".to_string()), channels: vec![rotation_channel(2, AnimationInterpolation::Linear, vec![0.0, 0.5, 1.0], &[Quat::IDENTITY, Quat::from_rotation_z(150f32.to_radians()), Quat::IDENTITY])] };
    let slide = ModelAnimation { index: 1, name: Some("slide".to_string()), channels: vec![ModelAnimationChannel { node: 1, path: AnimationPath::Translation, interpolation: AnimationInterpolation::Step, times: vec![0.0, 1.0], values: vec![0.0, 0.0, 0.0, 3.0, 0.0, 0.0], component_count: 3 }] };
    let data = model("bending", &vertex_bytes, &skin_bytes, vec![mesh], nodes, vec![skin], vec![bend, slide]);
    let (bounds, reached) = assert_holds_every_pose(&data, 600);
    // The bent tip reaches x = -sin(150) - ... to the left and the slide 3 m to the right.
    assert!(reached.min.x < -0.4 && reached.max.x > 3.0);
    assert!(bounds.max.x < 3.1 * 1.2 && bounds.min.x > -2.0, "{bounds:?}");
}

#[test]
fn morph_targets_count_at_full_weight_and_no_clips_means_no_margin() {
    // A quad whose one target lifts vertex 3 by 2 m along z; the mesh's default weight 0.25 is
    // baked into the vertex buffer (vertex 3 stored at z = 0.5).
    let positions = [Vec3::ZERO, Vec3::X, Vec3::Y, Vec3::new(1.0, 1.0, 0.5)];
    let vertices: Vec<ModelVertex> = positions.iter().copied().map(vertex).collect();
    let bytes: Vec<u8> = bytemuck::cast_slice(&vertices).to_vec();
    let target = ModelMorphTarget { position_deltas: vec![Vec3::ZERO, Vec3::ZERO, Vec3::ZERO, Vec3::new(0.0, 0.0, 2.0)], normal_deltas: Vec::new(), tangent_deltas: Vec::new() };
    let mesh = ModelMesh { index: 0, name: None, primitives: vec![primitive(0, &positions, false, vec![target])], default_morph_weights: vec![0.25] };
    let nodes = vec![node(0, None, Vec::new(), Some(0), None, NodeTransform::IDENTITY, vec![0.25])];
    let data = model("lift", &bytes, &[], vec![mesh], nodes, Vec::new(), Vec::new());
    assert_holds_every_pose(&data, 1);
    // Weight 1: z = 0.5 + 0.75 * 2 = 2; weight 0: z = 0.5 - 0.25 * 2 = 0. Exact: no clips, no margin.
    assert_eq!(data.culling_bounds(), (Vec3::ZERO, Vec3::new(1.0, 1.0, 2.0)));
    assert_eq!(data.bounds(), (Vec3::ZERO, Vec3::new(1.0, 1.0, 0.5)));
}

#[test]
fn bounds_place_and_grow_as_boxes() {
    let mut bounds = Bounds::EMPTY;
    assert!(bounds.is_empty());
    bounds.add(Bounds::EMPTY);
    assert!(bounds.is_empty());
    bounds.add_point(Vec3::ONE);
    bounds.add(Bounds::new(Vec3::ZERO, Vec3::splat(0.5)));
    assert_eq!(bounds, Bounds::new(Vec3::ZERO, Vec3::ONE));
    // A quarter turn about z of the unit box at the origin: x from -1 to 0.
    let turned = bounds.transformed(&Mat4::from_rotation_z(std::f32::consts::FRAC_PI_2));
    assert!(turned.min.distance(Vec3::new(-1.0, 0.0, 0.0)) < 1e-6 && turned.max.distance(Vec3::new(0.0, 1.0, 1.0)) < 1e-6, "{turned:?}");
    assert!(Bounds::EMPTY.transformed(&Mat4::IDENTITY).is_empty());
}

/// The GraphicsDemo sample's glTF models, loaded with the crate's own glTF loader, when their
/// folder can be found: next to the crate in the repository, or named by the environment variable
/// CODEBRIX_GRAPHICS_TEST_SAMPLE_MODELS. A copy of the crate elsewhere (the vendor folder the
/// .NET tests run the unit tests in) skips this test.
#[test]
fn the_sample_glb_models_bounds_hold_every_pose() {
    let folder = std::env::var("CODEBRIX_GRAPHICS_TEST_SAMPLE_MODELS").map(std::path::PathBuf::from).unwrap_or_else(|_| std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../../samples/GraphicsDemo/backend/GraphicsDemo.Assets/Models"));
    let Ok(packs) = std::fs::read_dir(&folder) else {
        eprintln!("SKIPPED: no sample model folder at {} (set CODEBRIX_GRAPHICS_TEST_SAMPLE_MODELS)", folder.display());
        return;
    };
    let mut files: Vec<std::path::PathBuf> = packs.flatten().flat_map(|pack| std::fs::read_dir(pack.path()).into_iter().flatten().flatten().map(|f| f.path())).filter(|p| p.extension().is_some_and(|e| e == "glb")).collect();
    files.sort();
    let mut checked = 0;
    let mut moving = 0;
    for file in files {
        let bytes = std::fs::read(&file).unwrap();
        let name = file.file_stem().unwrap().to_string_lossy().to_string();
        let document = crate::GltfDocument::parse(&bytes).unwrap_or_else(|e| panic!("{}: {e}", file.display()));
        let mut set = crate::GltfModelSet::new();
        set.add(&document, &name, None, &|_| None, &crate::GltfLoadOptions { load_textures: false, ..crate::GltfLoadOptions::default() }).unwrap_or_else(|e| panic!("{}: {e}", file.display()));
        let data = set.data_set();
        for model in &data.models {
            assert_holds_every_pose(model, 120);
            checked += 1;
            if !model.animations.is_empty() || !model.skins.is_empty() {
                moving += 1;
            }
        }
    }
    eprintln!("checked {checked} sample models, {moving} of them animated or skinned");
    assert!(checked > 0);
}
