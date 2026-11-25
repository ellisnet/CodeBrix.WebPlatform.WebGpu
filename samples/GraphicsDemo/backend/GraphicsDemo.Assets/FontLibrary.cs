using System;
using System.Collections.Concurrent;
using System.Collections.Generic;
using System.IO;
using System.Reflection;
using System.Threading;

namespace GraphicsDemo.Assets;

/// <summary>
/// The sample's fonts: every TrueType or OpenType file embedded in this assembly below Fonts/ (Fonts/&lt;pack&gt;/
/// &lt;file&gt;.ttf or .otf, each pack with its font's original licence file beside it), found when the library is made,
/// read on first use and kept. A font is known by its FILE NAME in lower case (Fonts/roboto/Roboto-Regular.ttf is
/// "roboto-regular.ttf"): adding a font is adding its file (and its pack's licence file and notices entry), with no
/// code to change here. The backend serves the files unchanged; the "animation" frontend registers their bytes with
/// the animation crate's FontRegistry and chooses itself which fonts it asks for (the crate takes font bytes from
/// anywhere). The sample's fonts are SIL Open Font License 1.1 fonts, allowed in the sample only. Every member is safe
/// to call from several threads at once.
/// </summary>
public sealed class FontLibrary
{
    /// <summary>The content type of a TrueType font file.</summary>
    public const string TrueTypeContentType = "font/ttf";

    /// <summary>The content type of an OpenType (CFF) font file.</summary>
    public const string OpenTypeContentType = "font/otf";

    private const string ResourcePrefix = "Fonts/";

    private readonly Assembly assembly = typeof(FontLibrary).Assembly;
    private readonly Dictionary<string, string> resources = new Dictionary<string, string>(StringComparer.Ordinal);
    private readonly ConcurrentDictionary<string, Lazy<FontFile>> fonts = new ConcurrentDictionary<string, Lazy<FontFile>>(StringComparer.Ordinal);

    /// <summary>Finds the embedded fonts (nothing is read yet).</summary>
    /// <exception cref="InvalidOperationException">Two embedded fonts have the same file name.</exception>
    public FontLibrary()
    {
        List<string> names = new List<string>();
        foreach (string resource in assembly.GetManifestResourceNames())
        {
            // The resource name is the path below the project, with '/' on every build machine; a Windows build may write '\'.
            string path = resource.Replace('\\', '/');
            if (!path.StartsWith(ResourcePrefix, StringComparison.Ordinal) || ContentTypeOf(path) == null) continue;
            string name = Path.GetFileName(path).ToLowerInvariant();
            if (resources.TryGetValue(name, out string other))
            {
                throw new InvalidOperationException("Two embedded fonts are named '" + name + "' ('" + other + "' and '" + resource + "'). Rename one: the library finds fonts by file name.");
            }

            resources[name] = resource;
            names.Add(name);
        }

        names.Sort(StringComparer.Ordinal);
        FontNames = names;
    }

    /// <summary>The names of every embedded font - each file name in lower case, such as "roboto-regular.ttf" - in ordinal order.</summary>
    public IReadOnlyList<string> FontNames { get; }

    /// <summary>A font by its name (its file name; any case), read once and kept; null when there is no such font.</summary>
    /// <param name="name">The font's name, such as "roboto-regular.ttf".</param>
    /// <returns>The font file, or null.</returns>
    public FontFile GetFont(string name)
    {
        if (name == null) return null;
        string key = name.ToLowerInvariant();
        if (!resources.TryGetValue(key, out string resource)) return null;
        return fonts.GetOrAdd(key, found => new Lazy<FontFile>(() => new FontFile(found, Read(resource), ContentTypeOf(found)), LazyThreadSafetyMode.ExecutionAndPublication)).Value;
    }

    /// <summary>The content type of a font file by its extension, or null for a file that is not a font.</summary>
    private static string ContentTypeOf(string path)
    {
        string extension = Path.GetExtension(path);
        if (extension.Equals(".ttf", StringComparison.OrdinalIgnoreCase)) return TrueTypeContentType;
        if (extension.Equals(".otf", StringComparison.OrdinalIgnoreCase)) return OpenTypeContentType;
        return null;
    }

    private byte[] Read(string resource)
    {
        using Stream stream = assembly.GetManifestResourceStream(resource) ?? throw new InvalidOperationException("The embedded resource '" + resource + "' is missing; check the EmbeddedResource items of GraphicsDemo.Assets.csproj.");
        using MemoryStream copy = new MemoryStream();
        stream.CopyTo(copy);
        return copy.ToArray();
    }
}
