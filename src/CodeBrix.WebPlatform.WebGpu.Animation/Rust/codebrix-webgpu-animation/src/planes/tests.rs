//! Host tests of DrawingRandom and the 2D gradient noise, with the reference values NoSkia's
//! DrawingRandomTests and DrawingNoiseTests state.

use crate::planes::{DrawingNoise, DrawingRandom};

#[test]
fn next_uint64_matches_the_reference_sequences() {
    let mut zero = DrawingRandom::new(0);
    assert_eq!([zero.next_uint64(), zero.next_uint64(), zero.next_uint64(), zero.next_uint64()], [0x99EC5F36CB75F2B4, 0xBF6E1F784956452A, 0x1A5F849D4933E6E0, 0x6AA594F1262D2D2C]);
    let mut forty_two = DrawingRandom::new(42);
    assert_eq!([forty_two.next_uint64(), forty_two.next_uint64(), forty_two.next_uint64(), forty_two.next_uint64()], [0x15780B2E0C2EC716, 0x6104D9866D113A7E, 0xAE17533239E499A1, 0xECB8AD4703B360A1]);
    assert_eq!(DrawingRandom::new(-1).next_uint64(), 0x8F5520D52A7EAD08);
}

#[test]
fn next_double_and_single_use_the_top_bits() {
    let mut r = DrawingRandom::new(12345);
    assert_eq!([r.next_double(), r.next_double(), r.next_double()], [0.7438081631565894, 0.13004553462783452, 0.9633344930128545]);
    let (mut a, mut b) = (DrawingRandom::new(0), DrawingRandom::new(0));
    assert_eq!(a.next_single(), (b.next_uint64() >> 40) as f32 * (1.0 / 16777216.0));
}

#[test]
fn for_key_derives_the_documented_seed() {
    let mut keyed = DrawingRandom::for_key(7, 3);
    assert_eq!(keyed.seed(), 5112237441497708052);
    assert_eq!(keyed.next_uint64(), 0xD0EA68C108EC9A94);
}

#[test]
fn next_int_stays_in_range_and_refuses_an_empty_range() {
    let mut r = DrawingRandom::new(3);
    for _ in 0..1000 {
        let v = r.next_int_between(-5, 5).unwrap();
        assert!((-5..5).contains(&v));
    }
    assert!(r.next_int(0).is_err());
}

#[test]
fn gradient_noise_is_zero_at_lattice_points_and_in_range_between() {
    for y in -3..=3 {
        for x in -3..=3 {
            assert_eq!(DrawingNoise::gradient(x as f64, y as f64, 11), 0.0);
        }
    }
    let a = DrawingNoise::gradient(1.25, 7.5, 100);
    assert_eq!(a, DrawingNoise::gradient(1.25, 7.5, 100));
    assert_ne!(a, DrawingNoise::gradient(1.25, 7.5, 101));
    for i in 0..500 {
        let v = DrawingNoise::gradient(i as f64 * 0.137, i as f64 * 0.071, 4);
        assert!((-1.0..=1.0).contains(&v));
    }
}
