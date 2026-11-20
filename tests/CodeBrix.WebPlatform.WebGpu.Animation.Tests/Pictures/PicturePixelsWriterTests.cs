using System;
using System.IO;
using CodeBrix.WebPlatform.WebGpu.Animation.Pictures;
using SilverAssertions;
using Xunit;

namespace CodeBrix.WebPlatform.WebGpu.Animation.Tests.Pictures;

public class PicturePixelsWriterTests
{
    private static readonly byte[] TwoPixels = [1, 2, 3, 4, 5, 6, 7, 8];

    [Fact]
    public void Write_puts_the_magic_the_versions_and_the_size_before_the_pixels()
    {
        //Arrange
        byte[] expected = [(byte)'C', (byte)'B', (byte)'P', (byte)'X', 1, 0, 0, 0, 2, 0, 0, 0, 1, 0, 0, 0, 1, 2, 3, 4, 5, 6, 7, 8];

        //Act
        byte[] form = PicturePixelsWriter.Write(TwoPixels, 2, 1);

        //Assert
        form.Should().Equal(expected);
        form.Length.Should().Be((int)PicturePixelsWriter.GetLength(2, 1));
    }

    [Fact]
    public void Write_to_a_stream_gives_the_same_bytes_as_the_array()
    {
        //Arrange
        using var stream = new MemoryStream();

        //Act
        PicturePixelsWriter.Write(TwoPixels, 1, 2, stream);

        //Assert
        stream.ToArray().Should().Equal(PicturePixelsWriter.Write(TwoPixels, 1, 2));
    }

    [Fact]
    public void Write_writes_large_sizes_little_endian()
    {
        //Arrange
        byte[] rgba = new byte[300 * 2 * 4];

        //Act
        byte[] form = PicturePixelsWriter.Write(rgba, 300, 2);

        //Assert
        form[8].Should().Be(0x2C);
        form[9].Should().Be(0x01);
        form.Length.Should().Be(PicturePixelsFormat.HeaderLength + rgba.Length);
    }

    [Fact]
    public void Write_refuses_pixels_that_do_not_match_the_size()
    {
        //Act
        Action act = () => PicturePixelsWriter.Write(TwoPixels, 2, 2);

        //Assert
        act.Should().Throw<ArgumentException>().WithMessage("*width x height x 4 bytes*");
    }

    [Fact]
    public void Write_refuses_an_empty_or_oversized_picture()
    {
        //Act
        Action empty = () => PicturePixelsWriter.Write(ReadOnlySpan<byte>.Empty, 0, 1);
        Action wide = () => PicturePixelsWriter.GetLength(PicturePixelsFormat.MaxDimension + 1, 1);

        //Assert
        empty.Should().Throw<ArgumentException>().WithMessage("*between 1 x 1*");
        wide.Should().Throw<ArgumentException>();
    }

    [Fact]
    public void Write_to_a_null_stream_is_refused()
    {
        //Act
        Action act = () => PicturePixelsWriter.Write(TwoPixels, 2, 1, null);

        //Assert
        act.Should().Throw<ArgumentNullException>();
    }

    [Fact]
    public void The_format_constants_are_the_ones_the_crate_reads()
    {
        //Assert
        PicturePixelsFormat.Magic.ToArray().Should().Equal(new byte[] { 0x43, 0x42, 0x50, 0x58 });
        PicturePixelsFormat.MajorVersion.Should().Be(1);
        PicturePixelsFormat.HeaderLength.Should().Be(16);
    }
}
