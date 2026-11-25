using System;
using System.Security.Cryptography;

namespace GraphicsDemo.Assets;

/// <summary>
/// One JPEG file served as it is (the "animation" frontend's crate decodes it with <c>ImageSource::from_jpeg</c>), with a
/// strong entity tag.
/// </summary>
public sealed class JpegFile
{
    /// <summary>Wraps the bytes of a JPEG file.</summary>
    /// <param name="bytes">The file's bytes.</param>
    /// <exception cref="ArgumentNullException"><paramref name="bytes" /> is null.</exception>
    /// <exception cref="ArgumentException">The bytes do not start with the JPEG start-of-image marker.</exception>
    public JpegFile(byte[] bytes)
    {
        ArgumentNullException.ThrowIfNull(bytes);
        if (bytes.Length < 4 || bytes[0] != 0xFF || bytes[1] != 0xD8)
        {
            throw new ArgumentException("The bytes are not a JPEG file (they do not start with the start-of-image marker FF D8).", nameof(bytes));
        }
        Bytes = bytes;
        ETag = "\"" + Convert.ToHexStringLower(SHA256.HashData(bytes)) + "\"";
    }

    /// <summary>The file's bytes. Never modify them: they are shared by every request.</summary>
    public byte[] Bytes { get; }

    /// <summary>A strong entity tag (the quoted lower-case hex SHA-256 of the bytes), for an HTTP ETag header.</summary>
    public string ETag { get; }
}
