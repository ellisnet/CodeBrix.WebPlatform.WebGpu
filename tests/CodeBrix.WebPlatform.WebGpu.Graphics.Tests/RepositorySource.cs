using System.IO;
using System.Runtime.CompilerServices;

namespace CodeBrix.WebPlatform.WebGpu.Graphics.Tests;

/// <summary>
/// The Graphics library's crate folder in this repository, found from the folder this source
/// file was compiled from (the build output may be outside the repository, with an artifacts
/// path).
/// </summary>
internal static class RepositorySource
{
    /// <summary>src/CodeBrix.WebPlatform.WebGpu.Graphics/Rust/codebrix-webgpu-graphics, or null when the repository is not there.</summary>
    public static string GraphicsCrateFolder([CallerFilePath] string sourceFile = "")
    {
        for (var directory = new DirectoryInfo(Path.GetDirectoryName(sourceFile) ?? string.Empty); directory != null; directory = directory.Parent)
        {
            if (File.Exists(Path.Combine(directory.FullName, "CodeBrix.WebPlatform.WebGpu.slnx")))
            {
                var crate = Path.Combine(directory.FullName, "src", "CodeBrix.WebPlatform.WebGpu.Graphics", "Rust", "codebrix-webgpu-graphics");
                return Directory.Exists(crate) ? crate : null;
            }
        }

        return null;
    }

    /// <summary>The fixtures folder of the Graphics library's crate (the model data blobs the Rust reader tests read), or null when the repository is not there.</summary>
    public static string FixturesFolder([CallerFilePath] string sourceFile = "")
    {
        var crate = GraphicsCrateFolder(sourceFile);
        return crate == null ? null : Path.Combine(crate, "fixtures");
    }
}
