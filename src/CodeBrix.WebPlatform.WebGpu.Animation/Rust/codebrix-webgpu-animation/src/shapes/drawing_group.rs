//! `DrawingGroup`: elements drawn as one - one transform, one opacity (faded as a unit), one
//! content colour, one mask (C# `DrawingGroup`).

use crate::color::Color;
use crate::effects::DrawingMask;
use crate::geometry::RectangleF;
use crate::models::ElementId;
use crate::shapes::DrawingShape;

/// A group of elements in the group's own coordinates (C# `DrawingGroup`). As an element of a
/// layer it has a transform, an opacity - below 1 the group is drawn whole and then faded, so its
/// children never show through each other - and a time window. Children without a colour of their
/// own take the group's CONTENT colour, else the group's colour, else what the group inherits.
#[derive(Clone, Debug, PartialEq)]
pub struct DrawingGroup {
    pub(crate) color: Option<Color>,
    pub(crate) content_color: Option<Color>,
    pub(crate) children: Vec<DrawingShape>,
    pub(crate) mask: Option<DrawingMask>,
}

impl DrawingGroup {
    /// An empty group, with a colour for its children or None to inherit one (C#
    /// `DrawingGroup(Color? color = null)`).
    pub fn new(color: Option<Color>) -> DrawingGroup {
        DrawingGroup { color, content_color: None, children: Vec::new(), mask: None }
    }

    /// The group's colour (None: inherited).
    pub fn color(&self) -> Option<Color> {
        self.color
    }

    /// The colour its children without their own take, ahead of the group's colour (C#
    /// `ContentColor`; bindable as `AnimatedProperty::Color`).
    pub fn content_color(&self) -> Option<Color> {
        self.content_color
    }

    /// Sets the content colour.
    pub fn set_content_color(&mut self, color: Option<Color>) {
        self.content_color = color;
    }

    /// The group's mask (C# `Mask`; None by default): in the group's coordinates, so it moves,
    /// turns and scales with the group. To mask one element, put it in a group.
    pub fn mask(&self) -> Option<&DrawingMask> {
        self.mask.as_ref()
    }

    /// Sets (or removes) the mask.
    pub fn set_mask(&mut self, mask: Option<DrawingMask>) {
        self.mask = mask;
    }

    /// The number of children (C# `Count`).
    pub fn count(&self) -> usize {
        self.children.len()
    }

    /// Adds a child on top and returns its id (C# `Add`, which returns the element).
    pub fn add(&mut self, element: impl Into<DrawingShape>) -> ElementId {
        let element = element.into();
        let id = element.id();
        self.children.push(element);
        id
    }

    /// Removes the child with this id; false when there is none (C# `Remove`).
    pub fn remove(&mut self, id: ElementId) -> bool {
        match self.children.iter().position(|c| c.id() == id) {
            Some(i) => {
                self.children.remove(i);
                true
            }
            None => false,
        }
    }

    /// Removes every child.
    pub fn clear(&mut self) {
        self.children.clear();
    }

    /// The children, bottom first (C# `GetChildren`).
    pub fn get_children(&self) -> &[DrawingShape] {
        &self.children
    }

    /// The children, to change them.
    pub fn get_children_mut(&mut self) -> &mut [DrawingShape] {
        &mut self.children
    }

    /// The union of the children's transformed bounds, in the group's coordinates (C#
    /// `GetBounds`); an empty group is an empty rectangle; None when a child has no bounds.
    pub fn get_bounds(&self) -> Option<RectangleF> {
        self.bounds_core(None)
    }

    /// The same, counting only the children shown at a time (C# `GetBoundsAt`).
    pub fn get_bounds_at(&self, seconds: f64) -> Option<RectangleF> {
        self.bounds_core(Some(seconds))
    }

    fn bounds_core(&self, seconds: Option<f64>) -> Option<RectangleF> {
        let mut union: Option<RectangleF> = None;
        for child in &self.children {
            if seconds.is_some_and(|s| !child.is_visible_at(s)) {
                continue;
            }
            let bounds = child.get_transformed_bounds_at(seconds)?;
            union = Some(match union {
                None => bounds,
                Some(u) => {
                    let (l, t) = (u.x.min(bounds.x), u.y.min(bounds.y));
                    let (r, b) = (u.right().max(bounds.right()), u.bottom().max(bounds.bottom()));
                    RectangleF::new(l, t, r - l, b - t)
                }
            });
        }
        Some(union.unwrap_or_default())
    }
}
