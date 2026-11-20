//! Host tests: playback as a function of the session time - loop, ping-pong, hold, speed, the
//! begin time, the start offset, ranges, whole frames, markers as named points in time.

use std::sync::Arc;

use crate::geometry::RectangleF;
use crate::lottie::test_support::*;
use crate::lottie::*;

const BASICS: &str = include_str!("test_files/basics.json");

/// basics.json: frames 0..20 at 10 fps, a 2-second range.
fn frame(options: LottiePlayOptions, time: f64) -> f64 {
    load(BASICS).frame_at(&options, time).unwrap()
}

#[test]
fn loop_repeats_the_range() {
    //Arrange
    let options = LottiePlayOptions::default();
    //Act
    let frames: Vec<f64> = [0.0, 0.5, 1.99, 2.0, 2.5, 7.25].iter().map(|t| frame(options.clone(), *t)).collect();
    //Assert
    let expected = [0.0, 5.0, 19.9, 0.0, 5.0, 12.5];
    for (f, e) in frames.iter().zip(expected) {
        assert!(near(*f, e, 1e-9), "{frames:?}");
    }
}

#[test]
fn ping_pong_plays_forwards_then_backwards() {
    //Arrange
    let options = LottiePlayOptions::playing(LottiePlayback::PingPong);
    //Act
    let frames: Vec<f64> = [0.5, 1.5, 2.5, 3.9, 4.5].iter().map(|t| frame(options.clone(), *t)).collect();
    //Assert
    let expected = [5.0, 15.0, 15.0, 1.0, 5.0];
    for (f, e) in frames.iter().zip(expected) {
        assert!(near(*f, e, 1e-9), "{frames:?}");
    }
    assert!(near(frame(options, 2.0), 20.0, 1e-5) && frame(LottiePlayOptions::playing(LottiePlayback::PingPong), 2.0) < 20.0, "the turn shows the last frame just inside the exclusive out point");
}

#[test]
fn hold_plays_once_and_holds_the_last_frame() {
    //Arrange
    let lottie = load(BASICS);
    let options = LottiePlayOptions::playing(LottiePlayback::Hold);
    let shape = DrawingLottieShape::new(Arc::new(lottie), RectangleF::new(0.0, 0.0, 100.0, 100.0), DrawingLottieAspect::Meet, options.clone()).unwrap();
    //Act
    let (mid, end, later) = (shape.frame_at(1.0), shape.frame_at(2.0), shape.frame_at(60.0));
    //Assert
    assert!(near(mid, 10.0, 1e-9));
    assert!(end < 20.0 && near(end, 20.0, 1e-5) && later == end);
    assert!(!shape.is_finished(1.99) && shape.is_finished(2.0) && shape.is_finished(60.0));
}

#[test]
fn speed_scales_time_and_negative_speed_plays_backwards() {
    //Arrange
    let double = LottiePlayOptions { speed: 2.0, ..LottiePlayOptions::default() };
    let backwards = LottiePlayOptions { speed: -1.0, ..LottiePlayOptions::default() };
    let frozen = LottiePlayOptions { speed: 0.0, start_offset: 0.7, ..LottiePlayOptions::default() };
    //Act
    let (a, b, c, d) = (frame(double.clone(), 0.25), frame(double, 1.5), frame(backwards, 0.5), frame(frozen, 99.0));
    //Assert
    assert!(near(a, 5.0, 1e-9) && near(b, 10.0, 1e-9) && near(c, 15.0, 1e-9) && near(d, 7.0, 1e-9), "{a} {b} {c} {d}");
}

#[test]
fn begin_time_and_start_offset_place_playback_on_the_session_timeline() {
    //Arrange
    let options = LottiePlayOptions { begin_time: 3.0, start_offset: 0.5, ..LottiePlayOptions::default() };
    //Act
    let frames: Vec<f64> = [0.0, 2.99, 3.0, 3.5, 4.5].iter().map(|t| frame(options.clone(), *t)).collect();
    //Assert
    let expected = [5.0, 5.0, 5.0, 10.0, 0.0];
    for (f, e) in frames.iter().zip(expected) {
        assert!(near(*f, e, 1e-9), "{frames:?}");
    }
}

#[test]
fn a_marker_range_plays_its_span() {
    //Arrange
    let options = LottiePlayOptions { range: LottieRange::Marker("second half".to_string()), ..LottiePlayOptions::default() };
    //Act
    let frames: Vec<f64> = [0.0, 0.5, 1.0, 1.25].iter().map(|t| frame(options.clone(), *t)).collect();
    //Assert
    let expected = [10.0, 15.0, 10.0, 12.5];
    for (f, e) in frames.iter().zip(expected) {
        assert!(near(*f, e, 1e-9), "{frames:?}");
    }
}

#[test]
fn whole_frames_rounds_the_frame_down() {
    //Arrange
    let options = LottiePlayOptions { whole_frames: true, ..LottiePlayOptions::default() };
    //Act
    let (a, b) = (frame(options.clone(), 0.57), frame(options, 1.0));
    //Assert
    assert_eq!((a, b), (5.0, 10.0));
}

#[test]
fn seeking_changes_the_frame_of_one_shape_only() {
    //Arrange
    let lottie = Arc::new(load(BASICS));
    let destination = RectangleF::new(0.0, 0.0, 100.0, 100.0);
    let mut seeked = DrawingLottieShape::new(lottie.clone(), destination, DrawingLottieAspect::Meet, LottiePlayOptions::default()).unwrap();
    let other = DrawingLottieShape::new(lottie, destination, DrawingLottieAspect::Meet, LottiePlayOptions::default()).unwrap();
    //Act - at session time 5 s, jump this one to 1.5 s into the document
    seeked.set_play_options(LottiePlayOptions { begin_time: 5.0, start_offset: 1.5, ..LottiePlayOptions::default() }).unwrap();
    //Assert
    assert!(near(seeked.frame_at(5.0), 15.0, 1e-9) && near(seeked.frame_at(5.25), 17.5, 1e-9));
    assert!(near(other.frame_at(5.25), 12.5, 1e-9));
    assert!(seeked.set_play_options(LottiePlayOptions { range: LottieRange::Marker("nope".to_string()), ..LottiePlayOptions::default() }).is_err());
    assert!(near(seeked.frame_at(5.0), 15.0, 1e-9), "a refused change changes nothing");
}

#[test]
fn markers_become_named_points_in_time() {
    //Arrange
    let lottie = Arc::new(load(BASICS));
    let destination = RectangleF::new(0.0, 0.0, 100.0, 100.0);
    let plain = DrawingLottieShape::new(lottie.clone(), destination, DrawingLottieAspect::Meet, LottiePlayOptions::default()).unwrap();
    let later = DrawingLottieShape::new(lottie.clone(), destination, DrawingLottieAspect::Meet, LottiePlayOptions { begin_time: 2.0, speed: 2.0, ..LottiePlayOptions::default() }).unwrap();
    let second_half = DrawingLottieShape::new(lottie, destination, DrawingLottieAspect::Meet, LottiePlayOptions { range: LottieRange::Marker("second half".to_string()), begin_time: 1.0, ..LottiePlayOptions::default() }).unwrap();
    //Act
    let (a, b, c) = (plain.marker_times(), later.marker_times(), second_half.marker_times());
    //Assert
    assert_eq!(a, vec![("intro".to_string(), 0.0), ("second half".to_string(), 1.0), ("flash".to_string(), 1.5)]);
    assert_eq!(b, vec![("intro".to_string(), 2.0), ("second half".to_string(), 2.5), ("flash".to_string(), 2.75)]);
    assert_eq!(c, vec![("second half".to_string(), 1.0), ("flash".to_string(), 1.5)]);
}
