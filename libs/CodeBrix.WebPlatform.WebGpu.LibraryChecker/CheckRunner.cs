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
/// third-party crates (the WebGpu package's, the Graphics package's and the Animation package's),
/// their agreement with the platform's crates (and the Graphics and Animation sets' with the WebGpu
/// set's, and with each other's for an application that registers both), the first-party crates, the
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
        BundleCrateSet graphics = null;
        BundleCrateSet animation = null;
        CrateSnapshot webGpuCrates = null;
        CrateSnapshot platformCrates = null;
        if (Wanted("vendor"))
        {
            set = VendoredSet.Derive(layout);
            results.Add(VendorCheck.Crates(layout, set));

            results.Add(VendorCheck.GitVisibility(layout));
            results.Add(GitHubWorkflowCheck.Run(layout));
        }

        if (Wanted("vendor") || Wanted("platform") || Wanted("licenses") || Wanted("notices"))
        {
            webGpuCrates = CrateSnapshot.FromDirectory(layout.Vendor);
            platformCrates = PlatformRelease.LoadCrates();
            graphics = GraphicsCrateSet.Derive(layout, webGpuCrates, platformCrates);
            animation = AnimationCrateSet.Derive(layout, webGpuCrates, platformCrates);
        }

        if (Wanted("vendor"))
        {
            results.Add(VendorCheck.GraphicsCrates(layout, graphics.Vendored));
            results.Add(VendorCheck.AnimationCrates(layout, animation.Vendored));
        }

        if (Wanted("platform"))
        {
            results.Add(PlatformCheck.StrictRule(layout));
            results.Add(GraphicsCrateSet.StrictRule(graphics, CrateSnapshot.FromDirectory(layout.GraphicsVendor), webGpuCrates, platformCrates, PlatformRelease.Version));
            results.Add(AnimationCrateSet.StrictRule(animation, CrateSnapshot.FromDirectory(layout.AnimationVendor), webGpuCrates, platformCrates, PlatformRelease.Version));
            results.Add(BundleCrateSet.BothBundles(graphics, animation));
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

                results.Add(SampleNoticesCheck.Run(layout));
            }

            foreach (var bundle in new[] { graphics, animation })
            {
                var merged = BundleCrateSet.CreateMergedSource(layout, bundle, PlatformRelease.ArchivePath);
                try
                {
                    var (bundleResults, bundleAudits) = await LicenseCheck.RunBundleAsync(layout, bundle.Package, allowlist, merged, cancellationToken).ConfigureAwait(false);
                    if (Wanted("licenses"))
                    {
                        results.AddRange(bundleResults);
                    }

                    if (Wanted("notices"))
                    {
                        if (writeNotices)
                        {
                            var changed = NoticesCheck.WriteBundle(layout, bundle, bundleAudits);
                            results.Add(new CheckResult($"Notices: {bundle.Package.NoticesFileName} written", Array.Empty<string>(), new[] { changed ? "the file changed" : "the file was already up to date" }));
                        }
                        else
                        {
                            results.Add(NoticesCheck.CheckBundle(layout, bundle, bundleAudits));
                        }

                        results.Add(ReferenceEquals(bundle.Package, BundlePackage.Graphics) ? GraphicsNoticesCheck.Run(layout) : AnimationNoticesCheck.Run(layout));
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
        }

        return results;
    }
}
