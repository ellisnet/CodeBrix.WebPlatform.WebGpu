//! The dither (dot) pattern of a level-of-detail fade (Not in C#): the code an instance record
//! carries for it (`PbrInstance::flags[1]`), and - for the host tests - which pixels a fading level
//! keeps, the same arithmetic as `dither_rank` / `dither_keeps` / `dither_coverage` in
//! `pbr_body.wgsl` and `shadow_depth.wgsl` (a test checks the WGSL text).
//!
//! THE PATTERN: an 8 x 8 ordered (Bayer) matrix gives every pixel a rank 0..63 from the low three
//! bits of its x and y. A fade at step `k` of [`DITHER_STEPS`] keeps, for the level coming IN, the
//! pixels of rank below `k` and, for the level going OUT, the pixels of rank `k` and above: the two
//! sets are complementary for every `k` - every pixel is drawn by exactly one of the two levels, so
//! a cross-fade never leaves a hole and never covers a pixel twice. Integer arithmetic only, so
//! every shader variant (and this file) agrees on every pixel. The test is per PIXEL (the
//! fragment's position), so a discard drops the whole pixel at 1 and at 4 samples alike.

/// The steps of a fade: a fade's coverage moves in steps of 1/64 (one rank of the 8 x 8 matrix).
pub(crate) const DITHER_STEPS: u32 = 64;
/// The code's flag of the level coming in (keeps ranks below the step).
pub(crate) const DITHER_INCOMING: u32 = 0x100;
/// The code's flag of the level going out (keeps ranks from the step up).
pub(crate) const DITHER_OUTGOING: u32 = 0x200;

/// The rank of pixel (x, y) in the 8 x 8 ordered-dither matrix, 0..63: the bits of `x ^ y` and of
/// `y` interleaved, lowest bits first and most significant (the recursive Bayer matrix).
#[cfg(test)]
pub(crate) fn dither_rank(x: u32, y: u32) -> u32 {
    let (x, y) = (x & 7, y & 7);
    let v = x ^ y;
    ((v & 1) << 5) | ((y & 1) << 4) | ((v & 2) << 2) | ((y & 2) << 1) | ((v & 4) >> 1) | ((y & 4) >> 2)
}

/// The record code of a fading level at `step` (0..=64) of its fade: `incoming` for the level that
/// fades in (coverage step / 64), else the level that fades out (coverage 1 - step / 64). 0 (no
/// dither) when that coverage is whole.
pub(crate) fn dither_code(incoming: bool, step: u32) -> u32 {
    let step = step.min(DITHER_STEPS);
    match (incoming, step) {
        (true, DITHER_STEPS) | (false, 0) => 0,
        (true, _) => DITHER_INCOMING | step,
        (false, _) => DITHER_OUTGOING | step,
    }
}

/// True when a draw with `code` keeps pixel (x, y) (code 0: every pixel).
#[cfg(test)]
pub(crate) fn dither_keeps(x: u32, y: u32, code: u32) -> bool {
    let step = code & 0xFF;
    if code & DITHER_INCOMING != 0 {
        dither_rank(x, y) < step
    } else if code & DITHER_OUTGOING != 0 {
        dither_rank(x, y) >= step
    } else {
        true
    }
}

/// The share of pixels a draw with `code` keeps - what a BLENDED part multiplies its alpha by
/// instead of dithering (code 0: 1).
#[cfg(test)]
pub(crate) fn dither_coverage(code: u32) -> f32 {
    let step = (code & 0xFF).min(DITHER_STEPS) as f32 / DITHER_STEPS as f32;
    if code & DITHER_INCOMING != 0 {
        step
    } else if code & DITHER_OUTGOING != 0 {
        1.0 - step
    } else {
        1.0
    }
}

#[cfg(test)]
mod tests {
    use super::{dither_code, dither_coverage, dither_keeps, dither_rank, DITHER_STEPS};

    #[test]
    fn the_ranks_of_an_8_by_8_tile_are_every_value_once_and_repeat_every_8_pixels() {
        let mut ranks: Vec<u32> = (0..8).flat_map(|y| (0..8).map(move |x| dither_rank(x, y))).collect();
        // The 2 x 2 corner is the classic [[0, 32], [48, 16]] (the 2 x 2 matrix times 16).
        assert_eq!([dither_rank(0, 0), dither_rank(1, 0), dither_rank(0, 1), dither_rank(1, 1)], [0, 32, 48, 16]);
        ranks.sort_unstable();
        assert_eq!(ranks, (0..64).collect::<Vec<u32>>());
        assert_eq!(dither_rank(3, 5), dither_rank(3 + 8 * 41, 5 + 8 * 7));
    }

    #[test]
    fn the_two_patterns_of_a_fade_are_complementary_for_every_step() {
        for step in 0..=DITHER_STEPS {
            let incoming = dither_code(true, step);
            let outgoing = dither_code(false, step);
            let mut kept_in = 0;
            // A 24 x 16 area (three tiles by two, starting off the tile grid).
            for y in 5..21u32 {
                for x in 3..27u32 {
                    let a = dither_keeps(x, y, incoming);
                    let b = dither_keeps(x, y, outgoing);
                    assert!(a != b, "step {step}: pixel ({x}, {y}) is kept by {}", if a { "both" } else { "neither" });
                    kept_in += u32::from(a);
                }
            }
            // Coverage is exactly step / 64 (6 whole tiles), and the alpha form agrees.
            assert_eq!(kept_in, step * 6, "step {step}");
            assert_eq!(dither_coverage(incoming) + dither_coverage(outgoing), 1.0);
            assert_eq!(dither_coverage(incoming), step as f32 / 64.0);
        }
    }

    #[test]
    fn a_whole_coverage_needs_no_dither_and_steps_are_clamped() {
        assert_eq!(dither_code(true, 64), 0);
        assert_eq!(dither_code(false, 0), 0);
        assert_eq!(dither_code(true, 99), 0);
        assert_eq!(dither_coverage(0), 1.0);
        assert!((0..64).all(|x| dither_keeps(x, x / 3, 0)));
        // The step and the side fit the code's low bits.
        assert_eq!(dither_code(true, 1), 0x101);
        assert_eq!(dither_code(false, 63), 0x23F);
    }

    #[test]
    fn the_shaders_use_the_same_arithmetic() {
        // The WGSL copies of the rank and the keep test (pbr_body.wgsl, shadow_depth.wgsl) must
        // stay the same text as each other and the same arithmetic as dither_rank above.
        let rank = "return ((v & 1u) << 5u) | ((y & 1u) << 4u) | ((v & 2u) << 2u) | ((y & 2u) << 1u) | ((v & 4u) >> 1u) | ((y & 4u) >> 2u);";
        for (name, body) in [("pbr_body", crate::shaders::PbrShader::BODY), ("shadow_depth", crate::shaders::ShadowDepthShader::BODY)] {
            assert!(body.contains(rank), "{name}.wgsl: the rank differs");
            assert!(body.contains("let x = u32(pixel.x) & 7u;") && body.contains("let y = u32(pixel.y) & 7u;"), "{name}.wgsl: the pixel differs");
            assert!(body.contains("if ((code & 0x100u) != 0u) {\n        return rank < step;"), "{name}.wgsl: the incoming test differs");
            assert!(body.contains("if ((code & 0x200u) != 0u) {\n        return rank >= step;"), "{name}.wgsl: the outgoing test differs");
        }
    }
}
