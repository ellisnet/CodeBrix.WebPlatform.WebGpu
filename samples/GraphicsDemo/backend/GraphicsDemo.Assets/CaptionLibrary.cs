using System;
using System.IO;
using System.Reflection;
using System.Security.Cryptography;
using System.Threading;

namespace GraphicsDemo.Assets;

/// <summary>
/// The sample's caption files: SubRip (SRT) text embedded in this assembly (Captions/&lt;file&gt;.srt), read on first use
/// and kept. The backend serves them unchanged; the "animation" frontend parses them with the animation crate's
/// CaptionTrack (the sample's rule: resources stay on the backend; the crate itself takes caption text from anywhere).
/// Every member is safe to call from several threads at once.
/// </summary>
public sealed class CaptionLibrary
{
    /// <summary>The embedded resource name of the captions scene's cues (the C# demo's own SRT text).</summary>
    public const string CaptionSceneResource = "Captions/caption-scene.srt";

    /// <summary>The content type a SubRip file is served with (UTF-8 text).</summary>
    public const string SubRipContentType = "text/plain; charset=utf-8";

    private readonly Assembly assembly = typeof(CaptionLibrary).Assembly;
    private readonly Lazy<byte[]> captionScene;
    private readonly Lazy<string> captionSceneETag;

    /// <summary>Creates the library (nothing is read yet).</summary>
    public CaptionLibrary()
    {
        captionScene = new Lazy<byte[]>(() => Read(CaptionSceneResource), LazyThreadSafetyMode.ExecutionAndPublication);
        captionSceneETag = new Lazy<string>(() => "\"" + Convert.ToHexStringLower(SHA256.HashData(captionScene.Value)) + "\"", LazyThreadSafetyMode.ExecutionAndPublication);
    }

    /// <summary>The captions scene's SRT file, its bytes (UTF-8). Never modify them: they are shared by every request.</summary>
    public byte[] CaptionScene => captionScene.Value;

    /// <summary>A strong entity tag of <see cref="CaptionScene"/> (the quoted lower-case hex SHA-256 of its bytes).</summary>
    public string CaptionSceneETag => captionSceneETag.Value;

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
