//! The motion and formation bindings of `DrawingAnimationsMut` (C# `DrawingAnimations`): COMBINED bindings
//! (`Bind(..., BindingMode)`), ROLLING and overshooting along a path (`BindAlongPath` with
//! `PathTravelOptions`, `BindRollAlongPath`), TRAINS (`BindTrainAlongPath`) and FORMATIONS
//! (`BindFormation`).

use std::cell::RefCell;
use std::rc::Rc;

use crate::formations::formation_elements_driver;
use crate::formations::{FormationElements, FormationMember};
use crate::geometry::PointF;
use crate::models::binding_combine::{get_slot, slots_for};
use crate::models::drawing_animations::BindingSource;
use crate::models::path_train::TrainState;
use crate::models::path_travel::{Roll, TravelDriver};
use crate::models::{AnimatedProperty, AnimationBinding, BindingMode, BindingTarget, BoundTrack, DrawingAnimationsMut, ElementId, PathTrain, PathTrainCar, PathTrainOptions, PathTravelOptions};
use crate::paths::PathGeometry;
use crate::shapes::DrawingShape;

impl DrawingAnimationsMut<'_> {
    /// Binds a float or point track - or a value function of the time - COMBINING with the
    /// property as `mode` says (C#'s `Bind(element | layer | camera, property, track, mode)` and
    /// `Bind(element, property, Func<double, T>, mode)`): Add adds the value to whatever the other
    /// bindings set (a sway on top of a turn, an offset on top of a path), Multiply multiplies by
    /// it (component by component for a point), Replace is [`bind`](DrawingAnimationsMut::bind).
    /// Combinable: on an element or a layer Rotation, Scale (float: both axes; point: X and Y),
    /// ScaleX, ScaleY, Opacity (clamped to 0..1 after combining) and Position (point: the
    /// translation); on the camera Rotation (roll), Scale (zoom; keep the result positive - a zoom
    /// it refuses leaves the camera as it was) and Position (pan). THE ORDER: every slot a
    /// combining binding drives is first put back to its resting value (its value when it was
    /// first bound that way), then every Replace binding and callback applies in order, then every
    /// Multiply, then every Add - so the result does not depend on the order the bindings were
    /// made in. The error says what can be combined instead.
    pub fn bind_with_mode(&mut self, target: impl Into<BindingTarget>, property: AnimatedProperty, track: impl Into<BoundTrack>, mode: BindingMode) -> Result<AnimationBinding, String> {
        let target = target.into();
        let track = track.into();
        if mode == BindingMode::Replace {
            return self.bind(target, property, track);
        }
        self.check_target(&target)?;
        let slots = slots_for(&target, property, &track)?;
        let current: Vec<f32> = slots.iter().map(|s| get_slot(self.session, &target, *s).unwrap_or(0.0)).collect();
        self.session.animations.register_rests(&target, &slots, &current);
        Ok(self.push_with_mode(Some(target), Some(property), BindingSource::Combine { track, slots }, mode))
    }

    /// The along-path binding with a roll or an overshoot (what `bind_along_path` makes when its
    /// options ask for one).
    pub(crate) fn bind_travel_along_path(&mut self, element: ElementId, path: &PathGeometry, progress: BoundTrack, options: PathTravelOptions) -> Result<AnimationBinding, String> {
        if !options.roll_radius.is_finite() || !options.upright_at_progress.is_finite() {
            return Err("The roll radius and the upright progress must be finite numbers.".to_string());
        }
        let roll = if options.roll_radius != 0.0 { Some(Roll::new(path, options.roll_radius, options.upright_at_progress, &progress)) } else { None };
        let driver = TravelDriver { element, path: path.clone(), progress, options, roll };
        Ok(self.push(Some(BindingTarget::Element(element)), Some(AnimatedProperty::Position), BindingSource::Driver(Rc::new(driver))))
    }

    /// Rolls an element along a path like a wheel (C# `BindRollAlongPath(element, path, progress,
    /// radius, uprightAtProgress = 1, orientToPath = false)`): its transform ANCHOR (set it to the
    /// wheel's hub first) rides the path at the progress's fraction of the length, and it turns by
    /// the distance travelled over `radius` (`(p - upright_at_progress) x length / radius`
    /// radians), upright at `upright_at_progress` (None: 1, the path's end), facing along the path
    /// as well when `orient_to_path` is set (the two add). Negative radii roll the other way. The short
    /// form of [`bind_along_path`](DrawingAnimationsMut::bind_along_path) with a roll radius; use
    /// that for an overshoot past the ends. The error says what to do with a radius of 0 or a
    /// value that is not finite.
    pub fn bind_roll_along_path(&mut self, element: ElementId, path: &PathGeometry, progress: impl Into<BoundTrack>, radius: f32, upright_at_progress: Option<f32>, orient_to_path: bool) -> Result<AnimationBinding, String> {
        if !radius.is_finite() || radius == 0.0 {
            return Err("The rolling radius must be a finite distance other than 0.".to_string());
        }
        let upright = upright_at_progress.unwrap_or(1.0);
        if !upright.is_finite() {
            return Err("The upright progress must be a finite fraction.".to_string());
        }
        self.bind_along_path(element, path, progress, PathTravelOptions { roll_radius: radius, upright_at_progress: upright, orient_to_path, ..PathTravelOptions::default() })
    }

    /// Sends several elements along ONE path as a train (C# `BindTrainAlongPath(elements, path,
    /// progress, options)`): one behind the other with gaps that may open and close, each
    /// following the progress a little later than the one ahead, each optionally facing along the
    /// path, rolling, scaling and fading as it goes - see [`PathTrainOptions`] for the rule. The
    /// elements must already be on the session's layers (C# takes element objects and adds them
    /// with `AddTo(layer)`; in Rust add them first, then bind them by id). With
    /// `anchor_at_center` each element's anchor is moved to the centre of its bounds first. One
    /// binding places them all. The error says what is wrong.
    pub fn bind_train_along_path(&mut self, elements: &[ElementId], path: &PathGeometry, progress: impl Into<BoundTrack>, options: PathTrainOptions) -> Result<PathTrain, String> {
        self.bind_train(elements, None, path, progress.into(), options)
    }

    /// The train (C#'s private `BindTrain`); `baselines` (one per element) are given by a text
    /// train (BindTextAlongPath), which settles in place and may roll each piece on its baseline.
    pub(crate) fn bind_train(&mut self, elements: &[ElementId], baselines: Option<&[f32]>, path: &PathGeometry, progress: BoundTrack, options: PathTrainOptions) -> Result<PathTrain, String> {
        if progress.kind_name() != "float" {
            return Err("The progress along a path is a float track (fractions of the path's length).".to_string());
        }
        let mut settings = options;
        if baselines.is_some() {
            settings.settle_in_place = true;
        }
        settings.validate()?;
        if let Some(b) = baselines {
            if b.len() != elements.len() {
                return Err("A text train needs one baseline per piece.".to_string());
            }
        }
        for id in elements {
            self.check_target(&BindingTarget::Element(*id))?;
        }
        let end = path.point_at_distance_value(path.length_value()).position;
        let count = elements.len();
        let mut cars = Vec::with_capacity(count);
        for (i, id) in elements.iter().enumerate() {
            let element = self.session.find_element_mut(*id).ok_or("The element is not on any of the session's layers: add it (draw_shape or a layer's add_element) before binding it. An element inside a group counts only once that group is on a layer: add the group first, then bind its elements.")?;
            if settings.anchor_at_center {
                if let Some(b) = element.get_bounds() {
                    element.set_transform(element.transform().with_anchor(PointF::new(b.x + (b.width / 2.0), b.y + (b.height / 2.0))));
                }
            }
            let rank = settings.order.rank(i, count);
            let mut radius = settings.roll_radius;
            let anchor = element.transform().anchor();
            if let (Some(b), true) = (baselines, settings.roll_on_baseline) {
                radius = (b[i] - anchor.y).max(1.0);
            }
            cars.push(PathTrainCar {
                element: *id,
                index: i,
                rank,
                gap: rank as f32 * settings.spacing,
                delay: rank as f64 * settings.stagger,
                roll_radius: radius,
                reference: if settings.settle_in_place { Some(end) } else { None },
            });
        }
        let state = Rc::new(RefCell::new(TrainState::new(path.clone(), progress, settings, cars)));
        let binding = self.push(None, Some(AnimatedProperty::Position), BindingSource::Driver(PathTrain::driver(&state)));
        Ok(PathTrain { state, binding })
    }

    /// Turns a formation into time-keyed elements of the session (C# `BindFormation(members,
    /// restingLayer, travellingLayer, createResting, createTravelling = null)`): for each member,
    /// in ARRIVAL order (a stable sort by arrival time), `create_resting(k)` makes its resting
    /// element - already placed where it rests - which goes on the layer named `resting_layer`,
    /// shown from its arrival on; and for a member that travels, `create_travelling(k)` makes the
    /// elements seen on the way - drawn CENTRED ON (0, 0) - which go on `travelling_layer`, shown
    /// only while it travels, and are posed by one binding (the path's pose as a transform). Every
    /// frame is a function of its time alone. The error says what is wrong.
    pub fn bind_formation(&mut self, members: &[FormationMember], resting_layer: &str, travelling_layer: &str, mut create_resting: impl FnMut(usize) -> DrawingShape, mut create_travelling: Option<&mut dyn FnMut(usize) -> Vec<DrawingShape>>) -> Result<FormationElements, String> {
        for name in [resting_layer, travelling_layer] {
            if self.session.get_layer(name).is_none() {
                return Err(format!("The session has no layer named '{name}': add it first."));
            }
        }
        let count = members.len();
        let mut order: Vec<usize> = (0..count).collect();
        // A stable sort by arrival time: members arriving together keep their given order.
        order.sort_by(|a, b| members[*a].arrival_time().partial_cmp(&members[*b].arrival_time()).unwrap_or(std::cmp::Ordering::Equal).then(a.cmp(b)));
        let mut resting: Vec<Option<ElementId>> = vec![None; count];
        let mut travelling: Vec<Vec<ElementId>> = vec![Vec::new(); count];
        for &i in &order {
            let member = &members[i];
            let mut rest = create_resting(i);
            let end = rest.end_time();
            rest.set_time_window(Some(member.arrival_time()), end);
            resting[i] = self.session.get_layer_mut(resting_layer).map(|l| l.add_element(rest));
            if let (Some(path), Some(make)) = (member.path(), create_travelling.as_mut()) {
                for mut part in make(i) {
                    part.set_time_window(Some(path.start_time()), Some(member.arrival_time()));
                    if let Some(layer) = self.session.get_layer_mut(travelling_layer) {
                        travelling[i].push(layer.add_element(part));
                    }
                }
            }
        }
        let resting: Vec<ElementId> = resting.into_iter().flatten().collect();
        let members = Rc::new(members.to_vec());
        let travelling = Rc::new(travelling);
        let driver = formation_elements_driver(members.clone(), travelling.clone());
        let binding = self.push(None, None, BindingSource::Driver(driver));
        Ok(FormationElements { members, resting, travelling, order, binding })
    }
}
