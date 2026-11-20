using System.Linq;
using CodeBrix.WebPlatform.WebGpu.LibraryChecker.Checks;
using CodeBrix.WebPlatform.WebGpu.LibraryChecker.Tests.Helpers;
using SilverAssertions;
using Xunit;

namespace CodeBrix.WebPlatform.WebGpu.LibraryChecker.Tests.Checks;

public class FamilyPinCheckTests
{
    private const string Targets = """
        <Project>
          <Target Name="_PinProjectReferenceVersionsExactly" AfterTargets="_GetProjectReferenceVersions" Condition="'$(NuspecFile)' == ''">
            <ItemGroup>
              <_ProjectReferencesWithVersions Condition="'%(_ProjectReferencesWithVersions.ProjectVersion)' != ''">
                <ProjectVersion>[%(_ProjectReferencesWithVersions.ProjectVersion)]</ProjectVersion>
              </_ProjectReferencesWithVersions>
            </ItemGroup>
          </Target>
        </Project>
        """;

    private const string Project = """
        <Project Sdk="Microsoft.NET.Sdk">
          <ItemGroup>
            <ProjectReference Include="..\CodeBrix.WebPlatform.WebGpu\CodeBrix.WebPlatform.WebGpu.csproj" />
          </ItemGroup>
          <Import Project="..\CodeBrix.WebPlatform.WebGpu\ExactProjectReferencePins.targets" />
        </Project>
        """;

    private static CheckResult Check(string targets = Targets, string project = Project)
        => FamilyPinCheck.Check(targets, "ExactProjectReferencePins.targets", new[] { ("Graphics.csproj", project), ("Animation.csproj", Project) });

    [Fact]
    public void Check_passes_for_the_shared_target_imported_by_both_libraries()
    {
        //Act
        var result = Check();

        //Assert
        result.Passed.Should().BeTrue(result.ToText());
    }

    [Fact]
    public void Check_fails_when_a_library_does_not_import_the_shared_targets()
    {
        //Act
        var result = Check(project: Project.Replace("<Import Project=\"..\\CodeBrix.WebPlatform.WebGpu\\ExactProjectReferencePins.targets\" />", string.Empty));

        //Assert
        result.Problems.Single().Should().StartWith("Graphics.csproj does not import ExactProjectReferencePins.targets");
    }

    [Fact]
    public void Check_fails_when_the_target_runs_at_another_point()
    {
        //Act
        var result = Check(targets: Targets.Replace("AfterTargets=\"_GetProjectReferenceVersions\"", "AfterTargets=\"Build\""));

        //Assert
        result.Problems.Single().Should().Contain("AfterTargets=\"_GetProjectReferenceVersions\"");
    }

    [Fact]
    public void Check_fails_when_the_target_does_not_write_the_exact_range()
    {
        //Act
        var result = Check(targets: Targets.Replace("[%(_ProjectReferencesWithVersions.ProjectVersion)]", "%(_ProjectReferencesWithVersions.ProjectVersion)"));

        //Assert
        result.Problems.Single().Should().Contain("must write <ProjectVersion>[%(_ProjectReferencesWithVersions.ProjectVersion)]</ProjectVersion>");
    }

    [Fact]
    public void Check_fails_when_the_targets_file_is_missing()
        => Check(targets: null).Problems.Single().Should().Contain("does not exist");

    [Fact]
    public void Check_fails_for_a_package_reference_to_the_webgpu_package()
    {
        //Act
        var result = Check(project: Project.Replace("</ItemGroup>", "  <PackageReference Include=\"CodeBrix.WebPlatform.WebGpu.ApacheLicenseForever\" Version=\"[1.0.0]\" />\n  </ItemGroup>"));

        //Assert
        result.Problems.Single().Should().Contain("as a PackageReference");
    }

    [Fact]
    public void the_repository_passes()
    {
        //Act
        var result = FamilyPinCheck.Run(RepositoryUnderTest.Find());

        //Assert
        result.Passed.Should().BeTrue(result.ToText());
    }
}
