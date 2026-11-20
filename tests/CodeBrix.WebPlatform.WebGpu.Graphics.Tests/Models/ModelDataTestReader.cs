using System;
using System.Buffers.Binary;
using System.Collections.Generic;
using System.Numerics;
using System.Text;
using CodeBrix.WebPlatform.WebGpu.Graphics.Models;

namespace CodeBrix.WebPlatform.WebGpu.Graphics.Tests.Models;

/// <summary>
/// A small reader of the model data format for the tests only (the library has no reader; the Rust crate does): it
/// reads a blob back into a <see cref="ModelDataSet" /> so that the tests can round-trip, and exposes the header and
/// the section table. It trusts the blob except for what a round trip needs.
/// </summary>
internal sealed class ModelDataTestReader
{
    private readonly byte[] blob;

    public ModelDataTestReader(byte[] blob)
    {
        this.blob = blob;
        SectionCount = (int)U32(8);
        for (var index = 0; index < SectionCount; index++)
        {
            var entry = 32 + index * 24;
            var tag = Encoding.ASCII.GetString(blob, entry, 4);
            var section = new ModelDataTestSection(tag, (int)U32(entry + 4), (int)U32(entry + 8), (int)U32(entry + 12), (int)U32(entry + 16), (int)U32(entry + 20));
            Sections.Add(section);
            SectionsByTag.Add(tag, section);
        }
    }

    public int SectionCount { get; }

    public List<ModelDataTestSection> Sections { get; } = new List<ModelDataTestSection>();

    public Dictionary<string, ModelDataTestSection> SectionsByTag { get; } = new Dictionary<string, ModelDataTestSection>(StringComparer.Ordinal);

    public uint U32(int offset) => BinaryPrimitives.ReadUInt32LittleEndian(blob.AsSpan(offset));

    public int I32(int offset) => BinaryPrimitives.ReadInt32LittleEndian(blob.AsSpan(offset));

    public ushort U16(int offset) => BinaryPrimitives.ReadUInt16LittleEndian(blob.AsSpan(offset));

    public float F32(int offset) => BinaryPrimitives.ReadSingleLittleEndian(blob.AsSpan(offset));

    /// <summary>The absolute byte offset of a record.</summary>
    public int Record(string tag, int index) => SectionsByTag[tag].Offset + index * SectionsByTag[tag].RecordSize;

    public ModelDataSet Read()
    {
        var set = new ModelDataSet();
        for (var index = 0; index < SectionsByTag["TXTR"].Count; index++) set.Textures.Add(ReadTexture(Record("TXTR", index)));
        for (var index = 0; index < SectionsByTag["MODL"].Count; index++) set.Models.Add(ReadModel(Record("MODL", index)));
        return set;
    }

    public string Str(int offset)
    {
        var start = U32(offset);
        var length = (int)U32(offset + 4);
        if (start == uint.MaxValue) return null;
        return Encoding.UTF8.GetString(blob, SectionsByTag["STRG"].Offset + (int)start, length);
    }

    public List<string> StrList(int offset)
    {
        var list = new List<string>();
        var start = (int)U32(offset);
        var count = (int)U32(offset + 4);
        for (var index = 0; index < count; index++) list.Add(Str(SectionsByTag["U32S"].Offset + (start + index * 2) * 4));
        return list;
    }

    public List<int> U32List(int offset)
    {
        var list = new List<int>();
        var start = (int)U32(offset);
        var count = (int)U32(offset + 4);
        for (var index = 0; index < count; index++) list.Add((int)U32(SectionsByTag["U32S"].Offset + (start + index) * 4));
        return list;
    }

    public float[] F32List(int offset)
    {
        var start = (int)U32(offset);
        var values = new float[U32(offset + 4)];
        for (var index = 0; index < values.Length; index++) values[index] = F32(SectionsByTag["F32S"].Offset + (start + index) * 4);
        return values;
    }

    private Vector3 Vec3(int offset) => new Vector3(F32(offset), F32(offset + 4), F32(offset + 8));

    private Vector4 Vec4(int offset) => new Vector4(F32(offset), F32(offset + 4), F32(offset + 8), F32(offset + 12));

    private Matrix4x4 Matrix(int offset)
    {
        var f = new float[16];
        for (var index = 0; index < 16; index++) f[index] = F32(offset + index * 4);
        return new Matrix4x4(f[0], f[1], f[2], f[3], f[4], f[5], f[6], f[7], f[8], f[9], f[10], f[11], f[12], f[13], f[14], f[15]);
    }

    private Vector3[] Vec3List(int offset)
    {
        var floats = F32List(offset);
        var values = new Vector3[floats.Length / 3];
        for (var index = 0; index < values.Length; index++) values[index] = new Vector3(floats[index * 3], floats[index * 3 + 1], floats[index * 3 + 2]);
        return values;
    }

    private ModelTextureData ReadTexture(int r)
    {
        var length = (int)U32(r + 32);
        var pixels = new byte[length];
        Array.Copy(blob, SectionsByTag["PIX0"].Offset + (int)U32(r + 28), pixels, 0, length);
        return new ModelTextureData
        {
            Name = Str(r),
            Width = (int)U32(r + 8),
            Height = (int)U32(r + 12),
            ColorSpace = (TextureColorSpace)U32(r + 20),
            Mipmaps = U32(r + 24) == 1,
            Pixels = pixels,
        };
    }

    private ModelData ReadModel(int r)
    {
        var model = new ModelData
        {
            Name = Str(r),
            IndexFormat = (ModelIndexFormat)U32(r + 24),
            RootNodes = U32List(r + 64),
            Textures = U32List(r + 88),
            Warnings = StrList(r + 104),
        };

        var vertexStart = SectionsByTag["VTX0"].Offset + (int)U32(r + 8);
        model.Vertices = new ModelVertex[U32(r + 12)];
        for (var index = 0; index < model.Vertices.Length; index++)
        {
            var v = vertexStart + index * 64;
            model.Vertices[index] = new ModelVertex
            {
                Position = Vec3(v),
                Normal = Vec3(v + 12),
                Tangent = Vec4(v + 24),
                TexCoord0 = new Vector2(F32(v + 40), F32(v + 44)),
                TexCoord1 = new Vector2(F32(v + 48), F32(v + 52)),
                ColorR = U16(v + 56),
                ColorG = U16(v + 58),
                ColorB = U16(v + 60),
                ColorA = U16(v + 62),
            };
        }

        var indexStart = SectionsByTag["IDX0"].Offset + (int)U32(r + 16);
        model.Indices = new uint[U32(r + 20)];
        for (var index = 0; index < model.Indices.Length; index++)
        {
            model.Indices[index] = model.IndexFormat == ModelIndexFormat.Uint16 ? U16(indexStart + index * 2) : U32(indexStart + index * 4);
        }

        var skinCount = (int)U32(r + 32);
        if (skinCount > 0)
        {
            var skinStart = SectionsByTag["SKV0"].Offset + (int)U32(r + 28);
            model.SkinVertices = new ModelSkinVertex[skinCount];
            for (var index = 0; index < skinCount; index++)
            {
                var s = skinStart + index * 24;
                model.SkinVertices[index] = new ModelSkinVertex { Joint0 = U16(s), Joint1 = U16(s + 2), Joint2 = U16(s + 4), Joint3 = U16(s + 6), Weights = Vec4(s + 8) };
            }
        }

        for (var index = 0; index < (int)U32(r + 40); index++) model.Meshes.Add(ReadMesh(Record("MESH", (int)U32(r + 36) + index)));
        for (var index = 0; index < (int)U32(r + 48); index++) model.Materials.Add(ReadMaterial(Record("MTRL", (int)U32(r + 44) + index)));
        model.DefaultMaterial = ReadMaterial(Record("MTRL", (int)U32(r + 52)));
        for (var index = 0; index < (int)U32(r + 60); index++) model.Nodes.Add(ReadNode(Record("NODE", (int)U32(r + 56) + index)));
        for (var index = 0; index < (int)U32(r + 76); index++) model.Skins.Add(ReadSkin(Record("SKIN", (int)U32(r + 72) + index)));
        for (var index = 0; index < (int)U32(r + 84); index++) model.Animations.Add(ReadAnimation(Record("ANIM", (int)U32(r + 80) + index)));
        for (var index = 0; index < (int)U32(r + 100); index++) model.Samplers.Add(ReadSampler(Record("SMPL", (int)U32(r + 96) + index)));
        return model;
    }

    private ModelMeshData ReadMesh(int r)
    {
        var mesh = new ModelMeshData { Name = Str(r), DefaultMorphWeights = F32List(r + 16) };
        for (var index = 0; index < (int)U32(r + 12); index++) mesh.Primitives.Add(ReadPrimitive(Record("PRIM", (int)U32(r + 8) + index)));
        return mesh;
    }

    private ModelPrimitiveData ReadPrimitive(int r)
    {
        var flags = U32(r + 48);
        var primitive = new ModelPrimitiveData
        {
            Material = I32(r),
            FirstIndex = U32(r + 4),
            IndexCount = U32(r + 8),
            BaseVertex = (int)U32(r + 12),
            VertexCount = (int)U32(r + 16),
            SkinBaseVertex = I32(r + 20),
            BoundsMin = Vec3(r + 24),
            BoundsMax = Vec3(r + 36),
            HasVertexColors = (flags & 1) != 0,
            HasTexCoord1 = (flags & 2) != 0,
            HasTangents = (flags & 4) != 0,
            TangentsGenerated = (flags & 8) != 0,
            NormalsGenerated = (flags & 16) != 0,
        };
        for (var index = 0; index < (int)U32(r + 56); index++)
        {
            var t = Record("MTGT", (int)U32(r + 52) + index);
            primitive.MorphTargets.Add(new ModelMorphTargetData { PositionDeltas = Vec3List(t), NormalDeltas = Vec3List(t + 8), TangentDeltas = Vec3List(t + 16) });
        }

        return primitive;
    }

    private ModelTextureSlotData ReadSlot(int s) => new ModelTextureSlotData
    {
        Texture = I32(s),
        Sampler = I32(s + 4),
        TexCoord = (int)U32(s + 8),
        HasTransform = U32(s + 12) == 1,
        Transform = new Matrix3x2(F32(s + 16), F32(s + 20), F32(s + 24), F32(s + 28), F32(s + 32), F32(s + 36)),
        TextureIndex = I32(s + 40),
    };

    private ModelMaterialData ReadMaterial(int r) => new ModelMaterialData
    {
        Name = Str(r),
        Index = I32(r + 8),
        BaseColorFactor = Vec4(r + 12),
        BaseColorTexture = ReadSlot(r + 28),
        MetallicFactor = F32(r + 72),
        RoughnessFactor = F32(r + 76),
        MetallicRoughnessTexture = ReadSlot(r + 80),
        NormalTexture = ReadSlot(r + 124),
        NormalScale = F32(r + 168),
        OcclusionTexture = ReadSlot(r + 172),
        OcclusionStrength = F32(r + 216),
        EmissiveFactor = Vec3(r + 220),
        EmissiveStrength = F32(r + 232),
        EmissiveTexture = ReadSlot(r + 236),
        AlphaMode = (MaterialAlphaMode)U32(r + 280),
        AlphaCutoff = F32(r + 284),
        DoubleSided = U32(r + 288) == 1,
        Unlit = U32(r + 292) == 1,
        IndexOfRefraction = F32(r + 296),
        ClearCoatFactor = F32(r + 300),
        ClearCoatRoughness = F32(r + 304),
        SpecularFactor = F32(r + 308),
        SpecularColor = Vec3(r + 312),
        TransmissionFactor = F32(r + 324),
        ApproximatedFeatures = StrList(r + 328),
    };

    private ModelSamplerData ReadSampler(int r) => new ModelSamplerData
    {
        AddressModeU = (SamplerAddressMode)U32(r),
        AddressModeV = (SamplerAddressMode)U32(r + 4),
        MagFilter = (SamplerFilter)U32(r + 8),
        MinFilter = (SamplerFilter)U32(r + 12),
        MipmapFilter = (SamplerFilter)U32(r + 16),
        MaxLod = F32(r + 20),
    };

    private ModelNodeData ReadNode(int r) => new ModelNodeData
    {
        Name = Str(r),
        Parent = I32(r + 8),
        Children = U32List(r + 12),
        Mesh = I32(r + 20),
        Skin = I32(r + 24),
        Translation = Vec3(r + 28),
        Rotation = new Quaternion(F32(r + 40), F32(r + 44), F32(r + 48), F32(r + 52)),
        Scale = Vec3(r + 56),
        HasFixedMatrix = U32(r + 68) == 1,
        LocalMatrix = Matrix(r + 72),
        DefaultMorphWeights = F32List(r + 136),
        InScene = U32(r + 144) == 1,
    };

    private ModelSkinData ReadSkin(int r)
    {
        var skin = new ModelSkinData { Name = Str(r), Joints = U32List(r + 8), Skeleton = I32(r + 24) };
        var floats = F32List(r + 16);
        for (var index = 0; index < floats.Length / 16; index++)
        {
            var f = floats.AsSpan(index * 16, 16);
            skin.InverseBindMatrices.Add(new Matrix4x4(f[0], f[1], f[2], f[3], f[4], f[5], f[6], f[7], f[8], f[9], f[10], f[11], f[12], f[13], f[14], f[15]));
        }

        return skin;
    }

    private ModelAnimationData ReadAnimation(int r)
    {
        var animation = new ModelAnimationData { Name = Str(r) };
        for (var index = 0; index < (int)U32(r + 12); index++)
        {
            var c = Record("CHAN", (int)U32(r + 8) + index);
            animation.Channels.Add(new ModelAnimationChannelData
            {
                Node = (int)U32(c),
                Path = (AnimationPath)U32(c + 4),
                Interpolation = (AnimationInterpolation)U32(c + 8),
                ComponentCount = (int)U32(c + 12),
                Times = F32List(c + 16),
                Values = F32List(c + 24),
            });
        }

        return animation;
    }
}
