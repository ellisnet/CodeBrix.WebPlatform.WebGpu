using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using System.Security.Cryptography;
using System.Text;

namespace CodeBrix.WebPlatform.WebGpu.LibraryChecker.Tests.Helpers;

/// <summary>
/// A throw-away folder under the system temporary folder, removed when disposed.
/// </summary>
internal sealed class TempFolder : IDisposable
{
    public TempFolder()
    {
        Path = System.IO.Path.Combine(System.IO.Path.GetTempPath(), "webgpu-gate-tests-" + Guid.NewGuid().ToString("N"));
        Directory.CreateDirectory(Path);
    }

    public string Path { get; }

    public string Full(string relativePath) => System.IO.Path.Combine(Path, relativePath.Replace('/', System.IO.Path.DirectorySeparatorChar));

    public string Write(string relativePath, string contents)
    {
        var full = Full(relativePath);
        Directory.CreateDirectory(System.IO.Path.GetDirectoryName(full));
        File.WriteAllText(full, contents);
        return full;
    }

    /// <summary>
    /// Writes a crate folder the way <c>cargo vendor --versioned-dirs</c> does: a Cargo.toml, a
    /// source file and a .cargo-checksum.json that records both files and a package checksum.
    /// </summary>
    public string WriteCrate(string vendorRelative, string name, string version, string source = "pub fn f() {}", string packageChecksum = null)
    {
        var folder = $"{vendorRelative}/{name}-{version}";
        var files = new Dictionary<string, string>(StringComparer.Ordinal)
        {
            ["Cargo.toml"] = $"[package]\nedition = \"2021\"\nname = \"{name}\"\nversion = \"{version}\"\nlicense = \"MIT\"\n\n[dependencies]\n",
            ["src/lib.rs"] = source,
        };
        foreach (var (file, text) in files)
        {
            Write($"{folder}/{file}", text);
        }

        var recorded = string.Join(",", files.OrderBy(f => f.Key, StringComparer.Ordinal).Select(f => $"\"{f.Key}\":\"{Sha256(f.Value)}\""));
        Write($"{folder}/.cargo-checksum.json", $"{{\"files\":{{{recorded}}},\"package\":\"{packageChecksum ?? Sha256(name + version)}\"}}");
        return Full(folder);
    }

    public void Dispose()
    {
        try
        {
            Directory.Delete(Path, recursive: true);
        }
        catch (IOException)
        {
            // Best effort.
        }
    }

    public static string Sha256(string text) => Convert.ToHexStringLower(SHA256.HashData(Encoding.UTF8.GetBytes(text)));
}
