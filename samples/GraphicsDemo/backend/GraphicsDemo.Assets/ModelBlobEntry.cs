using System.Collections.Generic;

namespace GraphicsDemo.Assets;

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
    {
        Name = name;
        Index = index;
        VertexCount = vertexCount;
        TriangleCount = triangleCount;
        Warnings = warnings;
    }

    /// <summary>The model's name.</summary>
    public string Name { get; }

    /// <summary>The model's index in the blob.</summary>
    public int Index { get; }

    /// <summary>The model's vertex count.</summary>
    public int VertexCount { get; }

    /// <summary>The model's triangle count.</summary>
    public int TriangleCount { get; }

    /// <summary>The warnings of the model's conversion (empty when there were none).</summary>
    public IReadOnlyList<string> Warnings { get; }
}
