using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using System.Xml.Linq;
using CodeBrix.WebPlatform.WebGpu.LibraryChecker.Platform;

namespace CodeBrix.WebPlatform.WebGpu.LibraryChecker.Checks;

/// <summary>
/// The platform's STRICT RULE for crate bundles, checked here so that a consumer never meets it:
/// every vendored crate whose name the platform's crates also hold must be there at the same
/// version with the same <c>.cargo-checksum.json</c>, and none may be at a version the platform
/// does not carry. Also confirms that the platform release the comparison uses is the one the
/// packages of this repository pin Core to, and the one the temporary feed holds.
/// </summary>
public static class PlatformCheck
{
    /// <summary>The package id of the platform's Core library, which every crate-bearing package pins exactly.</summary>
    public const string CorePackageId = "CodeBrix.WebPlatform.Core.ApacheLicenseForever";

    /// <summary>The package id of the FrontendBuilder library this tool compares against.</summary>
    public const string FrontendBuilderPackageId = "CodeBrix.WebPlatform.FrontendBuilder.ApacheLicenseForever";

    /// <summary>
    /// Runs the strict-rule check against the platform release this tool references.
    /// </summary>
    /// <param name="layout">The repository's folders.</param>
    public static CheckResult StrictRule(RepositoryLayout layout)
    {
        ArgumentNullException.ThrowIfNull(layout);
        return StrictRule(CrateSnapshot.FromDirectory(layout.Vendor), PlatformRelease.LoadCrates(), PlatformRelease.Version, layout.Relative(layout.CrateSet));
    }

    /// <summary>
    /// Compares a vendor folder with the platform's crates under the strict rule.
    /// </summary>
    /// <param name="ours">This repository's vendored crates.</param>
    /// <param name="platform">The platform's crates.</param>
    /// <param name="platformVersion">The platform version, for the messages.</param>
    /// <param name="crateSetShown">The crate set's folder as the messages show it.</param>
    public static CheckResult StrictRule(CrateSnapshot ours, CrateSnapshot platform, string platformVersion, string crateSetShown = "the crate set")
    {
        ArgumentNullException.ThrowIfNull(ours);
        ArgumentNullException.ThrowIfNull(platform);
        var name = $"Platform: every crate the platform also carries is at the platform's version, with the same content (CodeBrix.WebPlatform {platformVersion})";
        var problems = new List<string>();
        var shared = 0;
        var ownOnly = 0;
        foreach (var crate in ours.Crates)
        {
            var platformVersions = platform.Find(crate.Name);
            if (platformVersions.Count == 0)
            {
                ownOnly++;
                continue;
            }

            var same = platformVersions.FirstOrDefault(p => string.Equals(p.Version, crate.Version, StringComparison.Ordinal));
            if (same == null)
            {
                problems.Add($"{crate.Name}: this repository vendors version {crate.Version}, but the platform's crates carry version {string.Join(" and ", platformVersions.Select(p => p.Version))}");
            }
            else if (!same.HasSameContentAs(crate))
            {
                problems.Add($"{crate.Name} {crate.Version}: this repository and the platform's crates both carry it, with different content (.cargo-checksum.json differs)");
            }
            else
            {
                shared++;
            }
        }

        if (problems.Count > 0)
        {
            problems.Add($"A consumer's build would stop on these. Re-resolve the crate set ({crateSetShown}) against the platform's Cargo.lock (the one CodeBrix.WebPlatform {platformVersion} carries) and re-vendor (`LibraryChecker revendor`); the Core pin of the packages and the temporary feed must name that same platform version.");
        }

        return new CheckResult(name, problems, new[]
        {
            $"{ours.Crates.Count} vendored crates: {shared} shared with the platform's {platform.Crates.Count} at identical content, {ownOnly} that the platform does not carry",
        });
    }

    /// <summary>
    /// Confirms that every package of this repository that references the platform's Core pins it
    /// to exactly the platform version this tool compares against, and that the temporary feed
    /// (when there is one) holds that version of Core and of FrontendBuilder.
    /// </summary>
    /// <param name="layout">The repository's folders.</param>
    public static CheckResult CorePin(RepositoryLayout layout) => CorePin(layout, PlatformRelease.Version);

    /// <summary>
    /// Confirms the Core pins and the temporary feed against a given platform version.
    /// </summary>
    /// <param name="layout">The repository's folders.</param>
    /// <param name="platformVersion">The platform version the crates are compared against.</param>
    public static CheckResult CorePin(RepositoryLayout layout, string platformVersion)
    {
        ArgumentNullException.ThrowIfNull(layout);
        var name = $"Platform: the packages pin Core to the platform version the crates are checked against ({platformVersion})";
        var problems = new List<string>();
        var notes = new List<string>();
        var expected = "[" + platformVersion + "]";
        var pinned = 0;
        foreach (var relative in RepositoryLayout.PackageProjectFiles)
        {
            var path = layout.Combine(relative);
            if (!File.Exists(path))
            {
                notes.Add($"{relative} does not exist yet");
                continue;
            }

            XDocument project;
            try
            {
                project = XDocument.Load(path);
            }
            catch (System.Xml.XmlException exception)
            {
                problems.Add($"{relative} is not a readable project file: {exception.Message}");
                continue;
            }

            foreach (var reference in project.Descendants().Where(e => e.Name.LocalName == "PackageReference"))
            {
                var include = (string)reference.Attribute("Include") ?? (string)reference.Attribute("Update");
                if (!string.Equals(include, CorePackageId, StringComparison.OrdinalIgnoreCase))
                {
                    continue;
                }

                var version = ((string)reference.Attribute("Version") ?? reference.Elements().FirstOrDefault(e => e.Name.LocalName == "Version")?.Value ?? string.Empty).Trim();
                pinned++;
                if (!string.Equals(version, expected, StringComparison.Ordinal))
                {
                    problems.Add($"{relative} references {CorePackageId} as '{version}', but the crates are checked against CodeBrix.WebPlatform {platformVersion}; pin it exactly as Version=\"{expected}\", or move this tool's FrontendBuilder reference to the pinned version and re-resolve the crate set");
                }
            }
        }

        notes.Insert(0, pinned == 0
            ? "no package project references Core yet (a crate-bearing package must pin it exactly once it registers its crates)"
            : $"{pinned} Core references in the package projects, each pinned to {expected}");
        if (Directory.Exists(layout.TempPackages))
        {
            var files = Directory.EnumerateFiles(layout.TempPackages, "*.nupkg").Select(Path.GetFileName).ToHashSet(StringComparer.OrdinalIgnoreCase);
            foreach (var id in new[] { CorePackageId, FrontendBuilderPackageId })
            {
                var file = $"{id}.{platformVersion}.nupkg";
                if (!files.Contains(file))
                {
                    problems.Add($"the temporary feed ({layout.Relative(layout.TempPackages)}) has no {file}; copy that platform release's packages into it, or move the pins to the version it holds");
                }
            }

            notes.Add($"the temporary feed holds {files.Count} packages");
        }
        else
        {
            notes.Add("no temporary feed: the platform's packages come from nuget.org");
        }

        return new CheckResult(name, problems, notes);
    }
}
