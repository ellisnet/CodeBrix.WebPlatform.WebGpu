using System;
using System.Collections.Generic;
using System.Linq;
using System.Threading;
using System.Threading.Tasks;
using CodeBrix.RustTools;
using CodeBrix.RustTools.Licensing;
using CodeBrix.RustTools.Projects;
using CodeBrix.RustTools.Vendoring;

namespace CodeBrix.WebPlatform.WebGpu.LibraryChecker.Checks;

/// <summary>
/// The license gate: audits the crates that are compiled into a platform application that uses
/// wgpu (the wasm32 build of the crate set) against the strictest tier, and every vendored crate
/// against the vendored tier - each time also scanning the crate folders for license files the
/// crates do not declare. It also confirms that every vendored crate was audited.
/// </summary>
public static class LicenseCheck
{
    /// <summary>The platform a platform application is compiled for.</summary>
    public const string ApplicationTarget = "wasm32-unknown-unknown";

    /// <summary>
    /// Runs the audits. cargo is run offline against the vendor folder.
    /// </summary>
    /// <param name="layout">The repository's folders.</param>
    /// <param name="allowlist">The approved licenses.</param>
    /// <param name="set">The vendored set; the vendored audit must cover it.</param>
    /// <param name="cancellationToken">Cancels the cargo runs.</param>
    /// <returns>One result per audit, a coverage result, and the audit reports.</returns>
    public static async Task<(IReadOnlyList<CheckResult> Results, LicenseAudits Audits)> RunAsync(RepositoryLayout layout, LicenseAllowlist allowlist, VendoredSet set, CancellationToken cancellationToken = default)
    {
        ArgumentNullException.ThrowIfNull(layout);
        ArgumentNullException.ThrowIfNull(allowlist);
        ArgumentNullException.ThrowIfNull(set);
        var workbench = new RustWorkbench();
        var crateSet = workbench.OpenProject(layout.CrateSet);

        var application = await workbench.AuditLicensesAsync(
            crateSet,
            allowlist.BuildApplicationPolicy(),
            new CargoMetadataOptions { Frozen = true, FilterPlatform = ApplicationTarget },
            LicenseAuditScope.LinkedClosure,
            cancellationToken).ConfigureAwait(false);
        var vendored = await workbench.AuditLicensesAsync(
            crateSet,
            allowlist.BuildVendoredPolicy(),
            new CargoMetadataOptions { Frozen = true },
            LicenseAuditScope.AllPackages,
            cancellationToken).ConfigureAwait(false);

        var results = new List<CheckResult>
        {
            ToResult("Licenses: the crates compiled into a platform application that uses wgpu (strictest tier)", application),
            ToResult("Licenses: the vendored crates (carried by the WebGpu package)", vendored),
            Coverage("Licenses: every vendored crate was audited", VendorFolder.Load(layout.Vendor), vendored),
        };
        return (results, new LicenseAudits { Application = application, Vendored = vendored });
    }

    /// <summary>
    /// Runs the audits of the Animation crate set, offline, in a temporary copy of the set whose
    /// vendor folder merges the Animation package's own crates with the shared ones (see
    /// <see cref="AnimationCrateSet.CreateMergedSource"/>). The audits cover the whole set; the
    /// results judge every crate of it, and the coverage check confirms that every crate of the
    /// Animation package's vendor folder was audited. The caller makes the temporary copy and
    /// deletes it when it no longer needs the reports (the notices are written from them).
    /// </summary>
    /// <param name="layout">The repository's folders.</param>
    /// <param name="allowlist">The approved licenses.</param>
    /// <param name="merged">The temporary copy of the Animation crate set with its merged vendor folder.</param>
    /// <param name="cancellationToken">Cancels the cargo runs.</param>
    /// <returns>One result per audit, a coverage result, and the audit reports.</returns>
    public static async Task<(IReadOnlyList<CheckResult> Results, LicenseAudits Audits)> RunAnimationAsync(RepositoryLayout layout, LicenseAllowlist allowlist, AnimationCrateSet.MergedSource merged, CancellationToken cancellationToken = default)
    {
        ArgumentNullException.ThrowIfNull(layout);
        ArgumentNullException.ThrowIfNull(allowlist);
        ArgumentNullException.ThrowIfNull(merged);
        var workbench = new RustWorkbench();
        var crateSet = workbench.OpenProject(merged.CrateSet);
        var application = await workbench.AuditLicensesAsync(
            crateSet,
            allowlist.BuildApplicationPolicy(),
            new CargoMetadataOptions { Frozen = true, FilterPlatform = ApplicationTarget },
            LicenseAuditScope.LinkedClosure,
            cancellationToken).ConfigureAwait(false);
        var vendored = await workbench.AuditLicensesAsync(
            crateSet,
            allowlist.BuildVendoredPolicy(),
            new CargoMetadataOptions { Frozen = true },
            LicenseAuditScope.AllPackages,
            cancellationToken).ConfigureAwait(false);
        var results = new List<CheckResult>
        {
            ToResult("Licenses: the Animation crate set's crates compiled into a platform application that uses codebrix-webgpu-animation (strictest tier)", application),
            ToResult("Licenses: the Animation crate set's crates (its own carried by the Animation package; the shared ones by the WebGpu package or the platform)", vendored),
            Coverage("Licenses: every crate of the Animation package's vendor folder was audited", VendorFolder.Load(layout.AnimationVendor), vendored),
        };
        return (results, new LicenseAudits { Application = application, Vendored = vendored });
    }

    /// <summary>
    /// Turns an audit report into a check result: one problem per blocking crate.
    /// </summary>
    /// <param name="name">The check's name.</param>
    /// <param name="report">The audit report.</param>
    public static CheckResult ToResult(string name, LicenseAuditReport report)
    {
        ArgumentNullException.ThrowIfNull(report);
        var problems = report.BlockingFindings
            .Select(f => $"{f.CrateName} {f.Version} ({f.DeclaredExpression ?? "no license declared"}): {f.Reason}; see WHEN THE GATE REJECTS SOMETHING in MAINTAINER-README.txt")
            .ToList();
        var scanned = report.Findings.Count(f => f.BundledLicensesScanned);
        var notes = new[]
        {
            $"{report.Findings.Count} crates: {report.ApprovedCount} approved, {report.ExemptCount} exempt (our own), {report.RejectedCount} rejected, {report.UnresolvedCount} unresolved; {scanned} crate folders scanned for license files",
        };
        return new CheckResult(name, problems, notes);
    }

    /// <summary>
    /// Confirms that every crate in a vendor folder has a finding in at least one of the audits,
    /// so nothing sits in the repository without having been judged.
    /// </summary>
    /// <param name="name">The check's name.</param>
    /// <param name="vendor">The vendor folder.</param>
    /// <param name="reports">The audits that should cover it.</param>
    public static CheckResult Coverage(string name, VendorFolder vendor, params LicenseAuditReport[] reports)
    {
        ArgumentNullException.ThrowIfNull(vendor);
        ArgumentNullException.ThrowIfNull(reports);
        var audited = new HashSet<string>(reports.SelectMany(r => r.Findings).Select(f => f.CrateName + "\n" + f.Version), StringComparer.Ordinal);
        var problems = vendor.Crates
            .Where(c => !audited.Contains(c.Name + "\n" + c.Version))
            .Select(c => $"{c.Name} {c.Version} is vendored ({c.DirectoryName}) but no license audit covered it; it is not part of the crate set's derivation - run `LibraryChecker revendor`")
            .ToList();
        return new CheckResult(name, problems, new[] { $"{vendor.Crates.Count} vendored crates" });
    }
}
