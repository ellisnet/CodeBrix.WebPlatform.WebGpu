namespace GraphicsDemo.Assets.Conversion;

/// <summary>
/// Options for <see cref="GltfModelConverter" />. The defaults are those of the C# library's model loading options
/// (CodeBrix.WebGpu.Graphics ModelLoadOptions), so a converted model holds what that library's loader holds.
/// </summary>
public sealed class ModelConversionOptions
{
    /// <summary>True (default) to decode the material textures; false leaves every texture out.</summary>
    public bool LoadTextures { get; set; } = true;

    /// <summary>True (default) to mark every texture for a mip chain, which the reader generates on the GPU.</summary>
    public bool Mipmaps { get; set; } = true;

    /// <summary>
    /// True (default) to mark colour textures (base colour, emissive and the other colour channels) as sRGB, as glTF
    /// specifies; false marks them linear. Data textures (normal, metallic-roughness, occlusion) are always linear.
    /// </summary>
    public bool ColorTexturesSrgb { get; set; } = true;

    /// <summary>True (default) to generate tangents for primitives whose material has a normal map and whose file has no TANGENT.</summary>
    public bool GenerateTangents { get; set; } = true;

    /// <summary>True (default) to decode the images of a model on several threads.</summary>
    public bool DecodeTexturesInParallel { get; set; } = true;
}
