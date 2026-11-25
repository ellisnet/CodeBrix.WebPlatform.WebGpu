using System;
using System.Collections.Generic;
using System.Diagnostics;
using System.IO;
using System.Linq;
using CodeBrix.RustTools.Projects;
using CodeBrix.RustTools.Vendoring;

namespace CodeBrix.WebPlatform.WebGpu.LibraryChecker.Checks;

/// <summary>
/// Confirms that each vendor folder (the WebGpu package's, and the Graphics and Animation
/// packages' own) is an exact, untouched copy of the crates its crate set's derivation names (see
/// <see cref="VendoredSet"/>), and that git neither hides nor rewrites any file inside them.
/// </summary>
public static class VendorCheck
{
    /// <summary>
    /// Verifies the vendor folder against the derived set: every crate of the set is present and
    /// untouched (each file matches its <c>.cargo-checksum.json</c> entry, nothing added, nothing
    /// missing), and no other crate is there.
    /// </summary>
    /// <param name="layout">The repository's folders.</param>
    /// <param name="set">The crates the vendor folder must hold.</param>
    public static CheckResult Crates(RepositoryLayout layout, VendoredSet set)
    {
        ArgumentNullException.ThrowIfNull(layout);
        ArgumentNullException.ThrowIfNull(set);
        var vendor = VendorFolder.Load(layout.Vendor);
        var report = VendorVerifier.Verify(vendor, new CargoLockFile { FilePath = layout.CrateSetLock, Packages = set.Packages });
        var problems = report.Issues.Select(i => i.Kind switch
        {
            VendorIssueKind.MissingCrate => i.Message + $" ({set.Description}); run `LibraryChecker revendor`",
            VendorIssueKind.UnexpectedCrate => i.Message + $" ({set.Description}); run `LibraryChecker revendor`, which removes it",
            VendorIssueKind.ModifiedFile or VendorIssueKind.MissingFile or VendorIssueKind.ExtraFile => i.Message + "; vendored crates are never edited - run `LibraryChecker revendor` to restore it",
            _ => i.Message,
        });
        return new CheckResult(
            "Vendored crates: the wgpu crates are untouched and are exactly the crate set's " + set.ShortName,
            problems,
            new[] { $"{report.CratesChecked} crates, {report.FilesChecked} files hashed; the set: {set.Description}" });
    }

    /// <summary>
    /// Verifies the Graphics package's vendor folder against its own crates (the Graphics crate
    /// set's lock minus what the WebGpu package and the platform carry): every one present and
    /// untouched, and no other crate there.
    /// </summary>
    /// <param name="layout">The repository's folders.</param>
    /// <param name="set">The Graphics package's own crates.</param>
    public static CheckResult GraphicsCrates(RepositoryLayout layout, VendoredSet set) => BundleCrates(layout, BundlePackage.Graphics, set);

    /// <summary>
    /// Verifies the Animation package's vendor folder against its own crates (the Animation crate
    /// set's lock minus what the WebGpu package and the platform carry): every one present and
    /// untouched, and no other crate there.
    /// </summary>
    /// <param name="layout">The repository's folders.</param>
    /// <param name="set">The Animation package's own crates.</param>
    public static CheckResult AnimationCrates(RepositoryLayout layout, VendoredSet set) => BundleCrates(layout, BundlePackage.Animation, set);

    /// <summary>
    /// Verifies a package's vendor folder against its own crates (its crate set's lock minus what
    /// the WebGpu package and the platform carry): every one present and untouched, and no other
    /// crate there.
    /// </summary>
    /// <param name="layout">The repository's folders.</param>
    /// <param name="package">The package (Graphics or Animation).</param>
    /// <param name="set">The package's own crates.</param>
    public static CheckResult BundleCrates(RepositoryLayout layout, BundlePackage package, VendoredSet set)
    {
        ArgumentNullException.ThrowIfNull(layout);
        ArgumentNullException.ThrowIfNull(package);
        ArgumentNullException.ThrowIfNull(set);
        var vendor = VendorFolder.Load(package.Vendor(layout));
        var report = VendorVerifier.Verify(vendor, new CargoLockFile { FilePath = package.CrateSetLock(layout), Packages = set.Packages });
        var problems = report.Issues.Select(i => i.Kind switch
        {
            VendorIssueKind.MissingCrate or VendorIssueKind.UnexpectedCrate => i.Message + $" ({set.Description}); re-vendor the {package.Label} crate set ({package.UpgradeNotesShown})",
            VendorIssueKind.ModifiedFile or VendorIssueKind.MissingFile or VendorIssueKind.ExtraFile => i.Message + $"; vendored crates are never edited - restore the crate as crates.io published it ({package.UpgradeNotesShown})",
            _ => i.Message,
        });
        return new CheckResult(
            $"Vendored crates: the {package.Label} package's crates are untouched and are exactly the {package.Label} crate set's " + set.ShortName,
            problems,
            new[] { $"{report.CratesChecked} crates, {report.FilesChecked} files hashed; the set: {set.Description}" });
    }

    /// <summary>
    /// Asks git whether any file inside the three vendor folders (the WebGpu package's, the
    /// Graphics package's and the Animation package's), or the three crate sets' lock files, is
    /// ignored. An ignored file would be missing from a fresh clone, and cargo refuses to build
    /// from a vendored crate with a missing file. Passes with a note when git is not installed or
    /// this is not a git checkout.
    /// </summary>
    /// <param name="layout">The repository's folders.</param>
    public static CheckResult GitVisibility(RepositoryLayout layout)
    {
        ArgumentNullException.ThrowIfNull(layout);
        const string name = "Vendored crates: git ignores no vendored file";
        string output;
        try
        {
            var start = new ProcessStartInfo("git")
            {
                WorkingDirectory = layout.Root,
                RedirectStandardOutput = true,
                RedirectStandardError = true,
                UseShellExecute = false,
            };
            var paths = new[] { layout.Vendor, layout.CrateSetLock, layout.GraphicsVendor, layout.GraphicsCrateSetLock, layout.AnimationVendor, layout.AnimationCrateSetLock }.Where(p => File.Exists(p) || Directory.Exists(p)).Select(layout.Relative);
            foreach (var argument in new[] { "ls-files", "--others", "--ignored", "--exclude-standard", "--" }.Concat(paths))
            {
                start.ArgumentList.Add(argument);
            }

            using var process = Process.Start(start);
            output = process.StandardOutput.ReadToEnd();
            process.StandardError.ReadToEnd();
            process.WaitForExit();
            if (process.ExitCode != 0)
            {
                return new CheckResult(name, Array.Empty<string>(), new[] { "not checked: this folder is not a git checkout" });
            }
        }
        catch (Exception exception) when (exception is System.ComponentModel.Win32Exception or InvalidOperationException or IOException)
        {
            return new CheckResult(name, Array.Empty<string>(), new[] { "not checked: git is not installed" });
        }

        var ignored = output.Split('\n', StringSplitOptions.RemoveEmptyEntries | StringSplitOptions.TrimEntries);
        return new CheckResult(name, ignored.Select(f => $"{f} is ignored by git and would be missing from a clone; un-ignore it in the root .gitignore (its last lines un-ignore the vendor folders)"));
    }
}
