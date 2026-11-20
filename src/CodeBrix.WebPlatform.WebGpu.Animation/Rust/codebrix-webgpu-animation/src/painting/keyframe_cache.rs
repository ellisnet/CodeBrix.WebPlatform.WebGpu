//! The keyframe store both players share (internal; C#'s `KeyframeCache` and `KeyframeTiming`
//! WITHOUT the background worker): keyframes are painted ON DEMAND, on the caller's one thread,
//! and at most `capacity` painted ones are kept - the ones farthest from what is shown are
//! dropped first, by C#'s rule.

/// C#'s `KeyframeTiming.KeyframeAt`: the keyframe a time starts from, `floor(seconds /
/// interval)`, and the fraction of the interval past it. The error says what to do.
pub(crate) fn keyframe_at(seconds: f64, interval: f64) -> Result<(i32, f32), String> {
    if !seconds.is_finite() {
        return Err("The time must be a finite number of seconds.".to_string());
    }
    let position = seconds / interval;
    let floor = position.floor();
    if floor < i32::MIN as f64 + 2.0 || floor > i32::MAX as f64 - 3.0 {
        return Err("The time is too far from 0 for a keyframe number: keep it within about two billion intervals.".to_string());
    }
    Ok((floor as i32, (position - floor) as f32))
}

/// Painted keyframes by number, at most `capacity` of them.
pub(crate) struct KeyframeCache<T> {
    capacity: usize,
    entries: Vec<(i32, T)>,
    spare: Option<T>,
    paint_count: u64,
}

impl<T> KeyframeCache<T> {
    pub(crate) fn new(capacity: usize) -> KeyframeCache<T> {
        KeyframeCache { capacity, entries: Vec::with_capacity(capacity + 1), spare: None, paint_count: 0 }
    }

    pub(crate) fn paint_count(&self) -> u64 {
        self.paint_count
    }

    pub(crate) fn cached_count(&self) -> usize {
        self.entries.len()
    }

    pub(crate) fn contains(&self, key: i32) -> bool {
        self.entries.iter().any(|(k, _)| *k == key)
    }

    pub(crate) fn get(&self, key: i32) -> Option<&T> {
        self.entries.iter().find(|(k, _)| *k == key).map(|(_, frame)| frame)
    }

    /// Paints a keyframe that is not cached yet: into a recycled frame (made ready by `reset`) or
    /// a new one (`allocate`), with `paint`; a failed painting keeps nothing and its error is
    /// returned (a later request paints it afresh). Then keeps at most `capacity` keyframes
    /// around `focus`, the keyframe shown now.
    pub(crate) fn ensure(&mut self, key: i32, focus: i32, allocate: impl FnOnce() -> Result<T, String>, reset: impl FnOnce(&mut T), paint: impl FnOnce(i32, &mut T) -> Result<(), String>) -> Result<(), String> {
        if self.contains(key) {
            return Ok(());
        }
        let mut frame = match self.spare.take() {
            Some(mut frame) => {
                reset(&mut frame);
                frame
            }
            None => allocate()?,
        };
        self.paint_count += 1;
        if let Err(e) = paint(key, &mut frame) {
            self.spare = Some(frame);
            return Err(e);
        }
        self.entries.push((key, frame));
        self.evict(focus);
        Ok(())
    }

    // How far a keyframe is from what is needed: the ones ahead of the focus first, in order,
    // then the ones behind it (C#'s `Distance`)
    fn distance(key: i32, focus: i32) -> i64 {
        if key >= focus {
            key as i64 - focus as i64
        } else {
            (focus as i64 - key as i64) + i32::MAX as i64
        }
    }

    // C#'s `Evict`: drops the keyframes farthest from the focus (ties: the larger number), never
    // the three the focus needs (focus, focus + 1, focus + 2). The last dropped frame is kept to
    // be painted into again.
    fn evict(&mut self, focus: i32) {
        while self.entries.len() > self.capacity {
            let mut victim: Option<usize> = None;
            let mut worst = -1i64;
            for (i, (key, _)) in self.entries.iter().enumerate() {
                if *key >= focus && *key <= focus.saturating_add(2) {
                    continue;
                }
                let distance = KeyframeCache::<T>::distance(*key, focus);
                if distance > worst || (distance == worst && victim.is_some_and(|v| *key > self.entries[v].0)) {
                    worst = distance;
                    victim = Some(i);
                }
            }
            let Some(v) = victim else {
                return;
            };
            let (_, frame) = self.entries.swap_remove(v);
            self.spare = Some(frame);
        }
    }
}
