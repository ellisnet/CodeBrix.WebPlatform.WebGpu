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

namespace CodeBrix.WebPlatform.WebGpu.Graphics.Tests;

/// <summary>
/// The crate archive the Graphics project writes (webgpu-graphics-crates.zip beside the test
/// assembly): the first-party crate codebrix-webgpu-graphics in vendored form, and nothing else.
/// </summary>
public class GraphicsCratesArchiveTests
{
    private static readonly string CrateManifest = Path.Combine(AppContext.BaseDirectory, "codebrix-webgpu-graphics", "Cargo.toml");

    private static string CrateFolder
    {
        get
        {
            var version = Regex.Match(File.ReadAllText(CrateManifest), @"(?m)^version\s*=\s*""([^""]+)""").Groups[1].Value;
            return "codebrix-webgpu-graphics-" + version;
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

    [Fact]
    public void the_archive_holds_only_the_codebrix_webgpu_graphics_folder()
    {
        //Arrange
        using var archive = ZipFile.OpenRead(GraphicsCrates.GetArchivePath());

        //Act
        var tops = archive.Entries.Select(e => e.FullName.Replace('\\', '/')).Select(e => e.Contains('/') ? e.Substring(0, e.IndexOf('/')) : e).Distinct().ToList();

        //Assert
        tops.Should().Equal(CrateFolder);
    }

    [Fact]
    public void the_archive_carries_codebrix_webgpu_graphics_in_vendored_form()
    {
        //Arrange
        using var archive = ZipFile.OpenRead(GraphicsCrates.GetArchivePath());

        //Act
        var files = Files(archive, CrateFolder);

        //Assert
        files.Keys.Should().Contain(new[] { "Cargo.toml", "src/lib.rs", ".cargo-checksum.json" });
    }

    [Fact]
    public void the_checksum_file_lists_every_other_file_with_its_sha256_and_no_package_checksum()
    {
        //Arrange
        using var archive = ZipFile.OpenRead(GraphicsCrates.GetArchivePath());
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
        var source = RepositorySource.GraphicsCrateFolder();
        Assert.SkipWhen(source == null, "The repository's source is not beside this test's source file.");
        var expected = Directory.GetFiles(source, "*", SearchOption.AllDirectories)
            .Select(f => Path.GetRelativePath(source, f).Replace('\\', '/'))
            .Where(f => !f.StartsWith("target/", StringComparison.Ordinal) && !f.StartsWith(".cargo/", StringComparison.Ordinal) && f != "Cargo.lock")
            .OrderBy(f => f, StringComparer.Ordinal).ToList();
        using var archive = ZipFile.OpenRead(GraphicsCrates.GetArchivePath());

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
        using var archive = ZipFile.OpenRead(GraphicsCrates.GetArchivePath());

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
        var webGpuFolders = webGpu.Entries.Select(e => e.FullName.Split('/')[0]).ToHashSet(StringComparer.Ordinal);
        using var archive = ZipFile.OpenRead(GraphicsCrates.GetArchivePath());

        //Act
        var folders = archive.Entries.Select(e => e.FullName.Split('/')[0]).Distinct().ToList();

        //Assert
        folders.Should().NotContain(f => webGpuFolders.Contains(f));
    }
}
