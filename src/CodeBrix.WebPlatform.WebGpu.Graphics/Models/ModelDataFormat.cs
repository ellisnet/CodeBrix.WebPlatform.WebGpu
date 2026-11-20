using System;

namespace CodeBrix.WebPlatform.WebGpu.Graphics.Models;

/// <summary>
/// The constants of the CodeBrix model data format (a ".cbmd" blob): what <see cref="ModelDataWriter" /> writes and
/// the crate codebrix-webgpu-graphics reads (module <c>models</c>). One blob is a model set: one or more models and
/// the textures they use, every texture stored once.
/// </summary>
public static class ModelDataFormat
{
    /// <summary>The major version the writer writes; a reader refuses a blob of another major version.</summary>
    public const int MajorVersion = 1;

    /// <summary>The minor version the writer writes; a reader accepts a higher minor version and ignores what it does not know.</summary>
    public const int MinorVersion = 0;

    /// <summary>The file extension of a blob, with its dot.</summary>
    public const string FileExtension = ".cbmd";

    /// <summary>The content type a server sends a blob with.</summary>
    public const string ContentType = "application/octet-stream";

    /// <summary>The largest width or height of a <see cref="ModelTextureData" />, in pixels.</summary>
    public const int MaxTextureDimension = 8192;

    /// <summary>The four bytes every blob starts with: "CBMD" in ASCII.</summary>
    public static ReadOnlySpan<byte> Magic => "CBMD"u8;
}
