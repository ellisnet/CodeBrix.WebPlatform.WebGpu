using System;
using System.IO;
using CodeBrix.WebPlatform.WebGpu.Graphics.Models;
using SilverAssertions;
using Xunit;

namespace CodeBrix.WebPlatform.WebGpu.Graphics.Tests.Models;

/// <summary>
/// The committed fixture blobs of the crate (src/CodeBrix.WebPlatform.WebGpu.Graphics/Rust/codebrix-webgpu-graphics/fixtures)
/// are exactly what <see cref="ModelDataWriter" /> writes for <see cref="ModelDataFixtures" />. With the environment
/// variable CODEBRIX_REGENERATE_FIXTURES set to 1 the test writes the files instead of comparing them (after a
/// deliberate change to the format, the writer or the fixtures; the Rust reader tests must then be updated too).
/// </summary>
public class ModelDataFixturesTests
{
    private static bool Regenerate => Environment.GetEnvironmentVariable("CODEBRIX_REGENERATE_FIXTURES") == "1";

    [Fact]
    public void the_committed_fixture_blobs_are_what_the_writer_writes()
    {
        //Arrange
        var folder = RepositorySource.FixturesFolder();
        Assert.SkipWhen(folder == null, "The repository is not beside the test assembly's source.");

        foreach (var (fileName, set) in ModelDataFixtures.All)
        {
            //Act
            var bytes = ModelDataWriter.Write(set);
            var path = Path.Combine(folder, fileName);
            if (Regenerate)
            {
                Directory.CreateDirectory(folder);
                File.WriteAllBytes(path, bytes);
            }

            //Assert
            File.Exists(path).Should().BeTrue("the fixture " + fileName + " is committed; run this test with CODEBRIX_REGENERATE_FIXTURES=1 to write it");
            File.ReadAllBytes(path).Should().Equal(bytes, "the fixture " + fileName + " must be exactly what the writer writes; regenerate it with CODEBRIX_REGENERATE_FIXTURES=1 after a deliberate change");
        }
    }

    [Fact]
    public void the_triangle_fixture_is_1792_bytes()
        => ModelDataWriter.Write(ModelDataFixtures.Triangle()).Length.Should().Be(1792);
}
