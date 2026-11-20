using System;
using System.Collections.Generic;
using System.Linq;
using CodeBrix.RustTools;
using CodeBrix.RustTools.Projects;

namespace CodeBrix.WebPlatform.WebGpu.LibraryChecker.Checks;

/// <summary>
/// The crates the vendor folder must hold, derived from the crate set
/// (<c>Rust/webgpu-crates</c>) and its Cargo.lock.
/// </summary>
public sealed class VendoredSet
{
    private VendoredSet(string shortName, string description, IReadOnlyList<CargoLockPackage> packages)
    {
        ShortName = shortName;
        Description = description;
        Packages = packages;
    }

    /// <summary>What the set is, in a few words (for check names).</summary>
    public string ShortName { get; }

    /// <summary>How the set is derived (for notes and messages).</summary>
    public string Description { get; }

    /// <summary>The crates of the set, as the lock file records them (name, version, checksum).</summary>
    public IReadOnlyList<CargoLockPackage> Packages { get; }

    /// <summary>
    /// The whole lock file: every crates.io package it names (what plain <c>cargo vendor</c> takes).
    /// </summary>
    /// <param name="lockFile">The crate set's lock file.</param>
    public static VendoredSet FromLock(CargoLockFile lockFile)
    {
        ArgumentNullException.ThrowIfNull(lockFile);
        return new VendoredSet(
            "Cargo.lock",
            "every crate the crate set's Cargo.lock names (what `cargo vendor --locked --versioned-dirs` writes)",
            lockFile.Packages.Where(p => !p.IsLocal).ToList());
    }

    /// <summary>
    /// A set of given crates (the Animation package's: the crates of its lock that neither the
    /// platform nor the WebGpu package carries).
    /// </summary>
    /// <param name="shortName">What the set is, in a few words.</param>
    /// <param name="description">How the set is derived.</param>
    /// <param name="packages">The crates, as the lock file records them.</param>
    public static VendoredSet FromPackages(string shortName, string description, IReadOnlyList<CargoLockPackage> packages)
    {
        ArgumentNullException.ThrowIfNull(packages);
        return new VendoredSet(shortName, description, packages);
    }

    /// <summary>
    /// Derives the set the way the vendor folder is produced (Rust/vendoring/wgpu/UPGRADE-README.txt,
    /// HOW THE VENDORED SET IS DERIVED): every package the crate set's Cargo.lock names, which is
    /// what <c>cargo vendor --locked --versioned-dirs</c> writes. It is not reduced to what a
    /// browser build compiles: a consumer's offline resolve needs the whole lock.
    /// </summary>
    /// <param name="layout">The repository's folders.</param>
    /// <exception cref="System.IO.FileNotFoundException">The crate set has no Cargo.lock.</exception>
    /// <exception cref="RustToolsException">The lock file is not valid.</exception>
    public static VendoredSet Derive(RepositoryLayout layout)
    {
        ArgumentNullException.ThrowIfNull(layout);
        return FromLock(CargoLockFile.Load(layout.CrateSetLock));
    }
}
