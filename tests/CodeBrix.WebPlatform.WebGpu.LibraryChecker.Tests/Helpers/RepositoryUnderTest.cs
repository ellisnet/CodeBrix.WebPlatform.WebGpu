using System;
using System.IO;
using System.Runtime.CompilerServices;

namespace CodeBrix.WebPlatform.WebGpu.LibraryChecker.Tests.Helpers;

/// <summary>
/// Finds the repository this test project belongs to: by walking up from the test assembly's
/// folder (an in-tree build), or else from the folder this source file was compiled from (a build
/// whose output goes elsewhere, for example with <c>--artifacts-path</c>).
/// </summary>
internal static class RepositoryUnderTest
{
    public static RepositoryLayout Find([CallerFilePath] string sourceFile = "")
        => RepositoryLayout.Find(AppContext.BaseDirectory)
            ?? (string.IsNullOrEmpty(sourceFile) ? null : RepositoryLayout.Find(Path.GetDirectoryName(sourceFile)));
}
