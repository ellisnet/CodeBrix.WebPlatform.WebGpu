using System;
using System.Numerics;

namespace GraphicsDemo.Assets.Scene;

/// <summary>
/// One static prop of the Viewer scene, as the desktop model viewer's props table gives it: a model placed on a circle
/// around the scene's centre, facing the centre. Its world matrix is scale, then a rotation about Y by
/// <see cref="Angle" /> + PI, then the translation (sin(angle) x radius, lift, cos(angle) x radius).
/// </summary>
public sealed class ViewerProp
{
    /// <summary>The height flat props (the grass patches, whose names start with "patch-") sit above the ground, so that the two surfaces never fight over depth.</summary>
    public const float FlatPropLift = 0.03f;

    /// <summary>Creates a prop.</summary>
    /// <param name="model">The model's name.</param>
    /// <param name="angle">The angle around the centre, in radians.</param>
    /// <param name="radius">The distance from the centre.</param>
    /// <param name="scale">The uniform scale.</param>
    public ViewerProp(string model, float angle, float radius, float scale)
    {
        Model = model;
        Angle = angle;
        Radius = radius;
        Scale = scale;
    }

    /// <summary>The model's name (a model of <see cref="ModelLibrary" />).</summary>
    public string Model { get; }

    /// <summary>The angle around the centre, in radians.</summary>
    public float Angle { get; }

    /// <summary>The distance from the centre.</summary>
    public float Radius { get; }

    /// <summary>The uniform scale.</summary>
    public float Scale { get; }

    /// <summary>The rotation about Y, in radians: <see cref="Angle" /> + PI, so that the prop faces the centre.</summary>
    public float RotationY => Angle + MathF.PI;

    /// <summary>The height above the ground: <see cref="FlatPropLift" /> for a flat prop, else 0.</summary>
    public float Lift => Model.StartsWith("patch-", StringComparison.Ordinal) ? FlatPropLift : 0f;

    /// <summary>The translation: (sin(angle) x radius, lift, cos(angle) x radius).</summary>
    public Vector3 Translation => new Vector3(MathF.Sin(Angle) * Radius, Lift, MathF.Cos(Angle) * Radius);

    /// <summary>The world matrix, exactly as the desktop viewer computes it.</summary>
    public Matrix4x4 World => Matrix4x4.CreateScale(Scale) * Matrix4x4.CreateRotationY(RotationY) * Matrix4x4.CreateTranslation(Translation);
}
