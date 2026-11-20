using System;

namespace CodeBrix.WebPlatform.WebGpu.Animation.Pictures;

/// <summary>
/// The constants of the picture-pixels form: DECODED picture pixels as <see cref="PicturePixelsWriter" /> writes them
/// on a backend and the crate codebrix-webgpu-animation reads them in the browser (<c>ImageSource::from_picture_pixels</c>).
/// The layout: the magic "CBPX" (4 bytes), the major and the minor version (two unsigned 16-bit little-endian
/// integers), the width and the height in pixels (two unsigned 32-bit little-endian integers), then width x height x 4
/// bytes of straight-alpha RGBA (sRGB-encoded, rows top to bottom, no padding) - <see cref="HeaderLength" /> +
/// width x height x 4 bytes in all.
/// </summary>
public static class PicturePixelsFormat
{
    /// <summary>The major version the writer writes; a reader refuses another major version.</summary>
    public const int MajorVersion = 1;

    /// <summary>The minor version the writer writes; a reader accepts a higher minor version and ignores what follows the pixels.</summary>
    public const int MinorVersion = 0;

    /// <summary>The length of the header (magic, versions, width, height), in bytes.</summary>
    public const int HeaderLength = 16;

    /// <summary>The largest width or height of a picture, in pixels (a GPU texture's limit in every browser with WebGPU).</summary>
    public const int MaxDimension = 8192;

    /// <summary>The content type a server sends the form with.</summary>
    public const string ContentType = "application/octet-stream";

    /// <summary>The four bytes every picture starts with: "CBPX" in ASCII.</summary>
    public static ReadOnlySpan<byte> Magic => "CBPX"u8;
}
