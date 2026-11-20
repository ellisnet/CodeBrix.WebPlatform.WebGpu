using System;
using System.IO;
using CodeBrix.WebPlatform.WebGpu.Graphics.Models;
using GraphicsDemo.Assets.Conversion.Internal;
using Schema = CodeBrix.Graphics3D.Gltf2.Schema2;
using ValidationMode = CodeBrix.Graphics3D.Gltf2.Validation.ValidationMode;

namespace GraphicsDemo.Assets.Conversion;

/// <summary>
/// Converts a glTF 2.0 model (.glb bytes, or .gltf bytes whose external files the resolver supplies) read by
/// CodeBrix.Graphics3D.Gltf2 into a <see cref="ModelData" /> of a <see cref="ModelDataSet" />, holding what the C#
/// library's glTF loader holds: one vertex array and one index array for every primitive, materials, textures decoded
/// by CodeBrix.Imaging to RGBA8, samplers, nodes, skins, morph targets and animation clips. Runs on the CPU only; the
/// set then goes through <see cref="ModelDataWriter" /> to become one blob.
/// </summary>
public static class GltfModelConverter
{
    /// <summary>
    /// Reads and converts a model and adds it (and the textures it needs that the set does not hold yet) to
    /// <paramref name="set" />. A document that fails glTF validation is read once more without validation, and the
    /// validation message becomes a model warning (the C# library's default, TryFix).
    /// </summary>
    /// <param name="set">The set the model and its textures are added to.</param>
    /// <param name="gltf">The .glb or .gltf bytes.</param>
    /// <param name="name">The model's name.</param>
    /// <param name="resolveFile">
    /// Returns the bytes of an external file the model names by its relative URI (an external .bin or image such as
    /// "Textures/colormap.png"), or null when the model has no external files.
    /// </param>
    /// <param name="baseKey">
    /// The key external images resolve against for <paramref name="cache" /> (for example the pack's folder name);
    /// two models with the same base key that name the same relative image share one texture. Null = no sharing.
    /// </param>
    /// <param name="cache">The texture cache of <paramref name="set" />, or null.</param>
    /// <param name="options">The options (null for the defaults).</param>
    /// <returns>The model, which is now the last of <see cref="ModelDataSet.Models" />.</returns>
    /// <exception cref="ArgumentNullException"><paramref name="set" />, <paramref name="gltf" /> or <paramref name="name" /> is null.</exception>
    /// <exception cref="ArgumentException"><paramref name="cache" /> belongs to another set.</exception>
    /// <exception cref="InvalidDataException">The bytes are not a glTF 2.0 model the reader accepts (the message says why).</exception>
    public static ModelData Convert(ModelDataSet set, byte[] gltf, string name, Func<string, byte[]> resolveFile, string baseKey, ModelTextureCache cache, ModelConversionOptions options = null)
    {
        ArgumentNullException.ThrowIfNull(set);
        ArgumentNullException.ThrowIfNull(gltf);
        ArgumentNullException.ThrowIfNull(name);
        if (cache != null && !ReferenceEquals(cache.Set, set)) throw new ArgumentException("The texture cache belongs to another model data set. Create one cache per set and pass that set's cache.", nameof(cache));
        options ??= new ModelConversionOptions();
        Schema.ModelRoot root = Read(gltf, resolveFile, "'" + name + "'", out string validationWarning);
        GltfModelBuilder builder = new GltfModelBuilder(set, root, name, baseKey, cache, options);
        if (validationWarning != null) builder.Warn(validationWarning);
        ModelData model = builder.Build();
        set.Models.Add(model);
        return model;
    }

    /// <summary>Joins a base key and an image's relative URI into a cache key ('/' separators, percent escapes undone).</summary>
    /// <param name="baseKey">The base key.</param>
    /// <param name="relativeUri">The image's relative URI.</param>
    /// <returns>The key.</returns>
    public static string CombineKey(string baseKey, string relativeUri)
    {
        string path = Uri.UnescapeDataString(relativeUri).Replace('\\', '/');
        return baseKey.TrimEnd('/') + "/" + path.TrimStart('/');
    }

    private static Schema.ModelRoot Read(byte[] gltf, Func<string, byte[]> resolveFile, string what, out string validationWarning)
    {
        Schema.ModelRoot ReadWith(ValidationMode validation)
        {
            Schema.ReadContext context = Schema.ReadContext.Create(asset =>
            {
                byte[] bytes = resolveFile?.Invoke(asset);
                if (bytes == null) throw new FileNotFoundException("The model refers to the external file '" + asset + "', which the resolver does not supply. Add the file beside the model.");
                return new ArraySegment<byte>(bytes);
            });
            using MemoryStream stream = new MemoryStream(gltf, false);
            return context.WithSettingsFrom(new Schema.ReadSettings { Validation = validation }).ReadSchema2(stream);
        }

        validationWarning = null;
        try
        {
            return ReadWith(ValidationMode.TryFix);
        }
        catch (Exception exception) when (exception is not OutOfMemoryException)
        {
            if (exception is CodeBrix.Graphics3D.Gltf2.Validation.ModelException)
            {
                try
                {
                    Schema.ModelRoot root = ReadWith(ValidationMode.Skip);
                    validationWarning = "The file failed glTF validation and was read without it: " + exception.Message;
                    return root;
                }
                catch (Exception retry) when (retry is not OutOfMemoryException)
                {
                    throw new InvalidDataException("The glTF model " + what + " could not be read: " + exception.Message + " (reading it without validation failed too: " + retry.Message + ")", exception);
                }
            }

            throw new InvalidDataException("The glTF model " + what + " could not be read: " + exception.Message, exception);
        }
    }
}
