using System;
using System.Collections.Generic;
using System.Security.Cryptography;

namespace CodeBrix.WebPlatform.WebGpu.Graphics.Tests.GltfReference;

/// <summary>
/// One model data blob (the CodeBrix model data format, written by ModelDataWriter): its bytes, a strong entity tag
/// (a hash of the bytes), what it holds and what it costs. The reference folder (<see cref="GltfReferenceFolder" />)
/// writes the bytes of one blob per case.
/// </summary>
public sealed class ModelBlob
{
    /// <summary>Wraps written bytes.</summary>
    /// <param name="bytes">The blob.</param>
    /// <param name="models">The models it holds, in blob order.</param>
    /// <param name="textureCount">The number of textures it holds.</param>
    /// <exception cref="ArgumentNullException">An argument is null.</exception>
    public ModelBlob(byte[] bytes, IReadOnlyList<ModelBlobEntry> models, int textureCount)
        : this(bytes, models, textureCount, null)
    {
    }

    /// <summary>Wraps written bytes with their statistics.</summary>
    /// <param name="bytes">The blob.</param>
    /// <param name="models">The models it holds, in blob order.</param>
    /// <param name="textureCount">The number of textures it holds.</param>
    /// <param name="statistics">What the blob costs, or null when unknown.</param>
    /// <exception cref="ArgumentNullException"><paramref name="bytes" /> or <paramref name="models" /> is null.</exception>
    public ModelBlob(byte[] bytes, IReadOnlyList<ModelBlobEntry> models, int textureCount, ModelBlobStatistics statistics)
    {
        ArgumentNullException.ThrowIfNull(bytes);
        ArgumentNullException.ThrowIfNull(models);
        Bytes = bytes;
        Models = models;
        TextureCount = textureCount;
        Statistics = statistics ?? new ModelBlobStatistics { BlobBytes = bytes.Length, TextureCount = textureCount, ModelCount = models.Count };
        ETag = "\"" + Convert.ToHexStringLower(SHA256.HashData(bytes)) + "\"";
    }

    /// <summary>The blob's bytes. Never modify them.</summary>
    public byte[] Bytes { get; }

    /// <summary>The blob's size in bytes.</summary>
    public int Length => Bytes.Length;

    /// <summary>A strong entity tag: the quoted lower-case hex SHA-256 of the bytes.</summary>
    public string ETag { get; }

    /// <summary>The models the blob holds, in blob order.</summary>
    public IReadOnlyList<ModelBlobEntry> Models { get; }

    /// <summary>The number of textures the blob holds (each stored once, however many models use it).</summary>
    public int TextureCount { get; }

    /// <summary>What the blob costs: bytes, geometry, textures, draws, the GPU memory estimate, the conversion time.</summary>
    public ModelBlobStatistics Statistics { get; }

    /// <summary>The entry of the model named <paramref name="name" />, or null.</summary>
    /// <param name="name">The model's name.</param>
    /// <returns>The entry, or null.</returns>
    public ModelBlobEntry Find(string name)
    {
        foreach (ModelBlobEntry entry in Models)
        {
            if (string.Equals(entry.Name, name, StringComparison.Ordinal)) return entry;
        }

        return null;
    }
}
