using System;
using System.Collections.Generic;
using CodeBrix.WebPlatform.WebGpu.Graphics.Models;

namespace GraphicsDemo.Assets.Conversion;

/// <summary>
/// The textures of one <see cref="ModelDataSet" /> that came from external image files, so that several models
/// converted into the same set share them: an image file (for example a kit's shared Textures/colormap.png) in one
/// colour space is decoded once and stored ONCE in <see cref="ModelDataSet.Textures" />, and every model that names it
/// refers to that one entry. The C# library's ModelTextureCache does the same for GPU textures. Not thread-safe: one
/// set is built on one thread.
/// </summary>
public sealed class ModelTextureCache
{
    private readonly Dictionary<(string Key, TextureColorSpace ColorSpace, bool Mipmaps), int> textures = new Dictionary<(string, TextureColorSpace, bool), int>();

    /// <summary>Creates a cache for one set.</summary>
    /// <param name="set">The set whose textures the cache adds and finds.</param>
    /// <exception cref="ArgumentNullException"><paramref name="set" /> is null.</exception>
    public ModelTextureCache(ModelDataSet set)
    {
        ArgumentNullException.ThrowIfNull(set);
        Set = set;
    }

    /// <summary>The set the cached textures belong to.</summary>
    public ModelDataSet Set { get; }

    /// <summary>The number of cached textures.</summary>
    public int Count => textures.Count;

    /// <summary>Looks a texture up.</summary>
    /// <param name="key">The image file's key (its normalised path; see <see cref="GltfModelConverter" />).</param>
    /// <param name="colorSpace">The colour space.</param>
    /// <param name="mipmaps">Whether the texture is marked for mipmaps.</param>
    /// <param name="index">The texture's index in <see cref="ModelDataSet.Textures" />.</param>
    /// <returns>True when the texture is cached.</returns>
    public bool TryGet(string key, TextureColorSpace colorSpace, bool mipmaps, out int index) => textures.TryGetValue((key, colorSpace, mipmaps), out index);

    /// <summary>Adds a texture to the set and remembers it under its key.</summary>
    /// <param name="key">The image file's key.</param>
    /// <param name="texture">The texture.</param>
    /// <returns>The texture's index in <see cref="ModelDataSet.Textures" />.</returns>
    /// <exception cref="ArgumentNullException">An argument is null.</exception>
    public int Add(string key, ModelTextureData texture)
    {
        ArgumentNullException.ThrowIfNull(key);
        ArgumentNullException.ThrowIfNull(texture);
        int index = Set.Textures.Count;
        Set.Textures.Add(texture);
        textures[(key, texture.ColorSpace, texture.Mipmaps)] = index;
        return index;
    }
}
