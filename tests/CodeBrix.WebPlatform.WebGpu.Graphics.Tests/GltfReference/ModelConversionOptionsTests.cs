using CodeBrix.WebPlatform.WebGpu.Graphics.Models;
using SilverAssertions;
using Xunit;

namespace CodeBrix.WebPlatform.WebGpu.Graphics.Tests.GltfReference;

public class ModelConversionOptionsTests
{
    [Fact]
    public void Describe_names_the_default_options() =>
        new ModelConversionOptions().Describe().Should().Be("textures=True;mipmaps=True;srgb=True;tangents=True;max=0");

    [Fact]
    public void Describe_names_changed_options()
    {
        //Arrange
        var options = new ModelConversionOptions { LoadTextures = false, Mipmaps = false, ColorTexturesSrgb = false, GenerateTangents = false, MaxTextureSize = 256, DecodeTexturesInParallel = false };

        //Act
        var text = options.Describe();

        //Assert
        text.Should().Be("textures=False;mipmaps=False;srgb=False;tangents=False;max=256");
    }

    [Theory]
    [InlineData(0, 512)]
    [InlineData(512, 512)]
    [InlineData(4096, 512)]
    [InlineData(511, 256)]
    [InlineData(300, 256)]
    [InlineData(64, 64)]
    [InlineData(63, 32)]
    public void MaxTextureSize_halves_the_texture_until_both_sides_fit(int maxTextureSize, int expectedSide)
    {
        //Arrange
        var source = GltfReferenceFolder.Source(GltfReferenceFolder.DefaultPacksFolder, "survival-kit/barrel.glb");
        var set = new ModelDataSet();
        var options = GltfReferenceFolder.DefaultOptions();
        options.MaxTextureSize = maxTextureSize;

        //Act
        GltfModelConverter.Convert(set, source.Gltf, source.Name, source.Resolve, source.BaseKey, new ModelTextureCache(set), options);

        //Assert
        set.Textures.Should().ContainSingle();
        set.Textures[0].Width.Should().Be(expectedSide);
        set.Textures[0].Height.Should().Be(expectedSide);
        set.Textures[0].Pixels.Length.Should().Be(expectedSide * expectedSide * 4);
    }

    [Fact]
    public void Defaults_are_those_of_the_C_sharp_library()
    {
        //Arrange
        var options = new ModelConversionOptions();

        //Act
        var flags = new[] { options.LoadTextures, options.Mipmaps, options.ColorTexturesSrgb, options.GenerateTangents, options.DecodeTexturesInParallel };

        //Assert
        flags.Should().OnlyContain(flag => flag);
        options.MaxTextureSize.Should().Be(0);
    }
}
