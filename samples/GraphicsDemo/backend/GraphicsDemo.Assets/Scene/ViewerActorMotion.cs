namespace GraphicsDemo.Assets.Scene;

/// <summary>How an actor of the Viewer scene moves, as the desktop model viewer places it.</summary>
public enum ViewerActorMotion
{
    /// It stays where it is placed (the chest, the standing animals).
    Fixed,

    /// It walks a ring around the centre, facing along the ring (the walking animals).
    Walk,

    /// It flies a circle overhead, bobbing up and down (the Avian).
    Fly,
}
