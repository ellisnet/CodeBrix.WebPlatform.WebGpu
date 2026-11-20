using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using System.Text.Json;
using System.Threading.Tasks;
using CodeBrix.WebPlatform.Core.Crates;
using CodeBrix.WebPlatform.FrontendBuilder.Crates;
using CodeBrix.WebPlatform.WebGpu.Graphics;
using SilverAssertions;
using Xunit;

namespace CodeBrix.WebPlatform.WebGpu.Animation.Tests.Materialize;

/// <summary>
/// The platform's real MaterializeAsync after <see cref="AnimationCrates.Register"/>, against the
/// real platform archive (from the FrontendBuilder package), the real WebGpu archive and the real
/// Animation archive; and a second one with the Graphics package registered as well. The
/// Materialize runs happen once, in <see cref="MaterializedFrontend"/>; these tests read what they
/// wrote and reported. The cargo tests are skipped on a machine without cargo or without the
/// wasm32 target; none needs the network, dx, a browser or a container.
/// </summary>
[Collection(MaterializeCollection.Name)]
public class AnimationCratesMaterializeTests
{
    private readonly MaterializedFrontend _frontend;

    public AnimationCratesMaterializeTests(MaterializedFrontend frontend)
    {
        _frontend = frontend;
        Assert.SkipWhen(frontend.NoCratesReason != null, frontend.NoCratesReason ?? string.Empty);
    }

    private string Vendor => _frontend.Options.ResolvedVendorDirectory;

    private string BothVendor => _frontend.BothOptions.ResolvedVendorDirectory;

    private CrateBundleOutcome AnimationOutcome => _frontend.First.CrateBundles.Single(b => b.Name == AnimationCrates.BundleName);

    private string AnimationCrate => "codebrix-webgpu-animation " + _frontend.AnimationCrateVersion;

    [Fact]
    public void MaterializeAsync_reports_the_webgpu_and_the_webgpu_animation_bundles_in_that_order()
        => _frontend.First.CrateBundles.Select(b => b.Name).Should().Equal(WebGpuCrates.BundleName, AnimationCrates.BundleName);

    [Fact]
    public void MaterializeAsync_reports_the_webgpu_animation_bundle_with_the_packages_name_and_version()
        => AnimationOutcome.Origin.Should().Be(AnimationCrates.PackageId + " " + AnimationCrates.PackageVersion);

    [Fact]
    public void the_webgpu_animation_bundle_adds_its_crate_and_its_own_third_party_crates_and_shares_nothing()
    {
        //Arrange
        var expected = CrateFolders.InArchive(AnimationCrates.GetArchivePath());

        //Act
        var added = AnimationOutcome.AddedCrates;
        var shared = AnimationOutcome.SharedCrates;

        //Assert
        added.OrderBy(c => c, StringComparer.Ordinal).Should().Equal(expected);
        added.Should().Contain(new[] { AnimationCrate, "kurbo 0.13.1", "peniko 0.6.1", "i_overlay 9.2.0" });
        added.Should().Contain(new[] { "usvg 0.50.0", "svgtypes 0.16.2", "fontdb 0.24.0", "skrifa 0.49.0" }, "SVG documents, with usvg's text feature");
        shared.Should().BeEmpty("the Animation bundle carries only what neither the platform nor the WebGpu package carries");
    }

    [Fact]
    public void the_vendor_folder_holds_codebrix_webgpu_animation_with_its_checksum_file()
    {
        //Arrange
        var folder = Path.Combine(Vendor, "codebrix-webgpu-animation-" + _frontend.AnimationCrateVersion);

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
            .Union(CrateFolders.InArchive(AnimationCrates.GetArchivePath()), StringComparer.Ordinal)
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
        var animation = CrateBundles.Registered.Single(b => b.Name == AnimationCrates.BundleName);
        var webGpu = CrateBundles.Registered.Single(b => b.Name == WebGpuCrates.BundleName);

        //Act
        AnimationCrates.Register();
        AnimationCrates.Register();

        //Assert
        CrateBundles.Registered.Single(b => b.Name == AnimationCrates.BundleName).Should().BeSameAs(animation);
        CrateBundles.Registered.Single(b => b.Name == WebGpuCrates.BundleName).Should().BeSameAs(webGpu);
    }

    [Fact]
    public async Task cargo_metadata_resolves_codebrix_webgpu_animation_offline_from_the_vendor_folder()
    {
        //Arrange
        Assert.SkipUnless(Cargo.IsAvailable, "cargo is not installed (not on PATH): this test runs `cargo metadata --offline` on the materialized frontend.");
        var vendored = CrateFolders.InFolder(Vendor).Select(f => f.Crate).ToHashSet(StringComparer.Ordinal);

        //Act
        var run = await Cargo.RunAsync(_frontend.FrontendDirectory, Path.Combine(_frontend.Folder, "cargo-target"),
            new[] { "metadata", "--offline", "--format-version", "1" }, TestContext.Current.CancellationToken);

        //Assert
        run.ExitCode.Should().Be(0, run.Errors);
        var registryPackages = RegistryPackages(run.Output);
        registryPackages.Should().Contain(AnimationCrate);
        registryPackages.Should().Contain("codebrix-webgpu " + _frontend.WebGpuCrateVersion);
        registryPackages.Should().Contain(new[] { "kurbo 0.13.1", "peniko 0.6.1" });
        registryPackages.Should().OnlyContain(p => vendored.Contains(p), "cargo takes every crate from the vendor folder");
        registryPackages.Should().NotContain(p => p.StartsWith("codebrix-webgpu-graphics ", StringComparison.Ordinal), "the Animation crate never depends on the Graphics crate");
    }

    [Fact]
    public async Task cargo_check_compiles_the_frontend_that_names_codebrix_webgpu_animation_for_wasm32_offline()
    {
        //Arrange
        Assert.SkipUnless(Cargo.HasWasmTarget, $"cargo or the {Cargo.WasmTarget} target is not installed: this test runs `cargo check --offline --target {Cargo.WasmTarget}` on the materialized frontend.");

        //Act
        var run = await Cargo.RunAsync(_frontend.FrontendDirectory, Path.Combine(_frontend.Folder, "cargo-target"),
            new[] { "check", "--offline", "--target", Cargo.WasmTarget }, TestContext.Current.CancellationToken);

        //Assert
        run.ExitCode.Should().Be(0, run.Errors);
        run.Errors.Should().Contain("codebrix-webgpu-animation v" + _frontend.AnimationCrateVersion);
        run.Errors.Should().Contain("codebrix-webgpu v" + _frontend.WebGpuCrateVersion);
        run.Errors.Should().Contain("webgpu-animation-test-frontend v0.1.0");
        run.Errors.Should().NotContain("warning: `codebrix-webgpu-animation`", "the Animation crate compiles without a warning");
        run.Errors.Should().NotContain("src/main.rs", "the frontend that uses the crate compiles without a warning");
    }

    [Fact]
    public async Task cargo_test_runs_the_unit_tests_of_codebrix_webgpu_animation_offline_from_the_vendor_folder()
    {
        //Arrange
        Assert.SkipUnless(Cargo.IsAvailable, "cargo is not installed (not on PATH): this test runs `cargo test --offline` on a copy of the codebrix-webgpu-animation crate from the materialized vendor folder.");
        var source = Path.Combine(Vendor, "codebrix-webgpu-animation-" + _frontend.AnimationCrateVersion);
        var crate = Path.Combine(_frontend.Folder, "codebrix-webgpu-animation-tests");
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

        var fonts = new Dictionary<string, string> { ["CODEBRIX_ANIMATION_TEST_FONTS"] = Path.Combine(AppContext.BaseDirectory, "Fonts") };
        fonts["CODEBRIX_ANIMATION_TEST_LOTTIE"] = Path.Combine(AppContext.BaseDirectory, "Lottie");

        //Act
        var run = await Cargo.RunAsync(crate, Path.Combine(_frontend.Folder, "cargo-target-host"),
            new[] { "test", "--offline" }, fonts, TestContext.Current.CancellationToken);

        //Assert
        run.ExitCode.Should().Be(0, run.Output + run.Errors);
        run.Output.Should().Contain("test result: ok.");
        run.Output.Should().NotContain("FAILED");
        run.Output.Should().Contain("text::tests::the_three_missing_character_modes_draw_what_the_owner_ruled ... ok", "the text tests run with the test project's fonts");
        run.Output.Should().Contain("text::layout_tests::missing_characters_in_a_block_follow_the_three_modes ... ok", "the text layout tests run with the test project's fonts");
        run.Output.Should().Contain("paths::tests_combine::union_intersect_subtract_and_xor_of_two_overlapping_squares ... ok", "the path combining tests run");
        run.Output.Should().Contain("svg::tests_draw::svg_text_is_laid_out_into_paths_with_a_supplied_font_and_reported_without_one ... ok", "the SVG tests run, SVG text with the test project's fonts");
        (run.Output + run.Errors).Should().NotContain("SKIPPED: no test font", "the test fonts are beside the test assembly");
        run.Output.Should().Contain("lottie::tests_draw::the_invented_lotties_give_the_trial_counts ... ok", "the Lottie tests run with the sample's trial files");
        run.Output.Should().Contain("lottie::tests_element::image_layers_draw_the_pixels_the_application_hands_over_and_nothing_without_them ... ok", "the Lottie drawing tests run");
        (run.Output + run.Errors).Should().NotContain("SKIPPED: no Lottie trial file", "the Lottie trial files are beside the test assembly");
        run.Errors.Should().NotContain("warning: `codebrix-webgpu-animation`", "the Animation crate's tests compile without a warning");
    }

    [Fact]
    public void with_both_packages_registered_MaterializeAsync_reports_the_three_bundles()
        => _frontend.Both.CrateBundles.Select(b => b.Name).Should().BeEquivalentTo(new[] { WebGpuCrates.BundleName, AnimationCrates.BundleName, GraphicsCrates.BundleName });

    [Fact]
    public void with_both_packages_registered_the_vendor_folder_holds_one_folder_per_crate_of_the_four_archives()
    {
        //Arrange
        var expected = CrateFolders.InArchive(PlatformCrates.GetArchivePath())
            .Union(CrateFolders.InArchive(WebGpuCrates.GetArchivePath()), StringComparer.Ordinal)
            .Union(CrateFolders.InArchive(AnimationCrates.GetArchivePath()), StringComparer.Ordinal)
            .Union(CrateFolders.InArchive(GraphicsCrates.GetArchivePath()), StringComparer.Ordinal)
            .OrderBy(c => c, StringComparer.Ordinal).ToList();

        //Act
        var folders = CrateFolders.InFolder(BothVendor);

        //Assert
        folders.Select(f => f.Crate).OrderBy(c => c, StringComparer.Ordinal).Should().Equal(expected);
    }

    [Fact]
    public async Task with_both_packages_registered_cargo_check_compiles_a_frontend_that_names_both_crates_for_wasm32_offline()
    {
        //Arrange
        Assert.SkipUnless(Cargo.HasWasmTarget, $"cargo or the {Cargo.WasmTarget} target is not installed: this test runs `cargo check --offline --target {Cargo.WasmTarget}` on the materialized frontend.");

        //Act
        var run = await Cargo.RunAsync(_frontend.BothFrontendDirectory, Path.Combine(_frontend.Folder, "cargo-target"),
            new[] { "check", "--offline", "--target", Cargo.WasmTarget }, TestContext.Current.CancellationToken);

        //Assert
        run.ExitCode.Should().Be(0, run.Errors);
        run.Errors.Should().Contain("codebrix-webgpu-animation v" + _frontend.AnimationCrateVersion);
        run.Errors.Should().Contain("codebrix-webgpu-graphics v" + _frontend.GraphicsCrateVersion);
        run.Errors.Should().Contain("webgpu-both-test-frontend v0.1.0");
        run.Errors.Should().NotContain("src/main.rs", "the frontend that uses both crates compiles without a warning");
    }

    private static List<string> RegistryPackages(string metadataJson)
    {
        using var metadata = JsonDocument.Parse(metadataJson);
        return metadata.RootElement.GetProperty("packages").EnumerateArray()
            .Where(p => p.GetProperty("source").ValueKind == JsonValueKind.String)
            .Select(p => p.GetProperty("name").GetString() + " " + p.GetProperty("version").GetString())
            .ToList();
    }
}
