using System;
using System.IO;
using System.Numerics;
using CodeBrix.WebPlatform.WebGpu.Graphics.Models;
using SilverAssertions;
using Xunit;

namespace CodeBrix.WebPlatform.WebGpu.Graphics.Tests.Models;

/// <summary>Every writer check: what it refuses and that its message says what is wrong and what to do.</summary>
public class ModelDataWriterChecksTests
{
    private static string Refusal(Action<ModelDataSet> change)
    {
        var set = ModelDataFixtures.EverySection();
        set.Models[0].Name = "tree";
        change(set);
        Action act = () => ModelDataWriter.Write(set);
        return act.Should().Throw<ArgumentException>().Which.Message;
    }

    private static ModelData Model(ModelDataSet set) => set.Models[0];

    private static ModelPrimitiveData Quad(ModelDataSet set) => set.Models[0].Meshes[0].Primitives[0];

    private static ModelPrimitiveData Skinned(ModelDataSet set) => set.Models[0].Meshes[1].Primitives[0];

    [Fact]
    public void Write_refuses_a_null_set()
    {
        //Arrange
        Action act = () => ModelDataWriter.Write((ModelDataSet)null);

        //Act
        var assertion = act.Should();

        //Assert
        assertion.Throw<ArgumentNullException>();
    }

    [Fact]
    public void Write_refuses_a_null_stream()
    {
        //Arrange
        Action act = () => ModelDataWriter.Write(ModelDataFixtures.Triangle(), (Stream)null);

        //Act
        var assertion = act.Should();

        //Assert
        assertion.Throw<ArgumentNullException>();
    }

    [Fact]
    public void Write_refuses_a_set_without_models()
        => Refusal(set => set.Models.Clear()).Should().Be("The model set has no models; add at least one ModelData to ModelDataSet.Models. (Parameter 'set')");

    [Fact]
    public void Write_refuses_a_null_model()
        => Refusal(set => set.Models.Add(null)).Should().StartWith("Model 1 of the set is null; remove it");

    [Fact]
    public void Write_refuses_a_primitive_material_out_of_range_with_the_spec_message()
        => Refusal(set => Quad(set).Material = 5).Should().StartWith("Model 'tree' mesh 0 primitive 0 names material 5, but the model has 2 materials; set Material to -1 for the default material or to an index below 2.");

    [Fact]
    public void Write_refuses_a_primitive_material_below_minus_one()
        => Refusal(set => Quad(set).Material = -2).Should().Contain("names material -2");

    [Fact]
    public void Write_refuses_a_uint16_model_with_an_index_of_65536()
        => Refusal(set =>
        {
            Model(set).IndexFormat = ModelIndexFormat.Uint16;
            Model(set).Indices[8] = 65536;
        }).Should().StartWith("Model 'tree' has IndexFormat Uint16, but index 8 is 65536; set IndexFormat to ModelIndexFormat.Uint32");

    [Fact]
    public void Write_refuses_an_index_count_that_is_not_a_multiple_of_3()
        => Refusal(set => Quad(set).IndexCount = 5).Should().StartWith("Model 'tree' mesh 0 primitive 0 has 5 indices, which is not a multiple of 3; give it whole triangles");

    [Fact]
    public void Write_refuses_a_primitive_index_range_outside_the_indices()
        => Refusal(set => Skinned(set).FirstIndex = 7).Should().StartWith("Model 'tree' mesh 1 primitive 0 uses indices 7 to 9, but the model has 9 indices; keep FirstIndex + IndexCount within Indices.");

    [Fact]
    public void Write_refuses_a_primitive_vertex_range_outside_the_vertices()
        => Refusal(set => Skinned(set).BaseVertex = 5).Should().StartWith("Model 'tree' mesh 1 primitive 0 uses 3 vertices from vertex 5, but the model has 7 vertices; keep BaseVertex");

    [Fact]
    public void Write_refuses_an_index_at_or_past_the_primitive_vertex_count()
        => Refusal(set => Model(set).Indices[2] = 4).Should().StartWith("Model 'tree' mesh 0 primitive 0 index 2 is 4, but the primitive has 4 vertices; every index (relative to BaseVertex) must be below VertexCount.");

    [Fact]
    public void Write_refuses_a_skinned_primitive_without_enough_skin_vertices()
        => Refusal(set => Model(set).SkinVertices = null).Should().StartWith("Model 'tree' mesh 1 primitive 0 has SkinBaseVertex 0 for 3 vertices, but the model has 0 skin vertices; set SkinBaseVertex to -1");

    [Fact]
    public void Write_refuses_a_morph_target_delta_list_of_the_wrong_length()
        => Refusal(set => Skinned(set).MorphTargets[0].NormalDeltas = new Vector3[2]).Should().StartWith("Model 'tree' mesh 1 primitive 0 morph target 0 has 2 entries in NormalDeltas, but the primitive has 3 vertices; give one delta per vertex, or none.");

    [Fact]
    public void Write_refuses_a_morph_target_count_that_differs_from_the_mesh_weights()
        => Refusal(set => set.Models[0].Meshes[1].DefaultMorphWeights = Array.Empty<float>()).Should().StartWith("Model 'tree' mesh 1 primitive 0 has 1 morph target, but its mesh has 0 default morph weights; give the mesh one DefaultMorphWeights entry");

    [Fact]
    public void Write_refuses_a_texture_whose_pixels_are_not_width_x_height_x_4()
        => Refusal(set => set.Textures[0].Pixels = new byte[15]).Should().StartWith("Texture 0 ('checker') has 15 pixel bytes, but 2 x 2 RGBA8 pixels take 16; set Pixels to width x height x 4 bytes");

    [Fact]
    public void Write_refuses_a_texture_without_pixels()
        => Refusal(set => set.Textures[0].Pixels = null).Should().StartWith("Texture 0 ('checker') has 0 pixel bytes, but 2 x 2 RGBA8 pixels take 16");

    [Theory]
    [InlineData(0, 2)]
    [InlineData(2, 0)]
    [InlineData(8193, 1)]
    [InlineData(1, 8193)]
    public void Write_refuses_a_texture_dimension_of_0_or_over_8192(int width, int height)
        => Refusal(set =>
        {
            set.Textures[0].Width = width;
            set.Textures[0].Height = height;
        }).Should().StartWith("Texture 0 ('checker') is " + width + " x " + height + " pixels; give it a width and a height from 1 to 8192");

    [Fact]
    public void Write_accepts_a_texture_of_8192_x_1()
    {
        //Arrange
        var set = ModelDataFixtures.EverySection();
        set.Textures[0].Width = 8192;
        set.Textures[0].Height = 1;
        set.Textures[0].Pixels = new byte[8192 * 4];

        //Act
        var bytes = ModelDataWriter.Write(set);

        //Assert
        new ModelDataTestReader(bytes).Read().Textures[0].Width.Should().Be(8192);
    }

    [Fact]
    public void Write_refuses_a_model_texture_outside_the_set()
        => Refusal(set => Model(set).Textures[0] = 1).Should().StartWith("Model 'tree' texture 0 names set texture 1, but the set has 1 texture; add the texture to ModelDataSet.Textures or name one below 1.");

    [Fact]
    public void Write_refuses_a_slot_texture_outside_the_model_textures()
        => Refusal(set => Model(set).Materials[0].BaseColorTexture.Texture = 1).Should().StartWith("Model 'tree' material 0 BaseColorTexture names texture 1 in Texture, but the model has 1 texture; set Texture to -1 for no texture or to an index below 1.");

    [Fact]
    public void Write_refuses_a_slot_sampler_outside_the_model_samplers()
        => Refusal(set => Model(set).Materials[0].BaseColorTexture.Sampler = 3).Should().StartWith("Model 'tree' material 0 BaseColorTexture names sampler 3 in Sampler, but the model has 1 sampler;");

    [Fact]
    public void Write_refuses_a_slot_with_a_sampler_but_no_texture()
        => Refusal(set => Model(set).Materials[1].EmissiveTexture.Sampler = 0).Should().StartWith("Model 'tree' material 1 EmissiveTexture names sampler 0 but no texture; set Sampler to -1 when Texture is -1.");

    [Fact]
    public void Write_refuses_a_slot_texcoord_other_than_0_or_1()
        => Refusal(set => Model(set).Materials[0].BaseColorTexture.TexCoord = 2).Should().StartWith("Model 'tree' material 0 BaseColorTexture has TexCoord 2; set it to 0 (TEXCOORD_0) or 1 (TEXCOORD_1).");

    [Fact]
    public void Write_refuses_a_material_whose_index_is_not_its_position()
        => Refusal(set => Model(set).Materials[1].Index = 0).Should().StartWith("Model 'tree' material 1 has Index 0; set Index to 1 (its position in Materials).");

    [Fact]
    public void Write_refuses_a_default_material_whose_index_is_not_minus_one()
        => Refusal(set => Model(set).DefaultMaterial.Index = 0).Should().StartWith("Model 'tree' DefaultMaterial has Index 0; set it to -1");

    [Fact]
    public void Write_refuses_a_missing_default_material()
        => Refusal(set => Model(set).DefaultMaterial = null).Should().StartWith("Model 'tree' has no DefaultMaterial; set it to new ModelMaterialData");

    [Fact]
    public void Write_refuses_a_node_parent_out_of_range()
        => Refusal(set => Model(set).Nodes[1].Parent = 5).Should().StartWith("Model 'tree' node 1 names node 5 in Parent, but the model has 5 nodes; set Parent to -1 for a root or to an index below 5.");

    [Fact]
    public void Write_refuses_a_node_child_out_of_range()
        => Refusal(set => Model(set).Nodes[0].Children.Add(9)).Should().StartWith("Model 'tree' node 0 child 3 names node 9, but the model has 5 nodes; name an index from 0 to 4.");

    [Fact]
    public void Write_refuses_a_node_mesh_out_of_range()
        => Refusal(set => Model(set).Nodes[1].Mesh = 2).Should().StartWith("Model 'tree' node 1 names mesh 2 in Mesh, but the model has 2 meshes; set Mesh to -1 for no mesh");

    [Fact]
    public void Write_refuses_a_node_skin_out_of_range()
        => Refusal(set => Model(set).Nodes[2].Skin = 1).Should().StartWith("Model 'tree' node 2 names skin 1 in Skin, but the model has 1 skin; set Skin to -1 for no skin");

    [Fact]
    public void Write_refuses_a_root_node_out_of_range()
        => Refusal(set => Model(set).RootNodes.Add(-1)).Should().StartWith("Model 'tree' root node 1 names node -1, but the model has 5 nodes;");

    [Fact]
    public void Write_refuses_a_skin_joint_out_of_range()
        => Refusal(set => Model(set).Skins[0].Joints[1] = 5).Should().StartWith("Model 'tree' skin 0 joint 1 names node 5, but the model has 5 nodes;");

    [Fact]
    public void Write_refuses_inverse_bind_matrices_that_are_neither_empty_nor_one_per_joint()
        => Refusal(set => Model(set).Skins[0].InverseBindMatrices.RemoveAt(1)).Should().StartWith("Model 'tree' skin 0 has 1 inverse bind matrix for 2 joints; give one matrix per joint, or none for identities.");

    [Fact]
    public void Write_refuses_a_skin_skeleton_out_of_range()
        => Refusal(set => Model(set).Skins[0].Skeleton = 7).Should().StartWith("Model 'tree' skin 0 names node 7 in Skeleton");

    [Fact]
    public void Write_refuses_a_channel_node_out_of_range()
        => Refusal(set => Model(set).Animations[0].Channels[0].Node = 5).Should().StartWith("Model 'tree' animation 0 channel 0 names node 5, but the model has 5 nodes;");

    [Fact]
    public void Write_refuses_channel_values_that_are_not_keys_x_components()
        => Refusal(set => Model(set).Animations[0].Channels[1].Values = new float[7]).Should().StartWith("Model 'tree' animation 0 channel 1 has 7 values for 2 keys; give 4 floats per key (ComponentCount), 8 in all.");

    [Fact]
    public void Write_refuses_cubic_spline_values_that_are_not_three_per_key_component()
        => Refusal(set => Model(set).Animations[0].Channels[2].Values = new float[6]).Should().StartWith("Model 'tree' animation 0 channel 2 has 6 values for 2 keys; give 9 floats per key (ComponentCount x 3 for CubicSpline: in-tangent, value, out-tangent), 18 in all.");

    [Fact]
    public void Write_refuses_a_component_count_that_does_not_fit_the_path()
        => Refusal(set => Model(set).Animations[0].Channels[0].ComponentCount = 4).Should().StartWith("Model 'tree' animation 0 channel 0 (Translation) has ComponentCount 4; set it to 3.");

    [Fact]
    public void Write_refuses_a_weights_channel_without_components()
        => Refusal(set => Model(set).Animations[0].Channels[3].ComponentCount = 0).Should().StartWith("Model 'tree' animation 0 channel 3 (Weights) has ComponentCount 0; set it to at least 1 (the morph-target count).");

    [Fact]
    public void Write_refuses_an_undefined_enum_value()
        => Refusal(set => Model(set).Samplers[0].MagFilter = (SamplerFilter)7).Should().StartWith("Model 'tree' sampler 0 MagFilter is 7, which is not a SamplerFilter value; set it to one of Nearest, Linear.");

    [Fact]
    public void Write_names_an_unnamed_model_by_its_position()
        => Refusal(set =>
        {
            set.Models[0].Name = null;
            Quad(set).IndexCount = 4;
        }).Should().StartWith("Model 0 mesh 0 primitive 0 has 4 indices");

    [Fact]
    public void Write_refuses_a_blob_over_4_GiB_minus_1()
    {
        //Arrange
        Action act = () => ModelDataWriter.CheckLength(4L * 1024 * 1024 * 1024);

        //Act
        var message = act.Should().Throw<InvalidOperationException>().Which.Message;

        //Assert
        message.Should().Be("The model set takes 4294967296 bytes, but a model data blob holds at most 4294967295 (4 GiB - 1); split the models into several sets or use smaller textures.");
    }

    [Fact]
    public void Write_accepts_a_blob_of_exactly_4_GiB_minus_1()
    {
        //Arrange
        Action act = () => ModelDataWriter.CheckLength(uint.MaxValue);

        //Act
        var assertion = act.Should();

        //Assert
        assertion.NotThrow();
    }

    [Fact]
    public void Write_writes_nothing_to_the_stream_when_a_check_fails()
    {
        //Arrange
        var set = ModelDataFixtures.EverySection();
        Quad(set).IndexCount = 4;
        using var stream = new MemoryStream();

        //Act
        Action act = () => ModelDataWriter.Write(set, stream);

        //Assert
        act.Should().Throw<ArgumentException>();
        stream.Length.Should().Be(0);
    }
}
