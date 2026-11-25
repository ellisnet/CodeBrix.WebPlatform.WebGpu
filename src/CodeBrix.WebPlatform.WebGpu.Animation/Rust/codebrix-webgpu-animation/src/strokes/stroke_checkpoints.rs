//! `StrokeCheckpoints`: the optional checkpoint cache of a stroke set that does not move on
//! screen - the canvas as it is after the set's first strokes, kept on the GPU, so a frame draws
//! only the strokes after the nearest saved state instead of the whole set (no C# equivalent).
//!
//! THE RULE IT KEEPS. Every frame is still a function of the time passed in, pixel for pixel: a
//! frame drawn from a checkpoint is IDENTICAL, on the same GPU, to the frame drawn with every
//! stroke at its progress. That holds because
//! - a checkpoint is the draw's own multisampled target (every sample, 16 bytes per canvas
//!   pixel), copied at the point of the stroke draw: restoring it and drawing the strokes after
//!   it replays exactly the blending the whole draw would do, in the same order;
//! - it holds only a PREFIX of the set (in paint order) whose every stroke is WHOLE at the frame's
//!   time (`end <= time`, compared exactly as the shader compares it): a whole stroke draws the
//!   same fragments at any later time, and a stroke still being painted (0 < progress < 1) is
//!   never baked - it is drawn on top every frame, as are the strokes after it;
//! - it is used only under the SAME KEY: the same set, tips and colour source, the same draw
//!   block apart from the time (the element's device matrix, opacity, head softness, the fixed
//!   anchors, the colour pickup and the layer's impasto), the same canvas size and target level,
//!   and EVERYTHING drawn before the set in the frame (every operation, vertex, paint, picture
//!   and ramp the renderer encoded before it) the same. Any difference is another key: the draw
//!   is drawn whole from the first stroke (and a new checkpoint made once the key holds still).
//!
//! WHEN IT IS NOT USED (the draw falls back to the full redraw, automatically): no checkpoints
//! asked for; boil or flow (the strokes move with the time); an anchor that FOLLOWS an element
//! (it moves); colours picked from a picture target or an application's texture (they change
//! every frame); a stroke-level transition (a morph). [`checkpoint_refusal`] is that decision.
//! A key that changes every frame (a moving painting, a moving camera) never makes a checkpoint:
//! one is made only when the draw's key was the same in the frame before.
//!
//! THE PLAN OF A DRAW ([`CheckpointCache::step`]): the frame's whole prefix `w`; the held
//! checkpoint of the key with the most strokes not above `w` is restored (strokes `0..n` cost one
//! texture copy); a new checkpoint is made at `floor(w / spacing) x spacing` strokes when that is
//! more than the restored one and the key held still - by copying the target after drawing up to
//! it (no extra drawing: those strokes are drawn this frame anyway). At most one per draw per
//! frame; at most `capacity` per key (the one farthest from the new one gives up its texture).
//!
//! MEMORY. One checkpoint is the multisampled colour target: [`StrokeCheckpoints::BYTES_PER_PIXEL`]
//! (16) bytes per canvas pixel - 14.7 MB at 1280 x 720, 59 MB at 2560 x 1440. The renderer keeps
//! the checkpoints by the texture ledger's policy (rendering/core/texture_ledger.rs
//! `CHECKPOINT_RESIDENCY`): one not used for 120 frames goes, and while the checkpoints not used
//! this frame weigh more than 128 MiB the least recently used go first.

use std::collections::HashMap;

/// The checkpoint cache of a stroke set drawn by a
/// [`BrushStrokesShape`](crate::shapes::BrushStrokesShape) that does not move on screen (no C#
/// equivalent; see [`BrushStrokesShape::with_checkpoints`](crate::shapes::BrushStrokesShape::with_checkpoints)).
///
/// A frame then draws the strokes after the nearest saved state instead of the whole set, and
/// stays IDENTICAL, pixel for pixel on the same GPU, to the frame drawn whole (the cache is used
/// only where that holds; elsewhere the set is drawn whole, automatically). Painting on at the
/// head costs the strokes painted since the last checkpoint; a seek back restores the nearest
/// earlier checkpoint and replays the strokes after it.
///
/// THE DEFAULTS: a checkpoint every 2,000 whole strokes, at most 4 per draw. The spacing bounds
/// the strokes drawn per frame above a checkpoint (up to `spacing` plus those being painted); a
/// set with fewer whole strokes than `spacing` makes none. MEMORY: 16 bytes per canvas pixel per
/// checkpoint ([`StrokeCheckpoints::bytes_at`]: 14.7 MB at 1280 x 720, 59 MB at 2560 x 1440).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StrokeCheckpoints {
    /// Whole strokes between two checkpoints (checkpoints hold 1, 2, 3 ... times this many
    /// strokes from the first): at least 1. 2,000.
    pub spacing: usize,
    /// Checkpoints kept per draw: at least 1. 4.
    pub capacity: usize,
}

impl Default for StrokeCheckpoints {
    fn default() -> StrokeCheckpoints {
        StrokeCheckpoints { spacing: 2000, capacity: 4 }
    }
}

impl StrokeCheckpoints {
    /// The bytes one checkpoint takes per canvas pixel: the renderer's multisampled colour
    /// target, 4 samples of 4 bytes.
    pub const BYTES_PER_PIXEL: usize = 16;

    /// The GPU bytes of one checkpoint on a canvas of `width` x `height` device pixels.
    pub fn bytes_at(width: u32, height: u32) -> usize {
        width as usize * height as usize * StrokeCheckpoints::BYTES_PER_PIXEL
    }

    /// The error says which value is unusable.
    pub fn validate(&self) -> Result<(), String> {
        if self.spacing == 0 {
            return Err("Stroke checkpoints need a spacing of at least one stroke (2000 is the default).".to_string());
        }
        if self.capacity == 0 {
            return Err("Stroke checkpoints need a capacity of at least one checkpoint (4 is the default).".to_string());
        }
        Ok(())
    }
}

/// What a stroke draw is, for the checkpoint decision (crate-internal; the facts of the shape).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct CheckpointFacts {
    /// Checkpoints asked for.
    pub(crate) asked: bool,
    /// The strokes boil.
    pub(crate) boil: bool,
    /// The strokes flow.
    pub(crate) flow: bool,
    /// An anchor follows an element.
    pub(crate) followed_anchor: bool,
    /// The colours are picked from a picture target or an application's texture.
    pub(crate) live_colour: bool,
    /// A stroke-level transition.
    pub(crate) morph: bool,
}

/// Why a draw that asked for checkpoints cannot use them, or None when it can (they are then used
/// whenever the frame's key holds; see the module's text). The message is what a session report
/// says.
pub(crate) fn checkpoint_refusal(f: &CheckpointFacts) -> Option<&'static str> {
    if !f.asked {
        return None;
    }
    if f.boil {
        Some("its strokes boil (they move with the time)")
    } else if f.flow {
        Some("its strokes flow (they turn with the time)")
    } else if f.followed_anchor {
        Some("an anchor follows an element (the strokes move with it)")
    } else if f.live_colour {
        Some("its colours are picked from a picture target or an application's texture (they change every frame)")
    } else if f.morph {
        Some("it is a stroke-level transition (every stroke moves with the fraction)")
    } else {
        None
    }
}

/// What one stroke draw does this frame with its checkpoints (crate-internal).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct CheckpointStep {
    /// The checkpoint restored before drawing: (slot, the strokes it holds).
    pub(crate) restore: Option<(usize, u32)>,
    /// The checkpoint made this frame, after drawing up to it: (slot, the strokes it holds).
    pub(crate) bake: Option<(usize, u32)>,
}

impl CheckpointStep {
    /// The first stroke the frame draws (the strokes before it come from the checkpoint).
    pub(crate) fn first(&self) -> u32 {
        self.restore.map_or(0, |r| r.1)
    }

    /// True when the draw leaves the render pass (a texture copy before or inside it).
    pub(crate) fn splits(&self) -> bool {
        self.restore.is_some() || self.bake.is_some()
    }

    /// The instance ranges the frame draws, in order: up to the bake (then the copy out), then the
    /// rest up to `instances`. Empty ranges are dropped.
    pub(crate) fn ranges(&self, instances: u32) -> Vec<std::ops::Range<u32>> {
        let first = self.first();
        let mut out = Vec::new();
        match self.bake {
            Some((_, n)) => {
                out.push(first..n);
                out.push(n..instances);
            }
            None => out.push(first..instances),
        }
        out.retain(|r| r.end > r.start);
        out
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Held {
    key: u64,
    strokes: u32,
    last_used: u64,
    bytes: usize,
}

/// Which checkpoints the GPU half holds (by slot: a slot is one texture) and what each frame's
/// draws do with them - the CPU half of the cache, tested on the host (crate-internal).
#[derive(Debug, Default)]
pub(crate) struct CheckpointCache {
    slots: Vec<Option<Held>>,
    /// The frame each key was last drawn in (a checkpoint is made only for a key drawn in the
    /// frame before: a key that changes every frame makes none).
    seen: HashMap<u64, u64>,
}

impl CheckpointCache {
    /// The plan of one draw: `key` (see the module's text), `whole` = the frame's whole prefix
    /// (strokes `0..whole` have all ended), `options`, the frame number, and the bytes a
    /// checkpoint takes at the canvas size.
    pub(crate) fn step(&mut self, key: u64, whole: u32, options: StrokeCheckpoints, frame: u64, bytes: usize) -> CheckpointStep {
        let stable = self.seen.get(&key).is_some_and(|f| *f + 1 >= frame);
        self.seen.insert(key, frame);
        // The held checkpoint of this key with the most strokes that are all whole now.
        let restore = self
            .slots
            .iter()
            .enumerate()
            .filter_map(|(i, s)| s.filter(|h| h.key == key && h.strokes <= whole).map(|h| (i, h.strokes)))
            .max_by_key(|&(i, n)| (n, std::cmp::Reverse(i)));
        if let Some((i, _)) = restore {
            if let Some(h) = self.slots[i].as_mut() {
                h.last_used = frame;
            }
        }
        let base = restore.map_or(0, |r| r.1);
        let spacing = options.spacing.max(1) as u64;
        let target = ((whole as u64 / spacing) * spacing) as u32;
        let mut step = CheckpointStep { restore, bake: None };
        let have = self.slots.iter().any(|s| matches!(s, Some(h) if h.key == key && h.strokes == target));
        if !stable || target == 0 || target <= base || have {
            return step;
        }
        let mine: Vec<(usize, u32)> = self.slots.iter().enumerate().filter_map(|(i, s)| s.filter(|h| h.key == key).map(|h| (i, h.strokes))).collect();
        let slot = if mine.len() >= options.capacity.max(1) {
            // The one farthest from the new checkpoint gives up its texture (ties: the fewer
            // strokes, then the lower slot).
            mine.iter().max_by_key(|&&(i, n)| (n.abs_diff(target), std::cmp::Reverse(n), std::cmp::Reverse(i))).map(|&(i, _)| i).unwrap_or(0)
        } else {
            match self.slots.iter().position(|s| s.is_none()) {
                Some(i) => i,
                None => {
                    self.slots.push(None);
                    self.slots.len() - 1
                }
            }
        };
        self.slots[slot] = Some(Held { key, strokes: target, last_used: frame, bytes });
        step.bake = Some((slot, target));
        step
    }

    /// The frame's sweep: `choose` is the texture ledger's rule (rendering/core/texture_ledger.rs
    /// `sweep_choice` under `CHECKPOINT_RESIDENCY`) given (slot, the frame it was last used in,
    /// bytes) of every checkpoint held; the slots whose textures go (forgotten here already).
    pub(crate) fn sweep(&mut self, frame: u64, choose: impl Fn(&[(usize, u64, usize)], u64) -> Vec<usize>) -> Vec<usize> {
        let entries: Vec<(usize, u64, usize)> = self.slots.iter().enumerate().filter_map(|(i, s)| s.map(|h| (i, h.last_used, h.bytes))).collect();
        let gone = choose(&entries, frame);
        for i in &gone {
            self.slots[*i] = None;
        }
        self.seen.retain(|_, f| *f >= frame);
        gone
    }

    /// Forgets everything (the canvas size changed): the slots whose textures go.
    pub(crate) fn clear(&mut self) -> Vec<usize> {
        let gone = self.slots.iter().enumerate().filter(|(_, s)| s.is_some()).map(|(i, _)| i).collect();
        self.slots.clear();
        self.seen.clear();
        gone
    }

    /// The checkpoints held.
    pub(crate) fn held(&self) -> usize {
        self.slots.iter().filter(|s| s.is_some()).count()
    }

    /// Their bytes.
    pub(crate) fn held_bytes(&self) -> usize {
        self.slots.iter().flatten().map(|h| h.bytes).sum()
    }

    /// The strokes of each checkpoint held for a key, fewest first (tests).
    #[cfg(test)]
    pub(crate) fn held_for(&self, key: u64) -> Vec<u32> {
        let mut v: Vec<u32> = self.slots.iter().flatten().filter(|h| h.key == key).map(|h| h.strokes).collect();
        v.sort_unstable();
        v
    }
}
