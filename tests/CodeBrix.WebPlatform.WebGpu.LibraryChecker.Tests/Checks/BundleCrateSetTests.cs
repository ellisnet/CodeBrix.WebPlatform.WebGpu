using System;
using System.IO;
using System.Linq;
using CodeBrix.RustTools.Projects;
using CodeBrix.WebPlatform.WebGpu.LibraryChecker.Checks;
using CodeBrix.WebPlatform.WebGpu.LibraryChecker.Platform;
using CodeBrix.WebPlatform.WebGpu.LibraryChecker.Tests.Helpers;
using SilverAssertions;
using Xunit;

namespace CodeBrix.WebPlatform.WebGpu.LibraryChecker.Tests.Checks;

public class BundleCrateSetTests
{
    private static CargoLockFile Lock(params (string Name, string Version, string Checksum)[] packages)
    {
        var text = "version = 4\n\n[[package]]\nname = \"crate-set\"\nversion = \"1.0.0\"\n";
        foreach (var (name, version, checksum) in packages)
        {
            text += $"\n[[package]]\nname = \"{name}\"\nversion = \"{version}\"\nsource = \"registry+https://github.com/rust-lang/crates.io-index\"\nchecksum = \"{checksum}\"\n";
        }

        return CargoLockFile.Parse(text, "Cargo.lock");
    }

    private static (BundleCrateSet Graphics, BundleCrateSet Animation) Sets(CargoLockFile graphics, CargoLockFile animation)
    {
        using var folder = new TempFolder();
        Directory.CreateDirectory(folder.Full("empty"));
        var nothing = CrateSnapshot.FromDirectory(folder.Full("empty"));
        return (GraphicsCrateSet.FromLock(graphics, nothing, nothing), AnimationCrateSet.FromLock(animation, nothing, nothing));
    }

    [Fact]
    public void BothBundles_passes_when_no_crate_name_is_vendored_by_both_packages()
    {
        //Arrange
        var (graphics, animation) = Sets(Lock(("gltf", "1.4.1", TempFolder.Sha256("gltf1.4.1"))), Lock(("kurbo", "0.13.1", TempFolder.Sha256("kurbo0.13.1"))));

        //Act
        var result = BundleCrateSet.BothBundles(graphics, animation);

        //Assert
        result.Passed.Should().BeTrue(result.ToText());
        result.Name.Should().Contain("both the Graphics and the Animation package").And.Contain("an application may register both bundles");
        result.Notes.Single().Should().StartWith("no crate name is vendored by both (1 Graphics crates, 1 Animation crates)");
    }

    [Fact]
    public void BothBundles_fails_for_a_crate_both_packages_vendor_in_different_versions()
    {
        //Arrange
        var (graphics, animation) = Sets(Lock(("lazy_static", "1.5.1", TempFolder.Sha256("lazy_static1.5.1"))), Lock(("lazy_static", "1.4.0", TempFolder.Sha256("lazy_static1.4.0"))));

        //Act
        var result = BundleCrateSet.BothBundles(graphics, animation);

        //Assert
        result.Passed.Should().BeFalse();
        result.Problems.Single().Should().StartWith("lazy_static: the Graphics package vendors 1.5.1, the Animation package 1.4.0; an application that registers both bundles stops on it")
            .And.Contain("vendoring/gltf/UPGRADE-README.txt").And.Contain("vendoring/kurbo-peniko/UPGRADE-README.txt");
    }

    [Fact]
    public void BothBundles_fails_for_a_crate_both_packages_vendor_at_one_version_with_different_checksums()
    {
        //Arrange
        var (graphics, animation) = Sets(Lock(("lazy_static", "1.5.1", "aaaa")), Lock(("lazy_static", "1.5.1", "bbbb")));

        //Act
        var result = BundleCrateSet.BothBundles(graphics, animation);

        //Assert
        result.Passed.Should().BeFalse();
        result.Problems.Single().Should().StartWith("lazy_static 1.5.1: the Graphics and the Animation crate sets' locks record different checksums (aaaa / bbbb)");
    }

    [Fact]
    public void BothBundles_passes_with_a_note_for_a_crate_both_packages_vendor_at_one_version_and_content()
    {
        //Arrange
        var checksum = TempFolder.Sha256("lazy_static1.5.1");
        var (graphics, animation) = Sets(Lock(("lazy_static", "1.5.1", checksum)), Lock(("lazy_static", "1.5.1", checksum)));

        //Act
        var result = BundleCrateSet.BothBundles(graphics, animation);

        //Assert
        result.Passed.Should().BeTrue(result.ToText());
        result.Notes.Single().Should().Contain("vendored by both packages at the same version and content").And.Contain("lazy_static 1.5.1");
    }

    [Fact]
    public void the_packages_name_their_folders_and_notes()
    {
        //Arrange
        var layout = new RepositoryLayout(Path.Combine(Path.GetTempPath(), "repo"));

        //Assert
        BundlePackage.Graphics.CrateSet(layout).Should().Be(layout.GraphicsCrateSet);
        BundlePackage.Graphics.CrateSetLock(layout).Should().Be(layout.GraphicsCrateSetLock);
        BundlePackage.Graphics.Vendor(layout).Should().Be(layout.GraphicsVendor);
        BundlePackage.Graphics.NoticesHeaderFile(layout).Should().Be(layout.GraphicsNoticesHeaderFile);
        BundlePackage.Graphics.NoticesFile(layout).Should().Be(layout.GraphicsNoticesFile);
        BundlePackage.Graphics.UpgradeNotesShown.Should().Be("src/CodeBrix.WebPlatform.WebGpu.Graphics/Rust/vendoring/gltf/UPGRADE-README.txt");
        BundlePackage.Animation.CrateSet(layout).Should().Be(layout.AnimationCrateSet);
        BundlePackage.Animation.CrateSetLock(layout).Should().Be(layout.AnimationCrateSetLock);
        BundlePackage.Animation.Vendor(layout).Should().Be(layout.AnimationVendor);
        BundlePackage.Animation.NoticesHeaderFile(layout).Should().Be(layout.AnimationNoticesHeaderFile);
        BundlePackage.Animation.NoticesFile(layout).Should().Be(layout.AnimationNoticesFile);
        BundlePackage.Animation.UpgradeNotesShown.Should().Be("src/CodeBrix.WebPlatform.WebGpu.Animation/Rust/vendoring/kurbo-peniko/UPGRADE-README.txt");
    }

    [Fact]
    public void CreateMergedSource_puts_the_graphics_crate_set_beside_its_own_and_the_webgpu_packages_crates()
    {
        //Arrange
        using var folder = new TempFolder();
        var layout = new RepositoryLayout(folder.Path);
        folder.Write("src/CodeBrix.WebPlatform.WebGpu.Graphics/Rust/graphics-crates/Cargo.toml", "[package]\nname = \"crate-set\"\nversion = \"1.0.0\"\n");
        folder.Write("src/CodeBrix.WebPlatform.WebGpu.Graphics/Rust/graphics-crates/Cargo.lock", "version = 4\n");
        folder.Write("src/CodeBrix.WebPlatform.WebGpu.Graphics/Rust/graphics-crates/src/lib.rs", string.Empty);
        folder.WriteCrate(layout.GraphicsVendorRelative, "gltf", "1.4.1");
        folder.WriteCrate(layout.VendorRelative, "serde", "1.0.229");
        var webGpu = CrateSnapshot.FromDirectory(layout.Vendor);
        var set = GraphicsCrateSet.FromLock(Lock(("gltf", "1.4.1", TempFolder.Sha256("gltf1.4.1")), ("serde", "1.0.229", TempFolder.Sha256("serde1.0.229"))), webGpu, webGpu);

        //Act
        var merged = BundleCrateSet.CreateMergedSource(layout, set, Path.Combine(folder.Path, "no-platform-archive.zip"));

        //Assert
        try
        {
            Path.GetFileName(merged.CrateSet).Should().Be("graphics-crates");
            Path.GetFileName(merged.Folder).Should().StartWith("webgpu-graphics-crate-set-");
            Directory.GetDirectories(Path.Combine(merged.Folder, "vendor")).Select(Path.GetFileName).OrderBy(n => n, StringComparer.Ordinal).Should().Equal("gltf-1.4.1", "serde-1.0.229");
            File.ReadAllText(Path.Combine(merged.CrateSet, ".cargo", "config.toml")).Should().Contain("directory = \"../vendor\"");
        }
        finally
        {
            Directory.Delete(merged.Folder, recursive: true);
        }
    }
}
