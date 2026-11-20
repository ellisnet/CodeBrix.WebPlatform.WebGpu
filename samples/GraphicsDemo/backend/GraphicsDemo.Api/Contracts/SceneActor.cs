using System.Collections.Generic;
using CodeBrix.WebPlatform.Core.Mapping;

namespace GraphicsDemo.Api.Contracts;

/// <summary>
/// One animated actor of the scene: a model of the scene's blob, the clip it loops (or the routine of clips it
/// cross-fades through) and how it moves - exactly the desktop model viewer's placement functions of the scene time t:
/// "fixed": scale, then the rotation about Y, then the translation; "walk": angle = baseAngle + t x angularSpeed, scale,
/// then a rotation about Y by angle + PI / 2, then (sin(angle), 0, cos(angle)) x radius; "fly": angle = t x
/// angularSpeed, scale, then a rotation about Y by angle - PI / 2, then (sin(angle) x radius, height + sin(t x bobRate)
/// x bobAmplitude, cos(angle) x radius).
/// </summary>
[WireRequired]
public sealed class SceneActor
{
    /// <summary>The model's name.</summary>
    public string Model { get; set; }

    /// <summary>The model's index in the scene's model data blob.</summary>
    public int ModelIndex { get; set; }

    /// <summary>How it moves: "fixed", "walk" or "fly".</summary>
    public string Motion { get; set; }

    /// <summary>The clip it loops, or an empty string for an actor with a routine.</summary>
    public string Clip { get; set; }

    /// <summary>The clips it cross-fades through, one every <see cref="ViewerScene.RoutineSeconds" /> (empty for an actor with one clip).</summary>
    public List<string> Routine { get; set; } = new List<string>();

    /// <summary>The time offset in seconds: a looping clip starts at this playhead; a routine runs this far ahead.</summary>
    public float TimeOffset { get; set; }

    /// <summary>The uniform scale.</summary>
    public float Scale { get; set; }

    /// <summary>A fixed actor's rotation about Y, in radians.</summary>
    public float RotationY { get; set; }

    /// <summary>A fixed actor's translation.</summary>
    public SceneVector Translation { get; set; }

    /// <summary>A walking actor's angle on its ring at time 0, in radians.</summary>
    public float BaseAngle { get; set; }

    /// <summary>The radius of a walking or flying actor's circle.</summary>
    public float Radius { get; set; }

    /// <summary>How fast a walking or flying actor goes round, in radians per second.</summary>
    public float AngularSpeed { get; set; }

    /// <summary>A flying actor's height.</summary>
    public float Height { get; set; }

    /// <summary>How far a flying actor bobs up and down.</summary>
    public float BobAmplitude { get; set; }

    /// <summary>How fast a flying actor bobs, in radians per second.</summary>
    public float BobRate { get; set; }
}
