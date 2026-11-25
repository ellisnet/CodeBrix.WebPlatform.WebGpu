namespace CodeBrix.WebPlatform.WebGpu.Graphics.Tests.GltfReference;

/// <summary>
/// Options for <see cref="GltfModelConverter" />. The defaults are those of the C# library's model loading options
/// (CodeBrix.WebGpu.Graphics ModelLoadOptions), so a converted model holds what that library's loader holds. The
/// reference folder (<see cref="GltfReferenceFolder" />) names the options of each case so that the Rust glTF loader
/// is checked with the same ones.
/// </summary>
public sealed class ModelConversionOptions
{
    /// <summary>True (default) to decode the material textures; false leaves every texture out.</summary>
    public bool LoadTextures { get; set; } = true;

    /// <summary>True (default) to mark every texture for a mip chain, which the reader generates (the graphics crate makes it on the CPU).</summary>
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

    /// <summary>
    /// The largest side a texture may have, in pixels; 0 (default) keeps every image at its own size. A larger image
    /// is halved (both sides, so its aspect stays exact) until both sides fit, with CodeBrix.Imaging's default
    /// resampler, before it is stored. Decoded textures travel and sit in GPU memory as uncompressed RGBA8, so this is
    /// the main lever on a model's size: 2048 x 2048 is 16 MiB, 1024 x 1024 4 MiB.
    /// </summary>
    public int MaxTextureSize { get; set; }

    /// <summary>A short text that names every option, for cache keys (two option sets with the same text convert alike).</summary>
    /// <returns>The text.</returns>
    public string Describe() => "textures=" + LoadTextures + ";mipmaps=" + Mipmaps + ";srgb=" + ColorTexturesSrgb + ";tangents=" + GenerateTangents + ";max=" + MaxTextureSize;
}
