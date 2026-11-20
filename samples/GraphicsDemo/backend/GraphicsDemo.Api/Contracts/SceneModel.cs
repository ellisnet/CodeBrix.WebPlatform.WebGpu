using CodeBrix.WebPlatform.Core.Mapping;

namespace GraphicsDemo.Api.Contracts;

/// <summary>One model of the scene's model data blob.</summary>
[WireRequired]
public sealed class SceneModel
{
    /// <summary>The model's name, such as "tree" or "ground".</summary>
    public string Name { get; set; }

    /// <summary>The model's index in the blob (the order the blob holds the models in).</summary>
    public int Index { get; set; }

    /// <summary>The model's vertex count.</summary>
    public int VertexCount { get; set; }

    /// <summary>The model's triangle count.</summary>
    public int TriangleCount { get; set; }
}
