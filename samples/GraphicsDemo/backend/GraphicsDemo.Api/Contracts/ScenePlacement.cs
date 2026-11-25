using CodeBrix.WebPlatform.Core.Mapping;

namespace GraphicsDemo.Api.Contracts;

/// <summary>
/// One placed model of the scene. Its world matrix is the uniform scale, then the rotation about Y, then the
/// translation (System.Numerics order: Matrix4x4.CreateScale(scale) * CreateRotationY(rotationY) * CreateTranslation(translation)).
/// </summary>
[WireRequired]
public sealed class ScenePlacement
{
    /// <summary>The model's name.</summary>
    public string Model { get; set; }

    /// <summary>The model's index in the scene's list of models (<see cref="ViewerScene.Models" />).</summary>
    public int ModelIndex { get; set; }

    /// <summary>The translation.</summary>
    public SceneVector Translation { get; set; }

    /// <summary>The rotation about Y, in radians.</summary>
    public float RotationY { get; set; }

    /// <summary>The uniform scale.</summary>
    public float Scale { get; set; }
}
