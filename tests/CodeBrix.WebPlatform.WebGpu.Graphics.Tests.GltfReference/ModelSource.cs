using System;

namespace CodeBrix.WebPlatform.WebGpu.Graphics.Tests.GltfReference;

/// <summary>One glTF model to convert into a set: its name in the set, its .gltf (or .glb) bytes, how its external files are read, and the key its images are shared under.</summary>
public sealed class ModelSource
{
    /// <summary>Creates a source.</summary>
    /// <param name="name">The model's name in the set.</param>
    /// <param name="gltf">The .gltf or .glb bytes.</param>
    /// <param name="resolve">Reads an external file by the relative URI the model names (null when it has none).</param>
    /// <param name="baseKey">The folder key its images resolve against: models of one folder that name one image share one texture (null: no sharing).</param>
    /// <exception cref="ArgumentNullException"><paramref name="name" /> or <paramref name="gltf" /> is null.</exception>
    public ModelSource(string name, byte[] gltf, Func<string, byte[]> resolve, string baseKey)
    {
        ArgumentNullException.ThrowIfNull(name);
        ArgumentNullException.ThrowIfNull(gltf);
        Name = name;
        Gltf = gltf;
        Resolve = resolve;
        BaseKey = baseKey;
    }

    /// <summary>The model's name in the set.</summary>
    public string Name { get; }

    /// <summary>The .gltf or .glb bytes.</summary>
    public byte[] Gltf { get; }

    /// <summary>Reads an external file by its relative URI, or null.</summary>
    public Func<string, byte[]> Resolve { get; }

    /// <summary>The folder key images resolve against, or null.</summary>
    public string BaseKey { get; }
}
