using System;
using System.Linq;
using CodeBrix.RustTools.Projects;
using CodeBrix.WebPlatform.WebGpu.LibraryChecker.Checks;
using CodeBrix.WebPlatform.WebGpu.LibraryChecker.Platform;
using CodeBrix.WebPlatform.WebGpu.LibraryChecker.Tests.Helpers;
using SilverAssertions;
using Xunit;

namespace CodeBrix.WebPlatform.WebGpu.LibraryChecker.Tests.Checks;

public class GraphicsCrateSetTests
{
    private const string PlatformVersion = "1.0.300.100";

    private static CargoLockFile Lock(params (string Name, string Version, string Checksum)[] packages)
    {
        var text = "version = 4\n\n[[package]]\nname = \"codebrix-webplatform-webgpu-graphics-crates\"\nversion = \"1.0.0\"\n";
        foreach (var (name, version, checksum) in packages)
        {
            text += $"\n[[package]]\nname = \"{name}\"\nversion = \"{version}\"\nsource = \"registry+https://github.com/rust-lang/crates.io-index\"\nchecksum = \"{checksum}\"\n";
        }

        return CargoLockFile.Parse(text, "Cargo.lock");
    }

    private static (CrateSnapshot Own, CrateSnapshot WebGpu, CrateSnapshot Platform) Snapshots(TempFolder folder)
    {
        foreach (var name in new[] { "own", "webgpu", "platform" })
        {
            System.IO.Directory.CreateDirectory(folder.Full(name));
        }

        return (CrateSnapshot.FromDirectory(folder.Full("own")), CrateSnapshot.FromDirectory(folder.Full("webgpu")), CrateSnapshot.FromDirectory(folder.Full("platform")));
    }

    [Fact]
    public void FromLock_splits_the_crates_the_others_carry_from_the_packages_own()
    {
        //Arrange
        using var folder = new TempFolder();
        folder.WriteCrate("webgpu", "serde", "1.0.229");
        folder.WriteCrate("platform", "byteorder", "1.5.0");
        var (_, webGpu, platform) = Snapshots(folder);
        var lockFile = Lock(("byteorder", "1.5.0", TempFolder.Sha256("byteorder1.5.0")), ("gltf", "1.4.1", TempFolder.Sha256("gltf1.4.1")), ("serde", "1.0.229", TempFolder.Sha256("serde1.0.229")));

        //Act
        var set = GraphicsCrateSet.FromLock(lockFile, webGpu, platform);

        //Assert
        set.Package.Should().BeSameAs(BundlePackage.Graphics);
        set.Own.Select(p => p.Name).Should().Equal("gltf");
        set.Shared.Select(p => p.Name).Should().Equal("byteorder", "serde");
        set.Vendored.Packages.Select(p => p.Name).Should().Equal("gltf");
        set.Vendored.Description.Should().StartWith("the Graphics crate set's Cargo.lock minus");
    }

    [Fact]
    public void StrictRule_passes_when_the_shared_crates_match_and_the_own_ones_are_carried_by_neither()
    {
        //Arrange
        using var folder = new TempFolder();
        folder.WriteCrate("platform", "byteorder", "1.5.0");
        folder.WriteCrate("own", "gltf", "1.4.1");
        var (own, webGpu, platform) = Snapshots(folder);
        var set = GraphicsCrateSet.FromLock(Lock(("byteorder", "1.5.0", TempFolder.Sha256("byteorder1.5.0")), ("gltf", "1.4.1", TempFolder.Sha256("gltf1.4.1"))), webGpu, platform);

        //Act
        var result = GraphicsCrateSet.StrictRule(set, own, webGpu, platform, PlatformVersion);

        //Assert
        result.Passed.Should().BeTrue(result.ToText());
        result.Name.Should().StartWith("Platform: the Graphics crate set shares every crate").And.Contain(PlatformVersion);
        result.Notes.Single().Should().StartWith("1 crates of the set are the Graphics package's own (gltf 1.4.1)");
    }

    [Fact]
    public void StrictRule_fails_for_a_shared_crate_whose_lock_checksum_differs_and_points_at_the_gltf_notes()
    {
        //Arrange
        using var folder = new TempFolder();
        folder.WriteCrate("webgpu", "serde_json", "1.0.154");
        var (own, webGpu, platform) = Snapshots(folder);
        var set = GraphicsCrateSet.FromLock(Lock(("serde_json", "1.0.154", "0000")), webGpu, platform);

        //Act
        var result = GraphicsCrateSet.StrictRule(set, own, webGpu, platform, PlatformVersion);

        //Assert
        result.Passed.Should().BeFalse();
        result.Problems.Single().Should().StartWith("serde_json 1.0.154: the Graphics crate set's Cargo.lock records checksum 0000").And.Contain("(vendoring/gltf/UPGRADE-README.txt)");
    }

    [Fact]
    public void StrictRule_fails_for_an_own_crate_the_platform_carries_in_another_version()
    {
        //Arrange
        using var folder = new TempFolder();
        folder.WriteCrate("platform", "lazy_static", "1.4.0");
        folder.WriteCrate("own", "lazy_static", "1.5.1");
        var (own, webGpu, platform) = Snapshots(folder);
        var set = GraphicsCrateSet.FromLock(Lock(("lazy_static", "1.5.1", TempFolder.Sha256("lazy_static1.5.1"))), webGpu, platform);

        //Act
        var result = GraphicsCrateSet.StrictRule(set, own, webGpu, platform, PlatformVersion);

        //Assert
        result.Passed.Should().BeFalse();
        result.Problems.Should().Contain(p => p.StartsWith("lazy_static: the Graphics crate set needs version 1.5.1, but the WebGpu package or the platform carries 1.4.0", StringComparison.Ordinal));
        result.Problems.Should().Contain(p => p.StartsWith("lazy_static-1.5.1: the Graphics package vendors a crate the WebGpu package or the platform carries", StringComparison.Ordinal) && p.EndsWith("the Graphics library's Rust/vendor", StringComparison.Ordinal));
    }

    [Fact]
    public void StrictRule_rejects_the_animation_packages_set()
    {
        //Arrange
        using var folder = new TempFolder();
        var (own, webGpu, platform) = Snapshots(folder);
        var set = AnimationCrateSet.FromLock(Lock(), webGpu, platform);

        //Act
        Action act = () => GraphicsCrateSet.StrictRule(set, own, webGpu, platform, PlatformVersion);

        //Assert
        act.Should().Throw<ArgumentException>().WithMessage("*Animation package's, not the Graphics package's*");
    }
}
