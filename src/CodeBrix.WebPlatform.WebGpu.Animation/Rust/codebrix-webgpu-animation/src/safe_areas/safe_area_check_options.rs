//! `SafeAreaCheckOptions`: how `check_safe_area` checks (C# `SafeAreaCheckOptions`).

/// How [`DrawingSession::check_safe_area`](crate::DrawingSession::check_safe_area) checks (C#
/// `SafeAreaCheckOptions`).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct SafeAreaCheckOptions {
    /// The video's duration, seconds, for guides with a final-seconds rule (None: they always
    /// apply) (C# `Duration`).
    pub duration: Option<f64>,
    /// True to report elements whose bounds are not known (C# `ReportUncheckedElements`).
    pub report_unchecked_elements: bool,
    /// True to check every element, flagged or not (C# `CheckAllElements`).
    pub check_all_elements: bool,
}
