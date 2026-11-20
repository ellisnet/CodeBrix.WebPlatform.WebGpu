namespace CodeBrix.WebPlatform.WebGpu.Graphics.Models;

/// <summary>How a <see cref="ModelSamplerData" /> filters texels or mip levels.</summary>
public enum SamplerFilter
{
    /// <summary>The nearest texel (or mip level).</summary>
    Nearest = 0,

    /// <summary>Linear interpolation between texels (or mip levels).</summary>
    Linear = 1,
}
