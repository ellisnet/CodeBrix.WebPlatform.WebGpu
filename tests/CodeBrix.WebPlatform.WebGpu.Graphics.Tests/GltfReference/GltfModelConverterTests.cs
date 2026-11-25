using System;
using System.IO;
using System.Text;
using CodeBrix.WebPlatform.WebGpu.Graphics.Models;
using SilverAssertions;
using Xunit;

namespace CodeBrix.WebPlatform.WebGpu.Graphics.Tests.GltfReference;

public class GltfModelConverterTests
{
    [Theory]
    [InlineData("survival-kit", "Textures/colormap.png", "survival-kit/Textures/colormap.png")]
    [InlineData("forest/oak", "../Textures/bark.jpg", "forest/Textures/bark.jpg")]
    [InlineData("forest/oak/", "./Textures/./bark.jpg", "forest/oak/Textures/bark.jpg")]
    [InlineData("forest\\oak", "..\\Textures\\bark.jpg", "forest/Textures/bark.jpg")]
    [InlineData("pack", "Textures/bark%20dark%2Bwet.jpg", "pack/Textures/bark dark+wet.jpg")]
    [InlineData("pack", "/Textures//bark.jpg", "pack/Textures/bark.jpg")]
    [InlineData("pack", "../../bark.jpg", "../bark.jpg")]
    public void CombineKey_resolves_dots_backslashes_and_percent_escapes(string baseKey, string relativeUri, string expected) =>
        GltfModelConverter.CombineKey(baseKey, relativeUri).Should().Be(expected);

    [Theory]
    [InlineData("survival-kit/barrel.glb")]
    [InlineData("cube-pets/animal-fox.glb")]
    public void Convert_sample_model_holds_geometry_materials_and_its_texture(string modelPath)
    {
        //Arrange
        var source = GltfReferenceFolder.Source(GltfReferenceFolder.DefaultPacksFolder, modelPath);
        var set = new ModelDataSet();

        //Act
        var model = GltfModelConverter.Convert(set, source.Gltf, source.Name, source.Resolve, source.BaseKey, new ModelTextureCache(set), GltfReferenceFolder.DefaultOptions());

        //Assert
        set.Models.Should().ContainSingle().Which.Should().BeSameAs(model);
        model.Name.Should().Be(Path.GetFileNameWithoutExtension(modelPath));
        model.Vertices.Length.Should().BeGreaterThan(0);
        model.Indices.Length.Should().BeGreaterThan(0);
        model.Meshes.Should().NotBeEmpty();
        model.Nodes.Should().NotBeEmpty();
        model.RootNodes.Should().NotBeEmpty();
        model.Materials.Should().NotBeEmpty();
        set.Textures.Should().ContainSingle();
        set.Textures[0].Name.Should().Be(source.BaseKey + "/Textures/colormap.png");
        set.Textures[0].Pixels.Length.Should().Be(set.Textures[0].Width * set.Textures[0].Height * 4);
        model.Textures.Should().Equal(0);
    }

    [Fact]
    public void Convert_without_textures_leaves_every_texture_out()
    {
        //Arrange
        var source = GltfReferenceFolder.Source(GltfReferenceFolder.DefaultPacksFolder, "survival-kit/barrel.glb");
        var set = new ModelDataSet();
        var options = GltfReferenceFolder.DefaultOptions();
        options.LoadTextures = false;

        //Act
        var model = GltfModelConverter.Convert(set, source.Gltf, source.Name, source.Resolve, source.BaseKey, new ModelTextureCache(set), options);

        //Assert
        set.Textures.Should().BeEmpty();
        model.Textures.Should().BeEmpty();
        model.Vertices.Length.Should().BeGreaterThan(0);
    }

    [Fact]
    public void Convert_bad_bytes_throws_InvalidDataException_with_a_message()
    {
        //Arrange
        var set = new ModelDataSet();
        var bytes = Encoding.UTF8.GetBytes("this is not a glTF model");

        //Act
        Action act = () => GltfModelConverter.Convert(set, bytes, "broken", null, null, null);

        //Assert
        act.Should().Throw<InvalidDataException>().WithMessage("The glTF model 'broken' could not be read: *");
        set.Models.Should().BeEmpty();
    }

    [Fact]
    public void Convert_gltf_whose_buffer_the_resolver_does_not_supply_throws_InvalidDataException()
    {
        //Arrange
        var set = new ModelDataSet();
        var bytes = Encoding.UTF8.GetBytes("{\"asset\":{\"version\":\"2.0\"},\"buffers\":[{\"uri\":\"missing.bin\",\"byteLength\":12}]}");

        //Act
        Action act = () => GltfModelConverter.Convert(set, bytes, "lonely", uri => null, "pack", null);

        //Assert
        act.Should().Throw<InvalidDataException>().WithMessage("The glTF model 'lonely' could not be read: *");
    }

    [Fact]
    public void Convert_with_the_cache_of_another_set_throws_ArgumentException()
    {
        //Arrange
        var source = GltfReferenceFolder.Source(GltfReferenceFolder.DefaultPacksFolder, "survival-kit/barrel.glb");
        var set = new ModelDataSet();
        var otherCache = new ModelTextureCache(new ModelDataSet());

        //Act
        Action act = () => GltfModelConverter.Convert(set, source.Gltf, source.Name, source.Resolve, source.BaseKey, otherCache);

        //Assert
        act.Should().Throw<ArgumentException>().WithMessage("The texture cache belongs to another model data set*");
        set.Models.Should().BeEmpty();
    }
}
