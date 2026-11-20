using System;
using System.Collections.Generic;
using System.Numerics;

namespace GraphicsDemo.Assets.Scene;

/// <summary>
/// One animated actor of the Viewer scene, as the desktop model viewer's scene adds it: a model, the clip it loops (or
/// the routine of clips it cross-fades through), the time offset that keeps the actors out of step, and how it moves.
/// The desktop viewer's placement functions of the scene time t are:
/// <list type="bullet">
/// <item><see cref="ViewerActorMotion.Fixed" />: scale, then a rotation about Y by <see cref="RotationY" />, then
/// <see cref="Translation" />.</item>
/// <item><see cref="ViewerActorMotion.Walk" />: angle = <see cref="BaseAngle" /> + t x <see cref="AngularSpeed" />; scale,
/// then a rotation about Y by angle + PI / 2, then the translation (sin(angle), 0, cos(angle)) x <see cref="Radius" />.</item>
/// <item><see cref="ViewerActorMotion.Fly" />: angle = t x <see cref="AngularSpeed" />; scale, then a rotation about Y by
/// angle - PI / 2, then the translation (sin(angle) x radius, <see cref="Height" /> + sin(t x <see cref="BobRate" />) x
/// <see cref="BobAmplitude" />, cos(angle) x radius).</item>
/// </list>
/// </summary>
public sealed class ViewerActor
{
    /// <summary>Creates an actor.</summary>
    /// <param name="model">The model's name.</param>
    /// <param name="motion">How it moves.</param>
    /// <param name="clip">The clip it loops, or null for an actor with a routine.</param>
    /// <param name="routine">The clips it cross-fades through, or null for an actor with one clip.</param>
    /// <param name="timeOffset">The time offset, in seconds: a looping clip starts at this playhead; a routine runs this far ahead.</param>
    /// <param name="scale">The uniform scale.</param>
    public ViewerActor(string model, ViewerActorMotion motion, string clip, IReadOnlyList<string> routine, float timeOffset, float scale)
    {
        Model = model;
        Motion = motion;
        Clip = clip;
        Routine = routine ?? Array.Empty<string>();
        TimeOffset = timeOffset;
        Scale = scale;
    }

    /// <summary>The model's name (a model of <see cref="ModelLibrary" />).</summary>
    public string Model { get; }

    /// <summary>How it moves.</summary>
    public ViewerActorMotion Motion { get; }

    /// <summary>The clip it loops, or null when it has a <see cref="Routine" />.</summary>
    public string Clip { get; }

    /// <summary>The clips it cross-fades through, one every <see cref="ViewerSceneLayout.RoutineSeconds" /> (empty for an actor with one clip).</summary>
    public IReadOnlyList<string> Routine { get; }

    /// <summary>The time offset, in seconds.</summary>
    public float TimeOffset { get; }

    /// <summary>The uniform scale.</summary>
    public float Scale { get; }

    /// <summary>A fixed actor's rotation about Y, in radians.</summary>
    public float RotationY { get; init; }

    /// <summary>A fixed actor's translation.</summary>
    public Vector3 Translation { get; init; }

    /// <summary>A walking actor's angle on its ring at time 0, in radians.</summary>
    public float BaseAngle { get; init; }

    /// <summary>The radius of a walking or flying actor's circle.</summary>
    public float Radius { get; init; }

    /// <summary>How fast a walking or flying actor goes round, in radians per second (negative: the other way round).</summary>
    public float AngularSpeed { get; init; }

    /// <summary>A flying actor's height above the ground.</summary>
    public float Height { get; init; }

    /// <summary>How far a flying actor bobs up and down.</summary>
    public float BobAmplitude { get; init; }

    /// <summary>How fast a flying actor bobs, in radians per second.</summary>
    public float BobRate { get; init; }

    /// <summary>The world matrix at scene time <paramref name="time" />, exactly as the desktop viewer computes it.</summary>
    /// <param name="time">The scene time, in seconds.</param>
    /// <returns>The world matrix.</returns>
    public Matrix4x4 WorldAt(float time)
    {
        switch (Motion)
        {
            case ViewerActorMotion.Walk:
            {
                float angle = BaseAngle + (time * AngularSpeed);
                Vector3 position = new Vector3(MathF.Sin(angle), 0, MathF.Cos(angle)) * Radius;
                return Matrix4x4.CreateScale(Scale) * Matrix4x4.CreateRotationY(angle + (MathF.PI / 2)) * Matrix4x4.CreateTranslation(position);
            }

            case ViewerActorMotion.Fly:
            {
                float angle = time * AngularSpeed;
                Vector3 position = new Vector3(MathF.Sin(angle) * Radius, Height + (MathF.Sin(time * BobRate) * BobAmplitude), MathF.Cos(angle) * Radius);
                return Matrix4x4.CreateScale(Scale) * Matrix4x4.CreateRotationY(angle - (MathF.PI / 2)) * Matrix4x4.CreateTranslation(position);
            }

            default:
                return Matrix4x4.CreateScale(Scale) * Matrix4x4.CreateRotationY(RotationY) * Matrix4x4.CreateTranslation(Translation);
        }
    }
}
