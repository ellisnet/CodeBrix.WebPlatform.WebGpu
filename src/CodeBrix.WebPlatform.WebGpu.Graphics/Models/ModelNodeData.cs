using System;
using System.Collections.Generic;
using System.Numerics;

namespace CodeBrix.WebPlatform.WebGpu.Graphics.Models;

/// <summary>A node of a <see cref="ModelData" />'s hierarchy, in its rest pose.</summary>
public sealed class ModelNodeData
{
    /// <summary>The name, or null.</summary>
    public string Name { get; set; }

    /// <summary>The parent node, as a position in <see cref="ModelData.Nodes" />, or -1 for a root. Default -1.</summary>
    public int Parent { get; set; } = -1;

    /// <summary>The child nodes, as positions in <see cref="ModelData.Nodes" />.</summary>
    public List<int> Children { get; set; } = new List<int>();

    /// <summary>The mesh, as a position in <see cref="ModelData.Meshes" />, or -1. Default -1.</summary>
    public int Mesh { get; set; } = -1;

    /// <summary>The skin, as a position in <see cref="ModelData.Skins" />, or -1. Default -1.</summary>
    public int Skin { get; set; } = -1;

    /// <summary>The rest-pose translation (from the file's TRS, or the decomposed matrix).</summary>
    public Vector3 Translation { get; set; }

    /// <summary>The rest-pose rotation (unit quaternion). Default identity.</summary>
    public Quaternion Rotation { get; set; } = Quaternion.Identity;

    /// <summary>The rest-pose scale. Default one.</summary>
    public Vector3 Scale { get; set; } = Vector3.One;

    /// <summary>
    /// True when the file gave a matrix that does not decompose into translation, rotation and scale (for example a
    /// shear): the node then uses <see cref="LocalMatrix" /> as is.
    /// </summary>
    public bool HasFixedMatrix { get; set; }

    /// <summary>
    /// The rest-pose local matrix, used when <see cref="HasFixedMatrix" /> is true. When it is false the writer stores
    /// the matrix of <see cref="Translation" />, <see cref="Rotation" /> and <see cref="Scale" /> instead (scale, then
    /// rotation, then translation, System.Numerics row-vector order). Default identity.
    /// </summary>
    public Matrix4x4 LocalMatrix { get; set; } = Matrix4x4.Identity;

    /// <summary>The node's default morph weights (the node's own, else its mesh's; empty without morph targets).</summary>
    public float[] DefaultMorphWeights { get; set; } = Array.Empty<float>();

    /// <summary>True when the node is in the model's scene (drawn); false for nodes only other scenes use.</summary>
    public bool InScene { get; set; }
}
