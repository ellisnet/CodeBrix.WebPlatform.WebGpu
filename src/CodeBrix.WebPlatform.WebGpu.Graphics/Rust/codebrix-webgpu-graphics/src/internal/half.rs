//! Half-precision floats as bits (the RGBA16Float texels an environment panorama is uploaded as),
//! without a crate: round to nearest, ties to even.

/// The 16-bit IEEE half-precision bits of `value`, rounded to nearest (ties to even); values too
/// large for a half become infinity, NaN stays NaN.
pub(crate) fn f32_to_f16_bits(value: f32) -> u16 {
    let bits = value.to_bits();
    let sign = ((bits >> 16) & 0x8000) as u16;
    let exponent = ((bits >> 23) & 0xff) as i32;
    let mantissa = bits & 0x007f_ffff;
    if exponent == 0xff {
        return sign | 0x7c00 | if mantissa != 0 { 0x0200 } else { 0 };
    }
    let half_exponent = exponent - 127 + 15;
    if half_exponent >= 0x1f {
        return sign | 0x7c00;
    }
    if half_exponent <= 0 {
        if half_exponent < -10 {
            return sign;
        }
        let full = mantissa | 0x0080_0000;
        let shift = (14 - half_exponent) as u32;
        let mut half = full >> shift;
        let remainder = full & ((1 << shift) - 1);
        let halfway = 1 << (shift - 1);
        if remainder > halfway || (remainder == halfway && half & 1 == 1) {
            half += 1;
        }
        return sign | half as u16;
    }
    let mut half = (sign as u32) | ((half_exponent as u32) << 10) | (mantissa >> 13);
    let remainder = mantissa & 0x1fff;
    if remainder > 0x1000 || (remainder == 0x1000 && half & 1 == 1) {
        half += 1;
    }
    half as u16
}

/// The value of half-precision bits (for tests and readbacks).
#[cfg(test)]
pub(crate) fn f16_bits_to_f32(bits: u16) -> f32 {
    let sign = if bits & 0x8000 != 0 { -1.0 } else { 1.0 };
    let exponent = ((bits >> 10) & 0x1f) as i32;
    let mantissa = (bits & 0x03ff) as f32;
    match exponent {
        0 => sign * mantissa * 2f32.powi(-24),
        0x1f => {
            if mantissa == 0.0 {
                sign * f32::INFINITY
            } else {
                f32::NAN
            }
        }
        _ => sign * (1.0 + mantissa / 1024.0) * 2f32.powi(exponent - 15),
    }
}

#[cfg(test)]
mod tests {
    use super::{f16_bits_to_f32, f32_to_f16_bits};

    #[test]
    fn exact_values_convert_to_their_half_bits() {
        assert_eq!(f32_to_f16_bits(1.0), 0x3c00);
        assert_eq!(f32_to_f16_bits(0.5), 0x3800);
        assert_eq!(f32_to_f16_bits(2.0), 0x4000);
        assert_eq!(f32_to_f16_bits(-2.0), 0xc000);
        assert_eq!(f32_to_f16_bits(0.0), 0x0000);
        assert_eq!(f32_to_f16_bits(65504.0), 0x7bff);
    }

    #[test]
    fn large_values_become_infinity_and_nan_stays_nan() {
        assert_eq!(f32_to_f16_bits(1e9), 0x7c00);
        assert_eq!(f32_to_f16_bits(f32::INFINITY), 0x7c00);
        assert!(f16_bits_to_f32(f32_to_f16_bits(f32::NAN)).is_nan());
    }

    #[test]
    fn small_values_become_subnormals_or_zero() {
        assert_eq!(f32_to_f16_bits(2f32.powi(-24)), 0x0001);
        assert_eq!(f32_to_f16_bits(2f32.powi(-14)), 0x0400);
        assert_eq!(f32_to_f16_bits(1e-10), 0x0000);
    }

    #[test]
    fn rounding_is_to_nearest_even_and_round_trips_within_a_step() {
        // 1 + 2^-11 lies exactly between 1 and the next half (1 + 2^-10): ties go to the even 1.
        assert_eq!(f32_to_f16_bits(1.0 + 2f32.powi(-11)), 0x3c00);
        assert_eq!(f32_to_f16_bits(1.0 + 3.0 * 2f32.powi(-11)), 0x3c02);
        for value in [0.001f32, 0.1, 0.75, 3.3, 123.456, 4000.0] {
            let back = f16_bits_to_f32(f32_to_f16_bits(value));
            assert!((back - value).abs() <= value * 2f32.powi(-11), "{value} -> {back}");
        }
    }
}
