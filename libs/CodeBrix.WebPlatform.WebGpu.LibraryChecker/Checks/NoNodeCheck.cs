using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;

namespace CodeBrix.WebPlatform.WebGpu.LibraryChecker.Checks;

/// <summary>
/// Enforces the "no Node.js" rule in the files the repository owns outside <c>samples/</c>: no
/// package manifests, no node_modules, no lock files of a Node package manager, and no JavaScript
/// or TypeScript source of ours. The vendored crate folders are upstream snapshots and are not
/// scanned (a crate may carry a stray package.json that nothing reads); nor are the samples
/// (applications' own business), the local package feed and build output.
/// </summary>
public static class NoNodeCheck
{
    private static readonly HashSet<string> ForbiddenFileNames = new(StringComparer.OrdinalIgnoreCase)
    {
        "package.json", "package-lock.json", "npm-shrinkwrap.json", "yarn.lock", "pnpm-lock.yaml", "pnpm-workspace.yaml",
        "bun.lock", "bun.lockb", ".npmrc", ".yarnrc", ".yarnrc.yml", ".nvmrc", ".node-version", "deno.json", "deno.lock",
        "tsconfig.json", "webpack.config.js", "vite.config.js", "vite.config.ts", "rollup.config.js", "postcss.config.js",
        "tailwind.config.js", "tailwind.config.ts",
    };

    private static readonly HashSet<string> ForbiddenDirectoryNames = new(StringComparer.OrdinalIgnoreCase) { "node_modules", ".yarn", ".pnpm-store" };

    private static readonly HashSet<string> ScriptExtensions = new(StringComparer.OrdinalIgnoreCase) { ".js", ".mjs", ".cjs", ".jsx", ".ts", ".mts", ".cts", ".tsx" };

    private static readonly HashSet<string> BuildOutputDirectoryNames = new(StringComparer.OrdinalIgnoreCase) { "bin", "obj", "target", "wwwroot", "artifacts", "TestResults", ".git", ".vs", ".idea" };

    /// <summary>
    /// Scans the repository outside <c>samples/</c>.
    /// </summary>
    /// <param name="layout">The repository's folders.</param>
    public static CheckResult Run(RepositoryLayout layout)
    {
        ArgumentNullException.ThrowIfNull(layout);
        var skipped = new[] { layout.Samples, layout.Vendor, layout.AnimationVendor, layout.TempPackages };
        return Scan(layout.Root, skipped);
    }

    /// <summary>
    /// Scans a folder, leaving out the given folders and every build-output folder
    /// (bin, obj, target, wwwroot, artifacts).
    /// </summary>
    /// <param name="root">The folder to scan.</param>
    /// <param name="skippedDirectories">Folders that are upstream snapshots.</param>
    public static CheckResult Scan(string root, IEnumerable<string> skippedDirectories)
    {
        ArgumentException.ThrowIfNullOrWhiteSpace(root);
        var skipped = new HashSet<string>((skippedDirectories ?? Enumerable.Empty<string>()).Select(Path.GetFullPath), StringComparer.Ordinal);
        var problems = new List<string>();
        var filesSeen = 0;
        Walk(Path.GetFullPath(root), Path.GetFullPath(root), skipped, problems, ref filesSeen);
        problems.Sort(StringComparer.Ordinal);
        return new CheckResult("No Node.js: no package manifests, node_modules or JavaScript of ours", problems, new[] { $"{filesSeen} files scanned" });
    }

    private static void Walk(string root, string directory, HashSet<string> skipped, List<string> problems, ref int filesSeen)
    {
        foreach (var subdirectory in Directory.EnumerateDirectories(directory))
        {
            var name = Path.GetFileName(subdirectory);
            var relative = Path.GetRelativePath(root, subdirectory).Replace('\\', '/');
            if (ForbiddenDirectoryNames.Contains(name))
            {
                problems.Add($"{relative}/ is a Node.js folder");
                continue;
            }

            if (skipped.Contains(subdirectory) || BuildOutputDirectoryNames.Contains(name))
            {
                continue;
            }

            Walk(root, subdirectory, skipped, problems, ref filesSeen);
        }

        foreach (var file in Directory.EnumerateFiles(directory))
        {
            filesSeen++;
            var name = Path.GetFileName(file);
            var relative = Path.GetRelativePath(root, file).Replace('\\', '/');
            if (ForbiddenFileNames.Contains(name))
            {
                problems.Add($"{relative} is a Node.js / JavaScript tooling file; remove it");
            }
            else if (ScriptExtensions.Contains(Path.GetExtension(name)))
            {
                problems.Add($"{relative} is JavaScript or TypeScript source; this repository's frontend code is Rust only - remove it");
            }
        }
    }
}
