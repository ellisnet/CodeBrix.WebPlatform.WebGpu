using System;
using System.Buffers.Binary;
using GraphicsDemo.Assets;
using SilverAssertions;
using Xunit;

namespace GraphicsDemo.Assets.Tests;

/// <summary>The pictures decoded on the backend and their pixel form.</summary>
public class PictureLibraryTests
{
    private static readonly PictureLibrary Library = new PictureLibrary(new ModelLibrary());

    [Fact]
    public void CubePetsColormap_is_decoded_into_the_pixel_form()
    {
        //Arrange
        PicturePixels picture = Library.CubePetsColormap;

        //Act
        uint width = BinaryPrimitives.ReadUInt32LittleEndian(picture.Bytes.AsSpan(8, 4));
        uint height = BinaryPrimitives.ReadUInt32LittleEndian(picture.Bytes.AsSpan(12, 4));

        //Assert
        picture.Bytes.AsSpan(0, 4).ToArray().Should().Equal(new byte[] { (byte)'C', (byte)'B', (byte)'P', (byte)'X' });
        picture.Name.Should().Be(PictureLibrary.CubePetsColormapName);
        width.Should().Be(512u);
        height.Should().Be(512u);
        picture.Bytes.Length.Should().Be(PicturePixels.HeaderLength + (512 * 512 * 4));
        ReferenceEquals(picture, Library.CubePetsColormap).Should().BeTrue();
        picture.ETag.Length.Should().Be(66);
    }

    [Fact]
    public void LottieBadge_is_made_in_code_opaque_inside_and_transparent_at_the_corners()
    {
        //Arrange
        PicturePixels badge = Library.LottieBadge;
        int size = PictureLibrary.LottieBadgeSize;

        //Act
        uint width = BinaryPrimitives.ReadUInt32LittleEndian(badge.Bytes.AsSpan(8, 4));
        byte cornerAlpha = badge.Bytes[PicturePixels.HeaderLength + 3];
        byte centreAlpha = badge.Bytes[PicturePixels.HeaderLength + ((((size / 2) * size) + (size / 2)) * 4) + 3];

        //Assert
        badge.Name.Should().Be(PictureLibrary.LottieBadgeName);
        width.Should().Be((uint)size);
        badge.Bytes.Length.Should().Be(PicturePixels.HeaderLength + (size * size * 4));
        cornerAlpha.Should().Be((byte)0);
        centreAlpha.Should().Be((byte)255);
        ReferenceEquals(badge, Library.LottieBadge).Should().BeTrue();
    }

    [Fact]
    public void PicturePixels_refuses_pixels_that_do_not_match_the_size()
    {
        //Arrange
        byte[] rgba = new byte[15];

        //Act
        Action make = () => new PicturePixels("p", rgba, 2, 2);

        //Assert
        make.Should().Throw<ArgumentException>();
    }

    [Fact]
    public void CubePetsColormapPng_is_the_png_file_itself()
    {
        //Arrange
        PngFile png = Library.CubePetsColormapPng;

        //Act
        byte[] signature = png.Bytes.AsSpan(0, 8).ToArray();

        //Assert
        signature.Should().Equal(new byte[] { 0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A });
        png.ETag.Length.Should().Be(66);
        ReferenceEquals(png, Library.CubePetsColormapPng).Should().BeTrue();
    }
}
