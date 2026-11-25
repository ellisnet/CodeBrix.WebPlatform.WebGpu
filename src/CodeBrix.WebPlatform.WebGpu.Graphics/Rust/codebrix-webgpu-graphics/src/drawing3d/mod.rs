//! 3D drawing: cameras, the orbit controls and picking rays, mesh data and the built-in
//! primitives, meshes on the GPU, lit materials, scene lighting and the lit-mesh renderer (C#:
//! `Drawing3D/`).

mod camera;
mod lit_material;
mod mesh;
mod mesh_data;
mod mesh_renderer;
mod mesh_shapes;
mod mesh_vertex;
mod orbit_camera;
mod orbit_controller;
mod ray;
mod scene_lighting;

pub use camera::{Camera, CanvasProjection, ProjectionKind};
pub use lit_material::{LitMaterial, MaterialAlphaMode, MaterialOptions};
pub use mesh::Mesh;
pub use mesh_data::MeshData;
pub use mesh_renderer::MeshRenderer;
pub use mesh_shapes::MeshShapes;
pub use mesh_vertex::MeshVertex;
pub use orbit_camera::OrbitCamera;
pub use orbit_controller::{OrbitController, OrbitLimits};
pub use ray::Ray;
pub use scene_lighting::{AmbientLighting, DirectionalLight, SceneLighting};
