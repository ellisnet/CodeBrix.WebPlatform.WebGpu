//! The CPU half of the image textures: which picture textures the GPU holds, which COLOUR TABLE
//! each ownership-map texture holds in its rows below the owner numbers, and what a frame must
//! upload - a whole picture, only a colour table, or nothing. gpu.rs does what this decides, so
//! the upload rules (and the statistics they give) are tested on the host. Not in C#.
//!
//! An ownership map's picture is its owner numbers with a colour table in the rows below them.
//! Shapes made from one shape with other colours ([`OwnershipShape::with_colors`](crate::shapes::OwnershipShape::with_colors))
//! share the picture and bring a colour table of their own: the texture of the picture is kept,
//! and when a draw wants another colour table only those rows are written. Two such shapes drawn
//! in ONE frame with different colours need two textures (the frame's writes all land before its
//! draws), so the second gets a texture of its own (a whole upload, once - later frames reuse it).
//!
//! HOW LONG A TEXTURE IS KEPT ([`ResidencyPolicy`], one rule for pictures, stroke sets and brush
//! tips; swept EVERY frame): anything not drawn for more than `keep_frames` frames goes; and while
//! what is held but not drawn this frame weighs more than `idle_budget_bytes`, the least recently
//! drawn of the entries not drawn in the last two frames go first. So a scene that makes a new
//! picture every frame keeps only about the budget's worth of dead textures (before: every one of
//! them for 120 to 180 frames), while pictures that are drawn - every frame, or every other frame
//! - stay. Stroke sets' checkpoints follow the same rule with a budget of their own
//! (`CHECKPOINT_RESIDENCY`).

use std::collections::HashMap;

use peniko::{ImageBrush, ImageData};

/// A picture as a frame uses it: the brush, and for an ownership map the colour table drawn
/// with it (None: the picture's own rows) and the row where the table starts.
#[derive(Clone, Debug)]
pub(crate) struct FrameImage {
    pub(crate) brush: ImageBrush,
    /// An ownership map's colour table (rows of the picture's width) when it is not the
    /// picture's own.
    pub(crate) palette: Option<ImageData>,
    /// The first row of the colour table in the picture (an ownership map's height; 0 for a
    /// picture without one).
    pub(crate) palette_row: u32,
}

impl FrameImage {
    /// A plain picture.
    pub(crate) fn picture(brush: ImageBrush) -> FrameImage {
        FrameImage { brush, palette: None, palette_row: 0 }
    }

    /// The texture's key: the picture's pixels and its width.
    pub(crate) fn key(&self) -> (u64, u32) {
        (self.brush.image.data.id(), self.brush.image.width)
    }

    /// Which colour table the draw wants in the texture's rows (None: the picture's own).
    pub(crate) fn palette_id(&self) -> Option<u64> {
        self.palette.as_ref().map(|p| p.data.id())
    }

    /// The bytes of the whole picture.
    pub(crate) fn whole_bytes(&self) -> usize {
        self.brush.image.width as usize * self.brush.image.height as usize * 4
    }

    /// The bytes of its colour-table rows.
    pub(crate) fn table_bytes(&self) -> usize {
        let i = &self.brush.image;
        i.width as usize * i.height.saturating_sub(self.palette_row) as usize * 4
    }
}

/// What a frame must send for one picture use.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Upload {
    /// The texture holds it already.
    None,
    /// A new texture: the whole picture (with the wanted colour table in its rows).
    Whole,
    /// An existing texture of the picture: only the colour-table rows.
    ColorTable,
}

#[derive(Clone, Copy, Debug)]
struct Slot {
    palette: Option<u64>,
    last_used: u64,
    bytes: usize,
}

/// How long the GPU half keeps a texture or buffer it is not drawing (see the module's text).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ResidencyPolicy {
    /// Frames an entry may go undrawn before it is dropped whatever its size.
    pub(crate) keep_frames: u64,
    /// The bytes of entries not drawn this frame that may stay held.
    pub(crate) idle_budget_bytes: usize,
}

/// The renderer's policy: 120 frames, 16 MiB of idle entries (per kind: pictures, stroke sets,
/// brush tips).
pub(crate) const RESIDENCY: ResidencyPolicy = ResidencyPolicy { keep_frames: 120, idle_budget_bytes: 16 << 20 };

/// The policy of stroke sets' CHECKPOINTS (strokes/stroke_checkpoints.rs): 120 frames, 128 MiB of
/// checkpoints not used this frame (one is 16 bytes per canvas pixel - 14.7 MB at 1280 x 720, 59 MB
/// at 2560 x 1440 - so a draw keeps a few earlier states for seeking back besides the one it uses).
pub(crate) const CHECKPOINT_RESIDENCY: ResidencyPolicy = ResidencyPolicy { keep_frames: 120, idle_budget_bytes: 128 << 20 };

/// The entries to drop at a frame's sweep: (key, the frame it was last drawn in, bytes) in;
/// the keys to drop out (in the order they are dropped). Every entry undrawn for more than
/// `keep_frames`; then, while the undrawn entries left weigh more than the budget, those undrawn
/// for two frames or more, least recently drawn first (ties by key).
pub(crate) fn sweep_choice<K: Copy + Ord>(entries: &[(K, u64, usize)], frame: u64, policy: ResidencyPolicy) -> Vec<K> {
    let mut gone: Vec<K> = Vec::new();
    let mut idle: Vec<(u64, K, usize)> = Vec::new();
    let mut idle_bytes = 0usize;
    for &(key, last_used, bytes) in entries {
        let age = frame.saturating_sub(last_used);
        if age > policy.keep_frames {
            gone.push(key);
        } else if age >= 1 {
            idle_bytes += bytes;
            if age >= 2 {
                idle.push((last_used, key, bytes));
            }
        }
    }
    if idle_bytes > policy.idle_budget_bytes {
        idle.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.cmp(&b.1)));
        for (_, key, bytes) in idle {
            if idle_bytes <= policy.idle_budget_bytes {
                break;
            }
            gone.push(key);
            idle_bytes -= bytes;
        }
    }
    gone
}

/// What a frame needs of buffers and textures that are uploaded WHOLE and once (a stroke set's
/// records, a set of brush tips): which are held, when each was last drawn, and their bytes.
#[derive(Debug)]
pub(crate) struct Residency<K> {
    held: HashMap<K, (u64, usize)>,
}

impl<K> Default for Residency<K> {
    fn default() -> Residency<K> {
        Residency { held: HashMap::new() }
    }
}

impl<K: Copy + Ord + std::hash::Hash> Residency<K> {
    /// Marks an entry drawn this frame; true when it is not held yet (it must be uploaded).
    pub(crate) fn use_now(&mut self, key: K, bytes: usize, frame: u64) -> bool {
        match self.held.get_mut(&key) {
            Some(e) => {
                e.0 = frame;
                false
            }
            None => {
                self.held.insert(key, (frame, bytes));
                true
            }
        }
    }

    /// The frame's sweep: the keys to drop on the GPU (already forgotten here).
    pub(crate) fn sweep(&mut self, frame: u64, policy: ResidencyPolicy) -> Vec<K> {
        let entries: Vec<(K, u64, usize)> = self.held.iter().map(|(k, (f, b))| (*k, *f, *b)).collect();
        let gone = sweep_choice(&entries, frame, policy);
        for k in &gone {
            self.held.remove(k);
        }
        gone
    }

    /// The number held (tests).
    #[cfg(test)]
    pub(crate) fn len(&self) -> usize {
        self.held.len()
    }
}

/// The textures held, per picture: one slot per texture, each with the colour table in it.
#[derive(Default, Debug)]
pub(crate) struct TextureLedger {
    slots: HashMap<(u64, u32), Vec<Option<Slot>>>,
}

impl TextureLedger {
    #[cfg(test)]
    /// The texture slot a picture use draws from this frame, and what must be uploaded to it.
    pub(crate) fn acquire(&mut self, key: (u64, u32), palette: Option<u64>, frame: u64) -> (usize, Upload) {
        self.acquire_bytes(key, palette, frame, 0)
    }

    /// [`acquire`](TextureLedger::acquire), noting the texture's bytes (for the sweep's budget).
    pub(crate) fn acquire_bytes(&mut self, key: (u64, u32), palette: Option<u64>, frame: u64, bytes: usize) -> (usize, Upload) {
        let slots = self.slots.entry(key).or_default();
        // The texture that holds this colour table already.
        if let Some(i) = slots.iter().position(|s| matches!(s, Some(s) if s.palette == palette)) {
            let s = slots[i].as_mut().unwrap();
            s.last_used = frame;
            s.bytes = s.bytes.max(bytes);
            return (i, Upload::None);
        }
        // A texture of this picture not drawn yet this frame: only its colour table changes.
        if let Some(i) = slots.iter().position(|s| matches!(s, Some(s) if s.last_used != frame)) {
            slots[i] = Some(Slot { palette, last_used: frame, bytes });
            return (i, Upload::ColorTable);
        }
        // A new texture (a free slot, or one more).
        let slot = Some(Slot { palette, last_used: frame, bytes });
        match slots.iter().position(|s| s.is_none()) {
            Some(i) => {
                slots[i] = slot;
                (i, Upload::Whole)
            }
            None => {
                slots.push(slot);
                (slots.len() - 1, Upload::Whole)
            }
        }
    }

    #[cfg(test)]
    /// Forgets the textures unused for more than `keep` frames; returns them (to drop on the
    /// GPU). A picture's textures stay while any shape that shares it is drawn.
    pub(crate) fn evict(&mut self, frame: u64, keep: u64) -> Vec<((u64, u32), usize)> {
        let mut gone = Vec::new();
        for (key, slots) in self.slots.iter_mut() {
            for (i, s) in slots.iter_mut().enumerate() {
                if matches!(s, Some(s) if frame - s.last_used > keep) {
                    *s = None;
                    gone.push((*key, i));
                }
            }
        }
        self.slots.retain(|_, slots| slots.iter().any(|s| s.is_some()));
        gone
    }

    /// The frame's sweep under a policy (see the module's text): the textures to drop on the GPU
    /// (already forgotten here).
    pub(crate) fn sweep(&mut self, frame: u64, policy: ResidencyPolicy) -> Vec<((u64, u32), usize)> {
        let mut entries = Vec::new();
        for (key, slots) in &self.slots {
            for (i, s) in slots.iter().enumerate() {
                if let Some(s) = s {
                    entries.push(((*key, i), s.last_used, s.bytes));
                }
            }
        }
        let gone = sweep_choice(&entries, frame, policy);
        for (key, i) in &gone {
            if let Some(slots) = self.slots.get_mut(key) {
                slots[*i] = None;
            }
        }
        self.slots.retain(|_, slots| slots.iter().any(|s| s.is_some()));
        gone
    }

    /// The number of textures held (tests).
    #[cfg(test)]
    pub(crate) fn textures(&self) -> usize {
        self.slots.values().map(|v| v.iter().filter(|s| s.is_some()).count()).sum()
    }
}

/// What a stroke draw must upload this frame - its set's records, its tips - decided as gpu.rs
/// decides it (once per set and per tips while they stay held), with the counts into `stats`.
/// `held` says whether the GPU still has the entry (false forces an upload).
pub(crate) fn stroke_uploads(sets: &mut Residency<u64>, tips: &mut Residency<u64>, op: &crate::rendering::core::build::StrokeOp, frame: u64, held: &dyn Fn(bool, u64) -> bool, stats: &mut crate::rendering::RenderStats) -> (bool, bool) {
    let id = op.set.id();
    let set_bytes = op.set.len() * crate::strokes::stroke_gpu::RECORD_BYTES;
    // A transition's record buffers are its own (gpu.rs uploads them by the morph's identity).
    let set = op.morph.is_none() && (sets.use_now(id, set_bytes, frame) || !held(false, id));
    if set {
        stats.stroke_uploads += 1;
        stats.stroke_upload_bytes += set_bytes;
    }
    let tip = match &op.tips {
        Some(t) => {
            let up = tips.use_now(t.id(), t.bytes(), frame) || !held(true, t.id());
            if up {
                stats.tip_uploads += 1;
                stats.tip_upload_bytes += t.bytes();
            }
            up
        }
        None => false,
    };
    (set, tip)
}

/// The frame's upload counts (into [`RenderStats`](crate::rendering::RenderStats)).
pub(crate) fn count(stats: &mut crate::rendering::RenderStats, image: &FrameImage, upload: Upload) {
    match upload {
        Upload::None => {}
        Upload::Whole => {
            stats.image_uploads += 1;
            stats.image_upload_bytes += image.whole_bytes();
        }
        Upload::ColorTable => {
            stats.color_table_uploads += 1;
            stats.color_table_upload_bytes += image.table_bytes();
        }
    }
}

/// What a frame's pictures upload, decided as gpu.rs decides it (host tests of the rules).
#[cfg(test)]
pub(crate) fn plan(ledger: &mut TextureLedger, images: &[FrameImage], frame: u64) -> crate::rendering::RenderStats {
    let mut stats = crate::rendering::RenderStats::default();
    for image in images {
        let (_, upload) = ledger.acquire(image.key(), image.palette_id(), frame);
        count(&mut stats, image, upload);
    }
    stats
}
