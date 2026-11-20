using System;
using System.Text;
using GraphicsDemo.Assets;
using SilverAssertions;
using Xunit;

namespace GraphicsDemo.Assets.Tests;

/// <summary>The embedded Lottie files and how they are handed out.</summary>
public class LottieLibraryTests
{
    private static readonly LottieLibrary Library = new LottieLibrary();

    [Theory]
    [InlineData(LottieLibrary.ManyShapes, 640)]
    [InlineData(LottieLibrary.Painterly, 800)]
    [InlineData(LottieLibrary.BusyScene, 960)]
    [InlineData(LottieLibrary.Features, 400)]
    [InlineData(LottieLibrary.ImageLayer, 400)]
    public void GetFile_is_the_embedded_lottie_json_of_that_width(string name, int width)
    {
        //Arrange
        byte[] bytes = Library.GetFile(name);

        //Act
        string text = Encoding.UTF8.GetString(bytes);

        //Assert
        text.Should().StartWith("{");
        text.Should().Contain("\"w\":" + width + ",");
        text.Should().Contain("\"layers\":");
    }

    [Fact]
    public void The_image_layer_file_names_the_badge_asset_at_the_badge_s_size()
    {
        //Arrange
        byte[] bytes = Library.GetFile(LottieLibrary.ImageLayer);

        //Act
        string text = Encoding.UTF8.GetString(bytes);

        //Assert
        text.Should().Contain("\"id\":\"badge\",\"w\":" + PictureLibrary.LottieBadgeSize + ",\"h\":" + PictureLibrary.LottieBadgeSize + ",");
        text.Should().Contain("\"ty\":2,");
        text.Should().Contain("\"refId\":\"badge\"");
    }

    [Fact]
    public void A_file_is_read_once_and_carries_a_strong_etag_of_its_bytes()
    {
        //Arrange
        byte[] first = Library.GetFile(LottieLibrary.Features);

        //Act
        byte[] again = Library.GetFile(LottieLibrary.Features);

        //Assert
        ReferenceEquals(first, again).Should().BeTrue();
        Library.GetETag(LottieLibrary.Features).Should().StartWith("\"").And.EndWith("\"");
        Library.GetETag(LottieLibrary.Features).Length.Should().Be(66);
        LottieLibrary.LottieContentType.Should().Contain("utf-8");
    }

    [Fact]
    public void GetFile_refuses_a_name_that_is_not_one_of_the_samples_and_says_what_to_use()
    {
        //Arrange
        Action read = () => Library.GetFile("../secret.json");

        //Act
        var thrown = read.Should().Throw<ArgumentException>();

        //Assert
        thrown.Which.Message.Should().Contain("use one of the names LottieLibrary defines");
    }
}
