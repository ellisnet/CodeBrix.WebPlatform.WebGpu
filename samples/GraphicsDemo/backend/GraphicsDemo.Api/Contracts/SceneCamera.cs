using CodeBrix.WebPlatform.Core.Mapping;

namespace GraphicsDemo.Api.Contracts;

/// <summary>The scene's camera: a perspective camera on an orbit that turns on its own (yaw = YawStart + YawPerSecond x seconds).</summary>
[WireRequired]
public sealed class SceneCamera
{
    /// <summary>The vertical field of view, in degrees.</summary>
    public float FieldOfViewDegrees { get; set; }

    /// <summary>The near plane.</summary>
    public float NearPlane { get; set; }

    /// <summary>The far plane.</summary>
    public float FarPlane { get; set; }

    /// <summary>The point the camera orbits and looks at.</summary>
    public SceneVector Target { get; set; }

    /// <summary>The distance from the target.</summary>
    public float Distance { get; set; }

    /// <summary>The pitch, in radians.</summary>
    public float Pitch { get; set; }

    /// <summary>The yaw at time 0, in radians.</summary>
    public float YawStart { get; set; }

    /// <summary>How fast the yaw turns, in radians per second.</summary>
    public float YawPerSecond { get; set; }
}
