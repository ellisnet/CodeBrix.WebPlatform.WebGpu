//! The CPU half of the sprite batch: the quads collected between `begin` and `end`, their runs of
//! one texture, and the transform stack - everything but the GPU, so that it is tested on the
//! host. The texture is a type parameter (the batch uses its own key; the tests use numbers).

use glam::{Affine2, Vec2};

use crate::common::{Color4, RectangleF};
use crate::drawing2d::SpriteVertex;

/// The texture of a run: one the caller drew, or the context's white texture (a run that began
/// with a shape).
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum RunTexture<K> {
    /// The context's 1 x 1 white texture.
    White,
    /// A texture the caller drew.
    Texture(K),
}

/// A run of quads that share one texture: one indexed draw at `end`.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Run<K> {
    pub(crate) texture: RunTexture<K>,
    pub(crate) first_quad: usize,
    pub(crate) quad_count: usize,
}

/// The quads of one begin/end pair, in drawing order (later quads are drawn over earlier ones).
pub(crate) struct SpriteBuilder<K> {
    vertices: Vec<SpriteVertex>,
    runs: Vec<Run<K>>,
    transforms: Vec<Affine2>,
}

impl<K: Clone + PartialEq> SpriteBuilder<K> {
    /// A builder with room for `quads` quads before it grows.
    pub(crate) fn with_capacity(quads: usize) -> SpriteBuilder<K> {
        SpriteBuilder { vertices: Vec::with_capacity(quads * 4), runs: Vec::new(), transforms: vec![Affine2::IDENTITY] }
    }

    /// Forgets every quad and starts the transform stack at `transform` (C#: what `Begin` resets).
    pub(crate) fn reset(&mut self, transform: Affine2) {
        self.vertices.clear();
        self.runs.clear();
        self.transforms.clear();
        self.transforms.push(transform);
    }

    /// The collected vertices (four per quad).
    pub(crate) fn vertices(&self) -> &[SpriteVertex] {
        &self.vertices
    }

    /// The runs of one texture, in order.
    pub(crate) fn runs(&self) -> &[Run<K>] {
        &self.runs
    }

    /// The number of quads collected (a triangle counts as one).
    pub(crate) fn quad_count(&self) -> usize {
        self.vertices.len() / 4
    }

    /// The current transform.
    pub(crate) fn transform(&self) -> Affine2 {
        self.transforms.last().copied().unwrap_or(Affine2::IDENTITY)
    }

    /// Pushes `transform` combined with the current one: the new one applies FIRST (C#:
    /// `transform * Peek()` in System.Numerics' row-vector order; `current * transform` in glam's).
    pub(crate) fn push_transform(&mut self, transform: Affine2) {
        let combined = self.transform() * transform;
        self.transforms.push(combined);
    }

    /// Pops the last pushed transform; false when only the initial one is left.
    pub(crate) fn pop_transform(&mut self) -> bool {
        if self.transforms.len() <= 1 {
            return false;
        }
        self.transforms.pop();
        true
    }

    /// A region of a texture (`source`, in texels of a `texture_size` texture) into a rectangle.
    pub(crate) fn draw_region(&mut self, texture: K, texture_size: Vec2, destination: RectangleF, source: RectangleF, tint: Color4) {
        let (uv0, uv1) = uvs(source, texture_size);
        self.add_quad(
            Some(texture),
            [Vec2::new(destination.left(), destination.top()), Vec2::new(destination.right(), destination.top()), Vec2::new(destination.right(), destination.bottom()), Vec2::new(destination.left(), destination.bottom())],
            [uv0, Vec2::new(uv1.x, uv0.y), uv1, Vec2::new(uv0.x, uv1.y)],
            tint.to_packed_rgba(),
            1.0,
        );
    }

    /// A region of a texture positioned, rotated (clockwise, y down) and scaled about `origin` (in
    /// texels from the region's top-left corner).
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn draw_rotated(&mut self, texture: K, texture_size: Vec2, position: Vec2, source: RectangleF, tint: Color4, rotation: f32, origin: Vec2, scale: Vec2) {
        let local = sprite_transform(position, rotation, origin, scale);
        let corners = [Vec2::ZERO, Vec2::new(source.width, 0.0), Vec2::new(source.width, source.height), Vec2::new(0.0, source.height)].map(|corner| local.transform_point2(corner));
        let (uv0, uv1) = uvs(source, texture_size);
        self.add_quad(Some(texture), corners, [uv0, Vec2::new(uv1.x, uv0.y), uv1, Vec2::new(uv0.x, uv1.y)], tint.to_packed_rgba(), 1.0);
    }

    /// A flat convex quadrilateral (corners in order around it).
    pub(crate) fn fill_quad(&mut self, a: Vec2, b: Vec2, c: Vec2, d: Vec2, color: Color4) {
        self.add_quad(None, [a, b, c, d], [Vec2::ZERO; 4], color.to_packed_rgba(), 0.0);
    }

    /// A flat rectangle.
    pub(crate) fn fill_rectangle(&mut self, rectangle: RectangleF, color: Color4) {
        self.fill_quad(Vec2::new(rectangle.left(), rectangle.top()), Vec2::new(rectangle.right(), rectangle.top()), Vec2::new(rectangle.right(), rectangle.bottom()), Vec2::new(rectangle.left(), rectangle.bottom()), color);
    }

    /// A rectangle's outline, inside it.
    pub(crate) fn draw_rectangle(&mut self, rectangle: RectangleF, color: Color4, thickness: f32) {
        let t = thickness.min(rectangle.width.min(rectangle.height) / 2.0);
        self.fill_rectangle(RectangleF::new(rectangle.left(), rectangle.top(), rectangle.width, t), color);
        self.fill_rectangle(RectangleF::new(rectangle.left(), rectangle.bottom() - t, rectangle.width, t), color);
        self.fill_rectangle(RectangleF::new(rectangle.left(), rectangle.top() + t, t, rectangle.height - 2.0 * t), color);
        self.fill_rectangle(RectangleF::new(rectangle.right() - t, rectangle.top() + t, t, rectangle.height - 2.0 * t), color);
    }

    /// A line segment `thickness` wide.
    pub(crate) fn draw_line(&mut self, from: Vec2, to: Vec2, color: Color4, thickness: f32) {
        let direction = to - from;
        let length = direction.length();
        if length <= f32::EPSILON {
            return;
        }
        let normal = Vec2::new(-direction.y, direction.x) / length * (thickness / 2.0);
        self.fill_quad(from + normal, to + normal, to - normal, from - normal, color);
    }

    /// A flat triangle (one quad whose last two corners are the same).
    pub(crate) fn fill_triangle(&mut self, a: Vec2, b: Vec2, c: Vec2, color: Color4) {
        self.fill_quad(a, b, c, c, color);
    }

    /// A filled circle of `segments` triangles (0: chosen from the radius).
    pub(crate) fn fill_circle(&mut self, center: Vec2, radius: f32, color: Color4, segments: u32) {
        let count = segments_for(radius, segments);
        let mut previous = center + Vec2::new(radius, 0.0);
        for index in 1..=count {
            let angle = index as f32 * std::f32::consts::TAU / count as f32;
            let next = center + Vec2::new(angle.cos() * radius, angle.sin() * radius);
            self.fill_triangle(center, previous, next, color);
            previous = next;
        }
    }

    /// A circle's outline, centred on the radius.
    pub(crate) fn draw_circle(&mut self, center: Vec2, radius: f32, color: Color4, thickness: f32, segments: u32) {
        let count = segments_for(radius, segments);
        let inner = (radius - thickness / 2.0).max(0.0);
        let outer = radius + thickness / 2.0;
        for index in 0..count {
            let a0 = index as f32 * std::f32::consts::TAU / count as f32;
            let a1 = (index + 1) as f32 * std::f32::consts::TAU / count as f32;
            let d0 = Vec2::new(a0.cos(), a0.sin());
            let d1 = Vec2::new(a1.cos(), a1.sin());
            self.fill_quad(center + d0 * outer, center + d1 * outer, center + d1 * inner, center + d0 * inner, color);
        }
    }

    /// Adds one quad. A texture starts a new run when it differs from the last run's; a shape
    /// (`texture` None) joins the last run whatever its texture, because its vertices say "not
    /// textured" and the run's texture does not matter to it - so shapes NEVER break a run (C#'s
    /// rule). A shape with no run before it starts a run of the white texture.
    fn add_quad(&mut self, texture: Option<K>, corners: [Vec2; 4], uvs: [Vec2; 4], color: u32, textured: f32) {
        let run_texture = match texture {
            Some(texture) => RunTexture::Texture(texture),
            None => self.runs.last().map(|run| run.texture.clone()).unwrap_or(RunTexture::White),
        };
        let quad = self.quad_count();
        match self.runs.last_mut() {
            Some(last) if last.texture == run_texture => last.quad_count += 1,
            _ => self.runs.push(Run { texture: run_texture, first_quad: quad, quad_count: 1 }),
        }
        let m = self.transform();
        for (corner, uv) in corners.iter().zip(uvs.iter()) {
            let p = m.transform_point2(*corner);
            self.vertices.push(SpriteVertex { position: [p.x, p.y], tex_coord: [uv.x, uv.y], color, textured });
        }
    }
}

/// The transform of a positioned sprite (C#: `CreateTranslation(-origin) * CreateScale(scale) *
/// CreateRotation(rotation) * CreateTranslation(position)`, applied left to right; in glam the
/// same product reads right to left).
pub(crate) fn sprite_transform(position: Vec2, rotation: f32, origin: Vec2, scale: Vec2) -> Affine2 {
    Affine2::from_translation(position) * Affine2::from_angle(rotation) * Affine2::from_scale(scale) * Affine2::from_translation(-origin)
}

/// The texture coordinates of a region's top-left and bottom-right corners.
fn uvs(source: RectangleF, texture_size: Vec2) -> (Vec2, Vec2) {
    let size = texture_size.max(Vec2::ONE);
    (Vec2::new(source.left() / size.x, source.top() / size.y), Vec2::new(source.right() / size.x, source.bottom() / size.y))
}

/// C#'s segment rule: the count given when above 2, else 0.75 per pixel of radius, from 12 to 96.
pub(crate) fn segments_for(radius: f32, segments: u32) -> u32 {
    if segments > 2 {
        segments
    } else {
        ((radius * 0.75) as i32).clamp(12, 96) as u32
    }
}

#[cfg(test)]
mod tests {
    use glam::{Affine2, Vec2};

    use super::{segments_for, sprite_transform, Run, RunTexture, SpriteBuilder};
    use crate::common::{Color4, RectangleF};

    fn near(a: Vec2, b: Vec2) -> bool {
        (a - b).length() < 1e-4
    }

    fn position(builder: &SpriteBuilder<u32>, vertex: usize) -> Vec2 {
        Vec2::from(builder.vertices()[vertex].position)
    }

    #[test]
    fn a_region_maps_its_texels_to_texture_coordinates() {
        let mut batch = SpriteBuilder::<u32>::with_capacity(4);
        batch.draw_region(1, Vec2::new(128.0, 64.0), RectangleF::new(10.0, 20.0, 30.0, 40.0), RectangleF::new(32.0, 16.0, 32.0, 16.0), Color4::WHITE);
        let v = batch.vertices();
        assert_eq!(v.len(), 4);
        assert_eq!(v[0].tex_coord, [0.25, 0.25]);
        assert_eq!(v[2].tex_coord, [0.5, 0.5]);
        assert_eq!(v[0].position, [10.0, 20.0]);
        assert_eq!(v[2].position, [40.0, 60.0]);
        assert_eq!(v[0].textured, 1.0);
        assert_eq!(v[0].color, 0xFFFF_FFFF);
    }

    #[test]
    fn runs_break_only_on_a_new_texture_and_shapes_never_break_one() {
        let mut batch = SpriteBuilder::<u32>::with_capacity(4);
        let size = Vec2::splat(8.0);
        let rect = RectangleF::new(0.0, 0.0, 8.0, 8.0);
        batch.fill_rectangle(rect, Color4::RED);
        batch.draw_region(7, size, rect, rect, Color4::WHITE);
        batch.fill_triangle(Vec2::ZERO, Vec2::X, Vec2::Y, Color4::BLUE);
        batch.draw_region(7, size, rect, rect, Color4::WHITE);
        batch.draw_region(9, size, rect, rect, Color4::WHITE);
        batch.draw_region(7, size, rect, rect, Color4::WHITE);
        assert_eq!(
            batch.runs(),
            &[
                Run { texture: RunTexture::White, first_quad: 0, quad_count: 1 },
                Run { texture: RunTexture::Texture(7), first_quad: 1, quad_count: 3 },
                Run { texture: RunTexture::Texture(9), first_quad: 4, quad_count: 1 },
                Run { texture: RunTexture::Texture(7), first_quad: 5, quad_count: 1 },
            ]
        );
        assert_eq!(batch.quad_count(), 6);
        assert_eq!(batch.vertices()[8].textured, 0.0);
    }

    #[test]
    fn the_drawing_order_is_kept() {
        let mut batch = SpriteBuilder::<u32>::with_capacity(4);
        batch.fill_rectangle(RectangleF::new(0.0, 0.0, 1.0, 1.0), Color4::RED);
        batch.fill_rectangle(RectangleF::new(5.0, 0.0, 1.0, 1.0), Color4::GREEN);
        assert_eq!(batch.vertices()[0].color, Color4::RED.to_packed_rgba());
        assert_eq!(batch.vertices()[4].color, Color4::GREEN.to_packed_rgba());
        assert_eq!(position(&batch, 4), Vec2::new(5.0, 0.0));
    }

    #[test]
    fn a_rotated_sprite_turns_clockwise_about_its_origin_and_scales() {
        let mut batch = SpriteBuilder::<u32>::with_capacity(4);
        let source = RectangleF::new(0.0, 0.0, 10.0, 4.0);
        batch.draw_rotated(1, Vec2::new(10.0, 4.0), Vec2::new(100.0, 50.0), source, Color4::WHITE, std::f32::consts::FRAC_PI_2, Vec2::new(5.0, 2.0), Vec2::splat(2.0));
        // The origin lands on the position; the top-left corner (-5, -2) from the origin, scaled
        // by 2 to (-10, -4), turned a quarter clockwise (y down) to (4, -10).
        assert!(near(position(&batch, 0), Vec2::new(104.0, 40.0)), "{:?}", position(&batch, 0));
        assert!(near(sprite_transform(Vec2::new(100.0, 50.0), 0.3, Vec2::new(5.0, 2.0), Vec2::splat(2.0)).transform_point2(Vec2::new(5.0, 2.0)), Vec2::new(100.0, 50.0)));
    }

    #[test]
    fn pushed_transforms_apply_first_and_pop_back() {
        let mut batch = SpriteBuilder::<u32>::with_capacity(4);
        batch.reset(Affine2::from_scale(Vec2::splat(2.0)));
        batch.push_transform(Affine2::from_translation(Vec2::new(10.0, 0.0)));
        batch.fill_rectangle(RectangleF::new(1.0, 1.0, 1.0, 1.0), Color4::WHITE);
        // Translate first, then the initial scale: (1 + 10) * 2.
        assert_eq!(position(&batch, 0), Vec2::new(22.0, 2.0));
        assert!(batch.pop_transform());
        assert!(!batch.pop_transform());
        batch.fill_rectangle(RectangleF::new(1.0, 1.0, 1.0, 1.0), Color4::WHITE);
        assert_eq!(position(&batch, 4), Vec2::new(2.0, 2.0));
    }

    #[test]
    fn shapes_have_the_csharp_geometry() {
        let mut batch = SpriteBuilder::<u32>::with_capacity(4);
        batch.draw_rectangle(RectangleF::new(0.0, 0.0, 10.0, 6.0), Color4::WHITE, 2.0);
        assert_eq!(batch.quad_count(), 4);
        batch.reset(Affine2::IDENTITY);
        batch.draw_line(Vec2::ZERO, Vec2::new(10.0, 0.0), Color4::WHITE, 2.0);
        assert_eq!(position(&batch, 0), Vec2::new(0.0, 1.0));
        assert_eq!(position(&batch, 2), Vec2::new(10.0, -1.0));
        batch.reset(Affine2::IDENTITY);
        batch.draw_line(Vec2::ONE, Vec2::ONE, Color4::WHITE, 2.0);
        assert_eq!(batch.quad_count(), 0);
        batch.fill_circle(Vec2::ZERO, 10.0, Color4::WHITE, 0);
        assert_eq!(batch.quad_count(), 12);
        batch.reset(Affine2::IDENTITY);
        batch.draw_circle(Vec2::ZERO, 200.0, Color4::WHITE, 4.0, 5);
        assert_eq!(batch.quad_count(), 5);
        assert!(near(position(&batch, 0), Vec2::new(202.0, 0.0)));
        assert!(near(position(&batch, 3), Vec2::new(198.0, 0.0)));
    }

    #[test]
    fn the_segment_count_follows_the_radius_within_limits() {
        assert_eq!(segments_for(4.0, 0), 12);
        assert_eq!(segments_for(40.0, 0), 30);
        assert_eq!(segments_for(1000.0, 0), 96);
        assert_eq!(segments_for(1000.0, 7), 7);
        assert_eq!(segments_for(10.0, 2), 12);
    }
}
