using System;
using System.Collections.Generic;
using System.IO;
using CodeBrix.WebPlatform.WebGpu.Graphics.Models.Internal;

namespace CodeBrix.WebPlatform.WebGpu.Graphics.Models;

/// <summary>
/// Writes a <see cref="ModelDataSet" /> as a model data blob (<see cref="ModelDataFormat" />, a ".cbmd" file) for
/// the crate codebrix-webgpu-graphics to read in the browser: a 32-byte header, a section table and every section of
/// the format in its order (also the empty ones), each section and each model's or texture's range in the bulk
/// sections at a multiple of 16 bytes, zero padding, little-endian whatever the host's byte order. The same set always
/// gives the same bytes.
/// </summary>
/// <remarks>
/// The writer checks the set first: every index a record holds must be in range, every array must have the length the
/// format needs, and the blob must fit in 4 GiB - 1 bytes. A failed check throws before anything is written, with a
/// message that says what is wrong and what to do. Null lists and arrays count as empty, a null texture slot as an
/// empty slot. An empty <see cref="ModelSkinData.InverseBindMatrices" /> list is written as one identity per joint, and
/// a node without <see cref="ModelNodeData.HasFixedMatrix" /> gets the matrix of its translation, rotation and scale.
/// </remarks>
public static class ModelDataWriter
{
    /// <summary>Writes a model set to a new byte array.</summary>
    /// <param name="set">The set.</param>
    /// <returns>The blob.</returns>
    /// <exception cref="ArgumentNullException"><paramref name="set" /> is null.</exception>
    /// <exception cref="ArgumentException">Something in the set is out of range or of the wrong length (the message says what).</exception>
    /// <exception cref="InvalidOperationException">The blob would be larger than 4 GiB - 1 bytes, or larger than an array can be (write it to a stream instead).</exception>
    public static byte[] Write(ModelDataSet set)
    {
        var encoder = Encode(set);
        if (encoder.TotalLength > Array.MaxLength)
        {
            throw new InvalidOperationException("The model set takes " + encoder.TotalLength + " bytes, more than a byte array can hold; write it with ModelDataWriter.Write(set, stream) instead.");
        }

        using var memory = new MemoryStream((int)encoder.TotalLength);
        encoder.WriteTo(memory);
        return memory.ToArray();
    }

    /// <summary>Writes one model and the textures it uses as a set of one.</summary>
    /// <param name="model">The model; its <see cref="ModelData.Textures" /> are positions in <paramref name="textures" />.</param>
    /// <param name="textures">The textures the model uses, or null for none.</param>
    /// <returns>The blob.</returns>
    /// <exception cref="ArgumentNullException"><paramref name="model" /> is null.</exception>
    /// <exception cref="ArgumentException">Something in the model is out of range or of the wrong length (the message says what).</exception>
    /// <exception cref="InvalidOperationException">The blob would be larger than 4 GiB - 1 bytes, or larger than an array can be.</exception>
    public static byte[] Write(ModelData model, IReadOnlyList<ModelTextureData> textures)
    {
        ArgumentNullException.ThrowIfNull(model);
        var set = new ModelDataSet();
        set.Models.Add(model);
        if (textures != null) set.Textures.AddRange(textures);
        return Write(set);
    }

    /// <summary>Writes a model set to a stream (the stream is left open; nothing is written when a check fails).</summary>
    /// <param name="set">The set.</param>
    /// <param name="destination">The stream.</param>
    /// <exception cref="ArgumentNullException"><paramref name="set" /> or <paramref name="destination" /> is null.</exception>
    /// <exception cref="ArgumentException">Something in the set is out of range or of the wrong length (the message says what).</exception>
    /// <exception cref="InvalidOperationException">The blob would be larger than 4 GiB - 1 bytes.</exception>
    public static void Write(ModelDataSet set, Stream destination)
    {
        ArgumentNullException.ThrowIfNull(destination);
        Encode(set).WriteTo(destination);
    }

    private static ModelDataEncoder Encode(ModelDataSet set)
    {
        ArgumentNullException.ThrowIfNull(set);
        ModelDataValidator.Validate(set);
        var encoder = new ModelDataEncoder(set);
        CheckLength(encoder.TotalLength);
        return encoder;
    }

    /// <summary>Refuses a blob longer than the format's 32-bit offsets reach.</summary>
    /// <param name="totalLength">The blob's length in bytes.</param>
    /// <exception cref="InvalidOperationException">The length is over 4 GiB - 1 bytes.</exception>
    internal static void CheckLength(long totalLength)
    {
        if (totalLength > ModelDataSection.MaxBlobLength)
        {
            throw new InvalidOperationException("The model set takes " + totalLength + " bytes, but a model data blob holds at most " + ModelDataSection.MaxBlobLength + " (4 GiB - 1); split the models into several sets or use smaller textures.");
        }
    }
}
