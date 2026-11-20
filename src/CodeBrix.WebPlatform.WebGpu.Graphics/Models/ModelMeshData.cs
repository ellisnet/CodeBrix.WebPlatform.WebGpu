using System;
using System.Collections.Generic;

namespace CodeBrix.WebPlatform.WebGpu.Graphics.Models;

/// <summary>A glTF mesh of a <see cref="ModelData" />: its primitives and default morph weights.</summary>
public sealed class ModelMeshData
{
    /// <summary>The name, or null.</summary>
    public string Name { get; set; }

    /// <summary>The primitives (triangle lists).</summary>
    public List<ModelPrimitiveData> Primitives { get; set; } = new List<ModelPrimitiveData>();

    /// <summary>The mesh's default morph-target weights: one per morph target of each primitive (empty without morph targets).</summary>
    public float[] DefaultMorphWeights { get; set; } = Array.Empty<float>();
}
