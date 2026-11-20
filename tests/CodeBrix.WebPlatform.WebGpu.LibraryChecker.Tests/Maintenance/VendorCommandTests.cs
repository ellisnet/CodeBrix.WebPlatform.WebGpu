using System.IO;
using CodeBrix.WebPlatform.WebGpu.LibraryChecker.Maintenance;
using CodeBrix.WebPlatform.WebGpu.LibraryChecker.Platform;
using CodeBrix.WebPlatform.WebGpu.LibraryChecker.Tests.Helpers;
using SilverAssertions;
using Xunit;

namespace CodeBrix.WebPlatform.WebGpu.LibraryChecker.Tests.Maintenance;

public class VendorCommandTests
{
    [Fact]
    public void PrepareScratch_copies_the_crate_set_without_its_cargo_folder_and_seeds_the_platform_s_lock()
    {
        //Arrange
        using var folder = new TempFolder();
        folder.Write("src/CodeBrix.WebPlatform.WebGpu/Rust/webgpu-crates/Cargo.toml", "[package]\nname = \"set\"\n");
        folder.Write("src/CodeBrix.WebPlatform.WebGpu/Rust/webgpu-crates/src/lib.rs", "");
        folder.Write("src/CodeBrix.WebPlatform.WebGpu/Rust/webgpu-crates/Cargo.lock", "old lock");
        folder.Write("src/CodeBrix.WebPlatform.WebGpu/Rust/webgpu-crates/.cargo/config.toml", "[source.crates-io]");
        var command = new VendorCommand(new RepositoryLayout(folder.Path), null);
        var scratch = folder.Full("scratch");

        //Act
        command.PrepareScratch(scratch);

        //Assert
        File.ReadAllText(Path.Combine(scratch, "Cargo.toml")).Should().StartWith("[package]");
        File.Exists(Path.Combine(scratch, "src", "lib.rs")).Should().BeTrue();
        Directory.Exists(Path.Combine(scratch, ".cargo")).Should().BeFalse("cargo must read crates.io, not the vendor folder");
        File.ReadAllText(Path.Combine(scratch, "Cargo.lock")).Should().Be(PlatformRelease.LockFileText);
    }
}
