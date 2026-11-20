//! The data a [`Model`](crate::Model) derives from its nodes, meshes and skins when it is made -
//! the node order, what one instance draws, the morph-weight and joint-palette layouts and the
//! rest-pose bounds (C#: what the loader and the `Model` constructor compute). CPU only, tested on
//! the host.

use glam::{Mat4, Vec3};

use crate::models::{ModelMesh, ModelNode, ModelSkin};

/// One (node, mesh primitive) pair an instance draws (C#: `ModelDrawItem`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ModelDrawItem {
    pub(crate) node: usize,
    pub(crate) mesh: usize,
    pub(crate) primitive: usize,
}

/// What a model derives from its data.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct ModelLayout {
    /// Every node, parents before children (C#: `NodeOrder`).
    pub(crate) node_order: Vec<usize>,
    /// Every (node, primitive) of the scene's mesh nodes (C#: `DrawItems`).
    pub(crate) draw_items: Vec<ModelDrawItem>,
    /// Where each node's morph weights start in an instance's weights, None for none (C#:
    /// `MorphWeightOffsets`).
    pub(crate) morph_weight_offsets: Vec<Option<usize>>,
    /// The total number of per-instance morph weights (C#: `MorphWeightCount`).
    pub(crate) morph_weight_count: usize,
    /// The mesh default weight baked into the vertex buffer for every weight slot (C#:
    /// `BakedMorphWeights`).
    pub(crate) baked_morph_weights: Vec<f32>,
    /// Where each skin's joints start in an instance's palette (C#: `SkinPaletteOffsets`).
    pub(crate) skin_palette_offsets: Vec<usize>,
    /// The number of joint matrices in an instance's palette (C#: `PaletteSize`).
    pub(crate) palette_size: usize,
    /// The rest-pose bounds (C#: `BoundsMin`, `BoundsMax`).
    pub(crate) bounds_min: Vec3,
    pub(crate) bounds_max: Vec3,
    /// The triangles one instance draws (C#: `TriangleCount`).
    pub(crate) triangle_count: u32,
}

impl ModelLayout {
    /// Derives the layout exactly as the C# loader's `Build` and the `Model` constructor do.
    pub(crate) fn new(nodes: &[ModelNode], meshes: &[ModelMesh], skins: &[ModelSkin]) -> ModelLayout {
        // Breadth first from every node without a parent (C#'s ReadNodes order).
        let mut node_order = Vec::with_capacity(nodes.len());
        let mut visited = vec![false; nodes.len()];
        let mut queue = std::collections::VecDeque::new();
        for (index, node) in nodes.iter().enumerate() {
            if node.parent.is_some() {
                continue;
            }
            queue.push_back(index);
            while let Some(current) = queue.pop_front() {
                if visited[current] {
                    continue;
                }
                visited[current] = true;
                node_order.push(current);
                queue.extend(nodes[current].children.iter().copied());
            }
        }

        let mut draw_items = Vec::new();
        let mut morph_weight_offsets = vec![None; nodes.len()];
        let mut morph_weight_count = 0;
        for &index in &node_order {
            let node = &nodes[index];
            let Some(mesh) = node.mesh.and_then(|m| meshes.get(m)) else {
                continue;
            };
            if mesh.morph_target_count() > 0 {
                morph_weight_offsets[index] = Some(morph_weight_count);
                morph_weight_count += mesh.morph_target_count();
            }
            if !node.in_scene {
                continue;
            }
            for primitive in 0..mesh.primitives.len() {
                draw_items.push(ModelDrawItem { node: index, mesh: mesh.index, primitive });
            }
        }

        let mut baked_morph_weights = vec![0.0; morph_weight_count];
        for (index, node) in nodes.iter().enumerate() {
            if let (Some(offset), Some(mesh)) = (morph_weight_offsets[index], node.mesh.and_then(|m| meshes.get(m))) {
                let defaults = &mesh.default_morph_weights;
                baked_morph_weights[offset..offset + defaults.len()].copy_from_slice(defaults);
            }
        }

        let mut skin_palette_offsets = Vec::with_capacity(skins.len());
        let mut palette_size = 0;
        for skin in skins {
            skin_palette_offsets.push(palette_size);
            palette_size += skin.joint_count();
        }

        let matrices = rest_matrices(nodes, &node_order);
        let mut min = Vec3::splat(f32::MAX);
        let mut max = Vec3::splat(f32::MIN);
        let mut triangle_count = 0;
        for item in &draw_items {
            let primitive = &meshes[item.mesh].primitives[item.primitive];
            triangle_count += primitive.triangle_count();
            let matrix = if primitive.is_skinned() { Mat4::IDENTITY } else { matrices[item.node] };
            for corner in 0..8 {
                let point = Vec3::new(
                    if corner & 1 == 0 { primitive.bounds_min.x } else { primitive.bounds_max.x },
                    if corner & 2 == 0 { primitive.bounds_min.y } else { primitive.bounds_max.y },
                    if corner & 4 == 0 { primitive.bounds_min.z } else { primitive.bounds_max.z },
                );
                let transformed = matrix.transform_point3(point);
                min = min.min(transformed);
                max = max.max(transformed);
            }
        }
        if triangle_count == 0 {
            min = Vec3::ZERO;
            max = Vec3::ZERO;
        }
        ModelLayout { node_order, draw_items, morph_weight_offsets, morph_weight_count, baked_morph_weights, skin_palette_offsets, palette_size, bounds_min: min, bounds_max: max, triangle_count }
    }
}

/// Every node's rest-pose model-space matrix (parents first).
pub(crate) fn rest_matrices(nodes: &[ModelNode], order: &[usize]) -> Vec<Mat4> {
    let mut matrices = vec![Mat4::IDENTITY; nodes.len()];
    for &index in order {
        let node = &nodes[index];
        let local = if node.has_fixed_matrix { node.local_matrix } else { node.default_transform.to_matrix() };
        matrices[index] = match node.parent {
            Some(parent) => matrices[parent] * local,
            None => local,
        };
    }
    matrices
}
