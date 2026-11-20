using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using System.Threading;
using System.Threading.Tasks;
using CodeBrix.WebPlatform.WebGpu.LibraryChecker.Checks;
using CodeBrix.WebPlatform.WebGpu.LibraryChecker.Platform;

namespace CodeBrix.WebPlatform.WebGpu.LibraryChecker;

/// <summary>
/// Runs the checks that guard what the repository holds outside <c>samples/</c>: the vendored
/// third-party crates (the WebGpu package's and the Animation package's), their agreement with the
/// platform's crates (and the Animation set's with the WebGpu set's), the first-party crates, the
/// dependency rule between the Graphics and Animation packages, the licences, the notices, the
/// no-Node rule and the no-workflow rule. Used by the LibraryChecker command line and its tests.
/// </summary>
public static class CheckRunner
{
    /// <summary>The names of the check groups, in the order <c>all</c> runs them.</summary>
    public static readonly IReadOnlyList<string> Groups = new[] { "no-node", "vendor", "platform", "first-party", "dependency-rule", "licenses", "notices" };

    /// <summary>
    /// Runs one group of checks, or all of them.
    /// </summary>
    /// <param name="layout">The repository's folders.</param>
    /// <param name="group">One of <see cref="Groups"/>, or <c>all</c>.</param>
    /// <param name="writeNotices">For the notices group: write the generated notices file instead
    /// of comparing with it.</param>
    /// <param name="cancellationToken">Cancels the cargo runs.</param>
    /// <returns>Every check's result, in run order.</returns>
    /// <exception cref="ArgumentException"><paramref name="group"/> is not a known group.</exception>
    public static async Task<IReadOnlyList<CheckResult>> RunAsync(RepositoryLayout layout, string group = "all", bool writeNotices = false, CancellationToken cancellationToken = default)
    {
        ArgumentNullException.ThrowIfNull(layout);
        var all = string.Equals(group, "all", StringComparison.OrdinalIgnoreCase);
        if (!all && !Groups.Contains(group, StringComparer.OrdinalIgnoreCase))
        {
            throw new ArgumentException($"Unknown check '{group}'. Use one of: all, {string.Join(", ", Groups)}.", nameof(group));
        }

        bool Wanted(string name) => all || string.Equals(group, name, StringComparison.OrdinalIgnoreCase);
        var results = new List<CheckResult>();
        if (Wanted("no-node"))
        {
            results.Add(NoNodeCheck.Run(layout));
        }

        VendoredSet set = null;
        AnimationCrateSet animation = null;
        if (Wanted("vendor"))
        {
            set = VendoredSet.Derive(layout);
            results.Add(VendorCheck.Crates(layout, set));

            results.Add(VendorCheck.GitVisibility(layout));
            results.Add(GitHubWorkflowCheck.Run(layout));
        }

        if (Wanted("vendor") || Wanted("platform") || Wanted("licenses") || Wanted("notices"))
        {
            animation = AnimationCrateSet.Derive(layout, CrateSnapshot.FromDirectory(layout.Vendor), PlatformRelease.LoadCrates());
        }

        if (Wanted("vendor"))
        {
            results.Add(VendorCheck.AnimationCrates(layout, animation.Vendored));
        }

        if (Wanted("platform"))
        {
            results.Add(PlatformCheck.StrictRule(layout));
            results.Add(AnimationCrateSet.StrictRule(animation, CrateSnapshot.FromDirectory(layout.AnimationVendor), CrateSnapshot.FromDirectory(layout.Vendor), PlatformRelease.LoadCrates(), PlatformRelease.Version));
            results.Add(PlatformCheck.CorePin(layout));
            results.Add(FamilyPinCheck.Run(layout));
        }

        if (Wanted("first-party"))
        {
            results.AddRange(FirstPartyCrateCheck.Run(layout));
        }

        if (Wanted("dependency-rule"))
        {
            results.AddRange(DependencyRuleCheck.Run(layout));
        }

        if (Wanted("licenses") || Wanted("notices"))
        {
            set ??= VendoredSet.Derive(layout);
            var allowlist = LicenseAllowlist.Load(layout.AllowlistFile);
            var (licenseResults, audits) = await LicenseCheck.RunAsync(layout, allowlist, set, cancellationToken).ConfigureAwait(false);
            if (Wanted("licenses"))
            {
                results.AddRange(licenseResults);
            }

            if (Wanted("notices"))
            {
                if (writeNotices)
                {
                    var changed = NoticesCheck.Write(layout, audits);
                    results.Add(new CheckResult("Notices: THIRD-PARTY-NOTICES-WebGpuCrates.txt written", Array.Empty<string>(), new[] { changed ? "the file changed" : "the file was already up to date" }));
                }
                else
                {
                    results.Add(NoticesCheck.Check(layout, audits));
                }

                results.Add(GraphicsNoticesCheck.Run(layout));
                results.Add(SampleNoticesCheck.Run(layout));
            }

            var merged = AnimationCrateSet.CreateMergedSource(layout, animation, PlatformRelease.ArchivePath);
            try
            {
                var (animationResults, animationAudits) = await LicenseCheck.RunAnimationAsync(layout, allowlist, merged, cancellationToken).ConfigureAwait(false);
                if (Wanted("licenses"))
                {
                    results.AddRange(animationResults);
                }

                if (Wanted("notices"))
                {
                    if (writeNotices)
                    {
                        var changed = NoticesCheck.WriteAnimation(layout, animation, animationAudits);
                        results.Add(new CheckResult("Notices: THIRD-PARTY-NOTICES-WebGpuAnimationCrates.txt written", Array.Empty<string>(), new[] { changed ? "the file changed" : "the file was already up to date" }));
                    }
                    else
                    {
                        results.Add(NoticesCheck.CheckAnimation(layout, animation, animationAudits));
                    }

                    results.Add(AnimationNoticesCheck.Run(layout));
                }
            }
            finally
            {
                if (Directory.Exists(merged.Folder))
                {
                    Directory.Delete(merged.Folder, recursive: true);
                }
            }
        }

        return results;
    }
}
