using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using System.Text;
using System.Text.RegularExpressions;
using System.Xml;
using System.Xml.Linq;

namespace CodeBrix.WebPlatform.WebGpu.LibraryChecker.Checks;

/// <summary>
/// The HAND-WRITTEN header of the Graphics package's notices
/// (<c>src/CodeBrix.WebPlatform.WebGpu.Graphics/Rust/notices/notices-header-WebGpuGraphicsCrates.txt</c>,
/// the opening of the GENERATED THIRD-PARTY-NOTICES-WebGpuGraphicsCrates.txt, which the notices
/// group writes from it and the licence audit of the package's own crates): the package's crate is
/// this repository's own code, but three tone-mapping functions of its WGSL (and of its CPU
/// ToneMapper::map) and its Radiance .hdr decoder follow outside sources, and the header must carry
/// their entries; and it credits the glTF crates the package vendors. Checked: the header exists;
/// it names the crate and says it is this repository's own code under Apache-2.0; it has the four
/// entries - aces_fitted (BakingLab's ACES.hlsl by Stephen Hill, MIT, with the MIT notice text of
/// that repository's LICENSE, verbatim), aces_narkowicz (Krzysztof Narkowicz, CC0 elected),
/// pbr_neutral (KhronosGroup/ToneMapping, Apache-2.0) and RadianceHdrDecoder (stb_image.h of
/// nothings/stb, its public-domain alternative elected) - and the glTF part (gltf, gltf-json,
/// gltf-derive with its validator_derive credit, lazy_static, inflections); the generated file
/// carries the same; and the Graphics project packs the generated file beside its archive
/// (tools/webgpu-graphics-crates).
/// </summary>
public static class GraphicsNoticesCheck
{
    /// <summary>The check's name.</summary>
    public const string Name = "Notices: THIRD-PARTY-NOTICES-WebGpuGraphicsCrates.txt is packed, and it and its header have the entries of the Graphics crate's outside sources and of the glTF crates";

    /// <summary>The folder inside the Graphics package that holds the archive and the notices.</summary>
    public const string PackagePath = "tools/webgpu-graphics-crates";

    /// <summary>
    /// The MIT notice of the BakingLab repository (its LICENSE file), which the aces_fitted entry
    /// must carry verbatim (compared with the line breaks and runs of spaces made one space).
    /// </summary>
    public const string BakingLabMitNotice = """
        MIT License

        Copyright (c) 2016 MJP

        Permission is hereby granted, free of charge, to any person obtaining a copy
        of this software and associated documentation files (the "Software"), to deal
        in the Software without restriction, including without limitation the rights
        to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
        copies of the Software, and to permit persons to whom the Software is
        furnished to do so, subject to the following conditions:

        The above copyright notice and this permission notice shall be included in all
        copies or substantial portions of the Software.

        THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
        IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
        FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
        AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
        LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
        OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
        SOFTWARE.
        """;

    /// <summary>What each entry must say: the function, then the words that identify its source and licence.</summary>
    public static readonly IReadOnlyList<(string Function, IReadOnlyList<string> Words)> Entries = new (string, IReadOnlyList<string>)[]
    {
        ("aces_fitted", new[] { "TheRealMJP/BakingLab", "ACES.hlsl", "Stephen Hill", "MIT" }),
        ("aces_narkowicz", new[] { "Krzysztof Narkowicz", "ACES Filmic Tone Mapping Curve", "CC0" }),
        ("pbr_neutral", new[] { "KhronosGroup/ToneMapping", "pbrNeutral.glsl", "Apache-2.0" }),
        ("RadianceHdrDecoder", new[] { "nothings/stb", "stb_image.h", "Public Domain" }),
    };

    /// <summary>
    /// The words the glTF part of the header must hold: the crates the package vendors for its glTF
    /// loader, their project's copyright line, and gltf-derive's own credit to validator_derive.
    /// </summary>
    public static readonly IReadOnlyList<string> GltfWords = new[]
    {
        "gltf", "gltf-json", "gltf-derive", "The gltf Library Developers", "validator_derive", "https://github.com/Keats/validator",
        "lazy_static", "inflections",
    };

    /// <summary>Runs the check on the repository's files.</summary>
    /// <param name="layout">The repository's folders.</param>
    public static CheckResult Run(RepositoryLayout layout)
    {
        ArgumentNullException.ThrowIfNull(layout);
        var header = File.Exists(layout.GraphicsNoticesHeaderFile) ? File.ReadAllText(layout.GraphicsNoticesHeaderFile, Encoding.UTF8) : null;
        var notices = File.Exists(layout.GraphicsNoticesFile) ? File.ReadAllText(layout.GraphicsNoticesFile, Encoding.UTF8) : null;
        var project = File.Exists(layout.GraphicsProjectFile) ? File.ReadAllText(layout.GraphicsProjectFile) : null;
        var noticesRelative = Path.GetRelativePath(Path.GetDirectoryName(layout.GraphicsProjectFile), layout.GraphicsNoticesFile);
        return Check(header, layout.Relative(layout.GraphicsNoticesHeaderFile), notices, layout.Relative(layout.GraphicsNoticesFile), project, layout.Relative(layout.GraphicsProjectFile), noticesRelative);
    }

    /// <summary>
    /// Checks the header text, the generated notices text and the project file that packs the
    /// generated notices.
    /// </summary>
    /// <param name="headerText">The hand-written header, or <c>null</c> when it does not exist.</param>
    /// <param name="headerShown">Its path as the messages show it.</param>
    /// <param name="noticesText">The generated notices file, or <c>null</c> when it does not exist.</param>
    /// <param name="noticesShown">Its path as the messages show it.</param>
    /// <param name="projectText">The Graphics project file, or <c>null</c> when it does not exist.</param>
    /// <param name="projectShown">Its path as the messages show it.</param>
    /// <param name="noticesFromProject">The generated notices file's path relative to the project's folder.</param>
    public static CheckResult Check(string headerText, string headerShown, string noticesText, string noticesShown, string projectText, string projectShown, string noticesFromProject)
    {
        ArgumentNullException.ThrowIfNull(noticesFromProject);
        var problems = new List<string>();
        if (headerText == null)
        {
            problems.Add($"{headerShown} does not exist; it is hand-written - write it with the crate's own-code statement, the entries of its outside sources and the glTF crates' part");
        }
        else
        {
            problems.AddRange(CheckText(headerText, headerShown));
        }

        if (noticesText == null)
        {
            problems.Add($"{noticesShown} does not exist; it is generated from the header - run `LibraryChecker notices --write`");
        }
        else
        {
            problems.AddRange(CheckText(noticesText, noticesShown));
        }

        if (projectText == null)
        {
            problems.Add($"{projectShown} does not exist");
        }
        else if (!PacksNotices(projectText, noticesFromProject))
        {
            problems.Add($"{projectShown} does not pack {noticesFromProject.Replace('/', '\\')}: add <None Include=\"{noticesFromProject.Replace('/', '\\')}\" Pack=\"true\" PackagePath=\"{PackagePath.Replace('/', '\\')}\" ... /> so that the notices travel beside the archive");
        }

        return new CheckResult(Name, problems);
    }

    private static IEnumerable<string> CheckText(string text, string shown)
    {
        var flat = Flatten(text);
        foreach (var required in new[] { "codebrix-webgpu-graphics", "Apache-2.0" })
        {
            if (!flat.Contains(required, StringComparison.Ordinal))
            {
                yield return $"{shown} does not say \"{required}\"; it states that the crate codebrix-webgpu-graphics is this repository's own code under its Apache-2.0 LICENSE";
            }
        }

        foreach (var (function, words) in Entries)
        {
            if (!flat.Contains(function, StringComparison.Ordinal))
            {
                yield return $"{shown} has no entry for {function} (code of the crate that follows an outside source); add one with its source and licence";
                continue;
            }

            foreach (var word in words.Where(w => !flat.Contains(w, StringComparison.Ordinal)))
            {
                yield return $"{shown}: the entry for {function} must name \"{word}\" (its source and licence)";
            }
        }

        if (!flat.Contains(Flatten(BakingLabMitNotice), StringComparison.Ordinal))
        {
            yield return $"{shown} does not carry the MIT notice of the BakingLab repository's LICENSE verbatim (for aces_fitted); copy the text from that LICENSE file";
        }

        foreach (var word in GltfWords.Where(w => !flat.Contains(w, StringComparison.Ordinal)))
        {
            yield return $"{shown} does not say \"{word}\"; its glTF part names the crates the package vendors (gltf, gltf-json, gltf-derive, lazy_static, inflections), their copyright line and gltf-derive's credit to validator_derive";
        }
    }

    private static bool PacksNotices(string projectText, string noticesFromProject)
    {
        XDocument document;
        try
        {
            document = XDocument.Parse(projectText);
        }
        catch (XmlException)
        {
            return false;
        }

        static string Normal(string path) => (path ?? string.Empty).Trim().Replace('\\', '/').TrimEnd('/');
        return document.Descendants().Where(e => e.Name.LocalName == "None").Any(e =>
            string.Equals(Normal((string)e.Attribute("Include")), Normal(noticesFromProject), StringComparison.Ordinal)
            && string.Equals((string)e.Attribute("Pack"), "true", StringComparison.OrdinalIgnoreCase)
            && string.Equals(Normal((string)e.Attribute("PackagePath")), PackagePath, StringComparison.Ordinal));
    }

    private static string Flatten(string text) => Regex.Replace(text, @"\s+", " ").Trim();
}
