//! The Viewer's animated actors - the chest opening and closing, four animals walking a ring,
//! four standing animals cross-fading through their routine, the Avian circling overhead - played
//! and placed exactly as the desktop model viewer's scene does it (its ViewerScene.Update), with
//! the graphics crate's AnimationPlayer on each instance, from the backend's description of each
//! actor (the scene JSON).
//!
//! THE SCENE IS A FUNCTION OF THE FRAME'S TIME: every frame takes the host clock's time and delta.
//! A frame of ordinary playback advances the players by the frame's delta, as the desktop viewer
//! does (so pausing the clock stops everything and resuming goes on from the same time, and the
//! clock's speed scales everything). When the clock's time JUMPS (a seek: the time moved by
//! something other than the frame's delta), the actors are re-posed for the new time from the
//! schedule itself - each looping clip at its offset plus the time, each routine at the clip
//! (and, inside a cross-fade, the two clips and their weights) the desktop viewer would be
//! showing at that time - so a seek forwards or backwards lands on the same pose that playing up
//! to that time gives.

use codebrix_webgpu::glam::{Mat4, Quat, Vec3};
use codebrix_webgpu_graphics::{ModelInstance, PlayOptions};

use crate::generated::models::SceneActor;

/// One actor: its description and the routine step it shows.
struct Actor {
    data: SceneActor,
    /// The actor's instance in the layer's instance list.
    instance: usize,
    /// The routine step shown (None before the first frame).
    routine_step: Option<i64>,
}

/// The actors and the time of the last frame.
pub struct ViewerActors {
    actors: Vec<Actor>,
    routine_seconds: f32,
    cross_fade_seconds: f32,
    last_time: Option<f64>,
    /// The number of re-posings after a jump of the clock (shown on the page).
    pub seeks: u64,
}

impl ViewerActors {
    /// The actors `actors` describes; `first_instance` is the index of the first actor's
    /// instance in the layer's list (the actors' instances follow one another in their order).
    pub fn new(actors: Vec<SceneActor>, first_instance: usize, routine_seconds: f32, cross_fade_seconds: f32) -> ViewerActors {
        let actors = actors.into_iter().enumerate().map(|(index, data)| Actor { data, instance: first_instance + index, routine_step: None }).collect();
        ViewerActors { actors, routine_seconds, cross_fade_seconds, last_time: None, seeks: 0 }
    }

    /// The number of actors.
    pub fn len(&self) -> usize {
        self.actors.len()
    }

    /// The desktop viewer's world matrix of `actor` at scene time `time`.
    pub fn world_at(actor: &SceneActor, time: f32) -> Mat4 {
        let scale = Vec3::splat(actor.scale);
        match actor.motion.as_str() {
            "walk" => {
                let angle = actor.base_angle + time * actor.angular_speed;
                let position = Vec3::new(angle.sin(), 0.0, angle.cos()) * actor.radius;
                Mat4::from_scale_rotation_translation(scale, Quat::from_rotation_y(angle + std::f32::consts::FRAC_PI_2), position)
            }
            "fly" => {
                let angle = time * actor.angular_speed;
                let position = Vec3::new(angle.sin() * actor.radius, actor.height + (time * actor.bob_rate).sin() * actor.bob_amplitude, angle.cos() * actor.radius);
                Mat4::from_scale_rotation_translation(scale, Quat::from_rotation_y(angle - std::f32::consts::FRAC_PI_2), position)
            }
            _ => {
                let t = &actor.translation;
                Mat4::from_scale_rotation_translation(scale, Quat::from_rotation_y(actor.rotation_y), Vec3::new(t.x, t.y, t.z))
            }
        }
    }

    /// Moves the scene to the frame's time: re-poses it after a jump of the clock, else starts
    /// the routine clips due by now (cross-fading from the previous one), places the actors and
    /// advances every player by the frame's delta. Err with a message when a clip is missing.
    pub fn update(&mut self, time: f64, delta: f64, instances: &mut [ModelInstance]) -> Result<(), String> {
        let jumped = match self.last_time {
            None => true,
            Some(last) => (time - last - delta).abs() > 1e-6,
        };
        if jumped {
            if self.last_time.is_some() {
                self.seeks += 1;
            }
            self.pose_at(time, instances)?;
        } else {
            for actor in &mut self.actors {
                if actor.data.routine.is_empty() {
                    continue;
                }
                let step = routine_step(time, actor.data.time_offset, self.routine_seconds);
                if Some(step) != actor.routine_step {
                    let clip = routine_clip(&actor.data.routine, step);
                    let mut player = instances[actor.instance].animation();
                    let started = if actor.routine_step.is_none() { player.play_named(clip, PlayOptions::default()).map(|_| ()) } else { player.cross_fade_named(clip, self.cross_fade_seconds, PlayOptions::default()).map(|_| ()) };
                    started.map_err(|e| format!("{}: {e}", actor.data.model))?;
                    actor.routine_step = Some(step);
                }
            }
            for actor in &self.actors {
                instances[actor.instance].animation().update(delta as f32);
            }
        }
        for actor in &self.actors {
            instances[actor.instance].world = Self::world_at(&actor.data, time as f32);
        }
        self.last_time = Some(time);
        Ok(())
    }

    /// Re-poses every actor for scene time `time` from the schedule (what playing up to that
    /// time shows).
    fn pose_at(&mut self, time: f64, instances: &mut [ModelInstance]) -> Result<(), String> {
        let t = time as f32;
        for actor in &mut self.actors {
            let name = actor.data.model.clone();
            let mut player = instances[actor.instance].animation();
            player.stop();
            if actor.data.routine.is_empty() {
                if !actor.data.clip.is_empty() {
                    // A clip that loops from its own start time, so that the actors do not move in step.
                    player.play_named(&actor.data.clip, PlayOptions { start_time: actor.data.time_offset + t, ..PlayOptions::default() }).map_err(|e| format!("{name}: {e}"))?;
                }
            } else {
                let rs = self.routine_seconds;
                let offset = actor.data.time_offset;
                let step = routine_step(time, offset, rs);
                let first = routine_step(0.0, offset, rs);
                // The clip of a step starts at the step's time (the first one at time 0).
                let starts = |s: i64| ((s as f32) * rs - offset).max(0.0);
                let since = t - (step as f32 * rs - offset);
                let clip = routine_clip(&actor.data.routine, step);
                if step > first && since < self.cross_fade_seconds {
                    // Inside the cross-fade from the previous step's clip.
                    let previous = routine_clip(&actor.data.routine, step - 1);
                    let fraction = since / self.cross_fade_seconds;
                    let rest = self.cross_fade_seconds - since;
                    let old = player.play_named(previous, PlayOptions { start_time: t - starts(step - 1), ..PlayOptions::default() }).map_err(|e| format!("{name}: {e}"))?;
                    old.set_weight(1.0 - fraction);
                    old.fade_to(0.0, rest, true);
                    let new = player.blend_named(clip, fraction, PlayOptions { start_time: t - starts(step), ..PlayOptions::default() }).map_err(|e| format!("{name}: {e}"))?;
                    new.fade_to(1.0, rest, false);
                } else {
                    player.play_named(clip, PlayOptions { start_time: t - starts(step), ..PlayOptions::default() }).map_err(|e| format!("{name}: {e}"))?;
                }
                actor.routine_step = Some(step);
            }
            player.evaluate();
        }
        Ok(())
    }

    /// The number of clips playing on every actor together.
    pub fn clips_playing(&self, instances: &mut [ModelInstance]) -> usize {
        self.actors.iter().map(|a| instances[a.instance].animation().state_count()).sum()
    }

    /// A short fingerprint of every actor's pose: the sum of every node's local translation and
    /// rotation components (it changes whenever any animated node moves).
    pub fn pose_signature(&self, instances: &[ModelInstance]) -> f64 {
        self.actors
            .iter()
            .flat_map(|a| instances[a.instance].local_transforms().iter())
            .map(|l| (l.translation.x + l.translation.y + l.translation.z + l.rotation.x + l.rotation.y + l.rotation.z + l.rotation.w) as f64)
            .sum()
    }

    /// The world position of the actor whose motion is "fly" (the Avian), if there is one.
    pub fn bird_position(&self, instances: &[ModelInstance]) -> Option<Vec3> {
        let bird = self.actors.iter().find(|a| a.data.motion == "fly")?;
        Some(instances[bird.instance].world.w_axis.truncate())
    }

    /// The first standing animal's routine: its model, the step and the clips playing with their
    /// weights (to show a cross-fade on the page).
    pub fn routine_state(&self, instances: &mut [ModelInstance]) -> String {
        let Some(actor) = self.actors.iter().find(|a| !a.data.routine.is_empty()) else {
            return "none".to_string();
        };
        let player = instances[actor.instance].animation();
        let states: Vec<String> = (0..player.state_count()).filter_map(|i| player.get_state(i)).map(|s| format!("{} {:.2}", s.name().unwrap_or("?"), s.weight())).collect();
        format!("{}: {}", actor.data.model, states.join(" + "))
    }
}

/// The routine step at `time` for an actor `offset` seconds ahead (C#: floor((time + offset) /
/// RoutineSeconds)).
fn routine_step(time: f64, offset: f32, routine_seconds: f32) -> i64 {
    ((time + offset as f64) / routine_seconds as f64).floor() as i64
}

/// The routine's clip of a step (C#: routine[((step % n) + n) % n]).
fn routine_clip(routine: &[String], step: i64) -> &str {
    let n = routine.len() as i64;
    &routine[(((step % n) + n) % n) as usize]
}
