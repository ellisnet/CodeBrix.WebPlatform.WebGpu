using System;

namespace CodeBrix.WebPlatform.WebGpu.Graphics.Models;

/// <summary>
/// One decoded animation channel of a <see cref="ModelAnimationData" />: the target node and property, the key times
/// and the packed key values. <see cref="Values" /> holds <see cref="ComponentCount" /> floats per key (x3 for
/// <see cref="AnimationInterpolation.CubicSpline" />: in-tangent, value, out-tangent of each key in turn).
/// </summary>
public sealed class ModelAnimationChannelData
{
    /// <summary>The target node, as a position in <see cref="ModelData.Nodes" />.</summary>
    public int Node { get; set; }

    /// <summary>The animated property.</summary>
    public AnimationPath Path { get; set; } = AnimationPath.Translation;

    /// <summary>The interpolation. Default <see cref="AnimationInterpolation.Linear" />.</summary>
    public AnimationInterpolation Interpolation { get; set; } = AnimationInterpolation.Linear;

    /// <summary>Floats per value (3 translation / scale, 4 rotation, the morph-target count for weights).</summary>
    public int ComponentCount { get; set; }

    /// <summary>The key times in seconds (ascending).</summary>
    public float[] Times { get; set; } = Array.Empty<float>();

    /// <summary>The packed key values.</summary>
    public float[] Values { get; set; } = Array.Empty<float>();
}
