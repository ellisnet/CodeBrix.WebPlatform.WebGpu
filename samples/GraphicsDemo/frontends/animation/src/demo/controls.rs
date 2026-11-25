//! The controls and read-outs a scene may put on the page (the feature scenes of the second GO:
//! a blend mode, a wipe's progress, the sketch grid's settings ...). A scene lists its controls
//! (`DemoScene::controls`), the page draws them under the canvas while the scene is shown - each
//! with the id `<scene>-<key>` - keeps their values in a [`Settings`] per scene and hands every
//! change to the scene (`DemoScene::apply_settings`). Read-outs (`DemoScene::readouts`) are
//! collected after each frame and shown as "label: value", the value in the element
//! `<scene>-<key>`.

/// One control of a scene; its value is a number in [`Settings`] under `key` (toggles 0 / 1).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Control {
    /// On or off.
    Toggle { key: &'static str, label: &'static str, default: bool },
    /// A number between `min` and `max`.
    Slider { key: &'static str, label: &'static str, min: f32, max: f32, step: f32, default: f32 },
    /// One of a few named numbers.
    Choice { key: &'static str, label: &'static str, options: &'static [(&'static str, f32)], default: f32 },
    /// A push button: its value counts the presses (0 at first), so a scene sees a press as a
    /// change of the value.
    Button { key: &'static str, label: &'static str },
}

impl Control {
    /// The control's key (the id suffix and the name of its value).
    pub fn key(&self) -> &'static str {
        match self {
            Control::Toggle { key, .. } | Control::Slider { key, .. } | Control::Choice { key, .. } | Control::Button { key, .. } => key,
        }
    }

    fn default_value(&self) -> f32 {
        match self {
            Control::Toggle { default, .. } => f32::from(u8::from(*default)),
            Control::Slider { default, .. } | Control::Choice { default, .. } => *default,
            Control::Button { .. } => 0.0,
        }
    }
}

/// The values of a scene's controls, by key.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Settings {
    values: Vec<(&'static str, f32)>,
}

impl Settings {
    /// Every control at its default.
    pub fn from_controls(controls: &[Control]) -> Settings {
        Settings { values: controls.iter().map(|c| (c.key(), c.default_value())).collect() }
    }

    /// The value of `key` (0 for a key no control has).
    pub fn get(&self, key: &str) -> f32 {
        self.values.iter().find(|(k, _)| *k == key).map_or(0.0, |(_, v)| *v)
    }

    /// Whether the toggle `key` is on.
    pub fn on(&self, key: &str) -> bool {
        self.get(key) >= 0.5
    }

    /// Sets the value of `key` (nothing for a key no control has).
    pub fn set(&mut self, key: &str, value: f32) {
        if let Some(entry) = self.values.iter_mut().find(|(k, _)| *k == key) {
            entry.1 = value;
        }
    }
}

/// One read-out line of a scene: shown as "label: value", the value in the element `<scene>-<key>`.
#[derive(Clone, Debug, PartialEq)]
pub struct Readout {
    /// The id suffix.
    pub key: &'static str,
    /// What it is.
    pub label: &'static str,
    /// Its value now.
    pub value: String,
}

impl Readout {
    /// A read-out.
    pub fn new(key: &'static str, label: &'static str, value: impl ToString) -> Readout {
        Readout { key, label, value: value.to_string() }
    }
}

/// A number as the page shows it: at most two decimals, no trailing zeros.
pub fn trim(value: f32) -> String {
    let text = format!("{value:.2}");
    text.trim_end_matches('0').trim_end_matches('.').to_string()
}
