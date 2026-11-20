using System;
using System.IO;
using System.Linq;
using CodeBrix.WebPlatform.Core.Crates;
using SilverAssertions;
using Xunit;

namespace CodeBrix.WebPlatform.WebGpu.Animation.Tests;

public class AnimationCratesTests : IDisposable
{
    private readonly string _folder = Path.Combine(Path.GetTempPath(), "webgpu-animation-crates-tests-" + Guid.NewGuid().ToString("N"));

    public void Dispose()
    {
        if (Directory.Exists(_folder))
        {
            Directory.Delete(_folder, recursive: true);
        }
    }

    [Fact]
    public void Register_registers_the_webgpu_animation_bundle_of_this_package()
    {
        //Act
        AnimationCrates.Register();

        //Assert
        var bundle = CrateBundles.Registered.Single(b => b.Name == "webgpu-animation");
        bundle.PackageName.Should().Be("CodeBrix.WebPlatform.WebGpu.Animation.ApacheLicenseForever");
        bundle.PackageVersion.Should().Be(AnimationCrates.PackageVersion);
    }

    [Fact]
    public void Register_registers_the_webgpu_bundle_too()
    {
        //Act
        AnimationCrates.Register();

        //Assert
        var bundle = CrateBundles.Registered.Single(b => b.Name == WebGpuCrates.BundleName);
        bundle.PackageName.Should().Be(WebGpuCrates.PackageId);
    }

    [Fact]
    public void Register_registers_the_webgpu_bundle_before_its_own()
    {
        //Act
        AnimationCrates.Register();

        //Assert
        var names = CrateBundles.Registered.Select(b => b.Name).ToList();
        names.IndexOf(WebGpuCrates.BundleName).Should().BeGreaterThanOrEqualTo(0);
        names.IndexOf(WebGpuCrates.BundleName).Should().BeLessThan(names.IndexOf(AnimationCrates.BundleName));
    }

    [Fact]
    public void Register_changes_nothing_when_it_is_called_again()
    {
        //Arrange
        AnimationCrates.Register();
        var before = CrateBundles.Registered;

        //Act
        AnimationCrates.Register();
        WebGpuCrates.Register();
        AnimationCrates.Register();

        //Assert
        CrateBundles.Registered.Count(b => b.Name == AnimationCrates.BundleName).Should().Be(1);
        CrateBundles.Registered.Count(b => b.Name == WebGpuCrates.BundleName).Should().Be(1);
        CrateBundles.Registered.Single(b => b.Name == AnimationCrates.BundleName).Should().BeSameAs(before.Single(b => b.Name == AnimationCrates.BundleName));
        CrateBundles.Registered.Single(b => b.Name == WebGpuCrates.BundleName).Should().BeSameAs(before.Single(b => b.Name == WebGpuCrates.BundleName));
    }

    [Fact]
    public void PackageVersion_is_the_date_stamp_version_the_assembly_was_built_with()
    {
        //Arrange
        var assemblyVersion = typeof(AnimationCrates).Assembly.GetName().Version;

        //Assert
        AnimationCrates.PackageVersion.Should().MatchRegex(@"^1\.\d+\.\d+(\.\d+)?$");
        AnimationCrates.PackageVersion.Should().Be(AnimationCrates.NormalizeVersion(assemblyVersion.ToString()));
    }

    [Theory]
    [InlineData("1.0.323.1253", "1.0.323.1253")]
    [InlineData("1.0.323.1253+3059a19c93d6b62024ad7e999bbf1292ebff9902", "1.0.323.1253")]
    [InlineData("1.0.323.0", "1.0.323")]
    [InlineData("1.1.5.10", "1.1.5.10")]
    public void NormalizeVersion_writes_the_version_as_nuget_writes_a_package_version(string version, string expected)
        => AnimationCrates.NormalizeVersion(version).Should().Be(expected);

    [Fact]
    public void GetArchivePath_is_in_the_webgpu_animation_crates_folder_of_the_given_base_folder()
        => AnimationCrates.GetArchivePath(_folder).Should().Be(Path.Combine(_folder, "webgpu-animation-crates", "webgpu-animation-crates.zip"));

    [Fact]
    public void GetNoticesPath_is_in_the_webgpu_animation_crates_folder_of_the_given_base_folder()
        => AnimationCrates.GetNoticesPath(_folder).Should().Be(Path.Combine(_folder, "webgpu-animation-crates", "THIRD-PARTY-NOTICES-WebGpuAnimationCrates.txt"));

    [Fact]
    public void GetDirectory_uses_the_program_base_folder_when_none_is_given()
        => AnimationCrates.GetDirectory().Should().Be(Path.Combine(AppContext.BaseDirectory, "webgpu-animation-crates"));

    [Fact]
    public void the_bundle_says_what_to_do_when_the_archive_is_missing()
    {
        //Arrange
        var bundle = AnimationCrates.CreateBundle(_folder);

        //Act
        Action act = () => bundle.OpenArchive();

        //Assert
        act.Should().Throw<FileNotFoundException>()
            .WithMessage("*webgpu-animation-crates.zip' does not exist*CodeBrix.WebPlatform.WebGpu.Animation.ApacheLicenseForever*reference the package from the build program*restore and build the build program again*");
    }

    [Fact]
    public void the_bundle_opens_the_archive_in_the_webgpu_animation_crates_folder()
    {
        //Arrange
        Directory.CreateDirectory(AnimationCrates.GetDirectory(_folder));
        File.WriteAllBytes(AnimationCrates.GetArchivePath(_folder), new byte[] { 1, 2, 3 });
        var bundle = AnimationCrates.CreateBundle(_folder);

        //Act
        using var stream = bundle.OpenArchive();

        //Assert
        stream.Length.Should().Be(3);
    }

    [Fact]
    public void ReadNotices_says_what_to_do_when_the_notices_are_missing()
    {
        //Act
        Action act = () => AnimationCrates.ReadNotices(_folder);

        //Assert
        act.Should().Throw<FileNotFoundException>()
            .WithMessage("*THIRD-PARTY-NOTICES-WebGpuAnimationCrates.txt' does not exist*reference the package from the build program*");
    }

    [Fact]
    public void ReadNotices_reads_the_notices_file_beside_the_program()
    {
        //Act
        var text = AnimationCrates.ReadNotices();

        //Assert
        text.Should().Contain("codebrix-webgpu-animation");
        text.Should().Contain("vello_shaders 0.12.0").And.Contain("SkRasterPipeline_opts.h").And.Contain("BSD-3-Clause");
        text.Should().Contain("kurbo 0.13.1").And.Contain("peniko 0.6.1");
    }

    [Fact]
    public void the_archive_and_the_notices_are_copied_beside_the_program()
    {
        //Assert
        File.Exists(AnimationCrates.GetArchivePath()).Should().BeTrue();
        File.Exists(AnimationCrates.GetNoticesPath()).Should().BeTrue();
    }
}
