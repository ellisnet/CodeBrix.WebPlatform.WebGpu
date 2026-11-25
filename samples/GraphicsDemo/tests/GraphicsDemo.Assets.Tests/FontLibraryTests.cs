using System;
using GraphicsDemo.Assets;
using SilverAssertions;
using Xunit;

namespace GraphicsDemo.Assets.Tests;

/// <summary>The embedded fonts, found by their file names, and how they are handed out.</summary>
public class FontLibraryTests
{
    private static readonly FontLibrary Library = new FontLibrary();

    [Fact]
    public void FontNames_lists_every_embedded_font_file_by_its_lower_case_file_name()
    {
        //Arrange
        FontLibrary library = new FontLibrary();

        //Act
        System.Collections.Generic.IReadOnlyList<string> names = library.FontNames;

        //Assert
        names.Should().Equal("opensans-regular.ttf", "roboto-regular.ttf", "roboto.ttf");
    }

    [Fact]
    public void GetFont_hands_out_the_embedded_truetype_files_by_name_in_any_case()
    {
        //Arrange
        FontFile roboto = Library.GetFont("roboto-regular.ttf");

        //Act
        FontFile openSans = Library.GetFont("OpenSans-Regular.ttf");

        //Assert
        roboto.Name.Should().Be("roboto-regular.ttf");
        roboto.Length.Should().Be(159108);
        roboto.ContentType.Should().Be(FontLibrary.TrueTypeContentType);
        openSans.Name.Should().Be("opensans-regular.ttf");
        openSans.Length.Should().Be(130832);
        // A TrueType file starts with the sfnt version 0x00010000.
        BitConverter.ToString(roboto.Bytes, 0, 4).Should().Be("00-01-00-00");
        BitConverter.ToString(openSans.Bytes, 0, 4).Should().Be("00-01-00-00");
    }

    [Fact]
    public void GetFont_answers_null_for_a_name_that_is_not_an_embedded_font()
    {
        //Arrange
        string[] names = { "no-such-font.ttf", "OFL.txt", "roboto-regular", "../roboto/Roboto-Regular.ttf", null };

        //Act
        bool any = false;
        foreach (string name in names) any |= Library.GetFont(name) != null;

        //Assert
        any.Should().BeFalse();
    }

    [Fact]
    public void A_font_is_read_once_and_carries_a_strong_etag_of_its_bytes()
    {
        //Arrange
        FontFile first = Library.GetFont("roboto-regular.ttf");

        //Act
        FontFile again = Library.GetFont("roboto-regular.ttf");

        //Assert
        ReferenceEquals(first, again).Should().BeTrue();
        first.ETag.Should().StartWith("\"").And.EndWith("\"");
        first.ETag.Length.Should().Be(66);
        first.ETag.Should().NotBe(Library.GetFont("opensans-regular.ttf").ETag);
    }
}
