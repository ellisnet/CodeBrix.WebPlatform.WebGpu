namespace CodeBrix.WebPlatform.WebGpu.Graphics.Models;

/// <summary>A texture sampler of a <see cref="ModelData" /> (from a glTF sampler: wrap modes and filters). The defaults are glTF's when a texture names no sampler.</summary>
public sealed class ModelSamplerData
{
    /// <summary>The address mode across U (wrapS). Default <see cref="SamplerAddressMode.Repeat" />.</summary>
    public SamplerAddressMode AddressModeU { get; set; } = SamplerAddressMode.Repeat;

    /// <summary>The address mode across V (wrapT). Default <see cref="SamplerAddressMode.Repeat" />.</summary>
    public SamplerAddressMode AddressModeV { get; set; } = SamplerAddressMode.Repeat;

    /// <summary>The magnification filter. Default <see cref="SamplerFilter.Linear" />.</summary>
    public SamplerFilter MagFilter { get; set; } = SamplerFilter.Linear;

    /// <summary>The minification filter. Default <see cref="SamplerFilter.Linear" />.</summary>
    public SamplerFilter MinFilter { get; set; } = SamplerFilter.Linear;

    /// <summary>The filter between mip levels. Default <see cref="SamplerFilter.Linear" />.</summary>
    public SamplerFilter MipmapFilter { get; set; } = SamplerFilter.Linear;

    /// <summary>The highest mip level sampled: 0 for the glTF NEAREST and LINEAR minification filters (no mips), else 32. Default 32.</summary>
    public float MaxLod { get; set; } = 32f;
}
