//! The size of a canvas's surface: the canvas's displayed size in CSS pixels times the browser's
//! device pixel ratio, within the device's largest texture.

/// The device pixel ratio to use: the browser's, or 1 when it gives none (or nonsense).
pub(crate) fn pixel_ratio(reported: Option<f64>) -> f64 {
    reported.filter(|r| r.is_finite() && *r > 0.0).unwrap_or(1.0)
}

/// The surface size in device pixels for a canvas displayed at `css_width` x `css_height` CSS
/// pixels, each side at least 1 and at most `limit` (the device's `max_texture_dimension_2d`);
/// None while the canvas is not laid out (no size).
pub(crate) fn surface_size(css_width: i32, css_height: i32, pixel_ratio: f64, limit: u32) -> Option<(u32, u32)> {
    if css_width <= 0 || css_height <= 0 {
        return None;
    }
    let limit = limit.max(1);
    let side = |css: i32| ((css as f64 * pixel_ratio).round() as u32).clamp(1, limit);
    Some((side(css_width), side(css_height)))
}

#[cfg(test)]
mod tests {
    use super::{pixel_ratio, surface_size};

    #[test]
    fn the_surface_is_the_displayed_size_times_the_ratio() {
        assert_eq!(surface_size(388, 218, 2.0, 8192), Some((776, 436)));
        assert_eq!(surface_size(976, 549, 1.0, 8192), Some((976, 549)));
        assert_eq!(surface_size(100, 50, 1.25, 8192), Some((125, 63)));
    }

    #[test]
    fn a_canvas_without_a_size_has_no_surface() {
        assert_eq!(surface_size(0, 100, 1.0, 8192), None);
        assert_eq!(surface_size(100, -1, 1.0, 8192), None);
    }

    #[test]
    fn the_surface_stays_within_the_device_limit() {
        assert_eq!(surface_size(5000, 3000, 2.0, 8192), Some((8192, 6000)));
        assert_eq!(surface_size(1, 1, 0.1, 8192), Some((1, 1)));
    }

    #[test]
    fn a_missing_or_nonsense_ratio_is_one() {
        assert_eq!(pixel_ratio(None), 1.0);
        assert_eq!(pixel_ratio(Some(0.0)), 1.0);
        assert_eq!(pixel_ratio(Some(f64::NAN)), 1.0);
        assert_eq!(pixel_ratio(Some(1.5)), 1.5);
    }
}
