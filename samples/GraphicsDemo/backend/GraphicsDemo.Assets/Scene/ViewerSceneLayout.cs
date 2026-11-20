using System;
using System.Collections.Generic;
using System.Numerics;

namespace GraphicsDemo.Assets.Scene;

/// <summary>
/// The layout of the Viewer scene, taken from the desktop model viewer (its scene and renderer): the ground, the
/// animated actors (the chest in the middle, eight animals - four walking a ring, four standing closer in and
/// cross-fading through a routine - and the Avian circling overhead), the static props around the edge, and the camera
/// and lighting values.
/// </summary>
public static class ViewerSceneLayout
{
    /// <summary>How long a standing animal keeps one clip of its routine before it cross-fades to the next, in seconds.</summary>
    public const float RoutineSeconds = 3.5f;

    /// <summary>How long a cross-fade between two clips of a routine takes, in seconds.</summary>
    public const float CrossFadeSeconds = 0.4f;

    /// <summary>The animals, in the desktop viewer's order (the even ones walk, the odd ones stand).</summary>
    public static IReadOnlyList<string> Animals { get; } = new[] { "animal-cat", "animal-dog", "animal-fox", "animal-pig", "animal-cow", "animal-panda", "animal-penguin", "animal-bunny" };

    /// <summary>The routine the standing animals cross-fade through.</summary>
    public static IReadOnlyList<string> Routine { get; } = new[] { "idle", "dance", "eat", "gesture-positive", "idle", "dance" };

    /// <summary>The animated actors in the desktop viewer's order: the chest, the eight animals, the Avian.</summary>
    public static IReadOnlyList<ViewerActor> Actors { get; } = CreateActors();

    /// <summary>The static props, in the desktop viewer's order.</summary>
    public static IReadOnlyList<ViewerProp> Props { get; } = new[]
    {
        new ViewerProp("tree", 0.3f, 10.5f, 2.6f), new ViewerProp("tree-tall", 0.9f, 11.5f, 2.6f), new ViewerProp("tree-autumn", 1.6f, 10f, 2.8f), new ViewerProp("tree", 2.3f, 11f, 2.4f),
        new ViewerProp("tree-autumn", 3.1f, 10.8f, 2.6f), new ViewerProp("tree-tall", 3.9f, 10.2f, 2.5f), new ViewerProp("tree", 4.6f, 11.4f, 2.7f), new ViewerProp("tree-autumn", 5.4f, 10.4f, 2.5f),
        new ViewerProp("rock-a", 0.6f, 7.8f, 2.2f), new ViewerProp("rock-b", 2.0f, 8.2f, 2.0f), new ViewerProp("rock-c", 3.5f, 7.6f, 2.4f), new ViewerProp("rock-a", 5.0f, 8.0f, 1.8f),
        new ViewerProp("barrel", 1.2f, 7.4f, 2.2f), new ViewerProp("barrel", 1.32f, 7.9f, 2.2f), new ViewerProp("campfire-pit", 4.25f, 7.6f, 2.4f),
        new ViewerProp("patch-grass-large", 2.7f, 6.8f, 2.4f), new ViewerProp("patch-grass-large", 5.8f, 6.6f, 2.4f), new ViewerProp("patch-grass-large", 0.0f, 7.0f, 2.4f),
        new ViewerProp("fence", 2.75f, 8.8f, 2.4f), new ViewerProp("fence", 2.95f, 8.8f, 2.4f), new ViewerProp("fence", 3.15f, 8.8f, 2.4f),
    };

    /// <summary>The camera's vertical field of view, in degrees.</summary>
    public const float FieldOfViewDegrees = 50f;

    /// <summary>The camera's near plane.</summary>
    public const float NearPlane = 0.1f;

    /// <summary>The camera's far plane.</summary>
    public const float FarPlane = 200f;

    /// <summary>The orbit camera's distance from its target.</summary>
    public const float OrbitDistance = 10.5f;

    /// <summary>The orbit camera's pitch, in radians.</summary>
    public const float OrbitPitch = 0.30f;

    /// <summary>The orbit camera's yaw at time 0, in radians: yaw = <see cref="OrbitYawStart" /> + <see cref="OrbitYawPerSecond" /> x t.</summary>
    public const float OrbitYawStart = 0.7f;

    /// <summary>How fast the orbit camera turns on its own, in radians per second.</summary>
    public const float OrbitYawPerSecond = 0.18f;

    /// <summary>The sun's light intensity.</summary>
    public const float LightIntensity = 1.6f;

    /// <summary>The orbit camera's target.</summary>
    public static Vector3 OrbitTarget => new Vector3(0, 0.9f, 0);

    /// <summary>The direction the sun's light travels (unit length): normalize(-0.45, -1, -0.35).</summary>
    public static Vector3 SunDirection => Vector3.Normalize(new Vector3(-0.45f, -1f, -0.35f));

    /// <summary>The sun's light colour (linear).</summary>
    public static Vector3 LightColor => new Vector3(1f, 0.95f, 0.86f);

    /// <summary>The procedural sky's zenith colour (linear): the desktop viewer's late-morning sky.</summary>
    public static Vector3 SkyZenithColor => new Vector3(0.14f, 0.32f, 0.70f);

    /// <summary>The procedural sky's horizon colour (linear).</summary>
    public static Vector3 SkyHorizonColor => new Vector3(0.40f, 0.58f, 0.85f);

    /// <summary>The procedural sky's ground colour (linear): a dark meadow green, so the light bouncing up is green-tinted like the grass.</summary>
    public static Vector3 SkyGroundColor => new Vector3(0.10f, 0.15f, 0.07f);

    /// <summary>The procedural sky's haze (turbidity).</summary>
    public const float SkyTurbidity = 2f;

    /// <summary>The exposure when an HDR panorama lights the scene instead of the procedural sky (the panorama carries its own light; the sun light is off).</summary>
    public const float PanoramaExposure = 1.2f;

    /// <summary>The flat sky's clear colour as sRGB bytes (red, green, blue).</summary>
    public static ReadOnlySpan<byte> SkyColorSrgb => new byte[] { 150, 196, 232 };

    /// <summary>The distinct models the props use, in the order of their first use.</summary>
    /// <returns>The model names.</returns>
    public static IReadOnlyList<string> PropModels()
    {
        List<string> names = new List<string>();
        foreach (ViewerProp prop in Props)
        {
            if (!names.Contains(prop.Model)) names.Add(prop.Model);
        }

        return names;
    }

    /// <summary>The distinct models the scene places: the actors' models in the order of their first use, then the props' models.</summary>
    /// <returns>The model names.</returns>
    public static IReadOnlyList<string> SceneModels()
    {
        List<string> names = new List<string>();
        foreach (ViewerActor actor in Actors)
        {
            if (!names.Contains(actor.Model)) names.Add(actor.Model);
        }

        foreach (string name in PropModels())
        {
            if (!names.Contains(name)) names.Add(name);
        }

        return names;
    }

    /// <summary>The actors exactly as the desktop viewer's scene adds them.</summary>
    private static ViewerActor[] CreateActors()
    {
        List<ViewerActor> actors = new List<ViewerActor>
        {
            // The chest in the middle, opening and closing.
            new ViewerActor("chest", ViewerActorMotion.Fixed, "open-close", null, 0f, 3.2f) { RotationY = 0.6f },
        };

        // Four animals walk a ring; four more stand closer in, facing the chest, and cross-fade through the routine.
        for (int index = 0; index < Animals.Count; index++)
        {
            float baseAngle = index * MathF.Tau / Animals.Count;
            if (index % 2 == 0)
            {
                actors.Add(new ViewerActor(Animals[index], ViewerActorMotion.Walk, "walk", null, index * 0.13f, 0.55f) { BaseAngle = baseAngle, Radius = 5.4f, AngularSpeed = 0.28f });
            }
            else
            {
                Vector3 position = new Vector3(MathF.Sin(baseAngle), 0, MathF.Cos(baseAngle)) * 2.9f;
                actors.Add(new ViewerActor(Animals[index], ViewerActorMotion.Fixed, null, Routine, index * 0.9f, 0.55f) { RotationY = baseAngle + MathF.PI, Translation = position });
            }
        }

        // The Avian circles overhead, the other way round.
        actors.Add(new ViewerActor("avian", ViewerActorMotion.Fly, "fly", null, 0f, 0.36f) { Radius = 4.2f, AngularSpeed = -0.45f, Height = 3.0f, BobAmplitude = 0.25f, BobRate = 1.7f });
        return actors.ToArray();
    }
}
