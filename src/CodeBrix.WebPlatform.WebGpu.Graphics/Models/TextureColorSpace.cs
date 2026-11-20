namespace CodeBrix.WebPlatform.WebGpu.Graphics.Models;

/// <summary>How the RGB channels of a <see cref="ModelTextureData" /> are encoded.</summary>
public enum TextureColorSpace
{
    /// <summary>Linear values (metallic-roughness, normal and occlusion textures).</summary>
    Linear = 0,

    /// <summary>sRGB-encoded colours (base colour and emissive textures).</summary>
    Srgb = 1,
}
