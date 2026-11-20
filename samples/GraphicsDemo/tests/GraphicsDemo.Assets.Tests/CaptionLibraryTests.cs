using System.Text;
using GraphicsDemo.Assets;
using SilverAssertions;
using Xunit;

namespace GraphicsDemo.Assets.Tests;

/// <summary>The embedded caption file and how it is handed out.</summary>
public class CaptionLibraryTests
{
    private static readonly CaptionLibrary Library = new CaptionLibrary();

    [Fact]
    public void CaptionScene_is_the_embedded_srt_text_with_its_four_cues()
    {
        //Arrange
        byte[] bytes = Library.CaptionScene;

        //Act
        string text = Encoding.UTF8.GetString(bytes);

        //Assert
        text.Should().StartWith("1\n00:00:00,400 --> 00:00:02,600\n");
        text.Should().Contain("\n4\n00:00:06,500 --> 00:00:07,900\n");
        text.Split("-->").Length.Should().Be(5);
    }

    [Fact]
    public void The_caption_file_is_read_once_and_carries_a_strong_etag_of_its_bytes()
    {
        //Arrange
        byte[] first = Library.CaptionScene;

        //Act
        byte[] again = Library.CaptionScene;

        //Assert
        ReferenceEquals(first, again).Should().BeTrue();
        Library.CaptionSceneETag.Should().StartWith("\"").And.EndWith("\"");
        Library.CaptionSceneETag.Length.Should().Be(66);
        CaptionLibrary.SubRipContentType.Should().Contain("utf-8");
    }
}
