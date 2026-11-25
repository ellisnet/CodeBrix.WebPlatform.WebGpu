using System;
using System.Linq;
using System.Numerics;
using CodeBrix.WebPlatform.WebGpu.Graphics.Models;
using GraphicsDemo.Assets;
using GraphicsDemo.Assets.BuiltModels;
using SilverAssertions;
using Xunit;

namespace GraphicsDemo.Assets.Tests;

/// <summary>The morph-target shape the backend builds in code: its sphere, its three targets and its weights clip.</summary>
public class MorphShapeModelTests
{
    [Fact]
    public void Create_builds_a_unit_sphere_with_one_primitive_and_three_morph_targets()
    {
        //Arrange
        ModelData model = MorphShapeModel.Create();

        //Act
        ModelPrimitiveData primitive = model.Meshes.Single().Primitives.Single();

        //Assert
        model.Name.Should().Be("morph-shape");
        model.Vertices.Length.Should().Be((MorphShapeModel.Segments + 1) * (MorphShapeModel.Rings + 1));
        primitive.IndexCount.Should().Be((uint)(MorphShapeModel.Segments * MorphShapeModel.Rings * 6));
        primitive.VertexCount.Should().Be(model.Vertices.Length);
        model.Vertices.Should().OnlyContain(vertex => MathF.Abs(vertex.Position.Length() - 1f) < 1e-5f);
        model.Vertices.Should().OnlyContain(vertex => MathF.Abs(vertex.Normal.Length() - 1f) < 1e-4f && Vector3.Dot(vertex.Normal, vertex.Position) > 0.99f);
        primitive.MorphTargets.Count.Should().Be(3);
        primitive.MorphTargets.Should().OnlyContain(target => target.PositionDeltas.Length == model.Vertices.Length && target.NormalDeltas.Length == model.Vertices.Length);
        model.Meshes[0].DefaultMorphWeights.Should().Equal(0f, 0f, 0f);
        model.Nodes.Single().DefaultMorphWeights.Should().Equal(0f, 0f, 0f);
        model.RootNodes.Should().Equal(0);
        model.Materials.Single().Name.Should().Be("coral");
    }

    [Fact]
    public void Each_target_moves_the_surface_in_its_own_way()
    {
        //Arrange
        ModelData model = MorphShapeModel.Create();
        ModelPrimitiveData primitive = model.Meshes[0].Primitives[0];
        int top = 0;
        int equator = (MorphShapeModel.Rings / 2 * (MorphShapeModel.Segments + 1)) + 2;

        //Act
        ModelMorphTargetData stretch = primitive.MorphTargets[0];
        ModelMorphTargetData twist = primitive.MorphTargets[1];
        ModelMorphTargetData bumps = primitive.MorphTargets[2];

        //Assert - the stretch lifts the pole 0.7, the twist leaves heights alone and turns the upper rings, the bumps only push outwards.
        stretch.PositionDeltas[top].Y.Should().BeApproximately(0.7f, 1e-5f);
        stretch.PositionDeltas[equator].Length().Should().BeLessThan(1e-5f);
        twist.PositionDeltas.Should().OnlyContain(delta => MathF.Abs(delta.Y) < 1e-6f);
        twist.PositionDeltas.Max(delta => delta.Length()).Should().BeGreaterThan(0.5f);
        bumps.PositionDeltas.Select((delta, index) => Vector3.Dot(delta, model.Vertices[index].Position)).Should().OnlyContain(push => push >= -1e-6f);
        bumps.PositionDeltas.Max(delta => delta.Length()).Should().BeApproximately(0.35f, 0.01f);
        primitive.MorphTargets.Should().OnlyContain(target => target.NormalDeltas.Any(delta => delta.Length() > 0.05f));
    }

    [Fact]
    public void Morph_clip_raises_each_weight_in_turn_over_six_seconds()
    {
        //Arrange
        ModelData model = MorphShapeModel.Create();

        //Act
        ModelAnimationChannelData channel = model.Animations.Single().Channels.Single();

        //Assert
        model.Animations[0].Name.Should().Be(MorphShapeModel.ClipName);
        channel.Path.Should().Be(AnimationPath.Weights);
        channel.Interpolation.Should().Be(AnimationInterpolation.Linear);
        channel.ComponentCount.Should().Be(3);
        channel.Times.Should().Equal(0f, 1f, 2f, 3f, 4f, 5f, 6f);
        channel.Values.Length.Should().Be(channel.Times.Length * 3);
        channel.Values.Skip(3).Take(3).Should().Equal(1f, 0f, 0f);
        channel.Values.Skip(9).Take(3).Should().Equal(0f, 1f, 0f);
        channel.Values.Skip(15).Take(3).Should().Equal(0f, 0f, 1f);
        channel.Times[^1].Should().Be(MorphShapeModel.ClipSeconds);
    }

    [Fact]
    public void The_library_writes_it_as_a_blob_of_its_own()
    {
        //Arrange
        ModelLibrary library = new ModelLibrary();

        //Act
        ModelBlob blob = library.GetModelBlob(MorphShapeModel.Name);
        byte[] again = ModelDataWriter.Write(ModelLibrary.BuildModelSet(MorphShapeModel.Name));

        //Assert
        blob.Should().NotBeNull();
        blob.Models.Single().Name.Should().Be("morph-shape");
        blob.TextureCount.Should().Be(0);
        blob.Bytes.Should().Equal(again, "the shape is built the same way every time");
        (blob.Length % 16).Should().Be(0);
    }
}
