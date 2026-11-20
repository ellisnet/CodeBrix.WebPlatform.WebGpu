using CodeBrix.WebPlatform.Core.Mapping;

namespace GraphicsDemo.Api.Contracts;

/// <summary>The scene's light, its procedural sky, the exposure for an HDR panorama, and the flat clear colour.</summary>
[WireRequired]
public sealed class SceneLighting
{
    /// <summary>The direction the sun's light travels (unit length).</summary>
    public SceneVector SunDirection { get; set; }

    /// <summary>The sun's light colour (linear red, green, blue).</summary>
    public SceneVector LightColor { get; set; }

    /// <summary>The sun's light intensity.</summary>
    public float LightIntensity { get; set; }

    /// <summary>The procedural sky's zenith colour (linear red, green, blue).</summary>
    public SceneVector SkyZenithColor { get; set; }

    /// <summary>The procedural sky's horizon colour (linear red, green, blue).</summary>
    public SceneVector SkyHorizonColor { get; set; }

    /// <summary>The procedural sky's ground colour (linear red, green, blue).</summary>
    public SceneVector SkyGroundColor { get; set; }

    /// <summary>The procedural sky's haze (turbidity, 1 to 10).</summary>
    public float SkyTurbidity { get; set; }

    /// <summary>The exposure when the HDR panorama lights the scene (its sun light off).</summary>
    public float PanoramaExposure { get; set; }

    /// <summary>The flat sky's clear colour, sRGB red (0..255).</summary>
    public int SkyRed { get; set; }

    /// <summary>The flat sky's clear colour, sRGB green (0..255).</summary>
    public int SkyGreen { get; set; }

    /// <summary>The flat sky's clear colour, sRGB blue (0..255).</summary>
    public int SkyBlue { get; set; }
}
