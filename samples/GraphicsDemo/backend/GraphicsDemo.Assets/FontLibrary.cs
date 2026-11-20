using System;
using System.IO;
using System.Reflection;
using System.Threading;

namespace GraphicsDemo.Assets;

/// <summary>
/// The sample's fonts: TrueType files embedded in this assembly (Fonts/&lt;pack&gt;/&lt;file&gt;.ttf, each with its
/// original OFL.txt beside it), read on first use and kept. The backend serves them unchanged; the "animation"
/// frontend registers their bytes with the animation crate's FontRegistry (the sample's rule: fonts stay on the
/// backend; the crate itself takes font bytes from anywhere). Both are SIL Open Font License 1.1 fonts, allowed in
/// the sample only. Every member is safe to call from several threads at once.
/// </summary>
public sealed class FontLibrary
{
    /// <summary>The name of Roboto Regular, the font the C# AnimationDemoApp draws all of its text with.</summary>
    public const string RobotoRegularName = "roboto-regular";

    /// <summary>The embedded resource name of Roboto Regular.</summary>
    public const string RobotoRegularResource = "Fonts/roboto/Roboto-Regular.ttf";

    /// <summary>The name of Open Sans Regular, used by the missing-characters scene (its .notdef glyph has no ink).</summary>
    public const string OpenSansRegularName = "open-sans-regular";

    /// <summary>The embedded resource name of Open Sans Regular.</summary>
    public const string OpenSansRegularResource = "Fonts/open-sans/OpenSans-Regular.ttf";

    /// <summary>The content type of a TrueType font file.</summary>
    public const string TrueTypeContentType = "font/ttf";

    private readonly Assembly assembly = typeof(FontLibrary).Assembly;
    private readonly Lazy<FontFile> robotoRegular;
    private readonly Lazy<FontFile> openSansRegular;

    /// <summary>Creates the library (nothing is read yet).</summary>
    public FontLibrary()
    {
        robotoRegular = new Lazy<FontFile>(() => new FontFile(RobotoRegularName, Read(RobotoRegularResource)), LazyThreadSafetyMode.ExecutionAndPublication);
        openSansRegular = new Lazy<FontFile>(() => new FontFile(OpenSansRegularName, Read(OpenSansRegularResource)), LazyThreadSafetyMode.ExecutionAndPublication);
    }

    /// <summary>Roboto Regular (Copyright 2011 The Roboto Project Authors, SIL OFL 1.1).</summary>
    public FontFile RobotoRegular => robotoRegular.Value;

    /// <summary>Open Sans Regular (Copyright 2020 The Open Sans Project Authors, SIL OFL 1.1).</summary>
    public FontFile OpenSansRegular => openSansRegular.Value;

    private byte[] Read(string path)
    {
        // The resource name is the path with '/' on every build machine; a Windows build may write '\'.
        string resource = Array.Find(assembly.GetManifestResourceNames(), name => name.Replace('\\', '/') == path) ?? path;
        using Stream stream = assembly.GetManifestResourceStream(resource) ?? throw new InvalidOperationException("The embedded resource '" + resource + "' is missing; check the EmbeddedResource items of GraphicsDemo.Assets.csproj.");
        using MemoryStream copy = new MemoryStream();
        stream.CopyTo(copy);
        return copy.ToArray();
    }
}
