using CodeBrix.WebPlatform.WebGpu.LibraryChecker.Checks;
using CodeBrix.WebPlatform.WebGpu.LibraryChecker.Tests.Helpers;
using SilverAssertions;
using Xunit;

namespace CodeBrix.WebPlatform.WebGpu.LibraryChecker.Tests.Checks;

public class GitHubWorkflowCheckTests
{
    [Fact]
    public void Run_passes_for_a_github_folder_without_workflows()
    {
        //Arrange
        using var folder = new TempFolder();
        folder.Write(".github/copilot-instructions.md", "pointer");

        //Act
        var result = GitHubWorkflowCheck.Run(new RepositoryLayout(folder.Path));

        //Assert
        result.Passed.Should().BeTrue(result.ToText());
        result.Notes.Should().ContainSingle().Which.Should().Be("none in the vendored crates either");
    }

    [Fact]
    public void Run_fails_for_a_workflow_file_anywhere_outside_the_vendored_crates()
    {
        //Arrange
        using var folder = new TempFolder();
        folder.Write(".github/workflows/build.yml", "on: push");
        folder.Write("samples/GraphicsDemo/.github/workflows/ci.yaml", "on: push");

        //Act
        var result = GitHubWorkflowCheck.Run(new RepositoryLayout(folder.Path));

        //Assert
        result.Problems.Should().HaveCount(2);
        result.Problems[0].Should().StartWith(".github/workflows/build.yml is a GitHub workflow file");
        result.Problems[1].Should().StartWith("samples/GraphicsDemo/.github/workflows/ci.yaml");
    }

    [Fact]
    public void Run_names_the_vendored_crates_that_carry_workflow_files_in_a_note()
    {
        //Arrange
        using var folder = new TempFolder();
        folder.Write("src/CodeBrix.WebPlatform.WebGpu/Rust/vendor/a-crate-1.0.0/.github/workflows/ci.yml", "on: push");
        folder.Write("src/CodeBrix.WebPlatform.WebGpu/Rust/vendor/a-crate-1.0.0/.github/workflows/publish.yml", "on: push");
        folder.Write("src/CodeBrix.WebPlatform.WebGpu/Rust/vendor/b-crate-2.0.0/.github/workflows/ci.yml", "on: push");

        //Act
        var result = GitHubWorkflowCheck.Run(new RepositoryLayout(folder.Path));

        //Assert
        result.Passed.Should().BeTrue();
        result.Notes.Should().ContainSingle().Which.Should().EndWith("(never edited; reported for the maintainer, not judged): a-crate-1.0.0, b-crate-2.0.0");
    }

    [Fact]
    public void Run_does_not_scan_a_vendored_crate_that_a_frontend_build_unpacked()
    {
        //Arrange
        using var folder = new TempFolder();
        folder.Write("samples/GraphicsDemo/frontends/graphics/platform/vendor/a-crate-1.0.0/.cargo-checksum.json", "{}");
        folder.Write("samples/GraphicsDemo/frontends/graphics/platform/vendor/a-crate-1.0.0/.github/workflows/ci.yml", "on: push");

        //Act
        var result = GitHubWorkflowCheck.Run(new RepositoryLayout(folder.Path));

        //Assert
        result.Passed.Should().BeTrue(result.ToText());
        result.Notes.Should().ContainSingle().Which.Should().Be("none in the vendored crates either");
    }
}
