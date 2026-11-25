//! What the page keeps of what it fetched from the backend, for the life of the page: the bytes
//! of each model file, model-data blob and panorama, and the Viewer's scene description. A scene that is
//! selected again takes them from here and asks the backend for nothing; a failed request is not
//! kept, so the next visit asks again.
//!
//! This is the application's own memory (a page that never switches scenes would need none).
//! Across a page RELOAD the browser's HTTP cache takes over: the platform's client reads a file
//! answer as bytes with the fetch cache mode "no-cache", so the browser KEEPS the answer and
//! REVALIDATES it with the backend on the next request (sending the ETag back); the backend
//! answers 304 with no body while the file is unchanged, and the browser hands over the bytes it
//! kept (see the sample's MAINTAINER-README).

use std::cell::RefCell;
use std::collections::HashMap;
use std::future::Future;
use std::rc::Rc;

use crate::generated::models::ViewerScene as ViewerSceneData;

thread_local! {
    /// The bytes fetched so far, by the name of what they are ("model-file:survival-kit/tree.glb",
    /// "model:ground", ...).
    static BYTES: RefCell<HashMap<String, Rc<Vec<u8>>>> = RefCell::new(HashMap::new());
    /// The Viewer's scene description, once fetched.
    static VIEWER_SCENE: RefCell<Option<ViewerSceneData>> = const { RefCell::new(None) };
}

/// The bytes kept under `key`, or those `fetch` brings (kept under `key` when it succeeds).
pub async fn bytes<F>(key: &str, fetch: F) -> Result<Rc<Vec<u8>>, String>
where
    F: Future<Output = Result<Vec<u8>, String>>,
{
    if let Some(kept) = BYTES.with(|bytes| bytes.borrow().get(key).cloned()) {
        return Ok(kept);
    }
    let fetched = Rc::new(fetch.await?);
    BYTES.with(|bytes| bytes.borrow_mut().insert(key.to_string(), fetched.clone()));
    Ok(fetched)
}

/// The Viewer's scene description kept, or the one `fetch` brings (kept when it succeeds).
pub async fn viewer_scene<F>(fetch: F) -> Result<ViewerSceneData, String>
where
    F: Future<Output = Result<ViewerSceneData, String>>,
{
    if let Some(kept) = VIEWER_SCENE.with(|scene| scene.borrow().clone()) {
        return Ok(kept);
    }
    let fetched = fetch.await?;
    VIEWER_SCENE.with(|scene| *scene.borrow_mut() = Some(fetched.clone()));
    Ok(fetched)
}

/// The bytes kept under `key`, if they were fetched before (without asking for them).
pub fn kept_bytes(key: &str) -> Option<Rc<Vec<u8>>> {
    BYTES.with(|bytes| bytes.borrow().get(key).cloned())
}
