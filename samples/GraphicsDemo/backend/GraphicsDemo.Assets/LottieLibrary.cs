using System;
using System.Collections.Concurrent;
using System.IO;
using System.Reflection;
using System.Security.Cryptography;

namespace GraphicsDemo.Assets;

/// <summary>
/// The sample's Lottie animations: Lottie JSON embedded in this assembly (Lottie/&lt;file&gt;.json), read on first use and
/// kept. The backend serves them unchanged; the "animation" frontend reads them with the animation crate's
/// DrawingLottie (the sample's rule: resources stay on the backend; the crate itself takes Lottie JSON from anywhere).
/// The three trial files were INVENTED by a throw-away generator for the renderer trial (nothing downloaded); the
/// features file and the image-layer file were written by hand for the sample. Every member is safe to call from several threads at once.
/// </summary>
public sealed class LottieLibrary
{
    /// <summary>The trial file with three hundred shape layers (many paths, trim paths).</summary>
    public const string ManyShapes = "A_many_shapes.json";

    /// <summary>The trial file with gradients, a mask, a matte and morphing paths.</summary>
    public const string Painterly = "B_painterly.json";

    /// <summary>The trial file with nested precompositions and repeaters (many groups).</summary>
    public const string BusyScene = "C_busy_scene.json";

    /// <summary>The sample's own file: parenting, a trim path, a gradient, a matte and three markers.</summary>
    public const string Features = "D_features.json";

    /// <summary>The sample's own file with one IMAGE LAYER (asset id "badge", 128 x 128), whose pixels the page supplies.</summary>
    public const string ImageLayer = "E_image_layer.json";

    /// <summary>The content type a Lottie file is served with (JSON, UTF-8).</summary>
    public const string LottieContentType = "application/json; charset=utf-8";

    private readonly Assembly assembly = typeof(LottieLibrary).Assembly;
    private readonly ConcurrentDictionary<string, Lazy<(byte[] Bytes, string ETag)>> files = new ConcurrentDictionary<string, Lazy<(byte[] Bytes, string ETag)>>(StringComparer.Ordinal);

    /// <summary>The bytes of one of the embedded files (UTF-8 JSON). Never modify them: they are shared by every request.</summary>
    /// <param name="name">One of the file names this class defines (for example <see cref="ManyShapes"/>).</param>
    /// <returns>The file's bytes.</returns>
    public byte[] GetFile(string name) => Entry(name).Bytes;

    /// <summary>A strong entity tag of a file (the quoted lower-case hex SHA-256 of its bytes).</summary>
    /// <param name="name">One of the file names this class defines.</param>
    /// <returns>The entity tag.</returns>
    public string GetETag(string name) => Entry(name).ETag;

    private (byte[] Bytes, string ETag) Entry(string name)
    {
        if (name != ManyShapes && name != Painterly && name != BusyScene && name != Features && name != ImageLayer)
        {
            throw new ArgumentException("The Lottie file '" + name + "' is not one of the sample's; use one of the names LottieLibrary defines.", nameof(name));
        }

        return files.GetOrAdd(name, n => new Lazy<(byte[] Bytes, string ETag)>(() =>
        {
            byte[] bytes = Read("Lottie/" + n);
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
