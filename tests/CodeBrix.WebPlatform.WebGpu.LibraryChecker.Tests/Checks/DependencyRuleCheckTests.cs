using System.Linq;
using CodeBrix.WebPlatform.WebGpu.LibraryChecker.Checks;
using CodeBrix.WebPlatform.WebGpu.LibraryChecker.Tests.Helpers;
using SilverAssertions;
using Xunit;

namespace CodeBrix.WebPlatform.WebGpu.LibraryChecker.Tests.Checks;

public class DependencyRuleCheckTests
{
    private const string GraphicsProject = """
        <Project Sdk="Microsoft.NET.Sdk">
          <!-- Never a reference to the Animation package. -->
          <ItemGroup>
            <PackageReference Include="CodeBrix.WebPlatform.Core.ApacheLicenseForever" Version="[1.0.323.1353]" />
            <ProjectReference Include="..\CodeBrix.WebPlatform.WebGpu\CodeBrix.WebPlatform.WebGpu.csproj" />
          </ItemGroup>
        </Project>
        """;

    private const string GraphicsManifest = """
        # Depends on codebrix-webgpu and NEVER on codebrix-webgpu-animation.
        [package]
        name = "codebrix-webgpu-graphics"
        version = "1.0.0"

        [dependencies]
        codebrix-webgpu = "=1.0.0"
        wgpu = { version = "=30.0.2", default-features = false }
        """;

    private static readonly DependencyRuleCheck.Side Graphics = new("Graphics", "Graphics.csproj", "graphics-crate", true, "Animation", "CodeBrix.WebPlatform.WebGpu.Animation", "codebrix-webgpu-animation");

    private static DependencyRuleCheck.Side Side(TempFolder folder, DependencyRuleCheck.Side side)
        => side with { ProjectFile = folder.Full(side.ProjectFile), CrateFolder = folder.Full(side.CrateFolder) };

    [Fact]
    public void CheckProject_passes_for_a_reference_to_the_webgpu_project_and_a_comment_that_names_the_other_library()
        => DependencyRuleCheck.CheckProject(GraphicsProject, "Graphics.csproj", Graphics).Should().BeEmpty();

    [Fact]
    public void CheckProject_fails_for_a_project_reference_to_the_other_library()
    {
        //Arrange
        var project = GraphicsProject.Replace("</ItemGroup>", "  <ProjectReference Include=\"..\\CodeBrix.WebPlatform.WebGpu.Animation\\CodeBrix.WebPlatform.WebGpu.Animation.csproj\" />\n  </ItemGroup>");

        //Act
        var problems = DependencyRuleCheck.CheckProject(project, "Graphics.csproj", Graphics);

        //Assert
        problems.Single().Should().Contain("names the Animation library").And.Contain("never depend on each other");
    }

    [Fact]
    public void CheckProject_fails_for_a_package_reference_to_the_other_library()
    {
        //Arrange
        var project = GraphicsProject.Replace("</ItemGroup>", "  <PackageReference Include=\"CodeBrix.WebPlatform.WebGpu.Animation.ApacheLicenseForever\" Version=\"1.0.0\" />\n  </ItemGroup>");

        //Act
        var problems = DependencyRuleCheck.CheckProject(project, "Graphics.csproj", Graphics);

        //Assert
        problems.Single().Should().Contain("<PackageReference Include=\"CodeBrix.WebPlatform.WebGpu.Animation.ApacheLicenseForever\">");
    }

    [Fact]
    public void CheckProject_fails_for_a_package_beyond_core_and_says_it_belongs_in_the_application()
    {
        //Arrange
        var project = GraphicsProject.Replace("</ItemGroup>", "  <PackageReference Include=\"CodeBrix.Graphics3D.Gltf2.MitLicenseForever\" Version=\"1.0.290.996\" />\n  </ItemGroup>");

        //Act
        var problems = DependencyRuleCheck.CheckProject(project, "Graphics.csproj", Graphics);

        //Assert
        problems.Single().Should().Contain("CodeBrix.Graphics3D.Gltf2.MitLicenseForever").And.Contain("Core and the WebGpu package ONLY").And.Contain("belongs in the application");
    }

    [Fact]
    public void CheckProject_fails_for_another_project_reference()
    {
        //Arrange
        var project = GraphicsProject.Replace("</ItemGroup>", "  <ProjectReference Include=\"..\\Helpers\\Helpers.csproj\" />\n  </ItemGroup>");

        //Act
        var problems = DependencyRuleCheck.CheckProject(project, "Graphics.csproj", Graphics);

        //Assert
        problems.Single().Should().Contain("Helpers.csproj").And.Contain("is neither");
    }

    [Fact]
    public void CheckProject_fails_without_the_reference_to_the_webgpu_project_and_says_to_add_it()
    {
        //Arrange
        var project = GraphicsProject.Replace("<ProjectReference Include=\"..\\CodeBrix.WebPlatform.WebGpu\\CodeBrix.WebPlatform.WebGpu.csproj\" />", string.Empty);

        //Act
        var problems = DependencyRuleCheck.CheckProject(project, "Graphics.csproj", Graphics);

        //Assert
        problems.Single().Should().Contain("no <ProjectReference").And.Contain("add the reference");
    }

    [Fact]
    public void CheckManifest_passes_for_a_crate_that_names_codebrix_webgpu_and_mentions_the_other_crate_only_in_a_comment()
        => DependencyRuleCheck.CheckManifest(GraphicsManifest, "Cargo.toml", Graphics).Should().BeEmpty();

    [Fact]
    public void CheckManifest_fails_for_a_dependency_on_the_other_crate()
    {
        //Act
        var problems = DependencyRuleCheck.CheckManifest(GraphicsManifest + "codebrix-webgpu-animation = \"=1.0.0\"\n", "Cargo.toml", Graphics);

        //Assert
        problems.Single().Should().Contain("names the Animation package's crate codebrix-webgpu-animation");
    }

    [Fact]
    public void CheckManifest_fails_for_the_other_crate_under_another_name()
    {
        //Act
        var problems = DependencyRuleCheck.CheckManifest(GraphicsManifest + "motion = { package = \"codebrix-webgpu-animation\", version = \"=1.0.0\" }\n", "Cargo.toml", Graphics);

        //Assert
        problems.Single().Should().Contain("codebrix-webgpu-animation");
    }

    [Fact]
    public void CheckManifest_fails_when_codebrix_webgpu_is_not_a_dependency()
    {
        //Act
        var problems = DependencyRuleCheck.CheckManifest(GraphicsManifest.Replace("codebrix-webgpu = \"=1.0.0\"\n", string.Empty), "Cargo.toml", Graphics);

        //Assert
        problems.Single().Should().Contain("does not name codebrix-webgpu");
    }

    [Fact]
    public void CheckManifest_fails_when_codebrix_webgpu_is_only_a_dev_dependency()
    {
        //Arrange
        var manifest = GraphicsManifest.Replace("codebrix-webgpu = \"=1.0.0\"\n", string.Empty) + "\n[dev-dependencies]\ncodebrix-webgpu = \"=1.0.0\"\n";

        //Act
        var problems = DependencyRuleCheck.CheckManifest(manifest, "Cargo.toml", Graphics);

        //Assert
        problems.Single().Should().Contain("does not name codebrix-webgpu");
    }

    [Fact]
    public void CheckRustSource_fails_for_a_use_of_the_other_crate()
    {
        //Act
        var problems = DependencyRuleCheck.CheckRustSource("//! A module.\nuse codebrix_webgpu_animation::Clip;\n", "src/lib.rs", Graphics);

        //Assert
        problems.Single().Should().StartWith("src/lib.rs:2:").And.Contain("codebrix_webgpu_animation");
    }

    [Fact]
    public void Check_passes_for_a_library_with_its_project_and_crate_in_order()
    {
        //Arrange
        using var folder = new TempFolder();
        folder.Write("Graphics.csproj", GraphicsProject);
        folder.Write("graphics-crate/Cargo.toml", GraphicsManifest);
        folder.Write("graphics-crate/src/lib.rs", "//! The crate.\nuse codebrix_webgpu as _;\n");

        //Act
        var result = DependencyRuleCheck.Check(Side(folder, Graphics), p => p);

        //Assert
        result.Passed.Should().BeTrue(result.ToText());
        result.Name.Should().Be("Dependency rule: Graphics depends on Core and the WebGpu project only, on the WebGpu crate, and names nothing of Animation");
    }

    [Fact]
    public void Check_fails_when_a_required_crate_is_missing()
    {
        //Arrange
        using var folder = new TempFolder();
        folder.Write("Graphics.csproj", GraphicsProject);

        //Act
        var result = DependencyRuleCheck.Check(Side(folder, Graphics), p => p);

        //Assert
        result.Problems.Single().Should().Contain("Cargo.toml does not exist");
    }

    [Fact]
    public void Check_passes_with_a_note_when_an_optional_crate_does_not_exist_yet()
    {
        //Arrange
        using var folder = new TempFolder();
        var animation = new DependencyRuleCheck.Side("Animation", "Animation.csproj", "animation-crate", false, "Graphics", "CodeBrix.WebPlatform.WebGpu.Graphics", "codebrix-webgpu-graphics");
        folder.Write("Animation.csproj", GraphicsProject);

        //Act
        var result = DependencyRuleCheck.Check(Side(folder, animation), p => p);

        //Assert
        result.Passed.Should().BeTrue(result.ToText());
        result.Notes.Single().Should().Contain("does not exist yet");
    }

    [Fact]
    public void Check_fails_for_a_use_of_the_other_crate_in_the_crate_source()
    {
        //Arrange
        using var folder = new TempFolder();
        folder.Write("Graphics.csproj", GraphicsProject);
        folder.Write("graphics-crate/Cargo.toml", GraphicsManifest);
        folder.Write("graphics-crate/src/deep/mod.rs", "pub use codebrix_webgpu_animation::Clip;\n");

        //Act
        var result = DependencyRuleCheck.Check(Side(folder, Graphics), p => p);

        //Assert
        result.Problems.Single().Should().Contain("mod.rs:1:");
    }

    [Fact]
    public void Sides_are_graphics_then_animation_each_naming_the_other()
    {
        //Arrange
        var layout = new RepositoryLayout(System.IO.Path.Combine(System.IO.Path.GetTempPath(), "repo"));

        //Act
        var sides = DependencyRuleCheck.Sides(layout);

        //Assert
        sides.Select(s => (s.Library, s.OtherCrateName, s.CrateRequired)).Should().Equal(("Graphics", "codebrix-webgpu-animation", true), ("Animation", "codebrix-webgpu-graphics", true));
        sides[0].ProjectFile.Should().Be(layout.GraphicsProjectFile);
        sides[1].CrateFolder.Should().Be(layout.AnimationCrate);
    }
}
