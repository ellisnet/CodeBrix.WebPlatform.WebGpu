using System;
using System.Security.Cryptography;
using CodeBrix.WebPlatform.WebGpu.Animation.Pictures;

namespace GraphicsDemo.Assets;

/// <summary>
/// One picture as the "animation" frontend takes it: DECODED pixels in the Animation package's picture-pixels form
/// (<see cref="PicturePixelsFormat" />, written by <see cref="PicturePixelsWriter" />), which the animation crate's
/// <c>ImageSource::from_picture_pixels</c> reads without any image decoder in the browser: the magic "CBPX", the
/// major and minor version, the width and the height, then width x height x 4 bytes of straight-alpha RGBA
/// (sRGB-encoded, rows top to bottom, no padding).
/// </summary>
public sealed class PicturePixels
{
    /// <summary>The size of the header (magic, versions, width and height).</summary>
    public const int HeaderLength = PicturePixelsFormat.HeaderLength;

    /// <summary>Wraps decoded pixels in the pixel form.</summary>
    /// <param name="name">The picture's name (for example "cube-pets-colormap").</param>
    /// <param name="rgba">Straight-alpha RGBA pixels, width x height x 4 bytes.</param>
    /// <param name="width">The width in pixels.</param>
    /// <param name="height">The height in pixels.</param>
    /// <exception cref="ArgumentNullException">An argument is null.</exception>
    /// <exception cref="ArgumentException">The pixels do not match the size.</exception>
    public PicturePixels(string name, byte[] rgba, int width, int height)
    {
        ArgumentNullException.ThrowIfNull(name);
        ArgumentNullException.ThrowIfNull(rgba);
        if (width < 1 || height < 1 || rgba.Length != checked(width * height * 4))
        {
            throw new ArgumentException("The pixels must be width x height x 4 bytes of RGBA for a picture of at least 1 x 1.", nameof(rgba));
        }
        Name = name;
        Width = width;
        Height = height;
        byte[] bytes = PicturePixelsWriter.Write(rgba, width, height);
        Bytes = bytes;
        ETag = "\"" + Convert.ToHexStringLower(SHA256.HashData(bytes)) + "\"";
    }

    /// <summary>The picture's name.</summary>
    public string Name { get; }

    /// <summary>The width in pixels.</summary>
    public int Width { get; }

    /// <summary>The height in pixels.</summary>
    public int Height { get; }

    /// <summary>The pixel form (header, then the RGBA bytes). Never modify it: it is shared by every request.</summary>
    public byte[] Bytes { get; }

    /// <summary>A strong entity tag (the quoted lower-case hex SHA-256 of the bytes), for an HTTP ETag header.</summary>
    public string ETag { get; }
}
