using System;
using System.IO;
using CodeBrix.RustTools.Projects;
using CodeBrix.WebPlatform.WebGpu.LibraryChecker.Platform;

namespace CodeBrix.WebPlatform.WebGpu.LibraryChecker.Checks;

/// <summary>
/// The Graphics package's crate set (<c>src/CodeBrix.WebPlatform.WebGpu.Graphics/Rust/graphics-crates</c>:
/// gltf): its Cargo.lock, resolved against the platform's and the WebGpu crate set's, split into
/// the crates the package vendors itself (its OWN: those neither the platform nor the WebGpu
/// package carries, in <c>Rust/vendor</c> of the Graphics library and in its archive) and the
/// crates it SHARES (carried by the WebGpu package's vendor folder or the platform's archive, at
/// the same version). The package's bundle holds only what it adds (the owner's rule for the
/// Graphics and Animation packages), and it never depends on the Animation package, so its set is
/// checked against the WebGpu package and the platform only. The work is
/// <see cref="BundleCrateSet"/>'s, for <see cref="BundlePackage.Graphics"/>.
/// </summary>
public static class GraphicsCrateSet
{
    /// <summary>
    /// Derives the split from the lock file and what the WebGpu package and the platform carry.
    /// </summary>
    /// <param name="layout">The repository's folders.</param>
    /// <param name="webGpu">The WebGpu package's vendored crates.</param>
    /// <param name="platform">The platform's crates.</param>
    /// <exception cref="FileNotFoundException">The crate set has no Cargo.lock.</exception>
    public static BundleCrateSet Derive(RepositoryLayout layout, CrateSnapshot webGpu, CrateSnapshot platform)
        => BundleCrateSet.Derive(layout, BundlePackage.Graphics, webGpu, platform);

    /// <summary>
    /// Splits a lock file's crates.io packages into the package's own and the shared ones.
    /// </summary>
    /// <param name="lockFile">The crate set's lock file.</param>
    /// <param name="webGpu">The WebGpu package's vendored crates.</param>
    /// <param name="platform">The platform's crates.</param>
    public static BundleCrateSet FromLock(CargoLockFile lockFile, CrateSnapshot webGpu, CrateSnapshot platform)
        => BundleCrateSet.FromLock(BundlePackage.Graphics, lockFile, webGpu, platform);

    /// <summary>
    /// THE STRICT RULE for the Graphics crate set (<see cref="BundleCrateSet.StrictRule"/>).
    /// </summary>
    /// <param name="set">The split crate set.</param>
    /// <param name="ownVendored">The Graphics package's vendored crates.</param>
    /// <param name="webGpu">The WebGpu package's vendored crates.</param>
    /// <param name="platform">The platform's crates.</param>
    /// <param name="platformVersion">The platform version, for the messages.</param>
    /// <exception cref="ArgumentException"><paramref name="set"/> is not the Graphics package's.</exception>
    public static CheckResult StrictRule(BundleCrateSet set, CrateSnapshot ownVendored, CrateSnapshot webGpu, CrateSnapshot platform, string platformVersion)
    {
        ArgumentNullException.ThrowIfNull(set);
        if (!ReferenceEquals(set.Package, BundlePackage.Graphics))
        {
            throw new ArgumentException($"The crate set is the {set.Package.Label} package's, not the Graphics package's.", nameof(set));
        }

        return BundleCrateSet.StrictRule(set, ownVendored, webGpu, platform, platformVersion);
    }
}
