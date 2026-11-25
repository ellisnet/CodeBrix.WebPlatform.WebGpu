using System.IO;
using System.Linq;
using System.Text;
using SilverAssertions;
using Xunit;

namespace CodeBrix.WebPlatform.WebGpu.Graphics.Tests.GltfReference;

public class ModelSetConverterTests
{
    private static readonly string[] SurvivalKit = { "survival-kit/barrel.glb", "survival-kit/chest.glb", "survival-kit/rock-a.glb" };

    [Fact]
    public void Convert_several_kit_models_share_one_colormap_texture()
    {
        //Arrange
        var sources = SurvivalKit.Select(path => GltfReferenceFolder.Source(GltfReferenceFolder.DefaultPacksFolder, path)).ToList();
        var separately = sources.Sum(source => ModelSetConverter.Convert(new[] { source }, GltfReferenceFolder.DefaultOptions()).TextureCount);

        //Act
        var blob = ModelSetConverter.Convert(sources, GltfReferenceFolder.DefaultOptions());

        //Assert
        separately.Should().Be(3);
        blob.TextureCount.Should().Be(1);
        blob.TextureCount.Should().BeLessThan(separately);
        blob.Models.Select(entry => entry.Name).Should().Equal("barrel", "chest", "rock-a");
        blob.Models.Select(entry => entry.Index).Should().Equal(0, 1, 2);
        blob.Models.Should().OnlyContain(entry => entry.VertexCount > 0 && entry.TriangleCount > 0);
        blob.Statistics.ModelCount.Should().Be(3);
        blob.Statistics.TextureCount.Should().Be(1);
        blob.Statistics.BlobBytes.Should().Be(blob.Length);
        blob.Bytes.Take(4).Should().Equal("CBMD"u8.ToArray());
        blob.Find("chest").Should().BeSameAs(blob.Models[1]);
    }

    [Fact]
    public void Convert_null_options_are_the_defaults()
    {
        //Arrange
        var source = GltfReferenceFolder.Source(GltfReferenceFolder.DefaultPacksFolder, "survival-kit/barrel.glb");

        //Act
        var blob = ModelSetConverter.Convert(new[] { source }, null);

        //Assert
        blob.Bytes.Should().Equal(ModelSetConverter.Convert(new[] { source }, GltfReferenceFolder.DefaultOptions()).Bytes);
    }

    [Fact]
    public void ExternalUris_of_a_glb_is_empty()
    {
        //Arrange
        var glb = File.ReadAllBytes(Path.Combine(GltfReferenceFolder.DefaultPacksFolder, "survival-kit", "barrel.glb"));

        //Act
        var uris = ModelSetConverter.ExternalUris(glb);

        //Assert
        uris.Should().BeEmpty();
    }

    [Fact]
    public void ExternalUris_of_a_gltf_lists_its_buffer_and_image_files_but_no_data_uris()
    {
        //Arrange
        var gltf = Encoding.UTF8.GetBytes("{\"buffers\":[{\"uri\":\"model.bin\"},{\"uri\":\"data:application/octet-stream;base64,AAAA\"}],\"images\":[{\"uri\":\"Textures/bark%20dark.png\"},{\"bufferView\":0}]}");

        //Act
        var uris = ModelSetConverter.ExternalUris(gltf);

        //Assert
        uris.Should().Equal("model.bin", "Textures/bark%20dark.png");
    }
}
