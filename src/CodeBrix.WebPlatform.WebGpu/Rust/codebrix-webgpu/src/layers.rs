//! The layers of a canvas: the ORDERED list of renderers the canvas host draws, the first at the
//! bottom. The application puts them together (renderers from different crates know nothing of
//! each other) and may add, remove and reorder them while the canvas runs.

use std::cell::RefCell;
use std::rc::Rc;

use crate::layer_time::LayerTime;
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
///
/// A LAYER'S OWN TIME SCALE (Not in C#, [`CanvasLayers::set_time_scale`]): one layer may run
/// faster or slower than the clock - its frame's `time`, `delta` and `previous_time` (and so the
/// named points it reaches) are on its own timeline; every other layer keeps the clock's.
#[derive(Clone, Default)]
pub struct CanvasLayers {
    inner: Rc<RefCell<LayerList>>,
}

/// The layers with their time scales, and a count of the changes.
#[derive(Default)]
struct LayerList {
    entries: Vec<(RendererHandle, f64)>,
    /// Raised by every change of the list and by every redraw request: a canvas that idles while
    /// its clock is paused draws again when it moves.
    changes: u64,
}

impl LayerList {
    fn changed(&mut self) {
        self.changes = self.changes.wrapping_add(1);
    }
}

impl CanvasLayers {
    /// The layers, bottom first.
    pub fn new(layers: Vec<RendererHandle>) -> CanvasLayers {
        let mut entries: Vec<(RendererHandle, f64)> = Vec::new();
        for layer in layers {
            if !entries.iter().any(|(known, _)| *known == layer) {
                entries.push((layer, 1.0));
            }
        }
        CanvasLayers { inner: Rc::new(RefCell::new(LayerList { entries, changes: 0 })) }
    }

    /// One layer.
    pub fn single(layer: RendererHandle) -> CanvasLayers {
        CanvasLayers::new(vec![layer])
    }

    /// Puts a layer on top. A layer that is already in the list moves to the top (keeping its time
    /// scale).
    pub fn push(&self, layer: RendererHandle) {
        let mut list = self.inner.borrow_mut();
        let scale = take_scale(&mut list.entries, &layer);
        list.entries.push((layer, scale));
        list.changed();
    }

    /// Puts a layer at `index` (0 is the bottom; an index past the top puts it on top). A layer
    /// that is already in the list moves there (keeping its time scale).
    pub fn insert(&self, index: usize, layer: RendererHandle) {
        let mut list = self.inner.borrow_mut();
        let scale = take_scale(&mut list.entries, &layer);
        let index = index.min(list.entries.len());
        list.entries.insert(index, (layer, scale));
        list.changed();
    }

    /// Takes a layer out (its time scale is forgotten); false when it was not in the list.
    pub fn remove(&self, layer: &RendererHandle) -> bool {
        let mut list = self.inner.borrow_mut();
        let before = list.entries.len();
        list.entries.retain(|(l, _)| l != layer);
        let removed = list.entries.len() != before;
        if removed {
            list.changed();
        }
        removed
    }

    /// True when the layer is in the list.
    pub fn contains(&self, layer: &RendererHandle) -> bool {
        self.inner.borrow().entries.iter().any(|(l, _)| l == layer)
    }

    /// Where the layer is (0 is the bottom), or None.
    pub fn position(&self, layer: &RendererHandle) -> Option<usize> {
        self.inner.borrow().entries.iter().position(|(l, _)| l == layer)
    }

    /// The number of layers.
    pub fn len(&self) -> usize {
        self.inner.borrow().entries.len()
    }

    /// True when there are no layers (the canvas then shows only its clear colour).
    pub fn is_empty(&self) -> bool {
        self.inner.borrow().entries.is_empty()
    }

    /// The layers as they are now, bottom first (the host draws from such a copy, so a layer
    /// may change the list while it draws; the change applies from the next frame).
    pub fn snapshot(&self) -> Vec<RendererHandle> {
        self.inner.borrow().entries.iter().map(|(l, _)| l.clone()).collect()
    }

    /// Sets how fast one layer's time runs against the clock (Not in C#): 1 (the default) is the
    /// clock's own time, 2 twice as fast, 0.5 half as fast - for example the ride of a scene sped
    /// up while the forest around it keeps the clock's time. A negative or non-finite scale is
    /// taken as 0 (the layer's time stands still). The layer's time is the scale times the
    /// clock's time while the scale has not changed, so a seek of the clock shows exactly what
    /// playing shows; a CHANGE of the scale goes on from the layer's time at its last frame
    /// (no jump), along the new slope. Pausing the clock stops every layer. False when the layer
    /// is not in the list (add it first).
    pub fn set_time_scale(&self, layer: &RendererHandle, scale: f64) -> bool {
        let scale = if scale.is_finite() && scale > 0.0 { scale } else { 0.0 };
        let mut list = self.inner.borrow_mut();
        let Some(index) = list.entries.iter().position(|(l, _)| l == layer) else {
            return false;
        };
        list.entries[index].1 = scale;
        list.changed();
        true
    }

    /// The layer's time scale (Not in C#): 1 unless [`CanvasLayers::set_time_scale`] changed it,
    /// and 1 for a layer that is not in the list.
    pub fn time_scale(&self, layer: &RendererHandle) -> f64 {
        self.inner.borrow().entries.iter().find(|(l, _)| l == layer).map_or(1.0, |(_, scale)| *scale)
    }

    /// Asks for one more frame of a canvas that idles while its clock is paused (Not in C#;
    /// `WebGpuCanvas`'s `idle_when_paused`): call it after changing what a layer draws from
    /// outside the canvas - a session edited by the page, a picture that finished loading - so
    /// that the change shows although the clock stands still. A canvas that draws every frame
    /// (the default) needs no request.
    pub fn request_redraw(&self) {
        self.inner.borrow_mut().changed();
    }

    /// The layers with their time scales, bottom first.
    pub(crate) fn snapshot_with_scales(&self) -> Vec<(RendererHandle, f64)> {
        self.inner.borrow().entries.clone()
    }

    /// The count of changes and redraw requests so far.
    pub(crate) fn changes(&self) -> u64 {
        self.inner.borrow().changes
    }
}

/// Takes `layer` out of `entries`, returning its time scale (1 for a layer that was not there).
fn take_scale(entries: &mut Vec<(RendererHandle, f64)>, layer: &RendererHandle) -> f64 {
    match entries.iter().position(|(l, _)| l == layer) {
        Some(index) => entries.remove(index).1,
        None => 1.0,
    }
}

impl PartialEq for CanvasLayers {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.inner, &other.inner)
    }
}

/// What the host has told one layer: the target it was prepared for, the size it was given, and
/// its own timeline (the clock's unless the layer has a time scale).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct LayerState {
    prepared: Option<CanvasTarget>,
    size: Option<(u32, u32)>,
    pub(crate) time: LayerTime,
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
    fn a_layer_keeps_its_time_scale_when_it_moves_and_forgets_it_when_removed() {
        //Arrange
        let (a, b) = (RendererHandle::new(Nothing), RendererHandle::new(Nothing));
        let layers = CanvasLayers::new(vec![a.clone(), b.clone()]);
        //Act
        let set = layers.set_time_scale(&a, 2.5);
        layers.push(a.clone());
        layers.insert(0, a.clone());
        //Assert
        assert!(set);
        assert_eq!((layers.time_scale(&a), layers.time_scale(&b)), (2.5, 1.0));
        assert_eq!(layers.snapshot_with_scales().iter().map(|(_, s)| *s).collect::<Vec<_>>(), vec![2.5, 1.0]);
        assert!(layers.remove(&a));
        layers.push(a.clone());
        assert_eq!(layers.time_scale(&a), 1.0);
    }

    #[test]
    fn a_time_scale_is_set_only_on_a_layer_of_the_list_and_never_below_zero() {
        let (a, outside) = (RendererHandle::new(Nothing), RendererHandle::new(Nothing));
        let layers = CanvasLayers::single(a.clone());
        assert!(!layers.set_time_scale(&outside, 2.0));
        assert_eq!(layers.time_scale(&outside), 1.0);
        layers.set_time_scale(&a, -3.0);
        assert_eq!(layers.time_scale(&a), 0.0);
        layers.set_time_scale(&a, f64::INFINITY);
        assert_eq!(layers.time_scale(&a), 0.0);
        layers.set_time_scale(&a, 0.25);
        assert_eq!(layers.time_scale(&a), 0.25);
    }

    #[test]
    fn every_change_and_redraw_request_is_counted_for_all_clones() {
        //Arrange
        let (a, b) = (RendererHandle::new(Nothing), RendererHandle::new(Nothing));
        let layers = CanvasLayers::single(a.clone());
        let clone = layers.clone();
        let mut seen = vec![layers.changes()];
        //Act
        layers.push(b.clone());
        seen.push(clone.changes());
        layers.insert(0, b.clone());
        seen.push(clone.changes());
        layers.set_time_scale(&a, 2.0);
        seen.push(clone.changes());
        clone.request_redraw();
        seen.push(layers.changes());
        layers.remove(&b);
        seen.push(clone.changes());
        let unchanged = clone.changes();
        layers.remove(&b);
        layers.set_time_scale(&b, 2.0);
        //Assert
        assert!(seen.windows(2).all(|w| w[1] > w[0]), "{seen:?}");
        assert_eq!(clone.changes(), unchanged, "nothing changed");
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
