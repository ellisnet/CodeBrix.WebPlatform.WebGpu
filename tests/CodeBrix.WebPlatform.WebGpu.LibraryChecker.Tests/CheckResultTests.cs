using SilverAssertions;
using Xunit;

namespace CodeBrix.WebPlatform.WebGpu.LibraryChecker.Tests;

public class CheckResultTests
{
    [Fact]
    public void a_result_without_problems_passes_and_prints_its_notes()
    {
        //Act
        var result = new CheckResult("Vendor", null, new[] { "12 crates" });

        //Assert
        result.Passed.Should().BeTrue();
        result.ToText().Should().StartWith("PASS  Vendor").And.Contain("12 crates");
    }

    [Fact]
    public void a_result_with_problems_fails_and_prints_each_one()
    {
        //Act
        var result = new CheckResult("Licenses", new[] { "crate-a: GPL", "crate-b: unknown" });

        //Assert
        result.Passed.Should().BeFalse();
        result.Problems.Should().HaveCount(2);
        result.ToText().Should().StartWith("FAIL  Licenses").And.Contain("! crate-a: GPL").And.Contain("! crate-b: unknown");
    }
}
