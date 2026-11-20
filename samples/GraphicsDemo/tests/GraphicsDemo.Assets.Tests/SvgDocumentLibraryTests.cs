using System;
using System.Text;
using GraphicsDemo.Assets;
using SilverAssertions;
using Xunit;

namespace GraphicsDemo.Assets.Tests;

/// <summary>The embedded SVG documents and how they are handed out.</summary>
public class SvgDocumentLibraryTests
{
    private static readonly SvgDocumentLibrary Library = new SvgDocumentLibrary();

    [Fact]
    public void Every_document_is_embedded_svg_text()
    {
        foreach (string name in SvgDocumentLibrary.Names)
        {
            //Arrange
            byte[] bytes = Library.Get(name);

            //Act
            string text = Encoding.UTF8.GetString(bytes);

            //Assert
            text.Should().StartWith("<svg xmlns=\"http://www.w3.org/2000/svg\"");
            text.TrimEnd().Should().EndWith("</svg>");
        }
    }

    [Fact]
    public void A_document_is_read_once_and_carries_a_strong_etag_of_its_bytes()
    {
        //Arrange
        byte[] first = Library.Get(SvgDocumentLibrary.Clock);

        //Act
        byte[] again = Library.Get(SvgDocumentLibrary.Clock);

        //Assert
        ReferenceEquals(first, again).Should().BeTrue();
        Library.GetETag(SvgDocumentLibrary.Clock).Should().StartWith("\"").And.EndWith("\"");
        Library.GetETag(SvgDocumentLibrary.Clock).Length.Should().Be(66);
        Encoding.UTF8.GetString(first).Should().Contain("PER-PART");
    }

    [Fact]
    public void An_unknown_name_is_refused_with_what_to_use()
    {
        //Act
        Action act = () => Library.Get("missing");

        //Assert
        act.Should().Throw<ArgumentException>().WithMessage("*SvgDocumentLibrary.Names*");
    }
}
