using System;
using System.Collections.Concurrent;
using System.Collections.Generic;
using System.IO;
using System.Reflection;
using System.Security.Cryptography;

namespace GraphicsDemo.Assets;

/// <summary>
/// The sample's SVG documents: text embedded in this assembly (SvgDocuments/&lt;name&gt;.svg), read on first use and kept.
/// The backend serves them unchanged; the "animation" frontend loads them with the animation crate's DrawingSvg (the
/// sample's rule: resources stay on the backend; the crate itself takes SVG text from anywhere). Every member is safe to
/// call from several threads at once.
/// </summary>
public sealed class SvgDocumentLibrary
{
    /// <summary>The gear train of the "svg-parts" scene (the C# demo's own, generated the same way).</summary>
    public const string GearTrain = "gear-train";

    /// <summary>The clock of the "svg-parts" scene (the C# demo's own, generated the same way; it has a text label).</summary>
    public const string Clock = "clock";

    /// <summary>The document of the "svg-smil" scene, which animates itself with SMIL.</summary>
    public const string SmilScene = "smil-scene";

    /// <summary>The document of the "svg-css" scene, which animates itself with CSS keyframes.</summary>
    public const string CssScene = "css-scene";

    /// <summary>The document of the "svg-features" scene: the SVG features the crate draws.</summary>
    public const string Features = "features";

    /// <summary>The content type an SVG document is served with.</summary>
    public const string SvgContentType = "image/svg+xml";

    /// <summary>Every document name, in the order the scenes use them.</summary>
    public static readonly IReadOnlyList<string> Names = new[] { GearTrain, Clock, SmilScene, CssScene, Features };

    private readonly Assembly assembly = typeof(SvgDocumentLibrary).Assembly;
    private readonly ConcurrentDictionary<string, Lazy<(byte[] Bytes, string ETag)>> documents = new ConcurrentDictionary<string, Lazy<(byte[] Bytes, string ETag)>>(StringComparer.Ordinal);

    /// <summary>A document's bytes (UTF-8 SVG text). Never modify them: they are shared by every request.</summary>
    /// <param name="name">One of <see cref="Names"/>.</param>
    /// <returns>The bytes.</returns>
    /// <exception cref="ArgumentException">The name is not one of <see cref="Names"/>.</exception>
    public byte[] Get(string name) => Entry(name).Bytes;

    /// <summary>A strong entity tag of a document (the quoted lower-case hex SHA-256 of its bytes).</summary>
    /// <param name="name">One of <see cref="Names"/>.</param>
    /// <returns>The entity tag.</returns>
    /// <exception cref="ArgumentException">The name is not one of <see cref="Names"/>.</exception>
    public string GetETag(string name) => Entry(name).ETag;

    private (byte[] Bytes, string ETag) Entry(string name)
    {
        if (name == null || !((IList<string>)Names).Contains(name))
        {
            throw new ArgumentException("'" + name + "' is not one of the sample's SVG documents; use one of SvgDocumentLibrary.Names.", nameof(name));
        }

        return documents.GetOrAdd(name, n => new Lazy<(byte[], string)>(() =>
        {
            byte[] bytes = Read("SvgDocuments/" + n + ".svg");
            return (bytes, "\"" + Convert.ToHexStringLower(SHA256.HashData(bytes)) + "\"");
        })).Value;
    }

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
