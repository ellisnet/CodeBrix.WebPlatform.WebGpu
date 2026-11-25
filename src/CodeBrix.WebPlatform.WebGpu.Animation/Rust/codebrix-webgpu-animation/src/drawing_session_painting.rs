//! The painterly members of `DrawingSession` (C# `DrawBrushStroke`, `ExportOwnershipMap` and the
//! hard-edged `ExportMask`): brushstrokes, ownership maps and hard-edged masks, computed on the
//! CPU from geometry at the session's time; and stroke sets drawn on the GPU (Not in C#).

use crate::drawing_session::DrawingSession;
use crate::mask_meaning::MaskMeaning;
use crate::models::{DrawingLayer, ElementAxes, ElementId};
use crate::painting::DrawingOwnershipMap;
use crate::planes::DrawingPlane;
use crate::rendering::coverage_collector::{self, Area};
use crate::rendering::hard_edge_rasterizer::HardEdgeMatrix;
use crate::shapes::{BrushStrokeShape, BrushStrokeShapeOptions, BrushStrokesShape};

fn check_size(output_size: (u32, u32)) -> Result<(), String> {
    if output_size.0 < 1 || output_size.1 < 1 {
        return Err("The output size must be at least 1 x 1 pixels.".to_string());
    }
    Ok(())
}

fn check_meaning(meaning: MaskMeaning) -> Result<(), String> {
    match meaning {
        MaskMeaning::HardEdged => Ok(()),
        _ => Err("Only MaskMeaning::HardEdged is computed on the CPU: Coverage and VisibleAlpha measure the frame the GPU draws (read it back for tests only). Use MaskMeaning::HardEdged for a mask computed from geometry.".to_string()),
    }
}

impl DrawingSession {
    /// Adds a painterly brushstroke (a tapered, bowed lozenge - see [`BrushStrokeShape`]) to the
    /// active layer, filled in a flat colour (C# `DrawBrushStroke(centerX, centerY, length,
    /// width, angle, bend = 0, color = null)`, which returns the shape; here its id). The options
    /// carry the bend, the colour (None: the layer's) and the profile (C# always uses the default
    /// profile here). The error says what to do.
    pub fn draw_brush_stroke(&mut self, center_x: f32, center_y: f32, length: f32, width: f32, angle: f32, options: BrushStrokeShapeOptions) -> Result<ElementId, String> {
        let stroke = BrushStrokeShape::new(center_x, center_y, length, width, angle, options)?;
        Ok(self.draw_shape(stroke))
    }

    /// Adds a stroke set drawn on the GPU in ONE instanced draw at the session's time (see
    /// [`BrushStrokesShape`]) to the active layer and returns its id (no C# equivalent: draw
    /// thousands of strokes this way; [`draw_brush_stroke`](DrawingSession::draw_brush_stroke)
    /// draws one).
    pub fn draw_brush_strokes(&mut self, shape: BrushStrokesShape) -> ElementId {
        self.draw_shape(shape)
    }

    fn layers_named(&self, layer: Option<&str>) -> Result<Vec<&DrawingLayer>, String> {
        match layer {
            None => Ok(self.layers().iter().collect()),
            Some(name) => match self.get_layer(name) {
                Some(l) => Ok(vec![l]),
                None => Err(format!("The session has no layer named \"{name}\": name one of its layers.")),
            },
        }
    }

    // The hard-edged mask of some layers into a plane (C#'s RenderToMaskCore, HardEdged branch):
    // visible layers only, every element's areas at the session's time, 1 where covered.
    fn hard_edged_mask_into(&self, layers: &[&DrawingLayer], mask: &mut DrawingPlane) -> Result<(), String> {
        let (width, height) = mask.size();
        let data = mask.data_mut();
        data.fill(0.0);
        let mut areas: Vec<Area> = Vec::new();
        for layer in layers {
            if !layer.is_visible() {
                continue;
            }
            let parent = coverage_collector::layer_matrix(self, layer, width, height);
            for element in layer.get_elements() {
                coverage_collector::collect(element, &parent, self.time(), &mut areas)?;
            }
        }
        let w = width as usize;
        coverage_collector::rasterize(&areas, width, height, &mut |y, x0, x1| {
            let row = y as usize * w;
            data[row + x0 as usize..row + x1 as usize].fill(1.0);
        })
    }

    /// Exports a mask of every layer at a pixel size (C# `ExportMask(Size, MaskMeaning)`). Only
    /// [`MaskMeaning::HardEdged`] is computed here - 0 or 1 per pixel by the pixel-centre rule,
    /// from the elements' geometry at the session's [time](DrawingSession::time) - into a new plane;
    /// the drawing occupies its centred aspect-fit rectangle of that size. The error says what to
    /// do (an empty size; a meaning measured from the drawn frame).
    pub fn export_mask(&self, output_size: (u32, u32), meaning: MaskMeaning) -> Result<DrawingPlane, String> {
        check_size(output_size)?;
        check_meaning(meaning)?;
        let mut mask = DrawingPlane::new(output_size.0, output_size.1)?;
        self.hard_edged_mask_into(&self.layers_named(None)?, &mut mask)?;
        Ok(mask)
    }

    /// Exports the mask of ONE layer (C# `ExportMask(DrawingLayer, Size, MaskMeaning)`; see
    /// [`DrawingSession::export_mask`]). A hidden layer gives an empty mask, as in C#.
    pub fn export_layer_mask(&self, layer: &str, output_size: (u32, u32), meaning: MaskMeaning) -> Result<DrawingPlane, String> {
        check_size(output_size)?;
        check_meaning(meaning)?;
        let mut mask = DrawingPlane::new(output_size.0, output_size.1)?;
        self.hard_edged_mask_into(&self.layers_named(Some(layer))?, &mut mask)?;
        Ok(mask)
    }

    /// Exports the mask of every layer into an existing plane at that plane's size, overwriting
    /// every value - the allocation-free form for a frame loop (C# `ExportMask(DrawingPlane,
    /// MaskMeaning)`).
    pub fn export_mask_into(&self, destination: &mut DrawingPlane, meaning: MaskMeaning) -> Result<(), String> {
        check_meaning(meaning)?;
        self.hard_edged_mask_into(&self.layers_named(None)?, destination)
    }

    /// Exports the mask of ONE layer into an existing plane (C# `ExportMask(DrawingLayer,
    /// DrawingPlane, MaskMeaning)`).
    pub fn export_layer_mask_into(&self, layer: &str, destination: &mut DrawingPlane, meaning: MaskMeaning) -> Result<(), String> {
        check_meaning(meaning)?;
        self.hard_edged_mask_into(&self.layers_named(Some(layer))?, destination)
    }

    fn ownership_map(&self, layers: &[&DrawingLayer], output_size: (u32, u32)) -> Result<DrawingOwnershipMap, String> {
        check_size(output_size)?;
        let (width, height) = output_size;
        let total = (width as usize).checked_mul(height as usize).ok_or_else(|| "The map is too large to hold in memory.".to_string())?;
        let mut owners = vec![DrawingOwnershipMap::NOBODY; total];
        let (mut elements, mut element_layers, mut matrices, mut axes): (Vec<ElementId>, Vec<String>, Vec<HardEdgeMatrix>, Vec<ElementAxes>) = (Vec::new(), Vec::new(), Vec::new(), Vec::new());
        let mut areas: Vec<Area> = Vec::new();
        let w = width as usize;
        for layer in layers {
            if !layer.is_visible() {
                continue;
            }
            let parent = coverage_collector::layer_matrix(self, layer, width, height);
            for element in layer.get_elements() {
                let index = elements.len() as i32;
                elements.push(element.id());
                element_layers.push(layer.name().to_string());
                matrices.push(coverage_collector::own_matrix(element, &parent));
                // C#'s GetAllAxes: the shape's own axes, else those of its finite local bounds,
                // else all zeros.
                axes.push(element.get_axes().unwrap_or_else(|| match element.get_bounds() {
                    Some(b) if b.x.is_finite() && b.y.is_finite() && b.width.is_finite() && b.height.is_finite() => ElementAxes::from_bounds(b),
                    _ => ElementAxes::default(),
                }));
                areas.clear();
                coverage_collector::collect(element, &parent, self.time(), &mut areas)?;
                coverage_collector::rasterize(&areas, width, height, &mut |y, x0, x1| {
                    let row = y as usize * w;
                    owners[row + x0 as usize..row + x1 as usize].fill(index);
                })?;
            }
        }
        Ok(DrawingOwnershipMap::new(width, height, owners, elements, element_layers, matrices, axes))
    }

    /// Exports the OWNERSHIP MAP of everything drawn (C# `ExportOwnershipMap(Size)`): for each
    /// pixel, which element owns it - the last element in painting order whose area covers the
    /// pixel's centre, under the hard-edged rule of [`MaskMeaning::HardEdged`]. Elements are
    /// numbered across the visible layers, bottom layer first, each layer's elements in order -
    /// an element hidden at the session's time keeps its number and owns nothing; a group or an
    /// animated shape is ONE element. The drawing occupies its centred aspect-fit rectangle of
    /// the size, and elements count as they stand at the session's time. The error says what to
    /// do.
    pub fn export_ownership_map(&self, output_size: (u32, u32)) -> Result<DrawingOwnershipMap, String> {
        self.ownership_map(&self.layers_named(None)?, output_size)
    }

    /// Exports the ownership map of ONE layer's elements, numbered in that layer's order (C#
    /// `ExportOwnershipMap(DrawingLayer, Size)`).
    pub fn export_layer_ownership_map(&self, layer: &str, output_size: (u32, u32)) -> Result<DrawingOwnershipMap, String> {
        self.ownership_map(&self.layers_named(Some(layer))?, output_size)
    }
}
