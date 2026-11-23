using System.Linq;
using CodeBrix.WebPlatform.WebGpu.LibraryChecker.Checks;
using CodeBrix.WebPlatform.WebGpu.LibraryChecker.Platform;
using CodeBrix.WebPlatform.WebGpu.LibraryChecker.Tests.Helpers;
using SilverAssertions;
using Xunit;

namespace CodeBrix.WebPlatform.WebGpu.LibraryChecker.Tests.Checks;

public class PlatformCheckTests
{
    private const string PlatformVersion = "1.0.300.100";

    private static (CrateSnapshot Ours, CrateSnapshot Platform) Snapshots(TempFolder folder)
        => (CrateSnapshot.FromDirectory(folder.Full("ours")), CrateSnapshot.FromDirectory(folder.Full("platform")));

    [Fact]
    public void StrictRule_passes_when_every_shared_crate_is_identical_and_the_rest_are_new()
    {
        //Arrange
        using var folder = new TempFolder();
        folder.WriteCrate("platform", "js-sys", "0.3.107");
        folder.WriteCrate("platform", "windows-sys", "0.52.0");
        folder.WriteCrate("platform", "windows-sys", "0.61.2");
        folder.WriteCrate("ours", "js-sys", "0.3.107");
        folder.WriteCrate("ours", "windows-sys", "0.61.2");
        folder.WriteCrate("ours", "wgpu", "30.0.2");
        var (ours, platform) = Snapshots(folder);

        //Act
        var result = PlatformCheck.StrictRule(ours, platform, PlatformVersion);

        //Assert
        result.Passed.Should().BeTrue(result.ToText());
        result.Name.Should().Contain(PlatformVersion);
        result.Notes.Single().Should().Be("3 vendored crates: 2 shared with the platform's 3 at identical content, 1 that the platform does not carry");
    }

    [Fact]
    public void StrictRule_fails_for_a_crate_at_a_version_the_platform_does_not_carry_and_names_both_versions()
    {
        //Arrange
        using var folder = new TempFolder();
        folder.WriteCrate("platform", "bitflags", "2.9.0");
        folder.WriteCrate("ours", "bitflags", "2.13.2");
        var (ours, platform) = Snapshots(folder);

        //Act
        var result = PlatformCheck.StrictRule(ours, platform, PlatformVersion, "src/X/Rust/webgpu-crates");

        //Assert
        result.Passed.Should().BeFalse();
        result.Problems[0].Should().Be("bitflags: this repository vendors version 2.13.2, but the platform's crates carry version 2.9.0");
        result.Problems.Last().Should().Contain("Re-resolve the crate set (src/X/Rust/webgpu-crates) against the platform's Cargo.lock")
            .And.Contain("Core pin").And.Contain(PlatformVersion);
    }

    [Fact]
    public void StrictRule_fails_for_the_same_version_with_different_content()
    {
        //Arrange
        using var folder = new TempFolder();
        folder.WriteCrate("platform", "log", "0.4.28");
        folder.WriteCrate("ours", "log", "0.4.28", source: "pub fn changed() {}");
        var (ours, platform) = Snapshots(folder);

        //Act
        var result = PlatformCheck.StrictRule(ours, platform, PlatformVersion);

        //Assert
        result.Problems.Should().HaveCount(2);
        result.Problems[0].Should().Be("log 0.4.28: this repository and the platform's crates both carry it, with different content (.cargo-checksum.json differs)");
    }

    [Fact]
    public void CorePin_passes_for_an_exact_pin()
    {
        //Arrange
        using var folder = new TempFolder();
        folder.Write("src/CodeBrix.WebPlatform.WebGpu/CodeBrix.WebPlatform.WebGpu.csproj",
            $"<Project><ItemGroup><PackageReference Include=\"CodeBrix.WebPlatform.Core.ApacheLicenseForever\" Version=\"[{PlatformVersion}]\" /></ItemGroup></Project>");

        //Act
        var result = PlatformCheck.CorePin(new RepositoryLayout(folder.Path), PlatformVersion);

        //Assert
        result.Passed.Should().BeTrue(result.ToText());
        result.Notes[0].Should().Be($"1 Core references in the package projects, each pinned to [{PlatformVersion}]");
    }

    [Fact]
    public void CorePin_fails_for_a_loose_or_different_pin()
    {
        //Arrange
        using var folder = new TempFolder();
        folder.Write("src/CodeBrix.WebPlatform.WebGpu/CodeBrix.WebPlatform.WebGpu.csproj",
            "<Project><ItemGroup><PackageReference Include=\"CodeBrix.WebPlatform.Core.ApacheLicenseForever\" Version=\"1.0.300.100\" /></ItemGroup></Project>");
        folder.Write("src/CodeBrix.WebPlatform.WebGpu.Graphics/CodeBrix.WebPlatform.WebGpu.Graphics.csproj",
            "<Project><ItemGroup><PackageReference Include=\"CodeBrix.WebPlatform.Core.ApacheLicenseForever\" Version=\"[1.0.299.1]\" /></ItemGroup></Project>");

        //Act
        var result = PlatformCheck.CorePin(new RepositoryLayout(folder.Path), PlatformVersion);

        //Assert
        result.Problems.Should().HaveCount(2);
        result.Problems[0].Should().StartWith("src/CodeBrix.WebPlatform.WebGpu/CodeBrix.WebPlatform.WebGpu.csproj references CodeBrix.WebPlatform.Core.ApacheLicenseForever as '1.0.300.100'");
        result.Problems[1].Should().Contain("'[1.0.299.1]'");
    }

    [Fact]
    public void CorePin_notes_when_no_package_references_core_yet()
    {
        //Arrange
        using var folder = new TempFolder();
        folder.Write("src/CodeBrix.WebPlatform.WebGpu/CodeBrix.WebPlatform.WebGpu.csproj", "<Project />");

        //Act
        var result = PlatformCheck.CorePin(new RepositoryLayout(folder.Path), PlatformVersion);

        //Assert
        result.Passed.Should().BeTrue();
        result.Notes[0].Should().StartWith("no package project references Core yet");
        result.Notes.Should().Contain("the platform's packages come from nuget.org");
    }
}
