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
/// The Animation package's crate set (<c>src/CodeBrix.WebPlatform.WebGpu.Animation/Rust/animation-crates</c>):
/// its Cargo.lock, resolved against the platform's and the WebGpu crate set's, split into the
/// crates the package vendors itself (its OWN: those neither the platform nor the WebGpu package
/// carries, in <c>Rust/vendor</c> of the Animation library and in its archive) and the crates it
/// SHARES (carried by the WebGpu package's vendor folder or the platform's archive, at the same
/// version). The package's bundle holds only what it adds (the owner's rule for the Graphics and
/// Animation packages), so cargo can resolve the set only against a merged source - which
/// <see cref="CreateMergedSource"/> builds for the licence audits, as an application's vendor
/// folder holds all three.
/// </summary>
public sealed class AnimationCrateSet
{
    private AnimationCrateSet(CargoLockFile lockFile, IReadOnlyList<CargoLockPackage> own, IReadOnlyList<CargoLockPackage> shared)
    {
        LockFile = lockFile;
        Own = own;
        Shared = shared;
    }

    /// <summary>The crate set's lock file.</summary>
    public CargoLockFile LockFile { get; }

    /// <summary>The crates the Animation package vendors itself, as the lock records them.</summary>
    public IReadOnlyList<CargoLockPackage> Own { get; }

    /// <summary>The crates of the lock that the WebGpu package or the platform carries, at the same version.</summary>
    public IReadOnlyList<CargoLockPackage> Shared { get; }

    /// <summary>The package's own crates as the vendor check reads them.</summary>
    public VendoredSet Vendored => VendoredSet.FromPackages(
        "own crates",
        "the Animation crate set's Cargo.lock minus the crates the WebGpu package or the platform carries at the same version",
        Own);

    /// <summary>
    /// Derives the split from the lock file and what the WebGpu package and the platform carry.
    /// </summary>
    /// <param name="layout">The repository's folders.</param>
    /// <param name="webGpu">The WebGpu package's vendored crates.</param>
    /// <param name="platform">The platform's crates.</param>
    /// <exception cref="FileNotFoundException">The crate set has no Cargo.lock.</exception>
    public static AnimationCrateSet Derive(RepositoryLayout layout, CrateSnapshot webGpu, CrateSnapshot platform)
    {
        ArgumentNullException.ThrowIfNull(layout);
        return FromLock(CargoLockFile.Load(layout.AnimationCrateSetLock), webGpu, platform);
    }

    /// <summary>
    /// Splits a lock file's crates.io packages into the package's own and the shared ones.
    /// </summary>
    /// <param name="lockFile">The crate set's lock file.</param>
    /// <param name="webGpu">The WebGpu package's vendored crates.</param>
    /// <param name="platform">The platform's crates.</param>
    public static AnimationCrateSet FromLock(CargoLockFile lockFile, CrateSnapshot webGpu, CrateSnapshot platform)
    {
        ArgumentNullException.ThrowIfNull(lockFile);
        ArgumentNullException.ThrowIfNull(webGpu);
        ArgumentNullException.ThrowIfNull(platform);
        bool Carried(CargoLockPackage p) => webGpu.Find(p.Name).Concat(platform.Find(p.Name)).Any(c => string.Equals(c.Version, p.Version, StringComparison.Ordinal));
        var packages = lockFile.Packages.Where(p => !p.IsLocal).ToList();
        return new AnimationCrateSet(lockFile, packages.Where(p => !Carried(p)).ToList(), packages.Where(Carried).ToList());
    }

    /// <summary>
    /// THE STRICT RULE for the Animation crate set: every crate it shares is at the version the
    /// WebGpu package or the platform carries, with the same content (the lock's checksum equals
    /// the carried crate's package checksum); no crate the package vendors itself is a crate the
    /// WebGpu package or the platform carries in another version (a consumer's build would stop on
    /// it); and the package's vendor folder holds nothing the others carry.
    /// </summary>
    /// <param name="set">The split crate set.</param>
    /// <param name="ownVendored">The Animation package's vendored crates.</param>
    /// <param name="webGpu">The WebGpu package's vendored crates.</param>
    /// <param name="platform">The platform's crates.</param>
    /// <param name="platformVersion">The platform version, for the messages.</param>
    public static CheckResult StrictRule(AnimationCrateSet set, CrateSnapshot ownVendored, CrateSnapshot webGpu, CrateSnapshot platform, string platformVersion)
    {
        ArgumentNullException.ThrowIfNull(set);
        ArgumentNullException.ThrowIfNull(ownVendored);
        ArgumentNullException.ThrowIfNull(webGpu);
        ArgumentNullException.ThrowIfNull(platform);
        var name = $"Platform: the Animation crate set shares every crate the WebGpu package or the platform carries at its version and content, and vendors only crates neither carries (CodeBrix.WebPlatform {platformVersion})";
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
                problems.Add($"{package.Name} {package.Version}: the Animation crate set's Cargo.lock records checksum {package.Checksum}, but the copy the WebGpu package or the platform carries has {string.Join(" / ", carried)}; re-resolve the crate set against their locks (vendoring/kurbo-peniko/UPGRADE-README.txt)");
            }
        }

        foreach (var package in set.Own)
        {
            var others = webGpu.Find(package.Name).Concat(platform.Find(package.Name)).Select(c => c.Version).Distinct(StringComparer.Ordinal).ToList();
            if (others.Count > 0)
            {
                problems.Add($"{package.Name}: the Animation crate set needs version {package.Version}, but the WebGpu package or the platform carries {string.Join(" and ", others)}; a consumer's build would stop on it - re-resolve the crate set against their locks (vendoring/kurbo-peniko/UPGRADE-README.txt)");
            }
        }

        foreach (var crate in ownVendored.Crates)
        {
            if (webGpu.Find(crate.Name).Count > 0 || platform.Find(crate.Name).Count > 0)
            {
                problems.Add($"{crate.Folder}: the Animation package vendors a crate the WebGpu package or the platform carries; its bundle holds only what it adds - remove the folder from the Animation library's Rust/vendor");
            }
        }

        return new CheckResult(name, problems, new[]
        {
            $"{set.Own.Count} crates of the set are the Animation package's own ({string.Join(", ", set.Own.Select(p => p.Name + " " + p.Version))}); {set.Shared.Count} are shared with the WebGpu package or the platform ({string.Join(", ", set.Shared.Select(p => p.Name + " " + p.Version))})",
        });
    }

    /// <summary>
    /// Writes a temporary copy of the crate set whose cargo configuration points at a merged
    /// vendor folder: the Animation package's own crates, and each shared crate copied from the
    /// WebGpu package's vendor folder or unpacked from the platform's archive. cargo can resolve
    /// and audit the set offline there. The caller deletes <see cref="MergedSource.Folder"/>.
    /// </summary>
    /// <param name="layout">The repository's folders.</param>
    /// <param name="set">The split crate set.</param>
    /// <param name="platformArchive">The platform's crates archive.</param>
    /// <exception cref="InvalidDataException">A shared crate is in neither place.</exception>
    public static MergedSource CreateMergedSource(RepositoryLayout layout, AnimationCrateSet set, string platformArchive)
    {
        ArgumentNullException.ThrowIfNull(layout);
        ArgumentNullException.ThrowIfNull(set);
        var folder = Path.Combine(Path.GetTempPath(), "webgpu-animation-crate-set-" + Guid.NewGuid().ToString("N"));
        var vendor = Path.Combine(folder, "vendor");
        var crateSet = Path.Combine(folder, "animation-crates");
        Directory.CreateDirectory(vendor);
        Directory.CreateDirectory(Path.Combine(crateSet, "src"));
        Directory.CreateDirectory(Path.Combine(crateSet, ".cargo"));
        foreach (var file in new[] { "Cargo.toml", "Cargo.lock", Path.Combine("src", "lib.rs") })
        {
            File.Copy(Path.Combine(layout.AnimationCrateSet, file), Path.Combine(crateSet, file));
        }

        File.WriteAllText(Path.Combine(crateSet, ".cargo", "config.toml"),
            "[source.crates-io]\nreplace-with = \"merged\"\n\n[source.merged]\ndirectory = \"../vendor\"\n");
        var fromPlatform = new List<string>();
        foreach (var package in set.Own.Concat(set.Shared))
        {
            var name = package.Name + "-" + package.Version;
            foreach (var candidate in new[] { Path.Combine(layout.AnimationVendor, name), Path.Combine(layout.Vendor, name) })
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
                throw new InvalidDataException($"The Animation crate set names {string.Join(", ", missing)}, which neither the Animation package's vendor folder, the WebGpu package's nor the platform's archive holds; re-resolve and re-vendor the crate set (vendoring/kurbo-peniko/UPGRADE-README.txt).");
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
