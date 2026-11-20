using System.Numerics;

namespace CodeBrix.WebPlatform.WebGpu.Graphics.Models;

/// <summary>
/// One texture reference of a <see cref="ModelMaterialData" />: the texture, its sampler, the texture-coordinate set
/// and the KHR_texture_transform (identity when absent; its texCoord override is already applied to
/// <see cref="TexCoord" />).
/// </summary>
public sealed class ModelTextureSlotData
{
    /// <summary>The texture, as a position in <see cref="ModelData.Textures" />, or -1 when the slot is empty or its image could not be decoded. Default -1.</summary>
    public int Texture { get; set; } = -1;

    /// <summary>The sampler, as a position in <see cref="ModelData.Samplers" />, or -1 (always -1 when <see cref="Texture" /> is -1). Default -1.</summary>
    public int Sampler { get; set; } = -1;

    /// <summary>The texture-coordinate set (0 = TEXCOORD_0, 1 = TEXCOORD_1).</summary>
    public int TexCoord { get; set; }

    /// <summary>True when the file gave a KHR_texture_transform.</summary>
    public bool HasTransform { get; set; }

    /// <summary>The texture-coordinate transform (<c>Vector2.Transform(uv, Transform)</c>), identity when none.</summary>
    public Matrix3x2 Transform { get; set; } = Matrix3x2.Identity;

    /// <summary>The glTF texture index, or -1 for an empty slot. Default -1.</summary>
    public int TextureIndex { get; set; } = -1;

    /// <summary>A new empty slot (no texture, no sampler, set 0, identity transform).</summary>
    public static ModelTextureSlotData Empty => new ModelTextureSlotData();
}
