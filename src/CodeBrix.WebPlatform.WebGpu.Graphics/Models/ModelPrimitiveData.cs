using System.Collections.Generic;
using System.Numerics;

namespace CodeBrix.WebPlatform.WebGpu.Graphics.Models;

/// <summary>
/// One drawable part of a <see cref="ModelMeshData" />: an index range in the model's shared arrays and a material.
/// Indices are relative to <see cref="BaseVertex" />.
/// </summary>
public sealed class ModelPrimitiveData
{
    /// <summary>The material, as a position in <see cref="ModelData.Materials" />, or -1 for the model's <see cref="ModelData.DefaultMaterial" />. Default -1.</summary>
    public int Material { get; set; } = -1;

    /// <summary>The first index in <see cref="ModelData.Indices" />.</summary>
    public uint FirstIndex { get; set; }

    /// <summary>The number of indices (a multiple of 3).</summary>
    public uint IndexCount { get; set; }

    /// <summary>The primitive's first vertex in <see cref="ModelData.Vertices" /> (added to every index).</summary>
    public int BaseVertex { get; set; }

    /// <summary>The number of vertices (every index of the primitive is below it).</summary>
    public int VertexCount { get; set; }

    /// <summary>The first vertex in <see cref="ModelData.SkinVertices" />, or -1 when the primitive is not skinned. Default -1.</summary>
    public int SkinBaseVertex { get; set; } = -1;

    /// <summary>The minimum corner of the mesh-space bounds (with the default morph weights applied).</summary>
    public Vector3 BoundsMin { get; set; }

    /// <summary>The maximum corner of the mesh-space bounds.</summary>
    public Vector3 BoundsMax { get; set; }

    /// <summary>True when the file has COLOR_0 (otherwise the colour is white).</summary>
    public bool HasVertexColors { get; set; }

    /// <summary>True when the file has TEXCOORD_1.</summary>
    public bool HasTexCoord1 { get; set; }

    /// <summary>True when the file has TANGENT or tangents were generated (the material has a normal map).</summary>
    public bool HasTangents { get; set; }

    /// <summary>True when the tangents were generated at load.</summary>
    public bool TangentsGenerated { get; set; }

    /// <summary>True when the file had no NORMAL and smooth normals were generated.</summary>
    public bool NormalsGenerated { get; set; }

    /// <summary>
    /// The morph targets (deltas per vertex). The vertex array holds the shape with the mesh's
    /// <see cref="ModelMeshData.DefaultMorphWeights" /> already applied, so a renderer applies
    /// <c>(weight - defaultWeight) * delta</c>.
    /// </summary>
    public List<ModelMorphTargetData> MorphTargets { get; set; } = new List<ModelMorphTargetData>();
}
