using System;
using System.Security.Cryptography;

namespace GraphicsDemo.Assets;

/// <summary>
/// One environment panorama file the backend serves as it is (a Radiance .hdr file, decoded in the browser by the
/// graphics crate): its name, its bytes and a strong entity tag for HTTP caching.
/// </summary>
public sealed class EnvironmentFile
{
    /// <summary>Wraps a file's bytes.</summary>
    /// <param name="name">The environment's name (for example "small-cathedral").</param>
    /// <param name="bytes">The file's bytes.</param>
    /// <exception cref="ArgumentNullException">An argument is null.</exception>
    public EnvironmentFile(string name, byte[] bytes)
    {
        ArgumentNullException.ThrowIfNull(name);
        ArgumentNullException.ThrowIfNull(bytes);
        Name = name;
        Bytes = bytes;
        ETag = "\"" + Convert.ToHexStringLower(SHA256.HashData(bytes)) + "\"";
    }

    /// <summary>The environment's name.</summary>
    public string Name { get; }

    /// <summary>The file's bytes. Never modify them: they are shared by every request.</summary>
    public byte[] Bytes { get; }

    /// <summary>The file's size in bytes.</summary>
    public int Length => Bytes.Length;

    /// <summary>A strong entity tag (the quoted lower-case hex SHA-256 of the bytes), for an HTTP ETag header.</summary>
    public string ETag { get; }
}
