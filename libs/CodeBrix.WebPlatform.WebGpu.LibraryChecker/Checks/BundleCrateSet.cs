using System;
using System.Collections.Generic;
using System.IO;
using System.IO.Compression;
using System.Linq;
using System.Text.Json;
using CodeBrix.RustTools.Projects;
using CodeBrix.WebPlatform.WebGpu.LibraryChecker.Platform;

namespace CodeBrix.WebPlatform.WebGpu.LibraryChecker.Checks;

/// <summary>
/// The crate set of a package whose bundle holds only what it adds (<see cref="BundlePackage"/>:
/// the Graphics package's <c>Rust/graphics-crates</c>, the Animation package's
/// <c>Rust/animation-crates</c>): its Cargo.lock, resolved against the platform's and the WebGpu
/// crate set's, split into the crates the package vendors itself (its OWN: those neither the
/// platform nor the WebGpu package carries, in <c>Rust/vendor</c> of the library and in its
/// archive) and the crates it SHARES (carried by the WebGpu package's vendor folder or the
/// platform's archive, at the same version). The Graphics and Animation packages never depend on
/// each other, so neither set is checked against the other's crates - only
/// <see cref="BothBundles"/> compares the two, for an application that registers both. cargo can
/// resolve such a set only against a merged source, which <see cref="CreateMergedSource"/> builds
/// for the licence audits, as an application's vendor folder holds all of them.
/// </summary>
public sealed class BundleCrateSet
{
    private BundleCrateSet(BundlePackage package, CargoLockFile lockFile, IReadOnlyList<CargoLockPackage> own, IReadOnlyList<CargoLockPackage> shared)
    {
        Package = package;
        LockFile = lockFile;
        Own = own;
        Shared = shared;
    }

    /// <summary>The package the crate set belongs to.</summary>
    public BundlePackage Package { get; }

    /// <summary>The crate set's lock file.</summary>
    public CargoLockFile LockFile { get; }

    /// <summary>The crates the package vendors itself, as the lock records them.</summary>
    public IReadOnlyList<CargoLockPackage> Own { get; }

    /// <summary>The crates of the lock that the WebGpu package or the platform carries, at the same version.</summary>
    public IReadOnlyList<CargoLockPackage> Shared { get; }

    /// <summary>The package's own crates as the vendor check reads them.</summary>
    public VendoredSet Vendored => VendoredSet.FromPackages(
        "own crates",
        $"the {Package.Label} crate set's Cargo.lock minus the crates the WebGpu package or the platform carries at the same version",
        Own);

    /// <summary>
    /// Derives the split from the package's lock file and what the WebGpu package and the platform carry.
    /// </summary>
    /// <param name="layout">The repository's folders.</param>
    /// <param name="package">The package.</param>
    /// <param name="webGpu">The WebGpu package's vendored crates.</param>
    /// <param name="platform">The platform's crates.</param>
    /// <exception cref="FileNotFoundException">The crate set has no Cargo.lock.</exception>
    public static BundleCrateSet Derive(RepositoryLayout layout, BundlePackage package, CrateSnapshot webGpu, CrateSnapshot platform)
    {
        ArgumentNullException.ThrowIfNull(layout);
        ArgumentNullException.ThrowIfNull(package);
        return FromLock(package, CargoLockFile.Load(package.CrateSetLock(layout)), webGpu, platform);
    }

    /// <summary>
    /// Splits a lock file's crates.io packages into the package's own and the shared ones.
    /// </summary>
    /// <param name="package">The package.</param>
    /// <param name="lockFile">The crate set's lock file.</param>
    /// <param name="webGpu">The WebGpu package's vendored crates.</param>
    /// <param name="platform">The platform's crates.</param>
    public static BundleCrateSet FromLock(BundlePackage package, CargoLockFile lockFile, CrateSnapshot webGpu, CrateSnapshot platform)
    {
        ArgumentNullException.ThrowIfNull(package);
        ArgumentNullException.ThrowIfNull(lockFile);
        ArgumentNullException.ThrowIfNull(webGpu);
        ArgumentNullException.ThrowIfNull(platform);
        bool Carried(CargoLockPackage p) => webGpu.Find(p.Name).Concat(platform.Find(p.Name)).Any(c => string.Equals(c.Version, p.Version, StringComparison.Ordinal));
        var packages = lockFile.Packages.Where(p => !p.IsLocal).ToList();
        return new BundleCrateSet(package, lockFile, packages.Where(p => !Carried(p)).ToList(), packages.Where(Carried).ToList());
    }

    /// <summary>
    /// THE STRICT RULE for the package's crate set: every crate it shares is at the version the
    /// WebGpu package or the platform carries, with the same content (the lock's checksum equals
    /// the carried crate's package checksum); no crate the package vendors itself is a crate the
    /// WebGpu package or the platform carries in another version (a consumer's build would stop on
    /// it); and the package's vendor folder holds nothing the others carry.
    /// </summary>
    /// <param name="set">The split crate set.</param>
    /// <param name="ownVendored">The package's vendored crates.</param>
    /// <param name="webGpu">The WebGpu package's vendored crates.</param>
    /// <param name="platform">The platform's crates.</param>
    /// <param name="platformVersion">The platform version, for the messages.</param>
    public static CheckResult StrictRule(BundleCrateSet set, CrateSnapshot ownVendored, CrateSnapshot webGpu, CrateSnapshot platform, string platformVersion)
    {
        ArgumentNullException.ThrowIfNull(set);
        ArgumentNullException.ThrowIfNull(ownVendored);
        ArgumentNullException.ThrowIfNull(webGpu);
        ArgumentNullException.ThrowIfNull(platform);
        var label = set.Package.Label;
        var notes = set.Package.UpgradeNotes;
        var name = $"Platform: the {label} crate set shares every crate the WebGpu package or the platform carries at its version and content, and vendors only crates neither carries (CodeBrix.WebPlatform {platformVersion})";
        var problems = new List<string>();
        foreach (var package in set.Shared)
        {
            var carried = webGpu.Find(package.Name).Concat(platform.Find(package.Name))
                .Where(c => string.Equals(c.Version, package.Version, StringComparison.Ordinal))
                .Select(PackageChecksum)
                .Distinct(StringComparer.Ordinal)
                .ToList();
            if (string.IsNullOrEmpty(package.Checksum) || !carried.Contains(package.Checksum, StringComparer.Ordinal))
            {
                problems.Add($"{package.Name} {package.Version}: the {label} crate set's Cargo.lock records checksum {package.Checksum}, but the copy the WebGpu package or the platform carries has {string.Join(" / ", carried)}; re-resolve the crate set against their locks ({notes})");
            }
        }

        foreach (var package in set.Own)
        {
            var others = webGpu.Find(package.Name).Concat(platform.Find(package.Name)).Select(c => c.Version).Distinct(StringComparer.Ordinal).ToList();
            if (others.Count > 0)
            {
                problems.Add($"{package.Name}: the {label} crate set needs version {package.Version}, but the WebGpu package or the platform carries {string.Join(" and ", others)}; a consumer's build would stop on it - re-resolve the crate set against their locks ({notes})");
            }
        }

        foreach (var crate in ownVendored.Crates)
        {
            if (webGpu.Find(crate.Name).Count > 0 || platform.Find(crate.Name).Count > 0)
            {
                problems.Add($"{crate.Folder}: the {label} package vendors a crate the WebGpu package or the platform carries; its bundle holds only what it adds - remove the folder from the {label} library's Rust/vendor");
            }
        }

        return new CheckResult(name, problems, new[]
        {
            $"{set.Own.Count} crates of the set are the {label} package's own ({string.Join(", ", set.Own.Select(p => p.Name + " " + p.Version))}); {set.Shared.Count} are shared with the WebGpu package or the platform ({string.Join(", ", set.Shared.Select(p => p.Name + " " + p.Version))})",
        });
    }

    /// <summary>
    /// The two packages' bundles side by side, as an application that registers BOTH meets them
    /// (the platform's strict rule across bundles): a crate name that both packages vendor must be
    /// at one and the same version in both, with the same content (the locks' checksums), or
    /// Materialize refuses the pair and writes nothing. The same name, version and content in both
    /// breaks nothing - the platform writes one copy and the bundle registered later reports it as
    /// shared - so that is listed in a note, for the maintainer to know that the two packages carry
    /// one crate twice.
    /// </summary>
    /// <param name="first">One package's split crate set.</param>
    /// <param name="second">The other package's split crate set.</param>
    public static CheckResult BothBundles(BundleCrateSet first, BundleCrateSet second)
    {
        ArgumentNullException.ThrowIfNull(first);
        ArgumentNullException.ThrowIfNull(second);
        var a = first.Package.Label;
        var b = second.Package.Label;
        var name = $"Platform: no crate is vendored by both the {a} and the {b} package in another version or with other content (an application may register both bundles)";
        var problems = new List<string>();
        var twice = new List<string>();
        var names = first.Own.Select(p => p.Name).Intersect(second.Own.Select(p => p.Name), StringComparer.Ordinal).OrderBy(n => n, StringComparer.Ordinal);
        foreach (var crate in names)
        {
            var inFirst = first.Own.Where(p => p.Name == crate).ToList();
            var inSecond = second.Own.Where(p => p.Name == crate).ToList();
            var versions = inFirst.Concat(inSecond).Select(p => p.Version).Distinct(StringComparer.Ordinal).ToList();
            if (versions.Count > 1)
            {
                problems.Add($"{crate}: the {a} package vendors {string.Join(" and ", inFirst.Select(p => p.Version))}, the {b} package {string.Join(" and ", inSecond.Select(p => p.Version))}; an application that registers both bundles stops on it (the platform's strict rule: no bundle may bring a version of a crate another bundle carries in a different version) - re-resolve one crate set to the other's version ({first.Package.UpgradeNotesShown}, {second.Package.UpgradeNotesShown})");
                continue;
            }

            var checksums = inFirst.Concat(inSecond).Select(p => p.Checksum).Distinct(StringComparer.Ordinal).ToList();
            if (checksums.Count > 1 || checksums.Any(string.IsNullOrEmpty))
            {
                problems.Add($"{crate} {versions[0]}: the {a} and the {b} crate sets' locks record different checksums ({string.Join(" / ", checksums)}); an application that registers both bundles stops on it (the same name and version must have the same content) - re-vendor both from crates.io");
                continue;
            }

            twice.Add(crate + " " + versions[0]);
        }

        return new CheckResult(name, problems, new[]
        {
            twice.Count == 0
                ? $"no crate name is vendored by both ({first.Own.Count} {a} crates, {second.Own.Count} {b} crates)"
                : $"vendored by both packages at the same version and content (breaks nothing: the platform writes one copy, and the bundle registered later reports it as shared): {string.Join(", ", twice)}",
        });
    }

    /// <summary>
    /// Writes a temporary copy of the crate set whose cargo configuration points at a merged
    /// vendor folder: the package's own crates, and each shared crate copied from the WebGpu
    /// package's vendor folder or unpacked from the platform's archive. cargo can resolve and
    /// audit the set offline there. The caller deletes <see cref="MergedSource.Folder"/>.
    /// </summary>
    /// <param name="layout">The repository's folders.</param>
    /// <param name="set">The split crate set.</param>
    /// <param name="platformArchive">The platform's crates archive.</param>
    /// <exception cref="InvalidDataException">A shared crate is in neither place.</exception>
    public static MergedSource CreateMergedSource(RepositoryLayout layout, BundleCrateSet set, string platformArchive)
    {
        ArgumentNullException.ThrowIfNull(layout);
        ArgumentNullException.ThrowIfNull(set);
        var bundle = set.Package;
        var folder = Path.Combine(Path.GetTempPath(), "webgpu-" + bundle.Label.ToLowerInvariant() + "-crate-set-" + Guid.NewGuid().ToString("N"));
        var vendor = Path.Combine(folder, "vendor");
        var crateSet = Path.Combine(folder, bundle.CrateSetFolderName);
        Directory.CreateDirectory(vendor);
        Directory.CreateDirectory(Path.Combine(crateSet, "src"));
        Directory.CreateDirectory(Path.Combine(crateSet, ".cargo"));
        foreach (var file in new[] { "Cargo.toml", "Cargo.lock", Path.Combine("src", "lib.rs") })
        {
            File.Copy(Path.Combine(bundle.CrateSet(layout), file), Path.Combine(crateSet, file));
        }

        File.WriteAllText(Path.Combine(crateSet, ".cargo", "config.toml"),
            "[source.crates-io]\nreplace-with = \"merged\"\n\n[source.merged]\ndirectory = \"../vendor\"\n");
        var fromPlatform = new List<string>();
        foreach (var package in set.Own.Concat(set.Shared))
        {
            var name = package.Name + "-" + package.Version;
            foreach (var candidate in new[] { Path.Combine(bundle.Vendor(layout), name), Path.Combine(layout.Vendor, name) })
            {
                if (Directory.Exists(candidate))
                {
                    CopyFolder(candidate, Path.Combine(vendor, name));
                    break;
                }
            }

            if (!Directory.Exists(Path.Combine(vendor, name)))
            {
                fromPlatform.Add(name);
            }
        }

        if (fromPlatform.Count > 0)
        {
            using var archive = ZipFile.OpenRead(platformArchive);
            foreach (var entry in archive.Entries)
            {
                var path = entry.FullName.Replace('\\', '/');
                var top = path.Split('/')[0];
                if (!fromPlatform.Contains(top, StringComparer.Ordinal) || path.EndsWith('/'))
                {
                    continue;
                }

                var target = Path.Combine(vendor, path.Replace('/', Path.DirectorySeparatorChar));
                Directory.CreateDirectory(Path.GetDirectoryName(target));
                entry.ExtractToFile(target);
            }

            var missing = fromPlatform.Where(n => !Directory.Exists(Path.Combine(vendor, n))).ToList();
            if (missing.Count > 0)
            {
                Directory.Delete(folder, recursive: true);
                throw new InvalidDataException($"The {bundle.Label} crate set names {string.Join(", ", missing)}, which neither the {bundle.Label} package's vendor folder, the WebGpu package's nor the platform's archive holds; re-resolve and re-vendor the crate set ({bundle.UpgradeNotes}).");
            }
        }

        return new MergedSource(folder, crateSet);
    }

    private static string PackageChecksum(SnapshotCrate crate)
    {
        try
        {
            using var json = JsonDocument.Parse(crate.Checksum);
            return json.RootElement.TryGetProperty("package", out var package) && package.ValueKind == JsonValueKind.String ? package.GetString() : null;
        }
        catch (JsonException)
        {
            return null;
        }
    }

    private static void CopyFolder(string source, string destination)
    {
        foreach (var file in Directory.GetFiles(source, "*", SearchOption.AllDirectories))
        {
            var target = Path.Combine(destination, Path.GetRelativePath(source, file));
            Directory.CreateDirectory(Path.GetDirectoryName(target));
            File.Copy(file, target);
        }
    }

    /// <summary>A temporary copy of the crate set with a merged vendor folder.</summary>
    /// <param name="Folder">The temporary folder (delete it afterwards).</param>
    /// <param name="CrateSet">The crate set's copy inside it, whose cargo configuration points at the merged vendor folder.</param>
    public sealed record MergedSource(string Folder, string CrateSet);
}
