using System;
using System.Buffers.Binary;
using System.Collections.Generic;
using System.IO;
using System.Numerics;

namespace CodeBrix.WebPlatform.WebGpu.Graphics.Models.Internal;

/// <summary>
/// Lays out and writes one validated model set in the model data format (version 1.0). The tables and pools are built
/// in memory first, in a fixed order (model by model; in each model its name, root nodes, textures and warnings,
/// then its meshes with their primitives and morph targets, materials, default material, nodes, skins, animations
/// with their channels and samplers; then the set's textures), so the output is deterministic. The bulk sections
/// (vertices, indices, skin vertices, pixels) are streamed from the set's arrays.
/// </summary>
internal sealed class ModelDataEncoder
{
    private const int StringSection = 0;
    private const int U32Section = 1;
    private const int F32Section = 2;
    private const int VertexSection = 14;
    private const int IndexSection = 15;
    private const int SkinVertexSection = 16;
    private const int PixelSection = 17;

    private readonly ModelDataSet set;
    private readonly LittleEndianBuffer[] tables = new LittleEndianBuffer[14];
    private readonly long[] bulkLengths = new long[4];
    private readonly long[] sectionOffsets = new long[ModelDataSection.Tags.Length];
    private readonly long[] sectionLengths = new long[ModelDataSection.Tags.Length];
    private readonly List<long> vertexOffsets = new List<long>();
    private readonly List<long> indexOffsets = new List<long>();
    private readonly List<long> skinVertexOffsets = new List<long>();
    private readonly List<long> pixelOffsets = new List<long>();
    private int meshCount;
    private int primitiveCount;
    private int morphTargetCount;
    private int materialCount;
    private int nodeCount;
    private int skinCount;
    private int animationCount;
    private int channelCount;
    private int samplerCount;

    /// <summary>Lays out a validated set (<see cref="ModelDataValidator.Validate" />).</summary>
    /// <param name="set">The set.</param>
    public ModelDataEncoder(ModelDataSet set)
    {
        this.set = set;
        for (var index = 0; index < tables.Length; index++) tables[index] = new LittleEndianBuffer();
        LayOutBulk();
        var models = ModelDataValidator.List(set.Models);
        for (var index = 0; index < models.Count; index++) EncodeModel(models[index], index);
        var textures = ModelDataValidator.List(set.Textures);
        for (var index = 0; index < textures.Count; index++) EncodeTexture(textures[index], index);
        LayOutSections();
    }

    /// <summary>The total length of the blob in bytes (a multiple of 16).</summary>
    public long TotalLength { get; private set; }

    private LittleEndianBuffer Strings => tables[StringSection];

    private LittleEndianBuffer U32s => tables[U32Section];

    private LittleEndianBuffer F32s => tables[F32Section];

    private LittleEndianBuffer Models => tables[3];

    private LittleEndianBuffer Meshes => tables[4];

    private LittleEndianBuffer Primitives => tables[5];

    private LittleEndianBuffer MorphTargets => tables[6];

    private LittleEndianBuffer Materials => tables[7];

    private LittleEndianBuffer Textures => tables[8];

    private LittleEndianBuffer Samplers => tables[9];

    private LittleEndianBuffer Nodes => tables[10];

    private LittleEndianBuffer Skins => tables[11];

    private LittleEndianBuffer Animations => tables[12];

    private LittleEndianBuffer Channels => tables[13];

    /// <summary>Writes the blob.</summary>
    /// <param name="destination">The stream.</param>
    public void WriteTo(Stream destination)
    {
        var header = new byte[ModelDataSection.HeaderSize + ModelDataSection.Tags.Length * ModelDataSection.TableEntrySize];
        var span = header.AsSpan();
        ModelDataFormat.Magic.CopyTo(span);
        BinaryPrimitives.WriteUInt16LittleEndian(span.Slice(4), ModelDataFormat.MajorVersion);
        BinaryPrimitives.WriteUInt16LittleEndian(span.Slice(6), ModelDataFormat.MinorVersion);
        BinaryPrimitives.WriteUInt32LittleEndian(span.Slice(8), (uint)ModelDataSection.Tags.Length);
        BinaryPrimitives.WriteUInt32LittleEndian(span.Slice(16), (uint)TotalLength);
        for (var section = 0; section < ModelDataSection.Tags.Length; section++)
        {
            var entry = span.Slice(ModelDataSection.HeaderSize + section * ModelDataSection.TableEntrySize);
            var recordSize = ModelDataSection.RecordSizes[section];
            BinaryPrimitives.WriteUInt32LittleEndian(entry, ModelDataSection.TagValue(ModelDataSection.Tags[section]));
            BinaryPrimitives.WriteUInt32LittleEndian(entry.Slice(4), (uint)recordSize);
            BinaryPrimitives.WriteUInt32LittleEndian(entry.Slice(8), (uint)(sectionLengths[section] / recordSize));
            BinaryPrimitives.WriteUInt32LittleEndian(entry.Slice(16), (uint)sectionOffsets[section]);
            BinaryPrimitives.WriteUInt32LittleEndian(entry.Slice(20), (uint)sectionLengths[section]);
        }

        destination.Write(header, 0, header.Length);
        var chunk = new ChunkWriter(destination, header.Length);
        for (var section = 0; section < ModelDataSection.Tags.Length; section++)
        {
            chunk.PadTo(sectionOffsets[section]);
            if (section < tables.Length)
            {
                chunk.Flush();
                tables[section].WriteTo(destination);
                chunk.Advance(tables[section].Length);
            }
            else
            {
                WriteBulk(section, chunk, sectionOffsets[section]);
            }
        }

        chunk.PadTo(TotalLength);
        chunk.Flush();
    }

    private void WriteBulk(int section, ChunkWriter chunk, long start)
    {
        var models = ModelDataValidator.List(set.Models);
        if (section == PixelSection)
        {
            var textures = ModelDataValidator.List(set.Textures);
            for (var index = 0; index < textures.Count; index++)
            {
                chunk.PadTo(start + pixelOffsets[index]);
                chunk.Bytes(textures[index].Pixels);
            }

            chunk.PadTo(start + bulkLengths[3]);
            return;
        }

        for (var index = 0; index < models.Count; index++)
        {
            var model = models[index];
            if (section == VertexSection)
            {
                chunk.PadTo(start + vertexOffsets[index]);
                foreach (var vertex in ModelDataValidator.List(model.Vertices)) chunk.Vertex(vertex);
            }
            else if (section == IndexSection)
            {
                chunk.PadTo(start + indexOffsets[index]);
                var wide = model.IndexFormat == ModelIndexFormat.Uint32;
                foreach (var value in ModelDataValidator.List(model.Indices))
                {
                    if (wide) chunk.U32(value);
                    else chunk.U16((ushort)value);
                }
            }
            else
            {
                var skinVertices = ModelDataValidator.List(model.SkinVertices);
                if (skinVertices.Count == 0) continue;
                chunk.PadTo(start + skinVertexOffsets[index]);
                foreach (var vertex in skinVertices) chunk.SkinVertex(vertex);
            }
        }

        chunk.PadTo(start + bulkLengths[section - VertexSection]);
    }

    private void LayOutBulk()
    {
        foreach (var model in ModelDataValidator.List(set.Models))
        {
            vertexOffsets.Add(bulkLengths[0]);
            bulkLengths[0] = ModelDataSection.Align(bulkLengths[0] + (long)ModelDataValidator.List(model.Vertices).Count * ModelVertex.Size);
            indexOffsets.Add(bulkLengths[1]);
            bulkLengths[1] = ModelDataSection.Align(bulkLengths[1] + (long)ModelDataValidator.List(model.Indices).Count * (int)model.IndexFormat);
            var skinVertices = ModelDataValidator.List(model.SkinVertices).Count;
            skinVertexOffsets.Add(skinVertices == 0 ? 0 : bulkLengths[2]);
            bulkLengths[2] = ModelDataSection.Align(bulkLengths[2] + (long)skinVertices * ModelSkinVertex.Size);
        }

        foreach (var texture in ModelDataValidator.List(set.Textures))
        {
            pixelOffsets.Add(bulkLengths[3]);
            bulkLengths[3] = ModelDataSection.Align(bulkLengths[3] + texture.Pixels.Length);
        }
    }

    private void LayOutSections()
    {
        long position = ModelDataSection.HeaderSize + ModelDataSection.Tags.Length * ModelDataSection.TableEntrySize;
        for (var section = 0; section < ModelDataSection.Tags.Length; section++)
        {
            position = ModelDataSection.Align(position);
            sectionOffsets[section] = position;
            sectionLengths[section] = section < tables.Length ? tables[section].Length : bulkLengths[section - VertexSection];
            position += sectionLengths[section];
        }

        TotalLength = ModelDataSection.Align(position);
    }

    private void EncodeModel(ModelData model, int index)
    {
        var record = Models;
        var start = record.Length;
        var materials = ModelDataValidator.List(model.Materials);
        var meshes = ModelDataValidator.List(model.Meshes);
        var nodes = ModelDataValidator.List(model.Nodes);
        var skins = ModelDataValidator.List(model.Skins);
        var animations = ModelDataValidator.List(model.Animations);
        var samplers = ModelDataValidator.List(model.Samplers);
        var skinVertexCount = ModelDataValidator.List(model.SkinVertices).Count;

        StrRef(record, model.Name);
        record.U32((uint)vertexOffsets[index]);
        record.U32((uint)ModelDataValidator.List(model.Vertices).Count);
        record.U32((uint)indexOffsets[index]);
        record.U32((uint)ModelDataValidator.List(model.Indices).Count);
        record.U32((uint)model.IndexFormat);
        record.U32((uint)skinVertexOffsets[index]);
        record.U32((uint)skinVertexCount);
        record.U32((uint)meshCount);
        record.U32((uint)meshes.Count);
        record.U32((uint)materialCount);
        record.U32((uint)materials.Count);
        record.U32((uint)(materialCount + materials.Count));
        record.U32((uint)nodeCount);
        record.U32((uint)nodes.Count);
        U32List(record, ModelDataValidator.List(model.RootNodes));
        record.U32((uint)skinCount);
        record.U32((uint)skins.Count);
        record.U32((uint)animationCount);
        record.U32((uint)animations.Count);
        U32List(record, ModelDataValidator.List(model.Textures));
        record.U32((uint)samplerCount);
        record.U32((uint)samplers.Count);
        StrList(record, ModelDataValidator.List(model.Warnings));
        record.U32(0);
        record.U32(0);
        CheckRecord(record, start, 3);

        foreach (var mesh in meshes) EncodeMesh(mesh);
        foreach (var material in materials) EncodeMaterial(material);
        EncodeMaterial(model.DefaultMaterial);
        foreach (var node in nodes) EncodeNode(node);
        foreach (var skin in skins) EncodeSkin(skin);
        foreach (var animation in animations) EncodeAnimation(animation);
        foreach (var sampler in samplers) EncodeSampler(sampler);
    }

    private void EncodeMesh(ModelMeshData mesh)
    {
        var record = Meshes;
        var start = record.Length;
        var primitives = ModelDataValidator.List(mesh.Primitives);
        StrRef(record, mesh.Name);
        record.U32((uint)primitiveCount);
        record.U32((uint)primitives.Count);
        F32List(record, ModelDataValidator.List(mesh.DefaultMorphWeights));
        CheckRecord(record, start, 4);
        meshCount++;
        foreach (var primitive in primitives) EncodePrimitive(primitive);
    }

    private void EncodePrimitive(ModelPrimitiveData primitive)
    {
        var record = Primitives;
        var start = record.Length;
        var targets = ModelDataValidator.List(primitive.MorphTargets);
        record.I32(primitive.Material);
        record.U32(primitive.FirstIndex);
        record.U32(primitive.IndexCount);
        record.U32((uint)primitive.BaseVertex);
        record.U32((uint)primitive.VertexCount);
        record.I32(primitive.SkinBaseVertex);
        Vector3(record, primitive.BoundsMin);
        Vector3(record, primitive.BoundsMax);
        var flags = (primitive.HasVertexColors ? 1u : 0u)
            | (primitive.HasTexCoord1 ? 2u : 0u)
            | (primitive.HasTangents ? 4u : 0u)
            | (primitive.TangentsGenerated ? 8u : 0u)
            | (primitive.NormalsGenerated ? 16u : 0u);
        record.U32(flags);
        record.U32((uint)morphTargetCount);
        record.U32((uint)targets.Count);
        record.U32(0);
        CheckRecord(record, start, 5);
        primitiveCount++;
        foreach (var target in targets)
        {
            var targetStart = MorphTargets.Length;
            Vector3List(MorphTargets, ModelDataValidator.List(target.PositionDeltas));
            Vector3List(MorphTargets, ModelDataValidator.List(target.NormalDeltas));
            Vector3List(MorphTargets, ModelDataValidator.List(target.TangentDeltas));
            CheckRecord(MorphTargets, targetStart, 6);
            morphTargetCount++;
        }
    }

    private void EncodeMaterial(ModelMaterialData material)
    {
        var record = Materials;
        var start = record.Length;
        StrRef(record, material.Name);
        record.I32(material.Index);
        Vector4(record, material.BaseColorFactor);
        Slot(record, material.BaseColorTexture);
        record.F32(material.MetallicFactor);
        record.F32(material.RoughnessFactor);
        Slot(record, material.MetallicRoughnessTexture);
        Slot(record, material.NormalTexture);
        record.F32(material.NormalScale);
        Slot(record, material.OcclusionTexture);
        record.F32(material.OcclusionStrength);
        Vector3(record, material.EmissiveFactor);
        record.F32(material.EmissiveStrength);
        Slot(record, material.EmissiveTexture);
        record.U32((uint)material.AlphaMode);
        record.F32(material.AlphaCutoff);
        record.Bool(material.DoubleSided);
        record.Bool(material.Unlit);
        record.F32(material.IndexOfRefraction);
        record.F32(material.ClearCoatFactor);
        record.F32(material.ClearCoatRoughness);
        record.F32(material.SpecularFactor);
        Vector3(record, material.SpecularColor);
        record.F32(material.TransmissionFactor);
        StrList(record, ModelDataValidator.List(material.ApproximatedFeatures));
        CheckRecord(record, start, 7);
        materialCount++;
    }

    private static void Slot(LittleEndianBuffer record, ModelTextureSlotData slot)
    {
        slot ??= ModelTextureSlotData.Empty;
        record.I32(slot.Texture);
        record.I32(slot.Texture < 0 ? -1 : slot.Sampler);
        record.U32((uint)slot.TexCoord);
        record.Bool(slot.HasTransform);
        var transform = slot.Transform;
        record.F32(transform.M11);
        record.F32(transform.M12);
        record.F32(transform.M21);
        record.F32(transform.M22);
        record.F32(transform.M31);
        record.F32(transform.M32);
        record.I32(slot.TextureIndex);
    }

    private void EncodeNode(ModelNodeData node)
    {
        var record = Nodes;
        var start = record.Length;
        StrRef(record, node.Name);
        record.I32(node.Parent);
        U32List(record, ModelDataValidator.List(node.Children));
        record.I32(node.Mesh);
        record.I32(node.Skin);
        Vector3(record, node.Translation);
        var rotation = node.Rotation;
        record.F32(rotation.X);
        record.F32(rotation.Y);
        record.F32(rotation.Z);
        record.F32(rotation.W);
        Vector3(record, node.Scale);
        record.Bool(node.HasFixedMatrix);
        Matrix(record, node.HasFixedMatrix ? node.LocalMatrix : TrsMatrix(node.Translation, node.Rotation, node.Scale));
        F32List(record, ModelDataValidator.List(node.DefaultMorphWeights));
        record.Bool(node.InScene);
        CheckRecord(record, start, 10);
        nodeCount++;
    }

    private void EncodeSkin(ModelSkinData skin)
    {
        var record = Skins;
        var start = record.Length;
        var joints = ModelDataValidator.List(skin.Joints);
        var matrices = ModelDataValidator.List(skin.InverseBindMatrices);
        StrRef(record, skin.Name);
        U32List(record, joints);
        if (joints.Count == 0)
        {
            record.U32(0);
            record.U32(0);
        }
        else
        {
            record.U32((uint)(F32s.Length / 4));
            record.U32((uint)(joints.Count * 16));
            for (var joint = 0; joint < joints.Count; joint++) MatrixTo(F32s, matrices.Count == 0 ? Matrix4x4.Identity : matrices[joint]);
        }

        record.I32(skin.Skeleton);
        CheckRecord(record, start, 11);
        skinCount++;
    }

    private void EncodeAnimation(ModelAnimationData animation)
    {
        var record = Animations;
        var start = record.Length;
        var channels = ModelDataValidator.List(animation.Channels);
        StrRef(record, animation.Name);
        record.U32((uint)channelCount);
        record.U32((uint)channels.Count);
        CheckRecord(record, start, 12);
        animationCount++;
        foreach (var channel in channels)
        {
            var channelStart = Channels.Length;
            Channels.U32((uint)channel.Node);
            Channels.U32((uint)channel.Path);
            Channels.U32((uint)channel.Interpolation);
            Channels.U32((uint)channel.ComponentCount);
            F32List(Channels, ModelDataValidator.List(channel.Times));
            F32List(Channels, ModelDataValidator.List(channel.Values));
            CheckRecord(Channels, channelStart, 13);
            channelCount++;
        }
    }

    private void EncodeSampler(ModelSamplerData sampler)
    {
        var record = Samplers;
        var start = record.Length;
        record.U32((uint)sampler.AddressModeU);
        record.U32((uint)sampler.AddressModeV);
        record.U32((uint)sampler.MagFilter);
        record.U32((uint)sampler.MinFilter);
        record.U32((uint)sampler.MipmapFilter);
        record.F32(sampler.MaxLod);
        CheckRecord(record, start, 9);
        samplerCount++;
    }

    private void EncodeTexture(ModelTextureData texture, int index)
    {
        var record = Textures;
        var start = record.Length;
        StrRef(record, texture.Name);
        record.U32((uint)texture.Width);
        record.U32((uint)texture.Height);
        record.U32(1);
        record.U32((uint)texture.ColorSpace);
        record.Bool(texture.Mipmaps);
        record.U32((uint)pixelOffsets[index]);
        record.U32((uint)texture.Pixels.Length);
        record.U32(0);
        CheckRecord(record, start, 8);
    }

    private void StrRef(LittleEndianBuffer record, string value)
    {
        if (value == null)
        {
            record.U32(uint.MaxValue);
            record.U32(0);
        }
        else if (value.Length == 0)
        {
            record.U32(0);
            record.U32(0);
        }
        else
        {
            var offset = Strings.Length;
            var length = Strings.Utf8(value);
            record.U32((uint)offset);
            record.U32((uint)length);
        }
    }

    private void StrList(LittleEndianBuffer record, IReadOnlyList<string> values)
    {
        record.U32(values.Count == 0 ? 0u : (uint)(U32s.Length / 4));
        record.U32((uint)values.Count);
        foreach (var value in values) StrRef(U32s, value);
    }

    private void U32List(LittleEndianBuffer record, IReadOnlyList<int> values)
    {
        record.U32(values.Count == 0 ? 0u : (uint)(U32s.Length / 4));
        record.U32((uint)values.Count);
        foreach (var value in values) U32s.U32((uint)value);
    }

    private void F32List(LittleEndianBuffer record, IReadOnlyList<float> values)
    {
        record.U32(values.Count == 0 ? 0u : (uint)(F32s.Length / 4));
        record.U32((uint)values.Count);
        foreach (var value in values) F32s.F32(value);
    }

    private void Vector3List(LittleEndianBuffer record, IReadOnlyList<Vector3> values)
    {
        record.U32(values.Count == 0 ? 0u : (uint)(F32s.Length / 4));
        record.U32((uint)(values.Count * 3));
        foreach (var value in values) Vector3(F32s, value);
    }

    private static void Vector3(LittleEndianBuffer buffer, Vector3 value)
    {
        buffer.F32(value.X);
        buffer.F32(value.Y);
        buffer.F32(value.Z);
    }

    private static void Vector4(LittleEndianBuffer buffer, Vector4 value)
    {
        buffer.F32(value.X);
        buffer.F32(value.Y);
        buffer.F32(value.Z);
        buffer.F32(value.W);
    }

    private static void Matrix(LittleEndianBuffer record, Matrix4x4 value) => MatrixTo(record, value);

    private static void MatrixTo(LittleEndianBuffer buffer, Matrix4x4 m)
    {
        buffer.F32(m.M11);
        buffer.F32(m.M12);
        buffer.F32(m.M13);
        buffer.F32(m.M14);
        buffer.F32(m.M21);
        buffer.F32(m.M22);
        buffer.F32(m.M23);
        buffer.F32(m.M24);
        buffer.F32(m.M31);
        buffer.F32(m.M32);
        buffer.F32(m.M33);
        buffer.F32(m.M34);
        buffer.F32(m.M41);
        buffer.F32(m.M42);
        buffer.F32(m.M43);
        buffer.F32(m.M44);
    }

    /// <summary>
    /// The matrix scale, then rotation, then translation (System.Numerics row-vector order), computed with plain
    /// scalar arithmetic so that the bytes are the same on every host.
    /// </summary>
    /// <param name="t">The translation.</param>
    /// <param name="q">The rotation.</param>
    /// <param name="s">The scale.</param>
    /// <returns>The matrix.</returns>
    internal static Matrix4x4 TrsMatrix(Vector3 t, Quaternion q, Vector3 s)
    {
        float xx = q.X * q.X, yy = q.Y * q.Y, zz = q.Z * q.Z;
        float xy = q.X * q.Y, wz = q.Z * q.W, xz = q.Z * q.X;
        float wy = q.Y * q.W, yz = q.Y * q.Z, wx = q.X * q.W;
        return new Matrix4x4(
            (1f - 2f * (yy + zz)) * s.X, 2f * (xy + wz) * s.X, 2f * (xz - wy) * s.X, 0f,
            2f * (xy - wz) * s.Y, (1f - 2f * (zz + xx)) * s.Y, 2f * (yz + wx) * s.Y, 0f,
            2f * (xz + wy) * s.Z, 2f * (yz - wx) * s.Z, (1f - 2f * (yy + xx)) * s.Z, 0f,
            t.X, t.Y, t.Z, 1f);
    }

    private static void CheckRecord(LittleEndianBuffer table, int start, int section)
    {
        if (table.Length - start != ModelDataSection.RecordSizes[section])
        {
            throw new InvalidOperationException("Internal error: a " + ModelDataSection.Tags[section] + " record took " + (table.Length - start) + " bytes instead of " + ModelDataSection.RecordSizes[section] + "; report this as a bug of CodeBrix.WebPlatform.WebGpu.Graphics.");
        }
    }
}
