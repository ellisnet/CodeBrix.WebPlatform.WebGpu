using System.Collections.Generic;
using System.Linq;
using CodeBrix.WebPlatform.WebGpu.LibraryChecker.Checks;
using CodeBrix.WebPlatform.WebGpu.LibraryChecker.Platform;
using CodeBrix.WebPlatform.WebGpu.LibraryChecker.Tests.Helpers;
using SilverAssertions;
using Xunit;

namespace CodeBrix.WebPlatform.WebGpu.LibraryChecker.Tests.Checks;

public class FirstPartyCrateCheckTests
{
    private const string Good = """
        # a comment
        [package]
        name = "codebrix-webgpu"
        version = "1.0.0"
        edition = "2021"
        publish = false
        license = "Apache-2.0"

        [dependencies]
        dioxus = { version = "=0.7.10", default-features = false, features = ["macro"] }
        wgpu = { version = "=30.0.2", default-features = false }
        wasm-bindgen = "=0.2.130"

        [dev-dependencies]
        bytemuck = "=1.25.2"
        """;

    private static (CrateSnapshot Vendored, CrateSnapshot Platform) Snapshots(TempFolder folder)
    {
        folder.WriteCrate("vendor", "wgpu", "30.0.2");
        folder.WriteCrate("vendor", "bytemuck", "1.25.2");
        folder.WriteCrate("platform", "dioxus", "0.7.10");
        folder.WriteCrate("platform", "wasm-bindgen", "0.2.130");
        return (CrateSnapshot.FromDirectory(folder.Full("vendor")), CrateSnapshot.FromDirectory(folder.Full("platform")));
    }

    [Fact]
    public void Check_passes_for_a_crate_of_ours_whose_dependencies_are_carried_at_exact_versions()
    {
        //Arrange
        using var folder = new TempFolder();
        var (vendored, platform) = Snapshots(folder);

        //Act
        var result = FirstPartyCrateCheck.Check(Good, "Cargo.toml", vendored, platform);

        //Assert
        result.Passed.Should().BeTrue(result.ToText());
        result.Notes.Single().Should().StartWith("codebrix-webgpu 1.0.0: first-party");
    }

    [Fact]
    public void Check_fails_for_a_range_requirement_and_says_to_name_an_exact_version()
    {
        //Arrange
        using var folder = new TempFolder();
        var (vendored, platform) = Snapshots(folder);

        //Act
        var result = FirstPartyCrateCheck.Check(Good.Replace("wasm-bindgen = \"=0.2.130\"", "wasm-bindgen = \"0.2\""), "Cargo.toml", vendored, platform);

        //Assert
        result.Passed.Should().BeFalse();
        result.Problems.Single().Should().Contain("wasm-bindgen = \"0.2\"").And.Contain("EXACT version");
    }

    [Fact]
    public void Check_fails_for_a_version_no_carried_crate_has_and_names_the_carried_one()
    {
        //Arrange
        using var folder = new TempFolder();
        var (vendored, platform) = Snapshots(folder);

        //Act
        var result = FirstPartyCrateCheck.Check(Good.Replace("=0.7.10", "=0.7.9"), "Cargo.toml", vendored, platform);

        //Assert
        result.Problems.Single().Should().Contain("dioxus 0.7.9").And.Contain("carry dioxus 0.7.10");
    }

    [Fact]
    public void Check_fails_for_a_crate_nobody_carries()
    {
        //Arrange
        using var folder = new TempFolder();
        var (vendored, platform) = Snapshots(folder);

        //Act
        var result = FirstPartyCrateCheck.Check(Good + "\nreqwest = \"=0.12.0\"\n", "Cargo.toml", vendored, platform);

        //Assert
        result.Problems.Single().Should().Contain("neither the vendored crates nor the platform's carry reqwest");
    }

    [Fact]
    public void Check_fails_for_a_path_dependency()
    {
        //Arrange
        using var folder = new TempFolder();
        var (vendored, platform) = Snapshots(folder);

        //Act
        var result = FirstPartyCrateCheck.Check(Good + "\nhelper = { path = \"../helper\" }\n", "Cargo.toml", vendored, platform);

        //Assert
        result.Problems.Single().Should().Contain("helper is a path, git or workspace dependency");
    }

    [Fact]
    public void Check_fails_for_another_licence_a_missing_publish_false_and_a_third_party_name()
    {
        //Arrange
        using var folder = new TempFolder();
        var (vendored, platform) = Snapshots(folder);
        var manifest = Good.Replace("license = \"Apache-2.0\"", "license = \"MIT\"").Replace("publish = false\n", string.Empty).Replace("name = \"codebrix-webgpu\"", "name = \"wgpu\"");

        //Act
        var result = FirstPartyCrateCheck.Check(manifest, "Cargo.toml", vendored, platform);

        //Assert
        result.Problems.Should().HaveCount(3);
        result.Problems.Should().Contain(p => p.Contains("license = \"Apache-2.0\""));
        result.Problems.Should().Contain(p => p.Contains("publish = false"));
        result.Problems.Should().Contain(p => p.Contains("the name wgpu is a third-party crate's"));
    }

    [Fact]
    public void Check_fails_for_a_dependency_written_as_its_own_table()
    {
        //Arrange
        using var folder = new TempFolder();
        var (vendored, platform) = Snapshots(folder);

        //Act
        var result = FirstPartyCrateCheck.Check(Good + "\n[dependencies.glam]\nversion = \"=0.34.3\"\n", "Cargo.toml", vendored, platform);

        //Assert
        result.Passed.Should().BeFalse();
        result.Problems.First().Should().Contain("[dependencies.glam]").And.Contain("one line");
    }

    private const string Graphics = """
        [package]
        name = "codebrix-webgpu-graphics"
        version = "1.0.0"
        edition = "2021"
        publish = false
        license = "Apache-2.0"

        [dependencies]
        codebrix-webgpu = "=1.0.0"
        wgpu = { version = "=30.0.2", default-features = false, features = ["std", "webgpu", "wgsl"] }
        bytemuck = "=1.25.2"
        """;

    private static readonly IReadOnlyDictionary<string, string> WebGpuCrate = new Dictionary<string, string> { ["codebrix-webgpu"] = "1.0.0" };

    [Fact]
    public void Check_passes_for_a_crate_that_names_the_first_party_codebrix_webgpu_at_its_version()
    {
        //Arrange
        using var folder = new TempFolder();
        var (vendored, platform) = Snapshots(folder);

        //Act
        var result = FirstPartyCrateCheck.Check(Graphics, "Cargo.toml", vendored, platform, WebGpuCrate);

        //Assert
        result.Passed.Should().BeTrue(result.ToText());
        result.Name.Should().Contain("codebrix-webgpu-graphics");
        result.Notes.Single().Should().StartWith("codebrix-webgpu-graphics 1.0.0: first-party").And.Contain("3 dependencies");
    }

    [Fact]
    public void Check_fails_for_codebrix_webgpu_at_another_version_and_names_its_version()
    {
        //Arrange
        using var folder = new TempFolder();
        var (vendored, platform) = Snapshots(folder);

        //Act
        var result = FirstPartyCrateCheck.Check(Graphics.Replace("codebrix-webgpu = \"=1.0.0\"", "codebrix-webgpu = \"=1.0.1\""), "Cargo.toml", vendored, platform, WebGpuCrate);

        //Assert
        result.Problems.Single().Should().Contain("codebrix-webgpu 1.0.1").And.Contain("is at 1.0.0");
    }

    [Fact]
    public void Check_fails_for_codebrix_webgpu_when_it_is_not_known_as_a_first_party_crate()
    {
        //Arrange
        using var folder = new TempFolder();
        var (vendored, platform) = Snapshots(folder);

        //Act
        var result = FirstPartyCrateCheck.Check(Graphics, "Cargo.toml", vendored, platform);

        //Assert
        result.Problems.Single().Should().Contain("neither the vendored crates nor the platform's carry codebrix-webgpu");
    }

    [Fact]
    public void Check_fails_for_a_crate_named_as_another_first_party_crate()
    {
        //Arrange
        using var folder = new TempFolder();
        var (vendored, platform) = Snapshots(folder);

        //Act
        var result = FirstPartyCrateCheck.Check(Good, "Cargo.toml", vendored, platform, WebGpuCrate);

        //Assert
        result.Problems.Single().Should().Contain("the name codebrix-webgpu is another first-party crate's");
    }

    [Fact]
    public void ReadPackageVersion_reads_the_version_of_the_package_table_only()
        => FirstPartyCrateCheck.ReadPackageVersion("[dependencies]\nversion = \"9.9.9\"\n[package]\nname = \"x\"\nversion = \"1.2.3\"\n").Should().Be("1.2.3");
}
