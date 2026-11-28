//! `LivePicture` (Not in C#): a picture whose pixels CHANGE IN PLACE - a video frame, a camera
//! image, a simulation's colours, a painting being made - without a new [`ImageSource`] and a new
//! texture every time.
//!
//! An [`ImageSource`] is an immutable shared copy of pixels; the renderer keys its textures by
//! that copy, so a picture that changes every frame as a new `ImageSource` is a whole upload into
//! a NEW texture every frame (and the old ones wait for the sweep). A live picture keeps ONE
//! texture per device and writes only what changed into it. Its
//! [`image_source`](LivePicture::image_source) is a STAND-IN that goes wherever a picture goes (an
//! [`ImageShape`](crate::shapes::ImageShape), the picture a stroke set picks its colours from,
//! a water source's picture): the renderer finds the live picture by it, as it finds a
//! [`PictureTarget`](crate::rendering::PictureTarget) by its stand-in.
//!
//! Crate-internal: the registry that finds a live picture by its stand-in's blob id (one thread),
//! and the bookkeeping of what each texture still has to be sent ([`Pending`],
//! [`DeviceTextures`]: plain data, tested on the host).

use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt;
use std::rc::{Rc, Weak};
use std::sync::Arc;

use codebrix_webgpu::wgpu;
use peniko::{ImageAlphaType, ImageData, ImageFormat};

use crate::geometry::Rectangle;
use crate::images::ImageSource;

/// A region of whole pixels inside a picture (never empty).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct PixelRegion {
    pub(crate) x: u32,
    pub(crate) y: u32,
    pub(crate) width: u32,
    pub(crate) height: u32,
}

impl PixelRegion {
    /// The whole of a picture of `size`.
    pub(crate) fn whole(size: (u32, u32)) -> PixelRegion {
        PixelRegion { x: 0, y: 0, width: size.0, height: size.1 }
    }

    /// Its bytes (4 per pixel).
    pub(crate) fn bytes(&self) -> usize {
        self.width as usize * self.height as usize * 4
    }

    /// True when it holds all of `other`.
    fn contains(&self, other: &PixelRegion) -> bool {
        other.x >= self.x && other.y >= self.y && other.x + other.width <= self.x + self.width && other.y + other.height <= self.y + self.height
    }

    /// The smallest region holding both.
    fn union(&self, other: &PixelRegion) -> PixelRegion {
        let (x, y) = (self.x.min(other.x), self.y.min(other.y));
        let right = (self.x + self.width).max(other.x + other.width);
        let bottom = (self.y + self.height).max(other.y + other.height);
        PixelRegion { x, y, width: right - x, height: bottom - y }
    }
}

/// The most separate regions one texture keeps queued; one more and they become their bounding
/// region (one larger write instead of many small ones).
pub(crate) const MAX_PENDING_REGIONS: usize = 16;

/// What one texture of a live picture still has to be sent before its next draw.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) enum Pending {
    /// It holds the picture as it is.
    #[default]
    Nothing,
    /// The whole picture (a new texture, or writes that cover it).
    Whole,
    /// These regions (each written since the texture was last sent anything).
    Regions(Vec<PixelRegion>),
}

impl Pending {
    /// A write of `region` into a picture of `size`.
    pub(crate) fn add(&mut self, region: PixelRegion, size: (u32, u32)) {
        let whole = PixelRegion::whole(size);
        if region.contains(&whole) {
            *self = Pending::Whole;
            return;
        }
        match self {
            Pending::Whole => {}
            Pending::Nothing => *self = Pending::Regions(vec![region]),
            Pending::Regions(list) => {
                if list.iter().any(|r| r.contains(&region)) {
                    return;
                }
                list.retain(|r| !region.contains(r));
                list.push(region);
                if list.len() > MAX_PENDING_REGIONS {
                    let bound = list.iter().skip(1).fold(list[0], |b, r| b.union(r));
                    *self = if bound.contains(&whole) { Pending::Whole } else { Pending::Regions(vec![bound]) };
                }
            }
        }
    }

    /// The regions to send now, from the picture's pixels as they are (nothing is left pending).
    pub(crate) fn take(&mut self, size: (u32, u32)) -> Vec<PixelRegion> {
        match std::mem::take(self) {
            Pending::Nothing => Vec::new(),
            Pending::Whole => vec![PixelRegion::whole(size)],
            Pending::Regions(list) => list,
        }
    }
}

/// The most devices a live picture keeps a texture on; a further device's texture replaces the
/// least recently drawn one.
pub(crate) const MAX_DEVICES: usize = 4;

/// The textures of a picture, one per device, each with what it still has to be sent (generic
/// over the texture, so that the bookkeeping is tested on the host).
#[derive(Debug)]
pub(crate) struct DeviceTextures<T> {
    entries: Vec<(T, Pending, u64)>,
    uses: u64,
}

impl<T> Default for DeviceTextures<T> {
    fn default() -> DeviceTextures<T> {
        DeviceTextures { entries: Vec::new(), uses: 0 }
    }
}

impl<T> DeviceTextures<T> {
    /// A write of `region`: every texture has to be sent it.
    pub(crate) fn written(&mut self, region: PixelRegion, size: (u32, u32)) {
        for (_, pending, _) in self.entries.iter_mut() {
            pending.add(region, size);
        }
    }

    /// The texture on the device `is_device` recognises - made by `make` when there is none (it
    /// is then sent the whole picture) - and the regions to send it now.
    pub(crate) fn use_on(&mut self, is_device: impl Fn(&T) -> bool, make: impl FnOnce() -> T, size: (u32, u32)) -> (&T, Vec<PixelRegion>) {
        self.uses += 1;
        let index = match self.entries.iter().position(|(t, _, _)| is_device(t)) {
            Some(i) => i,
            None => {
                if self.entries.len() >= MAX_DEVICES {
                    if let Some(oldest) = self.entries.iter().enumerate().min_by_key(|(_, (_, _, used))| *used).map(|(i, _)| i) {
                        self.entries.remove(oldest);
                    }
                }
                self.entries.push((make(), Pending::Whole, 0));
                self.entries.len() - 1
            }
        };
        let entry = &mut self.entries[index];
        entry.2 = self.uses;
        let regions = entry.1.take(size);
        (&entry.0, regions)
    }

    /// The number of textures held (tests).
    #[cfg(test)]
    pub(crate) fn len(&self) -> usize {
        self.entries.len()
    }
}

/// A live picture's texture on one device.
struct LiveTexture {
    device: wgpu::Device,
    /// Kept with its view (written by the uploads).
    texture: wgpu::Texture,
    view: wgpu::TextureView,
}

struct Inner {
    size: (u32, u32),
    /// The stand-in the picture path draws the live picture with: no pixels (4 bytes), the
    /// picture's size, straight alpha as a decoded picture; its blob id is the registry key.
    stand_in: ImageData,
    /// The picture's pixels (straight-alpha RGBA), the source of every upload.
    pixels: RefCell<Vec<u8>>,
    textures: RefCell<DeviceTextures<LiveTexture>>,
}

thread_local! {
    static LIVE: RefCell<HashMap<u64, Weak<Inner>>> = RefCell::new(HashMap::new());
}

impl Drop for Inner {
    fn drop(&mut self) {
        let id = self.stand_in.data.id();
        let _ = LIVE.try_with(|l| l.borrow_mut().remove(&id));
    }
}

/// A picture whose pixels change in place (Not in C#; see the module's text). Clones are the SAME
/// picture (equality is identity).
///
/// THE PIXELS are straight-alpha RGBA, sRGB-encoded, rows top to bottom with no padding - exactly
/// an [`ImageSource::from_rgba`] picture's - and its texture is `Rgba8Unorm` holding them as they
/// are, as a decoded picture's texture does, so it DRAWS IDENTICALLY to an `ImageSource` of the
/// same pixels (the renderer premultiplies on the GPU). No mipmaps (pictures have none). A new
/// live picture is transparent.
///
/// WRITES. [`write`](LivePicture::write) replaces the whole picture, [`write_region`](LivePicture::write_region)
/// a rectangle of it; a region that is empty or reaches outside the picture, or too short a
/// buffer, is REFUSED with a message (nothing is written). A write copies the pixels into the
/// picture at once (the CPU copy is what [`to_rgba`](LivePicture::to_rgba) reads) and queues the
/// region for every texture of the picture; the renderer writes the queued regions into the SAME
/// texture when it next draws the picture, before that draw. A write before the picture's first
/// draw only changes the CPU copy: the texture made at the first draw gets the whole picture.
/// Several writes before a draw are all sent (a region inside another queued one adds nothing;
/// more than a few separate regions are sent as their bounding region), and where they overlap
/// the last write wins. A texture's writes land before the work submitted after them: on a canvas
/// all layers' passes are submitted together at the end of the frame, so EVERY draw of a frame
/// shows the picture with every write made before the frame's last layer that draws it (a write
/// made later shows in the next frame); for an
/// [`OffscreenRenderer`](crate::rendering::OffscreenRenderer), a write made before a `render`
/// that draws the picture is in that render, and in every render recorded into the same encoder.
///
/// SHARING. Every layer and offscreen renderer on one device draws the picture from its ONE
/// texture there, so each shows every write (the first renderer to draw it after a write sends the
/// write, and counts it in [`RenderStats::live_picture_writes`](crate::rendering::RenderStats::live_picture_writes));
/// on another device it has a texture of its own, sent the whole picture when made, then the same
/// writes. The texture is the live picture's, NOT the renderer's: it is not in the renderer's
/// picture cache, not counted in `image_uploads` and never swept; it is released when the last
/// clone of the live picture is dropped. KEEP THE LIVE PICTURE while it is shown: its stand-in does
/// not keep it alive, and a stand-in drawn after the live picture is gone draws nothing
/// (transparent), and reads as transparent pixels.
///
/// THE STAND-IN as a CPU picture: [`ImageSource::to_rgba`] and [`ImageSource::crop`] of it read
/// the live picture's pixels at that moment, so a sprite sheet or brush tips made from it are
/// copies, not live. The session's flat look (which averages a picture's pixels) paints it
/// transparent, as a picture target's. For strokes the picture's own texture can also be any
/// texture of the application's: [`StrokeColorSource::Texture`](crate::strokes::StrokeColorSource::Texture)
/// with an [`ExternalPicture`](crate::strokes::ExternalPicture) - a video frame the application
/// copies into a texture of its own needs no live picture there.
#[derive(Clone)]
pub struct LivePicture {
    inner: Rc<Inner>,
}

/// Checks a region against a picture of `size` and its buffer; the region and the bytes it needs.
pub(crate) fn check_region(size: (u32, u32), area: Rectangle, available: usize) -> Result<(PixelRegion, usize), String> {
    let (w, h) = (size.0 as i64, size.1 as i64);
    let (x, y, rw, rh) = (area.x as i64, area.y as i64, area.width as i64, area.height as i64);
    if rw < 1 || rh < 1 || x < 0 || y < 0 || x + rw > w || y + rh > h {
        return Err(format!("The region {area:?} must be at least 1 x 1 pixel and lie inside the {w} x {h} live picture."));
    }
    let required = (rw * rh * 4) as usize;
    if available < required {
        return Err(format!("The buffer holds {available} bytes but a {rw} x {rh} region needs {required}: pass width x height x 4 bytes (R, G, B, A per pixel, straight alpha, rows of the region top to bottom)."));
    }
    Ok((PixelRegion { x: x as u32, y: y as u32, width: rw as u32, height: rh as u32 }, required))
}

/// Copies the region's rows from `rgba` (tight rows of the region) into `pixels` (a picture of
/// width `width`).
pub(crate) fn copy_region(pixels: &mut [u8], width: u32, region: PixelRegion, rgba: &[u8]) {
    let row = region.width as usize * 4;
    for r in 0..region.height as usize {
        let at = ((region.y as usize + r) * width as usize + region.x as usize) * 4;
        pixels[at..at + row].copy_from_slice(&rgba[r * row..(r + 1) * row]);
    }
}

impl LivePicture {
    /// The largest side a live picture may have (WebGPU's guaranteed texture size).
    pub const MAX_SIZE: u32 = 8192;

    /// A transparent picture of `width` x `height` pixels. The error says what is wrong with the
    /// size.
    pub fn new(width: u32, height: u32) -> Result<LivePicture, String> {
        if width < 1 || height < 1 || width > Self::MAX_SIZE || height > Self::MAX_SIZE {
            return Err(format!("A live picture needs a width and a height of 1 to {} pixels (it was {width} x {height}).", Self::MAX_SIZE));
        }
        let stand_in = ImageData { data: peniko::Blob::new(Arc::new(vec![0u8; 4])), format: ImageFormat::Rgba8, alpha_type: ImageAlphaType::Alpha, width, height };
        let id = stand_in.data.id();
        let pixels = vec![0u8; width as usize * height as usize * 4];
        let inner = Rc::new(Inner { size: (width, height), stand_in, pixels: RefCell::new(pixels), textures: RefCell::new(DeviceTextures::default()) });
        LIVE.with(|l| l.borrow_mut().insert(id, Rc::downgrade(&inner)));
        Ok(LivePicture { inner })
    }

    /// The size in pixels (width, height).
    pub fn size(&self) -> (u32, u32) {
        self.inner.size
    }

    /// The stand-in picture to draw it with, wherever an [`ImageSource`] goes (clones of it are
    /// the same stand-in; see the type's text).
    pub fn image_source(&self) -> ImageSource {
        ImageSource::from_stand_in(self.inner.stand_in.clone())
    }

    /// Replaces the whole picture with straight-alpha RGBA pixels (width x height x 4 bytes, as
    /// [`ImageSource::from_rgba`] takes them). The error says what is wrong with the buffer.
    pub fn write(&self, rgba: &[u8]) -> Result<(), String> {
        let (w, h) = self.inner.size;
        self.write_region(Rectangle::new(0, 0, w as i32, h as i32), rgba)
    }

    /// Replaces the pixels of `area` (whole pixels inside the picture) with straight-alpha RGBA
    /// pixels: area.width x area.height x 4 bytes, the region's rows top to bottom with no
    /// padding. The error says what is wrong with the area or the buffer (nothing is written).
    pub fn write_region(&self, area: Rectangle, rgba: &[u8]) -> Result<(), String> {
        let (region, _) = check_region(self.inner.size, area, rgba.len())?;
        copy_region(&mut self.inner.pixels.borrow_mut(), self.inner.size.0, region, rgba);
        self.inner.textures.borrow_mut().written(region, self.inner.size);
        Ok(())
    }

    /// A copy of the pixels as they are now, straight-alpha RGBA.
    pub fn to_rgba(&self) -> Vec<u8> {
        self.inner.pixels.borrow().clone()
    }

    /// The live picture whose stand-in has this blob id.
    pub(crate) fn find(blob_id: u64) -> Option<LivePicture> {
        LIVE.try_with(|l| l.borrow().get(&blob_id).and_then(Weak::upgrade)).ok().flatten().map(|inner| LivePicture { inner })
    }

    /// The picture's texture view on this device - made at its first use there - with what was
    /// queued for it written into it first; and the regions written (for the counts).
    pub(crate) fn texture_view(&self, device: &wgpu::Device, queue: &wgpu::Queue) -> (wgpu::TextureView, Vec<PixelRegion>) {
        let size = self.inner.size;
        let mut textures = self.inner.textures.borrow_mut();
        let make = || {
            let texture = device.create_texture(&wgpu::TextureDescriptor { label: Some("animation live picture"), size: wgpu::Extent3d { width: size.0, height: size.1, depth_or_array_layers: 1 }, mip_level_count: 1, sample_count: 1, dimension: wgpu::TextureDimension::D2, format: wgpu::TextureFormat::Rgba8Unorm, usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST, view_formats: &[] });
            let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
            LiveTexture { device: device.clone(), texture, view }
        };
        let (entry, regions) = textures.use_on(|t| &t.device == device, make, size);
        let pixels = self.inner.pixels.borrow();
        let row = size.0 as usize * 4;
        for r in &regions {
            // The region's texels from the picture's rows: from its first texel to its last.
            let start = r.y as usize * row + r.x as usize * 4;
            let end = start + (r.height as usize - 1) * row + r.width as usize * 4;
            queue.write_texture(wgpu::TexelCopyTextureInfo { texture: &entry.texture, mip_level: 0, origin: wgpu::Origin3d { x: r.x, y: r.y, z: 0 }, aspect: wgpu::TextureAspect::All }, &pixels[start..end], wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(size.0 * 4), rows_per_image: Some(r.height) }, wgpu::Extent3d { width: r.width, height: r.height, depth_or_array_layers: 1 });
        }
        (entry.view.clone(), regions)
    }
}

impl PartialEq for LivePicture {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.inner, &other.inner)
    }
}

impl fmt::Debug for LivePicture {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "LivePicture({} x {}, id {})", self.inner.size.0, self.inner.size.1, self.inner.stand_in.data.id())
    }
}
