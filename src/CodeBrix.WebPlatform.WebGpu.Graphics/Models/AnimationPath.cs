namespace CodeBrix.WebPlatform.WebGpu.Graphics.Models;

/// <summary>Which node property a <see cref="ModelAnimationChannelData" /> drives.</summary>
public enum AnimationPath
{
    /// <summary>The translation (3 floats per key).</summary>
    Translation = 0,

    /// <summary>The rotation (4 floats per key: x, y, z, w).</summary>
    Rotation = 1,

    /// <summary>The scale (3 floats per key).</summary>
    Scale = 2,

    /// <summary>The morph weights (one float per morph target per key).</summary>
    Weights = 3,
}
