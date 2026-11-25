//! The demos' models: the kits' .glb files the backend serves (and the colour maps they name),
//! fetched once for the page and read by the graphics crate's glTF loader, and the ground, a model
//! the backend builds in code, from its blob. Every demo builds its OWN GPU models from these bytes
//! through one GltfModelSet (each kit's colour map decoded and uploaded once per demo); a demo that
//! changes a model's materials (glass, glowing copies) asks for a separate copy, whose only handle
//! it holds while it changes them.

use std::collections::HashMap;
use std::rc::Rc;

use codebrix_webgpu_graphics::{GltfLoadOptions, GltfModelSet, GraphicsContext, GraphicsException, Model, ModelLoadOptions};

use crate::generated::api::models;
use crate::model_files::{self, ModelFileRequest, ModelFiles};
use crate::page_cache;

/// The models the demos use: (name, file below the models folder, pack).
const MODEL_FILES: [(&str, &str, &str); 16] = [
    ("archer", "kenney-mini-forest/character-archer.glb", "kenney-mini-forest"),
    ("tree", "survival-kit/tree.glb", "survival-kit"),
    ("tree-tall", "survival-kit/tree-tall.glb", "survival-kit"),
    ("tree-autumn", "survival-kit/tree-autumn.glb", "survival-kit"),
    ("rock-a", "survival-kit/rock-a.glb", "survival-kit"),
    ("rock-b", "survival-kit/rock-b.glb", "survival-kit"),
    ("rock-c", "survival-kit/rock-c.glb", "survival-kit"),
    ("barrel", "survival-kit/barrel.glb", "survival-kit"),
    ("chest", "survival-kit/chest.glb", "survival-kit"),
    ("campfire", "survival-kit/campfire-pit.glb", "survival-kit"),
    ("fence", "survival-kit/fence.glb", "survival-kit"),
    ("fox", "cube-pets/animal-fox.glb", "cube-pets"),
    ("dog", "cube-pets/animal-dog.glb", "cube-pets"),
    ("cat", "cube-pets/animal-cat.glb", "cube-pets"),
    ("cow", "cube-pets/animal-cow.glb", "cube-pets"),
    ("avian", "crystals-and-avian/avian.glb", "crystals-and-avian"),
];

/// The bytes the demos' models come from.
#[derive(Clone)]
pub struct KitBytes {
    /// The model files (parsed) and the files they name.
    pub files: ModelFiles,
    /// The ground's model data blob.
    pub ground: Rc<Vec<u8>>,
}

impl PartialEq for KitBytes {
    fn eq(&self, other: &KitBytes) -> bool {
        self.files == other.files && Rc::ptr_eq(&self.ground, &other.ground)
    }
}

/// Fetches the model files and the ground's blob, or takes them from what the page kept.
pub async fn load() -> Result<KitBytes, String> {
    let requests: Vec<ModelFileRequest> = MODEL_FILES.iter().map(|(name, file, pack)| ModelFileRequest { name: name.to_string(), file: file.to_string(), pack: pack.to_string() }).collect();
    let files = model_files::fetch(&requests).await?;
    let ground = page_cache::bytes("model:ground", async { models::model_data_bytes("ground").await.map_err(|e| format!("The ground could not be read from the backend: {}", e.message())) }).await?;
    Ok(KitBytes { files, ground })
}

/// Builds a demo's GPU models from the kit's bytes.
pub struct Kit<'a> {
    /// The context the models are made on.
    pub graphics: &'a GraphicsContext,
    bytes: &'a KitBytes,
    set: GltfModelSet,
    made: HashMap<String, Model>,
}

impl<'a> Kit<'a> {
    /// A kit over `bytes`.
    pub fn new(graphics: &'a GraphicsContext, bytes: &'a KitBytes) -> Kit<'a> {
        Kit { graphics, bytes, set: GltfModelSet::new(), made: HashMap::new() }
    }

    /// Builds one more model from the model file `name` into the set (sharing the kit's textures).
    fn build(&mut self, name: &str) -> Result<Model, String> {
        let index = self.bytes.files.models.iter().position(|m| m.name == name).ok_or_else(|| format!("The demos have no model named '{name}'."))?;
        model_files::add_models(&mut self.set, &self.bytes.files, &[index], &GltfLoadOptions::default())?;
        let made = self.set.create_models(self.graphics, &ModelLoadOptions::default()).map_err(|e| e.to_string())?;
        made.into_iter().last().ok_or_else(|| format!("The model file '{name}' gave no model."))
    }

    /// The model `name` (one GPU model per name, shared by every instance of it).
    pub fn model(&mut self, name: &str) -> Result<Model, String> {
        if let Some(model) = self.made.get(name) {
            return Ok(model.clone());
        }
        let model = self.build(name)?;
        self.made.insert(name.to_string(), model.clone());
        Ok(model)
    }

    /// A SEPARATE copy of the model `name`, changed by `edit` while it is the only handle (its
    /// materials: glass, an emissive copy).
    pub fn changed(&mut self, name: &str, edit: impl FnOnce(&mut Model) -> Result<(), GraphicsException>) -> Result<Model, String> {
        let mut model = self.build(name)?;
        edit(&mut model).map_err(|e| format!("The copy of '{name}' could not be changed: {e}"))?;
        Ok(model)
    }

    /// The ground (90 m either way from the centre), from its blob.
    pub fn ground(&mut self) -> Result<Model, String> {
        if let Some(model) = self.made.get("ground") {
            return Ok(model.clone());
        }
        let model = self.graphics.load_model(&self.bytes.ground, &ModelLoadOptions::default()).map_err(|e| e.to_string())?;
        self.made.insert("ground".to_string(), model.clone());
        Ok(model)
    }
}

/// The page's clock in milliseconds (for read-outs only; the scenes' time comes from the frame).
pub fn now_ms() -> f64 {
    model_files::now_ms()
}
