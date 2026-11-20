//! The layers of a canvas: the ORDERED list of renderers the canvas host draws, the first at the
//! bottom. The application puts them together (renderers from different crates know nothing of
//! each other) and may add, remove and reorder them while the canvas runs.

use std::cell::RefCell;
use std::rc::Rc;

use crate::renderer::{CanvasTarget, RendererHandle};

/// The ordered renderers of a canvas, bottom first. Clone it to share it: the clones are the same
/// list, so the page can change the layers of a running canvas (keep it in the page with
/// `use_hook` and hand the same one to [`WebGpuCanvas`](crate::WebGpuCanvas)).
///
/// THE LAYER CONTRACT. Every frame the host clears the target (to the canvas's clear colour),
/// then hands the SAME frame - one encoder, one view, one target, one clock time - to each layer
/// in this order; a layer draws OVER what the layers below it drew (its render passes load the
/// target, `LoadOp::Load`, and never clear it). A layer added while the canvas runs is prepared
/// (and sized) before its first frame; a layer removed is simply no longer drawn. Input goes to
/// every layer, in the same order. A renderer is a layer of one canvas at a time.
#[derive(Clone, Default)]
pub struct CanvasLayers {
    inner: Rc<RefCell<Vec<RendererHandle>>>,
}

impl CanvasLayers {
    /// The layers, bottom first.
    pub fn new(layers: Vec<RendererHandle>) -> CanvasLayers {
        let mut list = Vec::new();
        for layer in layers {
            if !list.contains(&layer) {
                list.push(layer);
            }
        }
        CanvasLayers { inner: Rc::new(RefCell::new(list)) }
    }

    /// One layer.
    pub fn single(layer: RendererHandle) -> CanvasLayers {
        CanvasLayers::new(vec![layer])
    }

    /// Puts a layer on top. A layer that is already in the list moves to the top.
    pub fn push(&self, layer: RendererHandle) {
        let mut list = self.inner.borrow_mut();
        list.retain(|l| *l != layer);
        list.push(layer);
    }

    /// Puts a layer at `index` (0 is the bottom; an index past the top puts it on top). A layer
    /// that is already in the list moves there.
    pub fn insert(&self, index: usize, layer: RendererHandle) {
        let mut list = self.inner.borrow_mut();
        list.retain(|l| *l != layer);
        let index = index.min(list.len());
        list.insert(index, layer);
    }

    /// Takes a layer out; false when it was not in the list.
    pub fn remove(&self, layer: &RendererHandle) -> bool {
        let mut list = self.inner.borrow_mut();
        let before = list.len();
        list.retain(|l| l != layer);
        list.len() != before
    }

    /// True when the layer is in the list.
    pub fn contains(&self, layer: &RendererHandle) -> bool {
        self.inner.borrow().contains(layer)
    }

    /// Where the layer is (0 is the bottom), or None.
    pub fn position(&self, layer: &RendererHandle) -> Option<usize> {
        self.inner.borrow().iter().position(|l| l == layer)
    }

    /// The number of layers.
    pub fn len(&self) -> usize {
        self.inner.borrow().len()
    }

    /// True when there are no layers (the canvas then shows only its clear colour).
    pub fn is_empty(&self) -> bool {
        self.inner.borrow().is_empty()
    }

    /// The layers as they are now, bottom first (the host draws from such a copy, so a layer
    /// may change the list while it draws; the change applies from the next frame).
    pub fn snapshot(&self) -> Vec<RendererHandle> {
        self.inner.borrow().clone()
    }
}

impl PartialEq for CanvasLayers {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.inner, &other.inner)
    }
}

/// What the host has told one layer: the target it was prepared for and the size it was given.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct LayerState {
    prepared: Option<CanvasTarget>,
    size: Option<(u32, u32)>,
}

/// What a layer must be told before it draws a frame.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct LayerCalls {
    pub(crate) prepare: bool,
    pub(crate) resize: bool,
}

impl LayerState {
    /// The calls a layer needs before drawing into `target` at `size`, and the state after them.
    pub(crate) fn before_frame(&mut self, target: CanvasTarget, size: (u32, u32)) -> LayerCalls {
        let prepare = self.prepared != Some(target);
        let resize = prepare || self.size != Some(size);
        self.prepared = Some(target);
        self.size = Some(size);
        LayerCalls { prepare, resize }
    }
}

#[cfg(test)]
mod tests {
    use super::{CanvasLayers, LayerCalls, LayerState};
    use crate::renderer::{CanvasFrame, CanvasRenderer, CanvasTarget, RendererHandle};

    struct Nothing;

    impl CanvasRenderer for Nothing {
        fn render(&mut self, _frame: &mut CanvasFrame<'_>) {}
    }

    fn target() -> CanvasTarget {
        CanvasTarget { format: wgpu::TextureFormat::Bgra8UnormSrgb, surface_format: wgpu::TextureFormat::Bgra8Unorm, alpha_mode: wgpu::CompositeAlphaMode::Opaque }
    }

    #[test]
    fn layers_keep_their_order_and_move_when_added_again() {
        let (a, b, c) = (RendererHandle::new(Nothing), RendererHandle::new(Nothing), RendererHandle::new(Nothing));
        let layers = CanvasLayers::new(vec![a.clone(), b.clone(), a.clone()]);
        assert_eq!(layers.snapshot(), vec![a.clone(), b.clone()]);
        layers.push(c.clone());
        layers.push(a.clone());
        assert_eq!(layers.snapshot(), vec![b.clone(), c.clone(), a.clone()]);
        layers.insert(0, a.clone());
        assert_eq!(layers.position(&a), Some(0));
        layers.insert(99, b.clone());
        assert_eq!(layers.snapshot(), vec![a.clone(), c.clone(), b.clone()]);
    }

    #[test]
    fn a_removed_layer_is_gone_and_removing_it_again_says_so() {
        let (a, b) = (RendererHandle::new(Nothing), RendererHandle::new(Nothing));
        let layers = CanvasLayers::new(vec![a.clone(), b.clone()]);
        assert!(layers.remove(&a));
        assert!(!layers.remove(&a));
        assert!(!layers.contains(&a) && layers.contains(&b));
        assert_eq!(layers.len(), 1);
        assert!(layers.remove(&b) && layers.is_empty());
    }

    #[test]
    fn the_clones_are_the_same_list() {
        let layers = CanvasLayers::default();
        let clone = layers.clone();
        clone.push(RendererHandle::new(Nothing));
        assert_eq!(layers.len(), 1);
        assert!(layers == clone && layers != CanvasLayers::default());
    }

    #[test]
    fn a_new_layer_is_prepared_and_sized_once() {
        let mut state = LayerState::default();
        assert_eq!(state.before_frame(target(), (800, 600)), LayerCalls { prepare: true, resize: true });
        assert_eq!(state.before_frame(target(), (800, 600)), LayerCalls { prepare: false, resize: false });
    }

    #[test]
    fn a_new_size_is_a_resize_only() {
        let mut state = LayerState::default();
        state.before_frame(target(), (800, 600));
        assert_eq!(state.before_frame(target(), (400, 300)), LayerCalls { prepare: false, resize: true });
    }
}
