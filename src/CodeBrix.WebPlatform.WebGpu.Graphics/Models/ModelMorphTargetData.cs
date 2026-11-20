using System;
using System.Numerics;

namespace CodeBrix.WebPlatform.WebGpu.Graphics.Models;

/// <summary>One morph target of a <see cref="ModelPrimitiveData" />: deltas per vertex (each array empty, or one delta per vertex of the primitive).</summary>
public sealed class ModelMorphTargetData
{
    /// <summary>Position deltas, one per vertex (empty when the target moves no positions).</summary>
    public Vector3[] PositionDeltas { get; set; } = Array.Empty<Vector3>();

    /// <summary>Normal deltas, one per vertex, or empty.</summary>
    public Vector3[] NormalDeltas { get; set; } = Array.Empty<Vector3>();

    /// <summary>Tangent deltas, one per vertex, or empty.</summary>
    public Vector3[] TangentDeltas { get; set; } = Array.Empty<Vector3>();
}
