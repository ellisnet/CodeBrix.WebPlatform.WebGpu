using SilverAssertions;
using Xunit;

namespace CodeBrix.WebPlatform.WebGpu.Graphics.Tests;

public class GraphicsRustCrateTests
{
    [Fact]
    public void CrateName_is_the_name_cargo_knows_the_crate_by()
        => GraphicsRustCrate.CrateName.Should().Be("codebrix-webgpu-graphics");

    [Fact]
    public void RustName_is_the_crate_name_with_underscores_for_hyphens()
        => GraphicsRustCrate.RustName.Should().Be(GraphicsRustCrate.CrateName.Replace('-', '_'));
}
