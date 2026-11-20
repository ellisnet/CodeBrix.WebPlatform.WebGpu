using System;
using System.Collections.Generic;
using System.IO;
using System.IO.Compression;
using System.Linq;
using System.Text.RegularExpressions;

namespace CodeBrix.WebPlatform.WebGpu.Graphics.Tests.Materialize;

/// <summary>
/// Reads which crates an archive or a vendor folder holds, as "name version" (the form
/// MaterializeResult.CrateBundles uses), from each crate folder's Cargo.toml.
/// </summary>
internal static class CrateFolders
{
    private static readonly Regex Key = new Regex(@"^\s*(name|version)\s*=\s*""([^""]*)""", RegexOptions.CultureInvariant);

    /// <summary>The crates at the top of a crates archive, sorted ordinally.</summary>
    public static IReadOnlyList<string> InArchive(string archivePath)
    {
        using var archive = ZipFile.OpenRead(archivePath);
        var crates = new List<string>();
        foreach (var entry in archive.Entries)
        {
            var name = entry.FullName.Replace('\\', '/');
            var parts = name.Split('/');
            if (parts.Length != 2 || parts[1] != "Cargo.toml")
            {
                continue;
            }

            using var reader = new StreamReader(entry.Open());
            crates.Add(ReadPackage(reader.ReadToEnd()) ?? throw new InvalidOperationException($"'{name}' in '{archivePath}' has no package name and version."));
        }

        return crates.OrderBy(c => c, StringComparer.Ordinal).ToList();
    }

    /// <summary>Every crate folder of a vendor folder with the crate its Cargo.toml names (null when it names none).</summary>
    public static IReadOnlyList<(string Folder, string Crate)> InFolder(string vendorDirectory)
        => Directory.GetDirectories(vendorDirectory)
            .Select(d => (Path.GetFileName(d), File.Exists(Path.Combine(d, "Cargo.toml")) ? ReadPackage(File.ReadAllText(Path.Combine(d, "Cargo.toml"))) : null))
            .OrderBy(c => c.Item1, StringComparer.Ordinal)
            .ToList();

    /// <summary>"name version" of the [package] table of a Cargo.toml, or null.</summary>
    public static string ReadPackage(string manifest)
    {
        string table = null;
        string name = null;
        string version = null;
        foreach (var raw in manifest.Split('\n'))
        {
            var line = raw.Trim();
            if (line.StartsWith('['))
            {
                table = line.Trim('[', ']').Trim();
                continue;
            }

            if (table != "package")
            {
                continue;
            }

            var match = Key.Match(line);
            if (match.Success && match.Groups[1].Value == "name")
            {
                name ??= match.Groups[2].Value;
            }
            else if (match.Success)
            {
                version ??= match.Groups[2].Value;
            }
        }

        return name == null || version == null ? null : name + " " + version;
    }
}
