using System;
using System.Collections.Generic;

namespace CodeBrix.WebPlatform.WebGpu.Graphics.Tests.GltfReference;

/// <summary>One model of a <see cref="ModelBlob" />: its name, its index in the blob and its size.</summary>
public sealed class ModelBlobEntry
{
    /// <summary>Creates an entry.</summary>
    /// <param name="name">The model's name.</param>
    /// <param name="index">The model's index in the blob.</param>
    /// <param name="vertexCount">The model's vertex count.</param>
    /// <param name="triangleCount">The model's triangle count (every primitive's indices / 3).</param>
    /// <param name="warnings">The warnings of the model's conversion.</param>
    public ModelBlobEntry(string name, int index, int vertexCount, int triangleCount, IReadOnlyList<string> warnings)
        : this(name, index, vertexCount, triangleCount, 0, 0, Array.Empty<string>(), warnings)
    {
    }

    /// <summary>Creates an entry with the model's primitives and clips.</summary>
    /// <param name="name">The model's name.</param>
    /// <param name="index">The model's index in the blob.</param>
    /// <param name="vertexCount">The model's vertex count.</param>
    /// <param name="triangleCount">The model's triangle count (every primitive's indices / 3).</param>
    /// <param name="primitiveCount">The model's primitives.</param>
    /// <param name="blendedPrimitiveCount">The model's primitives whose material blends (alpha BLEND).</param>
    /// <param name="animations">The names of the model's animation clips, in model order.</param>
    /// <param name="warnings">The warnings of the model's conversion.</param>
    public ModelBlobEntry(string name, int index, int vertexCount, int triangleCount, int primitiveCount, int blendedPrimitiveCount, IReadOnlyList<string> animations, IReadOnlyList<string> warnings)
    {
        Name = name;
        Index = index;
        VertexCount = vertexCount;
        TriangleCount = triangleCount;
        PrimitiveCount = primitiveCount;
        BlendedPrimitiveCount = blendedPrimitiveCount;
        Animations = animations ?? Array.Empty<string>();
        Warnings = warnings ?? Array.Empty<string>();
    }

    /// <summary>The model's name.</summary>
    public string Name { get; }

    /// <summary>The model's index in the blob.</summary>
    public int Index { get; }

    /// <summary>The model's vertex count.</summary>
    public int VertexCount { get; }

    /// <summary>The model's triangle count.</summary>
    public int TriangleCount { get; }

    /// <summary>The model's primitives (each at least one draw call per frame).</summary>
    public int PrimitiveCount { get; }

    /// <summary>The model's blended primitives (one draw call per instance per frame each, drawn after the opaque ones).</summary>
    public int BlendedPrimitiveCount { get; }

    /// <summary>True when the model has a blended primitive: it belongs in the TRANSLUCENT pass, after the skybox.</summary>
    public bool IsTranslucent => BlendedPrimitiveCount > 0;

    /// <summary>The names of the model's animation clips (a clip without a name is listed as an empty string).</summary>
    public IReadOnlyList<string> Animations { get; }

    /// <summary>The warnings of the model's conversion (empty when there were none).</summary>
    public IReadOnlyList<string> Warnings { get; }
}
