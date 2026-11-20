using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using System.Xml;
using System.Xml.Linq;

namespace CodeBrix.WebPlatform.WebGpu.LibraryChecker.Checks;

/// <summary>
/// THE FAMILY RULE (the owner's): the three packages are built and published together from one
/// build, and each pins the packages it depends on EXACTLY. Core is pinned with
/// <c>Version="[...]"</c> (see <see cref="PlatformCheck.CorePin(RepositoryLayout)"/>); the WebGpu
/// package, which the Graphics and Animation packages reach through a ProjectReference, is pinned
/// by the shared targets file <c>src/CodeBrix.WebPlatform.WebGpu/ExactProjectReferencePins.targets</c>,
/// which rewrites the referenced project's version as <c>[x]</c> before the nuspec is written.
/// Checked: the targets file exists and holds that target, run after NuGet's
/// <c>_GetProjectReferenceVersions</c>; each of the two libraries imports it, references the
/// WebGpu project with a ProjectReference and never as a PackageReference.
/// </summary>
public static class FamilyPinCheck
{
    /// <summary>The check's name.</summary>
    public const string Name = "Family rule: the Graphics and Animation packages pin the WebGpu package of the same build exactly ([x])";

    /// <summary>The shared targets file's name.</summary>
    public const string TargetsFileName = "ExactProjectReferencePins.targets";

    /// <summary>The target the shared file must hold.</summary>
    public const string TargetName = "_PinProjectReferenceVersionsExactly";

    /// <summary>NuGet's target the pin must run after.</summary>
    public const string AfterTarget = "_GetProjectReferenceVersions";

    /// <summary>The WebGpu package's id.</summary>
    public const string WebGpuPackageId = "CodeBrix.WebPlatform.WebGpu.ApacheLicenseForever";

    /// <summary>Runs the check on the repository's project files.</summary>
    /// <param name="layout">The repository's folders.</param>
    public static CheckResult Run(RepositoryLayout layout)
    {
        ArgumentNullException.ThrowIfNull(layout);
        var targets = Path.Combine(Path.GetDirectoryName(layout.WebGpuProjectFile), TargetsFileName);
        var projects = new[] { layout.GraphicsProjectFile, layout.AnimationProjectFile }
            .Select(p => (Shown: layout.Relative(p), Text: File.Exists(p) ? File.ReadAllText(p) : null))
            .ToList();
        return Check(File.Exists(targets) ? File.ReadAllText(targets) : null, layout.Relative(targets), projects);
    }

    /// <summary>Checks the shared targets file and the projects that must import it.</summary>
    /// <param name="targetsText">The shared targets file, or <c>null</c> when it does not exist.</param>
    /// <param name="targetsShown">Its path as the messages show it.</param>
    /// <param name="projects">Each library project as the messages show it, with its text (<c>null</c> when it does not exist).</param>
    public static CheckResult Check(string targetsText, string targetsShown, IReadOnlyList<(string Shown, string Text)> projects)
    {
        ArgumentNullException.ThrowIfNull(projects);
        var problems = new List<string>();
        if (targetsText == null)
        {
            problems.Add($"{targetsShown} does not exist; it pins the WebGpu project reference exactly in the Graphics and Animation packages - restore it");
        }
        else
        {
            var document = Parse(targetsText, targetsShown, problems);
            var target = document?.Descendants().FirstOrDefault(e => e.Name.LocalName == "Target" && (string)e.Attribute("Name") == TargetName);
            if (document != null && target == null)
            {
                problems.Add($"{targetsShown} has no <Target Name=\"{TargetName}\">; it rewrites ProjectVersion of _ProjectReferencesWithVersions as [x]");
            }
            else if (target != null)
            {
                if (!string.Equals(((string)target.Attribute("AfterTargets") ?? string.Empty).Trim(), AfterTarget, StringComparison.Ordinal))
                {
                    problems.Add($"{targetsShown}: <Target Name=\"{TargetName}\"> must run with AfterTargets=\"{AfterTarget}\" (NuGet's pack targets fill _ProjectReferencesWithVersions there)");
                }

                if (!target.Descendants().Any(e => e.Name.LocalName == "ProjectVersion" && e.Value.Trim() == "[%(_ProjectReferencesWithVersions.ProjectVersion)]"))
                {
                    problems.Add($"{targetsShown}: <Target Name=\"{TargetName}\"> must write <ProjectVersion>[%(_ProjectReferencesWithVersions.ProjectVersion)]</ProjectVersion>");
                }
            }
        }

        foreach (var (shown, text) in projects)
        {
            if (text == null)
            {
                problems.Add($"{shown} does not exist");
                continue;
            }

            var document = Parse(text, shown, problems);
            if (document == null)
            {
                continue;
            }

            var elements = document.Descendants().ToList();
            if (!elements.Any(e => e.Name.LocalName == "Import" && ((string)e.Attribute("Project") ?? string.Empty).Replace('\\', '/').EndsWith("/" + TargetsFileName, StringComparison.Ordinal)))
            {
                problems.Add($"{shown} does not import {TargetsFileName}; add <Import Project=\"..\\CodeBrix.WebPlatform.WebGpu\\{TargetsFileName}\" /> so that its WebGpu dependency is packed as [x]");
            }

            if (!elements.Any(e => e.Name.LocalName == "ProjectReference" && ((string)e.Attribute("Include") ?? string.Empty).Replace('\\', '/').EndsWith("/" + DependencyRuleCheck.WebGpuProjectFileName, StringComparison.Ordinal)))
            {
                problems.Add($"{shown} has no ProjectReference to {DependencyRuleCheck.WebGpuProjectFileName}; the packages of one build depend on each other through project references");
            }

            if (elements.Any(e => e.Name.LocalName == "PackageReference" && string.Equals((string)e.Attribute("Include"), WebGpuPackageId, StringComparison.OrdinalIgnoreCase)))
            {
                problems.Add($"{shown} references {WebGpuPackageId} as a PackageReference; reference the WebGpu project instead, so that the packages of one build pin each other");
            }
        }

        return new CheckResult(Name, problems, new[] { $"{projects.Count} library projects import {TargetsFileName}; their nuspecs name the WebGpu package as [<the build's version>]" });
    }

    private static XDocument Parse(string text, string shown, List<string> problems)
    {
        try
        {
            return XDocument.Parse(text);
        }
        catch (XmlException exception)
        {
            problems.Add($"{shown} is not well-formed XML ({exception.Message}); fix it");
            return null;
        }
    }
}
