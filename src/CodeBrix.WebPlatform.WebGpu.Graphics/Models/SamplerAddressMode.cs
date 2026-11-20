namespace CodeBrix.WebPlatform.WebGpu.Graphics.Models;

/// <summary>What a <see cref="ModelSamplerData" /> does with texture coordinates outside 0..1 (glTF wrapS / wrapT).</summary>
public enum SamplerAddressMode
{
    /// <summary>REPEAT: the texture tiles.</summary>
    Repeat = 0,

    /// <summary>CLAMP_TO_EDGE: the edge texels extend outwards.</summary>
    ClampToEdge = 1,

    /// <summary>MIRRORED_REPEAT: the texture tiles, every other tile mirrored.</summary>
    MirrorRepeat = 2,
}
