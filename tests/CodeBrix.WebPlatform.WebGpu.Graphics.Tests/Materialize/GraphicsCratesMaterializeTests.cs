using System;
using System.IO;
using System.Linq;
using System.Text.Json;
using System.Threading.Tasks;
using CodeBrix.WebPlatform.Core.Crates;
using CodeBrix.WebPlatform.FrontendBuilder.Crates;
using SilverAssertions;
using Xunit;

namespace CodeBrix.WebPlatform.WebGpu.Graphics.Tests.Materialize;

/// <summary>
/// The platform's real MaterializeAsync after <see cref="GraphicsCrates.Register"/>, against the
/// real platform archive (from the FrontendBuilder package), the real WebGpu archive and the real
/// Graphics archive. The Materialize itself happens once, in <see cref="MaterializedFrontend"/>;
/// these tests read what it wrote and reported. The cargo tests are skipped on a machine without
/// cargo or without the wasm32 target; none needs the network, dx, a browser or a container.
/// </summary>
[Collection(MaterializeCollection.Name)]
public class GraphicsCratesMaterializeTests
{
    private readonly MaterializedFrontend _frontend;

    public GraphicsCratesMaterializeTests(MaterializedFrontend frontend)
    {
        _frontend = frontend;
        Assert.SkipWhen(frontend.NoCratesReason != null, frontend.NoCratesReason ?? string.Empty);
    }

    private string Vendor => _frontend.Options.ResolvedVendorDirectory;

    private CrateBundleOutcome GraphicsOutcome => _frontend.First.CrateBundles.Single(b => b.Name == GraphicsCrates.BundleName);

    private string GraphicsCrate => "codebrix-webgpu-graphics " + _frontend.GraphicsCrateVersion;

    [Fact]
    public void MaterializeAsync_reports_the_webgpu_and_the_webgpu_graphics_bundles_in_that_order()
        => _frontend.First.CrateBundles.Select(b => b.Name).Should().Equal(WebGpuCrates.BundleName, GraphicsCrates.BundleName);

    [Fact]
    public void MaterializeAsync_reports_the_webgpu_graphics_bundle_with_the_packages_name_and_version()
        => GraphicsOutcome.Origin.Should().Be(GraphicsCrates.PackageId + " " + GraphicsCrates.PackageVersion);

    [Fact]
    public void the_webgpu_graphics_bundle_adds_codebrix_webgpu_graphics_and_nothing_else()
    {
        //Act
        var added = GraphicsOutcome.AddedCrates;
        var shared = GraphicsOutcome.SharedCrates;

        //Assert
        added.Should().Equal(GraphicsCrate);
        shared.Should().BeEmpty();
    }

    [Fact]
    public void the_vendor_folder_holds_codebrix_webgpu_graphics_with_its_checksum_file()
    {
        //Arrange
        var folder = Path.Combine(Vendor, "codebrix-webgpu-graphics-" + _frontend.GraphicsCrateVersion);

        //Assert
        File.Exists(Path.Combine(folder, "Cargo.toml")).Should().BeTrue();
        File.Exists(Path.Combine(folder, ".cargo-checksum.json")).Should().BeTrue();
        File.Exists(Path.Combine(folder, "src", "lib.rs")).Should().BeTrue();
    }

    [Fact]
    public void the_vendor_folder_holds_exactly_one_folder_per_crate_of_the_three_archives()
    {
        //Arrange
        var expected = CrateFolders.InArchive(PlatformCrates.GetArchivePath())
            .Union(CrateFolders.InArchive(WebGpuCrates.GetArchivePath()), StringComparer.Ordinal)
            .Union(CrateFolders.InArchive(GraphicsCrates.GetArchivePath()), StringComparer.Ordinal)
            .OrderBy(c => c, StringComparer.Ordinal).ToList();

        //Act
        var folders = CrateFolders.InFolder(Vendor);

        //Assert
        folders.Should().OnlyContain(f => f.Crate != null && f.Folder == f.Crate.Replace(' ', '-'));
        folders.Select(f => f.Crate).OrderBy(c => c, StringComparer.Ordinal).Should().Equal(expected);
    }

    [Fact]
    public void Register_again_changes_nothing()
    {
        //Arrange
        var graphics = CrateBundles.Registered.Single(b => b.Name == GraphicsCrates.BundleName);
        var webGpu = CrateBundles.Registered.Single(b => b.Name == WebGpuCrates.BundleName);

        //Act
        GraphicsCrates.Register();
        GraphicsCrates.Register();

        //Assert
        CrateBundles.Registered.Single(b => b.Name == GraphicsCrates.BundleName).Should().BeSameAs(graphics);
        CrateBundles.Registered.Single(b => b.Name == WebGpuCrates.BundleName).Should().BeSameAs(webGpu);
    }

    [Fact]
    public async Task cargo_metadata_resolves_codebrix_webgpu_graphics_offline_from_the_vendor_folder()
    {
        //Arrange
        Assert.SkipUnless(Cargo.IsAvailable, "cargo is not installed (not on PATH): this test runs `cargo metadata --offline` on the materialized frontend.");
        var vendored = CrateFolders.InFolder(Vendor).Select(f => f.Crate).ToHashSet(StringComparer.Ordinal);

        //Act
        var run = await Cargo.RunAsync(_frontend.FrontendDirectory, Path.Combine(_frontend.Folder, "cargo-target"),
            new[] { "metadata", "--offline", "--format-version", "1" }, TestContext.Current.CancellationToken);

        //Assert
        run.ExitCode.Should().Be(0, run.Errors);
        using var metadata = JsonDocument.Parse(run.Output);
        var registryPackages = metadata.RootElement.GetProperty("packages").EnumerateArray()
            .Where(p => p.GetProperty("source").ValueKind == JsonValueKind.String)
            .Select(p => p.GetProperty("name").GetString() + " " + p.GetProperty("version").GetString())
            .ToList();
        registryPackages.Should().Contain(GraphicsCrate);
        registryPackages.Should().Contain("codebrix-webgpu " + _frontend.WebGpuCrateVersion);
        registryPackages.Should().OnlyContain(p => vendored.Contains(p), "cargo takes every crate from the vendor folder");
        registryPackages.Should().NotContain(p => p.StartsWith("codebrix-webgpu-animation ", StringComparison.Ordinal), "the Graphics crate never depends on the Animation crate");
    }

    [Fact]
    public async Task cargo_check_compiles_the_frontend_that_names_codebrix_webgpu_graphics_for_wasm32_offline()
    {
        //Arrange
        Assert.SkipUnless(Cargo.HasWasmTarget, $"cargo or the {Cargo.WasmTarget} target is not installed: this test runs `cargo check --offline --target {Cargo.WasmTarget}` on the materialized frontend.");

        //Act
        var run = await Cargo.RunAsync(_frontend.FrontendDirectory, Path.Combine(_frontend.Folder, "cargo-target"),
            new[] { "check", "--offline", "--target", Cargo.WasmTarget }, TestContext.Current.CancellationToken);

        //Assert
        run.ExitCode.Should().Be(0, run.Errors);
        run.Errors.Should().Contain("codebrix-webgpu-graphics v" + _frontend.GraphicsCrateVersion);
        run.Errors.Should().Contain("codebrix-webgpu v" + _frontend.WebGpuCrateVersion);
        run.Errors.Should().Contain("webgpu-graphics-test-frontend v0.1.0");
        run.Errors.Should().NotContain("warning: `codebrix-webgpu-graphics`", "the Graphics crate compiles without a warning");
        run.Errors.Should().NotContain("src/main.rs", "the frontend that uses the crate compiles without a warning");
    }

    [Fact]
    public async Task cargo_test_runs_the_unit_tests_of_codebrix_webgpu_graphics_offline_from_the_vendor_folder()
    {
        //Arrange
        Assert.SkipUnless(Cargo.IsAvailable, "cargo is not installed (not on PATH): this test runs `cargo test --offline` on a copy of the codebrix-webgpu-graphics crate from the materialized vendor folder.");
        var source = Path.Combine(Vendor, "codebrix-webgpu-graphics-" + _frontend.GraphicsCrateVersion);
        var crate = Path.Combine(_frontend.Folder, "codebrix-webgpu-graphics-tests");
        foreach (var file in Directory.GetFiles(source, "*", SearchOption.AllDirectories))
        {
            var relative = Path.GetRelativePath(source, file);
            if (relative == ".cargo-checksum.json")
            {
                continue;
            }

            Directory.CreateDirectory(Path.GetDirectoryName(Path.Combine(crate, relative)));
            File.Copy(file, Path.Combine(crate, relative));
        }

        Directory.CreateDirectory(Path.Combine(crate, ".cargo"));
        File.WriteAllText(Path.Combine(crate, ".cargo", "config.toml"),
            "[source.crates-io]\nreplace-with = \"vendored\"\n\n[source.vendored]\ndirectory = \"" + Vendor.Replace('\\', '/') + "\"\n\n[net]\noffline = true\n");

        //Act
        var run = await Cargo.RunAsync(crate, Path.Combine(_frontend.Folder, "cargo-target-host"),
            new[] { "test", "--offline" }, TestContext.Current.CancellationToken);

        //Assert
        run.ExitCode.Should().Be(0, run.Output + run.Errors);
        run.Output.Should().Contain("test result: ok.");
        run.Output.Should().NotContain("FAILED");
        run.Errors.Should().NotContain("warning: `codebrix-webgpu-graphics`", "the Graphics crate's tests compile without a warning");
    }
}
