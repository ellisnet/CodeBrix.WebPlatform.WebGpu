//! Host tests of the stagger calculation, ported from CodeBrix.Imaging.Drawing's
//! DrawingAnimationsStaggerTests (its expected values), with text pieces as the elements.

use crate::animation::{Easing, EasingKind};
use crate::geometry::{PointF, RectangleF};
use crate::models::{DrawingTransform, StaggerOptions, StaggerOrder, StaggerTracks};

/// C#'s Squares(count): 10 x 10 squares 20 units apart.
fn squares(count: usize) -> Vec<Option<RectangleF>> {
    (0..count).map(|i| Some(RectangleF::new(i as f32 * 20.0, 0.0, 10.0, 10.0))).collect()
}

#[test]
fn stagger_fades_each_element_in_after_the_one_before() {
    //Arrange
    let options = StaggerOptions { start_time: 1.0, interval: 0.5, duration: 1.0, easing: EasingKind::Linear, ..StaggerOptions::default() };

    //Act
    let tracks = StaggerTracks::create(&squares(3), &options).unwrap();

    //Assert
    assert_eq!(tracks.len(), 3);
    assert!((tracks[0].state_at(1.5).opacity - 0.5).abs() < 0.0001);
    assert_eq!(tracks[1].state_at(1.5).opacity, 0.0);
    assert_eq!(tracks[2].state_at(1.5).opacity, 0.0);
    assert_eq!(tracks[2].state_at(10.0).opacity, 1.0);
    assert_eq!(tracks[0].state_at(0.0).opacity, 0.0);
    assert!(tracks[0].position.is_none() && tracks[0].scale.is_none() && tracks[0].rotation.is_none());
    assert_eq!((tracks[1].start_time, tracks[1].end_time), (1.5, 2.5));
}

#[test]
fn stagger_moves_scales_and_turns_about_each_centre() {
    let options = StaggerOptions { interval: 1.0, duration: 1.0, from_offset: PointF::new(0.0, 30.0), from_scale: 0.5, from_rotation: 90.0, easing: EasingKind::Linear, ..StaggerOptions::default() };
    let tracks = StaggerTracks::create(&squares(2), &options).unwrap();
    assert_eq!(tracks[1].anchor, Some(PointF::new(25.0, 5.0)));
    let start = tracks[0].state_at(0.0).apply_to(DrawingTransform::default(), tracks[0].anchor);
    assert_eq!(start.translation(), PointF::new(0.0, 30.0));
    assert_eq!(start.scale_x(), 0.5);
    assert_eq!(start.rotation(), 90.0);
    let end = tracks[1].state_at(5.0).apply_to(DrawingTransform::default(), tracks[1].anchor);
    assert_eq!(end.translation(), PointF::new(0.0, 0.0));
    assert_eq!(end.scale_y(), 1.0);
    assert_eq!(end.rotation(), 0.0);
    assert_eq!(end.anchor(), PointF::new(25.0, 5.0));
}

#[test]
fn stagger_orders_the_starts() {
    for (order, ranks) in [(StaggerOrder::Forward, [0, 1, 2, 3, 4]), (StaggerOrder::Reverse, [4, 3, 2, 1, 0]), (StaggerOrder::CenterOut, [2, 1, 0, 1, 2])] {
        let tracks = StaggerTracks::create(&squares(5), &StaggerOptions { order, interval: 1.0, duration: 0.5, easing: EasingKind::Linear, ..StaggerOptions::default() }).unwrap();
        for (i, rank) in ranks.iter().enumerate() {
            // Each element is half way in a quarter second after its start.
            assert!((tracks[i].state_at(*rank as f64 + 0.25).opacity - 0.5).abs() < 0.0001, "{order:?} {i}");
            assert_eq!(tracks[i].rank, *rank);
        }
    }
    // CenterOut with an even count: floor(|i - 1.5|) = 1, 0, 0, 1.
    let ranks: Vec<usize> = (0..4).map(|i| StaggerOrder::CenterOut.rank_of(i, 4)).collect();
    assert_eq!(ranks, vec![1, 0, 0, 1]);
}

#[test]
fn the_default_stagger_eases_cubic_out_over_its_span() {
    let tracks = StaggerTracks::create(&squares(2), &StaggerOptions::default()).unwrap();
    // Default: interval 0.05, duration 0.4, CubicOut, 0 -> 1. Element 1 at t = 0.15 is a quarter in.
    let t = 0.05 + 0.1;
    let expected = Easing::lerp_f32(0.0, 1.0, Easing::evaluate(EasingKind::CubicOut, 0.25));
    assert!((tracks[1].state_at(t).opacity - expected).abs() < 1e-6);
    assert!((tracks[1].progress_at(t) - 0.25).abs() < 1e-12);
    assert_eq!(tracks[1].progress_at(-1.0), 0.0);
    assert_eq!(tracks[1].progress_at(9.0), 1.0);
    // Without bounds, or without anchoring at the centre, the anchor is left alone.
    assert_eq!(StaggerTracks::create(&[None], &StaggerOptions::default()).unwrap()[0].anchor, None);
    assert_eq!(StaggerTracks::create(&squares(1), &StaggerOptions { anchor_at_center: false, ..StaggerOptions::default() }).unwrap()[0].anchor, None);
}

#[test]
fn a_zero_duration_jumps_at_the_start() {
    let tracks = StaggerTracks::create(&squares(1), &StaggerOptions { start_time: 2.0, duration: 0.0, ..StaggerOptions::default() }).unwrap();
    assert_eq!(tracks[0].state_at(1.999).opacity, 0.0);
    assert_eq!(tracks[0].state_at(2.0).opacity, 1.0);
}

#[test]
fn stagger_rejects_bad_input() {
    assert!(StaggerTracks::create(&squares(1), &StaggerOptions { interval: -1.0, ..StaggerOptions::default() }).unwrap_err().contains("interval"));
    assert!(StaggerTracks::create(&squares(1), &StaggerOptions { from_opacity: 2.0, ..StaggerOptions::default() }).unwrap_err().contains("from_opacity"));
    assert!(StaggerTracks::create(&squares(1), &StaggerOptions { duration: f64::NAN, ..StaggerOptions::default() }).is_err());
    assert!(StaggerTracks::create(&squares(1), &StaggerOptions { from_scale: f32::INFINITY, ..StaggerOptions::default() }).is_err());
}

#[test]
fn text_pieces_stagger_about_their_own_centres() {
    let Some(fonts) = crate::text::layout_tests::fonts() else { return };
    let layout = crate::text::TextLayoutEngine::layout(&fonts, "Hi there", 40.0, None, None).unwrap();
    let block = crate::shapes::TextBlockShape::new(layout, 100.0, 50.0, None).unwrap();
    let words = block.create_pieces(crate::text::TextUnit::Word);
    let bounds: Vec<Option<RectangleF>> = words.iter().map(|w| Some(w.get_bounds())).collect();
    let tracks = StaggerTracks::create(&bounds, &StaggerOptions { from_scale: 0.0, ..StaggerOptions::default() }).unwrap();
    let b = words[1].get_bounds();
    assert_eq!(tracks[1].anchor, Some(PointF::new(b.x + b.width / 2.0, b.y + b.height / 2.0)));
    assert_eq!(tracks[1].state_at(0.0).scale, Some(0.0));
    assert_eq!(tracks[1].state_at(1.0).scale, Some(1.0));
}
