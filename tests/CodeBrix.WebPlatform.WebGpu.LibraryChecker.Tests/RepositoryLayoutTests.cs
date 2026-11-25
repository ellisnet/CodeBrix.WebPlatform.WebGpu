using System.IO;
using CodeBrix.WebPlatform.WebGpu.LibraryChecker.Tests.Helpers;
using SilverAssertions;
using Xunit;

namespace CodeBrix.WebPlatform.WebGpu.LibraryChecker.Tests;

public class RepositoryLayoutTests
{
    [Fact]
    public void Find_walks_up_to_the_folder_that_holds_the_solution_and_the_allowlist()
    {
        //Arrange
        using var folder = new TempFolder();
        folder.Write("CodeBrix.WebPlatform.WebGpu.slnx", "<Solution />");
        folder.Write("tools/licensing/license-allowlist.json", "{}");
        var deep = Directory.CreateDirectory(Path.Combine(folder.Path, "tests", "x", "bin", "Debug")).FullName;

        //Act
        var fromInside = RepositoryLayout.Find(deep);
        var fromRoot = RepositoryLayout.Find(folder.Path);

        //Assert
        fromInside.Root.Should().Be(folder.Path);
        fromRoot.Root.Should().Be(folder.Path);
    }

    [Fact]
    public void Find_returns_null_when_there_is_no_repository_root()
    {
        //Arrange
        using var folder = new TempFolder();
        folder.Write("tools/licensing/license-allowlist.json", "{}");
        folder.Write("CodeBrix.WebPlatform.slnx", "<Solution />");

        //Assert
        RepositoryLayout.Find(folder.Path).Should().BeNull("the platform's solution file does not mark this repository");
        RepositoryLayout.Find(" ").Should().BeNull();
        RepositoryLayout.IsRoot(folder.Path).Should().BeFalse();
    }

    [Fact]
    public void the_paths_hang_off_the_root_and_nothing_is_under_samples()
    {
        //Arrange
        var layout = new RepositoryLayout(Path.Combine(Path.GetTempPath(), "repo"));
        var rust = Path.Combine(layout.Root, "src", "CodeBrix.WebPlatform.WebGpu", "Rust");

        //Assert
        layout.Vendor.Should().Be(Path.Combine(rust, "vendor"));
        layout.VendorRelative.Should().Be("src/CodeBrix.WebPlatform.WebGpu/Rust/vendor");
        layout.CrateSetLock.Should().Be(Path.Combine(rust, "webgpu-crates", "Cargo.lock"));
        layout.NoticesFile.Should().Be(Path.Combine(rust, "notices", "THIRD-PARTY-NOTICES-WebGpuCrates.txt"));
        layout.NoticesHeaderFile.Should().Be(Path.Combine(rust, "notices", "notices-header-WebGpuCrates.txt"));
        layout.AllowlistFile.Should().Be(Path.Combine(layout.Root, "tools", "licensing", "license-allowlist.json"));
        foreach (var path in new[] { layout.Vendor, layout.CrateSet, layout.AllowlistFile, layout.NoticesFile })
        {
            path.Should().NotStartWith(layout.Samples);
        }
    }

    [Fact]
    public void the_graphics_and_animation_paths_hang_off_their_library_folders()
    {
        //Arrange
        var layout = new RepositoryLayout(Path.Combine(Path.GetTempPath(), "repo"));
        var graphics = Path.Combine(layout.Root, "src", "CodeBrix.WebPlatform.WebGpu.Graphics");
        var animation = Path.Combine(layout.Root, "src", "CodeBrix.WebPlatform.WebGpu.Animation");

        //Assert
        layout.GraphicsCrate.Should().Be(Path.Combine(graphics, "Rust", "codebrix-webgpu-graphics"));
        layout.GraphicsNoticesFile.Should().Be(Path.Combine(graphics, "Rust", "notices", "THIRD-PARTY-NOTICES-WebGpuGraphicsCrates.txt"));
        layout.GraphicsNoticesHeaderFile.Should().Be(Path.Combine(graphics, "Rust", "notices", "notices-header-WebGpuGraphicsCrates.txt"));
        layout.GraphicsCrateSetLock.Should().Be(Path.Combine(graphics, "Rust", "graphics-crates", "Cargo.lock"));
        layout.GraphicsVendor.Should().Be(Path.Combine(graphics, "Rust", "vendor"));
        layout.GraphicsVendorRelative.Should().Be("src/CodeBrix.WebPlatform.WebGpu.Graphics/Rust/vendor");
        layout.GraphicsProjectFile.Should().Be(Path.Combine(graphics, "CodeBrix.WebPlatform.WebGpu.Graphics.csproj"));
        layout.AnimationCrate.Should().Be(Path.Combine(animation, "Rust", "codebrix-webgpu-animation"));
        layout.AnimationProjectFile.Should().Be(Path.Combine(animation, "CodeBrix.WebPlatform.WebGpu.Animation.csproj"));
        layout.WebGpuProjectFile.Should().Be(Path.Combine(layout.Root, "src", "CodeBrix.WebPlatform.WebGpu", "CodeBrix.WebPlatform.WebGpu.csproj"));
    }
}
