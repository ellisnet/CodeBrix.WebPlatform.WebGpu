//! `RippleField`, `RippleDrop` and `RippleOptions`: water rings as a FUNCTION OF TIME (no C#
//! equivalent) - drops (given ones and rain) making rings that spread and fade, over an
//! optional swell, inside a rectangle whose edges can reflect them.

use std::rc::Rc;

use crate::water::ripple_math::{self, PackedRipples, RippleGpuHeader, RippleSample, DROP_VEC4S, FLAG_WALLS};
use crate::water::{RippleRain, RippleSwell};
use crate::{PointF, RectangleF};

/// One drop: where and when it falls, how hard, how big (no C# equivalent).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RippleDrop {
    /// Where it lands, calibrated units (the session's coordinates).
    pub center: PointF,
    /// Session seconds it lands.
    pub time: f64,
    /// 1 = the field's `height`; 0 = no ring. At least 0. Default 1.
    pub strength: f32,
    /// Scales the wavelength and the ring width; above 0. Default 1.
    pub size: f32,
}

impl Default for RippleDrop {
    fn default() -> RippleDrop {
        RippleDrop { center: PointF::new(0.0, 0.0), time: 0.0, strength: 1.0, size: 1.0 }
    }
}

impl RippleDrop {
    /// A drop of strength 1 and size 1 landing at `center` at session time `time` (no C#
    /// equivalent).
    pub fn new(center: PointF, time: f64) -> RippleDrop {
        RippleDrop { center, time, ..RippleDrop::default() }
    }

    fn validate(&self) -> Result<(), String> {
        if !(self.center.x.is_finite() && self.center.y.is_finite() && self.time.is_finite()) {
            return Err("A drop's centre and time must be finite numbers.".to_string());
        }
        if !(self.strength.is_finite() && self.strength >= 0.0) {
            return Err(format!("A drop's strength must be a finite number of at least 0 (it is {}).", self.strength));
        }
        if !(self.size.is_finite() && self.size > 0.0) {
            return Err(format!("A drop's size must be a finite number above 0 (it is {}).", self.size));
        }
        Ok(())
    }
}

/// How a field's rings look and move (no C# equivalent).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RippleOptions {
    /// Calibrated units per second the ring's front travels. Default 90.
    pub speed: f32,
    /// Crest to crest, calibrated units (times a drop's size). Default 16.
    pub wavelength: f32,
    /// How many crests are visible behind the front (the ring width is `rings x wavelength /
    /// 2`). Default 3.
    pub rings: f32,
    /// Seconds for a drop's rings to fall to 1/e. Default 1.8.
    pub decay: f32,
    /// The amplitude of a strength-1 drop, calibrated units. Default 1.
    pub height: f32,
    /// The field's rectangle reflects rings (mirror images: one bounce off each wall and the
    /// corners). Default false.
    pub walls: bool,
}

impl Default for RippleOptions {
    fn default() -> RippleOptions {
        RippleOptions { speed: 90.0, wavelength: 16.0, rings: 3.0, decay: 1.8, height: 1.0, walls: false }
    }
}

impl RippleOptions {
    fn validate(&self) -> Result<(), String> {
        for (name, value) in [("speed", self.speed), ("wavelength", self.wavelength), ("rings", self.rings), ("decay", self.decay)] {
            if !(value.is_finite() && value > 0.0) {
                return Err(format!("The ripple option `{name}` must be a finite number above 0 (it is {value})."));
            }
        }
        if !self.height.is_finite() {
            return Err(format!("The ripple option `height` must be a finite number (it is {}).", self.height));
        }
        Ok(())
    }
}

/// A RIPPLE FIELD (no C# equivalent): the water's height and slope over a rectangle at any
/// session time, FROM A FORMULA - drops (and rain) making rings that spread and fade, over an
/// optional swell. Reached by playing or by seeking, a time shows the same water: nothing is
/// simulated step by step, nothing is remembered between calls, and frames may be drawn in any
/// order. Clones share the drop list. A refraction or a reflection draws it on the GPU; the
/// methods here are its CPU twin (the same single-precision arithmetic, so the two agree within
/// float rounding).
///
/// ONE DROP, at age `a` (seconds since it landed) and distance `r` from its centre: the front is
/// at `R = speed a`; with `u = r - R`, `lambda = wavelength x size`, `w = rings x lambda / 2`,
/// `h = strength x height x e^(-a / decay) x ease(a) x S(r) x E(u) x sin(2 pi u / lambda)`,
/// where `S(r) = 1 / sqrt(1 + r / lambda)` (the 2D spreading), `E(u) = exp(-(u / w)^2)` behind
/// the front and `exp(-(u / (lambda / 4))^2)` ahead of it (a sharp leading edge; nothing
/// outruns the front) and `ease(a) = smoothstep(0, lambda / speed, a)` (a ring appears over one
/// period, no pop). The front is the centre of the ring's wave packet: the zero crossing between
/// its strongest trough, a quarter wavelength inside, and its leading crest, about 0.15
/// wavelength outside (the crest leads, so nothing is seen ahead of it). A drop counts from the
/// moment it lands for
/// [`drop_life`](RippleField::drop_life) seconds (then it is under 1 % and skipped).
///
/// The SWELL (optional): its waves summed under the drops. WALLS (option): each drop also
/// sounds from its mirror images across the place's edges (3 x 3 sources) - exactly a
/// rectangular pool's reflection for the first bounce; later bounces are not drawn. RAIN
/// (optional): [`RippleRain`], drops of each session second from that second's own seed.
///
/// LIMITS: formula rings, not a simulation - rings pass through each other (a linear sum), do
/// not meet shapes or masks (only the place's walls, first bounce), make no splashes; at most
/// [`RippleField::MAX_LIVE_DROPS`] drops count at a time (the strongest at that time).
#[derive(Clone, Debug, PartialEq)]
pub struct RippleField {
    place: RectangleF,
    options: RippleOptions,
    drops: Rc<[RippleDrop]>,
    rain: Option<RippleRain>,
    swell: Option<RippleSwell>,
}

impl RippleField {
    /// The most drops that count at one time (given ones and rain together); beyond it the
    /// weakest at that time are left out - on the CPU and on the GPU alike.
    pub const MAX_LIVE_DROPS: usize = 256;

    /// A still field over `place` (calibrated units) with no drops (no C# equivalent). The error
    /// says what is wrong: an empty or non-finite place, an option that is out of range.
    pub fn new(place: RectangleF, options: RippleOptions) -> Result<RippleField, String> {
        if place.is_empty() || !(place.x.is_finite() && place.y.is_finite() && place.width.is_finite() && place.height.is_finite()) {
            return Err("A ripple field's place must be a finite rectangle with an area.".to_string());
        }
        options.validate()?;
        Ok(RippleField { place, options, drops: Rc::from(Vec::new()), rain: None, swell: None })
    }

    /// The field with these drops ADDED to the ones it has (kept sorted by time; drops at the
    /// same time keep their order). The error names the first drop that is wrong.
    pub fn with_drops(self, drops: Vec<RippleDrop>) -> Result<RippleField, String> {
        for (i, drop) in drops.iter().enumerate() {
            drop.validate().map_err(|e| format!("Drop {i}: {e}"))?;
        }
        let mut all: Vec<RippleDrop> = self.drops.iter().copied().chain(drops).collect();
        all.sort_by(|a, b| a.time.total_cmp(&b.time));
        Ok(RippleField { drops: Rc::from(all), ..self })
    }

    /// The field with one more drop (a click's place and time, for example). The error says
    /// what is wrong with the drop.
    pub fn with_drop(self, drop: RippleDrop) -> Result<RippleField, String> {
        self.with_drops(vec![drop])
    }

    /// The field with rain (replacing any it had). The error says what to change.
    pub fn with_rain(self, rain: RippleRain) -> Result<RippleField, String> {
        rain.validate()?;
        Ok(RippleField { rain: Some(rain), ..self })
    }

    /// The field with a swell under the drops (replacing any it had). The error names the wave
    /// that is wrong.
    pub fn with_swell(self, swell: RippleSwell) -> Result<RippleField, String> {
        swell.validate()?;
        Ok(RippleField { swell: Some(swell), ..self })
    }

    /// The rectangle the field covers (and whose edges reflect with `walls`), calibrated units.
    pub fn place(&self) -> RectangleF {
        self.place
    }

    /// How the rings look and move.
    pub fn options(&self) -> RippleOptions {
        self.options
    }

    /// The given drops, sorted by time (rain's drops are not in it: see
    /// [`live_drops`](RippleField::live_drops)).
    pub fn drops(&self) -> &[RippleDrop] {
        &self.drops
    }

    /// The rain, if any.
    pub fn rain(&self) -> Option<&RippleRain> {
        self.rain.as_ref()
    }

    /// The swell, if any.
    pub fn swell(&self) -> Option<&RippleSwell> {
        self.swell.as_ref()
    }

    /// Seconds a drop counts after it lands: `decay x ln(100)` (then its rings are under 1 % and
    /// it is skipped).
    pub fn drop_life(&self) -> f64 {
        self.options.decay as f64 * 100f64.ln()
    }

    /// The radius of a drop's ring FRONT at a time, `speed x age` (`None` before it lands and
    /// after [`drop_life`](RippleField::drop_life)). The front is the centre of the ring's wave
    /// packet, with the strongest trough a quarter wavelength inside it and the leading crest
    /// about 0.15 wavelength outside it (see the type's formula).
    pub fn ring_radius(&self, drop: &RippleDrop, seconds: f64) -> Option<f32> {
        let age = seconds - drop.time;
        if !(age >= 0.0 && age < self.drop_life()) {
            return None;
        }
        Some((self.options.speed as f64 * age) as f32)
    }

    /// The water's height at a point and session time, calibrated units (the CPU twin of what
    /// the GPU draws).
    pub fn height_at(&self, x: f32, y: f32, seconds: f64) -> f32 {
        self.sample_at(x, y, seconds).height
    }

    /// The water's slope (dh/dx, dh/dy) at a point and session time - analytic, the CPU twin.
    pub fn slope_at(&self, x: f32, y: f32, seconds: f64) -> (f32, f32) {
        let s = self.sample_at(x, y, seconds);
        (s.slope[0], s.slope[1])
    }

    /// The water's Laplacian at a point and session time (where light focuses: negative on the
    /// crests, positive in the troughs), the CPU twin. Analytic and exact for each ring (every
    /// factor's derivative is kept); within a quarter wavelength of a young drop's centre the
    /// `1 / r` term is held at `r = wavelength / 4`.
    pub fn focus_at(&self, x: f32, y: f32, seconds: f64) -> f32 {
        self.sample_at(x, y, seconds).focus
    }

    /// The drops that count at a time - given ones and rain, at most
    /// [`MAX_LIVE_DROPS`](RippleField::MAX_LIVE_DROPS) (the strongest then), given drops first by
    /// time, then rain by window: what the GPU is sent. A drop counts while `0 <= age <
    /// drop_life` and its amplitude then is not 0 (at the very moment it lands it has none).
    pub fn live_drops(&self, seconds: f64) -> Vec<RippleDrop> {
        self.select_live(seconds).0
    }

    /// The live drops (capped) and how many the cap left out.
    pub(crate) fn select_live(&self, seconds: f64) -> (Vec<RippleDrop>, usize) {
        if !seconds.is_finite() {
            return (Vec::new(), 0);
        }
        let life = self.drop_life();
        let mut live = Vec::new();
        let first = self.drops.partition_point(|d| d.time <= seconds - life);
        let last = self.drops.partition_point(|d| d.time <= seconds);
        live.extend(self.drops[first..last].iter().copied().filter(|d| self.amplitude(d, seconds) != 0.0));
        if let Some(rain) = &self.rain {
            let mut window = Vec::new();
            for k in (seconds - life).floor() as i64..=seconds.floor() as i64 {
                window.clear();
                rain.window(k, self.place, &mut window);
                live.extend(window.iter().copied().filter(|d| {
                    let age = seconds - d.time;
                    age >= 0.0 && age < life && self.amplitude(d, seconds) != 0.0
                }));
            }
        }
        if live.len() <= RippleField::MAX_LIVE_DROPS {
            return (live, 0);
        }
        let skipped = live.len() - RippleField::MAX_LIVE_DROPS;
        let mut order: Vec<(usize, f64)> = live.iter().enumerate().map(|(i, d)| (i, self.amplitude(d, seconds).abs())).collect();
        order.sort_by(|a, b| b.1.total_cmp(&a.1).then(a.0.cmp(&b.0)));
        let mut keep: Vec<usize> = order[..RippleField::MAX_LIVE_DROPS].iter().map(|o| o.0).collect();
        keep.sort_unstable();
        (keep.into_iter().map(|i| live[i]).collect(), skipped)
    }

    /// A drop's amplitude at a time: `strength x height x e^(-age / decay) x ease(age)`, in
    /// double precision (0 outside its life).
    pub(crate) fn amplitude(&self, drop: &RippleDrop, seconds: f64) -> f64 {
        let age = seconds - drop.time;
        if !(age >= 0.0 && age < self.drop_life()) {
            return 0.0;
        }
        let period = (self.options.wavelength as f64 * drop.size as f64) / self.options.speed as f64;
        let t = (age / period).clamp(0.0, 1.0);
        let ease = t * t * (3.0 - 2.0 * t);
        drop.strength as f64 * self.options.height as f64 * (-age / self.options.decay as f64).exp() * ease
    }

    /// The field packed for the GPU at a time, its records counted from `base_offset` (in vec4
    /// units) of the frame's buffer.
    pub(crate) fn pack(&self, seconds: f64, base_offset: u32) -> PackedRipples {
        let (live, skipped) = self.select_live(seconds);
        let mut records = Vec::with_capacity(live.len() * DROP_VEC4S + 8);
        for d in &live {
            let age = seconds - d.time;
            let lambda = self.options.wavelength * d.size;
            let front = (self.options.speed as f64 * age) as f32;
            let width = self.options.rings * lambda * 0.5;
            records.extend(ripple_math::drop_record([d.center.x, d.center.y], front, self.amplitude(d, seconds) as f32, lambda, width, age as f32));
        }
        let waves = match &self.swell {
            Some(swell) => {
                records.extend(swell.records(seconds));
                swell.waves.len()
            }
            None => 0,
        };
        let header = RippleGpuHeader {
            place: [self.place.x, self.place.y, self.place.right(), self.place.bottom()],
            words: [base_offset, live.len() as u32, waves as u32, if self.options.walls { FLAG_WALLS } else { 0 }],
        };
        PackedRipples { header, records, skipped }
    }

    /// Height, slope and focus at a point, through the GPU's arithmetic.
    pub(crate) fn sample_at(&self, x: f32, y: f32, seconds: f64) -> RippleSample {
        let packed = self.pack(seconds, 0);
        ripple_math::evaluate(&packed.header, &packed.records, x, y)
    }
}
