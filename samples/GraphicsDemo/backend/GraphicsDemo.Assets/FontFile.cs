using System;
using System.Security.Cryptography;

namespace GraphicsDemo.Assets;

/// <summary>
/// One font file the backend serves as it is (a TrueType or OpenType file, read and shaped in the browser by the animation
/// crate): its name, its bytes and a strong entity tag for HTTP caching.
/// </summary>
public sealed class FontFile
{
    /// <summary>Wraps a font file's bytes.</summary>
    /// <param name="name">The font's name: its file name in lower case (for example "roboto-regular.ttf").</param>
    /// <param name="bytes">The file's bytes.</param>
    /// <param name="contentType">The file's content type ("font/ttf" or "font/otf").</param>
    /// <exception cref="ArgumentNullException">An argument is null.</exception>
    public FontFile(string name, byte[] bytes, string contentType)
    {
        ArgumentNullException.ThrowIfNull(name);
        ArgumentNullException.ThrowIfNull(bytes);
        ArgumentNullException.ThrowIfNull(contentType);
        Name = name;
        Bytes = bytes;
        ContentType = contentType;
        ETag = "\"" + Convert.ToHexStringLower(SHA256.HashData(bytes)) + "\"";
    }

    /// <summary>The font's name.</summary>
    public string Name { get; }

    /// <summary>The file's bytes. Never modify them: they are shared by every request.</summary>
    public byte[] Bytes { get; }

    /// <summary>The file's content type ("font/ttf" or "font/otf").</summary>
    public string ContentType { get; }

    /// <summary>The file's size in bytes.</summary>
    public int Length => Bytes.Length;

    /// <summary>A strong entity tag (the quoted lower-case hex SHA-256 of the bytes), for an HTTP ETag header.</summary>
    public string ETag { get; }
}
