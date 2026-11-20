using System;
using System.IO;
using System.Reflection;
using System.Threading;

namespace GraphicsDemo.Assets;

/// <summary>
/// The sample's environment panoramas: Radiance .hdr files embedded in this assembly
/// (Environments/&lt;pack&gt;/&lt;file&gt;.hdr, each pack's licence file beside it), read on first use and kept. The
/// backend serves them unchanged; the graphics frontend decodes them with the graphics crate's .hdr decoder and
/// builds an image-based-lighting environment from them. Every member is safe to call from several threads at once.
/// </summary>
public sealed class EnvironmentLibrary
{
    /// <summary>The name of the Poly Haven "Small Cathedral" panorama (1024 x 512, CC0).</summary>
    public const string SmallCathedralName = "small-cathedral";

    /// <summary>The embedded resource name of the Small Cathedral panorama.</summary>
    public const string SmallCathedralResource = "Environments/polyhaven-small-cathedral/small_cathedral_1k.hdr";

    /// <summary>The content type of a Radiance .hdr file.</summary>
    public const string RadianceContentType = "image/vnd.radiance";

    private readonly Assembly assembly = typeof(EnvironmentLibrary).Assembly;
    private readonly Lazy<EnvironmentFile> smallCathedral;

    /// <summary>Creates the library (nothing is read yet).</summary>
    public EnvironmentLibrary()
    {
        smallCathedral = new Lazy<EnvironmentFile>(() => new EnvironmentFile(SmallCathedralName, Read(SmallCathedralResource)), LazyThreadSafetyMode.ExecutionAndPublication);
    }

    /// <summary>The Poly Haven "Small Cathedral" panorama, an indoor HDRI with bright windows.</summary>
    public EnvironmentFile SmallCathedral => smallCathedral.Value;

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
