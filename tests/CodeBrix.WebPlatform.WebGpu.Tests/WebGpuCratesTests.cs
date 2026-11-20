using System;
using System.IO;
using System.IO.Compression;
using System.Linq;
using System.Text.RegularExpressions;
using CodeBrix.WebPlatform.Core.Crates;
using SilverAssertions;
using Xunit;

namespace CodeBrix.WebPlatform.WebGpu.Tests;

public class WebGpuCratesTests : IDisposable
{
    private readonly string _folder = Path.Combine(Path.GetTempPath(), "webgpu-crates-tests-" + Guid.NewGuid().ToString("N"));

    public void Dispose()
    {
        if (Directory.Exists(_folder))
        {
            Directory.Delete(_folder, recursive: true);
        }
    }

    [Fact]
    public void Register_registers_the_webgpu_bundle_of_this_package()
    {
        //Act
        WebGpuCrates.Register();

        //Assert
        var bundle = CrateBundles.Registered.Single(b => b.Name == "webgpu");
        bundle.PackageName.Should().Be("CodeBrix.WebPlatform.WebGpu.ApacheLicenseForever");
        bundle.PackageVersion.Should().Be(WebGpuCrates.PackageVersion);
    }

    [Fact]
    public void Register_changes_nothing_when_it_is_called_again()
    {
        //Act
        WebGpuCrates.Register();
        WebGpuCrates.Register();

        //Assert
        CrateBundles.Registered.Count(b => b.Name == WebGpuCrates.BundleName).Should().Be(1);
    }

    [Fact]
    public void PackageVersion_is_the_date_stamp_version_the_assembly_was_built_with()
    {
        //Arrange
        var assemblyVersion = typeof(WebGpuCrates).Assembly.GetName().Version;

        //Assert
        WebGpuCrates.PackageVersion.Should().MatchRegex(@"^1\.\d+\.\d+(\.\d+)?$");
        WebGpuCrates.PackageVersion.Should().Be(WebGpuCrates.NormalizeVersion(assemblyVersion.ToString()));
    }

    [Theory]
    [InlineData("1.0.323.1253", "1.0.323.1253")]
    [InlineData("1.0.323.1253+3059a19c93d6b62024ad7e999bbf1292ebff9902", "1.0.323.1253")]
    [InlineData("1.0.323.0", "1.0.323")]
    [InlineData("1.0.323.0+3059a19c", "1.0.323")]
    [InlineData("1.1.5.10", "1.1.5.10")]
    public void NormalizeVersion_writes_the_version_as_nuget_writes_a_package_version(string version, string expected)
        => WebGpuCrates.NormalizeVersion(version).Should().Be(expected);

    [Fact]
    public void GetArchivePath_is_in_the_webgpu_crates_folder_of_the_given_base_folder()
        => WebGpuCrates.GetArchivePath(_folder).Should().Be(Path.Combine(_folder, "webgpu-crates", "webgpu-crates.zip"));

    [Fact]
    public void GetNoticesPath_is_in_the_webgpu_crates_folder_of_the_given_base_folder()
        => WebGpuCrates.GetNoticesPath(_folder).Should().Be(Path.Combine(_folder, "webgpu-crates", "THIRD-PARTY-NOTICES-WebGpuCrates.txt"));

    [Fact]
    public void GetDirectory_uses_the_program_base_folder_when_none_is_given()
        => WebGpuCrates.GetDirectory().Should().Be(Path.Combine(AppContext.BaseDirectory, "webgpu-crates"));

    [Fact]
    public void the_bundle_says_what_to_do_when_the_archive_is_missing()
    {
        //Arrange
        var bundle = WebGpuCrates.CreateBundle(_folder);

        //Act
        Action act = () => bundle.OpenArchive();

        //Assert
        act.Should().Throw<FileNotFoundException>()
            .WithMessage("*webgpu-crates.zip' does not exist*CodeBrix.WebPlatform.WebGpu.ApacheLicenseForever*reference the package from the build program*restore and build the build program again*");
    }

    [Fact]
    public void the_bundle_opens_the_archive_in_the_webgpu_crates_folder()
    {
        //Arrange
        Directory.CreateDirectory(WebGpuCrates.GetDirectory(_folder));
        File.WriteAllBytes(WebGpuCrates.GetArchivePath(_folder), new byte[] { 1, 2, 3 });
        var bundle = WebGpuCrates.CreateBundle(_folder);

        //Act
        using var stream = bundle.OpenArchive();

        //Assert
        stream.Length.Should().Be(3);
    }

    [Fact]
    public void ReadNotices_says_what_to_do_when_the_notices_are_missing()
    {
        //Act
        Action act = () => WebGpuCrates.ReadNotices(_folder);

        //Assert
        act.Should().Throw<FileNotFoundException>()
            .WithMessage("*THIRD-PARTY-NOTICES-WebGpuCrates.txt' does not exist*reference the package from the build program*");
    }

    [Fact]
    public void ReadNotices_reads_the_notices_file()
    {
        //Arrange
        Directory.CreateDirectory(WebGpuCrates.GetDirectory(_folder));
        File.WriteAllText(WebGpuCrates.GetNoticesPath(_folder), "notices\n");

        //Act
        var text = WebGpuCrates.ReadNotices(_folder);

        //Assert
        text.Should().Be("notices\n");
    }

    [Fact]
    public void the_archive_is_copied_beside_the_program()
        => File.Exists(WebGpuCrates.GetArchivePath()).Should().BeTrue();

    [Fact]
    public void the_archive_holds_only_name_version_crate_folders_at_the_top()
    {
        //Arrange
        using var archive = ZipFile.OpenRead(WebGpuCrates.GetArchivePath());
        var entries = archive.Entries.Select(e => e.FullName.Replace('\\', '/')).ToList();
        Assert.SkipWhen(!entries.Any(e => e.StartsWith("wgpu-", StringComparison.Ordinal)), "The archive holds no vendored crates: src/CodeBrix.WebPlatform.WebGpu/Rust/vendor is empty or missing.");

        //Act
        var topFiles = entries.Where(e => !e.Contains('/')).ToList();
        var folders = entries.Where(e => e.Contains('/')).Select(e => e.Substring(0, e.IndexOf('/'))).Distinct().ToList();

        //Assert
        topFiles.Should().BeEmpty();
        folders.Should().OnlyContain(f => Regex.IsMatch(f, @"^[A-Za-z0-9_-]+-\d+\.\d+\.\d+([-+][0-9A-Za-z.-]+)?$"));
        folders.Should().OnlyContain(f => entries.Contains(f + "/Cargo.toml") && entries.Contains(f + "/.cargo-checksum.json"));
    }

    [Fact]
    public void the_archive_carries_the_wgpu_crate()
    {
        //Arrange
        using var archive = ZipFile.OpenRead(WebGpuCrates.GetArchivePath());
        Assert.SkipWhen(!archive.Entries.Any(e => e.FullName.StartsWith("wgpu-", StringComparison.Ordinal)), "The archive holds no vendored crates: src/CodeBrix.WebPlatform.WebGpu/Rust/vendor is empty or missing.");

        //Act
        var folders = archive.Entries.Select(e => e.FullName.Replace('\\', '/')).Where(e => e.Contains('/')).Select(e => e.Substring(0, e.IndexOf('/'))).Distinct().ToList();

        //Assert
        folders.Should().Contain(f => Regex.IsMatch(f, @"^wgpu-\d+\.\d+\.\d+$"));
        folders.Should().Contain(f => Regex.IsMatch(f, @"^wgpu-types-\d+\.\d+\.\d+$"));
    }
}
