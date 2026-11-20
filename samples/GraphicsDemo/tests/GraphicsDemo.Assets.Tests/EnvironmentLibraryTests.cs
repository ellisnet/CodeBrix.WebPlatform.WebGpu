using System;
using System.Text;
using GraphicsDemo.Assets;
using SilverAssertions;
using Xunit;

namespace GraphicsDemo.Assets.Tests;

/// <summary>The embedded HDR panorama and how it is handed out.</summary>
public class EnvironmentLibraryTests
{
    private static readonly EnvironmentLibrary Library = new EnvironmentLibrary();

    [Fact]
    public void SmallCathedral_is_the_embedded_radiance_file_with_its_resolution_line()
    {
        //Arrange
        EnvironmentFile file = Library.SmallCathedral;

        //Act
        string header = Encoding.ASCII.GetString(file.Bytes, 0, 200);

        //Assert
        file.Name.Should().Be(EnvironmentLibrary.SmallCathedralName);
        file.Length.Should().Be(1665438);
        header.Should().StartWith("#?RADIANCE");
        header.Should().Contain("FORMAT=32-bit_rle_rgbe");
        header.Should().Contain("-Y 512 +X 1024");
    }

    [Fact]
    public void SmallCathedral_is_read_once_and_carries_a_strong_etag_of_its_bytes()
    {
        //Arrange
        EnvironmentFile first = Library.SmallCathedral;

        //Act
        EnvironmentFile again = Library.SmallCathedral;

        //Assert
        ReferenceEquals(first, again).Should().BeTrue();
        first.ETag.Should().StartWith("\"").And.EndWith("\"");
        first.ETag.Length.Should().Be(66);
        new EnvironmentFile("x", Array.Empty<byte>()).ETag.Should().NotBe(first.ETag);
    }
}
