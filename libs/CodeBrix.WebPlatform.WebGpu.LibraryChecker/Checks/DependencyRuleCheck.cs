using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using System.Text.RegularExpressions;
using System.Xml;
using System.Xml.Linq;

namespace CodeBrix.WebPlatform.WebGpu.LibraryChecker.Checks;

/// <summary>
/// THE DEPENDENCY RULE of this repository (the owner's): the Graphics and the Animation packages
/// both build on the WebGpu package and NEVER on each other, and each depends on Core and the
/// WebGpu package ONLY. For each of the two it checks the library's project file (a
/// ProjectReference to the WebGpu project; no reference of any kind to the other library; no
/// reference at all beyond Core's package and the WebGpu project - so, for example, a glTF or
/// imaging library stays in the application that converts its models) and its first-party crate (its Cargo.toml names codebrix-webgpu among its
/// dependencies, and neither the manifest nor the crate's Rust source names the other package's
/// crate). Both crates must exist.
/// </summary>
public static class DependencyRuleCheck
{
    /// <summary>The WebGpu library's project file name, which both libraries reference.</summary>
    public const string WebGpuProjectFileName = "CodeBrix.WebPlatform.WebGpu.csproj";

    /// <summary>The one package, besides the WebGpu project, that the libraries may reference.</summary>
    public const string CorePackageId = "CodeBrix.WebPlatform.Core.ApacheLicenseForever";

    /// <summary>The crate both first-party crates of the libraries depend on.</summary>
    public const string WebGpuCrateName = "codebrix-webgpu";

    /// <summary>One side of the rule: a library, its crate, and what it must never name.</summary>
    /// <param name="Library">The library's short name, as the messages show it (Graphics, Animation).</param>
    /// <param name="ProjectFile">The library's project file.</param>
    /// <param name="CrateFolder">The library's crate folder.</param>
    /// <param name="CrateRequired">True when the crate must exist.</param>
    /// <param name="OtherLibrary">The other library's short name.</param>
    /// <param name="OtherProjectName">The other library's project (assembly) name.</param>
    /// <param name="OtherCrateName">The other library's crate name.</param>
    public sealed record Side(string Library, string ProjectFile, string CrateFolder, bool CrateRequired, string OtherLibrary, string OtherProjectName, string OtherCrateName);

    /// <summary>The two sides of the rule in this repository: Graphics, then Animation.</summary>
    /// <param name="layout">The repository's folders.</param>
    public static IReadOnlyList<Side> Sides(RepositoryLayout layout)
    {
        ArgumentNullException.ThrowIfNull(layout);
        return new[]
        {
            new Side("Graphics", layout.GraphicsProjectFile, layout.GraphicsCrate, true, "Animation", "CodeBrix.WebPlatform.WebGpu.Animation", "codebrix-webgpu-animation"),
            new Side("Animation", layout.AnimationProjectFile, layout.AnimationCrate, true, "Graphics", "CodeBrix.WebPlatform.WebGpu.Graphics", "codebrix-webgpu-graphics"),
        };
    }

    /// <summary>The check's name for one side.</summary>
    /// <param name="side">The side.</param>
    public static string NameFor(Side side)
    {
        ArgumentNullException.ThrowIfNull(side);
        return $"Dependency rule: {side.Library} depends on Core and the WebGpu project only, on the WebGpu crate, and names nothing of {side.OtherLibrary}";
    }

    /// <summary>Runs the check for both sides of the rule.</summary>
    /// <param name="layout">The repository's folders.</param>
    /// <returns>One result per side.</returns>
    public static IReadOnlyList<CheckResult> Run(RepositoryLayout layout)
    {
        ArgumentNullException.ThrowIfNull(layout);
        return Sides(layout).Select(side => Check(side, layout.Relative)).ToList();
    }

    /// <summary>Checks one side of the rule, reading its files.</summary>
    /// <param name="side">The side.</param>
    /// <param name="shown">How the messages show a path.</param>
    public static CheckResult Check(Side side, Func<string, string> shown)
    {
        ArgumentNullException.ThrowIfNull(side);
        ArgumentNullException.ThrowIfNull(shown);
        var problems = new List<string>();
        var notes = new List<string>();
        if (!File.Exists(side.ProjectFile))
        {
            problems.Add($"{shown(side.ProjectFile)} does not exist; the {side.Library} library's project file is checked for the dependency rule");
        }
        else
        {
            problems.AddRange(CheckProject(File.ReadAllText(side.ProjectFile), shown(side.ProjectFile), side));
        }

        var manifest = Path.Combine(side.CrateFolder, "Cargo.toml");
        if (!File.Exists(manifest))
        {
            if (side.CrateRequired)
            {
                problems.Add($"{shown(manifest)} does not exist; the {side.Library} package's archive carries its first-party crate from {shown(side.CrateFolder)}");
            }
            else
            {
                notes.Add($"{shown(side.CrateFolder)} does not exist yet; its crate is checked once it does");
            }
        }
        else
        {
            problems.AddRange(CheckManifest(File.ReadAllText(manifest), shown(manifest), side));
            var source = Path.Combine(side.CrateFolder, "src");
            if (Directory.Exists(source))
            {
                foreach (var file in Directory.GetFiles(source, "*.rs", SearchOption.AllDirectories).OrderBy(f => f, StringComparer.Ordinal))
                {
                    problems.AddRange(CheckRustSource(File.ReadAllText(file), shown(file), side));
                }
            }
        }

        return new CheckResult(NameFor(side), problems, notes);
    }

    /// <summary>
    /// Checks a library's project file: a ProjectReference to the WebGpu project, no
    /// ProjectReference, PackageReference or Reference that names the other library, and no
    /// reference beyond Core's package and the WebGpu project.
    /// </summary>
    /// <param name="projectText">The project file.</param>
    /// <param name="projectShown">Its path as the messages show it.</param>
    /// <param name="side">The side.</param>
    public static IReadOnlyList<string> CheckProject(string projectText, string projectShown, Side side)
    {
        ArgumentNullException.ThrowIfNull(projectText);
        ArgumentNullException.ThrowIfNull(side);
        XDocument document;
        try
        {
            document = XDocument.Parse(projectText);
        }
        catch (XmlException exception)
        {
            return new[] { $"{projectShown} is not well-formed XML ({exception.Message}); fix the project file" };
        }

        var problems = new List<string>();
        var references = document.Descendants()
            .Where(e => e.Name.LocalName is "ProjectReference" or "PackageReference" or "Reference")
            .Select(e => (Kind: e.Name.LocalName, Include: ((string)e.Attribute("Include") ?? string.Empty).Trim()))
            .ToList();
        foreach (var (kind, include) in references.Where(r => r.Include.Contains(side.OtherProjectName, StringComparison.OrdinalIgnoreCase)))
        {
            problems.Add($"{projectShown}: <{kind} Include=\"{include}\"> names the {side.OtherLibrary} library; the {side.Library} and {side.OtherLibrary} packages never depend on each other - remove the reference (shared code belongs in the WebGpu library)");
        }

        foreach (var (kind, include) in references.Where(r => !r.Include.Contains(side.OtherProjectName, StringComparison.OrdinalIgnoreCase) && !IsAllowed(r.Kind, r.Include)))
        {
            problems.Add($"{projectShown}: <{kind} Include=\"{include}\"> is neither {CorePackageId} nor the WebGpu project; the {side.Library} package depends on Core and the WebGpu package ONLY (the owner's rule) - remove it (a glTF, imaging or other library belongs in the application that uses the package)");
        }

        if (!references.Any(r => r.Kind == "ProjectReference" && string.Equals(r.Include.Replace('\\', '/').Split('/').Last(), WebGpuProjectFileName, StringComparison.OrdinalIgnoreCase)))
        {
            problems.Add($"{projectShown}: no <ProjectReference Include=\"..\\CodeBrix.WebPlatform.WebGpu\\{WebGpuProjectFileName}\" />; the {side.Library} package depends on the WebGpu package (its crates come from the WebGpu package's bundle) - add the reference");
        }

        return problems;
    }

    /// <summary>True for the two references a library may have: Core's package and the WebGpu project.</summary>
    private static bool IsAllowed(string kind, string include)
        => (kind == "PackageReference" && string.Equals(include, CorePackageId, StringComparison.OrdinalIgnoreCase))
            || (kind == "ProjectReference" && string.Equals(include.Replace('\\', '/').Split('/').Last(), WebGpuProjectFileName, StringComparison.OrdinalIgnoreCase));

    /// <summary>
    /// Checks a library's crate manifest: codebrix-webgpu among its [dependencies], and the other
    /// library's crate named nowhere outside comments.
    /// </summary>
    /// <param name="manifestText">The crate's Cargo.toml.</param>
    /// <param name="manifestShown">Its path as the messages show it.</param>
    /// <param name="side">The side.</param>
    public static IReadOnlyList<string> CheckManifest(string manifestText, string manifestShown, Side side)
    {
        ArgumentNullException.ThrowIfNull(manifestText);
        ArgumentNullException.ThrowIfNull(side);
        var problems = new List<string>();
        var other = new Regex(Regex.Escape(side.OtherCrateName) + "|" + Regex.Escape(side.OtherCrateName.Replace('-', '_')), RegexOptions.CultureInvariant);
        var dependency = new Regex(@"^(?<key>[A-Za-z0-9_-]+)\s*=", RegexOptions.CultureInvariant);
        var renamed = new Regex(@"\bpackage\s*=\s*""(?<name>[^""]+)""", RegexOptions.CultureInvariant);
        var namesWebGpu = false;
        string section = null;
        var number = 0;
        foreach (var raw in manifestText.Split('\n'))
        {
            number++;
            var line = StripComment(raw).Trim();
            if (line.Length == 0)
            {
                continue;
            }

            if (other.IsMatch(line))
            {
                problems.Add($"{manifestShown}:{number}: `{line}` names the {side.OtherLibrary} package's crate {side.OtherCrateName}; the {side.Library} crate never depends on it - remove it");
            }

            if (line.StartsWith('['))
            {
                section = line.Trim('[', ']').Trim();
                continue;
            }

            if (section != "dependencies")
            {
                continue;
            }

            var key = dependency.Match(line);
            var package = renamed.Match(line);
            if ((key.Success && key.Groups["key"].Value == WebGpuCrateName && !package.Success) || (package.Success && package.Groups["name"].Value == WebGpuCrateName))
            {
                namesWebGpu = true;
            }
        }

        if (!namesWebGpu)
        {
            problems.Add($"{manifestShown}: the [dependencies] table does not name {WebGpuCrateName}; the {side.Library} crate builds on the WebGpu package's crate - add `{WebGpuCrateName} = \"=<its version>\"`");
        }

        return problems;
    }

    /// <summary>Checks one Rust source file of a library's crate: it never names the other library's crate.</summary>
    /// <param name="sourceText">The file.</param>
    /// <param name="sourceShown">Its path as the messages show it.</param>
    /// <param name="side">The side.</param>
    public static IReadOnlyList<string> CheckRustSource(string sourceText, string sourceShown, Side side)
    {
        ArgumentNullException.ThrowIfNull(sourceText);
        ArgumentNullException.ThrowIfNull(side);
        var rustName = side.OtherCrateName.Replace('-', '_');
        var problems = new List<string>();
        var number = 0;
        foreach (var line in sourceText.Split('\n'))
        {
            number++;
            if (line.Contains(rustName, StringComparison.Ordinal))
            {
                problems.Add($"{sourceShown}:{number}: names {rustName}; the {side.Library} crate never uses the {side.OtherLibrary} package's crate - remove it");
            }
        }

        return problems;
    }

    private static string StripComment(string line)
    {
        var inString = false;
        for (var i = 0; i < line.Length; i++)
        {
            if (line[i] == '"' && (i == 0 || line[i - 1] != '\\'))
            {
                inString = !inString;
            }
            else if (line[i] == '#' && !inString)
            {
                return line.Substring(0, i);
            }
        }

        return line;
    }
}
