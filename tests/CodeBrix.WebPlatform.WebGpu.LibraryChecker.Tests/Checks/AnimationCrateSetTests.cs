using System.Linq;
using CodeBrix.RustTools.Projects;
using CodeBrix.WebPlatform.WebGpu.LibraryChecker.Checks;
using CodeBrix.WebPlatform.WebGpu.LibraryChecker.Platform;
using CodeBrix.WebPlatform.WebGpu.LibraryChecker.Tests.Helpers;
using SilverAssertions;
using Xunit;

namespace CodeBrix.WebPlatform.WebGpu.LibraryChecker.Tests.Checks;

public class AnimationCrateSetTests
{
    private const string PlatformVersion = "1.0.300.100";

    private static CargoLockFile Lock(params (string Name, string Version, string Checksum)[] packages)
    {
        var text = "version = 4\n\n[[package]]\nname = \"animation-crates\"\nversion = \"1.0.0\"\n";
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
        folder.WriteCrate("webgpu", "arrayvec", "0.7.8");
        folder.WriteCrate("platform", "euclid", "0.22.14");
        var (_, webGpu, platform) = Snapshots(folder);
        var lockFile = Lock(("arrayvec", "0.7.8", TempFolder.Sha256("arrayvec0.7.8")), ("euclid", "0.22.14", TempFolder.Sha256("euclid0.22.14")), ("kurbo", "0.13.1", TempFolder.Sha256("kurbo0.13.1")));

        //Act
        var set = AnimationCrateSet.FromLock(lockFile, webGpu, platform);

        //Assert
        set.Own.Select(p => p.Name).Should().Equal("kurbo");
        set.Shared.Select(p => p.Name).Should().Equal("arrayvec", "euclid");
        set.Vendored.Packages.Select(p => p.Name).Should().Equal("kurbo");
    }

    [Fact]
    public void StrictRule_passes_when_the_shared_crates_match_and_the_own_ones_are_carried_by_neither()
    {
        //Arrange
        using var folder = new TempFolder();
        folder.WriteCrate("webgpu", "arrayvec", "0.7.8");
        folder.WriteCrate("own", "kurbo", "0.13.1");
        var (own, webGpu, platform) = Snapshots(folder);
        var set = AnimationCrateSet.FromLock(Lock(("arrayvec", "0.7.8", TempFolder.Sha256("arrayvec0.7.8")), ("kurbo", "0.13.1", TempFolder.Sha256("kurbo0.13.1"))), webGpu, platform);

        //Act
        var result = AnimationCrateSet.StrictRule(set, own, webGpu, platform, PlatformVersion);

        //Assert
        result.Passed.Should().BeTrue(result.ToText());
        result.Name.Should().Contain(PlatformVersion);
    }

    [Fact]
    public void StrictRule_fails_for_a_shared_crate_whose_lock_checksum_differs()
    {
        //Arrange
        using var folder = new TempFolder();
        folder.WriteCrate("platform", "smallvec", "1.16.5");
        var (own, webGpu, platform) = Snapshots(folder);
        var set = AnimationCrateSet.FromLock(Lock(("smallvec", "1.16.5", "0000")), webGpu, platform);

        //Act
        var result = AnimationCrateSet.StrictRule(set, own, webGpu, platform, PlatformVersion);

        //Assert
        result.Passed.Should().BeFalse();
        result.Problems.Single().Should().StartWith("smallvec 1.16.5: the Animation crate set's Cargo.lock records checksum 0000").And.Contain("re-resolve the crate set");
    }

    [Fact]
    public void StrictRule_fails_for_an_own_crate_the_platform_carries_in_another_version()
    {
        //Arrange
        using var folder = new TempFolder();
        folder.WriteCrate("platform", "smallvec", "1.15.0");
        folder.WriteCrate("own", "smallvec", "1.16.5");
        var (own, webGpu, platform) = Snapshots(folder);
        var set = AnimationCrateSet.FromLock(Lock(("smallvec", "1.16.5", TempFolder.Sha256("smallvec1.16.5"))), webGpu, platform);

        //Act
        var result = AnimationCrateSet.StrictRule(set, own, webGpu, platform, PlatformVersion);

        //Assert
        result.Passed.Should().BeFalse();
        result.Problems.Should().Contain(p => p.StartsWith("smallvec: the Animation crate set needs version 1.16.5, but the WebGpu package or the platform carries 1.15.0", System.StringComparison.Ordinal));
        result.Problems.Should().Contain(p => p.StartsWith("smallvec-1.16.5: the Animation package vendors a crate the WebGpu package or the platform carries", System.StringComparison.Ordinal));
    }
}
