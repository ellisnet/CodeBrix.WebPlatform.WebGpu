using System;
using System.Collections.Generic;
using System.Security.Cryptography;

namespace GraphicsDemo.Assets;

/// <summary>
/// One model data blob (the CodeBrix model data format, written by ModelDataWriter), built once and kept: its bytes,
/// a strong entity tag for HTTP caching, and what it holds.
/// </summary>
public sealed class ModelBlob
{
    /// <summary>Wraps written bytes.</summary>
    /// <param name="bytes">The blob.</param>
    /// <param name="models">The models it holds, in blob order.</param>
    /// <param name="textureCount">The number of textures it holds.</param>
    /// <exception cref="ArgumentNullException">An argument is null.</exception>
    public ModelBlob(byte[] bytes, IReadOnlyList<ModelBlobEntry> models, int textureCount)
    {
        ArgumentNullException.ThrowIfNull(bytes);
        ArgumentNullException.ThrowIfNull(models);
        Bytes = bytes;
        Models = models;
        TextureCount = textureCount;
        ETag = "\"" + Convert.ToHexStringLower(SHA256.HashData(bytes)) + "\"";
    }

    /// <summary>The blob's bytes. Never modify them: they are shared by every request.</summary>
    public byte[] Bytes { get; }

    /// <summary>The blob's size in bytes.</summary>
    public int Length => Bytes.Length;

    /// <summary>A strong entity tag (the quoted lower-case hex SHA-256 of the bytes), for an HTTP ETag header.</summary>
    public string ETag { get; }

    /// <summary>The models the blob holds, in blob order.</summary>
    public IReadOnlyList<ModelBlobEntry> Models { get; }

    /// <summary>The number of textures the blob holds (each stored once, however many models use it).</summary>
    public int TextureCount { get; }
}
