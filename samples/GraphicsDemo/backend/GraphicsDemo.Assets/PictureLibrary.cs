using System;
using System.Threading;
using GraphicsDemo.Assets.Conversion.Internal;

namespace GraphicsDemo.Assets;

/// <summary>
/// The pictures of the "animation" application, DECODED here on the backend with CodeBrix.Imaging (the sample's
/// rule: resources stay on the backend, and no image decoder runs in the browser) and served as pixels in the
/// <see cref="PicturePixels"/> form. The decoded picture is an embedded file this assembly already carries for the
/// models: the Kenney "Cube Pets" pack's colour map (CC0; see THIRD-PARTY-NOTICES.txt at the sample's root), so no
/// new asset comes with it. The Lottie badge is made in code here (the picture of the "lottie-images" scene's image
/// layer). Made on first use and kept. Every member is safe to call from several threads.
/// </summary>
public sealed class PictureLibrary
{
    /// <summary>The name of the Cube Pets colour map.</summary>
    public const string CubePetsColormapName = "cube-pets-colormap";

    /// <summary>Its path below Models/ (the models' embedded resources).</summary>
    public const string CubePetsColormapPath = "cube-pets/Textures/colormap.png";

    /// <summary>The name of the badge made in code for the Lottie image layer.</summary>
    public const string LottieBadgeName = "lottie-badge";

    /// <summary>The width and height of the badge, in pixels (the image asset's size in the Lottie file).</summary>
    public const int LottieBadgeSize = 128;

    /// <summary>The content type of the pixel form.</summary>
    public const string PixelContentType = "application/octet-stream";

    /// <summary>The content type of a PNG file.</summary>
    public const string PngContentType = "image/png";

    private readonly Lazy<PicturePixels> cubePetsColormap;
    private readonly Lazy<PngFile> cubePetsColormapPng;
    private readonly Lazy<PicturePixels> lottieBadge = new Lazy<PicturePixels>(MakeLottieBadge, LazyThreadSafetyMode.ExecutionAndPublication);

    /// <summary>Creates the library (nothing is decoded yet).</summary>
    /// <param name="models">The model library whose embedded files hold the pictures.</param>
    /// <exception cref="ArgumentNullException">An argument is null.</exception>
    public PictureLibrary(ModelLibrary models)
    {
        ArgumentNullException.ThrowIfNull(models);
        cubePetsColormap = new Lazy<PicturePixels>(() => Decode(models, CubePetsColormapName, CubePetsColormapPath), LazyThreadSafetyMode.ExecutionAndPublication);
        cubePetsColormapPng = new Lazy<PngFile>(() => new PngFile(models.ReadResource(CubePetsColormapPath) ?? throw new InvalidOperationException("The embedded picture '" + CubePetsColormapPath + "' is missing; check the EmbeddedResource items of GraphicsDemo.Assets.csproj.")), LazyThreadSafetyMode.ExecutionAndPublication);
    }

    /// <summary>The Cube Pets colour map as the PNG FILE itself (not decoded here: the frontend's crate reads PNG files).</summary>
    public PngFile CubePetsColormapPng => cubePetsColormapPng.Value;

    /// <summary>The Cube Pets colour map (512 x 512), decoded.</summary>
    public PicturePixels CubePetsColormap => cubePetsColormap.Value;

    /// <summary>
    /// The badge of the Lottie image layer (<see cref="LottieBadgeSize"/> square), MADE IN CODE: a disc of eight
    /// alternating orange and violet wedges with a white rim, transparent outside (straight alpha, anti-aliased).
    /// </summary>
    public PicturePixels LottieBadge => lottieBadge.Value;

    private static PicturePixels MakeLottieBadge()
    {
        int size = LottieBadgeSize;
        byte[] rgba = new byte[size * size * 4];
        double center = size / 2.0;
        for (int y = 0; y < size; y++)
        {
            for (int x = 0; x < size; x++)
            {
                double dx = x + 0.5 - center;
                double dy = y + 0.5 - center;
                double r = Math.Sqrt((dx * dx) + (dy * dy));
                double alpha = Math.Clamp(center - 2.0 - r + 0.5, 0.0, 1.0);
                int wedge = (int)Math.Floor((Math.Atan2(dy, dx) + Math.PI) / (Math.PI / 4.0)) % 8;
                (double red, double green, double blue) = wedge % 2 == 0 ? (255.0, 176.0, 72.0) : (138.0, 104.0, 224.0);
                double rim = Math.Clamp(1.0 - (Math.Abs(r - (center - 9.0)) - 3.0), 0.0, 1.0);
                int i = ((y * size) + x) * 4;
                rgba[i] = (byte)Math.Round(red + ((255.0 - red) * rim));
                rgba[i + 1] = (byte)Math.Round(green + ((255.0 - green) * rim));
                rgba[i + 2] = (byte)Math.Round(blue + ((255.0 - blue) * rim));
                rgba[i + 3] = (byte)Math.Round(255.0 * alpha);
            }
        }

        return new PicturePixels(LottieBadgeName, rgba, size, size);
    }

    private static PicturePixels Decode(ModelLibrary models, string name, string path)
    {
        byte[] encoded = models.ReadResource(path) ?? throw new InvalidOperationException("The embedded picture '" + path + "' is missing; check the EmbeddedResource items of GraphicsDemo.Assets.csproj.");
        byte[] rgba = TextureDecoder.Decode(encoded, path, out int width, out int height);
        return new PicturePixels(name, rgba, width, height);
    }
}
