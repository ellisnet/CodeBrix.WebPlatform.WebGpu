//! The text bindings of `DrawingAnimationsMut` (C# `DrawingAnimations`): STAGGERS
//! (`BindStagger`) and TEXT ALONG A PATH (`BindTextAlongPath`).

use crate::models::{AnimatedProperty, AnimationBinding, BindingTarget, BoundTrack, DrawingAnimationsMut, ElementId, PathTrain, PathTrainOptions, StaggerOptions, StaggerTracks};
use crate::paths::PathGeometry;
use crate::shapes::TextBlockShape;
use crate::text::TextUnit;

impl DrawingAnimationsMut<'_> {
    /// STAGGERS a set of elements (C# `BindStagger(elements, options = null)`): each starts its
    /// entrance - a fade from the options' opacity, a slide by their offset, a scale and a turn
    /// about its own centre - a little after the one before, in the options' order. Binds, per
    /// element, Opacity, then Position, Scale and Rotation when the options move them (C#'s
    /// order), and returns the bindings. Text pieces from
    /// [`TextBlockShape::create_pieces`] stagger letter by letter, word by word or line by line.
    /// The options are checked BEFORE anything is bound (nothing is bound when they are bad);
    /// every element must be on a layer of the session. The error says what to do.
    pub fn bind_stagger(&mut self, elements: &[ElementId], options: Option<&StaggerOptions>) -> Result<Vec<AnimationBinding>, String> {
        let defaults = StaggerOptions::default();
        let options = options.unwrap_or(&defaults);
        for id in elements {
            self.check_target(&BindingTarget::Element(*id))?;
        }
        let bounds: Vec<_> = elements.iter().map(|id| self.session.find_element(*id).and_then(|e| e.get_bounds())).collect();
        let tracks = StaggerTracks::create(&bounds, options)?;
        let mut bindings = Vec::new();
        for (id, t) in elements.iter().zip(tracks) {
            if let Some(anchor) = t.anchor {
                if let Some(e) = self.session.find_element_mut(*id) {
                    let transform = e.transform().with_anchor(anchor);
                    e.set_transform(transform);
                }
            }
            bindings.push(self.bind(*id, AnimatedProperty::Opacity, t.opacity)?);
            if let Some(p) = t.position {
                bindings.push(self.bind(*id, AnimatedProperty::Position, p)?);
            }
            if let Some(s) = t.scale {
                bindings.push(self.bind(*id, AnimatedProperty::Scale, s)?);
            }
            if let Some(r) = t.rotation {
                bindings.push(self.bind(*id, AnimatedProperty::Rotation, r)?);
            }
        }
        Ok(bindings)
    }

    /// Sends a text block along a path piece by piece - characters, words or lines - as a TRAIN
    /// whose cars SETTLE IN PLACE at the pieces' own places in the block (C#
    /// `BindTextAlongPath(text, unit, path, progress, options = null)`): each piece travels a copy
    /// of the path that ends where it sits; with `roll_on_baseline` each rolls on its line's
    /// baseline. Pieces that are only white space are left out. The pieces are ADDED to the layer
    /// named `layer` here (C#'s `PathTrain.AddTo(layer)`); the block itself is not drawn. The
    /// error says what to do.
    pub fn bind_text_along_path(&mut self, text: &TextBlockShape, unit: TextUnit, layer: &str, path: &PathGeometry, progress: impl Into<BoundTrack>, options: PathTrainOptions) -> Result<PathTrain, String> {
        let progress = progress.into();
        if progress.kind_name() != "float" {
            return Err("The progress along a path is a float track (fractions of the path's length).".to_string());
        }
        let pieces = text.create_path_pieces(unit);
        let Some(target) = self.session.get_layer_mut(layer) else {
            return Err(format!("There is no layer named \"{layer}\": add it to the session first (add_layer)."));
        };
        let mut ids = Vec::with_capacity(pieces.len());
        let mut baselines = Vec::with_capacity(pieces.len());
        for piece in pieces {
            baselines.push(piece.baseline);
            ids.push(target.add_element(piece.shape));
        }
        self.bind_train(&ids, Some(&baselines), path, progress, options)
    }
}
