using System.IO;
using System.Linq;
using SilverAssertions;
using Xunit;

namespace CodeBrix.WebPlatform.WebGpu.Animation.Tests;

/// <summary>
/// The Animation package depends on the WebGpu package (which carries the wgpu crates and
/// codebrix-webgpu) and never on the Graphics package.
/// </summary>
public class AnimationPackageDependenciesTests
{
    [Fact]
    public void the_WebGpu_crate_archive_comes_with_the_animation_library()
        => File.Exists(WebGpuCrates.GetArchivePath()).Should().BeTrue("the WebGpu project's archive is copied beside whatever references the Animation library, as the package's buildTransitive targets do in a build program");

    [Fact]
    public void the_animation_assembly_does_not_reference_the_graphics_assembly()
        => typeof(AnimationRustCrate).Assembly.GetReferencedAssemblies().Select(a => a.Name).Should().NotContain("CodeBrix.WebPlatform.WebGpu.Graphics");
}
