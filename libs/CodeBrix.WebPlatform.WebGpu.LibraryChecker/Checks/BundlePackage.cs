using System;
using System.IO;

namespace CodeBrix.WebPlatform.WebGpu.LibraryChecker.Checks;

/// <summary>
/// A package of this repository whose crate bundle holds only what it adds (the owner's rule for
/// the Graphics and Animation packages): its first-party crate and the third-party crates of its
/// own crate set that neither the platform nor the WebGpu package carries. Names the package's
/// folders and how the gate's messages speak of it; <see cref="BundleCrateSet"/> does the work.
/// </summary>
public sealed class BundlePackage
{
    private readonly Func<RepositoryLayout, string> _rust;

    private BundlePackage(string label, string crateName, string rustRelative, string crateSetFolderName, string upgradeNotes, string noticesName, Func<RepositoryLayout, string> rust)
    {
        Label = label;
        CrateName = crateName;
        RustRelative = rustRelative;
        CrateSetFolderName = crateSetFolderName;
        UpgradeNotes = upgradeNotes;
        NoticesName = noticesName;
        _rust = rust;
    }

    /// <summary>The Graphics package (crate set Rust/graphics-crates: gltf).</summary>
    public static BundlePackage Graphics { get; } = new BundlePackage(
        "Graphics",
        FirstPartyCrateCheck.GraphicsCrateName,
        "src/CodeBrix.WebPlatform.WebGpu.Graphics/Rust",
        "graphics-crates",
        "vendoring/gltf/UPGRADE-README.txt",
        "WebGpuGraphicsCrates",
        layout => layout.GraphicsRust);

    /// <summary>The Animation package (crate set Rust/animation-crates: kurbo, peniko, ...).</summary>
    public static BundlePackage Animation { get; } = new BundlePackage(
        "Animation",
        FirstPartyCrateCheck.AnimationCrateName,
        "src/CodeBrix.WebPlatform.WebGpu.Animation/Rust",
        "animation-crates",
        "vendoring/kurbo-peniko/UPGRADE-README.txt",
        "WebGpuAnimationCrates",
        layout => layout.AnimationRust);

    /// <summary>The package's short name in messages ("Graphics", "Animation").</summary>
    public string Label { get; }

    /// <summary>The package's first-party crate (codebrix-webgpu-graphics, codebrix-webgpu-animation).</summary>
    public string CrateName { get; }

    /// <summary>The library's Rust folder, relative to the repository root, with forward slashes.</summary>
    public string RustRelative { get; }

    /// <summary>The crate set's folder name inside the Rust folder (graphics-crates, animation-crates).</summary>
    public string CrateSetFolderName { get; }

    /// <summary>The upgrade notes of the crate set, relative to the Rust folder, as the messages name them.</summary>
    public string UpgradeNotes { get; }

    /// <summary>The upgrade notes of the crate set, relative to the repository root.</summary>
    public string UpgradeNotesShown => RustRelative + "/" + UpgradeNotes;

    /// <summary>The part of the notices file names after "THIRD-PARTY-NOTICES-" and "notices-header-".</summary>
    public string NoticesName { get; }

    /// <summary>The generated notices file's name.</summary>
    public string NoticesFileName => "THIRD-PARTY-NOTICES-" + NoticesName + ".txt";

    /// <summary>The library's Rust folder.</summary>
    /// <param name="layout">The repository's folders.</param>
    public string Rust(RepositoryLayout layout)
    {
        ArgumentNullException.ThrowIfNull(layout);
        return _rust(layout);
    }

    /// <summary>The manifest folder of the package's crate set.</summary>
    /// <param name="layout">The repository's folders.</param>
    public string CrateSet(RepositoryLayout layout) => Path.Combine(Rust(layout), CrateSetFolderName);

    /// <summary>The lock file of the package's crate set.</summary>
    /// <param name="layout">The repository's folders.</param>
    public string CrateSetLock(RepositoryLayout layout) => Path.Combine(CrateSet(layout), "Cargo.lock");

    /// <summary>The package's vendored crates (only those it adds).</summary>
    /// <param name="layout">The repository's folders.</param>
    public string Vendor(RepositoryLayout layout) => Path.Combine(Rust(layout), "vendor");

    /// <summary>The folder of the package's notices.</summary>
    /// <param name="layout">The repository's folders.</param>
    public string NoticesFolder(RepositoryLayout layout) => Path.Combine(Rust(layout), "notices");

    /// <summary>The hand-written header of the package's notices.</summary>
    /// <param name="layout">The repository's folders.</param>
    public string NoticesHeaderFile(RepositoryLayout layout) => Path.Combine(NoticesFolder(layout), "notices-header-" + NoticesName + ".txt");

    /// <summary>The package's generated notices.</summary>
    /// <param name="layout">The repository's folders.</param>
    public string NoticesFile(RepositoryLayout layout) => Path.Combine(NoticesFolder(layout), NoticesFileName);

    /// <inheritdoc />
    public override string ToString() => Label;
}
