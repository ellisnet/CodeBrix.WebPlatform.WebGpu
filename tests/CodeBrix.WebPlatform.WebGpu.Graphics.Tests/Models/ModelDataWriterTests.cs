using System;
using System.IO;
using System.Linq;
using System.Numerics;
using System.Text;
using CodeBrix.WebPlatform.WebGpu.Graphics.Models;
using SilverAssertions;
using Xunit;

namespace CodeBrix.WebPlatform.WebGpu.Graphics.Tests.Models;

public class ModelDataWriterTests
{
    private static readonly string[] SectionOrder =
    {
        "STRG", "U32S", "F32S", "MODL", "MESH", "PRIM", "MTGT", "MTRL", "TXTR",
        "SMPL", "NODE", "SKIN", "ANIM", "CHAN", "VTX0", "IDX0", "SKV0", "PIX0",
    };

    [Fact]
    public void Write_triangle_header_is_the_golden_32_bytes()
    {
        //Arrange
        var bytes = ModelDataWriter.Write(ModelDataFixtures.Triangle());

        //Act
        var header = bytes.Take(32).ToArray();

        //Assert
        header.Should().Equal(new byte[]
        {
            0x43, 0x42, 0x4D, 0x44, 0x01, 0x00, 0x00, 0x00, 0x12, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x07, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        });
    }

    [Fact]
    public void Write_triangle_section_table_is_the_golden_table()
    {
        //Arrange
        var reader = new ModelDataTestReader(ModelDataWriter.Write(ModelDataFixtures.Triangle()));
        var expected = new (string Tag, int RecordSize, int Count, int Offset, int Length)[]
        {
            ("STRG", 1, 34, 464, 34), ("U32S", 4, 1, 512, 4), ("F32S", 4, 0, 528, 0), ("MODL", 120, 1, 528, 120),
            ("MESH", 24, 1, 656, 24), ("PRIM", 64, 1, 688, 64), ("MTGT", 24, 0, 752, 0), ("MTRL", 336, 2, 752, 672),
            ("TXTR", 40, 0, 1424, 0), ("SMPL", 24, 0, 1424, 0), ("NODE", 148, 1, 1424, 148), ("SKIN", 28, 0, 1584, 0),
            ("ANIM", 16, 0, 1584, 0), ("CHAN", 32, 0, 1584, 0), ("VTX0", 1, 192, 1584, 192), ("IDX0", 1, 16, 1776, 16),
            ("SKV0", 1, 0, 1792, 0), ("PIX0", 1, 0, 1792, 0),
        };

        //Act
        var actual = reader.Sections.Select(s => (s.Tag, s.RecordSize, s.Count, s.Offset, s.Length)).ToArray();

        //Assert
        actual.Should().Equal(expected);
        reader.Sections.Should().OnlyContain(s => s.Flags == 0);
    }

    [Fact]
    public void Write_triangle_section_tags_are_little_endian_ascii()
    {
        //Arrange
        var bytes = ModelDataWriter.Write(ModelDataFixtures.Triangle());

        //Act
        var firstTag = BitConverter.ToUInt32(bytes, 32);

        //Assert
        Encoding.ASCII.GetString(bytes, 32, 4).Should().Be("STRG");
        (firstTag == ((uint)'S' | ((uint)'T' << 8) | ((uint)'R' << 16) | ((uint)'G' << 24)) || !BitConverter.IsLittleEndian).Should().BeTrue();
    }

    [Fact]
    public void Write_triangle_strings_and_model_record_are_the_golden_values()
    {
        //Arrange
        var bytes = ModelDataWriter.Write(ModelDataFixtures.Triangle());
        var reader = new ModelDataTestReader(bytes);
        var model = reader.Record("MODL", 0);

        //Act
        var strings = Encoding.UTF8.GetString(bytes, 464, 34);
        var fields = Enumerable.Range(0, 30).Select(i => reader.U32(model + i * 4)).ToArray();

        //Assert
        strings.Should().Be("triangletrianglereddefaulttriangle");
        fields.Should().Equal(new uint[]
        {
            0, 8, 0, 3, 0, 3, 2, 0, 0, 0, 1, 0, 1, 1, 0, 1, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        });
    }

    [Fact]
    public void Write_triangle_primitive_material_and_node_records_are_the_golden_values()
    {
        //Arrange
        var reader = new ModelDataTestReader(ModelDataWriter.Write(ModelDataFixtures.Triangle()));
        var primitive = reader.Record("PRIM", 0);
        var material = reader.Record("MTRL", 0);
        var defaultMaterial = reader.Record("MTRL", 1);
        var node = reader.Record("NODE", 0);

        //Act
        var indices = new[] { reader.U16(1776), reader.U16(1778), reader.U16(1780), reader.U16(1782) };

        //Assert
        reader.I32(primitive).Should().Be(0);
        reader.U32(primitive + 8).Should().Be(3u);
        reader.I32(primitive + 20).Should().Be(-1);
        reader.F32(primitive + 36).Should().Be(1f);
        reader.F32(primitive + 44).Should().Be(0f);
        reader.U32(primitive + 48).Should().Be(0u);
        reader.Str(material).Should().Be("red");
        reader.F32(material + 12).Should().Be(1f);
        reader.F32(material + 16).Should().Be(0f);
        reader.I32(material + 28).Should().Be(-1);
        reader.I32(material + 32).Should().Be(-1);
        reader.F32(material + 44).Should().Be(1f);
        reader.I32(material + 68).Should().Be(-1);
        reader.F32(material + 76).Should().Be(0.5f);
        reader.F32(material + 296).Should().Be(1.5f);
        reader.Str(defaultMaterial).Should().Be("default");
        reader.I32(defaultMaterial + 8).Should().Be(-1);
        reader.Str(node).Should().Be("triangle");
        reader.I32(node + 8).Should().Be(-1);
        reader.I32(node + 20).Should().Be(0);
        reader.F32(node + 52).Should().Be(1f);
        reader.F32(node + 72).Should().Be(1f);
        reader.F32(node + 72 + 15 * 4).Should().Be(1f);
        reader.U32(node + 144).Should().Be(1u);
        indices.Should().Equal(new ushort[] { 0, 1, 2, 0 });
    }

    [Fact]
    public void Write_triangle_vertex_bytes_are_little_endian_records()
    {
        //Arrange
        var bytes = ModelDataWriter.Write(ModelDataFixtures.Triangle());
        var second = 1584 + 64;

        //Act
        var record = bytes.Skip(second).Take(64).ToArray();

        //Assert
        record.Take(12).Should().Equal(new byte[] { 0x00, 0x00, 0x80, 0x3F, 0, 0, 0, 0, 0, 0, 0, 0 });
        record.Skip(20).Take(4).Should().Equal(new byte[] { 0x00, 0x00, 0x80, 0x3F });
        record.Skip(40).Take(4).Should().Equal(new byte[] { 0x00, 0x00, 0x80, 0x3F });
        record.Skip(56).Should().Equal(new byte[] { 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF });
    }

    [Fact]
    public void Write_writes_every_section_in_the_format_order_even_when_empty()
    {
        //Arrange
        var bytes = ModelDataWriter.Write(ModelDataFixtures.Triangle());

        //Act
        var reader = new ModelDataTestReader(bytes);

        //Assert
        reader.SectionCount.Should().Be(18);
        reader.Sections.Select(s => s.Tag).Should().Equal(SectionOrder);
    }

    [Fact]
    public void Write_every_section_model_fills_every_section()
    {
        //Arrange
        var reader = new ModelDataTestReader(ModelDataWriter.Write(ModelDataFixtures.EverySection()));

        //Act
        var counts = reader.Sections.ToDictionary(s => s.Tag, s => s.Count);

        //Assert
        counts.Values.Should().OnlyContain(c => c > 0);
        counts["MODL"].Should().Be(1);
        counts["MESH"].Should().Be(2);
        counts["PRIM"].Should().Be(2);
        counts["MTGT"].Should().Be(1);
        counts["MTRL"].Should().Be(3);
        counts["TXTR"].Should().Be(1);
        counts["SMPL"].Should().Be(1);
        counts["NODE"].Should().Be(5);
        counts["SKIN"].Should().Be(1);
        counts["ANIM"].Should().Be(2);
        counts["CHAN"].Should().Be(4);
        counts["VTX0"].Should().Be(7 * 64);
        counts["IDX0"].Should().Be(48);
        counts["SKV0"].Should().Be(80);
        counts["PIX0"].Should().Be(16);
    }

    [Theory]
    [InlineData("triangle.cbmd")]
    [InlineData("every-section.cbmd")]
    [InlineData("two-models-shared-texture.cbmd")]
    public void Write_aligns_every_section_and_every_range_to_16_bytes(string fixture)
    {
        //Arrange
        var set = ModelDataFixtures.All.Single(f => f.FileName == fixture).Set;
        var bytes = ModelDataWriter.Write(set);
        var reader = new ModelDataTestReader(bytes);

        //Act
        var ranges = Enumerable.Range(0, reader.SectionsByTag["MODL"].Count)
            .SelectMany(m => new[] { 8, 16, 28 }.Select(f => reader.U32(reader.Record("MODL", m) + f)))
            .Concat(Enumerable.Range(0, reader.SectionsByTag["TXTR"].Count).Select(t => reader.U32(reader.Record("TXTR", t) + 28)))
            .ToList();

        //Assert
        (bytes.Length % 16).Should().Be(0);
        reader.U32(16).Should().Be((uint)bytes.Length);
        reader.Sections.Should().OnlyContain(s => s.Offset % 16 == 0 && s.Length == s.RecordSize * s.Count && s.Offset + s.Length <= bytes.Length);
        ranges.Should().OnlyContain(r => r % 16 == 0);
    }

    [Theory]
    [InlineData("triangle.cbmd")]
    [InlineData("every-section.cbmd")]
    [InlineData("two-models-shared-texture.cbmd")]
    public void Write_pads_with_zero_bytes(string fixture)
    {
        //Arrange
        var bytes = ModelDataWriter.Write(ModelDataFixtures.All.Single(f => f.FileName == fixture).Set);
        var reader = new ModelDataTestReader(bytes);
        var used = new bool[bytes.Length];
        for (var index = 0; index < 32 + reader.SectionCount * 24; index++) used[index] = true;
        foreach (var section in reader.Sections)
        {
            for (var index = section.Offset; index < section.Offset + section.Length; index++) used[index] = true;
        }

        //Act
        var gaps = Enumerable.Range(0, bytes.Length).Where(i => !used[i]).Select(i => bytes[i]);

        //Assert
        gaps.Should().OnlyContain(b => b == 0);
    }

    [Fact]
    public void Write_round_trips_the_every_section_model()
    {
        //Arrange
        var original = ModelDataFixtures.EverySection();

        //Act
        var read = new ModelDataTestReader(ModelDataWriter.Write(original)).Read();

        //Assert
        var model = read.Models.Single();
        var source = original.Models[0];
        model.Name.Should().Be("every-section");
        model.Vertices.Should().Equal(source.Vertices);
        model.Indices.Should().Equal(source.Indices);
        model.IndexFormat.Should().Be(ModelIndexFormat.Uint32);
        model.SkinVertices.Should().Equal(source.SkinVertices);
        model.Warnings.Should().Equal("points primitive skipped", "image 1 could not be decoded");
        model.Textures.Should().Equal(0);
        model.RootNodes.Should().Equal(0);
        model.Samplers.Single().AddressModeV.Should().Be(SamplerAddressMode.MirrorRepeat);
        model.Samplers.Single().MaxLod.Should().Be(0f);
        model.Meshes.Select(m => m.Name).Should().Equal("quad", "skinned-triangle");
        model.Meshes[1].DefaultMorphWeights.Should().Equal(0.25f);
        var skinned = model.Meshes[1].Primitives.Single();
        skinned.Material.Should().Be(-1);
        skinned.FirstIndex.Should().Be(6u);
        skinned.BaseVertex.Should().Be(4);
        skinned.SkinBaseVertex.Should().Be(0);
        skinned.HasTexCoord1.Should().BeTrue();
        skinned.NormalsGenerated.Should().BeTrue();
        skinned.MorphTargets.Single().PositionDeltas.Should().Equal(new Vector3(0, 0, 0.5f), new Vector3(0, 0, 0.5f), new Vector3(0, 0, 0.5f));
        skinned.MorphTargets.Single().NormalDeltas.Should().BeEmpty();
        var quad = model.Meshes[0].Primitives.Single();
        (quad.HasVertexColors && quad.HasTangents && quad.TangentsGenerated && !quad.HasTexCoord1).Should().BeTrue();
        model.Materials.Select(m => m.Name).Should().Equal("textured", "clearcoat");
        model.Materials[0].BaseColorTexture.Transform.Should().Be(new Matrix3x2(2, 0, 0, 2, 0.5f, 0.25f));
        model.Materials[0].BaseColorTexture.HasTransform.Should().BeTrue();
        model.Materials[0].AlphaMode.Should().Be(MaterialAlphaMode.Mask);
        model.Materials[1].ApproximatedFeatures.Should().Equal("KHR_materials_clearcoat", "KHR_materials_transmission");
        model.Materials[1].TransmissionFactor.Should().Be(0.375f);
        model.Materials[1].OcclusionTexture.TexCoord.Should().Be(1);
        model.Materials[1].Unlit.Should().BeTrue();
        model.DefaultMaterial.Index.Should().Be(-1);
        model.DefaultMaterial.Name.Should().Be("default");
        model.Nodes.Select(n => n.Name).Should().Equal("root", "quad-node", "skinned-node", "joint-0", "joint-1");
        model.Nodes[0].Children.Should().Equal(1, 2, 3);
        model.Nodes[1].HasFixedMatrix.Should().BeTrue();
        model.Nodes[1].LocalMatrix.M21.Should().Be(0.5f);
        model.Nodes[2].DefaultMorphWeights.Should().Equal(0.25f);
        model.Nodes[4].Rotation.Should().Be(new Quaternion(0, 0, 0.70710677f, 0.70710677f));
        model.Skins.Single().Joints.Should().Equal(3, 4);
        model.Skins.Single().InverseBindMatrices.Should().Equal(Matrix4x4.CreateTranslation(0, -1, 0), Matrix4x4.CreateTranslation(0, -2, 0));
        model.Skins.Single().Skeleton.Should().Be(3);
        model.Animations.Select(a => a.Name).Should().Equal("move", "idle");
        model.Animations[0].Channels.Select(c => c.Interpolation).Should().Equal(AnimationInterpolation.Step, AnimationInterpolation.Linear, AnimationInterpolation.CubicSpline, AnimationInterpolation.Linear);
        model.Animations[0].Channels.Select(c => c.Path).Should().Equal(AnimationPath.Translation, AnimationPath.Rotation, AnimationPath.Scale, AnimationPath.Weights);
        model.Animations[0].Channels[2].Values.Should().Equal(source.Animations[0].Channels[2].Values);
        model.Animations[1].Channels.Should().BeEmpty();
        read.Textures.Single().Pixels.Should().Equal(original.Textures[0].Pixels);
        read.Textures.Single().ColorSpace.Should().Be(TextureColorSpace.Srgb);
        read.Textures.Single().Mipmaps.Should().BeTrue();
    }

    [Fact]
    public void Write_stores_the_trs_matrix_for_a_node_without_a_fixed_matrix()
    {
        //Arrange
        var set = ModelDataFixtures.EverySection();
        var node = set.Models[0].Nodes[4];
        node.LocalMatrix = Matrix4x4.CreateScale(9f);

        //Act
        var read = new ModelDataTestReader(ModelDataWriter.Write(set)).Read();

        //Assert
        var expected = Matrix4x4.CreateScale(node.Scale) * Matrix4x4.CreateFromQuaternion(node.Rotation) * Matrix4x4.CreateTranslation(node.Translation);
        var actual = read.Models[0].Nodes[4].LocalMatrix;
        for (var row = 0; row < 4; row++)
        {
            for (var column = 0; column < 4; column++) actual[row, column].Should().BeApproximately(expected[row, column], 1e-6f);
        }

        read.Models[0].Nodes[0].LocalMatrix.Should().Be(new Matrix4x4(2, 0, 0, 0, 0, 2, 0, 0, 0, 0, 2, 0, 1, 2, 3, 1));
    }

    [Fact]
    public void Write_stores_identities_for_a_skin_without_inverse_bind_matrices()
    {
        //Arrange
        var set = ModelDataFixtures.EverySection();
        set.Models[0].Skins[0].InverseBindMatrices.Clear();

        //Act
        var read = new ModelDataTestReader(ModelDataWriter.Write(set)).Read();

        //Assert
        read.Models[0].Skins[0].InverseBindMatrices.Should().Equal(Matrix4x4.Identity, Matrix4x4.Identity);
    }

    [Fact]
    public void Write_shares_a_texture_between_two_models_once()
    {
        //Arrange
        var bytes = ModelDataWriter.Write(ModelDataFixtures.TwoModelsSharedTexture());
        var reader = new ModelDataTestReader(bytes);

        //Act
        var read = reader.Read();

        //Assert
        read.Textures.Select(t => t.Name).Should().Equal("shared", "second-only");
        read.Models.Select(m => m.Name).Should().Equal("first", "second");
        read.Models[0].Textures.Should().Equal(0);
        read.Models[1].Textures.Should().Equal(1, 0);
        read.Models[1].Materials.Single().BaseColorTexture.Texture.Should().Be(1);
        reader.U32(reader.Record("MODL", 1) + 16).Should().Be(16u);
        reader.U32(reader.Record("MODL", 1) + 8).Should().Be(192u);
        reader.U32(reader.Record("MODL", 1) + 52).Should().Be(3u);
        reader.U32(reader.Record("MODL", 1) + 36).Should().Be(1u);
        reader.U32(reader.Record("TXTR", 1) + 28).Should().Be(16u);
        reader.U32(reader.Record("TXTR", 1) + 24).Should().Be(0u);
        reader.SectionsByTag["IDX0"].Length.Should().Be(32);
    }

    [Fact]
    public void Write_writes_u16_indices_as_two_bytes_and_u32_as_four()
    {
        //Arrange
        var narrow = ModelDataFixtures.Triangle();
        var wide = ModelDataFixtures.Triangle();
        wide.Models[0].IndexFormat = ModelIndexFormat.Uint32;

        //Act
        var narrowReader = new ModelDataTestReader(ModelDataWriter.Write(narrow));
        var wideReader = new ModelDataTestReader(ModelDataWriter.Write(wide));

        //Assert
        narrowReader.SectionsByTag["IDX0"].Length.Should().Be(16);
        wideReader.SectionsByTag["IDX0"].Length.Should().Be(16);
        narrowReader.U32(narrowReader.Record("MODL", 0) + 24).Should().Be(2u);
        wideReader.U32(wideReader.Record("MODL", 0) + 24).Should().Be(4u);
        wideReader.Read().Models[0].Indices.Should().Equal(0u, 1u, 2u);
        narrowReader.Read().Models[0].Indices.Should().Equal(0u, 1u, 2u);
    }

    [Fact]
    public void Write_writes_null_and_empty_strings_as_the_format_says()
    {
        //Arrange
        var set = ModelDataFixtures.Triangle();
        set.Models[0].Name = null;
        set.Models[0].Meshes[0].Name = string.Empty;

        //Act
        var reader = new ModelDataTestReader(ModelDataWriter.Write(set));

        //Assert
        reader.U32(reader.Record("MODL", 0)).Should().Be(uint.MaxValue);
        reader.U32(reader.Record("MODL", 0) + 4).Should().Be(0u);
        reader.U32(reader.Record("MESH", 0)).Should().Be(0u);
        reader.U32(reader.Record("MESH", 0) + 4).Should().Be(0u);
        reader.Read().Models[0].Name.Should().BeNull();
    }

    [Fact]
    public void Write_is_deterministic_and_the_three_overloads_agree()
    {
        //Arrange
        var first = ModelDataWriter.Write(ModelDataFixtures.EverySection());
        using var stream = new MemoryStream();
        var set = ModelDataFixtures.EverySection();

        //Act
        var second = ModelDataWriter.Write(ModelDataFixtures.EverySection());
        ModelDataWriter.Write(ModelDataFixtures.EverySection(), stream);
        var single = ModelDataWriter.Write(set.Models[0], set.Textures);

        //Assert
        second.Should().Equal(first);
        stream.ToArray().Should().Equal(first);
        single.Should().Equal(first);
    }

    [Fact]
    public void ModelDataFormat_constants_are_the_format_s()
    {
        //Arrange
        var magic = ModelDataFormat.Magic.ToArray();

        //Act
        var text = Encoding.ASCII.GetString(magic);

        //Assert
        text.Should().Be("CBMD");
        ModelDataFormat.MajorVersion.Should().Be(1);
        ModelDataFormat.MinorVersion.Should().Be(0);
        ModelDataFormat.FileExtension.Should().Be(".cbmd");
        ModelDataFormat.ContentType.Should().Be("application/octet-stream");
        ModelDataFormat.MaxTextureDimension.Should().Be(8192);
    }

    [Fact]
    public void ModelVertex_and_ModelSkinVertex_have_the_format_s_sizes()
    {
        //Arrange
        var vertex = System.Runtime.CompilerServices.Unsafe.SizeOf<ModelVertex>();

        //Act
        var skin = System.Runtime.CompilerServices.Unsafe.SizeOf<ModelSkinVertex>();

        //Assert
        vertex.Should().Be(ModelVertex.Size).And.Be(64);
        skin.Should().Be(ModelSkinVertex.Size).And.Be(24);
    }

    [Fact]
    public void ModelData_defaults_are_the_csharp_library_s()
    {
        //Arrange
        var model = new ModelData();

        //Act
        var material = new ModelMaterialData();

        //Assert
        model.DefaultMaterial.Index.Should().Be(-1);
        model.DefaultMaterial.Name.Should().Be("default");
        model.SkinVertices.Should().BeNull();
        material.BaseColorFactor.Should().Be(Vector4.One);
        material.AlphaCutoff.Should().Be(0.5f);
        material.IndexOfRefraction.Should().Be(1.5f);
        material.SpecularColor.Should().Be(Vector3.One);
        material.BaseColorTexture.Texture.Should().Be(-1);
        material.BaseColorTexture.Transform.Should().Be(Matrix3x2.Identity);
        new ModelPrimitiveData().Material.Should().Be(-1);
        new ModelPrimitiveData().SkinBaseVertex.Should().Be(-1);
        new ModelNodeData().Rotation.Should().Be(Quaternion.Identity);
        new ModelSamplerData().MaxLod.Should().Be(32f);
        new ModelTextureData().Mipmaps.Should().BeTrue();
        ModelTextureSlotData.Empty.Should().NotBeSameAs(ModelTextureSlotData.Empty);
    }
}
