using System;
using System.IO;
using System.Linq;
using CodeBrix.WebPlatform.Core.Crates;
using SilverAssertions;
using Xunit;

namespace CodeBrix.WebPlatform.WebGpu.Graphics.Tests;

public class GraphicsCratesTests : IDisposable
{
    private readonly string _folder = Path.Combine(Path.GetTempPath(), "webgpu-graphics-crates-tests-" + Guid.NewGuid().ToString("N"));

    public void Dispose()
    {
        if (Directory.Exists(_folder))
        {
            Directory.Delete(_folder, recursive: true);
        }
    }

    [Fact]
    public void Register_registers_the_webgpu_graphics_bundle_of_this_package()
    {
        //Act
        GraphicsCrates.Register();

        //Assert
        var bundle = CrateBundles.Registered.Single(b => b.Name == "webgpu-graphics");
        bundle.PackageName.Should().Be("CodeBrix.WebPlatform.WebGpu.Graphics.ApacheLicenseForever");
        bundle.PackageVersion.Should().Be(GraphicsCrates.PackageVersion);
    }

    [Fact]
    public void Register_registers_the_webgpu_bundle_too()
    {
        //Act
        GraphicsCrates.Register();

        //Assert
        var bundle = CrateBundles.Registered.Single(b => b.Name == WebGpuCrates.BundleName);
        bundle.PackageName.Should().Be(WebGpuCrates.PackageId);
    }

    [Fact]
    public void Register_registers_the_webgpu_bundle_before_its_own()
    {
        //Act
        GraphicsCrates.Register();

        //Assert
        var names = CrateBundles.Registered.Select(b => b.Name).ToList();
        names.IndexOf(WebGpuCrates.BundleName).Should().BeGreaterThanOrEqualTo(0);
        names.IndexOf(WebGpuCrates.BundleName).Should().BeLessThan(names.IndexOf(GraphicsCrates.BundleName));
    }

    [Fact]
    public void Register_changes_nothing_when_it_is_called_again()
    {
        //Arrange
        GraphicsCrates.Register();
        var before = CrateBundles.Registered;

        //Act
        GraphicsCrates.Register();
        WebGpuCrates.Register();
        GraphicsCrates.Register();

        //Assert
        CrateBundles.Registered.Count(b => b.Name == GraphicsCrates.BundleName).Should().Be(1);
        CrateBundles.Registered.Count(b => b.Name == WebGpuCrates.BundleName).Should().Be(1);
        CrateBundles.Registered.Single(b => b.Name == GraphicsCrates.BundleName).Should().BeSameAs(before.Single(b => b.Name == GraphicsCrates.BundleName));
        CrateBundles.Registered.Single(b => b.Name == WebGpuCrates.BundleName).Should().BeSameAs(before.Single(b => b.Name == WebGpuCrates.BundleName));
    }

    [Fact]
    public void PackageVersion_is_the_date_stamp_version_the_assembly_was_built_with()
    {
        //Arrange
        var assemblyVersion = typeof(GraphicsCrates).Assembly.GetName().Version;

        //Assert
        GraphicsCrates.PackageVersion.Should().MatchRegex(@"^1\.\d+\.\d+(\.\d+)?$");
        GraphicsCrates.PackageVersion.Should().Be(GraphicsCrates.NormalizeVersion(assemblyVersion.ToString()));
    }

    [Theory]
    [InlineData("1.0.323.1253", "1.0.323.1253")]
    [InlineData("1.0.323.1253+3059a19c93d6b62024ad7e999bbf1292ebff9902", "1.0.323.1253")]
    [InlineData("1.0.323.0", "1.0.323")]
    [InlineData("1.1.5.10", "1.1.5.10")]
    public void NormalizeVersion_writes_the_version_as_nuget_writes_a_package_version(string version, string expected)
        => GraphicsCrates.NormalizeVersion(version).Should().Be(expected);

    [Fact]
    public void GetArchivePath_is_in_the_webgpu_graphics_crates_folder_of_the_given_base_folder()
        => GraphicsCrates.GetArchivePath(_folder).Should().Be(Path.Combine(_folder, "webgpu-graphics-crates", "webgpu-graphics-crates.zip"));

    [Fact]
    public void GetNoticesPath_is_in_the_webgpu_graphics_crates_folder_of_the_given_base_folder()
        => GraphicsCrates.GetNoticesPath(_folder).Should().Be(Path.Combine(_folder, "webgpu-graphics-crates", "THIRD-PARTY-NOTICES-WebGpuGraphicsCrates.txt"));

    [Fact]
    public void GetDirectory_uses_the_program_base_folder_when_none_is_given()
        => GraphicsCrates.GetDirectory().Should().Be(Path.Combine(AppContext.BaseDirectory, "webgpu-graphics-crates"));

    [Fact]
    public void the_bundle_says_what_to_do_when_the_archive_is_missing()
    {
        //Arrange
        var bundle = GraphicsCrates.CreateBundle(_folder);

        //Act
        Action act = () => bundle.OpenArchive();

        //Assert
        act.Should().Throw<FileNotFoundException>()
            .WithMessage("*webgpu-graphics-crates.zip' does not exist*CodeBrix.WebPlatform.WebGpu.Graphics.ApacheLicenseForever*reference the package from the build program*restore and build the build program again*");
    }

    [Fact]
    public void the_bundle_opens_the_archive_in_the_webgpu_graphics_crates_folder()
    {
        //Arrange
        Directory.CreateDirectory(GraphicsCrates.GetDirectory(_folder));
        File.WriteAllBytes(GraphicsCrates.GetArchivePath(_folder), new byte[] { 1, 2, 3 });
        var bundle = GraphicsCrates.CreateBundle(_folder);

        //Act
        using var stream = bundle.OpenArchive();

        //Assert
        stream.Length.Should().Be(3);
    }

    [Fact]
    public void ReadNotices_says_what_to_do_when_the_notices_are_missing()
    {
        //Act
        Action act = () => GraphicsCrates.ReadNotices(_folder);

        //Assert
        act.Should().Throw<FileNotFoundException>()
            .WithMessage("*THIRD-PARTY-NOTICES-WebGpuGraphicsCrates.txt' does not exist*reference the package from the build program*");
    }

    [Fact]
    public void ReadNotices_reads_the_notices_file_beside_the_program()
    {
        //Act
        var text = GraphicsCrates.ReadNotices();

        //Assert
        text.Should().Contain("codebrix-webgpu-graphics");
        text.Should().Contain("aces_fitted").And.Contain("aces_narkowicz").And.Contain("pbr_neutral");
    }

    [Fact]
    public void the_archive_and_the_notices_are_copied_beside_the_program()
    {
        //Assert
        File.Exists(GraphicsCrates.GetArchivePath()).Should().BeTrue();
        File.Exists(GraphicsCrates.GetNoticesPath()).Should().BeTrue();
    }
}
