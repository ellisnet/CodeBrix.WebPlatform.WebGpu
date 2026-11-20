//! 2D drawing: the sprite batch for sprites and shapes (C#: `Drawing2D/`). No text: on-screen
//! text is the Animation package's, as a layer of the canvas host.

mod sprite_batch;
mod sprite_builder;
mod sprite_vertex;

pub use sprite_batch::{CircleOptions, SpriteBatch, SpriteBatchBeginOptions};
pub(crate) use sprite_vertex::SpriteVertex;
