using System;
using System.Collections.Generic;
using CodeBrix.WebPlatform.WebGpu.Graphics.Models;

namespace CodeBrix.WebPlatform.WebGpu.Graphics.Tests.GltfReference;

/// <summary>Writes a <see cref="ModelDataSet" /> as one model data blob (ModelDataWriter) and records what it holds and costs.</summary>
public static class ModelBlobWriter
{
    /// <summary>Writes a set.</summary>
    /// <param name="set">The set.</param>
    /// <returns>The blob, with one entry per model in set order.</returns>
    /// <exception cref="ArgumentNullException"><paramref name="set" /> is null.</exception>
    public static ModelBlob Write(ModelDataSet set) => Write(set, 0);

    /// <summary>Writes a set and records how long its conversion took.</summary>
    /// <param name="set">The set.</param>
    /// <param name="conversionMilliseconds">The milliseconds the set's conversion took (recorded in the statistics).</param>
    /// <returns>The blob, with one entry per model in set order.</returns>
    /// <exception cref="ArgumentNullException"><paramref name="set" /> is null.</exception>
    public static ModelBlob Write(ModelDataSet set, double conversionMilliseconds)
    {
        ArgumentNullException.ThrowIfNull(set);
        byte[] bytes = ModelDataWriter.Write(set);
        ModelBlobStatistics statistics = new ModelBlobStatistics { BlobBytes = bytes.Length, TextureCount = set.Textures.Count, ModelCount = set.Models.Count, ConversionMilliseconds = conversionMilliseconds };
        List<ModelBlobEntry> entries = new List<ModelBlobEntry>(set.Models.Count);
        for (int index = 0; index < set.Models.Count; index++)
        {
            ModelData model = set.Models[index];
            long triangles = 0;
            int primitives = 0;
            int blended = 0;
            foreach (ModelMeshData mesh in model.Meshes)
            {
                foreach (ModelPrimitiveData primitive in mesh.Primitives)
                {
                    triangles += primitive.IndexCount / 3;
                    primitives++;
                    if (IsBlended(model, primitive)) blended++;
                    foreach (ModelMorphTargetData target in primitive.MorphTargets)
                    {
                        statistics.MorphBytes += 12L * (target.PositionDeltas.Length + target.NormalDeltas.Length + target.TangentDeltas.Length);
                    }
                }
            }

            statistics.VertexBytes += 64L * model.Vertices.Length;
            statistics.SkinVertexBytes += 24L * (model.SkinVertices?.Length ?? 0);
            statistics.IndexBytes += (model.IndexFormat == ModelIndexFormat.Uint16 ? 2L : 4L) * model.Indices.Length;
            statistics.Triangles += triangles;
            statistics.Primitives += primitives;
            statistics.BlendedPrimitives += blended;
            List<string> animations = new List<string>(model.Animations.Count);
            foreach (ModelAnimationData animation in model.Animations) animations.Add(animation.Name ?? string.Empty);
            entries.Add(new ModelBlobEntry(model.Name, index, model.Vertices.Length, (int)triangles, primitives, blended, animations, model.Warnings.ToArray()));
        }

        long mipped = 0;
        foreach (ModelTextureData texture in set.Textures)
        {
            long level = 4L * texture.Width * texture.Height;
            statistics.TextureBytes += level;
            mipped += texture.Mipmaps ? level * 4 / 3 : level;
        }

        statistics.GpuBytesEstimate = statistics.VertexBytes + statistics.SkinVertexBytes + statistics.IndexBytes + statistics.MorphBytes + mipped;
        return new ModelBlob(bytes, entries, set.Textures.Count, statistics);
    }

    /// <summary>True when <paramref name="primitive" /> of <paramref name="model" /> is drawn blended (its material's alpha mode is BLEND).</summary>
    /// <param name="model">The model.</param>
    /// <param name="primitive">One of its primitives.</param>
    /// <returns>True for a blended primitive.</returns>
    public static bool IsBlended(ModelData model, ModelPrimitiveData primitive)
    {
        ArgumentNullException.ThrowIfNull(model);
        ArgumentNullException.ThrowIfNull(primitive);
        ModelMaterialData material = primitive.Material >= 0 && primitive.Material < model.Materials.Count ? model.Materials[primitive.Material] : model.DefaultMaterial;
        return material != null && material.AlphaMode == MaterialAlphaMode.Blend;
    }
}
