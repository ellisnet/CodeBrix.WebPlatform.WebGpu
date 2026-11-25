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
    /// By mesh, primitive and morph target: how far the target moves the centre of the
    /// primitive's bounds at weight 1, from the vertex buffer's positions (the default weights
    /// baked in); empty until [`morph_center_shifts`] fills it at load. The translucent sort uses
    /// it. Not in C#.
    pub(crate) morph_center_shifts: Vec<Vec<Vec<Vec3>>>,
    /// The culling bounds: the rest-pose bounds until the model is made, then grown at load to
    /// hold every pose of the model's clips, its morph targets at full weight and its skin
    /// round its moving joints (`model_bounds::culling_bounds`). Not in C#.
    pub(crate) culling_bounds_min: Vec3,
    pub(crate) culling_bounds_max: Vec3,
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
        ModelLayout { node_order, draw_items, morph_weight_offsets, morph_weight_count, baked_morph_weights, skin_palette_offsets, palette_size, bounds_min: min, bounds_max: max, triangle_count, morph_center_shifts: Vec::new(), culling_bounds_min: min, culling_bounds_max: max }
    }
}

/// By mesh, primitive and morph target: how far the target, at weight 1, moves the centre of the
/// bounds of the primitive's positions in `vertex_bytes` (the [`ModelVertex`](crate::ModelVertex)
/// bytes of the model; the position is the vertex's first 12 bytes). A primitive whose vertices
/// lie outside the bytes, or a target without position deltas, gets zero shifts. Not in C#.
pub(crate) fn morph_center_shifts(meshes: &[ModelMesh], vertex_bytes: &[u8]) -> Vec<Vec<Vec<Vec3>>> {
    let stride = std::mem::size_of::<crate::ModelVertex>();
    let position = |vertex: usize| -> Option<Vec3> {
        let at = vertex.checked_mul(stride)?;
        let bytes = vertex_bytes.get(at..at + 12)?;
        let float = |i: usize| f32::from_le_bytes([bytes[i], bytes[i + 1], bytes[i + 2], bytes[i + 3]]);
        Some(Vec3::new(float(0), float(4), float(8)))
    };
    meshes
        .iter()
        .map(|mesh| {
            mesh.primitives
                .iter()
                .map(|primitive| {
                    let base = primitive.base_vertex as usize;
                    let positions: Option<Vec<Vec3>> = (0..primitive.vertex_count as usize).map(|v| position(base + v)).collect();
                    let positions = positions.unwrap_or_default();
                    let center = bounds_center(positions.iter().copied());
                    primitive
                        .morph_targets
                        .iter()
                        .map(|target| match (&center, target.position_deltas.is_empty()) {
                            (Some(center), false) => {
                                let moved = positions.iter().enumerate().map(|(v, p)| *p + target.position_deltas.get(v).copied().unwrap_or(Vec3::ZERO));
                                bounds_center(moved).map_or(Vec3::ZERO, |c| c - *center)
                            }
                            _ => Vec3::ZERO,
                        })
                        .collect()
                })
                .collect()
        })
        .collect()
}

/// The centre of the bounds of some points, None for none.
fn bounds_center(points: impl Iterator<Item = Vec3>) -> Option<Vec3> {
    let mut bounds: Option<(Vec3, Vec3)> = None;
    for point in points {
        bounds = Some(match bounds {
            Some((min, max)) => (min.min(point), max.max(point)),
            None => (point, point),
        });
    }
    bounds.map(|(min, max)| (min + max) * 0.5)
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

#[cfg(test)]
mod tests {
    use glam::Vec3;

    use super::morph_center_shifts;
    use crate::models::{ModelMesh, ModelMorphTarget, ModelPrimitive, ModelVertex};

    #[test]
    fn a_morph_target_s_centre_shift_is_the_move_of_the_bounds_centre() {
        // Two vertices at x = 0 and x = 2 (centre 1); the target moves the second to x = 6
        // (centre 3): a shift of 2 along x. A second target has no position deltas.
        let vertices = [ModelVertex { position: Vec3::ZERO, ..ModelVertex::default() }, ModelVertex { position: Vec3::new(2.0, 0.0, 0.0), ..ModelVertex::default() }];
        let bytes: Vec<u8> = bytemuck::cast_slice(&vertices).to_vec();
        let grows = ModelMorphTarget { position_deltas: vec![Vec3::ZERO, Vec3::new(4.0, 0.0, 0.0)], normal_deltas: Vec::new(), tangent_deltas: Vec::new() };
        let normals_only = ModelMorphTarget { position_deltas: Vec::new(), normal_deltas: vec![Vec3::X; 2], tangent_deltas: Vec::new() };
        let primitive = ModelPrimitive { index: 0, material: None, first_index: 0, index_count: 0, base_vertex: 0, vertex_count: 2, skin_base_vertex: None, bounds_min: Vec3::ZERO, bounds_max: Vec3::new(2.0, 0.0, 0.0), has_vertex_colors: false, has_tex_coord1: false, has_tangents: false, tangents_generated: false, normals_generated: false, morph_targets: vec![grows, normals_only] };
        let mesh = ModelMesh { index: 0, name: None, primitives: vec![primitive], default_morph_weights: vec![0.0, 0.0] };
        let shifts = morph_center_shifts(std::slice::from_ref(&mesh), &bytes);
        assert_eq!(shifts, vec![vec![vec![Vec3::new(2.0, 0.0, 0.0), Vec3::ZERO]]]);
        // Vertices outside the bytes: zero shifts, no panic.
        assert_eq!(morph_center_shifts(&[mesh], &bytes[..64]), vec![vec![vec![Vec3::ZERO, Vec3::ZERO]]]);
    }
}
