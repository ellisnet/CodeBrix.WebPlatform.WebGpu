using System;
using System.IO;
using System.Linq;
using System.Text.Json;
using System.Text.RegularExpressions;
using System.Threading.Tasks;
using CodeBrix.WebPlatform.Core.Crates;
using CodeBrix.WebPlatform.FrontendBuilder.Crates;
using SilverAssertions;
using Xunit;

namespace CodeBrix.WebPlatform.WebGpu.Tests.Materialize;

/// <summary>
/// The platform's real MaterializeAsync with this package's bundle registered, against the real
/// platform archive (from the FrontendBuilder package) and the real WebGpu archive. The
/// Materialize itself happens once, in <see cref="MaterializedFrontend"/>; these tests read what
/// it wrote and reported. The cargo tests are skipped on a machine without cargo or without the
/// wasm32 target; none needs the network, dx, a browser or a container.
/// </summary>
[Collection(MaterializeCollection.Name)]
public class WebGpuCratesMaterializeTests
{
    private readonly MaterializedFrontend _frontend;

    public WebGpuCratesMaterializeTests(MaterializedFrontend frontend)
    {
        _frontend = frontend;
        Assert.SkipWhen(frontend.NoCratesReason != null, frontend.NoCratesReason ?? string.Empty);
    }

    private string Vendor => _frontend.Options.ResolvedVendorDirectory;

    private CrateBundleOutcome WebGpuOutcome => _frontend.First.CrateBundles.Single(b => b.Name == WebGpuCrates.BundleName);

    [Fact]
    public void MaterializeAsync_reports_the_webgpu_bundle_with_the_packages_name_and_version()
        => WebGpuOutcome.Origin.Should().Be(WebGpuCrates.PackageId + " " + WebGpuCrates.PackageVersion);

    [Fact]
    public void MaterializeAsync_reports_every_registered_bundle_and_no_other()
        => _frontend.First.CrateBundles.Select(b => b.Name).Should().BeEquivalentTo(CrateBundles.Registered.Select(b => b.Name));

    [Fact]
    public void the_added_and_the_shared_crates_partition_the_crates_of_the_webgpu_archive()
    {
        //Arrange
        var archive = CrateFolders.InArchive(WebGpuCrates.GetArchivePath());

        //Act
        var added = WebGpuOutcome.AddedCrates;
        var shared = WebGpuOutcome.SharedCrates;

        //Assert
        added.Intersect(shared, StringComparer.Ordinal).Should().BeEmpty();
        added.Concat(shared).Should().OnlyHaveUniqueItems();
        added.Concat(shared).OrderBy(c => c, StringComparer.Ordinal).Should().Equal(archive);
    }

    [Fact]
    public void wgpu_wgpu_types_and_naga_types_are_among_the_added_crates()
    {
        //Act
        var added = WebGpuOutcome.AddedCrates;

        //Assert
        added.Should().Contain("wgpu " + _frontend.WgpuVersion);
        added.Should().Contain(c => c.StartsWith("wgpu-types ", StringComparison.Ordinal));
        added.Should().Contain(c => c.StartsWith("naga-types ", StringComparison.Ordinal));
    }

    [Fact]
    public void every_shared_crate_is_one_the_platforms_archive_holds_and_no_added_crate_is_a_platform_crate()
    {
        //Arrange
        var platform = CrateFolders.InArchive(PlatformCrates.GetArchivePath());
        var platformNames = platform.Select(c => c.Substring(0, c.IndexOf(' '))).ToHashSet(StringComparer.Ordinal);

        //Act
        var shared = WebGpuOutcome.SharedCrates;
        var added = WebGpuOutcome.AddedCrates;

        //Assert
        platform.Should().NotBeEmpty();
        shared.Should().OnlyContain(c => platform.Contains(c));
        added.Should().NotContain(c => platformNames.Contains(c.Substring(0, c.IndexOf(' '))),
            "the strict rule lets a bundle add only crates whose name the platform does not carry in any version");
    }

    [Fact]
    public void the_vendor_folder_holds_the_wgpu_folder_with_its_checksum_file()
    {
        //Arrange
        var wgpu = Path.Combine(Vendor, "wgpu-" + _frontend.WgpuVersion);

        //Assert
        File.Exists(Path.Combine(wgpu, "Cargo.toml")).Should().BeTrue();
        File.Exists(Path.Combine(wgpu, ".cargo-checksum.json")).Should().BeTrue();
        File.Exists(Path.Combine(wgpu, "src", "lib.rs")).Should().BeTrue();
    }

    [Fact]
    public void the_vendor_folder_holds_exactly_one_folder_per_crate_name_and_version_of_both_archives()
    {
        //Arrange
        var expected = CrateFolders.InArchive(PlatformCrates.GetArchivePath()).Union(CrateFolders.InArchive(WebGpuCrates.GetArchivePath()), StringComparer.Ordinal)
            .OrderBy(c => c, StringComparer.Ordinal).ToList();

        //Act
        var folders = CrateFolders.InFolder(Vendor);

        //Assert
        folders.Should().OnlyContain(f => f.Crate != null && f.Folder == f.Crate.Replace(' ', '-'));
        folders.Should().OnlyContain(f => File.Exists(Path.Combine(Vendor, f.Folder, ".cargo-checksum.json")));
        folders.Select(f => f.Crate).Should().OnlyHaveUniqueItems();
        folders.Select(f => f.Crate).OrderBy(c => c, StringComparer.Ordinal).Should().Equal(expected);
    }

    [Fact]
    public void the_cargo_configuration_points_at_the_vendor_folder_and_keeps_cargo_offline()
    {
        //Arrange
        var configuration = File.ReadAllText(_frontend.Options.CargoConfigurationFile);

        //Act
        var source = Regex.Match(configuration, @"replace-with\s*=\s*""([^""]+)""").Groups[1].Value;
        var directory = Regex.Match(configuration, @"\[source\." + Regex.Escape(source) + @"\]\s*\ndirectory\s*=\s*""([^""]+)""").Groups[1].Value;

        //Assert
        configuration.Should().Contain("[source.crates-io]");
        source.Should().NotBeEmpty();
        directory.Should().NotBeEmpty();
        Path.GetFullPath(Path.Combine(_frontend.FrontendDirectory, directory)).Should().Be(Vendor);
        configuration.Should().MatchRegex(@"\[net\]\s*\noffline\s*=\s*true");
    }

    [Fact]
    public void Register_again_changes_nothing()
    {
        //Arrange
        var before = CrateBundles.Registered.Single(b => b.Name == WebGpuCrates.BundleName);

        //Act
        WebGpuCrates.Register();
        WebGpuCrates.Register();

        //Assert
        CrateBundles.Registered.Single(b => b.Name == WebGpuCrates.BundleName).Should().BeSameAs(before);
    }

    [Fact]
    public async Task a_second_MaterializeAsync_of_an_unchanged_set_does_not_rewrite_the_vendor_folder()
    {
        //Arrange
        var stamp = Path.Combine(Vendor, PlatformCrates.StampFileName);
        var stampText = File.ReadAllText(stamp);
        var stampWritten = File.GetLastWriteTimeUtc(stamp);
        var vendorWritten = Directory.GetLastWriteTimeUtc(Vendor);
        WebGpuCrates.Register();

        //Act
        var second = await _frontend.Frontend.MaterializeAsync(TestContext.Current.CancellationToken);

        //Assert
        second.VendorChanged.Should().BeFalse();
        second.WasUpToDate.Should().BeTrue();
        File.ReadAllText(stamp).Should().Be(stampText);
        stampText.Should().StartWith("bundles-");
        File.GetLastWriteTimeUtc(stamp).Should().Be(stampWritten);
        Directory.GetLastWriteTimeUtc(Vendor).Should().Be(vendorWritten);
        var outcome = second.CrateBundles.Single(b => b.Name == WebGpuCrates.BundleName);
        outcome.Origin.Should().Be(WebGpuOutcome.Origin);
        outcome.AddedCrates.Should().Equal(WebGpuOutcome.AddedCrates);
        outcome.SharedCrates.Should().Equal(WebGpuOutcome.SharedCrates);
    }

    [Fact]
    public async Task cargo_metadata_resolves_the_materialized_frontend_offline_from_the_vendor_folder()
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
        registryPackages.Should().Contain("wgpu " + _frontend.WgpuVersion);
        registryPackages.Should().Contain("codebrix-webgpu " + _frontend.FirstPartyCrateVersion);
        registryPackages.Should().OnlyContain(p => vendored.Contains(p), "cargo takes every crate from the vendor folder");
        var glam = metadata.RootElement.GetProperty("resolve").GetProperty("nodes").EnumerateArray()
            .Single(n => n.GetProperty("id").GetString().Contains("#glam@", StringComparison.Ordinal));
        glam.GetProperty("deps").EnumerateArray().Select(d => d.GetProperty("name").GetString()).Should().Equal(new[] { "bytemuck" },
            "glam is resolved with bytemuck only: none of its other optional dependencies is needed (or vendored)");
    }

    [Fact]
    public void codebrix_webgpu_is_among_the_added_crates_with_its_checksum_file_in_the_vendor_folder()
    {
        //Arrange
        var folder = Path.Combine(Vendor, "codebrix-webgpu-" + _frontend.FirstPartyCrateVersion);

        //Act
        var added = WebGpuOutcome.AddedCrates;

        //Assert
        added.Should().Contain("codebrix-webgpu " + _frontend.FirstPartyCrateVersion);
        File.Exists(Path.Combine(folder, "Cargo.toml")).Should().BeTrue();
        File.Exists(Path.Combine(folder, ".cargo-checksum.json")).Should().BeTrue();
        File.Exists(Path.Combine(folder, "src", "lib.rs")).Should().BeTrue();
    }

    [Fact]
    public async Task cargo_check_compiles_the_frontend_that_uses_codebrix_webgpu_for_wasm32_offline()
    {
        //Arrange
        Assert.SkipUnless(Cargo.HasWasmTarget, $"cargo or the {Cargo.WasmTarget} target is not installed: this test runs `cargo check --offline --target {Cargo.WasmTarget}` on the materialized frontend.");

        //Act
        var run = await Cargo.RunAsync(_frontend.FrontendDirectory, Path.Combine(_frontend.Folder, "cargo-target"),
            new[] { "check", "--offline", "--target", Cargo.WasmTarget }, TestContext.Current.CancellationToken);

        //Assert
        run.ExitCode.Should().Be(0, run.Errors);
        run.Errors.Should().Contain("wgpu v" + _frontend.WgpuVersion);
        run.Errors.Should().Contain("codebrix-webgpu v" + _frontend.FirstPartyCrateVersion);
        run.Errors.Should().Contain("webgpu-test-frontend v0.1.0");
        run.Errors.Should().NotContain("warning: `codebrix-webgpu`", "the first-party crate compiles without a warning");
        run.Errors.Should().NotContain("src/main.rs", "the frontend that uses the crate's API compiles without a warning");
    }

    [Fact]
    public async Task cargo_test_runs_the_unit_tests_of_codebrix_webgpu_offline_from_the_vendor_folder()
    {
        //Arrange
        Assert.SkipUnless(Cargo.IsAvailable, "cargo is not installed (not on PATH): this test runs `cargo test --offline` on a copy of the codebrix-webgpu crate from the materialized vendor folder.");
        var source = Path.Combine(Vendor, "codebrix-webgpu-" + _frontend.FirstPartyCrateVersion);
        var crate = Path.Combine(_frontend.Folder, "codebrix-webgpu-tests");
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
        run.Errors.Should().NotContain("warning: `codebrix-webgpu`", "the first-party crate's tests compile without a warning");
    }
}
