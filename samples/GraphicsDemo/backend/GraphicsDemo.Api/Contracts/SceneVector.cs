using CodeBrix.WebPlatform.Core.Mapping;

namespace GraphicsDemo.Api.Contracts;

/// <summary>Three numbers: a position, a direction or a linear colour (red, green, blue).</summary>
[WireRequired]
public sealed class SceneVector
{
    /// <summary>X (or red).</summary>
    public float X { get; set; }

    /// <summary>Y (or green).</summary>
    public float Y { get; set; }

    /// <summary>Z (or blue).</summary>
    public float Z { get; set; }
}
