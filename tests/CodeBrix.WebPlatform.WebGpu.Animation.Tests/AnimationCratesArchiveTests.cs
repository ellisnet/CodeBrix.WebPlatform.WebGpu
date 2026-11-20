using System;
using System.Collections.Generic;
using System.IO;
using System.IO.Compression;
using System.Linq;
using System.Security.Cryptography;
using System.Text.Json;
using System.Text.RegularExpressions;
using SilverAssertions;
using Xunit;

namespace CodeBrix.WebPlatform.WebGpu.Animation.Tests;

/// <summary>
/// The crate archive the Animation project writes (webgpu-animation-crates.zip beside the test
/// assembly): the first-party crate codebrix-webgpu-animation in vendored form, the third-party
/// crates only this package carries byte for byte as they are vendored, and nothing else.
/// </summary>
public class AnimationCratesArchiveTests
{
    private static readonly string CrateManifest = Path.Combine(AppContext.BaseDirectory, "codebrix-webgpu-animation", "Cargo.toml");

    private static string CrateFolder
    {
        get
        {
            var version = Regex.Match(File.ReadAllText(CrateManifest), @"(?m)^version\s*=\s*""([^""]+)""").Groups[1].Value;
            return "codebrix-webgpu-animation-" + version;
        }
    }

    private static Dictionary<string, ZipArchiveEntry> Files(ZipArchive archive, string folder)
        => archive.Entries.Where(e => e.FullName.StartsWith(folder + "/", StringComparison.Ordinal))
            .ToDictionary(e => e.FullName.Substring(folder.Length + 1), StringComparer.Ordinal);

    private static byte[] Read(ZipArchiveEntry entry)
    {
        using var stream = entry.Open();
        using var memory = new MemoryStream();
        stream.CopyTo(memory);
        return memory.ToArray();
    }

    private static bool Same(byte[] first, byte[] second) => first.AsSpan().SequenceEqual(second);

    private static List<string> Tops(ZipArchive archive)
        => archive.Entries.Select(e => e.FullName.Replace('\\', '/')).Select(e => e.Contains('/') ? e.Substring(0, e.IndexOf('/')) : e).Distinct().OrderBy(e => e, StringComparer.Ordinal).ToList();

    [Fact]
    public void the_archive_holds_the_codebrix_webgpu_animation_folder_and_the_kurbo_and_peniko_crates()
    {
        //Arrange
        using var archive = ZipFile.OpenRead(AnimationCrates.GetArchivePath());

        //Act
        var tops = Tops(archive);

        //Assert
        tops.Should().Contain(new[] { CrateFolder, "kurbo-0.13.1", "peniko-0.6.1" });
    }

    [Fact]
    public void the_archive_holds_exactly_the_vendor_folder_and_the_crate()
    {
        //Arrange
        var vendor = RepositorySource.AnimationVendorFolder();
        Assert.SkipWhen(vendor == null, "The repository's source is not beside this test's source file.");
        var expected = Directory.GetDirectories(vendor).Select(Path.GetFileName).Append(CrateFolder).OrderBy(f => f, StringComparer.Ordinal).ToList();
        using var archive = ZipFile.OpenRead(AnimationCrates.GetArchivePath());

        //Act
        var tops = Tops(archive);

        //Assert
        tops.Should().Equal(expected);
    }

    [Fact]
    public void the_vendored_crates_are_in_the_archive_byte_for_byte()
    {
        //Arrange
        var vendor = RepositorySource.AnimationVendorFolder();
        Assert.SkipWhen(vendor == null, "The repository's source is not beside this test's source file.");
        using var archive = ZipFile.OpenRead(AnimationCrates.GetArchivePath());
        var entries = archive.Entries.Where(e => !e.FullName.StartsWith(CrateFolder + "/", StringComparison.Ordinal)).ToDictionary(e => e.FullName, StringComparer.Ordinal);

        //Act
        var files = Directory.GetFiles(vendor, "*", SearchOption.AllDirectories).Select(f => Path.GetRelativePath(vendor, f).Replace('\\', '/')).OrderBy(f => f, StringComparer.Ordinal).ToList();

        //Assert
        entries.Keys.OrderBy(k => k, StringComparer.Ordinal).Should().Equal(files);
        files.Should().OnlyContain(f => Same(Read(entries[f]), File.ReadAllBytes(Path.Combine(vendor, f))));
    }

    [Fact]
    public void the_archive_carries_codebrix_webgpu_animation_in_vendored_form()
    {
        //Arrange
        using var archive = ZipFile.OpenRead(AnimationCrates.GetArchivePath());

        //Act
        var files = Files(archive, CrateFolder);

        //Assert
        files.Keys.Should().Contain(new[] { "Cargo.toml", "src/lib.rs", "src/rendering/core/composite.wgsl", ".cargo-checksum.json" });
    }

    [Fact]
    public void the_checksum_file_lists_every_other_file_with_its_sha256_and_no_package_checksum()
    {
        //Arrange
        using var archive = ZipFile.OpenRead(AnimationCrates.GetArchivePath());
        var files = Files(archive, CrateFolder);
        using var checksum = JsonDocument.Parse(Read(files[".cargo-checksum.json"]));

        //Act
        var listed = checksum.RootElement.GetProperty("files").EnumerateObject().ToDictionary(p => p.Name, p => p.Value.GetString(), StringComparer.Ordinal);

        //Assert
        checksum.RootElement.GetProperty("package").ValueKind.Should().Be(JsonValueKind.Null);
        listed.Keys.OrderBy(k => k, StringComparer.Ordinal).Should().Equal(files.Keys.Where(k => k != ".cargo-checksum.json").OrderBy(k => k, StringComparer.Ordinal));
        listed.Should().OnlyContain(p => p.Value == Convert.ToHexString(SHA256.HashData(Read(files[p.Key]))).ToLowerInvariant());
    }

    [Fact]
    public void the_crate_in_the_archive_is_its_source_without_build_output_cargo_folder_or_lock_file()
    {
        //Arrange
        var source = RepositorySource.AnimationCrateFolder();
        Assert.SkipWhen(source == null, "The repository's source is not beside this test's source file.");
        var expected = Directory.GetFiles(source, "*", SearchOption.AllDirectories)
            .Select(f => Path.GetRelativePath(source, f).Replace('\\', '/'))
            .Where(f => !f.StartsWith("target/", StringComparison.Ordinal) && !f.StartsWith(".cargo/", StringComparison.Ordinal) && f != "Cargo.lock")
            .OrderBy(f => f, StringComparer.Ordinal).ToList();
        using var archive = ZipFile.OpenRead(AnimationCrates.GetArchivePath());

        //Act
        var files = Files(archive, CrateFolder);

        //Assert
        files.Keys.Where(k => k != ".cargo-checksum.json").OrderBy(k => k, StringComparer.Ordinal).Should().Equal(expected);
        expected.Should().OnlyContain(f => Same(Read(files[f]), File.ReadAllBytes(Path.Combine(source, f))));
    }

    [Fact]
    public void the_archive_is_written_in_path_order_with_one_time_stamp()
    {
        //Arrange
        using var archive = ZipFile.OpenRead(AnimationCrates.GetArchivePath());

        //Act
        var names = archive.Entries.Select(e => e.FullName).ToList();
        var stamps = archive.Entries.Select(e => e.LastWriteTime.DateTime).Distinct().ToList();

        //Assert
        names.Should().Equal(names.OrderBy(n => n, StringComparer.Ordinal));
        names.Should().NotContain(n => n.EndsWith('/'));
        stamps.Should().Equal(new DateTime(1980, 1, 1, 0, 0, 0));
    }

    [Fact]
    public void the_archive_carries_none_of_the_webgpu_archives_crates()
    {
        //Arrange
        using var webGpu = ZipFile.OpenRead(WebGpuCrates.GetArchivePath());
        var webGpuNames = webGpu.Entries.Where(e => e.FullName.Count(c => c == '/') == 1 && e.FullName.EndsWith("/Cargo.toml", StringComparison.Ordinal))
            .Select(e => e.FullName.Split('/')[0]).Select(f => f.Substring(0, f.LastIndexOf('-'))).ToHashSet(StringComparer.Ordinal);
        using var archive = ZipFile.OpenRead(AnimationCrates.GetArchivePath());

        //Act
        var names = Tops(archive).Select(f => f.Substring(0, f.LastIndexOf('-'))).ToList();

        //Assert
        names.Should().NotContain(n => webGpuNames.Contains(n), "the WebGpu package's crates (wgpu, glam, codebrix-webgpu, arrayvec, ...) come from its bundle, in no other version");
    }
}
