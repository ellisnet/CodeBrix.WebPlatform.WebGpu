using System;
using System.IO;
using System.IO.Compression;
using System.Linq;
using CodeBrix.WebPlatform.WebGpu.LibraryChecker.Platform;
using CodeBrix.WebPlatform.WebGpu.LibraryChecker.Tests.Helpers;
using SilverAssertions;
using Xunit;

namespace CodeBrix.WebPlatform.WebGpu.LibraryChecker.Tests.Platform;

public class CrateSnapshotTests
{
    [Fact]
    public void FromDirectory_and_FromArchive_read_the_same_crates()
    {
        //Arrange
        using var folder = new TempFolder();
        folder.WriteCrate("vendor", "bytemuck", "1.25.2");
        folder.WriteCrate("vendor", "windows-sys", "0.52.0");
        folder.WriteCrate("vendor", "windows-sys", "0.61.2");
        var archive = Path.Combine(folder.Path, "crates.zip");
        ZipFile.CreateFromDirectory(folder.Full("vendor"), archive);

        //Act
        var onDisk = CrateSnapshot.FromDirectory(folder.Full("vendor"));
        var zipped = CrateSnapshot.FromArchive(archive);

        //Assert
        onDisk.Crates.Select(c => c.ToString()).Should().Equal("bytemuck 1.25.2", "windows-sys 0.52.0", "windows-sys 0.61.2");
        zipped.Crates.Select(c => c.ToString()).Should().Equal(onDisk.Crates.Select(c => c.ToString()));
        zipped.Find("windows-sys").Should().HaveCount(2);
        zipped.Crates.Zip(onDisk.Crates).All(p => p.First.HasSameContentAs(p.Second)).Should().BeTrue();
    }

    [Fact]
    public void a_folder_that_is_not_a_vendored_crate_is_rejected()
    {
        //Arrange
        using var folder = new TempFolder();
        folder.Write("vendor/loose-1.0.0/Cargo.toml", "[package]\nname = \"loose\"\nversion = \"1.0.0\"\n");

        //Act
        Action act = () => CrateSnapshot.FromDirectory(folder.Full("vendor"));

        //Assert
        act.Should().Throw<InvalidDataException>().WithMessage("*.cargo-checksum.json*");
    }

    [Fact]
    public void ReadPackage_takes_the_name_and_version_of_the_package_table_only()
    {
        //Act
        var (name, version) = CrateSnapshot.ReadPackage("[package]\nedition = \"2021\"\nname = \"wgpu\"\nversion = \"30.0.2\"\n\n[dependencies.bitflags]\nversion = \"2.6\"\n");
        var (noName, noVersion) = CrateSnapshot.ReadPackage("[workspace]\nmembers = []\n");

        //Assert
        name.Should().Be("wgpu");
        version.Should().Be("30.0.2");
        noName.Should().BeNull();
        noVersion.Should().BeNull();
    }

    [Fact]
    public void the_platform_s_archive_is_beside_the_tool_and_its_lock_file_names_every_crate_in_it()
    {
        //Act
        var crates = PlatformRelease.LoadCrates();
        var lockText = PlatformRelease.LockFileText;

        //Assert
        PlatformRelease.Version.Should().NotBeNullOrEmpty();
        crates.Crates.Should().NotBeEmpty();
        foreach (var crate in crates.Crates)
        {
            lockText.Should().Contain($"name = \"{crate.Name}\"\nversion = \"{crate.Version}\"");
        }
    }
}
