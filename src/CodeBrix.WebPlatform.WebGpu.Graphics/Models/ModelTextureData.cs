namespace CodeBrix.WebPlatform.WebGpu.Graphics.Models;

/// <summary>One decoded texture of a <see cref="ModelDataSet" />: RGBA8 pixels, rows top to bottom, no row padding.</summary>
public sealed class ModelTextureData
{
    /// <summary>A description (for example the image's URI or "image 0"), or null.</summary>
    public string Name { get; set; }

    /// <summary>The width in pixels (1 to <see cref="ModelDataFormat.MaxTextureDimension" />).</summary>
    public int Width { get; set; }

    /// <summary>The height in pixels (1 to <see cref="ModelDataFormat.MaxTextureDimension" />).</summary>
    public int Height { get; set; }

    /// <summary>How the RGB channels are encoded. Default <see cref="TextureColorSpace.Linear" />.</summary>
    public TextureColorSpace ColorSpace { get; set; } = TextureColorSpace.Linear;

    /// <summary>True when the reader generates the mip chain; false for level 0 only. Default true.</summary>
    public bool Mipmaps { get; set; } = true;

    /// <summary>The pixels: <see cref="Width" /> x <see cref="Height" /> x 4 bytes (R, G, B, A), rows top to bottom.</summary>
    public byte[] Pixels { get; set; }
}
