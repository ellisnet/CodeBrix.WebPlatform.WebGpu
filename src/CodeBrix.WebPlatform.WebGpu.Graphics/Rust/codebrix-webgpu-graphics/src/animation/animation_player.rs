//! [`AnimationPlayer`] and [`PlayOptions`]: playing a model's animation clips on one
//! [`ModelInstance`] (C#: `AnimationPlayer`). CPU only - the pose it writes is what the model
//! renderer draws - and tested on the host.

use glam::{Quat, Vec3};

use crate::animation::animation_sampling::{nlerp, sample};
use crate::animation::{AnimationLoopMode, AnimationState};
use crate::context::GraphicsException;
use crate::models::{AnimationPath, ModelAnimation, ModelInstance, ModelNode, NodeTransform};

/// The defaulted arguments of C#'s `Play`, `CrossFade` and `Blend` (`loopMode`, `speed`,
/// `startTime`, `layer`), with C#'s defaults.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PlayOptions {
    /// Loop (default), ping-pong or once (hold the last frame).
    pub loop_mode: AnimationLoopMode,
    /// The clip's playback rate; default 1.
    pub speed: f32,
    /// The starting playhead in clip seconds; default 0.
    pub start_time: f32,
    /// The blend layer; default 0.
    pub layer: i32,
}

impl Default for PlayOptions {
    fn default() -> PlayOptions {
        PlayOptions { loop_mode: AnimationLoopMode::Loop, speed: 1.0, start_time: 0.0, layer: 0 }
    }
}

impl PlayOptions {
    /// The defaults with another loop mode (`PlayOptions::looping(AnimationLoopMode::Once)`).
    pub fn looping(loop_mode: AnimationLoopMode) -> PlayOptions {
        PlayOptions { loop_mode, ..PlayOptions::default() }
    }
}

/// What one instance's player keeps between calls (C#: the player's fields). It lives in the
/// [`ModelInstance`], created on first use.
#[derive(Debug)]
pub(crate) struct PlayerData {
    states: Vec<AnimationState>,
    speed: f32,
    paused: bool,
    last_time: Option<f64>,
    finished: Vec<usize>,
    sample: Vec<f32>,
    scratch: Option<BlendScratch>,
}

impl PlayerData {
    /// The playback state of a model with these clips.
    pub(crate) fn new(animations: &[ModelAnimation]) -> PlayerData {
        let components = animations.iter().flat_map(|a| a.channels.iter()).map(|c| c.component_count).max().unwrap_or(0);
        PlayerData { states: Vec::with_capacity(AnimationPlayer::MAX_STATES), speed: 1.0, paused: false, last_time: None, finished: Vec::new(), sample: vec![0.0; components.max(4)], scratch: None }
    }
}

/// The model data a player reads.
pub(crate) struct PoseSource<'a> {
    pub(crate) nodes: &'a [ModelNode],
    pub(crate) animations: &'a [ModelAnimation],
    pub(crate) morph_offsets: &'a [Option<usize>],
    pub(crate) morph_count: usize,
}

/// The instance's pose a player writes.
pub(crate) struct PoseTarget<'a> {
    pub(crate) locals: &'a mut [NodeTransform],
    pub(crate) weights: &'a mut [f32],
    pub(crate) posed: &'a mut bool,
    pub(crate) version: &'a mut u64,
}

/// Plays a model's animation clips on one [`ModelInstance`] (C#: `AnimationPlayer`, reached as
/// `instance.Animation`): [`ModelInstance::animation`] hands it out. The clips are shared,
/// immutable model data; the player holds only this instance's playback state - up to
/// [`AnimationPlayer::MAX_STATES`] clips at once, each with its own playhead, speed, loop mode,
/// weight and layer. Every update advances the playheads and writes the blended pose into the
/// instance's local transforms and morph weights; node matrices, joint palettes (skinning) and the
/// renderer's morph weights follow from there.
///
/// PLAYING: [`AnimationPlayer::play`] replaces what plays on a layer;
/// [`AnimationPlayer::cross_fade`] fades the layer's current clips out while the new one fades in;
/// [`AnimationPlayer::blend`] adds a clip with a weight beside the others. Each takes the clip's
/// index (C#'s `Play(int ...)`) and has a `_named` twin taking its name (C#'s `Play(string ...)`).
///
/// BLENDING (as C#): per animated property (a node's translation, rotation, scale or morph
/// weights) the states of one layer are averaged by weight (rotations: a weighted quaternion sum
/// with hemisphere correction, normalised; weights adding up past 1 are normalised). A layer then
/// replaces what lies below it (the rest pose, then lower layers) by its summed weight up to 1.
/// Properties no playing clip animates keep what they hold; when a clip stops, the properties it
/// animated return to the rest pose.
///
/// TIME COMES FROM THE FRAME - the player never reads a clock. Either hand it the frame's delta
/// ([`AnimationPlayer::update`], C#'s `Update(deltaSeconds)`: `frame.info.delta`, which is 0 while
/// the host's clock is paused and scaled by its speed), or the frame's TIME
/// ([`AnimationPlayer::update_to`]: the player advances by the difference to the time of its
/// previous `update_to`, so a seek of the host's clock moves the clips to the matching place -
/// looping and ping-pong clips land exactly where playing forwards would have put them; a fade or
/// a once-clip that already ended stays ended). The player's own speed and pause scale or stop
/// only this instance, on top of the clock's.
///
/// THE DIFFERENCE FROM C#: C#'s `ClipFinished` callback becomes a polled list,
/// [`AnimationPlayer::finished_clips`], read after an update.
pub struct AnimationPlayer<'a> {
    data: &'a mut PlayerData,
    source: PoseSource<'a>,
    target: PoseTarget<'a>,
}

impl<'a> AnimationPlayer<'a> {
    /// The most clips one player plays at once, cross-fades included (C#: `MaxStates`).
    pub const MAX_STATES: usize = 8;

    pub(crate) fn new(data: &'a mut PlayerData, source: PoseSource<'a>, target: PoseTarget<'a>) -> AnimationPlayer<'a> {
        AnimationPlayer { data, source, target }
    }

    /// Scales every clip's time and every fade; 1 normal, 0 freezes like a pause (C#: `Speed`).
    pub fn speed(&self) -> f32 {
        self.data.speed
    }

    /// Sets the speed (C#: the `Speed` setter).
    pub fn set_speed(&mut self, speed: f32) {
        self.data.speed = speed;
    }

    /// True while every playhead and fade is frozen; an update still applies the current pose
    /// (C#: `Paused`).
    pub fn paused(&self) -> bool {
        self.data.paused
    }

    /// Freezes or releases every playhead and fade of this player (C#: the `Paused` setter).
    pub fn set_paused(&mut self, paused: bool) {
        self.data.paused = paused;
    }

    /// True while at least one clip plays (C#: `IsPlaying`).
    pub fn is_playing(&self) -> bool {
        self.data.states.iter().any(|s| !s.stop_requested())
    }

    /// The number of playing clips (C#: `StateCount`).
    pub fn state_count(&self) -> usize {
        self.data.states.iter().filter(|s| !s.stop_requested()).count()
    }

    /// A playing state, 0..state_count() in start order; None for no such state (C#: `GetState`,
    /// which throws).
    pub fn get_state(&self, index: usize) -> Option<&AnimationState> {
        self.data.states.iter().filter(|s| !s.stop_requested()).nth(index)
    }

    /// A playing state, to change it (C#: `GetState`).
    pub fn get_state_mut(&mut self, index: usize) -> Option<&mut AnimationState> {
        self.data.states.iter_mut().filter(|s| !s.stop_requested()).nth(index)
    }

    /// The most recently started playing state of a clip, or None (C#: `FindState`).
    pub fn find_state(&self, clip: usize) -> Option<&AnimationState> {
        self.data.states.iter().rev().find(|s| s.clip_index() == clip && !s.stop_requested())
    }

    /// The most recently started playing state of a clip, to change it (C#: `FindState`).
    pub fn find_state_mut(&mut self, clip: usize) -> Option<&mut AnimationState> {
        self.data.states.iter_mut().rev().find(|s| s.clip_index() == clip && !s.stop_requested())
    }

    /// The clip indices of the once-clips the last update finished (C#: the `ClipFinished`
    /// callback, raised once per such clip). Start the next clip from here.
    pub fn finished_clips(&self) -> &[usize] {
        &self.data.finished
    }

    /// Plays clip `clip` (an index into the model's animations) at weight 1, replacing every
    /// clip on its layer (C#: `Play(int, loopMode, speed, startTime, layer)`).
    pub fn play(&mut self, clip: usize, options: PlayOptions) -> Result<&mut AnimationState, GraphicsException> {
        self.sweep();
        self.check_clip(clip)?;
        self.stop_layer(options.layer);
        let index = self.start(clip, options, 1.0)?;
        Ok(&mut self.data.states[index])
    }

    /// Plays a clip by name (C#: `Play(string, ...)`).
    pub fn play_named(&mut self, clip: &str, options: PlayOptions) -> Result<&mut AnimationState, GraphicsException> {
        let index = self.clip_named(clip)?;
        self.play(index, options)
    }

    /// Fades clip `clip` in over `duration` seconds of player time while every other clip on its
    /// layer fades out (and stops at weight 0); a duration of 0 or less is
    /// [`AnimationPlayer::play`] (C#: `CrossFade(int, duration, ...)`). The new state starts at
    /// weight 0. Err when the clip does not exist, or when MAX_STATES clips play and none of them
    /// is fading out.
    pub fn cross_fade(&mut self, clip: usize, duration: f32, options: PlayOptions) -> Result<&mut AnimationState, GraphicsException> {
        self.sweep();
        self.check_clip(clip)?;
        if duration <= 0.0 {
            return self.play(clip, options);
        }
        for state in self.data.states.iter_mut().filter(|s| s.layer() == options.layer) {
            state.fade_to(0.0, duration, true);
        }
        let index = self.start(clip, options, 0.0)?;
        let state = &mut self.data.states[index];
        state.fade_to(1.0, duration, false);
        Ok(state)
    }

    /// Cross-fades to a clip by name (C#: `CrossFade(string, ...)`).
    pub fn cross_fade_named(&mut self, clip: &str, duration: f32, options: PlayOptions) -> Result<&mut AnimationState, GraphicsException> {
        let index = self.clip_named(clip)?;
        self.cross_fade(index, duration, options)
    }

    /// Adds clip `clip` with a weight beside the clips already playing - layered or weighted
    /// blending (C#: `Blend(int, weight, ...)`).
    pub fn blend(&mut self, clip: usize, weight: f32, options: PlayOptions) -> Result<&mut AnimationState, GraphicsException> {
        self.sweep();
        self.check_clip(clip)?;
        let index = self.start(clip, options, weight)?;
        Ok(&mut self.data.states[index])
    }

    /// Adds a clip by name with a weight (C#: `Blend(string, ...)`).
    pub fn blend_named(&mut self, clip: &str, weight: f32, options: PlayOptions) -> Result<&mut AnimationState, GraphicsException> {
        let index = self.clip_named(clip)?;
        self.blend(index, weight, options)
    }

    /// Stops every clip; the properties they animated return to the rest pose (C#: `Stop()`).
    pub fn stop(&mut self) {
        while !self.data.states.is_empty() {
            self.remove(self.data.states.len() - 1);
        }
    }

    /// Stops every clip on a layer (C#: `Stop(int layer)`).
    pub fn stop_layer(&mut self, layer: i32) {
        for index in (0..self.data.states.len()).rev() {
            if self.data.states[index].layer() == layer {
                self.remove(index);
            }
        }
    }

    /// Stops the playing state `index` (0..state_count()); nothing for no such state (C#:
    /// `Stop(AnimationState)`).
    pub fn stop_state(&mut self, index: usize) {
        self.sweep();
        if index < self.data.states.len() {
            self.remove(index);
        }
    }

    /// Advances every playhead and fade by `delta_seconds` times the player's speed (nothing
    /// moves while it is paused), writes the blended pose into the instance and lists the
    /// once-clips that finished ([`AnimationPlayer::finished_clips`]) (C#: `Update(deltaSeconds)`).
    /// Pass the frame's delta (`frame.info.delta as f32`).
    pub fn update(&mut self, delta_seconds: f32) {
        self.sweep();
        self.data.finished.clear();
        if self.data.states.is_empty() {
            return;
        }
        let seconds = if self.data.paused { 0.0 } else { delta_seconds * self.data.speed };
        for state in &mut self.data.states {
            state.clear_flags();
            let scaled = seconds * state.speed;
            state.advance(scaled);
        }
        for index in (0..self.data.states.len()).rev() {
            if self.data.states[index].advance_fade(seconds) {
                self.remove(index);
            }
        }
        self.evaluate();
        let finished: Vec<usize> = self.data.states.iter().filter(|s| s.finished_this_update()).map(AnimationState::clip_index).collect();
        self.data.finished = finished;
    }

    /// Advances to the frame's TIME: by `time` minus the time of the previous `update_to` (the
    /// first call only records the time and applies the pose). Negative after a seek back: the
    /// clips play backwards to that place (a fade or a finished once-clip does not run back).
    /// Pass `frame.info.time`. Not in C#, whose viewer computes the delta itself.
    pub fn update_to(&mut self, time: f64) {
        let delta = match self.data.last_time {
            Some(last) => (time - last) as f32,
            None => 0.0,
        };
        self.data.last_time = Some(time);
        self.update(delta);
    }

    /// Writes the pose at the current playheads into the instance without advancing time - after
    /// a seek, for example (C#: `Evaluate`).
    pub fn evaluate(&mut self) {
        self.sweep();
        if self.data.states.is_empty() {
            return;
        }
        if self.data.states.len() == 1 && self.data.states[0].weight() >= 1.0 {
            self.apply_direct();
        } else {
            self.apply_blended();
        }
        self.mark_changed();
    }

    /// Updates the players of many instances by the frame's delta; instances whose player was
    /// never used are skipped (C#: `UpdateAll`).
    pub fn update_all(instances: &mut [ModelInstance], delta_seconds: f32) {
        for instance in instances.iter_mut().filter(|i| i.has_animation_player()) {
            instance.animation().update(delta_seconds);
        }
    }

    /// Updates the players of many instances to the frame's time ([`AnimationPlayer::update_to`]);
    /// instances whose player was never used are skipped.
    pub fn update_all_to(instances: &mut [ModelInstance], time: f64) {
        for instance in instances.iter_mut().filter(|i| i.has_animation_player()) {
            instance.animation().update_to(time);
        }
    }

    fn check_clip(&self, clip: usize) -> Result<(), GraphicsException> {
        if clip >= self.source.animations.len() {
            return Err(GraphicsException::new(format!("The model has {} animation(s), so there is no clip {clip}. Use an index below that, or find one by name with Model::find_animation.", self.source.animations.len())));
        }
        Ok(())
    }

    fn clip_named(&self, name: &str) -> Result<usize, GraphicsException> {
        self.source.animations.iter().position(|a| a.name.as_deref() == Some(name)).ok_or_else(|| {
            let names: Vec<&str> = self.source.animations.iter().filter_map(|a| a.name.as_deref()).collect();
            GraphicsException::new(format!("The model has no animation named '{name}'. Its clips are: {}.", if names.is_empty() { "none".to_string() } else { names.join(", ") }))
        })
    }

    /// Starts a state, making room by stopping the faintest state that is fading out when
    /// MAX_STATES play.
    fn start(&mut self, clip: usize, options: PlayOptions, weight: f32) -> Result<usize, GraphicsException> {
        if self.data.states.len() >= Self::MAX_STATES {
            let mut victim: Option<usize> = None;
            for (index, state) in self.data.states.iter().enumerate() {
                if state.stops_when_faded() && victim.is_none_or(|v| state.weight() < self.data.states[v].weight()) {
                    victim = Some(index);
                }
            }
            let Some(victim) = victim else {
                return Err(GraphicsException::new(format!("The player already plays {} clips; stop one first (stop_state, stop_layer or stop).", Self::MAX_STATES)));
            };
            self.remove(victim);
        }
        let state = AnimationState::start(clip, &self.source.animations[clip], options.loop_mode, options.speed, options.start_time, weight, options.layer);
        self.data.states.push(state);
        Ok(self.data.states.len() - 1)
    }

    /// Removes the states that were stopped through themselves.
    fn sweep(&mut self) {
        for index in (0..self.data.states.len()).rev() {
            if self.data.states[index].stop_requested() {
                self.remove(index);
            }
        }
    }

    /// Removes a state and returns the properties its clip animated to the rest pose.
    fn remove(&mut self, index: usize) {
        let state = self.data.states.remove(index);
        let nodes = self.source.nodes;
        for channel in &self.source.animations[state.clip_index()].channels {
            let Some(node) = nodes.get(channel.node) else {
                continue;
            };
            let rest = node.default_transform;
            let local = &mut self.target.locals[channel.node];
            match channel.path {
                AnimationPath::Translation => local.translation = rest.translation,
                AnimationPath::Rotation => local.rotation = rest.rotation,
                AnimationPath::Scale => local.scale = rest.scale,
                AnimationPath::Weights => {
                    if let Some(range) = morph_range(&self.source, channel.node) {
                        self.target.weights[range].copy_from_slice(&node.default_morph_weights);
                    }
                }
            }
        }
        self.mark_changed();
    }

    fn mark_changed(&mut self) {
        *self.target.posed = false;
        *self.target.version += 1;
    }

    /// One state at full weight: sample straight into the pose.
    fn apply_direct(&mut self) {
        let PlayerData { states, sample: value, .. } = &mut *self.data;
        let state = &mut states[0];
        let time = state.time();
        let clip = &self.source.animations[state.clip_index()];
        for (channel_index, channel) in clip.channels.iter().enumerate() {
            if channel.node >= self.target.locals.len() {
                continue;
            }
            sample(channel, time, &mut state.cursors[channel_index], value);
            let local = &mut self.target.locals[channel.node];
            match channel.path {
                AnimationPath::Translation => local.translation = Vec3::new(value[0], value[1], value[2]),
                AnimationPath::Rotation => local.rotation = Quat::from_xyzw(value[0], value[1], value[2], value[3]),
                AnimationPath::Scale => local.scale = Vec3::new(value[0], value[1], value[2]),
                AnimationPath::Weights => {
                    if let Some(range) = morph_range(&self.source, channel.node) {
                        let count = range.len().min(channel.component_count);
                        self.target.weights[range.start..range.start + count].copy_from_slice(&value[..count]);
                    }
                }
            }
        }
    }

    /// Several states or partial weights: blend per layer (see the type's documentation).
    fn apply_blended(&mut self) {
        let source = &self.source;
        let PlayerData { states, sample: value, scratch, .. } = &mut *self.data;
        let scratch = scratch.get_or_insert_with(|| BlendScratch::new(source.nodes.len(), source.morph_count));
        scratch.next_evaluation();
        let mut layer = i32::MIN;
        loop {
            let next = states.iter().map(AnimationState::layer).filter(|&l| l > layer).min();
            let Some(next) = next else {
                break;
            };
            layer = next;
            scratch.next_layer();
            for state in states.iter_mut().filter(|s| s.layer() == layer && s.weight() > 0.0) {
                accumulate(scratch, state, source, value);
            }
            resolve_layer(scratch, source, &mut self.target);
        }
    }
}

/// A node's slots in the instance's morph weights, None when its mesh has no morph targets.
fn morph_range(source: &PoseSource<'_>, node: usize) -> Option<std::ops::Range<usize>> {
    let offset = (*source.morph_offsets.get(node)?)?;
    let count = source.nodes[node].default_morph_weights.len();
    (offset + count <= source.morph_count).then_some(offset..offset + count)
}

/// The property slot of a channel's path (four per node).
fn path_index(path: AnimationPath) -> usize {
    match path {
        AnimationPath::Translation => 0,
        AnimationPath::Rotation => 1,
        AnimationPath::Scale => 2,
        AnimationPath::Weights => 3,
    }
}

const PATH_COUNT: usize = 4;

fn accumulate(scratch: &mut BlendScratch, state: &mut AnimationState, source: &PoseSource<'_>, value: &mut [f32]) {
    let time = state.time();
    let weight = state.weight();
    let clip = &source.animations[state.clip_index()];
    for (channel_index, channel) in clip.channels.iter().enumerate() {
        let node = channel.node;
        if node >= source.nodes.len() {
            continue;
        }
        let property = node * PATH_COUNT + path_index(channel.path);
        let morph = if channel.path == AnimationPath::Weights {
            match morph_range(source, node) {
                Some(range) => Some(range),
                None => continue,
            }
        } else {
            None
        };
        sample(channel, time, &mut state.cursors[channel_index], value);
        let first = scratch.touch(property);
        match channel.path {
            AnimationPath::Translation | AnimationPath::Scale => {
                let sum = if first { Vec3::ZERO } else { scratch.vectors[property] };
                scratch.vectors[property] = sum + Vec3::new(value[0], value[1], value[2]) * weight;
            }
            AnimationPath::Rotation => {
                let mut q = Quat::from_xyzw(value[0], value[1], value[2], value[3]);
                if first {
                    scratch.rotations[node] = q * weight;
                } else {
                    let sum = scratch.rotations[node];
                    if sum.dot(q) < 0.0 {
                        q = -q;
                    }
                    scratch.rotations[node] = sum + q * weight;
                }
            }
            AnimationPath::Weights => {
                let range = morph.unwrap_or(0..0);
                let count = range.len().min(channel.component_count);
                let sums = &mut scratch.morph[range.start..range.start + count];
                if first {
                    sums.fill(0.0);
                }
                for (target, sum) in sums.iter_mut().enumerate() {
                    *sum += value[target] * weight;
                }
            }
        }
        scratch.weights[property] = if first { 0.0 } else { scratch.weights[property] } + weight;
    }
}

fn resolve_layer(scratch: &mut BlendScratch, source: &PoseSource<'_>, target: &mut PoseTarget<'_>) {
    for touched in 0..scratch.touched.len() {
        let property = scratch.touched[touched];
        let node = property / PATH_COUNT;
        let path = property % PATH_COUNT;
        let total = scratch.weights[property];
        if total <= 0.0 {
            continue;
        }
        let layer_weight = total.min(1.0);
        let reset = scratch.first_this_evaluation(property);
        let data = &source.nodes[node];
        let local = &mut target.locals[node];
        match path {
            0 | 2 => {
                let value = scratch.vectors[property] / total;
                let (current, rest) = if path == 0 { (&mut local.translation, data.default_transform.translation) } else { (&mut local.scale, data.default_transform.scale) };
                if reset {
                    *current = rest;
                }
                *current = if layer_weight >= 1.0 { value } else { current.lerp(value, layer_weight) };
            }
            1 => {
                let sum = scratch.rotations[node];
                let length = sum.length();
                let value = if length > 1e-12 { sum * (1.0 / length) } else { Quat::IDENTITY };
                if reset {
                    local.rotation = data.default_transform.rotation;
                }
                local.rotation = if layer_weight >= 1.0 { value } else { nlerp(local.rotation, value, layer_weight) };
            }
            _ => {
                let Some(range) = morph_range(source, node) else {
                    continue;
                };
                let weights = &mut target.weights[range.clone()];
                if reset {
                    weights.copy_from_slice(&data.default_morph_weights);
                }
                let sums = &scratch.morph[range];
                let scale = 1.0 / total;
                for (weight, sum) in weights.iter_mut().zip(sums) {
                    let value = sum * scale;
                    *weight = if layer_weight >= 1.0 { value } else { *weight + (value - *weight) * layer_weight };
                }
            }
        }
    }
}

/// Per-player accumulators for blending, made on the first blended evaluation.
#[derive(Debug)]
struct BlendScratch {
    layer_stamps: Vec<u32>,
    evaluation_stamps: Vec<u32>,
    layer_pass: u32,
    evaluation_pass: u32,
    /// The summed weight per property.
    weights: Vec<f32>,
    /// The weighted translation or scale sums, by property.
    vectors: Vec<Vec3>,
    /// The weighted rotation sums, by node.
    rotations: Vec<Quat>,
    /// The weighted morph-weight sums, laid out like the instance's weights.
    morph: Vec<f32>,
    /// The properties touched in the current layer.
    touched: Vec<usize>,
}

impl BlendScratch {
    fn new(nodes: usize, morph_count: usize) -> BlendScratch {
        let properties = nodes * PATH_COUNT;
        BlendScratch {
            layer_stamps: vec![0; properties],
            evaluation_stamps: vec![0; properties],
            layer_pass: 0,
            evaluation_pass: 0,
            weights: vec![0.0; properties],
            vectors: vec![Vec3::ZERO; properties],
            rotations: vec![Quat::IDENTITY; nodes],
            morph: vec![0.0; morph_count],
            touched: Vec::with_capacity(properties),
        }
    }

    fn next_evaluation(&mut self) {
        self.evaluation_pass = self.evaluation_pass.wrapping_add(1);
        if self.evaluation_pass == 0 {
            self.evaluation_stamps.fill(0);
            self.evaluation_pass = 1;
        }
    }

    fn next_layer(&mut self) {
        self.touched.clear();
        self.layer_pass = self.layer_pass.wrapping_add(1);
        if self.layer_pass == 0 {
            self.layer_stamps.fill(0);
            self.layer_pass = 1;
        }
    }

    /// Marks a property as touched in this layer; true the first time (its sums start from zero).
    fn touch(&mut self, property: usize) -> bool {
        if self.layer_stamps[property] == self.layer_pass {
            return false;
        }
        self.layer_stamps[property] = self.layer_pass;
        self.touched.push(property);
        true
    }

    /// True the first time a property is resolved in this evaluation (it starts from the rest
    /// pose).
    fn first_this_evaluation(&mut self, property: usize) -> bool {
        if self.evaluation_stamps[property] == self.evaluation_pass {
            return false;
        }
        self.evaluation_stamps[property] = self.evaluation_pass;
        true
    }
}

#[cfg(test)]
#[path = "animation_player_tests.rs"]
mod tests;
