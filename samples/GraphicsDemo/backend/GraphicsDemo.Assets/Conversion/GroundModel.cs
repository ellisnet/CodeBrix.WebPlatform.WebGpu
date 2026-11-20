using System;
using System.Numerics;
using CodeBrix.WebPlatform.WebGpu.Graphics.Models;
using Gltf = CodeBrix.Graphics3D.Gltf2.Schema2;

namespace GraphicsDemo.Assets.Conversion;

/// <summary>
/// The ground of the Viewer scene: a 180 x 180 grass-green plane (reaching nearly to the horizon) facing up, one quad
/// with a rough, non-metallic material. It is built exactly as the desktop model viewer builds it - a glTF document
/// made in memory with CodeBrix.Graphics3D.Gltf2 and written as .glb - and then converted like every other model.
/// </summary>
public static class GroundModel
{
    /// <summary>The ground model's name.</summary>
    public const string Name = "ground";

    /// <summary>Half the side of the plane, in metres.</summary>
    public const float HalfSize = 90f;

    /// <summary>Builds the ground as .glb bytes.</summary>
    /// <returns>The .glb file.</returns>
    public static byte[] CreateGlb()
    {
        Gltf.ModelRoot root = Gltf.ModelRoot.CreateModel();
        Gltf.Material material = Gltf.Toolkit.WithPBRMetallicRoughness(root.CreateMaterial("grass"));

        // glTF colour factors are linear: sRGB (110, 168, 84) is the green the scene always had.
        Gltf.Toolkit.WithChannelColor(material, "BaseColor", new Vector4(Linear(110), Linear(168), Linear(84), 1f));
        Gltf.Toolkit.WithChannelFactor(material, "MetallicRoughness", "MetallicFactor", 0f);
        Gltf.Toolkit.WithChannelFactor(material, "MetallicRoughness", "RoughnessFactor", 0.95f);

        Gltf.MeshPrimitive primitive = root.CreateMesh("ground").CreatePrimitive();
        Gltf.Toolkit.WithVertexAccessor(primitive, "POSITION", new[] { new Vector3(-HalfSize, 0, HalfSize), new Vector3(HalfSize, 0, HalfSize), new Vector3(HalfSize, 0, -HalfSize), new Vector3(-HalfSize, 0, -HalfSize) });
        Gltf.Toolkit.WithVertexAccessor(primitive, "NORMAL", new[] { Vector3.UnitY, Vector3.UnitY, Vector3.UnitY, Vector3.UnitY });
        Gltf.Toolkit.WithIndicesAccessor(primitive, Gltf.PrimitiveType.TRIANGLES, new[] { 0, 1, 2, 0, 2, 3 });
        Gltf.Toolkit.WithMaterial(primitive, material);
        root.UseScene(0).CreateNode("ground").Mesh = root.LogicalMeshes[0];
        return root.WriteGLB().ToArray();
    }

    /// <summary>Builds the ground and converts it into <paramref name="set" />.</summary>
    /// <param name="set">The set the ground is added to.</param>
    /// <returns>The ground model (named <see cref="Name" />).</returns>
    /// <exception cref="ArgumentNullException"><paramref name="set" /> is null.</exception>
    public static ModelData AddTo(ModelDataSet set)
    {
        ArgumentNullException.ThrowIfNull(set);
        return GltfModelConverter.Convert(set, CreateGlb(), Name, null, null, null);
    }

    private static float Linear(byte srgb)
    {
        float c = srgb / 255f;
        return c <= 0.04045f ? c / 12.92f : MathF.Pow((c + 0.055f) / 1.055f, 2.4f);
    }
}
