//! "particles" (AnimationDemoApp's ParticleScene): three particle systems as PURE FUNCTIONS OF THE
//! TIME - a fountain (soft dots), six firework bursts (softness 0.6) and drifting snow (solid
//! dots) - each frame's particles queried at the frame's time and drawn by the crate as ONE
//! instanced GPU draw of soft discs per system (session.draw_particles), over a night sky; the
//! live counts below.

use codebrix_webgpu_animation::fills::LinearGradientFill;
use codebrix_webgpu_animation::planes::{DrawingParticleEmitter, DrawingParticleSystem};
use codebrix_webgpu_animation::rendering::VectorScene;
use codebrix_webgpu_animation::shapes::RectangleShape;
use codebrix_webgpu_animation::text::{FontRegistry, TextAlign};
use codebrix_webgpu_animation::{Color, DrawingSession, PointF};

use crate::demo::{add_header, create_session, label, DemoScene, BACKDROP, CONTENT_TOP, DIM, FRAME_HEIGHT, FRAME_WIDTH, INK};

const TITLE: &str = "Particles";
const DESCRIPTION: &str = "DrawingParticleSystem: particles as a pure function of time; session.draw_particles draws them as soft dots (one instanced GPU draw)";

pub struct ParticleScene {
    session: DrawingSession,
    fountain: DrawingParticleSystem,
    fireworks: DrawingParticleSystem,
    snow: DrawingParticleSystem,
    fonts: FontRegistry,
}

impl ParticleScene {
    pub fn new(fonts: &FontRegistry) -> ParticleScene {
        let mut fountain = DrawingParticleSystem::new(1);
        {
            let e = fountain.add_emitter(DrawingParticleEmitter::new());
            e.set_x(250.0);
            e.set_y(640.0);
            e.set_rate(90.0).unwrap();
            e.set_lifetime(2.2).unwrap();
            e.set_lifetime_variation(0.6).unwrap();
            e.set_direction(-90.0);
            e.set_spread(26.0);
            e.set_speed(330.0);
            e.set_speed_variation(60.0).unwrap();
            e.set_gravity_y(260.0);
            e.set_start_size(12.0);
            e.set_end_size(4.0);
            e.set_size_variation(0.3).unwrap();
            e.set_start_opacity(1.0);
            e.set_end_opacity(0.0);
            e.set_start_color(Color::from_rgb(140, 220, 255));
            e.set_end_color(Color::from_rgb(40, 90, 255));
        }
        let colours = [Color::from_rgb(255, 200, 90), Color::from_rgb(255, 90, 140), Color::from_rgb(150, 255, 160), Color::from_rgb(200, 160, 255)];
        let mut fireworks = DrawingParticleSystem::new(2);
        for i in 0..6u32 {
            let e = fireworks.add_emitter(DrawingParticleEmitter::new());
            e.set_x(560.0 + ((i * 137) % 420) as f32);
            e.set_y(200.0 + ((i * 71) % 160) as f32);
            e.set_burst(110);
            e.set_start_time(0.3 + i as f64 * 1.25).unwrap();
            e.set_lifetime(1.7).unwrap();
            e.set_lifetime_variation(0.5).unwrap();
            e.set_spread(360.0);
            e.set_speed(170.0);
            e.set_speed_variation(70.0).unwrap();
            e.set_gravity_y(70.0);
            e.set_start_size(9.0);
            e.set_end_size(3.0);
            e.set_start_opacity(1.0);
            e.set_end_opacity(0.0);
            e.set_start_color(Color::WHITE);
            e.set_end_color(colours[i as usize % colours.len()]);
        }
        let mut snow = DrawingParticleSystem::new(3);
        {
            let e = snow.add_emitter(DrawingParticleEmitter::new());
            e.set_x(1130.0);
            e.set_y(40.0);
            e.set_spawn_radius(110.0).unwrap();
            e.set_rate(35.0).unwrap();
            e.set_start_time(-6.0).unwrap();
            e.set_lifetime(7.0).unwrap();
            e.set_direction(90.0);
            e.set_spread(23.0);
            e.set_speed(70.0);
            e.set_speed_variation(20.0).unwrap();
            e.set_drift_x(-12.0);
            e.set_start_size(7.0);
            e.set_end_size(5.0);
            e.set_size_variation(0.4).unwrap();
            e.set_start_opacity(0.95);
            e.set_end_opacity(0.6);
            e.set_start_color(Color::from_rgb(240, 245, 255));
            e.set_end_color(Color::from_rgb(200, 215, 240));
        }
        // The header goes on last, over an opaque band: particles fly up past the content's top.
        let mut session = create_session(FRAME_WIDTH, FRAME_HEIGHT, BACKDROP, fonts);
        let sky = LinearGradientFill::from_colors(PointF::new(0.0, CONTENT_TOP), PointF::new(0.0, FRAME_HEIGHT as f32), Color::from_rgb(8, 10, 26), Color::from_rgb(30, 26, 52)).unwrap();
        session.add_layer("Sky", BACKDROP).add_shape(RectangleShape::with_fill(0.0, CONTENT_TOP, FRAME_WIDTH as f32, FRAME_HEIGHT as f32 - CONTENT_TOP, sky.into(), 0.0).unwrap());
        session.add_layer("Particles", INK);
        session.add_layer("Notes", DIM);
        add_header(&mut session, TITLE, DESCRIPTION, true);
        ParticleScene { session, fountain, fireworks, snow, fonts: fonts.clone() }
    }
}

impl DemoScene for ParticleScene {
    fn name(&self) -> &'static str {
        "particles"
    }

    fn title(&self) -> &'static str {
        TITLE
    }

    fn description(&self) -> &'static str {
        DESCRIPTION
    }

    fn render(&mut self, seconds: f64, scene: &mut VectorScene, width: u32, height: u32) {
        self.session.set_time(seconds);
        self.session.get_layer_mut("Particles").unwrap().clear();
        self.session.set_active_layer("Particles");
        self.session.draw_particles(&self.fountain, seconds, None).unwrap();
        self.session.draw_particles(&self.fireworks, seconds, Some(0.6)).unwrap();
        self.session.draw_particles(&self.snow, seconds, Some(0.0)).unwrap();
        let count = |s: &DrawingParticleSystem| s.get_particles(seconds).map(|p| p.len()).unwrap_or(0);
        let (a, b, c) = (count(&self.fountain), count(&self.fireworks), count(&self.snow));
        let fonts = self.fonts.clone();
        let notes = self.session.get_layer_mut("Notes").unwrap();
        notes.clear();
        label(notes, &fonts, &format!("fountain: {a} live, softness 1"), 250.0, 690.0, 14.0, DIM, TextAlign::Center);
        label(notes, &fonts, &format!("fireworks: {b} live, bursts, softness 0.6"), 760.0, 690.0, 14.0, DIM, TextAlign::Center);
        label(notes, &fonts, &format!("snow: {c} live, softness 0"), 1130.0, 690.0, 14.0, DIM, TextAlign::Center);
        self.session.render_with_background(scene, width, height);
    }
}
