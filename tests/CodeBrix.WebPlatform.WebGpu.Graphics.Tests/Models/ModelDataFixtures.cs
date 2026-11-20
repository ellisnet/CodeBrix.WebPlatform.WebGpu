using System.Collections.Generic;
using System.Numerics;
using CodeBrix.WebPlatform.WebGpu.Graphics.Models;

namespace CodeBrix.WebPlatform.WebGpu.Graphics.Tests.Models;

/// <summary>
/// The model sets of the three fixture blobs in src/CodeBrix.WebPlatform.WebGpu.Graphics/Rust/codebrix-webgpu-graphics/fixtures
/// (written by <see cref="ModelDataWriter" />; the Rust reader tests assert the values below, so change them only
/// together with those tests and regenerate the files - see ModelDataFixturesTests).
/// <para>
/// triangle.cbmd (<see cref="Triangle" />, 1792 bytes): 1 model "triangle"; no textures, no samplers, no warnings.
/// 3 vertices: positions (0,0,0) (1,0,0) (0,1,0), normals (0,0,1), tangents 0, texcoord0 (0,0) (1,0) (0,1),
/// texcoord1 0, colour 65535 x 4. Indices 0 1 2, Uint16 (format 2). 1 mesh "triangle" with 1 primitive: material 0,
/// first index 0, index count 3, base vertex 0, vertex count 3, skin base vertex -1, bounds (0,0,0)..(1,1,0),
/// flags 0, no morph targets; no default morph weights. 2 MTRL records: material 0 "red" (index 0, base colour
/// (1,0,0,1), metallic 0, roughness 0.5, the rest default: every slot texture -1 sampler -1 texcoord 0 no transform
/// identity transform glTF index -1, normal scale 1, occlusion 1, emissive (0,0,0), emissive strength 1, Opaque,
/// cutoff 0.5, single-sided, lit, IOR 1.5, clear coat 0 / 0, specular 1 and (1,1,1), transmission 0, no approximated
/// features) and the default material "default" (index -1, all defaults) at MTRL 1 (= the model's default material).
/// 1 node "triangle": parent -1, no children, mesh 0, skin -1, translation 0, rotation identity, scale 1, not fixed,
/// local matrix identity, no morph weights, in scene. Root nodes [0]. No skins, animations, skin vertices.
/// </para>
/// <para>
/// every-section.cbmd (<see cref="EverySection" />): 1 model "every-section"; 1 set texture "checker" 2 x 2, Srgb,
/// mipmaps, pixels red (255,0,0,255), green (0,255,0,255), blue (0,0,255,255), white (255,255,255,255).
/// Model textures [0]; 1 sampler: U ClampToEdge, V MirrorRepeat, mag Nearest, min Linear, mip Nearest, max LOD 0.
/// Warnings ["points primitive skipped", "image 1 could not be decoded"].
/// 7 vertices, 9 indices, Uint32 (format 4): mesh 0 "quad" vertices 0..3 at z 0 ((-1,-1) (1,-1) (1,1) (-1,1), normal
/// (0,0,1), tangent (1,0,0,1), colour (65535, 32768, 0, 65535)), indices 0 1 2 0 2 3; mesh 1 "skinned-triangle" vertices
/// 4..6 ((0,0,0) (1,0,0) (0,1,0), texcoord1 (0.5,0.5)), indices 0 1 2.
/// 2 meshes, 2 primitives, 1 morph target. Primitive 0 (mesh "quad"): material 0, first index 0, 6 indices, base
/// vertex 0, 4 vertices, skin base -1, bounds (-1,-1,0)..(1,1,0), flags HasVertexColors | HasTangents |
/// TangentsGenerated (= 13). Primitive 1 (mesh "skinned-triangle"): material -1, first index 6, 3 indices, base vertex 4,
/// 3 vertices, skin base vertex 0, bounds (0,0,0)..(1,1,0.5), flags HasTexCoord1 | NormalsGenerated (= 18), 1 morph
/// target: position deltas (0,0,0.5) x 3, normal and tangent deltas empty. Mesh 1 default morph weights [0.25];
/// mesh 0 none.
/// 3 skin vertices: joints (0,1,0,0) for each, weights (1,0,0,0) (0.5,0.5,0,0) (0,1,0,0).
/// 3 MTRL records: material 0 "textured" (base colour (1,1,1,1), base colour slot texture 0 sampler 0 texcoord 0,
/// has transform, transform M11 2 M12 0 M21 0 M22 2 M31 0.5 M32 0.25, glTF index 0; Mask, cutoff 0.25, double-sided);
/// material 1 "clearcoat" (base colour (0.5,0.5,0.5,1), metallic 0.25, roughness 0.75, normal scale 0.5, occlusion
/// strength 0.875, emissive (1,0.5,0.25) strength 2, Blend, unlit, IOR 1.25, clear coat 0.75 roughness 0.125,
/// specular 0.5 colour (1,0.5,0.25), transmission 0.375, approximated features ["KHR_materials_clearcoat",
/// "KHR_materials_transmission"]; occlusion slot: texture 0, sampler -1, texcoord 1, glTF index 0); default material
/// "default" at MTRL 2.
/// 5 nodes, roots [0]: 0 "root" (children [1,2,3], translation (1,2,3), rotation identity, scale (2,2,2), not fixed,
/// so the local matrix is diag(2,2,2,1) with M41..M43 = 1,2,3); 1 "quad-node" (parent 0, mesh 0, fixed matrix:
/// identity with M21 = 0.5 and M41 = 4); 2 "skinned-node" (parent 0, mesh 1, skin 0, morph weights [0.25]);
/// 3 "joint-0" (parent 0, children [4], translation (0,1,0)); 4 "joint-1" (parent 3, translation (0,1,0), rotation
/// (0,0,0.70710677,0.70710677)). Every node in scene.
/// 1 skin "skin": joints [3,4], inverse bind matrices translation (0,-1,0) and (0,-2,0) (M42 = -1, -2), skeleton 3.
/// 2 animations, 4 channels: "move" with channel 0 (node 0, Translation, Step, 3 components, times [0,1], values
/// [0,0,0, 1,2,3]), channel 1 (node 3, Rotation, Linear, 4, times [0,1], values [0,0,0,1, 0,0,0.70710677,0.70710677]),
/// channel 2 (node 4, Scale, CubicSpline, 3, times [0,2], values 18 floats: in (0,0,0) value (1,1,1) out (0,0,0),
/// in (0,0,0) value (2,2,2) out (0,0,0)), channel 3 (node 2, Weights, Linear, 1, times [0,0.5], values [0.25,1]);
/// "idle" with no channels.
/// </para>
/// <para>
/// two-models-shared-texture.cbmd (<see cref="TwoModelsSharedTexture" />): 2 models, 2 set textures:
/// texture 0 "shared" 2 x 1, Srgb, mipmaps, pixels (10,20,30,255) (40,50,60,255); texture 1 "second-only" 1 x 1,
/// Linear, no mipmaps, pixel (1,2,3,4). Pixel ranges at PIX0 offsets 0 and 16. Model 0 "first": 3 vertices
/// ((0,0,0) (1,0,0) (0,1,0)), indices 0 1 2 Uint16 (IDX0 offset 0, VTX0 offset 0), textures [0], 1 sampler (all
/// defaults: Repeat, Repeat, Linear, Linear, Linear, 32), 1 material "first-material" (base colour slot texture 0
/// sampler 0, glTF index 0), default material at MTRL 1, 1 mesh "first-mesh" (1 primitive, material 0), 1 node
/// "first-node" (mesh 0, in scene), roots [0]. Model 1 "second": 4 vertices ((0,0,1) (1,0,1) (1,1,1) (0,1,1)),
/// indices 0 1 2 0 2 3 Uint16 (IDX0 offset 16, VTX0 offset 192), textures [1, 0] (absolute: local 0 = set texture 1,
/// local 1 = set texture 0), 1 sampler (ClampToEdge both, Linear, Linear, Linear, 32), 1 material "second-material"
/// at MTRL 2 (base colour slot texture 1 = "shared", sampler 0, glTF index 1; emissive slot texture 0 =
/// "second-only", sampler 0, glTF index 0; emissive (1,1,1)), default material at MTRL 3, first mesh 1, 1 mesh
/// "second-mesh" (1 primitive, material 0, 6 indices, 4 vertices), first node 1, 1 node "second-node" (mesh 0, in
/// scene), roots [0].
/// </para>
/// </summary>
internal static class ModelDataFixtures
{
    /// <summary>The file names and the sets, in a fixed order.</summary>
    public static IReadOnlyList<(string FileName, ModelDataSet Set)> All => new[]
    {
        ("triangle.cbmd", Triangle()),
        ("every-section.cbmd", EverySection()),
        ("two-models-shared-texture.cbmd", TwoModelsSharedTexture()),
    };

    /// <summary>The tiny golden model: one triangle, one material, no texture.</summary>
    public static ModelDataSet Triangle()
    {
        var model = new ModelData
        {
            Name = "triangle",
            Vertices = TriangleVertices(Vector3.Zero),
            Indices = new uint[] { 0, 1, 2 },
            IndexFormat = ModelIndexFormat.Uint16,
            RootNodes = new List<int> { 0 },
        };
        model.Meshes.Add(new ModelMeshData
        {
            Name = "triangle",
            Primitives = { new ModelPrimitiveData { Material = 0, IndexCount = 3, VertexCount = 3, BoundsMin = Vector3.Zero, BoundsMax = new Vector3(1, 1, 0) } },
        });
        model.Materials.Add(new ModelMaterialData { Index = 0, Name = "red", BaseColorFactor = new Vector4(1, 0, 0, 1), MetallicFactor = 0f, RoughnessFactor = 0.5f });
        model.Nodes.Add(new ModelNodeData { Name = "triangle", Mesh = 0, InScene = true });
        var set = new ModelDataSet();
        set.Models.Add(model);
        return set;
    }

    /// <summary>One model that fills every section of the format.</summary>
    public static ModelDataSet EverySection()
    {
        var set = new ModelDataSet();
        set.Textures.Add(new ModelTextureData
        {
            Name = "checker",
            Width = 2,
            Height = 2,
            ColorSpace = TextureColorSpace.Srgb,
            Pixels = new byte[] { 255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255, 255, 255, 255, 255 },
        });

        var vertices = new ModelVertex[7];
        var corners = new[] { new Vector3(-1, -1, 0), new Vector3(1, -1, 0), new Vector3(1, 1, 0), new Vector3(-1, 1, 0) };
        for (var index = 0; index < 4; index++)
        {
            vertices[index] = new ModelVertex
            {
                Position = corners[index],
                Normal = Vector3.UnitZ,
                Tangent = new Vector4(1, 0, 0, 1),
                TexCoord0 = new Vector2((corners[index].X + 1) / 2, (1 - corners[index].Y) / 2),
                ColorR = 65535,
                ColorG = 32768,
                ColorB = 0,
                ColorA = 65535,
            };
        }

        var triangle = TriangleVertices(Vector3.Zero);
        for (var index = 0; index < 3; index++)
        {
            triangle[index].TexCoord1 = new Vector2(0.5f, 0.5f);
            vertices[4 + index] = triangle[index];
        }

        var model = new ModelData
        {
            Name = "every-section",
            Vertices = vertices,
            Indices = new uint[] { 0, 1, 2, 0, 2, 3, 0, 1, 2 },
            IndexFormat = ModelIndexFormat.Uint32,
            SkinVertices = new[]
            {
                new ModelSkinVertex { Joint0 = 0, Joint1 = 1, Weights = new Vector4(1, 0, 0, 0) },
                new ModelSkinVertex { Joint0 = 0, Joint1 = 1, Weights = new Vector4(0.5f, 0.5f, 0, 0) },
                new ModelSkinVertex { Joint0 = 0, Joint1 = 1, Weights = new Vector4(0, 1, 0, 0) },
            },
            RootNodes = new List<int> { 0 },
            Textures = new List<int> { 0 },
            Warnings = new List<string> { "points primitive skipped", "image 1 could not be decoded" },
        };
        model.Samplers.Add(new ModelSamplerData
        {
            AddressModeU = SamplerAddressMode.ClampToEdge,
            AddressModeV = SamplerAddressMode.MirrorRepeat,
            MagFilter = SamplerFilter.Nearest,
            MinFilter = SamplerFilter.Linear,
            MipmapFilter = SamplerFilter.Nearest,
            MaxLod = 0f,
        });

        model.Meshes.Add(new ModelMeshData
        {
            Name = "quad",
            Primitives =
            {
                new ModelPrimitiveData
                {
                    Material = 0,
                    IndexCount = 6,
                    VertexCount = 4,
                    BoundsMin = new Vector3(-1, -1, 0),
                    BoundsMax = new Vector3(1, 1, 0),
                    HasVertexColors = true,
                    HasTangents = true,
                    TangentsGenerated = true,
                },
            },
        });
        model.Meshes.Add(new ModelMeshData
        {
            Name = "skinned-triangle",
            DefaultMorphWeights = new[] { 0.25f },
            Primitives =
            {
                new ModelPrimitiveData
                {
                    Material = -1,
                    FirstIndex = 6,
                    IndexCount = 3,
                    BaseVertex = 4,
                    VertexCount = 3,
                    SkinBaseVertex = 0,
                    BoundsMin = Vector3.Zero,
                    BoundsMax = new Vector3(1, 1, 0.5f),
                    HasTexCoord1 = true,
                    NormalsGenerated = true,
                    MorphTargets = { new ModelMorphTargetData { PositionDeltas = new[] { new Vector3(0, 0, 0.5f), new Vector3(0, 0, 0.5f), new Vector3(0, 0, 0.5f) } } },
                },
            },
        });

        model.Materials.Add(new ModelMaterialData
        {
            Index = 0,
            Name = "textured",
            BaseColorTexture = new ModelTextureSlotData { Texture = 0, Sampler = 0, HasTransform = true, Transform = new Matrix3x2(2, 0, 0, 2, 0.5f, 0.25f), TextureIndex = 0 },
            AlphaMode = MaterialAlphaMode.Mask,
            AlphaCutoff = 0.25f,
            DoubleSided = true,
        });
        model.Materials.Add(new ModelMaterialData
        {
            Index = 1,
            Name = "clearcoat",
            BaseColorFactor = new Vector4(0.5f, 0.5f, 0.5f, 1),
            MetallicFactor = 0.25f,
            RoughnessFactor = 0.75f,
            NormalScale = 0.5f,
            OcclusionTexture = new ModelTextureSlotData { Texture = 0, Sampler = -1, TexCoord = 1, TextureIndex = 0 },
            OcclusionStrength = 0.875f,
            EmissiveFactor = new Vector3(1, 0.5f, 0.25f),
            EmissiveStrength = 2f,
            AlphaMode = MaterialAlphaMode.Blend,
            Unlit = true,
            IndexOfRefraction = 1.25f,
            ClearCoatFactor = 0.75f,
            ClearCoatRoughness = 0.125f,
            SpecularFactor = 0.5f,
            SpecularColor = new Vector3(1, 0.5f, 0.25f),
            TransmissionFactor = 0.375f,
            ApproximatedFeatures = { "KHR_materials_clearcoat", "KHR_materials_transmission" },
        });

        var shear = Matrix4x4.Identity;
        shear.M21 = 0.5f;
        shear.M41 = 4f;
        model.Nodes.Add(new ModelNodeData { Name = "root", Children = { 1, 2, 3 }, Translation = new Vector3(1, 2, 3), Scale = new Vector3(2, 2, 2), InScene = true });
        model.Nodes.Add(new ModelNodeData { Name = "quad-node", Parent = 0, Mesh = 0, HasFixedMatrix = true, LocalMatrix = shear, InScene = true });
        model.Nodes.Add(new ModelNodeData { Name = "skinned-node", Parent = 0, Mesh = 1, Skin = 0, DefaultMorphWeights = new[] { 0.25f }, InScene = true });
        model.Nodes.Add(new ModelNodeData { Name = "joint-0", Parent = 0, Children = { 4 }, Translation = new Vector3(0, 1, 0), InScene = true });
        model.Nodes.Add(new ModelNodeData { Name = "joint-1", Parent = 3, Translation = new Vector3(0, 1, 0), Rotation = new Quaternion(0, 0, 0.70710677f, 0.70710677f), InScene = true });

        model.Skins.Add(new ModelSkinData
        {
            Name = "skin",
            Joints = { 3, 4 },
            InverseBindMatrices = { Matrix4x4.CreateTranslation(0, -1, 0), Matrix4x4.CreateTranslation(0, -2, 0) },
            Skeleton = 3,
        });

        model.Animations.Add(new ModelAnimationData
        {
            Name = "move",
            Channels =
            {
                new ModelAnimationChannelData { Node = 0, Path = AnimationPath.Translation, Interpolation = AnimationInterpolation.Step, ComponentCount = 3, Times = new[] { 0f, 1f }, Values = new[] { 0f, 0f, 0f, 1f, 2f, 3f } },
                new ModelAnimationChannelData { Node = 3, Path = AnimationPath.Rotation, Interpolation = AnimationInterpolation.Linear, ComponentCount = 4, Times = new[] { 0f, 1f }, Values = new[] { 0f, 0f, 0f, 1f, 0f, 0f, 0.70710677f, 0.70710677f } },
                new ModelAnimationChannelData
                {
                    Node = 4,
                    Path = AnimationPath.Scale,
                    Interpolation = AnimationInterpolation.CubicSpline,
                    ComponentCount = 3,
                    Times = new[] { 0f, 2f },
                    Values = new[] { 0f, 0f, 0f, 1f, 1f, 1f, 0f, 0f, 0f, 0f, 0f, 0f, 2f, 2f, 2f, 0f, 0f, 0f },
                },
                new ModelAnimationChannelData { Node = 2, Path = AnimationPath.Weights, Interpolation = AnimationInterpolation.Linear, ComponentCount = 1, Times = new[] { 0f, 0.5f }, Values = new[] { 0.25f, 1f } },
            },
        });
        model.Animations.Add(new ModelAnimationData { Name = "idle" });

        set.Models.Add(model);
        return set;
    }

    /// <summary>Two models naming the same texture, which the set stores once.</summary>
    public static ModelDataSet TwoModelsSharedTexture()
    {
        var set = new ModelDataSet();
        set.Textures.Add(new ModelTextureData { Name = "shared", Width = 2, Height = 1, ColorSpace = TextureColorSpace.Srgb, Pixels = new byte[] { 10, 20, 30, 255, 40, 50, 60, 255 } });
        set.Textures.Add(new ModelTextureData { Name = "second-only", Width = 1, Height = 1, ColorSpace = TextureColorSpace.Linear, Mipmaps = false, Pixels = new byte[] { 1, 2, 3, 4 } });

        var first = new ModelData
        {
            Name = "first",
            Vertices = TriangleVertices(Vector3.Zero),
            Indices = new uint[] { 0, 1, 2 },
            IndexFormat = ModelIndexFormat.Uint16,
            RootNodes = new List<int> { 0 },
            Textures = new List<int> { 0 },
            Samplers = { new ModelSamplerData() },
        };
        first.Materials.Add(new ModelMaterialData { Index = 0, Name = "first-material", BaseColorTexture = new ModelTextureSlotData { Texture = 0, Sampler = 0, TextureIndex = 0 } });
        first.Meshes.Add(new ModelMeshData { Name = "first-mesh", Primitives = { new ModelPrimitiveData { Material = 0, IndexCount = 3, VertexCount = 3, BoundsMax = new Vector3(1, 1, 0) } } });
        first.Nodes.Add(new ModelNodeData { Name = "first-node", Mesh = 0, InScene = true });

        var quad = new ModelVertex[4];
        var corners = new[] { new Vector3(0, 0, 1), new Vector3(1, 0, 1), new Vector3(1, 1, 1), new Vector3(0, 1, 1) };
        for (var index = 0; index < 4; index++) quad[index] = new ModelVertex { Position = corners[index], Normal = Vector3.UnitZ, TexCoord0 = new Vector2(corners[index].X, 1 - corners[index].Y), ColorR = 65535, ColorG = 65535, ColorB = 65535, ColorA = 65535 };
        var second = new ModelData
        {
            Name = "second",
            Vertices = quad,
            Indices = new uint[] { 0, 1, 2, 0, 2, 3 },
            IndexFormat = ModelIndexFormat.Uint16,
            RootNodes = new List<int> { 0 },
            Textures = new List<int> { 1, 0 },
            Samplers = { new ModelSamplerData { AddressModeU = SamplerAddressMode.ClampToEdge, AddressModeV = SamplerAddressMode.ClampToEdge } },
        };
        second.Materials.Add(new ModelMaterialData
        {
            Index = 0,
            Name = "second-material",
            BaseColorTexture = new ModelTextureSlotData { Texture = 1, Sampler = 0, TextureIndex = 1 },
            EmissiveTexture = new ModelTextureSlotData { Texture = 0, Sampler = 0, TextureIndex = 0 },
            EmissiveFactor = Vector3.One,
        });
        second.Meshes.Add(new ModelMeshData { Name = "second-mesh", Primitives = { new ModelPrimitiveData { Material = 0, IndexCount = 6, VertexCount = 4, BoundsMin = new Vector3(0, 0, 1), BoundsMax = Vector3.One } } });
        second.Nodes.Add(new ModelNodeData { Name = "second-node", Mesh = 0, InScene = true });

        set.Models.Add(first);
        set.Models.Add(second);
        return set;
    }

    /// <summary>The triangle (0,0,0) (1,0,0) (0,1,0) facing +Z, white, with texture coordinates matching the positions (V down).</summary>
    public static ModelVertex[] TriangleVertices(Vector3 offset)
    {
        var positions = new[] { new Vector3(0, 0, 0), new Vector3(1, 0, 0), new Vector3(0, 1, 0) };
        var texCoords = new[] { new Vector2(0, 0), new Vector2(1, 0), new Vector2(0, 1) };
        var vertices = new ModelVertex[3];
        for (var index = 0; index < 3; index++)
        {
            vertices[index] = new ModelVertex { Position = positions[index] + offset, Normal = Vector3.UnitZ, TexCoord0 = texCoords[index], ColorR = 65535, ColorG = 65535, ColorB = 65535, ColorA = 65535 };
        }

        return vertices;
    }
}
