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
/// The HAND-WRITTEN header of the Animation package's notices
/// (<c>src/CodeBrix.WebPlatform.WebGpu.Animation/Rust/notices/notices-header-WebGpuAnimationCrates.txt</c>,
/// the opening of the generated THIRD-PARTY-NOTICES-WebGpuAnimationCrates.txt): the package's
/// first-party crate is this repository's own code, but its renderer contains code taken from
/// vello (vello_shaders and vello_encoding 0.12.0, Apache-2.0 elected), and inside it about ten
/// lines vello marks as coming from Skia (BSD-3-Clause). Checked: the header exists; it names the
/// crate as this repository's own under Apache-2.0; it has the vello entry (the two crates, the
/// licence elected, the copyright line, the files of ours that hold the material) and the Skia
/// entry with Skia's BSD-3-Clause licence text verbatim; and the Animation project packs the
/// generated file beside its archive (tools/webgpu-animation-crates).
/// </summary>
public static class AnimationNoticesCheck
{
    /// <summary>The check's name.</summary>
    public const string Name = "Notices: the Animation package's notices header has the entries of its crate's outside sources (vello, Skia) and of harfrust's Unicode data, and the notices are packed";

    /// <summary>The folder inside the Animation package that holds the archive and the notices.</summary>
    public const string PackagePath = "tools/webgpu-animation-crates";

    /// <summary>
    /// Skia's licence (the LICENSE file of the Skia repository, BSD-3-Clause), which the Skia entry
    /// must carry verbatim (compared with the line breaks and runs of spaces made one space).
    /// </summary>
    public const string SkiaLicense = """
        Copyright (c) 2011 Google Inc. All rights reserved.

        Redistribution and use in source and binary forms, with or without
        modification, are permitted provided that the following conditions are
        met:

          * Redistributions of source code must retain the above copyright
            notice, this list of conditions and the following disclaimer.

          * Redistributions in binary form must reproduce the above copyright
            notice, this list of conditions and the following disclaimer in
            the documentation and/or other materials provided with the
            distribution.

          * Neither the name of the copyright holder nor the names of its
            contributors may be used to endorse or promote products derived
            from this software without specific prior written permission.

        THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS
        "AS IS" AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT
        LIMITED TO, THE IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR
        A PARTICULAR PURPOSE ARE DISCLAIMED. IN NO EVENT SHALL THE COPYRIGHT
        OWNER OR CONTRIBUTORS BE LIABLE FOR ANY DIRECT, INDIRECT, INCIDENTAL,
        SPECIAL, EXEMPLARY, OR CONSEQUENTIAL DAMAGES (INCLUDING, BUT NOT
        LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR SERVICES; LOSS OF USE,
        DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER CAUSED AND ON ANY
        THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY, OR TORT
        (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE
        OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.
        """;

    /// <summary>
    /// The Unicode licence (UNICODE LICENSE V3) that the header must carry verbatim for the Unicode
    /// data harfrust's compiled tables are generated from (the text the platform's unicode-ident
    /// crate carries as LICENSE-UNICODE; compared with the line breaks and runs of spaces made one
    /// space).
    /// </summary>
    public const string UnicodeLicense = """
        UNICODE LICENSE V3

        COPYRIGHT AND PERMISSION NOTICE

        Copyright © 1991-2023 Unicode, Inc.

        NOTICE TO USER: Carefully read the following legal agreement. BY
        DOWNLOADING, INSTALLING, COPYING OR OTHERWISE USING DATA FILES, AND/OR
        SOFTWARE, YOU UNEQUIVOCALLY ACCEPT, AND AGREE TO BE BOUND BY, ALL OF THE
        TERMS AND CONDITIONS OF THIS AGREEMENT. IF YOU DO NOT AGREE, DO NOT
        DOWNLOAD, INSTALL, COPY, DISTRIBUTE OR USE THE DATA FILES OR SOFTWARE.

        Permission is hereby granted, free of charge, to any person obtaining a
        copy of data files and any associated documentation (the "Data Files") or
        software and any associated documentation (the "Software") to deal in the
        Data Files or Software without restriction, including without limitation
        the rights to use, copy, modify, merge, publish, distribute, and/or sell
        copies of the Data Files or Software, and to permit persons to whom the
        Data Files or Software are furnished to do so, provided that either (a)
        this copyright and permission notice appear with all copies of the Data
        Files or Software, or (b) this copyright and permission notice appear in
        associated Documentation.

        THE DATA FILES AND SOFTWARE ARE PROVIDED "AS IS", WITHOUT WARRANTY OF ANY
        KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF
        MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT OF
        THIRD PARTY RIGHTS.

        IN NO EVENT SHALL THE COPYRIGHT HOLDER OR HOLDERS INCLUDED IN THIS NOTICE
        BE LIABLE FOR ANY CLAIM, OR ANY SPECIAL INDIRECT OR CONSEQUENTIAL DAMAGES,
        OR ANY DAMAGES WHATSOEVER RESULTING FROM LOSS OF USE, DATA OR PROFITS,
        WHETHER IN AN ACTION OF CONTRACT, NEGLIGENCE OR OTHER TORTIOUS ACTION,
        ARISING OUT OF OR IN CONNECTION WITH THE USE OR PERFORMANCE OF THE DATA
        FILES OR SOFTWARE.

        Except as contained in this notice, the name of a copyright holder shall
        not be used in advertising or otherwise to promote the sale, use or other
        dealings in these Data Files or Software without prior written
        authorization of the copyright holder.
        """;

    /// <summary>The words the header must hold: the crate, its licence, the vello entry and the Skia entry.</summary>
    public static readonly IReadOnlyList<string> RequiredWords = new[]
    {
        "codebrix-webgpu-animation", "Apache-2.0",
        "vello_shaders 0.12.0", "vello_encoding 0.12.0", "USED UNDER Apache-2.0", "Copyright 2022 the Vello Authors",
        "composite.wgsl", "paint.wgsl", "paint.rs", "blend.wgsl", "fine.wgsl", "draw_leaf.wgsl", "ramp_cache.rs",
        "Skia", "SkRasterPipeline_opts.h", "BSD-3-Clause",
        "harfrust 0.15.0", "HarfBuzz", "read-fonts", "agl",
    };

    /// <summary>Runs the check on the repository's files.</summary>
    /// <param name="layout">The repository's folders.</param>
    public static CheckResult Run(RepositoryLayout layout)
    {
        ArgumentNullException.ThrowIfNull(layout);
        var header = File.Exists(layout.AnimationNoticesHeaderFile) ? File.ReadAllText(layout.AnimationNoticesHeaderFile, Encoding.UTF8) : null;
        var project = File.Exists(layout.AnimationProjectFile) ? File.ReadAllText(layout.AnimationProjectFile) : null;
        var noticesRelative = Path.GetRelativePath(Path.GetDirectoryName(layout.AnimationProjectFile), layout.AnimationNoticesFile);
        return Check(header, layout.Relative(layout.AnimationNoticesHeaderFile), project, layout.Relative(layout.AnimationProjectFile), noticesRelative);
    }

    /// <summary>
    /// Checks the header text and the project file that packs the generated notices.
    /// </summary>
    /// <param name="headerText">The header file, or <c>null</c> when it does not exist.</param>
    /// <param name="headerShown">Its path as the messages show it.</param>
    /// <param name="projectText">The Animation project file, or <c>null</c> when it does not exist.</param>
    /// <param name="projectShown">Its path as the messages show it.</param>
    /// <param name="noticesFromProject">The generated notices file's path relative to the project's folder.</param>
    public static CheckResult Check(string headerText, string headerShown, string projectText, string projectShown, string noticesFromProject)
    {
        ArgumentNullException.ThrowIfNull(noticesFromProject);
        var problems = new List<string>();
        if (headerText == null)
        {
            problems.Add($"{headerShown} does not exist; it is hand-written - write it with the crate's own-code statement and the vello and Skia entries");
        }
        else
        {
            var flat = Flatten(headerText);
            foreach (var word in RequiredWords.Where(w => !flat.Contains(w, StringComparison.Ordinal)))
            {
                problems.Add($"{headerShown} does not say \"{word}\"; it states that codebrix-webgpu-animation is this repository's own code under Apache-2.0 and lists the vello material (its crates, the licence elected, the copyright line, the files), the Skia function with Skia's licence, and harfrust's Unicode data (with read-fonts' Adobe Glyph List feature, agl, kept off)");
            }

            if (!flat.Contains(Flatten(SkiaLicense), StringComparison.Ordinal))
            {
                problems.Add($"{headerShown} does not carry Skia's licence (the LICENSE file of the Skia repository) verbatim; copy the text from that file");
            }

            if (!flat.Contains(Flatten(UnicodeLicense), StringComparison.Ordinal))
            {
                problems.Add($"{headerShown} does not carry the Unicode licence (UNICODE LICENSE V3) verbatim for the Unicode data compiled into harfrust's tables; copy the text of the LICENSE-UNICODE file of the unicode-ident crate");
            }
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
