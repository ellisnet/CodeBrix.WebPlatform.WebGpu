//! `BrushTip` and `BrushTips`: the pictures of brush marks a stroke set's strokes are painted
//! with - THE APPLICATION'S pictures (the crate ships none), resampled once on the CPU into one
//! size with a mip chain, uploaded to the GPU once as one texture array (no C# equivalent).

use std::rc::Rc;
use std::sync::atomic::{AtomicU64, Ordering};

use crate::images::ImageSource;
use crate::planes::{DrawingPlane, DrawingPlaneResampling};

static NEXT_TIPS_ID: AtomicU64 = AtomicU64::new(1);

/// One brush mark: how much paint it lays (and, for a surface finish, how thick), measured
/// ALONG the stroke (x: from the first tip to the last) and ACROSS it (y: from the left of the
/// direction of travel to its right), stretched over the stroke's whole length and width (no C#
/// equivalent). Values are 0..1.
#[derive(Clone, Debug, PartialEq)]
pub struct BrushTip {
    /// How much paint, 0..1 (multiplies the stroke's alpha).
    pub coverage: DrawingPlane,
    /// How thick the paint lies, 0..1 (read by a surface finish); None: flat - as thick as it
    /// covers (stored as 1).
    pub height: Option<DrawingPlane>,
}

impl BrushTip {
    /// A tip from planes: the coverage and, optionally, the height (any sizes: both are
    /// resampled to the tips' size by [`BrushTips::new`]).
    pub fn new(coverage: DrawingPlane, height: Option<DrawingPlane>) -> BrushTip {
        BrushTip { coverage, height }
    }

    /// A tip from a picture: the coverage is the picture's ALPHA, and with
    /// `height_from_luminance` the height is its luminance (Rec. 709 weights of the sRGB-encoded
    /// channels; dark = thin), otherwise None (flat). The picture's rows run along the stroke
    /// left to right, its columns across it top (left of travel) to bottom.
    pub fn from_image(picture: &ImageSource, height_from_luminance: bool) -> Result<BrushTip, String> {
        let (w, h) = (picture.width(), picture.height());
        let rgba = picture.to_rgba();
        let mut coverage = Vec::with_capacity((w * h) as usize);
        let mut height = Vec::with_capacity(if height_from_luminance { (w * h) as usize } else { 0 });
        for px in rgba.chunks_exact(4) {
            coverage.push(px[3] as f32 / 255.0);
            if height_from_luminance {
                height.push(((0.2126 * px[0] as f32) + (0.7152 * px[1] as f32) + (0.0722 * px[2] as f32)) / 255.0);
            }
        }
        let coverage = DrawingPlane::from_data(w, h, coverage)?;
        let height = if height_from_luminance { Some(DrawingPlane::from_data(w, h, height)?) } else { None };
        Ok(BrushTip { coverage, height })
    }
}

/// One mip level of every tip: `layers x height x width` texels of two bytes (coverage, height).
#[derive(Clone)]
pub(crate) struct TipLevel {
    pub(crate) width: u32,
    pub(crate) height: u32,
    pub(crate) bytes: Vec<u8>,
}

/// The brush tips of a stroke shape (no C# equivalent): up to [`BrushTips::MAX_TIPS`] tips, all
/// resampled to ONE size (bilinear) and given a mip chain down to 1 x 1 (box filter), on the
/// CPU, once. A record's `tip` is a 1-based index into the list given (0 = no picture). Clones
/// share the pixels and are equal (the GPU uploads them once, as one texture array); two sets of
/// tips made separately are not equal.
#[derive(Clone)]
pub struct BrushTips {
    id: u64,
    size: (u32, u32),
    count: u32,
    levels: Rc<[TipLevel]>,
}

impl PartialEq for BrushTips {
    fn eq(&self, other: &BrushTips) -> bool {
        self.id == other.id
    }
}

impl std::fmt::Debug for BrushTips {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BrushTips").field("id", &self.id).field("size", &self.size).field("count", &self.count).field("levels", &self.levels.len()).finish()
    }
}

fn byte(v: f32) -> u8 {
    if v.is_nan() || v <= 0.0 {
        0
    } else if v >= 1.0 {
        255
    } else {
        ((v * 255.0) + 0.5) as u8
    }
}

impl BrushTips {
    /// The most tips one set of tips holds (a record's tip index is 1..=255; 0 = none).
    pub const MAX_TIPS: usize = 255;

    /// The largest side of a tip, texels.
    pub const MAX_SIZE: u32 = 2048;

    /// Tips resampled to `size` (each side a power of two, 1..=2048 - so every mip level halves
    /// exactly), with their mip chains. The error says what to do (no tips, too many, a size
    /// that is not a power of two).
    pub fn new(size: (u32, u32), tips: &[BrushTip]) -> Result<BrushTips, String> {
        let (w, h) = size;
        if !w.is_power_of_two() || !h.is_power_of_two() || w > BrushTips::MAX_SIZE || h > BrushTips::MAX_SIZE {
            return Err(format!("The brush tips' size {w} x {h} must be a power of two on each side, 1 to {} (for example 64 x 32).", BrushTips::MAX_SIZE));
        }
        if tips.is_empty() || tips.len() > BrushTips::MAX_TIPS {
            return Err(format!("Give 1 to {} brush tips ({} were given).", BrushTips::MAX_TIPS, tips.len()));
        }
        // Each tip's two planes at the size, then halved level by level.
        let mut planes: Vec<(DrawingPlane, Option<DrawingPlane>)> = Vec::with_capacity(tips.len());
        for (i, tip) in tips.iter().enumerate() {
            let coverage = tip.coverage.resized(w, h, Some(DrawingPlaneResampling::Bilinear)).map_err(|e| format!("Brush tip {i}: {e}"))?;
            let height = match &tip.height {
                Some(p) => Some(p.resized(w, h, Some(DrawingPlaneResampling::Bilinear)).map_err(|e| format!("Brush tip {i}: {e}"))?),
                None => None,
            };
            planes.push((coverage, height));
        }
        let count = w.max(h).trailing_zeros() + 1;
        let mut levels = Vec::with_capacity(count as usize);
        for level in 0..count {
            let (lw, lh) = ((w >> level).max(1), (h >> level).max(1));
            let mut bytes = Vec::with_capacity(tips.len() * (lw * lh * 2) as usize);
            for (coverage, height) in &mut planes {
                if level > 0 {
                    *coverage = coverage.downsampled(2)?;
                    if let Some(hp) = height.as_mut() {
                        *hp = hp.downsampled(2)?;
                    }
                }
                debug_assert_eq!(coverage.size(), (lw, lh));
                for (k, c) in coverage.data().iter().enumerate() {
                    bytes.push(byte(*c));
                    bytes.push(height.as_ref().map_or(255, |hp| byte(hp.data()[k])));
                }
            }
            levels.push(TipLevel { width: lw, height: lh, bytes });
        }
        Ok(BrushTips { id: NEXT_TIPS_ID.fetch_add(1, Ordering::Relaxed), size, count: tips.len() as u32, levels: levels.into() })
    }

    /// The tips' identity: the same for every clone (what the renderer keys the one upload by).
    pub fn id(&self) -> u64 {
        self.id
    }

    /// The size every tip was resampled to (level 0).
    pub fn size(&self) -> (u32, u32) {
        self.size
    }

    /// The number of tips.
    pub fn len(&self) -> usize {
        self.count as usize
    }

    /// Always false: a set of tips holds at least one.
    pub fn is_empty(&self) -> bool {
        self.count == 0
    }

    /// The number of mip levels (level 0 at the size, down to 1 x 1).
    pub fn mip_levels(&self) -> u32 {
        self.levels.len() as u32
    }

    /// One tip's texels at a mip level: (coverage, height) pairs, row by row (None outside the
    /// range).
    pub fn texels(&self, tip: usize, level: u32) -> Option<&[u8]> {
        let l = self.levels.get(level as usize)?;
        if tip >= self.count as usize {
            return None;
        }
        let n = (l.width * l.height * 2) as usize;
        Some(&l.bytes[tip * n..(tip + 1) * n])
    }

    /// Every level, for the upload.
    pub(crate) fn levels(&self) -> &[TipLevel] {
        &self.levels
    }

    /// The bytes of every level (the upload's size).
    pub(crate) fn bytes(&self) -> usize {
        self.levels.iter().map(|l| l.bytes.len()).sum()
    }
}
