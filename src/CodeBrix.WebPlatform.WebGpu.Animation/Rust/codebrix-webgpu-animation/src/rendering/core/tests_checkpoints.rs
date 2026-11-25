//! Host tests of the GPU half's keep/drop rule for stroke checkpoints: the texture ledger's
//! policy with the checkpoints' own budget (`CHECKPOINT_RESIDENCY`), and a resize forgetting them.

use crate::rendering::core::texture_ledger::{sweep_choice, CHECKPOINT_RESIDENCY};
use crate::strokes::stroke_checkpoints::CheckpointCache;
use crate::strokes::StrokeCheckpoints;

fn options(spacing: usize, capacity: usize) -> StrokeCheckpoints {
    StrokeCheckpoints { spacing, capacity }
}

#[test]
fn checkpoints_follow_the_texture_ledgers_rule_and_go_with_a_resize() {
    //Arrange: two keys' checkpoints of 100 MiB each.
    let mut cache = CheckpointCache::default();
    let o = options(10, 4);
    let big = 100 << 20;
    for frame in 1..=2 {
        cache.step(1, 10, o, frame, big);
        cache.step(2, 10, o, frame, big);
    }
    let rule = |e: &[(usize, u64, usize)], f: u64| sweep_choice(e, f, CHECKPOINT_RESIDENCY);
    //Act + Assert: key 1 drawn on; key 2 idle - kept while within the budget (one of 100 MiB).
    for frame in 3..6 {
        cache.step(1, 10, o, frame, big);
        assert!(cache.sweep(frame, rule).is_empty(), "frame {frame}");
    }
    assert_eq!(cache.held(), 2);
    // A third key's checkpoint: at frame 9 the ones not used this frame weigh 300 MiB (key 1's,
    // used in the frame before, counts but stays): the least recently used go until 128 MiB is
    // not exceeded - key 2's (last used at frame 2), then key 3's (frame 7).
    cache.step(3, 10, o, 6, big);
    cache.step(3, 10, o, 7, big);
    cache.step(1, 10, o, 8, big);
    let gone = cache.sweep(9, rule);
    assert_eq!(gone, vec![1, 2], "key 2's slot first, then key 3's");
    assert_eq!(cache.held(), 1);
    assert_eq!(cache.step(1, 10, o, 10, big).restore, Some((0, 10)), "key 1's stays");
    // Unused for more than 120 frames: gone whatever the size.
    let mut small = CheckpointCache::default();
    small.step(1, 10, o, 1, 8);
    small.step(1, 10, o, 2, 8);
    assert!(small.sweep(100, rule).is_empty());
    assert_eq!(small.sweep(123, rule), vec![0]);
    // A resize forgets every checkpoint.
    assert_eq!(cache.clear(), vec![0]);
    assert_eq!(cache.held(), 0);
}
