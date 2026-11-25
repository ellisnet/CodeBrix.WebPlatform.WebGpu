//! Visibility: the convex volume a renderer culls against - a camera's view, or later a light's
//! (Not in C#: the C# library draws every instance it is given).

mod cull_volume;

pub use cull_volume::CullVolume;
