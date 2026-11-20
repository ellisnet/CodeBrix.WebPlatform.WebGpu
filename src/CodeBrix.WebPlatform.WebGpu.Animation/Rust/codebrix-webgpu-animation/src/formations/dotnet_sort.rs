//! .NET's unstable sort, reproduced (internal): `List<T>.Sort(Comparison<T>)` is an
//! INTROSPECTIVE sort (insertion sort below 17 elements, median-of-three quicksort, heap sort past
//! a depth limit of 2 x (floor(log2 n) + 1)), so elements with EQUAL keys end in an order of its
//! own. NoSkia sorts formation targets with it; to give the same targets in the same order when
//! keys tie (a grid with no jitter, say), this follows that algorithm step for step - as written
//! in .NET's `ArraySortHelper<T>` (INFERENCE: ported from the published algorithm of .NET 5 and
//! later, which has not changed since; not checked against a .NET run here).

use std::cmp::Ordering;

const INTROSORT_SIZE_THRESHOLD: usize = 16;

/// Sorts as .NET's `List<T>.Sort(Comparison<T>)` does.
pub(crate) fn sort_unstable_as_dotnet<T: Copy>(keys: &mut [T], compare: impl Fn(&T, &T) -> Ordering) {
    if keys.len() > 1 {
        let depth = 2 * ((keys.len() as u32).ilog2() as usize + 1);
        intro_sort(keys, depth, &compare);
    }
}

fn swap_if_greater<T: Copy>(keys: &mut [T], i: usize, j: usize, compare: &impl Fn(&T, &T) -> Ordering) {
    if compare(&keys[i], &keys[j]) == Ordering::Greater {
        keys.swap(i, j);
    }
}

fn intro_sort<T: Copy>(keys: &mut [T], mut depth_limit: usize, compare: &impl Fn(&T, &T) -> Ordering) {
    let mut partition_size = keys.len();
    while partition_size > 1 {
        if partition_size <= INTROSORT_SIZE_THRESHOLD {
            if partition_size == 2 {
                swap_if_greater(keys, 0, 1, compare);
                return;
            }
            if partition_size == 3 {
                swap_if_greater(keys, 0, 1, compare);
                swap_if_greater(keys, 0, 2, compare);
                swap_if_greater(keys, 1, 2, compare);
                return;
            }
            insertion_sort(&mut keys[..partition_size], compare);
            return;
        }
        if depth_limit == 0 {
            heap_sort(&mut keys[..partition_size], compare);
            return;
        }
        depth_limit -= 1;
        let p = pick_pivot_and_partition(&mut keys[..partition_size], compare);
        intro_sort(&mut keys[p + 1..partition_size], depth_limit, compare);
        partition_size = p;
    }
}

fn pick_pivot_and_partition<T: Copy>(keys: &mut [T], compare: &impl Fn(&T, &T) -> Ordering) -> usize {
    let hi = keys.len() - 1;
    let middle = hi >> 1;
    swap_if_greater(keys, 0, middle, compare);
    swap_if_greater(keys, 0, hi, compare);
    swap_if_greater(keys, middle, hi, compare);
    let pivot = keys[middle];
    keys.swap(middle, hi - 1);
    let (mut left, mut right) = (0usize, hi - 1);
    while left < right {
        loop {
            left += 1;
            if compare(&keys[left], &pivot) != Ordering::Less {
                break;
            }
        }
        loop {
            right -= 1;
            if compare(&pivot, &keys[right]) != Ordering::Less {
                break;
            }
        }
        if left >= right {
            break;
        }
        keys.swap(left, right);
    }
    if left != hi - 1 {
        keys.swap(left, hi - 1);
    }
    left
}

fn heap_sort<T: Copy>(keys: &mut [T], compare: &impl Fn(&T, &T) -> Ordering) {
    let n = keys.len();
    let mut i = n >> 1;
    while i >= 1 {
        down_heap(keys, i, n, compare);
        i -= 1;
    }
    let mut i = n;
    while i > 1 {
        keys.swap(0, i - 1);
        down_heap(keys, 1, i - 1, compare);
        i -= 1;
    }
}

fn down_heap<T: Copy>(keys: &mut [T], mut i: usize, n: usize, compare: &impl Fn(&T, &T) -> Ordering) {
    let d = keys[i - 1];
    while i <= n >> 1 {
        let mut child = 2 * i;
        if child < n && compare(&keys[child - 1], &keys[child]) == Ordering::Less {
            child += 1;
        }
        if compare(&d, &keys[child - 1]) != Ordering::Less {
            break;
        }
        keys[i - 1] = keys[child - 1];
        i = child;
    }
    keys[i - 1] = d;
}

fn insertion_sort<T: Copy>(keys: &mut [T], compare: &impl Fn(&T, &T) -> Ordering) {
    for i in 0..keys.len() - 1 {
        let t = keys[i + 1];
        let mut j = i as isize;
        while j >= 0 && compare(&t, &keys[j as usize]) == Ordering::Less {
            keys[j as usize + 1] = keys[j as usize];
            j -= 1;
        }
        keys[(j + 1) as usize] = t;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sorts_every_length_by_key_like_any_sort() {
        for n in 0..200usize {
            let mut v: Vec<(u32, usize)> = (0..n).map(|i| (((i * 7919) % 13) as u32, i)).collect();
            sort_unstable_as_dotnet(&mut v, |a, b| a.0.cmp(&b.0));
            assert!(v.windows(2).all(|w| w[0].0 <= w[1].0), "length {n}");
            let mut idx: Vec<usize> = v.iter().map(|p| p.1).collect();
            idx.sort();
            assert_eq!(idx, (0..n).collect::<Vec<_>>());
        }
    }

    #[test]
    fn small_ties_follow_dotnets_insertion_sort_which_is_stable() {
        let mut v = [(1, 'a'), (0, 'b'), (1, 'c'), (0, 'd')];
        sort_unstable_as_dotnet(&mut v, |a, b| a.0.cmp(&b.0));
        assert_eq!(v, [(0, 'b'), (0, 'd'), (1, 'a'), (1, 'c')]);
    }
}
