using System.Collections.Generic;
using System.Numerics;

namespace CodeBrix.WebPlatform.WebGpu.Graphics.Models;

/// <summary>A glTF skin of a <see cref="ModelData" />: joint nodes and inverse bind matrices.</summary>
public sealed class ModelSkinData
{
    /// <summary>The name, or null.</summary>
    public string Name { get; set; }

    /// <summary>The joint nodes, as positions in <see cref="ModelData.Nodes" />; <see cref="ModelSkinVertex" /> joint numbers index this list.</summary>
    public List<int> Joints { get; set; } = new List<int>();

    /// <summary>One inverse bind matrix per joint, or empty for identities (the writer then stores one identity per joint).</summary>
    public List<Matrix4x4> InverseBindMatrices { get; set; } = new List<Matrix4x4>();

    /// <summary>The skeleton root node, as a position in <see cref="ModelData.Nodes" />, or -1. Default -1.</summary>
    public int Skeleton { get; set; } = -1;
}
