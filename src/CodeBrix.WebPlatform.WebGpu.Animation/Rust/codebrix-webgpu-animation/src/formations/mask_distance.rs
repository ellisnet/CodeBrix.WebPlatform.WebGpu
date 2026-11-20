//! The exact Euclidean distance transform of a pixel set (internal; C#'s `MaskDistance`):
//! Felzenszwalb and Huttenlocher's lower envelope of parabolas, columns then rows, in units -
//! pixel spacing `unit_x` across, `unit_y` down. Double arithmetic in C#'s order.

/// For every pixel, the distance from its centre to the nearest pixel centre on the other side:
/// with `from_inside`, inside pixels measure to the nearest outside pixel; without, outside
/// pixels to the nearest inside one. Infinity when there is none.
pub(crate) fn compute(inside: &[bool], width: usize, height: usize, unit_x: f64, unit_y: f64, from_inside: bool) -> Vec<f32> {
    let n = width.max(height);
    let mut f = vec![0.0f64; n];
    let mut d = vec![0.0f64; n];
    let mut v = vec![0usize; n];
    let mut z = vec![0.0f64; n + 1];
    let mut squared = vec![0.0f64; width * height];
    for x in 0..width {
        for y in 0..height {
            let feature = inside[(y * width) + x] != from_inside;
            f[y] = if feature { 0.0 } else { f64::INFINITY };
        }
        transform(&f, height, unit_y, &mut d, &mut v, &mut z);
        for y in 0..height {
            squared[(y * width) + x] = d[y];
        }
    }
    let mut result = vec![0.0f32; width * height];
    for y in 0..height {
        for x in 0..width {
            f[x] = squared[(y * width) + x];
        }
        transform(&f, width, unit_x, &mut d, &mut v, &mut z);
        for x in 0..width {
            result[(y * width) + x] = d[x].sqrt() as f32;
        }
    }
    result
}

fn transform(f: &[f64], n: usize, spacing: f64, d: &mut [f64], v: &mut [usize], z: &mut [f64]) {
    let mut k: isize = -1;
    for q in 0..n {
        if f[q] == f64::INFINITY {
            continue;
        }
        let mut crossing = f64::NEG_INFINITY;
        while k >= 0 {
            let p = v[k as usize];
            let zq = q as f64 * spacing;
            let zp = p as f64 * spacing;
            crossing = ((f[q] + (zq * zq)) - (f[p] + (zp * zp))) / (2.0 * (zq - zp));
            if crossing <= z[k as usize] {
                k -= 1;
            } else {
                break;
            }
        }
        k += 1;
        v[k as usize] = q;
        z[k as usize] = if k == 0 { f64::NEG_INFINITY } else { crossing };
        z[k as usize + 1] = f64::INFINITY;
    }
    let mut j = 0usize;
    for p in 0..n {
        if k < 0 {
            d[p] = f64::INFINITY;
            continue;
        }
        let zp = p as f64 * spacing;
        while z[j + 1] < zp {
            j += 1;
        }
        let offset = (p as f64 - v[j] as f64) * spacing;
        d[p] = (offset * offset) + f[v[j]];
    }
}
