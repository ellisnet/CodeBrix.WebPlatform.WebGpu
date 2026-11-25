using System.Collections.Generic;
using CodeBrix.WebPlatform.Core.Mapping;

namespace GraphicsDemo.Api.Contracts;

/// <summary>
/// The Viewer scene: the models it places and where the frontend gets each one (a model file served unchanged, or for
/// the ground, which is built in code, a model data blob), where each model is placed, the animated actors and how they
/// play and move, and the camera and lighting. The values are the desktop model viewer's.
/// </summary>
[WireRequired]
public sealed class ViewerScene
{
    /// <summary>The models the scene places, in index order: the ground first, then the actors' and the props' models in the order of their first use.</summary>
    public List<SceneModel> Models { get; set; } = new List<SceneModel>();

    /// <summary>The placed static models: the ground first (no transform), then the props.</summary>
    public List<ScenePlacement> Placements { get; set; } = new List<ScenePlacement>();

    /// <summary>The animated actors, in the desktop viewer's order: the chest, the eight animals, the Avian.</summary>
    public List<SceneActor> Actors { get; set; } = new List<SceneActor>();

    /// <summary>How long a standing animal keeps one clip of its routine before it cross-fades to the next, in seconds.</summary>
    public float RoutineSeconds { get; set; }

    /// <summary>How long a cross-fade between two clips of a routine takes, in seconds.</summary>
    public float CrossFadeSeconds { get; set; }

    /// <summary>The camera.</summary>
    public SceneCamera Camera { get; set; }

    /// <summary>The light and the clear colour.</summary>
    public SceneLighting Lighting { get; set; }
}
