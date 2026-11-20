//! `FormationArea::from_mask`: a formation area from a mask PLANE (C# `FormationArea.FromMask`),
//! now that the pixel planes exist.

use crate::formations::FormationArea;
use crate::geometry::RectangleF;
use crate::planes::DrawingPlane;

impl FormationArea {
    /// The pixels of a mask plane at or above `threshold` (None: 0.5), the plane stretched over
    /// `destination` in calibrated units (C# `FromMask(DrawingPlane mask, RectangleF destination,
    /// threshold = 0.5)`). The plane's values are read once; later changes to it are not seen.
    /// Distances are exact Euclidean distances between pixel centres. The error says what is
    /// wrong with the destination or the threshold.
    pub fn from_mask(mask: &DrawingPlane, destination: RectangleF, threshold: Option<f32>) -> Result<FormationArea, String> {
        FormationArea::from_mask_data(mask.width() as usize, mask.height() as usize, mask.data(), destination, threshold)
    }
}

#[cfg(test)]
mod tests {
    use crate::formations::FormationArea;
    use crate::geometry::RectangleF;
    use crate::planes::DrawingPlane;
    use crate::Rectangle;

    #[test]
    fn a_mask_plane_gives_the_area_its_values_give() {
        let mut mask = DrawingPlane::new(8, 6).unwrap();
        mask.fill_area(Rectangle::new(2, 1, 3, 4), 1.0);
        let destination = RectangleF::new(10.0, 20.0, 80.0, 60.0);
        let from_plane = FormationArea::from_mask(&mask, destination, None).unwrap();
        let from_data = FormationArea::from_mask_data(8, 6, mask.data(), destination, None).unwrap();
        for (x, y) in [(35.0f32, 45.0f32), (15.0, 25.0), (55.0, 55.0), (80.0, 70.0)] {
            assert_eq!(from_plane.contains(x, y), from_data.contains(x, y));
            assert_eq!(from_plane.distance_to_edge(x, y).to_bits(), from_data.distance_to_edge(x, y).to_bits());
        }
        assert!(from_plane.contains(35.0, 45.0));
        assert!(FormationArea::from_mask(&mask, destination, Some(f32::NAN)).is_err());
    }
}
