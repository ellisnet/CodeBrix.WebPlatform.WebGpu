using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using System.Text.RegularExpressions;
using CodeBrix.WebPlatform.WebGpu.LibraryChecker.Platform;

namespace CodeBrix.WebPlatform.WebGpu.LibraryChecker.Checks;

/// <summary>
/// The first-party crates of this repository: codebrix-webgpu (Rust/codebrix-webgpu of the WebGpu
/// library), which the WebGpu package carries in its crate archive beside the vendored crates, and
/// codebrix-webgpu-graphics (Rust/codebrix-webgpu-graphics of the Graphics library), which the
/// Graphics package carries alone in its own archive, and codebrix-webgpu-animation
/// (Rust/codebrix-webgpu-animation of the Animation library), which the Animation package carries
/// in its own archive beside the third-party crates only it needs. They are OURS: covered by the repository's
/// Apache-2.0 LICENSE, never published, and therefore exempt from the licence audit and absent
/// from the generated third-party notices. What can still go wrong is checked here: each manifest
/// declares the crate as ours; its name is no third-party crate's (nor another first-party
/// crate's); and every dependency is at an EXACT version that the vendored crates or the
/// platform's crates carry - or, for a crate that builds on another first-party crate (the
/// Graphics crate on codebrix-webgpu), that crate at its own version - so that a consumer's
/// offline build resolves it from the one vendor folder.
/// </summary>
public static class FirstPartyCrateCheck
{
    /// <summary>The licence the first-party crate declares: the repository's.</summary>
    public const string License = "Apache-2.0";

    private static readonly Regex Section = new Regex(@"^\[(?<name>[^\[\]]+)\]\s*(#.*)?$", RegexOptions.CultureInvariant);
    private static readonly Regex KeyString = new Regex(@"^(?<key>[A-Za-z0-9_-]+)\s*=\s*""(?<value>[^""]*)""\s*(#.*)?$", RegexOptions.CultureInvariant);
    private static readonly Regex KeyBool = new Regex(@"^(?<key>[A-Za-z0-9_-]+)\s*=\s*(?<value>true|false)\s*(#.*)?$", RegexOptions.CultureInvariant);
    private static readonly Regex KeyTable = new Regex(@"^(?<key>[A-Za-z0-9_-]+)\s*=\s*\{(?<body>.*)\}\s*(#.*)?$", RegexOptions.CultureInvariant);
    private static readonly Regex TableString = new Regex(@"(?<key>[A-Za-z0-9_-]+)\s*=\s*""(?<value>[^""]*)""", RegexOptions.CultureInvariant);
    private static readonly Regex ExactVersion = new Regex(@"^=\s*(?<version>\d+\.\d+\.\d+([-+][0-9A-Za-z.+-]+)?)$", RegexOptions.CultureInvariant);

    /// <summary>The name of the WebGpu package's first-party crate.</summary>
    public const string WebGpuCrateName = "codebrix-webgpu";

    /// <summary>The name of the Graphics package's first-party crate.</summary>
    public const string GraphicsCrateName = "codebrix-webgpu-graphics";

    /// <summary>The name of the Animation package's first-party crate.</summary>
    public const string AnimationCrateName = "codebrix-webgpu-animation";

    /// <summary>
    /// Runs the check on each of the repository's first-party crates (codebrix-webgpu, then
    /// codebrix-webgpu-graphics, then codebrix-webgpu-animation), against the vendored crates and
    /// the platform release this tool references; the Graphics and Animation crates may also name
    /// codebrix-webgpu, at that crate's version, and the Animation crate the crates only the
    /// Animation package vendors (kurbo, peniko).
    /// </summary>
    /// <param name="layout">The repository's folders.</param>
    /// <returns>One result per crate.</returns>
    public static IReadOnlyList<CheckResult> Run(RepositoryLayout layout)
    {
        ArgumentNullException.ThrowIfNull(layout);
        var vendored = CrateSnapshot.FromDirectory(layout.Vendor);
        var platform = PlatformRelease.LoadCrates();
        var webGpuManifest = Path.Combine(layout.FirstPartyCrate, "Cargo.toml");
        var graphicsManifest = Path.Combine(layout.GraphicsCrate, "Cargo.toml");
        var results = new List<CheckResult>();
        string webGpuVersion = null;
        if (!File.Exists(webGpuManifest))
        {
            results.Add(new CheckResult(NameFor(WebGpuCrateName), new[] { $"{layout.Relative(webGpuManifest)} does not exist; the WebGpu package's archive carries the first-party crate from {layout.Relative(layout.FirstPartyCrate)}" }));
        }
        else
        {
            var text = File.ReadAllText(webGpuManifest);
            webGpuVersion = ReadPackageVersion(text);
            results.Add(Check(text, layout.Relative(webGpuManifest), vendored, platform));
        }

        if (!File.Exists(graphicsManifest))
        {
            results.Add(new CheckResult(NameFor(GraphicsCrateName), new[] { $"{layout.Relative(graphicsManifest)} does not exist; the Graphics package's archive carries the first-party crate from {layout.Relative(layout.GraphicsCrate)}" }));
        }
        else
        {
            var firstParty = new Dictionary<string, string>(StringComparer.Ordinal);
            if (webGpuVersion != null)
            {
                firstParty[WebGpuCrateName] = webGpuVersion;
            }

            results.Add(Check(File.ReadAllText(graphicsManifest), layout.Relative(graphicsManifest), vendored, platform, firstParty));
        }

        var animationManifest = Path.Combine(layout.AnimationCrate, "Cargo.toml");
        if (!File.Exists(animationManifest))
        {
            results.Add(new CheckResult(NameFor(AnimationCrateName), new[] { $"{layout.Relative(animationManifest)} does not exist; the Animation package's archive carries the first-party crate from {layout.Relative(layout.AnimationCrate)}" }));
        }
        else
        {
            var firstParty = new Dictionary<string, string>(StringComparer.Ordinal);
            if (webGpuVersion != null)
            {
                firstParty[WebGpuCrateName] = webGpuVersion;
            }

            var animationVendored = Directory.Exists(layout.AnimationVendor) ? CrateSnapshot.Combine(vendored, CrateSnapshot.FromDirectory(layout.AnimationVendor)) : vendored;
            results.Add(Check(File.ReadAllText(animationManifest), layout.Relative(animationManifest), animationVendored, platform, firstParty));
        }

        return results;
    }

    /// <summary>The check's name for one crate.</summary>
    /// <param name="crateName">The crate's name (or how the manifest is shown, when it names none).</param>
    public static string NameFor(string crateName)
        => $"First-party crate: {crateName} is ours, and every dependency is at an exact version the vendored, the platform's or this repository's first-party crates carry";

    /// <summary>
    /// Checks a first-party crate's manifest.
    /// </summary>
    /// <param name="manifestText">The crate's Cargo.toml.</param>
    /// <param name="manifestShown">Its path as the messages show it.</param>
    /// <param name="vendored">This repository's vendored crates.</param>
    /// <param name="platform">The platform's crates.</param>
    /// <param name="firstPartyCrates">The other first-party crates this crate may name, by name,
    /// each with its version (the crate must name exactly that version); <c>null</c> for
    /// none.</param>
    public static CheckResult Check(string manifestText, string manifestShown, CrateSnapshot vendored, CrateSnapshot platform, IReadOnlyDictionary<string, string> firstPartyCrates = null)
    {
        ArgumentNullException.ThrowIfNull(manifestText);
        ArgumentNullException.ThrowIfNull(vendored);
        ArgumentNullException.ThrowIfNull(platform);
        firstPartyCrates ??= new Dictionary<string, string>(StringComparer.Ordinal);
        var problems = new List<string>();
        var package = new Dictionary<string, string>(StringComparer.Ordinal);
        var dependencies = new List<(string Section, string Name, string Version)>();
        string section = null;
        foreach (var raw in manifestText.Split('\n'))
        {
            var line = raw.Trim();
            if (line.Length == 0 || line.StartsWith('#'))
            {
                continue;
            }

            var header = Section.Match(line);
            if (header.Success)
            {
                section = header.Groups["name"].Value.Trim();
                if (section.StartsWith("dependencies.", StringComparison.Ordinal) || section.StartsWith("dev-dependencies.", StringComparison.Ordinal) || section.StartsWith("build-dependencies.", StringComparison.Ordinal))
                {
                    problems.Add($"{manifestShown}: [{section}] - write each dependency on one line of its [dependencies] table (`name = {{ version = \"=x.y.z\", ... }}`), so that the check can read it");
                }

                continue;
            }

            if (section == "package")
            {
                var text = KeyString.Match(line);
                var flag = KeyBool.Match(line);
                if (text.Success)
                {
                    package[text.Groups["key"].Value] = text.Groups["value"].Value;
                }
                else if (flag.Success)
                {
                    package[flag.Groups["key"].Value] = flag.Groups["value"].Value;
                }

                continue;
            }

            if (section == null || !IsDependencyTable(section))
            {
                continue;
            }

            var simple = KeyString.Match(line);
            var table = KeyTable.Match(line);
            if (simple.Success)
            {
                dependencies.Add((section, simple.Groups["key"].Value, simple.Groups["value"].Value));
            }
            else if (table.Success)
            {
                var keys = TableString.Matches(table.Groups["body"].Value).ToDictionary(m => m.Groups["key"].Value, m => m.Groups["value"].Value, StringComparer.Ordinal);
                var name = table.Groups["key"].Value;
                if (keys.ContainsKey("path") || keys.ContainsKey("git") || table.Groups["body"].Value.Contains("workspace", StringComparison.Ordinal))
                {
                    problems.Add($"{manifestShown}: [{section}] {name} is a path, git or workspace dependency; a crate in the archive can name only crates of the vendor folder, by exact version");
                    continue;
                }

                if (keys.TryGetValue("package", out var renamed))
                {
                    name = renamed;
                }

                dependencies.Add((section, name, keys.TryGetValue("version", out var version) ? version : null));
            }
            else
            {
                problems.Add($"{manifestShown}: [{section}] `{line}` - write each dependency on one line, as `name = \"=x.y.z\"` or `name = {{ version = \"=x.y.z\", ... }}`");
            }
        }

        package.TryGetValue("name", out var crateName);
        package.TryGetValue("version", out var crateVersion);
        if (string.IsNullOrEmpty(crateName) || string.IsNullOrEmpty(crateVersion))
        {
            problems.Add($"{manifestShown}: the [package] table names no name and version on lines of their own; the archive folder is named <name>-<version> from them");
        }

        if (!package.TryGetValue("license", out var license) || license != License)
        {
            problems.Add($"{manifestShown}: the crate declares license = \"{license}\"; a crate of this repository declares license = \"{License}\" (the repository's LICENSE)");
        }

        if (!package.TryGetValue("publish", out var publish) || publish != "false")
        {
            problems.Add($"{manifestShown}: the crate does not say publish = false; crates of this repository are never published on crates.io (they travel in the package's archive)");
        }

        if (!string.IsNullOrEmpty(crateName) && (vendored.Find(crateName).Count > 0 || platform.Find(crateName).Count > 0))
        {
            problems.Add($"{manifestShown}: the name {crateName} is a third-party crate's (vendored or the platform's); give the first-party crate a name no other crate uses");
        }

        if (!string.IsNullOrEmpty(crateName) && firstPartyCrates.ContainsKey(crateName))
        {
            problems.Add($"{manifestShown}: the name {crateName} is another first-party crate's; give each first-party crate a name of its own");
        }

        foreach (var (where, name, requirement) in dependencies)
        {
            var exact = requirement == null ? Match.Empty : ExactVersion.Match(requirement.Trim());
            if (!exact.Success)
            {
                problems.Add($"{manifestShown}: [{where}] {name} = \"{requirement}\" - name an EXACT version (\"=x.y.z\") of a crate the vendored or the platform's crates carry");
                continue;
            }

            var version = exact.Groups["version"].Value;
            if (firstPartyCrates.TryGetValue(name, out var firstPartyVersion))
            {
                if (version != firstPartyVersion)
                {
                    problems.Add($"{manifestShown}: [{where}] {name} {version} - the first-party crate {name} of this repository is at {firstPartyVersion}; name that version");
                }

                continue;
            }

            var carried = vendored.Find(name).Concat(platform.Find(name)).Select(c => c.Version).Distinct(StringComparer.Ordinal).ToList();
            if (!carried.Contains(version, StringComparer.Ordinal))
            {
                problems.Add(carried.Count == 0
                    ? $"{manifestShown}: [{where}] {name} {version} - neither the vendored crates nor the platform's carry {name}; add it to the crate set (the owner approves a new crate first) or drop the dependency"
                    : $"{manifestShown}: [{where}] {name} {version} - the vendored or the platform's crates carry {name} {string.Join(", ", carried)}; name that version");
            }
        }

        var notes = new List<string>
        {
            $"{crateName} {crateVersion}: first-party (license {License}, the repository's LICENSE), exempt from the licence audit and not in the generated third-party notices; {dependencies.Count} dependencies, each at an exact carried version",
        };
        return new CheckResult(NameFor(string.IsNullOrEmpty(crateName) ? manifestShown : crateName), problems, notes);
    }

    /// <summary>The version of the [package] table of a Cargo.toml (on a line of its own), or null.</summary>
    /// <param name="manifestText">The Cargo.toml.</param>
    public static string ReadPackageVersion(string manifestText)
    {
        ArgumentNullException.ThrowIfNull(manifestText);
        string section = null;
        foreach (var raw in manifestText.Split('\n'))
        {
            var line = raw.Trim();
            var header = Section.Match(line);
            if (header.Success)
            {
                section = header.Groups["name"].Value.Trim();
                continue;
            }

            var text = KeyString.Match(line);
            if (section == "package" && text.Success && text.Groups["key"].Value == "version")
            {
                return text.Groups["value"].Value;
            }
        }

        return null;
    }

    private static bool IsDependencyTable(string section)
        => section is "dependencies" or "dev-dependencies" or "build-dependencies"
            || (section.StartsWith("target.", StringComparison.Ordinal) && (section.EndsWith(".dependencies", StringComparison.Ordinal) || section.EndsWith(".dev-dependencies", StringComparison.Ordinal) || section.EndsWith(".build-dependencies", StringComparison.Ordinal)));
}
