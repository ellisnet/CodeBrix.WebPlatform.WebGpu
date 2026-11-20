//! `FormationPose`: where a formation member is and how it is turned and scaled (C#
//! `FormationPose`).

use crate::geometry::PointF;
use crate::models::DrawingTransform;

/// A member's pose (C# `FormationPose`): position, rotation (degrees) and scale.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FormationPose {
    /// The x position.
    pub x: f32,
    /// The y position.
    pub y: f32,
    /// Degrees, clockwise on screen.
    pub rotation: f32,
    /// The horizontal scale.
    pub scale_x: f32,
    /// The vertical scale.
    pub scale_y: f32,
}

impl FormationPose {
    /// A pose (C# `FormationPose(x, y, rotation, scaleX, scaleY)`).
    pub fn new(x: f32, y: f32, rotation: f32, scale_x: f32, scale_y: f32) -> FormationPose {
        FormationPose { x, y, rotation, scale_x, scale_y }
    }

    /// A pose at a point, unturned, at scale 1 (C# `FormationPose(x, y)` with its defaults).
    pub fn at(x: f32, y: f32) -> FormationPose {
        FormationPose::new(x, y, 0.0, 1.0, 1.0)
    }

    /// The position (C# `Position`).
    pub fn position(&self) -> PointF {
        PointF::new(self.x, self.y)
    }

    /// The pose as a transform (translation = position, anchor at the origin) - for an element
    /// drawn centred on (0, 0) (C# `ToTransform`).
    pub fn to_transform(&self) -> DrawingTransform {
        DrawingTransform::IDENTITY.with_translation(PointF::new(self.x, self.y)).with_rotation(self.rotation).with_scale_xy(self.scale_x, self.scale_y)
    }

    pub(crate) fn check(&self, name: &str) -> Result<(), String> {
        if !self.x.is_finite() || !self.y.is_finite() || !self.rotation.is_finite() || !self.scale_x.is_finite() || !self.scale_y.is_finite() {
            return Err(format!("Every value of the {name} pose must be finite."));
        }
        Ok(())
    }
}
