using System;
using GraphicsDemo.Assets;
using SilverAssertions;
using Xunit;

namespace GraphicsDemo.Assets.Tests;

/// <summary>The embedded fonts and how they are handed out.</summary>
public class FontLibraryTests
{
    private static readonly FontLibrary Library = new FontLibrary();

    [Fact]
    public void RobotoRegular_and_OpenSansRegular_are_the_embedded_truetype_files()
    {
        //Arrange
        FontFile roboto = Library.RobotoRegular;

        //Act
        FontFile openSans = Library.OpenSansRegular;

        //Assert
        roboto.Name.Should().Be(FontLibrary.RobotoRegularName);
        roboto.Length.Should().Be(159108);
        openSans.Name.Should().Be(FontLibrary.OpenSansRegularName);
        openSans.Length.Should().Be(130832);
        // A TrueType file starts with the sfnt version 0x00010000.
        BitConverter.ToString(roboto.Bytes, 0, 4).Should().Be("00-01-00-00");
        BitConverter.ToString(openSans.Bytes, 0, 4).Should().Be("00-01-00-00");
    }

    [Fact]
    public void A_font_is_read_once_and_carries_a_strong_etag_of_its_bytes()
    {
        //Arrange
        FontFile first = Library.RobotoRegular;

        //Act
        FontFile again = Library.RobotoRegular;

        //Assert
        ReferenceEquals(first, again).Should().BeTrue();
        first.ETag.Should().StartWith("\"").And.EndWith("\"");
        first.ETag.Length.Should().Be(66);
        first.ETag.Should().NotBe(Library.OpenSansRegular.ETag);
    }
}
