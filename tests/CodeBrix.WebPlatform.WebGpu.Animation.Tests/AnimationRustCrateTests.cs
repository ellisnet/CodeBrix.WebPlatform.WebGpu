using SilverAssertions;
using Xunit;

namespace CodeBrix.WebPlatform.WebGpu.Animation.Tests;

public class AnimationRustCrateTests
{
    [Fact]
    public void CrateName_is_the_name_cargo_knows_the_crate_by()
        => AnimationRustCrate.CrateName.Should().Be("codebrix-webgpu-animation");

    [Fact]
    public void RustName_is_the_crate_name_with_underscores_for_hyphens()
        => AnimationRustCrate.RustName.Should().Be(AnimationRustCrate.CrateName.Replace('-', '_'));
}
