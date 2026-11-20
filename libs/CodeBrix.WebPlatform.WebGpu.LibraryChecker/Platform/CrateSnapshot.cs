using System;
using System.Collections.Generic;
using System.IO;
using System.IO.Compression;
using System.Linq;
using System.Text;

namespace CodeBrix.WebPlatform.WebGpu.LibraryChecker.Platform;

/// <summary>
/// The crates of a vendored snapshot - a vendor folder on disk, or an archive of one such as the
/// platform's <c>platform-crates.zip</c> - read the way the platform's crate-bundle merge reads
/// them: one folder per crate at the top, each with its Cargo.toml (whose [package] table names
/// the crate and its version) and its <c>.cargo-checksum.json</c>.
/// </summary>
public sealed class CrateSnapshot
{
    /// <summary>The manifest file of a vendored crate.</summary>
    public const string ManifestFileName = "Cargo.toml";

    /// <summary>The checksum file <c>cargo vendor</c> writes into every crate folder.</summary>
    public const string ChecksumFileName = ".cargo-checksum.json";

    private CrateSnapshot(string source, IReadOnlyList<SnapshotCrate> crates)
    {
        Source = source;
        Crates = crates;
    }

    /// <summary>Where the snapshot was read from.</summary>
    public string Source { get; }

    /// <summary>The crates, sorted by folder name.</summary>
    public IReadOnlyList<SnapshotCrate> Crates { get; }

    /// <summary>
    /// The crates of a given name (a snapshot may hold a crate in more than one version).
    /// </summary>
    /// <param name="name">The crate's name.</param>
    public IReadOnlyList<SnapshotCrate> Find(string name)
        => Crates.Where(c => string.Equals(c.Name, name, StringComparison.Ordinal)).ToList();

    /// <summary>
    /// The crates of several snapshots together (for a check that accepts a crate any of them
    /// carries), sorted by folder name; a folder that two snapshots hold is listed once.
    /// </summary>
    /// <param name="snapshots">The snapshots.</param>
    public static CrateSnapshot Combine(params CrateSnapshot[] snapshots)
    {
        ArgumentNullException.ThrowIfNull(snapshots);
        var crates = snapshots.SelectMany(s => s.Crates)
            .GroupBy(c => c.Folder, StringComparer.Ordinal)
            .Select(g => g.First())
            .OrderBy(c => c.Folder, StringComparer.Ordinal)
            .ToList();
        return new CrateSnapshot(string.Join(" + ", snapshots.Select(s => s.Source)), crates);
    }

    /// <summary>
    /// Reads a vendor folder on disk. Folders whose names start with a dot are left out.
    /// </summary>
    /// <param name="directory">The vendor folder.</param>
    /// <exception cref="DirectoryNotFoundException">The folder does not exist.</exception>
    /// <exception cref="InvalidDataException">A folder is not a vendored crate.</exception>
    public static CrateSnapshot FromDirectory(string directory)
    {
        if (!Directory.Exists(directory))
        {
            throw new DirectoryNotFoundException($"The vendor folder '{directory}' does not exist.");
        }

        var crates = new List<SnapshotCrate>();
        foreach (var folder in Directory.EnumerateDirectories(directory).OrderBy(d => Path.GetFileName(d), StringComparer.Ordinal))
        {
            var folderName = Path.GetFileName(folder);
            if (folderName.StartsWith('.'))
            {
                continue;
            }

            var manifest = Path.Combine(folder, ManifestFileName);
            var checksum = Path.Combine(folder, ChecksumFileName);
            if (!File.Exists(manifest) || !File.Exists(checksum))
            {
                throw new InvalidDataException($"The folder '{folder}' is not a vendored crate: it needs a {ManifestFileName} and a {ChecksumFileName} (as `cargo vendor` writes them).");
            }

            var (name, version) = ReadPackage(File.ReadAllText(manifest, Encoding.UTF8));
            if (name == null || version == null)
            {
                throw new InvalidDataException($"The {ManifestFileName} of '{folder}' names no [package] name and version.");
            }

            crates.Add(new SnapshotCrate(folderName, name, version, File.ReadAllBytes(checksum)));
        }

        return new CrateSnapshot(directory, crates);
    }

    /// <summary>
    /// Reads an archive of a vendor folder (one folder per crate at the top of the archive).
    /// </summary>
    /// <param name="archivePath">The zip file.</param>
    /// <exception cref="FileNotFoundException">The archive does not exist.</exception>
    /// <exception cref="InvalidDataException">The archive is not a readable zip file, or a folder
    /// in it is not a vendored crate.</exception>
    public static CrateSnapshot FromArchive(string archivePath)
    {
        if (!File.Exists(archivePath))
        {
            throw new FileNotFoundException($"The crate archive '{archivePath}' does not exist.", archivePath);
        }

        using var archive = ZipFile.OpenRead(archivePath);
        var manifests = new Dictionary<string, ZipArchiveEntry>(StringComparer.Ordinal);
        var checksums = new Dictionary<string, ZipArchiveEntry>(StringComparer.Ordinal);
        var folders = new SortedSet<string>(StringComparer.Ordinal);
        foreach (var entry in archive.Entries)
        {
            var path = entry.FullName.Replace('\\', '/');
            var slash = path.IndexOf('/');
            if (slash <= 0)
            {
                if (path.Length > 0 && !path.StartsWith('.'))
                {
                    throw new InvalidDataException($"The archive '{archivePath}' has the file '{path}' outside any crate's folder. An archive of crates holds only one folder per crate.");
                }

                continue;
            }

            var folder = path.Substring(0, slash);
            if (folder.StartsWith('.'))
            {
                continue;
            }

            folders.Add(folder);
            var rest = path.Substring(slash + 1);
            if (rest == ManifestFileName)
            {
                manifests[folder] = entry;
            }
            else if (rest == ChecksumFileName)
            {
                checksums[folder] = entry;
            }
        }

        var crates = new List<SnapshotCrate>();
        foreach (var folder in folders)
        {
            if (!manifests.TryGetValue(folder, out var manifest) || !checksums.TryGetValue(folder, out var checksum))
            {
                throw new InvalidDataException($"The folder '{folder}' in the archive '{archivePath}' is not a vendored crate: it needs a {ManifestFileName} and a {ChecksumFileName} (as `cargo vendor` writes them).");
            }

            var (name, version) = ReadPackage(ReadText(manifest));
            if (name == null || version == null)
            {
                throw new InvalidDataException($"The {ManifestFileName} of '{folder}' in the archive '{archivePath}' names no [package] name and version.");
            }

            crates.Add(new SnapshotCrate(folder, name, version, ReadBytes(checksum)));
        }

        return new CrateSnapshot(archivePath, crates);
    }

    /// <summary>The name and version in the [package] table of a Cargo.toml as crates.io publishes it.</summary>
    /// <param name="manifest">The Cargo.toml text.</param>
    internal static (string Name, string Version) ReadPackage(string manifest)
    {
        string name = null;
        string version = null;
        var inPackage = false;
        foreach (var raw in manifest.Split('\n'))
        {
            var line = raw.Trim();
            if (line.StartsWith('['))
            {
                if (inPackage)
                {
                    break;
                }

                inPackage = line == "[package]";
                continue;
            }

            if (!inPackage)
            {
                continue;
            }

            name ??= ReadString(line, "name");
            version ??= ReadString(line, "version");
        }

        return (name, version);
    }

    private static string ReadString(string line, string key)
    {
        if (!line.StartsWith(key, StringComparison.Ordinal))
        {
            return null;
        }

        var rest = line.Substring(key.Length).TrimStart();
        if (!rest.StartsWith('='))
        {
            return null;
        }

        rest = rest.Substring(1).Trim();
        if (rest.Length < 2 || rest[0] != '"')
        {
            return null;
        }

        var end = rest.IndexOf('"', 1);
        return end < 0 ? null : rest.Substring(1, end - 1);
    }

    private static string ReadText(ZipArchiveEntry entry)
    {
        using var reader = new StreamReader(entry.Open(), Encoding.UTF8);
        return reader.ReadToEnd();
    }

    private static byte[] ReadBytes(ZipArchiveEntry entry)
    {
        using var stream = entry.Open();
        using var memory = new MemoryStream();
        stream.CopyTo(memory);
        return memory.ToArray();
    }
}
