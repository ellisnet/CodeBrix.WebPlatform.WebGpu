using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using System.Text;

namespace CodeBrix.WebPlatform.WebGpu.LibraryChecker.Checks;

/// <summary>
/// The notices of the GraphicsDemo sample's MODELS, ENVIRONMENT PANORAMAS and FONTS
/// (samples/GraphicsDemo/THIRD-PARTY-NOTICES.txt): the sample carries third-party model packs in
/// backend/GraphicsDemo.Assets/Models/&lt;pack&gt;/, HDR panoramas in
/// backend/GraphicsDemo.Assets/Environments/&lt;pack&gt;/ and fonts in
/// backend/GraphicsDemo.Assets/Fonts/&lt;pack&gt;/, and an asset is carried only with its pack's
/// licence file and a notices entry (the owner's rule). Checked, for every pack folder of the
/// three: it holds a licence file (a file whose name starts with LICENSE - or, in the fonts
/// folder, with OFL, the name an OFL font's original licence file has); and the sample's notices
/// file exists and names the pack folder, the path of each of its licence files and every asset
/// file of the pack (a model, a panorama or a font, by its name without the extension). The
/// sample is never packed; this keeps its records complete.
/// <para>
/// THE OWNER'S OWN PACKS are the exception, named one by one in <see cref="OwnersOwnPacks" />: a
/// pack the repository's owner made himself is not third-party material (his rule: no
/// provenance, attribution or notice is needed for his own work), so the check demands neither a
/// licence file nor a notices entry for it, and reports it as a note instead. A pack is added to
/// that list only on the owner's word.
/// </para>
/// </summary>
public static class SampleNoticesCheck
{
    /// <summary>The check's name.</summary>
    public const string Name = "Notices: the GraphicsDemo sample's THIRD-PARTY-NOTICES.txt names every model, panorama and font pack, its licence file and its files";

    /// <summary>The sample folder, from the repository root.</summary>
    public const string SampleFolder = "samples/GraphicsDemo";

    /// <summary>The folder of the model packs, from the sample folder.</summary>
    public const string ModelsFolder = "backend/GraphicsDemo.Assets/Models";

    /// <summary>The folder of the environment panorama packs, from the sample folder.</summary>
    public const string EnvironmentsFolder = "backend/GraphicsDemo.Assets/Environments";

    /// <summary>The folder of the font packs, from the sample folder.</summary>
    public const string FontsFolder = "backend/GraphicsDemo.Assets/Fonts";

    /// <summary>
    /// The pack folders that are the repository owner's own work (no licence file and no notices
    /// entry demanded): crystals-and-avian, the Avian character.
    /// </summary>
    public static readonly IReadOnlyList<string> OwnersOwnPacks = new[] { "crystals-and-avian" };

    /// <summary>The extensions of the files counted as assets: models and panoramas (textures and licence files are not).</summary>
    public static readonly IReadOnlyList<string> ModelExtensions = new[] { ".glb", ".gltf", ".hdr" };

    /// <summary>The extensions of the files counted as fonts in the fonts folder.</summary>
    public static readonly IReadOnlyList<string> FontExtensions = new[] { ".ttf", ".otf" };

    /// <summary>One pack: its folder name, its files (relative to the pack folder, with forward slashes) and the folder it is in.</summary>
    /// <param name="Name">The pack folder's name.</param>
    /// <param name="Files">The files of the pack.</param>
    /// <param name="Folder">The folder of packs it is in, from the sample folder (<see cref="ModelsFolder" />, <see cref="EnvironmentsFolder" /> or <see cref="FontsFolder" />).</param>
    public sealed record Pack(string Name, IReadOnlyList<string> Files, string Folder = ModelsFolder);

    /// <summary>Runs the check on the repository's files.</summary>
    /// <param name="layout">The repository's folders.</param>
    public static CheckResult Run(RepositoryLayout layout)
    {
        ArgumentNullException.ThrowIfNull(layout);
        var sample = layout.Combine(SampleFolder);
        var noticesFile = Path.Combine(sample, "THIRD-PARTY-NOTICES.txt");
        var packs = new List<Pack>();
        foreach (var packsFolder in new[] { ModelsFolder, EnvironmentsFolder, FontsFolder })
        {
            var root = Path.Combine(sample, packsFolder.Replace('/', Path.DirectorySeparatorChar));
            if (!Directory.Exists(root)) continue;
            foreach (var folder in Directory.GetDirectories(root).OrderBy(f => f, StringComparer.Ordinal))
            {
                var files = Directory.GetFiles(folder, "*", SearchOption.AllDirectories)
                    .Select(f => Path.GetRelativePath(folder, f).Replace('\\', '/'))
                    .OrderBy(f => f, StringComparer.Ordinal)
                    .ToList();
                packs.Add(new Pack(Path.GetFileName(folder), files, packsFolder));
            }
        }

        var notices = File.Exists(noticesFile) ? File.ReadAllText(noticesFile, Encoding.UTF8) : null;
        return Check(notices, layout.Relative(noticesFile), packs);
    }

    /// <summary>Checks the notices text against the packs.</summary>
    /// <param name="notices">The notices file's text, or null when it does not exist.</param>
    /// <param name="noticesShown">Its path as the messages show it.</param>
    /// <param name="packs">The packs the sample carries.</param>
    public static CheckResult Check(string notices, string noticesShown, IReadOnlyList<Pack> packs)
    {
        ArgumentNullException.ThrowIfNull(packs);
        var problems = new List<string>();
        var notes = new List<string>();
        if (packs.Count == 0)
        {
            notes.Add($"the sample carries no model packs ({SampleFolder}/{ModelsFolder} is empty or missing)");
            return new CheckResult(Name, problems, notes);
        }

        if (notices == null)
        {
            problems.Add($"{noticesShown} does not exist; the sample carries model packs ({string.Join(", ", packs.Select(p => p.Name))}), and each needs a notices entry there - write it");
            return new CheckResult(Name, problems, notes);
        }

        foreach (var pack in packs)
        {
            if (OwnersOwnPacks.Contains(pack.Name, StringComparer.Ordinal))
            {
                notes.Add($"{pack.Name}: the repository owner's own work ({nameof(OwnersOwnPacks)}) - no licence file or notices entry needed");
                continue;
            }

            var fonts = pack.Folder == FontsFolder;
            var assetExtensions = fonts ? FontExtensions : ModelExtensions;
            var licences = pack.Files.Where(f => !f.Contains('/') && (f.StartsWith("LICENSE", StringComparison.OrdinalIgnoreCase) || (fonts && f.StartsWith("OFL", StringComparison.OrdinalIgnoreCase)))).ToList();
            if (licences.Count == 0)
            {
                problems.Add(fonts
                    ? $"{pack.Folder}/{pack.Name} has no licence file beside its fonts; a font is carried only with its ORIGINAL licence file (OFL.txt or LICENSE..., copied with the font) - add it, or remove the pack"
                    : $"{pack.Folder}/{pack.Name} has no licence file beside its files; a pack is carried only with its own licence file (LICENSE-....txt, copied from where the pack came from) - add it, or remove the pack");
            }

            if (!notices.Contains(pack.Name, StringComparison.Ordinal))
            {
                problems.Add($"{noticesShown} does not name the pack '{pack.Name}'; add an entry with its source, its licence and its files");
            }

            foreach (var licence in licences.Where(l => !notices.Contains($"{pack.Folder}/{pack.Name}/{l}", StringComparison.Ordinal)))
            {
                problems.Add($"{noticesShown} does not give the path of the licence file {pack.Folder}/{pack.Name}/{licence}; name it in the pack's entry");
            }

            foreach (var model in pack.Files.Where(f => assetExtensions.Contains(Path.GetExtension(f), StringComparer.OrdinalIgnoreCase)))
            {
                var name = Path.GetFileNameWithoutExtension(model);
                if (!notices.Contains(name, StringComparison.Ordinal))
                {
                    problems.Add($"{noticesShown} does not name the file {pack.Name}/{model}; list it among the pack's files");
                }
            }

            notes.Add($"{pack.Name}: {pack.Files.Count(f => assetExtensions.Contains(Path.GetExtension(f), StringComparer.OrdinalIgnoreCase))} {(fonts ? "font" : "model or panorama")} file(s), licence {string.Join(", ", licences)}");
        }

        return new CheckResult(Name, problems, notes);
    }
}
