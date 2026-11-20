using System.Linq;
using CodeBrix.WebPlatform.WebGpu.LibraryChecker.Checks;
using SilverAssertions;
using Xunit;

namespace CodeBrix.WebPlatform.WebGpu.LibraryChecker.Tests.Checks;

public class GraphicsNoticesCheckTests
{
    private const string NoticesFromProject = "Rust/notices/THIRD-PARTY-NOTICES-WebGpuGraphicsCrates.txt";

    private const string Project = """
        <Project Sdk="Microsoft.NET.Sdk">
          <ItemGroup>
            <None Include="Rust\notices\THIRD-PARTY-NOTICES-WebGpuGraphicsCrates.txt"
                  Pack="true" PackagePath="tools\webgpu-graphics-crates"
                  Link="webgpu-graphics-crates\THIRD-PARTY-NOTICES-WebGpuGraphicsCrates.txt" />
          </ItemGroup>
        </Project>
        """;

    private static string Notices(string mitNotice = null) => """
        The archive holds codebrix-webgpu-graphics, this repository's own code (Apache-2.0).

        1. aces_fitted - from ACES.hlsl of https://github.com/TheRealMJP/BakingLab,
           originally written by Stephen Hill (@self_shadow). License: MIT.

        """ + (mitNotice ?? GraphicsNoticesCheck.BakingLabMitNotice.Replace("\n", "\n   ")) + """


        2. aces_narkowicz - Krzysztof Narkowicz, "ACES Filmic Tone Mapping Curve" (2016); CC0 elected.

        3. pbr_neutral - pbrNeutral.glsl of https://github.com/KhronosGroup/ToneMapping; Apache-2.0.

        4. RadianceHdrDecoder - the logic of stb_image.h, https://github.com/nothings/stb; Public Domain elected.
        """;

    private static CheckResult Check(string notices, string project = Project)
        => GraphicsNoticesCheck.Check(notices, "notices.txt", project, "Graphics.csproj", NoticesFromProject);

    [Fact]
    public void Check_passes_for_packed_notices_with_the_four_entries_and_the_mit_text_reflowed()
    {
        //Act
        var result = Check(Notices());

        //Assert
        result.Passed.Should().BeTrue(result.ToText());
    }

    [Fact]
    public void Check_fails_when_the_notices_file_is_missing_and_says_it_is_hand_written()
        => Check(null).Problems.Single().Should().Contain("does not exist").And.Contain("hand-written");

    [Fact]
    public void Check_fails_without_the_mit_text_of_the_bakinglab_licence()
    {
        //Act
        var result = Check(Notices(mitNotice: "MIT License\n\nCopyright (c) 2016 MJP\n"));

        //Assert
        result.Problems.Single().Should().Contain("MIT notice of the BakingLab repository's LICENSE verbatim");
    }

    [Fact]
    public void Check_fails_for_a_missing_entry_and_names_the_function()
    {
        //Act
        var result = Check(Notices().Replace("pbr_neutral", "neutral"));

        //Assert
        result.Problems.Single().Should().Contain("no entry for pbr_neutral");
    }

    [Fact]
    public void Check_fails_for_an_entry_without_its_licence()
    {
        //Act
        var result = Check(Notices().Replace("CC0 elected", "credited"));

        //Assert
        result.Problems.Single().Should().Contain("aces_narkowicz must name \"CC0\"");
    }

    [Fact]
    public void Check_fails_when_the_project_does_not_pack_the_notices_beside_the_archive()
    {
        //Act
        var result = Check(Notices(), Project.Replace("tools\\webgpu-graphics-crates", "tools\\elsewhere"));

        //Assert
        result.Problems.Single().Should().Contain("does not pack").And.Contain("PackagePath=\"tools\\webgpu-graphics-crates\"");
    }

    [Fact]
    public void the_repository_notices_pass()
    {
        //Arrange
        var layout = Helpers.RepositoryUnderTest.Find();

        //Act
        var result = GraphicsNoticesCheck.Run(layout);

        //Assert
        result.Passed.Should().BeTrue(result.ToText());
    }
}
