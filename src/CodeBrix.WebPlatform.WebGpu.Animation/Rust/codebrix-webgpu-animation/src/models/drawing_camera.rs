//! `DrawingCamera`: a view over the whole drawing - pan, zoom and roll - that each layer follows by
//! its parallax (C# `DrawingCamera`).

use kurbo::Affine;

use crate::geometry::PointF;

/// A camera over a session's drawing (C# `DrawingCamera`): PAN moves the view (scene units),
/// ZOOM magnifies about the PIVOT (default the centre of the calibrated drawing), ROTATION rolls the
/// view, degrees (the scene turns the other way on screen). Each layer follows it by its
/// [`parallax`](crate::models::DrawingLayer::parallax): 1 moves with the scene, 0 is fixed to the
/// frame (a HUD), in between moves less (pan and roll times the parallax, zoom to the power of it).
/// Set it on the session ([`DrawingSession::set_camera`](crate::DrawingSession::set_camera)) and
/// bind its properties to tracks; it is applied while drawing.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DrawingCamera {
    pan: PointF,
    zoom: f32,
    rotation: f32,
    pivot: Option<PointF>,
}

impl Default for DrawingCamera {
    fn default() -> DrawingCamera {
        DrawingCamera { pan: PointF::new(0.0, 0.0), zoom: 1.0, rotation: 0.0, pivot: None }
    }
}

impl DrawingCamera {
    /// A camera that changes nothing (C# `new DrawingCamera()`).
    pub fn new() -> DrawingCamera {
        DrawingCamera::default()
    }

    /// The pan, scene units.
    pub fn pan(&self) -> PointF {
        self.pan
    }

    /// Sets the pan. The error says what to do with a non-finite value.
    pub fn set_pan(&mut self, value: PointF) -> Result<(), String> {
        if !(value.x.is_finite() && value.y.is_finite()) {
            return Err("A camera's pan must be finite numbers.".to_string());
        }
        self.pan = value;
        Ok(())
    }

    /// The zoom (1 = none).
    pub fn zoom(&self) -> f32 {
        self.zoom
    }

    /// Sets the zoom. The error says what to do with a value that is not positive.
    pub fn set_zoom(&mut self, value: f32) -> Result<(), String> {
        if !(value.is_finite() && value > 0.0) {
            return Err("A camera's zoom must be a positive number (1 = no zoom).".to_string());
        }
        self.zoom = value;
        Ok(())
    }

    /// The roll, degrees.
    pub fn rotation(&self) -> f32 {
        self.rotation
    }

    /// Sets the roll.
    pub fn set_rotation(&mut self, value: f32) -> Result<(), String> {
        if !value.is_finite() {
            return Err("A camera's rotation must be a finite number of degrees.".to_string());
        }
        self.rotation = value;
        Ok(())
    }

    /// The point zoom and roll turn about; None (the default) for the drawing's centre.
    pub fn pivot(&self) -> Option<PointF> {
        self.pivot
    }

    /// Sets the pivot.
    pub fn set_pivot(&mut self, value: Option<PointF>) -> Result<(), String> {
        if let Some(p) = value {
            if !(p.x.is_finite() && p.y.is_finite()) {
                return Err("A camera's pivot must be finite numbers.".to_string());
            }
        }
        self.pivot = value;
        Ok(())
    }

    /// True when it changes nothing.
    pub fn is_identity(&self) -> bool {
        self.pan.x == 0.0 && self.pan.y == 0.0 && self.zoom == 1.0 && self.rotation == 0.0
    }

    /// The camera as a layer with this parallax feels it: (pivot, pan, zoom, roll), or None when
    /// it changes nothing there (C#'s TryGetEffective).
    pub(crate) fn effective(&self, calibration_size: (u32, u32), parallax: f32) -> Option<(PointF, PointF, f32, f32)> {
        let pivot = self.pivot.unwrap_or(PointF::new(calibration_size.0 as f32 / 2.0, calibration_size.1 as f32 / 2.0));
        let (pan, zoom, roll) = if parallax == 1.0 {
            (self.pan, self.zoom, self.rotation)
        } else {
            let zoom = if parallax == 0.0 { 1.0 } else { (self.zoom as f64).powf(parallax as f64) as f32 };
            (PointF::new(self.pan.x * parallax, self.pan.y * parallax), zoom, self.rotation * parallax)
        };
        let changes = pan.x != 0.0 || pan.y != 0.0 || zoom != 1.0 || roll != 0.0;
        changes.then_some((pivot, pan, zoom, roll))
    }

    /// The scene-to-view matrix for a layer of this parallax (C# `GetMatrix(calibrationSize,
    /// parallax = 1)`): translate(-pivot - pan), scale(zoom), rotate(-roll), translate(pivot).
    pub fn get_matrix(&self, calibration_size: (u32, u32), parallax: f32) -> Affine {
        match self.effective(calibration_size, parallax) {
            None => Affine::IDENTITY,
            Some((pivot, pan, zoom, roll)) => Affine::translate((pivot.x as f64, pivot.y as f64)) * Affine::rotate(-(roll as f64).to_radians()) * Affine::scale(zoom as f64) * Affine::translate(((-pivot.x - pan.x) as f64, (-pivot.y - pan.y) as f64)),
        }
    }

    /// Where a scene point is seen (C# `ToView`).
    pub fn to_view(&self, scene_point: PointF, calibration_size: (u32, u32), parallax: f32) -> PointF {
        let p = self.get_matrix(calibration_size, parallax) * scene_point.to_kurbo();
        PointF::new(p.x as f32, p.y as f32)
    }

    /// The scene point seen at a view point (C# `ToScene`).
    pub fn to_scene(&self, view_point: PointF, calibration_size: (u32, u32), parallax: f32) -> PointF {
        let p = self.get_matrix(calibration_size, parallax).inverse() * view_point.to_kurbo();
        PointF::new(p.x as f32, p.y as f32)
    }
}
