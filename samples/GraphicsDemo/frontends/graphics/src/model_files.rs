//! The MODEL FILES a scene draws: .glb files the backend serves UNCHANGED (api/models/files/...),
//! read here by the graphics crate's glTF loader. Nothing is converted on the backend.
//!
//! Loading is the loader's two steps, with the application doing the fetching (the crate does no
//! networking, and a browser cannot wait for a fetch inside a call):
//!
//! 1. [`fetch`]: for each model, fetch its file through the generated client, parse it
//!    ([`GltfDocument::parse`]) and ask it which files it names
//!    ([`GltfDocument::external_files`], with the model's PACK FOLDER as the base address - so the
//!    pack's shared `Textures/colormap.png` is ONE address, `<pack>/Textures/colormap.png`, for
//!    every model of the pack), and fetch each of those once. Every file's bytes are kept for the
//!    life of the page (page_cache.rs).
//! 2. In the layer, [`add_models`] builds each model into the scene's ONE [`GltfModelSet`], the
//!    file function answering from what step 1 fetched; then `GltfModelSet::create_models` makes
//!    the GPU models. An image named by several models of the set is decoded and uploaded once.
//!
//! The addresses map onto the backend's two [WireFile] actions: `<pack>/<file>` is
//! `models::model_file_bytes(pack, file)` and `<pack>/<folder>/<file>` is
//! `models::model_folder_file_bytes(pack, folder, file)`.

use std::collections::HashMap;
use std::rc::Rc;

use codebrix_webgpu_graphics::{GltfDocument, GltfLoadOptions, GltfModelSet};

use crate::generated::api::models;
use crate::page_cache;

/// One model file a scene asks for.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ModelFileRequest {
    /// The model's name (the file's name without its extension, such as "tree").
    pub name: String,
    /// The file's path below the backend's models folder, such as "survival-kit/tree.glb".
    pub file: String,
    /// The folder the file is in (its pack, such as "survival-kit"): the base address of the files
    /// it names.
    pub pack: String,
}

/// One parsed model file.
pub struct ModelFileEntry {
    /// The model's name.
    pub name: String,
    /// The model's pack folder (the base address of the files it names).
    pub pack: String,
    /// The parsed document.
    pub document: GltfDocument,
}

/// What [`fetch`] brought: each model's parsed document, in the order asked for, and the bytes of
/// every file a model names, by address. Cheap to clone (shared).
#[derive(Clone)]
pub struct ModelFiles {
    /// The models, in the order they were asked for.
    pub models: Rc<Vec<ModelFileEntry>>,
    /// The bytes of each external file (textures, buffers), by the address the loader asks for.
    pub files: Rc<HashMap<String, Rc<Vec<u8>>>>,
    /// The bytes of every distinct file fetched (the model files and the files they name).
    pub bytes: usize,
    /// The number of distinct files fetched.
    pub file_count: usize,
    /// How long fetching and parsing took (milliseconds of the page's clock), 0 when everything
    /// came from what the page kept.
    pub fetch_ms: f64,
}

impl PartialEq for ModelFiles {
    fn eq(&self, other: &ModelFiles) -> bool {
        Rc::ptr_eq(&self.models, &other.models) && Rc::ptr_eq(&self.files, &other.files)
    }
}

/// Fetches an address below the backend's models folder through the generated client: its
/// `<pack>/<file>` and `<pack>/<folder>/<file>` actions. The bytes are kept for the life of the
/// page under the address (a second scene asks for nothing).
async fn fetch_address(address: &str) -> Result<Rc<Vec<u8>>, String> {
    let parts: Vec<&str> = address.split('/').collect();
    let key = format!("model-file:{address}");
    let message = |e: codebrix_webplatform::api::ApiError| format!("The model file '{address}' could not be read from the backend: {}", e.message());
    match parts.as_slice() {
        [pack, file] => page_cache::bytes(&key, async { models::model_file_bytes(pack, file).await.map_err(message) }).await,
        [pack, folder, file] => page_cache::bytes(&key, async { models::model_folder_file_bytes(pack, folder, file).await.map_err(message) }).await,
        _ => Err(format!("The model file '{address}' is not one the backend serves: a model's files are <pack>/<file> or <pack>/<folder>/<file>.")),
    }
}

/// Fetches and parses the model files, then fetches every file they name (each address once).
pub async fn fetch(requests: &[ModelFileRequest]) -> Result<ModelFiles, String> {
    let started = now_ms();
    let mut models = Vec::with_capacity(requests.len());
    let mut files: HashMap<String, Rc<Vec<u8>>> = HashMap::new();
    let mut bytes = 0usize;
    let mut file_count = 0usize;
    for request in requests {
        let model = fetch_address(&request.file).await?;
        bytes += model.len();
        file_count += 1;
        let document = GltfDocument::parse(&model).map_err(|e| format!("The model file '{}' could not be read: {e}", request.file))?;
        for external in document.external_files(Some(&request.pack), true) {
            if files.contains_key(&external.address) {
                continue;
            }
            let file = fetch_address(&external.address).await?;
            bytes += file.len();
            file_count += 1;
            files.insert(external.address, file);
        }
        models.push(ModelFileEntry { name: request.name.clone(), pack: request.pack.clone(), document });
    }
    Ok(ModelFiles { models: Rc::new(models), files: Rc::new(files), bytes, file_count, fetch_ms: ((now_ms() - started) * 10.0).round() / 10.0 })
}

/// Builds the models `which` names (indices into `files.models`) into `set`, in that order, each
/// with its pack folder as the base address, the file function answering from what [`fetch`]
/// brought, with `options`. Returns each model's index in the set.
pub fn add_models(set: &mut GltfModelSet, files: &ModelFiles, which: &[usize], options: &GltfLoadOptions) -> Result<Vec<usize>, String> {
    let lookup = |address: &str| files.files.get(address).map(|bytes| bytes.as_slice());
    let mut added = Vec::with_capacity(which.len());
    for &index in which {
        let Some(entry) = files.models.get(index) else {
            return Err(format!("There is no model {index} among the {} fetched.", files.models.len()));
        };
        let at = set.add(&entry.document, &entry.name, Some(&entry.pack), &lookup, options).map_err(|e| format!("The model '{}' could not be built: {e}", entry.name))?;
        added.push(at);
    }
    Ok(added)
}

/// The page's clock in milliseconds (for the load-time read-outs only).
pub fn now_ms() -> f64 {
    web_sys::window().and_then(|w| w.performance()).map_or(0.0, |p| p.now())
}
