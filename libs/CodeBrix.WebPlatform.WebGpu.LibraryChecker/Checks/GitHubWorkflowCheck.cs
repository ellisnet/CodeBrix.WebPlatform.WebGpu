using System;
using System.Collections.Generic;
using System.IO;

namespace CodeBrix.WebPlatform.WebGpu.LibraryChecker.Checks;

/// <summary>
/// The repository has no GitHub workflow files: nothing under a <c>.github/workflows</c> folder
/// anywhere outside the vendored crates. Inside the vendored crates (which are never edited) the
/// crates that carry such files are named in a note for the maintainer, so that a crate that
/// brings one is seen. A copy of a vendored crate elsewhere - the vendor folder a sample's
/// frontend build writes, which is build output - is recognised by the
/// <c>.cargo-checksum.json</c> in the crate's folder and is not scanned.
/// </summary>
public static class GitHubWorkflowCheck
{
    private static readonly HashSet<string> BuildOutputDirectoryNames = new(StringComparer.OrdinalIgnoreCase) { "bin", "obj", "target", "artifacts", "TestResults", ".git", ".vs", ".idea" };

    /// <summary>
    /// Scans the repository.
    /// </summary>
    /// <param name="layout">The repository's folders.</param>
    public static CheckResult Run(RepositoryLayout layout)
    {
        ArgumentNullException.ThrowIfNull(layout);
        return Scan(layout.Root, layout.Vendor);
    }

    /// <summary>
    /// Scans a folder for workflow files.
    /// </summary>
    /// <param name="root">The folder to scan.</param>
    /// <param name="vendorDirectory">The vendored crates: reported as notes, not failures; <c>null</c> for none.</param>
    public static CheckResult Scan(string root, string vendorDirectory)
    {
        ArgumentException.ThrowIfNullOrWhiteSpace(root);
        var fullRoot = Path.GetFullPath(root);
        var vendor = vendorDirectory == null ? null : Path.GetFullPath(vendorDirectory);
        var problems = new List<string>();
        var vendored = new SortedSet<string>(StringComparer.Ordinal);
        Walk(fullRoot, fullRoot, vendor, problems, vendored);
        problems.Sort(StringComparer.Ordinal);
        var notes = new[]
        {
            vendored.Count == 0
                ? "none in the vendored crates either"
                : $"vendored crates that carry upstream workflow files (never edited; reported for the maintainer, not judged): {string.Join(", ", vendored)}",
        };

        return new CheckResult("No GitHub workflows: no .github/workflows files in the repository", problems, notes);
    }

    private static void Walk(string root, string directory, string vendor, List<string> problems, SortedSet<string> vendored)
    {
        foreach (var subdirectory in Directory.EnumerateDirectories(directory))
        {
            var name = Path.GetFileName(subdirectory);
            if (BuildOutputDirectoryNames.Contains(name))
            {
                continue;
            }

            if (string.Equals(name, "workflows", StringComparison.OrdinalIgnoreCase)
                && string.Equals(Path.GetFileName(directory), ".github", StringComparison.OrdinalIgnoreCase))
            {
                var inVendor = vendor != null && (subdirectory + Path.DirectorySeparatorChar).StartsWith(vendor + Path.DirectorySeparatorChar, StringComparison.Ordinal);
                if (!inVendor && File.Exists(Path.Combine(Path.GetDirectoryName(directory), ".cargo-checksum.json")))
                {
                    // A vendored crate that a frontend build unpacked (build output): the crates
                    // themselves are judged where the repository carries them.
                    continue;
                }

                foreach (var file in Directory.EnumerateFiles(subdirectory, "*", SearchOption.AllDirectories))
                {
                    var relative = Path.GetRelativePath(root, file).Replace('\\', '/');
                    if (inVendor)
                    {
                        vendored.Add(Path.GetRelativePath(vendor, file).Replace('\\', '/').Split('/')[0]);
                    }
                    else
                    {
                        problems.Add($"{relative} is a GitHub workflow file; this repository has none - remove it");
                    }
                }

                continue;
            }

            Walk(root, subdirectory, vendor, problems, vendored);
        }
    }
}
