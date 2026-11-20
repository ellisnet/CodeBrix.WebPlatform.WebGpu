//! `SafeAreaMargins`: the unsafe margins of a frame, as fractions of its size (C#
//! `SafeAreaMargins`).

/// The margins a platform's interface covers, as fractions of the frame's height (top, bottom)
/// and width (left, right) (C# `SafeAreaMargins`).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct SafeAreaMargins {
    top: f32,
    bottom: f32,
    left: f32,
    right: f32,
}

fn check(value: f32, name: &str) -> Result<(), String> {
    if value.is_nan() || !(0.0..=1.0).contains(&value) {
        return Err(format!("The {name} margin must be a fraction, 0..1."));
    }
    Ok(())
}

impl SafeAreaMargins {
    /// No margins (C# `None`).
    pub const NONE: SafeAreaMargins = SafeAreaMargins { top: 0.0, bottom: 0.0, left: 0.0, right: 0.0 };
    /// The whole frame is unsafe (C# `WholeFrame`: half from each side).
    pub const WHOLE_FRAME: SafeAreaMargins = SafeAreaMargins { top: 0.5, bottom: 0.5, left: 0.5, right: 0.5 };

    /// Margins (C#'s constructor). The error says which margin is out of range, or which two
    /// exceed the frame together.
    pub fn new(top: f32, bottom: f32, left: f32, right: f32) -> Result<SafeAreaMargins, String> {
        check(top, "top")?;
        check(bottom, "bottom")?;
        check(left, "left")?;
        check(right, "right")?;
        if top + bottom > 1.0 {
            return Err("The top and bottom margins together exceed the frame.".to_string());
        }
        if left + right > 1.0 {
            return Err("The left and right margins together exceed the frame.".to_string());
        }
        Ok(SafeAreaMargins { top, bottom, left, right })
    }

    /// The same margin on every side (C# `Uniform`).
    pub fn uniform(fraction: f32) -> Result<SafeAreaMargins, String> {
        SafeAreaMargins::new(fraction, fraction, fraction, fraction)
    }

    /// The larger of two margins on each side, kept within the frame (C# `Max`).
    pub fn max(a: SafeAreaMargins, b: SafeAreaMargins) -> SafeAreaMargins {
        let top = a.top.max(b.top);
        let bottom = a.bottom.max(b.bottom).min(1.0 - top);
        let left = a.left.max(b.left);
        let right = a.right.max(b.right).min(1.0 - left);
        SafeAreaMargins { top, bottom, left, right }
    }

    /// The top margin, a fraction of the height (C# `Top`).
    pub fn top(&self) -> f32 {
        self.top
    }

    /// The bottom margin (C# `Bottom`).
    pub fn bottom(&self) -> f32 {
        self.bottom
    }

    /// The left margin, a fraction of the width (C# `Left`).
    pub fn left(&self) -> f32 {
        self.left
    }

    /// The right margin (C# `Right`).
    pub fn right(&self) -> f32 {
        self.right
    }

    /// True when every margin is 0 (C# `IsEmpty`).
    pub fn is_empty(&self) -> bool {
        self.top == 0.0 && self.bottom == 0.0 && self.left == 0.0 && self.right == 0.0
    }
}
