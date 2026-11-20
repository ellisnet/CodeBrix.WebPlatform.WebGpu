using System;
using System.Linq;
using System.Threading.Tasks;
using CodeBrix.WebPlatform.WebGpu.LibraryChecker.Checks;
using CodeBrix.WebPlatform.WebGpu.LibraryChecker.Tests.Helpers;
using SilverAssertions;
using Xunit;

namespace CodeBrix.WebPlatform.WebGpu.LibraryChecker.Tests;

/// <summary>
/// Runs every LibraryChecker check against the real repository this test project lives in: the
/// vendored crates, the comparison with the platform's crates, the first-party crates, the dependency rule, the license gate, the notices, the
/// no-Node rule and the no-workflow rule. These tests need cargo (they run it offline against the
/// vendor folder).
/// </summary>
public class CheckRunnerTests
{
    private static RepositoryLayout Layout()
    {
        var layout = RepositoryUnderTest.Find();
        layout.Should().NotBeNull("the test project sits under the repository's tests folder");
        return layout;
    }

    [Theory]
    [InlineData("no-node")]
    [InlineData("vendor")]
    [InlineData("platform")]
    [InlineData("first-party")]
    [InlineData("dependency-rule")]
    [InlineData("licenses")]
    [InlineData("notices")]
    public async Task the_repository_passes(string group)
    {
        //Act
        var results = await CheckRunner.RunAsync(Layout(), group, cancellationToken: TestContext.Current.CancellationToken);

        //Assert
        results.Should().NotBeEmpty();
        results.Where(r => !r.Passed).Select(r => r.ToText()).Should().BeEmpty();
    }

    [Fact]
    public void Groups_are_the_seven_check_groups_in_run_order()
        => CheckRunner.Groups.Should().Equal("no-node", "vendor", "platform", "first-party", "dependency-rule", "licenses", "notices");

    [Fact]
    public async Task the_first_party_group_checks_the_three_first_party_crates()
    {
        //Act
        var results = await CheckRunner.RunAsync(Layout(), "first-party", cancellationToken: TestContext.Current.CancellationToken);

        //Assert
        results.Select(r => r.Name).Should().HaveCount(3);
        results[0].Name.Should().Contain("codebrix-webgpu is ours");
        results[1].Name.Should().Contain("codebrix-webgpu-graphics is ours");
        results[2].Name.Should().Contain("codebrix-webgpu-animation is ours");
    }

    [Fact]
    public async Task the_vendor_and_platform_groups_check_the_animation_crate_set_too()
    {
        //Act
        var vendor = await CheckRunner.RunAsync(Layout(), "vendor", cancellationToken: TestContext.Current.CancellationToken);
        var platform = await CheckRunner.RunAsync(Layout(), "platform", cancellationToken: TestContext.Current.CancellationToken);

        //Assert
        vendor.Should().Contain(r => r.Name.Contains("the Animation package's crates are untouched", StringComparison.Ordinal) && r.Passed);
        platform.Should().Contain(r => r.Name.Contains("the Animation crate set shares every crate", StringComparison.Ordinal) && r.Passed);
    }

    [Fact]
    public async Task the_notices_group_checks_the_graphics_notices_too()
    {
        //Act
        var results = await CheckRunner.RunAsync(Layout(), "notices", cancellationToken: TestContext.Current.CancellationToken);

        //Assert
        results.Select(r => r.Name).Should().Contain(n => n.Contains("THIRD-PARTY-NOTICES-WebGpuGraphicsCrates.txt"));
        results.Select(r => r.Name).Should().Contain(SampleNoticesCheck.Name);
    }

    [Fact]
    public async Task RunAsync_rejects_an_unknown_check()
    {
        //Arrange
        Func<Task> act = () => CheckRunner.RunAsync(Layout(), "patches", cancellationToken: TestContext.Current.CancellationToken);

        //Assert
        await act.Should().ThrowAsync<ArgumentException>();
    }
}
