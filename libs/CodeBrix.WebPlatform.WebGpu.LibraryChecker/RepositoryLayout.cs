using System;
using System.Collections.Generic;
using System.IO;

namespace CodeBrix.WebPlatform.WebGpu.LibraryChecker;

/// <summary>
/// The folders and files of the CodeBrix.WebPlatform.WebGpu repository that the maintainer tool
/// checks, all derived from the repository root (the folder that holds
/// CodeBrix.WebPlatform.WebGpu.slnx and the licence allowlist). Only what is in the repository
/// outside <c>samples/</c> is checked: the wgpu crates that the WebGpu package carries, their
/// crate set and lock file, the third-party crates that only the Animation package carries and
/// their crate set, the first-party crates, the dependency rule between the packages, the
/// notices, the licence allowlist and the no-Node rule.
/// </summary>
public sealed class RepositoryLayout
{
    /// <summary>The file, relative to the repository root, that marks it (with the solution).</summary>
    public const string MarkerFile = "tools/licensing/license-allowlist.json";

    /// <summary>The solution file at the repository root.</summary>
    public const string SolutionFile = "CodeBrix.WebPlatform.WebGpu.slnx";

    /// <summary>The packable library projects, relative to the repository root.</summary>
    public static readonly IReadOnlyList<string> PackageProjectFiles = new[]
    {
        "src/CodeBrix.WebPlatform.WebGpu/CodeBrix.WebPlatform.WebGpu.csproj",
        "src/CodeBrix.WebPlatform.WebGpu.Graphics/CodeBrix.WebPlatform.WebGpu.Graphics.csproj",
        "src/CodeBrix.WebPlatform.WebGpu.Animation/CodeBrix.WebPlatform.WebGpu.Animation.csproj",
    };

    /// <summary>
    /// Creates a layout for a known root folder.
    /// </summary>
    /// <param name="root">The repository root.</param>
    public RepositoryLayout(string root)
    {
        ArgumentException.ThrowIfNullOrWhiteSpace(root);
        Root = Path.GetFullPath(root);
    }

    /// <summary>The repository root.</summary>
    public string Root { get; }

    /// <summary>The folder of the WebGpu library's Rust material.</summary>
    public string WebGpuRust => Combine("src/CodeBrix.WebPlatform.WebGpu/Rust");

    /// <summary>The manifest folder that names the wgpu crate set (what <c>cargo vendor</c> takes).</summary>
    public string CrateSet => Path.Combine(WebGpuRust, "webgpu-crates");

    /// <summary>The lock file of the crate set.</summary>
    public string CrateSetLock => Path.Combine(CrateSet, "Cargo.lock");

    /// <summary>
    /// The first-party crate of this repository (codebrix-webgpu): carried in the same archive as
    /// the vendored crates, in vendored form, but ours - not third-party.
    /// </summary>
    public string FirstPartyCrate => Path.Combine(WebGpuRust, "codebrix-webgpu");

    /// <summary>The WebGpu library's project file.</summary>
    public string WebGpuProjectFile => Combine(PackageProjectFiles[0]);

    /// <summary>The Graphics library's project file.</summary>
    public string GraphicsProjectFile => Combine(PackageProjectFiles[1]);

    /// <summary>The Animation library's project file.</summary>
    public string AnimationProjectFile => Combine(PackageProjectFiles[2]);

    /// <summary>The folder of the Graphics library's Rust material.</summary>
    public string GraphicsRust => Combine("src/CodeBrix.WebPlatform.WebGpu.Graphics/Rust");

    /// <summary>
    /// The first-party crate of the Graphics package (codebrix-webgpu-graphics): carried in that
    /// package's own archive, in vendored form, alone; ours - not third-party.
    /// </summary>
    public string GraphicsCrate => Path.Combine(GraphicsRust, "codebrix-webgpu-graphics");

    /// <summary>The hand-written notices for what the Graphics crate contains, packed into the Graphics package.</summary>
    public string GraphicsNoticesFile => Path.Combine(GraphicsRust, "notices", "THIRD-PARTY-NOTICES-WebGpuGraphicsCrates.txt");

    /// <summary>The folder of the Animation library's Rust material.</summary>
    public string AnimationRust => Combine("src/CodeBrix.WebPlatform.WebGpu.Animation/Rust");

    /// <summary>
    /// The first-party crate of the Animation package (codebrix-webgpu-animation): carried in that
    /// package's own archive, in vendored form, beside the third-party crates only it needs; ours -
    /// not third-party.
    /// </summary>
    public string AnimationCrate => Path.Combine(AnimationRust, "codebrix-webgpu-animation");

    /// <summary>
    /// The manifest folder that names the Animation package's crate set (kurbo, peniko), whose
    /// Cargo.lock is resolved against the platform's and the WebGpu crate set's.
    /// </summary>
    public string AnimationCrateSet => Path.Combine(AnimationRust, "animation-crates");

    /// <summary>The lock file of the Animation crate set.</summary>
    public string AnimationCrateSetLock => Path.Combine(AnimationCrateSet, "Cargo.lock");

    /// <summary>
    /// The third-party crates that ONLY the Animation package carries (the Animation crate set's
    /// crates that neither the platform nor the WebGpu package carries), in its archive.
    /// </summary>
    public string AnimationVendor => Path.Combine(AnimationRust, "vendor");

    /// <summary>The Animation package's vendored crates, relative to the repository root, with forward slashes.</summary>
    public string AnimationVendorRelative => Relative(AnimationVendor);

    /// <summary>The folder of the Animation package's notices: the hand-written header and the generated file.</summary>
    public string AnimationNoticesFolder => Path.Combine(AnimationRust, "notices");

    /// <summary>The hand-written opening of the Animation package's notices (the vello and Skia material of its crate).</summary>
    public string AnimationNoticesHeaderFile => Path.Combine(AnimationNoticesFolder, "notices-header-WebGpuAnimationCrates.txt");

    /// <summary>The generated notices of the Animation package, packed into it.</summary>
    public string AnimationNoticesFile => Path.Combine(AnimationNoticesFolder, "THIRD-PARTY-NOTICES-WebGpuAnimationCrates.txt");

    /// <summary>The vendored crates, which the WebGpu package carries as one archive.</summary>
    public string Vendor => Path.Combine(WebGpuRust, "vendor");

    /// <summary>The vendored crates, relative to the repository root, with forward slashes.</summary>
    public string VendorRelative => Relative(Vendor);

    /// <summary>The upgrade notes of the libraries chosen by name (wgpu).</summary>
    public string Vendoring => Path.Combine(WebGpuRust, "vendoring");

    /// <summary>The folder of the crates' notices: the hand-written header and the generated file.</summary>
    public string NoticesFolder => Path.Combine(WebGpuRust, "notices");

    /// <summary>The hand-written opening of the crates' notices.</summary>
    public string NoticesHeaderFile => Path.Combine(NoticesFolder, "notices-header-WebGpuCrates.txt");

    /// <summary>The generated notices of the vendored crates, packed into the WebGpu package.</summary>
    public string NoticesFile => Path.Combine(NoticesFolder, "THIRD-PARTY-NOTICES-WebGpuCrates.txt");

    /// <summary>The licence policy folder.</summary>
    public string Licensing => Combine("tools/licensing");

    /// <summary>The licence allowlist that only the repository owner edits.</summary>
    public string AllowlistFile => Combine(MarkerFile);

    /// <summary>The sample applications, which this tool does not check.</summary>
    public string Samples => Combine("samples");

    /// <summary>
    /// True when a folder is the repository root.
    /// </summary>
    /// <param name="directory">The folder.</param>
    public static bool IsRoot(string directory)
        => !string.IsNullOrWhiteSpace(directory)
            && File.Exists(Path.Combine(directory, SolutionFile))
            && File.Exists(Path.Combine(directory, MarkerFile.Replace('/', Path.DirectorySeparatorChar)));

    /// <summary>
    /// Finds the repository root by walking up from <paramref name="start"/>.
    /// </summary>
    /// <param name="start">Where to start; a file path is reduced to its folder.</param>
    /// <returns>The layout, or <c>null</c> when no repository root was found.</returns>
    public static RepositoryLayout Find(string start)
    {
        if (string.IsNullOrWhiteSpace(start))
        {
            return null;
        }

        for (var directory = new DirectoryInfo(Path.GetFullPath(start)); directory != null; directory = directory.Parent)
        {
            if (IsRoot(directory.FullName))
            {
                return new RepositoryLayout(directory.FullName);
            }
        }

        return null;
    }

    /// <summary>A path below the root, relative to it with forward slashes.</summary>
    /// <param name="path">A full path.</param>
    public string Relative(string path) => Path.GetRelativePath(Root, path).Replace('\\', '/');

    /// <summary>A path below the root, from a path relative to it with forward slashes.</summary>
    /// <param name="relative">The relative path.</param>
    public string Combine(string relative) => Path.Combine(Root, relative.Replace('/', Path.DirectorySeparatorChar));
}
