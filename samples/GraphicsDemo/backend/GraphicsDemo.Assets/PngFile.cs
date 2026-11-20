using System;
using System.Security.Cryptography;

namespace GraphicsDemo.Assets;

/// <summary>
/// One PNG file served as it is stored (the "animation" frontend's crate decodes it with <c>ImageSource::from_png</c>),
/// with a strong entity tag.
/// </summary>
public sealed class PngFile
{
    /// <summary>Wraps the bytes of a PNG file.</summary>
    /// <param name="bytes">The file's bytes.</param>
    /// <exception cref="ArgumentNullException"><paramref name="bytes" /> is null.</exception>
    /// <exception cref="ArgumentException">The bytes do not start with the PNG signature.</exception>
    public PngFile(byte[] bytes)
    {
        ArgumentNullException.ThrowIfNull(bytes);
        if (bytes.Length < 8 || bytes[0] != 0x89 || bytes[1] != (byte)'P' || bytes[2] != (byte)'N' || bytes[3] != (byte)'G')
        {
            throw new ArgumentException("The bytes are not a PNG file (they do not start with the PNG signature).", nameof(bytes));
        }
        Bytes = bytes;
        ETag = "\"" + Convert.ToHexStringLower(SHA256.HashData(bytes)) + "\"";
    }

    /// <summary>The file's bytes. Never modify them: they are shared by every request.</summary>
    public byte[] Bytes { get; }

    /// <summary>A strong entity tag (the quoted lower-case hex SHA-256 of the bytes), for an HTTP ETag header.</summary>
    public string ETag { get; }
}
