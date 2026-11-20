//! glTF models with physically based shading: the model-data READER (the blob the Graphics
//! package's .NET `ModelDataWriter` makes on the backend), the model types, models on the GPU,
//! instances and the model renderer (C#: `Models/`).

mod data_reader;
mod model;
mod model_animation;
mod model_data_set;
mod model_instance;
mod model_layout;
mod model_material;
mod model_mesh;
mod model_node;
mod model_pose_edit;
mod model_renderer;
mod model_vertex;
pub(crate) mod pbr_data;

#[cfg(test)]
mod fixture_tests;

pub use model::{Model, ModelLoadOptions};
pub use model_animation::{AnimationInterpolation, AnimationPath, ModelAnimation, ModelAnimationChannel};
pub use model_data_set::{ModelData, ModelDataFormat, ModelDataSet, ModelTextureData};
pub use model_instance::ModelInstance;
pub use model_material::{ModelMaterial, ModelTextureSlot};
pub use model_mesh::{ModelMesh, ModelMorphTarget, ModelPrimitive};
pub use model_node::{ModelNode, ModelSkin, NodeTransform};
pub use model_pose_edit::ModelPoseEdit;
pub use model_renderer::ModelRenderer;
pub use model_vertex::{ModelSkinVertex, ModelVertex};
