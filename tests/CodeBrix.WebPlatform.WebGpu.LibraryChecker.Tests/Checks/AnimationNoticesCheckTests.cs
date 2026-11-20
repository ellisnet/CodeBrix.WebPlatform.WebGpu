using System.IO;
using System.Linq;
using CodeBrix.WebPlatform.WebGpu.LibraryChecker.Checks;
using SilverAssertions;
using Xunit;

namespace CodeBrix.WebPlatform.WebGpu.LibraryChecker.Tests.Checks;

public class AnimationNoticesCheckTests
{
    private const string Project = """
        <Project Sdk="Microsoft.NET.Sdk">
          <ItemGroup>
            <None Include="Rust\notices\THIRD-PARTY-NOTICES-WebGpuAnimationCrates.txt" Pack="true" PackagePath="tools\webgpu-animation-crates" />
          </ItemGroup>
        </Project>
        """;

    private static readonly string NoticesFromProject = Path.Combine("Rust", "notices", "THIRD-PARTY-NOTICES-WebGpuAnimationCrates.txt");

    private static string Header => string.Join("\n", AnimationNoticesCheck.RequiredWords) + "\n\n" + AnimationNoticesCheck.SkiaLicense + "\n\n" + AnimationNoticesCheck.UnicodeLicense;

    [Fact]
    public void Check_passes_for_a_header_with_every_entry_and_a_project_that_packs_the_notices()
    {
        //Act
        var result = AnimationNoticesCheck.Check(Header, "header.txt", Project, "Animation.csproj", NoticesFromProject);

        //Assert
        result.Passed.Should().BeTrue(result.ToText());
    }

    [Fact]
    public void Check_fails_without_the_skia_licence_text()
    {
        //Act
        var result = AnimationNoticesCheck.Check(string.Join("\n", AnimationNoticesCheck.RequiredWords) + "\n" + AnimationNoticesCheck.UnicodeLicense, "header.txt", Project, "Animation.csproj", NoticesFromProject);

        //Assert
        result.Passed.Should().BeFalse();
        result.Problems.Single().Should().Contain("does not carry Skia's licence").And.Contain("copy the text from that file");
    }

    [Fact]
    public void Check_fails_without_the_unicode_licence_text_for_harfrusts_tables()
    {
        //Act
        var result = AnimationNoticesCheck.Check(string.Join("\n", AnimationNoticesCheck.RequiredWords) + "\n" + AnimationNoticesCheck.SkiaLicense, "header.txt", Project, "Animation.csproj", NoticesFromProject);

        //Assert
        result.Passed.Should().BeFalse();
        result.Problems.Single().Should().Contain("UNICODE LICENSE V3").And.Contain("harfrust");
    }

    [Fact]
    public void Check_fails_when_the_header_is_missing_or_the_project_does_not_pack_the_notices()
    {
        //Act
        var result = AnimationNoticesCheck.Check(null, "header.txt", "<Project />", "Animation.csproj", NoticesFromProject);

        //Assert
        result.Problems.Should().HaveCount(2);
        result.Problems[0].Should().Contain("header.txt does not exist").And.Contain("hand-written");
        result.Problems[1].Should().Contain("does not pack").And.Contain("tools\\webgpu-animation-crates");
    }

    [Fact]
    public void Check_names_each_missing_entry()
    {
        //Act
        var result = AnimationNoticesCheck.Check(Header.Replace("SkRasterPipeline_opts.h", "somewhere", System.StringComparison.Ordinal), "header.txt", Project, "Animation.csproj", NoticesFromProject);

        //Assert
        result.Problems.Single().Should().Contain("does not say \"SkRasterPipeline_opts.h\"");
    }
}
