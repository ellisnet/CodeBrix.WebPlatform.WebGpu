using System.IO;
using System.Linq;
using CodeBrix.RustTools.Projects;
using CodeBrix.WebPlatform.WebGpu.LibraryChecker.Checks;
using CodeBrix.WebPlatform.WebGpu.LibraryChecker.Tests.Helpers;
using SilverAssertions;
using Xunit;

namespace CodeBrix.WebPlatform.WebGpu.LibraryChecker.Tests.Checks;

public class VendorCheckTests
{
    private const string Vendor = "src/CodeBrix.WebPlatform.WebGpu/Rust/vendor";

    private static string LockEntry(string name, string version, bool registry = true)
        => $"[[package]]\nname = \"{name}\"\nversion = \"{version}\"\n"
            + (registry ? $"source = \"registry+https://github.com/rust-lang/crates.io-index\"\nchecksum = \"{TempFolder.Sha256(name + version)}\"\n" : string.Empty)
            + "\n";

    private static CargoLockFile Lock(params string[] entries) => CargoLockFile.Parse("version = 4\n\n" + string.Concat(entries));

    [Fact]
    public void Crates_passes_for_an_untouched_folder_that_holds_exactly_the_set()
    {
        //Arrange
        using var folder = new TempFolder();
        folder.WriteCrate(Vendor, "wgpu", "30.0.2");
        folder.WriteCrate(Vendor, "arrayvec", "0.7.8");
        var set = VendoredSet.FromLock(Lock(LockEntry("codebrix-webplatform-webgpu-crates", "0.0.0", registry: false), LockEntry("wgpu", "30.0.2"), LockEntry("arrayvec", "0.7.8")));

        //Act
        var result = VendorCheck.Crates(new RepositoryLayout(folder.Path), set);

        //Assert
        result.Passed.Should().BeTrue(result.ToText());
        result.Notes.Single().Should().StartWith("2 crates, 4 files hashed");
        set.Packages.Should().HaveCount(2, "the crate set itself is not vendored");
    }

    [Fact]
    public void Crates_fails_for_a_missing_an_unexpected_and_an_edited_crate()
    {
        //Arrange
        using var folder = new TempFolder();
        folder.WriteCrate(Vendor, "wgpu", "30.0.2");
        folder.WriteCrate(Vendor, "extra", "1.0.0");
        var wgpu = folder.Full(Vendor + "/wgpu-30.0.2");
        File.AppendAllText(Path.Combine(wgpu, "src", "lib.rs"), "// edited");
        File.WriteAllText(Path.Combine(wgpu, "NOTES.txt"), "a note");
        var set = VendoredSet.FromLock(Lock(LockEntry("wgpu", "30.0.2"), LockEntry("arrayvec", "0.7.8")));

        //Act
        var result = VendorCheck.Crates(new RepositoryLayout(folder.Path), set);

        //Assert
        result.Problems.Should().HaveCount(4);
        result.Problems.Should().Contain(p => p.StartsWith("arrayvec v0.7.8 is in Cargo.lock but not in the vendor folder") && p.EndsWith("run `LibraryChecker revendor`"));
        result.Problems.Should().Contain(p => p.StartsWith("extra v1.0.0 (extra-1.0.0) is in the vendor folder but not in Cargo.lock"));
        result.Problems.Should().Contain(p => p.StartsWith("wgpu v30.0.2: src/lib.rs does not match its recorded checksum") && p.Contains("never edited"));
        result.Problems.Should().Contain(p => p.StartsWith("wgpu v30.0.2: NOTES.txt is not recorded"));
    }

    [Fact]
    public void GraphicsCrates_passes_for_the_graphics_packages_own_crates_and_names_its_check()
    {
        //Arrange
        using var folder = new TempFolder();
        folder.WriteCrate("src/CodeBrix.WebPlatform.WebGpu.Graphics/Rust/vendor", "gltf", "1.4.1");
        var set = VendoredSet.FromPackages("own crates", "the Graphics crate set's own", Lock(LockEntry("gltf", "1.4.1")).Packages);

        //Act
        var result = VendorCheck.GraphicsCrates(new RepositoryLayout(folder.Path), set);

        //Assert
        result.Passed.Should().BeTrue(result.ToText());
        result.Name.Should().Be("Vendored crates: the Graphics package's crates are untouched and are exactly the Graphics crate set's own crates");
    }

    [Fact]
    public void GraphicsCrates_fails_for_a_missing_and_an_edited_crate_and_points_at_the_gltf_notes()
    {
        //Arrange
        using var folder = new TempFolder();
        var gltf = folder.WriteCrate("src/CodeBrix.WebPlatform.WebGpu.Graphics/Rust/vendor", "gltf", "1.4.1");
        File.AppendAllText(Path.Combine(gltf, "src", "lib.rs"), "// edited");
        var set = VendoredSet.FromPackages("own crates", "the Graphics crate set's own", Lock(LockEntry("gltf", "1.4.1"), LockEntry("inflections", "1.1.1")).Packages);

        //Act
        var result = VendorCheck.GraphicsCrates(new RepositoryLayout(folder.Path), set);

        //Assert
        result.Problems.Should().HaveCount(2);
        result.Problems.Should().Contain(p => p.StartsWith("inflections v1.1.1 is in Cargo.lock but not in the vendor folder") && p.EndsWith("re-vendor the Graphics crate set (src/CodeBrix.WebPlatform.WebGpu.Graphics/Rust/vendoring/gltf/UPGRADE-README.txt)"));
        result.Problems.Should().Contain(p => p.StartsWith("gltf v1.4.1: src/lib.rs does not match its recorded checksum") && p.Contains("never edited"));
    }
}
