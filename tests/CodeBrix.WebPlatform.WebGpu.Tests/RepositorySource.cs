using System;
using System.IO;
using System.Runtime.CompilerServices;

namespace CodeBrix.WebPlatform.WebGpu.Tests;

/// <summary>
/// The WebGpu library's Rust folder in this repository, found from the folder this source file
/// was compiled from (the build output may be outside the repository, with an artifacts path).
/// </summary>
internal static class RepositorySource
{
    /// <summary>src/CodeBrix.WebPlatform.WebGpu/Rust, or null when the repository is not there.</summary>
    public static string RustFolder([CallerFilePath] string sourceFile = "")
    {
        for (var directory = new DirectoryInfo(Path.GetDirectoryName(sourceFile) ?? string.Empty); directory != null; directory = directory.Parent)
        {
            if (File.Exists(Path.Combine(directory.FullName, "CodeBrix.WebPlatform.WebGpu.slnx")))
            {
                var rust = Path.Combine(directory.FullName, "src", "CodeBrix.WebPlatform.WebGpu", "Rust");
                return Directory.Exists(rust) ? rust : null;
            }
        }

        return null;
    }
}
