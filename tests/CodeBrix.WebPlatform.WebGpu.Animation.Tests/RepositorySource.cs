using System.IO;
using System.Runtime.CompilerServices;

namespace CodeBrix.WebPlatform.WebGpu.Animation.Tests;

/// <summary>
/// The Animation library's Rust folders in this repository, found from the folder this source
/// file was compiled from (the build output may be outside the repository, with an artifacts
/// path).
/// </summary>
internal static class RepositorySource
{
    /// <summary>src/CodeBrix.WebPlatform.WebGpu.Animation/Rust, or null when the repository is not there.</summary>
    public static string AnimationRustFolder([CallerFilePath] string sourceFile = "")
    {
        for (var directory = new DirectoryInfo(Path.GetDirectoryName(sourceFile) ?? string.Empty); directory != null; directory = directory.Parent)
        {
            if (File.Exists(Path.Combine(directory.FullName, "CodeBrix.WebPlatform.WebGpu.slnx")))
            {
                var rust = Path.Combine(directory.FullName, "src", "CodeBrix.WebPlatform.WebGpu.Animation", "Rust");
                return Directory.Exists(rust) ? rust : null;
            }
        }

        return null;
    }

    /// <summary>The Animation library's crate folder (codebrix-webgpu-animation), or null when the repository is not there.</summary>
    public static string AnimationCrateFolder([CallerFilePath] string sourceFile = "")
    {
        var rust = AnimationRustFolder(sourceFile);
        return rust == null ? null : Path.Combine(rust, "codebrix-webgpu-animation");
    }

    /// <summary>The Animation library's vendor folder (the third-party crates only it carries), or null when the repository is not there.</summary>
    public static string AnimationVendorFolder([CallerFilePath] string sourceFile = "")
    {
        var rust = AnimationRustFolder(sourceFile);
        return rust == null ? null : Path.Combine(rust, "vendor");
    }
}
