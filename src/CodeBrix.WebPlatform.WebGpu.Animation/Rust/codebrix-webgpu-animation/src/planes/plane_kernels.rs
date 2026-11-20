//! The arithmetic shared by the planes, the plane images, the transitions and the motion-blur
//! helpers (C#'s internal `PlaneKernels`), on ONE thread: C# shares rows out across threads, but
//! every value there is computed by one thread in a fixed order, so these plain loops give the
//! same values. Every expression keeps C#'s order of operations and its float / double widths.

/// The library's float -> byte rule for 0..1 values (C# `UnitToByte`): NaN and values at or
/// below 0 give 0, values at or above 1 give 255, anything else `(byte)(value * 255 + 0.5)`.
pub(crate) fn unit_to_byte(value: f32) -> u8 {
    if !(value > 0.0) {
        return 0;
    }
    if value >= 1.0 {
        return 255;
    }
    ((value * 255.0) + 0.5) as u8
}

/// The same rule for values already on the 0..255 scale (C# `LevelToByte`).
pub(crate) fn level_to_byte(level: f32) -> u8 {
    if !(level > 0.0) {
        return 0;
    }
    if level >= 255.0 {
        return 255;
    }
    (level + 0.5) as u8
}

/// `byte / 255f` (C# `ByteToUnit`).
pub(crate) fn byte_to_unit(value: u8) -> f32 {
    value as f32 / 255.0
}

/// 0..1, NaN to 0 (C# `Clamp01`).
pub(crate) fn clamp01(value: f32) -> f32 {
    if !(value > 0.0) {
        return 0.0;
    }
    if value >= 1.0 {
        1.0
    } else {
        value
    }
}

/// The Gaussian kernel (C# `GaussianKernel`): radius `(int)(4 * sigma + 0.5)`; each weight
/// `exp(-0.5 * i * i / (sigma * sigma))`, their sum (from -radius to +radius) and each weight
/// divided by that sum in double; only the normalised weight is rounded, once, to float.
pub(crate) fn gaussian_kernel(sigma: f32) -> (Vec<f32>, i32) {
    let s = sigma as f64;
    let radius = ((4.0 * s) + 0.5) as i32;
    let mut weights = Vec::with_capacity((2 * radius + 1) as usize);
    let mut sum = 0.0f64;
    for i in -radius..=radius {
        let weight = (-0.5 * i as f64 * i as f64 / (s * s)).exp();
        weights.push(weight);
        sum += weight;
    }
    (weights.iter().map(|w| (w / sum) as f32).collect(), radius)
}

/// The mirror edge rule (C# `Mirror`): index -1 reads 0, -2 reads 1, `size` reads `size - 1` -
/// repeated for kernels wider than the plane.
pub(crate) fn mirror(mut i: i64, size: i64) -> i64 {
    while i < 0 || i >= size {
        if i < 0 {
            i = -i - 1;
        }
        if i >= size {
            i = (2 * size) - i - 1;
        }
    }
    i
}

/// The separable blur, IN PLACE (C# `Blur` with source = destination): a horizontal pass into a
/// scratch buffer, then a vertical pass back. Each output value is the sum, in kernel order from
/// -radius to +radius and starting from 0, of weight * sample. A sigma that is not above 0 does
/// nothing.
pub(crate) fn blur(data: &mut [f32], width: usize, height: usize, sigma: f32) {
    if !(sigma > 0.0) {
        return;
    }
    let (kernel, radius) = gaussian_kernel(sigma);
    let r = radius as i64;
    let w = width as i64;
    let mut across = vec![0.0f32; width * height];
    // The interior columns (x - radius and x + radius inside the row) read no mirrored sample.
    // They are summed tap by tap across the row - each column still adds weight * sample in
    // kernel order starting from 0, exactly C#'s per-column loop, so every value is the same;
    // the loop order only lets the compiler work on many columns at once.
    let (interior_start, interior_end) = (radius as usize, if w > 2 * r { width - radius as usize } else { 0 });
    for y in 0..height {
        let row = y * width;
        let source = &data[row..row + width];
        let target = &mut across[row..row + width];
        if interior_start < interior_end {
            let count = interior_end - interior_start;
            for (k, weight) in kernel.iter().enumerate() {
                let taps = &source[k..k + count];
                for (t, s) in target[interior_start..interior_end].iter_mut().zip(taps) {
                    *t += *weight * *s;
                }
            }
        }
        for x in 0..width {
            if x >= interior_start && x < interior_end {
                continue;
            }
            let xi = x as i64;
            let mut total = 0.0f32;
            for k in -r..=r {
                total += kernel[(k + r) as usize] * source[mirror(xi + k, w) as usize];
            }
            target[x] = total;
        }
    }
    let h = height as i64;
    for y in 0..height {
        let row = y * width;
        let target = &mut data[row..row + width];
        target.fill(0.0);
        for k in -r..=r {
            let weight = kernel[(k + r) as usize];
            let from = mirror(y as i64 + k, h) as usize * width;
            for (t, s) in target.iter_mut().zip(&across[from..from + width]) {
                *t += weight * *s;
            }
        }
    }
}

/// Bilinear sampling positions along one axis (C# `BilinearAxis`): the pixel-centre grid
/// `(i + 0.5) * from / to - 0.5`, or the corner-aligned grid `i * (from - 1) / (to - 1)`, in
/// double, clamped to the source; the left index is the floor (capped at from - 2) and the
/// fraction `position - left` is stored as float.
pub(crate) fn bilinear_axis(from: usize, to: usize, align_corners: bool) -> (Vec<usize>, Vec<f32>) {
    let mut index = vec![0usize; to];
    let mut fraction = vec![0.0f32; to];
    for i in 0..to {
        if from == 1 {
            continue;
        }
        let mut position = if align_corners {
            if to == 1 {
                0.0
            } else {
                i as f64 * (from as f64 - 1.0) / (to as f64 - 1.0)
            }
        } else {
            ((i as f64 + 0.5) * from as f64 / to as f64) - 0.5
        };
        if position < 0.0 {
            position = 0.0;
        }
        if position > (from - 1) as f64 {
            position = (from - 1) as f64;
        }
        let left = (position.floor() as i64).min(from as i64 - 2);
        index[i] = left as usize;
        fraction[i] = (position - left as f64) as f32;
    }
    (index, fraction)
}

/// Nearest sampling (C# `NearestIndex`): output pixel i reads source pixel
/// `floor((2i + 1) * from / (2 * to))`, in exact integers.
pub(crate) fn nearest_index(i: usize, from: usize, to: usize) -> usize {
    let n = ((2 * i as i64 + 1) * from as i64) / (2 * to as i64);
    n.min(from as i64 - 1) as usize
}

/// Nearest resize (C# `ResizeNearest`).
pub(crate) fn resize_nearest(source: &[f32], source_width: usize, source_height: usize, width: usize, height: usize) -> Vec<f32> {
    let columns: Vec<usize> = (0..width).map(|x| nearest_index(x, source_width, width)).collect();
    let mut destination = vec![0.0f32; width * height];
    for y in 0..height {
        let from = nearest_index(y, source_height, height) * source_width;
        let row = y * width;
        for x in 0..width {
            destination[row + x] = source[from + columns[x]];
        }
    }
    destination
}

/// Bilinear resize (C# `ResizeBilinear`): `p00*w00 + p10*w10 + p01*w01 + p11*w11`, summed left to
/// right, with `w00 = (1 - fx)(1 - fy)`, `w10 = fx(1 - fy)`, `w01 = (1 - fx)fy`, `w11 = fx fy`.
pub(crate) fn resize_bilinear(source: &[f32], source_width: usize, source_height: usize, width: usize, height: usize, align_corners: bool) -> Vec<f32> {
    let (columns, fx) = bilinear_axis(source_width, width, align_corners);
    let (rows, fy) = bilinear_axis(source_height, height, align_corners);
    let step_x = if source_width > 1 { 1 } else { 0 };
    let step_y = if source_height > 1 { source_width } else { 0 };
    let mut destination = vec![0.0f32; width * height];
    for y in 0..height {
        let top = rows[y] * source_width;
        let v = fy[y];
        let iv = 1.0 - v;
        let row = y * width;
        for x in 0..width {
            let u = fx[x];
            let iu = 1.0 - u;
            let p00 = top + columns[x];
            let (w00, w10, w01, w11) = (iu * iv, u * iv, iu * v, u * v);
            destination[row + x] = (source[p00] * w00) + (source[p00 + step_x] * w10) + (source[p00 + step_y] * w01) + (source[p00 + step_y + step_x] * w11);
        }
    }
    destination
}

/// Box downsample by a whole factor (C# `Downsample`): each block row summed left to right from
/// its first value, the row sums added top to bottom from 0, the total divided by the pixel count
/// as a float.
pub(crate) fn downsample(source: &[f32], source_width: usize, source_height: usize, factor: usize, width: usize, height: usize) -> Vec<f32> {
    let mut destination = vec![0.0f32; width * height];
    for y in 0..height {
        let y0 = y * factor;
        let y1 = (y0 + factor).min(source_height);
        for x in 0..width {
            let x0 = x * factor;
            let x1 = (x0 + factor).min(source_width);
            let mut sum = 0.0f32;
            for sy in y0..y1 {
                let row = sy * source_width;
                let mut row_sum = source[row + x0];
                for sx in (x0 + 1)..x1 {
                    row_sum += source[row + sx];
                }
                sum += row_sum;
            }
            destination[(y * width) + x] = sum / (((x1 - x0) * (y1 - y0)) as i32) as f32;
        }
    }
    destination
}

/// Mixes straight-alpha colour a towards b by t in premultiplied space (C# `MixUnit`): t at or
/// below 0 gives a, at or above 1 gives b; equal alphas give the straight mix with alpha kept.
#[allow(clippy::too_many_arguments)]
pub(crate) fn mix_unit(ar: f32, ag: f32, ab: f32, aa: f32, br: f32, bg: f32, bb: f32, ba: f32, t: f32) -> [f32; 4] {
    if !(t > 0.0) {
        return [ar, ag, ab, aa];
    }
    if t >= 1.0 {
        return [br, bg, bb, ba];
    }
    let s = 1.0 - t;
    if aa == ba {
        return [(ar * s) + (br * t), (ag * s) + (bg * t), (ab * s) + (bb * t), aa];
    }
    let (wa, wb) = (aa * s, ba * t);
    let a = wa + wb;
    if a == 0.0 {
        return [0.0; 4];
    }
    [((ar * wa) + (br * wb)) / a, ((ag * wa) + (bg * wb)) / a, ((ab * wa) + (bb * wb)) / a, a]
}

/// The same mix on straight-alpha RGBA bytes, in float on the 0..255 scale (C# `MixBytes`).
pub(crate) fn mix_bytes(a: [u8; 4], b: [u8; 4], t: f32) -> [u8; 4] {
    if !(t > 0.0) {
        return a;
    }
    if t >= 1.0 {
        return b;
    }
    let s = 1.0 - t;
    let f = |v: u8| v as f32;
    if a[3] == b[3] {
        return [((f(a[0]) * s) + (f(b[0]) * t) + 0.5) as u8, ((f(a[1]) * s) + (f(b[1]) * t) + 0.5) as u8, ((f(a[2]) * s) + (f(b[2]) * t) + 0.5) as u8, a[3]];
    }
    let (wa, wb) = (f(a[3]) * s, f(b[3]) * t);
    let alpha = wa + wb;
    if !(alpha > 0.0) {
        return [0; 4];
    }
    [
        level_to_byte(((f(a[0]) * wa) + (f(b[0]) * wb)) / alpha),
        level_to_byte(((f(a[1]) * wa) + (f(b[1]) * wb)) / alpha),
        level_to_byte(((f(a[2]) * wa) + (f(b[2]) * wb)) / alpha),
        level_to_byte(alpha),
    ]
}

/// Four bytes of a pixel at a byte offset.
pub(crate) fn pixel_at(bytes: &[u8], offset: usize) -> [u8; 4] {
    [bytes[offset], bytes[offset + 1], bytes[offset + 2], bytes[offset + 3]]
}
