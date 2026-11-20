namespace CodeBrix.WebPlatform.WebGpu.Graphics.Models;

/// <summary>How a <see cref="ModelAnimationChannelData" /> interpolates between keys.</summary>
public enum AnimationInterpolation
{
    /// <summary>Hold each key's value until the next key.</summary>
    Step = 0,

    /// <summary>Linear (rotations: spherical linear).</summary>
    Linear = 1,

    /// <summary>Cubic Hermite spline; each key stores in-tangent, value and out-tangent.</summary>
    CubicSpline = 2,
}
