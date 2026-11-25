using System;
using System.Collections.Generic;
using System.Numerics;
using CodeBrix.WebPlatform.WebGpu.Graphics.Models;

namespace GraphicsDemo.Assets.BuiltModels;

/// <summary>
/// The model of the "Morph targets" scene, BUILT IN CODE (no asset file): a sphere with three morph targets - "stretch"
/// (it grows tall), "twist" (its rings turn about the vertical axis, more the higher they are) and "bumps" (lumps rise
/// out of its surface) - and one clip, "morph", that moves the three weights in turn (each from 0 up to 1 and back over
/// two seconds, one after the other, six seconds in all, LINEAR). Each target carries position AND normal deltas (the
/// normals of the fully morphed shape minus the sphere's), so the light follows the shape. It is made directly as model
/// data (the Graphics package's types) and written by ModelDataWriter like every other model.
/// </summary>
public static class MorphShapeModel
{
    /// <summary>The model's name.</summary>
    public const string Name = "morph-shape";

    /// <summary>The name of its one clip.</summary>
    public const string ClipName = "morph";

    /// <summary>The names of its morph targets, in order.</summary>
    public static IReadOnlyList<string> TargetNames { get; } = new[] { "stretch", "twist", "bumps" };

    /// <summary>The sphere's segments around the vertical axis.</summary>
    public const int Segments = 48;

    /// <summary>The sphere's rings from pole to pole.</summary>
    public const int Rings = 32;

    /// <summary>The clip's length, in seconds.</summary>
    public const float ClipSeconds = 6f;

    /// <summary>Builds the model.</summary>
    /// <returns>The model data (no textures).</returns>
    public static ModelData Create()
    {
        int columns = Segments + 1;
        int count = columns * (Rings + 1);
        Vector3[] positions = new Vector3[count];
        Vector2[] uvs = new Vector2[count];
        for (int ring = 0; ring <= Rings; ring++)
        {
            float latitude = MathF.PI * ring / Rings;
            for (int segment = 0; segment <= Segments; segment++)
            {
                float longitude = MathF.Tau * segment / Segments;
                int index = (ring * columns) + segment;
                positions[index] = new Vector3(MathF.Sin(latitude) * MathF.Sin(longitude), MathF.Cos(latitude), MathF.Sin(latitude) * MathF.Cos(longitude));
                uvs[index] = new Vector2((float)segment / Segments, (float)ring / Rings);
            }
        }

        List<uint> indices = new List<uint>(Segments * Rings * 6);
        for (int ring = 0; ring < Rings; ring++)
        {
            for (int segment = 0; segment < Segments; segment++)
            {
                uint a = (uint)((ring * columns) + segment);
                uint b = a + (uint)columns;
                // Counter-clockwise seen from outside (the front face).
                indices.AddRange(new[] { a, b, a + 1, a + 1, b, b + 1 });
            }
        }

        uint[] triangles = indices.ToArray();
        Vector3[] normals = Normals(positions, triangles);
        ModelVertex[] vertices = new ModelVertex[count];
        for (int index = 0; index < count; index++)
        {
            vertices[index] = new ModelVertex
            {
                Position = positions[index],
                Normal = normals[index],
                Tangent = new Vector4(1, 0, 0, 1),
                TexCoord0 = uvs[index],
                Color = Vector4.One,
            };
        }

        ModelPrimitiveData primitive = new ModelPrimitiveData
        {
            Material = 0,
            FirstIndex = 0,
            IndexCount = (uint)triangles.Length,
            BaseVertex = 0,
            VertexCount = count,
            BoundsMin = new Vector3(-1, -1, -1),
            BoundsMax = new Vector3(1, 1, 1),
        };
        foreach (Func<Vector3, Vector3> shape in Shapes())
        {
            Vector3[] morphed = new Vector3[count];
            Vector3[] positionDeltas = new Vector3[count];
            for (int index = 0; index < count; index++)
            {
                morphed[index] = shape(positions[index]);
                positionDeltas[index] = morphed[index] - positions[index];
            }

            Vector3[] morphedNormals = Normals(morphed, triangles);
            Vector3[] normalDeltas = new Vector3[count];
            for (int index = 0; index < count; index++) normalDeltas[index] = morphedNormals[index] - normals[index];
            primitive.MorphTargets.Add(new ModelMorphTargetData { PositionDeltas = positionDeltas, NormalDeltas = normalDeltas });
        }

        float[] rest = new float[TargetNames.Count];
        ModelData model = new ModelData
        {
            Name = Name,
            Vertices = vertices,
            Indices = triangles,
            IndexFormat = ModelIndexFormat.Uint16,
        };
        model.Meshes.Add(new ModelMeshData { Name = "shape", Primitives = { primitive }, DefaultMorphWeights = rest });
        // A warm coral, glossy enough to show the shape in its highlights.
        model.Materials.Add(new ModelMaterialData { Index = 0, Name = "coral", BaseColorFactor = new Vector4(0.80f, 0.28f, 0.20f, 1f), MetallicFactor = 0f, RoughnessFactor = 0.35f });
        model.Nodes.Add(new ModelNodeData { Name = "shape", Mesh = 0, DefaultMorphWeights = (float[])rest.Clone(), InScene = true });
        model.RootNodes.Add(0);
        model.Animations.Add(new ModelAnimationData
        {
            Name = ClipName,
            Channels = { new ModelAnimationChannelData { Node = 0, Path = AnimationPath.Weights, Interpolation = AnimationInterpolation.Linear, ComponentCount = TargetNames.Count, Times = KeyTimes(), Values = KeyValues() } },
        });
        return model;
    }

    /// <summary>Builds the model and adds it to <paramref name="set" />.</summary>
    /// <param name="set">The set the model is added to.</param>
    /// <returns>The model.</returns>
    /// <exception cref="ArgumentNullException"><paramref name="set" /> is null.</exception>
    public static ModelData AddTo(ModelDataSet set)
    {
        ArgumentNullException.ThrowIfNull(set);
        ModelData model = Create();
        set.Models.Add(model);
        return model;
    }

    /// <summary>The key times: one every second, 0 to <see cref="ClipSeconds" />.</summary>
    /// <returns>The times.</returns>
    public static float[] KeyTimes() => new[] { 0f, 1f, 2f, 3f, 4f, 5f, 6f };

    /// <summary>The key values, three weights per key: each target rises to 1 and falls back in turn.</summary>
    /// <returns>The values.</returns>
    public static float[] KeyValues() => new[]
    {
        0f, 0f, 0f,
        1f, 0f, 0f,
        0f, 0f, 0f,
        0f, 1f, 0f,
        0f, 0f, 0f,
        0f, 0f, 1f,
        0f, 0f, 0f,
    };

    /// <summary>The three fully morphed shapes, as functions of a point of the unit sphere.</summary>
    private static Func<Vector3, Vector3>[] Shapes() => new Func<Vector3, Vector3>[]
    {
        // stretch: 1.7 times as tall.
        p => new Vector3(p.X, p.Y * 1.7f, p.Z),
        // twist: each ring turned about Y by 1.4 radians per unit of height.
        p =>
        {
            float angle = p.Y * 1.4f;
            float c = MathF.Cos(angle);
            float s = MathF.Sin(angle);
            return new Vector3((p.X * c) + (p.Z * s), p.Y, (-p.X * s) + (p.Z * c));
        },
        // bumps: lumps rise out of the surface in a checkerboard - twelve sectors around, four bands from pole to pole, every other one.
        p =>
        {
            float longitude = MathF.Atan2(p.X, p.Z);
            float latitude = MathF.Acos(Math.Clamp(p.Y, -1f, 1f));
            float lump = MathF.Max(0f, MathF.Sin(longitude * 6f) * MathF.Sin(latitude * 4f));
            return p * (1f + (0.35f * lump * lump));
        },
    };

    /// <summary>Smooth vertex normals: area-weighted face normals summed over every vertex at the same place (the seam and the poles are welded).</summary>
    private static Vector3[] Normals(Vector3[] positions, uint[] triangles)
    {
        Dictionary<(int, int, int), Vector3> sums = new Dictionary<(int, int, int), Vector3>();
        static (int, int, int) Key(Vector3 p) => ((int)MathF.Round(p.X * 10000f), (int)MathF.Round(p.Y * 10000f), (int)MathF.Round(p.Z * 10000f));
        for (int index = 0; index < triangles.Length; index += 3)
        {
            Vector3 a = positions[triangles[index]];
            Vector3 b = positions[triangles[index + 1]];
            Vector3 c = positions[triangles[index + 2]];
            Vector3 face = Vector3.Cross(b - a, c - a);
            foreach (Vector3 corner in new[] { a, b, c })
            {
                (int, int, int) key = Key(corner);
                sums[key] = sums.TryGetValue(key, out Vector3 sum) ? sum + face : face;
            }
        }

        Vector3[] normals = new Vector3[positions.Length];
        for (int index = 0; index < positions.Length; index++)
        {
            Vector3 sum = sums.TryGetValue(Key(positions[index]), out Vector3 value) ? value : Vector3.Zero;
            normals[index] = sum.LengthSquared() > 1e-20f ? Vector3.Normalize(sum) : Vector3.Normalize(positions[index]);
        }

        return normals;
    }
}
