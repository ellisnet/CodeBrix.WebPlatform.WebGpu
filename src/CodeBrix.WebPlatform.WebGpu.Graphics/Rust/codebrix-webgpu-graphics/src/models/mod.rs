//! glTF models with physically based shading: the model-data READER (the blob the Graphics
//! package's .NET `ModelDataWriter` makes on the backend), the model types, models on the GPU,
//! instances and the model renderer (C#: `Models/`), and level-of-detail objects (Not in C#).

mod data_reader;
mod lod_dither;
mod lod_instance;
mod material_textures;
mod model;
mod model_animation;
mod model_bounds;
mod model_data_set;
mod model_draw_plan;
mod model_instance;
mod model_layout;
mod model_material;
mod model_mesh;
mod model_node;
mod model_pose_edit;
mod model_renderer;
mod model_shadows;
mod model_vertex;
pub(crate) mod pbr_data;

#[cfg(test)]
mod fixture_tests;
#[cfg(test)]
mod lod_tests;

pub use lod_instance::{LodInstance, LodLevel, LodState};
pub use model::{Model, ModelLoadOptions};
pub use model_animation::{AnimationInterpolation, AnimationPath, ModelAnimation, ModelAnimationChannel};
pub use model_data_set::{ModelData, ModelDataFormat, ModelDataSet, ModelTextureData};
pub use model_instance::ModelInstance;
pub use model_material::{ModelMaterial, ModelTextureKind, ModelTextureSlot};
pub use model_mesh::{ModelMesh, ModelMorphTarget, ModelPrimitive};
pub use model_node::{ModelNode, ModelSkin, NodeTransform};
pub use model_pose_edit::ModelPoseEdit;
pub use model_renderer::{ModelRenderStats, ModelRenderer};
pub use model_shadows::SunShadowMap;
pub use model_shadows::SunShadowCascades;
pub(crate) use model_shadows::SunShadowNumbers;
#[cfg(test)]
pub(crate) use model_shadows::sun_shadow_numbers;
pub use model_vertex::{ModelSkinVertex, ModelVertex};
