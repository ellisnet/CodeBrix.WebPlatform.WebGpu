using System;
using System.Collections.Generic;
using System.Diagnostics;
using System.Text.Json;
using CodeBrix.WebPlatform.WebGpu.Graphics.Models;

namespace CodeBrix.WebPlatform.WebGpu.Graphics.Tests.GltfReference;

/// <summary>
/// Converts several glTF models into ONE model data set - textures shared through one <see cref="ModelTextureCache" />,
/// so an image several models name travels once - and writes it as one blob. Each case of the reference folder
/// (<see cref="GltfReferenceFolder" />) is one call.
/// </summary>
public static class ModelSetConverter
{
    /// <summary>Converts the sources into one blob.</summary>
    /// <param name="sources">The models, in blob order.</param>
    /// <param name="options">The conversion options (null for the defaults).</param>
    /// <returns>The blob.</returns>
    /// <exception cref="ArgumentNullException"><paramref name="sources" /> is null.</exception>
    /// <exception cref="System.IO.InvalidDataException">A model is not a glTF 2.0 model the reader accepts (the message says why).</exception>
    public static ModelBlob Convert(IReadOnlyList<ModelSource> sources, ModelConversionOptions options)
    {
        ArgumentNullException.ThrowIfNull(sources);
        options ??= new ModelConversionOptions();
        Stopwatch watch = Stopwatch.StartNew();
        ModelDataSet set = new ModelDataSet();
        ModelTextureCache cache = new ModelTextureCache(set);
        foreach (ModelSource source in sources)
        {
            GltfModelConverter.Convert(set, source.Gltf, source.Name, source.Resolve, source.BaseKey, cache, options);
        }

        return ModelBlobWriter.Write(set, watch.Elapsed.TotalMilliseconds);
    }

    /// <summary>The relative URIs of a .gltf's buffers and images (none for a .glb, even one that names external images, and none for a data URI).</summary>
    /// <param name="gltf">The file's bytes.</param>
    /// <returns>The URIs, in file order.</returns>
    public static IReadOnlyList<string> ExternalUris(byte[] gltf)
    {
        ArgumentNullException.ThrowIfNull(gltf);
        List<string> uris = new List<string>();
        if (gltf.Length == 0 || gltf[0] != (byte)'{') return uris;
        using JsonDocument document = JsonDocument.Parse(gltf);
        foreach (string collection in new[] { "buffers", "images" })
        {
            if (!document.RootElement.TryGetProperty(collection, out JsonElement items) || items.ValueKind != JsonValueKind.Array) continue;
            foreach (JsonElement item in items.EnumerateArray())
            {
                if (item.TryGetProperty("uri", out JsonElement uri) && uri.ValueKind == JsonValueKind.String && !uri.GetString().StartsWith("data:", StringComparison.Ordinal)) uris.Add(uri.GetString());
            }
        }

        return uris;
    }
}
