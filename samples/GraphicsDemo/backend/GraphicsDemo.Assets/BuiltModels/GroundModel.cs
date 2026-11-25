using System;
using System.Numerics;
using CodeBrix.WebPlatform.WebGpu.Graphics.Models;

namespace GraphicsDemo.Assets.BuiltModels;

/// <summary>
/// The ground of the Viewer scene, BUILT IN CODE (no asset file): a 180 x 180 grass-green plane (reaching nearly to the
/// horizon) facing up, one quad with a rough, non-metallic material - the desktop model viewer's ground. It is made
/// directly as model data (the Graphics package's types), exactly as the glTF loader reads the desktop viewer's ground
/// document (four vertices with an upward normal and no texture coordinates, one node, one material), and written by
/// ModelDataWriter; the frontend loads it with the graphics crate's blob reader.
/// </summary>
public static class GroundModel
{
    /// <summary>The ground model's name.</summary>
    public const string Name = "ground";

    /// <summary>Half the side of the plane, in metres.</summary>
    public const float HalfSize = 90f;

    /// <summary>Builds the ground.</summary>
    /// <returns>The model data (no textures).</returns>
    public static ModelData Create()
    {
        Vector3[] corners =
        {
            new Vector3(-HalfSize, 0, HalfSize),
            new Vector3(HalfSize, 0, HalfSize),
            new Vector3(HalfSize, 0, -HalfSize),
            new Vector3(-HalfSize, 0, -HalfSize),
        };
        ModelVertex[] vertices = new ModelVertex[corners.Length];
        for (int index = 0; index < corners.Length; index++)
        {
            // No texture coordinates and no tangents (the material has no texture); white vertex colour.
            vertices[index] = new ModelVertex { Position = corners[index], Normal = Vector3.UnitY, Color = Vector4.One };
        }

        ModelPrimitiveData primitive = new ModelPrimitiveData
        {
            Material = 0,
            FirstIndex = 0,
            IndexCount = 6,
            BaseVertex = 0,
            VertexCount = vertices.Length,
            BoundsMin = new Vector3(-HalfSize, 0, -HalfSize),
            BoundsMax = new Vector3(HalfSize, 0, HalfSize),
        };
        ModelData model = new ModelData
        {
            Name = Name,
            Vertices = vertices,
            Indices = new uint[] { 0, 1, 2, 0, 2, 3 },
            IndexFormat = ModelIndexFormat.Uint16,
        };
        model.Meshes.Add(new ModelMeshData { Name = Name, Primitives = { primitive } });
        // glTF colour factors are linear: sRGB (110, 168, 84) is the green the scene always had.
        model.Materials.Add(new ModelMaterialData { Index = 0, Name = "grass", BaseColorFactor = new Vector4(Linear(110), Linear(168), Linear(84), 1f), MetallicFactor = 0f, RoughnessFactor = 0.95f });
        model.Nodes.Add(new ModelNodeData { Name = Name, Mesh = 0, InScene = true });
        model.RootNodes.Add(0);
        return model;
    }

    /// <summary>Builds the ground and adds it to <paramref name="set" />.</summary>
    /// <param name="set">The set the ground is added to.</param>
    /// <returns>The ground model (named <see cref="Name" />).</returns>
    /// <exception cref="ArgumentNullException"><paramref name="set" /> is null.</exception>
    public static ModelData AddTo(ModelDataSet set)
    {
        ArgumentNullException.ThrowIfNull(set);
        ModelData model = Create();
        set.Models.Add(model);
        return model;
    }

    private static float Linear(byte srgb)
    {
        float c = srgb / 255f;
        return c <= 0.04045f ? c / 12.92f : MathF.Pow((c + 0.055f) / 1.055f, 2.4f);
    }
}
