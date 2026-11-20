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

namespace CodeBrix.WebPlatform.WebGpu.Tests;

/// <summary>
/// The crate archive the WebGpu project writes (webgpu-crates.zip beside the test assembly): the
/// vendored crates byte for byte, plus the first-party crate codebrix-webgpu in vendored form.
/// </summary>
public class WebGpuCratesArchiveTests
{
    private static readonly string FirstPartyManifest = Path.Combine(AppContext.BaseDirectory, "codebrix-webgpu", "Cargo.toml");

    private static string FirstPartyFolder
    {
        get
        {
            var version = Regex.Match(File.ReadAllText(FirstPartyManifest), @"(?m)^version\s*=\s*""([^""]+)""").Groups[1].Value;
            return "codebrix-webgpu-" + version;
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
    public void the_archive_carries_codebrix_webgpu_in_vendored_form()
    {
        //Arrange
        using var archive = ZipFile.OpenRead(WebGpuCrates.GetArchivePath());

        //Act
        var files = Files(archive, FirstPartyFolder);

        //Assert
        files.Keys.Should().Contain(new[] { "Cargo.toml", "src/lib.rs", ".cargo-checksum.json" });
    }

    [Fact]
    public void the_checksum_file_of_codebrix_webgpu_lists_every_other_file_with_its_sha256_and_no_package_checksum()
    {
        //Arrange
        using var archive = ZipFile.OpenRead(WebGpuCrates.GetArchivePath());
        var files = Files(archive, FirstPartyFolder);
        using var checksum = JsonDocument.Parse(Read(files[".cargo-checksum.json"]));

        //Act
        var listed = checksum.RootElement.GetProperty("files").EnumerateObject().ToDictionary(p => p.Name, p => p.Value.GetString(), StringComparer.Ordinal);

        //Assert
        checksum.RootElement.GetProperty("package").ValueKind.Should().Be(JsonValueKind.Null);
        listed.Keys.OrderBy(k => k, StringComparer.Ordinal).Should().Equal(files.Keys.Where(k => k != ".cargo-checksum.json").OrderBy(k => k, StringComparer.Ordinal));
        listed.Should().OnlyContain(p => p.Value == Convert.ToHexString(SHA256.HashData(Read(files[p.Key]))).ToLowerInvariant());
    }

    [Fact]
    public void codebrix_webgpu_in_the_archive_is_its_source_without_build_output_or_lock_file()
    {
        //Arrange
        var rust = RepositorySource.RustFolder();
        Assert.SkipWhen(rust == null, "The repository's source is not beside this test's source file.");
        var source = Path.Combine(rust, "codebrix-webgpu");
        var expected = Directory.GetFiles(source, "*", SearchOption.AllDirectories)
            .Select(f => Path.GetRelativePath(source, f).Replace('\\', '/'))
            .Where(f => !f.StartsWith("target/", StringComparison.Ordinal) && !f.StartsWith(".cargo/", StringComparison.Ordinal) && f != "Cargo.lock")
            .OrderBy(f => f, StringComparer.Ordinal).ToList();
        using var archive = ZipFile.OpenRead(WebGpuCrates.GetArchivePath());

        //Act
        var files = Files(archive, FirstPartyFolder);

        //Assert
        files.Keys.Where(k => k != ".cargo-checksum.json").OrderBy(k => k, StringComparer.Ordinal).Should().Equal(expected);
        expected.Should().OnlyContain(f => Same(Read(files[f]), File.ReadAllBytes(Path.Combine(source, f))));
    }

    [Fact]
    public void the_archive_is_the_vendor_folder_byte_for_byte_and_codebrix_webgpu_and_nothing_else()
    {
        //Arrange
        var rust = RepositorySource.RustFolder();
        Assert.SkipWhen(rust == null, "The repository's source is not beside this test's source file.");
        var vendor = Path.Combine(rust, "vendor");
        Assert.SkipWhen(!Directory.Exists(vendor), "src/CodeBrix.WebPlatform.WebGpu/Rust/vendor is missing.");
        var vendored = Directory.GetFiles(vendor, "*", SearchOption.AllDirectories)
            .Select(f => Path.GetRelativePath(vendor, f).Replace('\\', '/'))
            .ToHashSet(StringComparer.Ordinal);
        using var archive = ZipFile.OpenRead(WebGpuCrates.GetArchivePath());

        //Act
        var entries = archive.Entries.ToDictionary(e => e.FullName, StringComparer.Ordinal);
        var others = entries.Keys.Where(k => !vendored.Contains(k)).ToList();

        //Assert
        vendored.Should().OnlyContain(f => entries.ContainsKey(f));
        others.Should().OnlyContain(k => k.StartsWith(FirstPartyFolder + "/", StringComparison.Ordinal));
        vendored.Should().OnlyContain(f => entries[f].Length == new FileInfo(Path.Combine(vendor, f)).Length
            && Same(Read(entries[f]), File.ReadAllBytes(Path.Combine(vendor, f))));
    }

    [Fact]
    public void the_archive_is_written_in_path_order_with_one_time_stamp()
    {
        //Arrange
        using var archive = ZipFile.OpenRead(WebGpuCrates.GetArchivePath());

        //Act
        var names = archive.Entries.Select(e => e.FullName).ToList();
        var stamps = archive.Entries.Select(e => e.LastWriteTime.DateTime).Distinct().ToList();

        //Assert
        names.Should().Equal(names.OrderBy(n => n, StringComparer.Ordinal));
        names.Should().NotContain(n => n.EndsWith('/'));
        stamps.Should().ContainSingle();
    }
}
