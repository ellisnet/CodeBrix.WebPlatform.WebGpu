using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using System.Text;
using CodeBrix.RustTools.Licensing;
using CodeBrix.RustTools.Notices;
using CodeBrix.RustTools.Projects;

namespace CodeBrix.WebPlatform.WebGpu.LibraryChecker.Checks;

/// <summary>
/// Writes, or checks, the generated notices of the vendored crates
/// (<c>Rust/notices/THIRD-PARTY-NOTICES-WebGpuCrates.txt</c>, packed into the WebGpu package): the
/// hand-written header beside it, then Part 1, the crates compiled into a platform application
/// that uses wgpu, and Part 2, the rest of the vendored crates, each with its license texts; and
/// the same for the Graphics and the Animation packages' own crates
/// (<see cref="GenerateBundle"/>).
/// </summary>
public static class NoticesCheck
{
    /// <summary>
    /// Builds the notices text from the license audits.
    /// </summary>
    /// <param name="layout">The repository's folders.</param>
    /// <param name="audits">The license audits.</param>
    /// <exception cref="FileNotFoundException">The notices header file is missing.</exception>
    public static string Generate(RepositoryLayout layout, LicenseAudits audits)
    {
        ArgumentNullException.ThrowIfNull(layout);
        ArgumentNullException.ThrowIfNull(audits);
        var lockFile = CargoLockFile.Load(layout.CrateSetLock);
        var linkedIds = new HashSet<string>(audits.Application.Findings.Select(f => f.PackageId), StringComparer.Ordinal);
        var vendoredOnly = audits.Vendored.Findings.Where(f => !linkedIds.Contains(f.PackageId)).ToList();
        var sections = new[]
        {
            new ThirdPartyNoticesSection
            {
                Title = "PART 1 - the crates compiled into a platform application that uses wgpu",
                Introduction = "These crates are linked into the WebAssembly file of a platform application that uses the wgpu\n"
                    + "crates this package brings (the crate set's wgpu with its features; an application compiles in\n"
                    + "those it uses). Many of them are also among the platform's own crates, at the same version and\n"
                    + "with the same content. Their source is in the crate archive of the\n"
                    + "CodeBrix.WebPlatform.WebGpu package, and in the repository at\n"
                    + $"{layout.VendorRelative}, unmodified.",
                Report = audits.Application,
                LockFile = lockFile,
            },
            new ThirdPartyNoticesSection
            {
                Title = "PART 2 - the crates that are carried but not compiled into an application",
                Introduction = "These crates are named by the crate set's Cargo.lock - for native targets (wgpu's native\n"
                    + "implementation and its dependencies), for build scripts and procedural macros, which run while\n"
                    + "an application is compiled - and cargo needs them to resolve a frontend offline, so the package\n"
                    + "carries them, unmodified. None of their code is in an application's WebAssembly file.",
                Report = new LicenseAuditReport { Findings = vendoredOnly, Policy = audits.Vendored.Policy, Scope = audits.Vendored.Scope },
                LockFile = lockFile,
            },
        };
        return ThirdPartyNoticesGenerator.Generate(sections, new ThirdPartyNoticesOptions { Header = ReadHeader(layout.NoticesHeaderFile) });
    }

    private static string ReadHeader(string path)
    {
        if (!File.Exists(path))
        {
            throw new FileNotFoundException($"The notices header '{path}' does not exist. It is hand-written; create it before generating the notices.", path);
        }

        return File.ReadAllText(path, Encoding.UTF8);
    }

    /// <summary>
    /// Compares the generated text with the committed notices file.
    /// </summary>
    /// <param name="layout">The repository's folders.</param>
    /// <param name="audits">The license audits.</param>
    public static CheckResult Check(RepositoryLayout layout, LicenseAudits audits)
    {
        ArgumentNullException.ThrowIfNull(layout);
        const string name = "Notices: THIRD-PARTY-NOTICES-WebGpuCrates.txt is up to date";
        var problems = new List<string>();
        var expected = Generate(layout, audits);
        var shown = layout.Relative(layout.NoticesFile);
        if (!File.Exists(layout.NoticesFile))
        {
            problems.Add($"{shown} does not exist; run `LibraryChecker notices --write`");
        }
        else
        {
            var actual = File.ReadAllText(layout.NoticesFile, Encoding.UTF8).Replace("\r\n", "\n", StringComparison.Ordinal);
            if (!string.Equals(actual, expected, StringComparison.Ordinal))
            {
                problems.Add($"{shown} differs from what the vendored crates produce; run `LibraryChecker notices --write` and review the change");
            }
        }

        return new CheckResult(name, problems);
    }

    /// <summary>
    /// Writes the notices file (line feeds, UTF-8 without a byte-order mark).
    /// </summary>
    /// <param name="layout">The repository's folders.</param>
    /// <param name="audits">The license audits.</param>
    /// <returns>True when the file's content changed.</returns>
    public static bool Write(RepositoryLayout layout, LicenseAudits audits)
    {
        ArgumentNullException.ThrowIfNull(layout);
        var text = Generate(layout, audits);
        var existing = File.Exists(layout.NoticesFile) ? File.ReadAllText(layout.NoticesFile, Encoding.UTF8) : null;
        if (string.Equals(existing, text, StringComparison.Ordinal))
        {
            return false;
        }

        Directory.CreateDirectory(layout.NoticesFolder);
        File.WriteAllText(layout.NoticesFile, text, new UTF8Encoding(false));
        return true;
    }

    /// <summary>
    /// Builds a package's notices text (<c>THIRD-PARTY-NOTICES-WebGpuGraphicsCrates.txt</c>,
    /// <c>THIRD-PARTY-NOTICES-WebGpuAnimationCrates.txt</c>): its hand-written header (the
    /// first-party crate's own outside sources and the credits for its crates), then Part 1, the
    /// package's OWN third-party crates that are compiled into an application that uses its crate,
    /// and Part 2, its own crates that are carried only. The crates it shares with the WebGpu
    /// package and the platform are left out: their notices come with those.
    /// </summary>
    /// <param name="layout">The repository's folders.</param>
    /// <param name="set">The package's split crate set.</param>
    /// <param name="audits">The crate set's license audits.</param>
    /// <exception cref="FileNotFoundException">The notices header file is missing.</exception>
    public static string GenerateBundle(RepositoryLayout layout, BundleCrateSet set, LicenseAudits audits)
    {
        ArgumentNullException.ThrowIfNull(layout);
        ArgumentNullException.ThrowIfNull(set);
        ArgumentNullException.ThrowIfNull(audits);
        var package = set.Package;
        var own = new HashSet<string>(set.Own.Select(p => p.Name + "\n" + p.Version), StringComparer.Ordinal);
        bool IsOwn(CrateLicenseFinding f) => own.Contains(f.CrateName + "\n" + f.Version);
        var linked = audits.Application.Findings.Where(IsOwn).ToList();
        var linkedIds = new HashSet<string>(linked.Select(f => f.PackageId), StringComparer.Ordinal);
        var carriedOnly = audits.Vendored.Findings.Where(f => IsOwn(f) && !linkedIds.Contains(f.PackageId)).ToList();
        var sections = new[]
        {
            new ThirdPartyNoticesSection
            {
                Title = $"PART 1 - the crates compiled into a platform application that uses {package.CrateName}",
                Introduction = "These crates are linked into the WebAssembly file of a platform application whose frontend uses\n"
                    + $"the crate {package.CrateName}. They are the third-party crates that only the\n"
                    + $"CodeBrix.WebPlatform.WebGpu.{package.Label} package carries; their source is in its crate archive, and\n"
                    + $"in the repository at {layout.Relative(package.Vendor(layout))}, unmodified.",
                Report = new LicenseAuditReport { Findings = linked, Policy = audits.Application.Policy, Scope = audits.Application.Scope },
                LockFile = set.LockFile,
            },
            new ThirdPartyNoticesSection
            {
                Title = "PART 2 - the crates that are carried but not compiled into an application",
                Introduction = $"These crates are named by the {package.Label} crate set's Cargo.lock and carried by the package so\n"
                    + "that cargo can resolve a frontend offline, unmodified. None of their code is in an\n"
                    + "application's WebAssembly file.",
                Report = new LicenseAuditReport { Findings = carriedOnly, Policy = audits.Vendored.Policy, Scope = audits.Vendored.Scope },
                LockFile = set.LockFile,
            },
        };
        return ThirdPartyNoticesGenerator.Generate(sections, new ThirdPartyNoticesOptions { Header = ReadHeader(package.NoticesHeaderFile(layout)) });
    }

    /// <summary>
    /// Compares a package's generated notices with the committed file.
    /// </summary>
    /// <param name="layout">The repository's folders.</param>
    /// <param name="set">The package's split crate set.</param>
    /// <param name="audits">The crate set's license audits.</param>
    public static CheckResult CheckBundle(RepositoryLayout layout, BundleCrateSet set, LicenseAudits audits)
    {
        ArgumentNullException.ThrowIfNull(layout);
        ArgumentNullException.ThrowIfNull(set);
        var package = set.Package;
        var name = $"Notices: {package.NoticesFileName} is up to date";
        var problems = new List<string>();
        var expected = GenerateBundle(layout, set, audits);
        var file = package.NoticesFile(layout);
        var shown = layout.Relative(file);
        if (!File.Exists(file))
        {
            problems.Add($"{shown} does not exist; run `LibraryChecker notices --write`");
        }
        else
        {
            var actual = File.ReadAllText(file, Encoding.UTF8).Replace("\r\n", "\n", StringComparison.Ordinal);
            if (!string.Equals(actual, expected, StringComparison.Ordinal))
            {
                problems.Add($"{shown} differs from what the header and the {package.Label} package's crates produce; run `LibraryChecker notices --write` and review the change");
            }
        }

        return new CheckResult(name, problems);
    }

    /// <summary>
    /// Writes a package's notices file (line feeds, UTF-8 without a byte-order mark).
    /// </summary>
    /// <param name="layout">The repository's folders.</param>
    /// <param name="set">The package's split crate set.</param>
    /// <param name="audits">The crate set's license audits.</param>
    /// <returns>True when the file's content changed.</returns>
    public static bool WriteBundle(RepositoryLayout layout, BundleCrateSet set, LicenseAudits audits)
    {
        ArgumentNullException.ThrowIfNull(layout);
        ArgumentNullException.ThrowIfNull(set);
        var text = GenerateBundle(layout, set, audits);
        var file = set.Package.NoticesFile(layout);
        var existing = File.Exists(file) ? File.ReadAllText(file, Encoding.UTF8) : null;
        if (string.Equals(existing, text, StringComparison.Ordinal))
        {
            return false;
        }

        Directory.CreateDirectory(set.Package.NoticesFolder(layout));
        File.WriteAllText(file, text, new UTF8Encoding(false));
        return true;
    }
}
