namespace CodeBrix.WebPlatform.WebGpu.Graphics.Models;

/// <summary>How a <see cref="ModelMaterialData" /> treats the base colour's alpha (glTF alphaMode).</summary>
public enum MaterialAlphaMode
{
    /// <summary>OPAQUE: alpha is ignored.</summary>
    Opaque = 0,

    /// <summary>MASK: fragments whose alpha is below <see cref="ModelMaterialData.AlphaCutoff" /> are discarded, the rest are opaque.</summary>
    Mask = 1,

    /// <summary>BLEND: the base colour is alpha-blended.</summary>
    Blend = 2,
}
