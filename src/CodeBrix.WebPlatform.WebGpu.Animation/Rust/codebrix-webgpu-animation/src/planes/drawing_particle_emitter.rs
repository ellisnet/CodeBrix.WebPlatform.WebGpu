//! `DrawingParticleEmitter`: the settings of one particle emitter (C# `DrawingParticleEmitter`).

use crate::Color;

/// The settings of one particle emitter in a
/// [`DrawingParticleSystem`](crate::planes::DrawingParticleSystem) (C# `DrawingParticleEmitter`):
/// where and when particles are born, how they move and how they look over their life. Positions
/// and sizes are in the units the particles are drawn in; times are seconds; angles are DEGREES
/// clockwise on screen from +x with y pointing DOWN (0 moves right, 90 down, -90 up).
///
/// Every random choice is drawn per particle from
/// [`DrawingRandom::for_key`](crate::planes::DrawingRandom::for_key), so changing one emitter or
/// querying another time never changes any other particle. C#'s validating property setters are
/// `set_*` methods here; those that refuse a value return an error saying what is allowed.
#[derive(Clone, Debug, PartialEq)]
pub struct DrawingParticleEmitter {
    x: f32,
    y: f32,
    spawn_radius: f32,
    rate: f32,
    burst: u32,
    start_time: f64,
    end_time: f64,
    lifetime: f64,
    lifetime_variation: f64,
    direction: f32,
    spread: f32,
    speed: f32,
    speed_variation: f32,
    gravity_x: f32,
    gravity_y: f32,
    drift_x: f32,
    drift_y: f32,
    start_size: f32,
    end_size: f32,
    size_variation: f32,
    start_opacity: f32,
    end_opacity: f32,
    start_color: Color,
    end_color: Color,
}

impl Default for DrawingParticleEmitter {
    fn default() -> DrawingParticleEmitter {
        DrawingParticleEmitter {
            x: 0.0,
            y: 0.0,
            spawn_radius: 0.0,
            rate: 0.0,
            burst: 0,
            start_time: 0.0,
            end_time: f64::INFINITY,
            lifetime: 1.0,
            lifetime_variation: 0.0,
            direction: 0.0,
            spread: 0.0,
            speed: 0.0,
            speed_variation: 0.0,
            gravity_x: 0.0,
            gravity_y: 0.0,
            drift_x: 0.0,
            drift_y: 0.0,
            start_size: 4.0,
            end_size: 4.0,
            size_variation: 0.0,
            start_opacity: 1.0,
            end_opacity: 0.0,
            start_color: Color::WHITE,
            end_color: Color::WHITE,
        }
    }
}

fn require_non_negative(value: f32, name: &str) -> Result<f32, String> {
    if !(value >= 0.0) || value.is_infinite() {
        return Err(format!("The emitter's {name} must be a finite number, 0 or more."));
    }
    Ok(value)
}

impl DrawingParticleEmitter {
    /// An emitter with C#'s defaults: at (0, 0), no burst, no rate, lifetime 1 s, size 4, opacity
    /// 1 to 0, white (C# `new DrawingParticleEmitter()`).
    pub fn new() -> DrawingParticleEmitter {
        DrawingParticleEmitter::default()
    }

    /// The emitter's x position (C# `X`).
    pub fn x(&self) -> f32 {
        self.x
    }

    /// Sets the emitter's x position.
    pub fn set_x(&mut self, value: f32) {
        self.x = value;
    }

    /// The emitter's y position (C# `Y`).
    pub fn y(&self) -> f32 {
        self.y
    }

    /// Sets the emitter's y position.
    pub fn set_y(&mut self, value: f32) {
        self.y = value;
    }

    /// Particles start at a uniformly random point within a disc of this radius around the
    /// emitter; 0 (the default) starts them all at the emitter (C# `SpawnRadius`).
    pub fn spawn_radius(&self) -> f32 {
        self.spawn_radius
    }

    /// Sets the spawn radius (finite, 0 or more).
    pub fn set_spawn_radius(&mut self, value: f32) -> Result<(), String> {
        self.spawn_radius = require_non_negative(value, "spawn radius")?;
        Ok(())
    }

    /// Continuous emission, particles per second, from the start time until the end time:
    /// particle n is born at `start_time + n / rate`; 0 (the default) emits only the burst (C#
    /// `Rate`).
    pub fn rate(&self) -> f32 {
        self.rate
    }

    /// Sets the rate (finite, 0 or more).
    pub fn set_rate(&mut self, value: f32) -> Result<(), String> {
        self.rate = require_non_negative(value, "rate")?;
        Ok(())
    }

    /// The particles born together at the start time (0 by default) (C# `Burst`; C# refuses a
    /// negative count, which `u32` cannot hold).
    pub fn burst(&self) -> u32 {
        self.burst
    }

    /// Sets the burst count.
    pub fn set_burst(&mut self, value: u32) {
        self.burst = value;
    }

    /// When emission starts, seconds (0 by default) (C# `StartTime`).
    pub fn start_time(&self) -> f64 {
        self.start_time
    }

    /// Sets the start time (finite).
    pub fn set_start_time(&mut self, value: f64) -> Result<(), String> {
        if !value.is_finite() {
            return Err("The emitter's start time must be a finite number of seconds.".to_string());
        }
        self.start_time = value;
        Ok(())
    }

    /// When continuous emission stops, seconds, exclusive (born particles live on); infinity,
    /// the default, never stops (C# `EndTime`).
    pub fn end_time(&self) -> f64 {
        self.end_time
    }

    /// Sets the end time (a number; not negative infinity).
    pub fn set_end_time(&mut self, value: f64) -> Result<(), String> {
        if value.is_nan() || value == f64::NEG_INFINITY {
            return Err("The emitter's end time must be a number of seconds or positive infinity.".to_string());
        }
        self.end_time = value;
        Ok(())
    }

    /// The mean lifetime, seconds (1 by default) (C# `Lifetime`).
    pub fn lifetime(&self) -> f64 {
        self.lifetime
    }

    /// Sets the mean lifetime (finite, above 0).
    pub fn set_lifetime(&mut self, value: f64) -> Result<(), String> {
        if !(value > 0.0) || value.is_infinite() {
            return Err("The emitter's lifetime must be a finite number of seconds above 0.".to_string());
        }
        self.lifetime = value;
        Ok(())
    }

    /// Each lifetime is `lifetime + uniform(-variation, +variation)`, never under 1% of the
    /// lifetime (0 by default) (C# `LifetimeVariation`).
    pub fn lifetime_variation(&self) -> f64 {
        self.lifetime_variation
    }

    /// Sets the lifetime variation (finite, 0 or more).
    pub fn set_lifetime_variation(&mut self, value: f64) -> Result<(), String> {
        if !(value >= 0.0) || value.is_infinite() {
            return Err("The emitter's lifetime variation must be a finite number of seconds, 0 or more.".to_string());
        }
        self.lifetime_variation = value;
        Ok(())
    }

    /// The mean launch direction, degrees clockwise on screen from +x (C# `Direction`).
    pub fn direction(&self) -> f32 {
        self.direction
    }

    /// Sets the launch direction.
    pub fn set_direction(&mut self, value: f32) {
        self.direction = value;
    }

    /// The full width of the launch cone, degrees: each particle launches at `direction +
    /// uniform(-spread / 2, +spread / 2)`; 360 launches every way (C# `Spread`).
    pub fn spread(&self) -> f32 {
        self.spread
    }

    /// Sets the spread.
    pub fn set_spread(&mut self, value: f32) {
        self.spread = value;
    }

    /// The mean launch speed, units per second (C# `Speed`).
    pub fn speed(&self) -> f32 {
        self.speed
    }

    /// Sets the launch speed.
    pub fn set_speed(&mut self, value: f32) {
        self.speed = value;
    }

    /// Each speed is `speed + uniform(-variation, +variation)` (0 by default) (C# `SpeedVariation`).
    pub fn speed_variation(&self) -> f32 {
        self.speed_variation
    }

    /// Sets the speed variation (finite, 0 or more).
    pub fn set_speed_variation(&mut self, value: f32) -> Result<(), String> {
        self.speed_variation = require_non_negative(value, "speed variation")?;
        Ok(())
    }

    /// Horizontal acceleration, units per second squared (C# `GravityX`).
    pub fn gravity_x(&self) -> f32 {
        self.gravity_x
    }

    /// Sets the horizontal acceleration.
    pub fn set_gravity_x(&mut self, value: f32) {
        self.gravity_x = value;
    }

    /// Vertical acceleration, units per second squared; positive pulls DOWN (C# `GravityY`).
    pub fn gravity_y(&self) -> f32 {
        self.gravity_y
    }

    /// Sets the vertical acceleration.
    pub fn set_gravity_y(&mut self, value: f32) {
        self.gravity_y = value;
    }

    /// A constant horizontal velocity added to every particle (C# `DriftX`).
    pub fn drift_x(&self) -> f32 {
        self.drift_x
    }

    /// Sets the horizontal drift.
    pub fn set_drift_x(&mut self, value: f32) {
        self.drift_x = value;
    }

    /// A constant vertical velocity added to every particle (C# `DriftY`).
    pub fn drift_y(&self) -> f32 {
        self.drift_y
    }

    /// Sets the vertical drift.
    pub fn set_drift_y(&mut self, value: f32) {
        self.drift_y = value;
    }

    /// The size (diameter) at birth (4 by default) (C# `StartSize`).
    pub fn start_size(&self) -> f32 {
        self.start_size
    }

    /// Sets the size at birth.
    pub fn set_start_size(&mut self, value: f32) {
        self.start_size = value;
    }

    /// The size at the end of life (4 by default); sizes blend linearly over life (C# `EndSize`).
    pub fn end_size(&self) -> f32 {
        self.end_size
    }

    /// Sets the size at the end of life.
    pub fn set_end_size(&mut self, value: f32) {
        self.end_size = value;
    }

    /// Each particle's sizes are multiplied by `1 + uniform(-variation, +variation)` (0 by
    /// default; 0.25 = up to 25% larger or smaller) (C# `SizeVariation`).
    pub fn size_variation(&self) -> f32 {
        self.size_variation
    }

    /// Sets the size variation (finite, 0 or more).
    pub fn set_size_variation(&mut self, value: f32) -> Result<(), String> {
        self.size_variation = require_non_negative(value, "size variation")?;
        Ok(())
    }

    /// The opacity at birth, 0..1 (1 by default) (C# `StartOpacity`).
    pub fn start_opacity(&self) -> f32 {
        self.start_opacity
    }

    /// Sets the opacity at birth.
    pub fn set_start_opacity(&mut self, value: f32) {
        self.start_opacity = value;
    }

    /// The opacity at the end of life (0 by default); opacity blends linearly (C# `EndOpacity`).
    pub fn end_opacity(&self) -> f32 {
        self.end_opacity
    }

    /// Sets the opacity at the end of life.
    pub fn set_end_opacity(&mut self, value: f32) {
        self.end_opacity = value;
    }

    /// The colour at birth (white by default) (C# `StartColor`).
    pub fn start_color(&self) -> Color {
        self.start_color
    }

    /// Sets the colour at birth.
    pub fn set_start_color(&mut self, value: Color) {
        self.start_color = value;
    }

    /// The colour at the end of life (white by default); colours blend linearly on their 8-bit
    /// channels, rounded half up (C# `EndColor`).
    pub fn end_color(&self) -> Color {
        self.end_color
    }

    /// Sets the colour at the end of life.
    pub fn set_end_color(&mut self, value: Color) {
        self.end_color = value;
    }
}
