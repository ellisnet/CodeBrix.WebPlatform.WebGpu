//! [`GpuTimer`]: GPU time per pass in milliseconds from timestamp queries (C#: `GpuTimer`).

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use crate::context::{GraphicsContext, GraphicsException};
use crate::frames::GraphicsFrame;
use crate::internal::oneshot;
use crate::queries::{GpuQuerySet, PassTimestamps};
use crate::resources::GpuBuffer;

/// Measures GPU time in milliseconds with timestamp queries (C#: `GpuTimer`). Each of its
/// [`GpuTimer::capacity`] slots measures one PASS: give [`GpuTimer::pass`] to the pass as its
/// `timestamps` option, call [`GpuTimer::resolve`] once in the frame after the measured passes
/// (outside any pass), and read the results with [`GpuTimer::read_milliseconds`] - ASYNC here (C#'s
/// `ReadMilliseconds` waits for the GPU; a browser cannot). Made by
/// [`GraphicsContext::create_timer`]; needs the device feature `TIMESTAMP_QUERY`.
///
/// A cheap handle: clones are the same timer, so a layer can resolve in its render while a spawned
/// task reads. While a read is in flight, `resolve` still resolves but does not copy into the
/// read-back buffer (WebGPU refuses a copy into a buffer that is being mapped), so the next read
/// sees a later frame. Not here (C#'s `WriteStart` / `WriteEnd`): timestamps between commands or
/// inside a pass are native features a browser does not have.
#[derive(Clone)]
pub struct GpuTimer {
    inner: Rc<Inner>,
}

struct Inner {
    context: GraphicsContext,
    capacity: u32,
    query_set: GpuQuerySet,
    resolve_buffer: GpuBuffer,
    readback_buffer: GpuBuffer,
    resolved: Cell<bool>,
    reading: Cell<bool>,
    ticks: RefCell<Vec<u64>>,
}

impl GpuTimer {
    pub(crate) fn new(context: &GraphicsContext, capacity: u32, label: Option<&str>) -> Result<GpuTimer, GraphicsException> {
        if capacity < 1 || capacity * 2 > GraphicsContext::MAX_QUERIES_PER_SET {
            return Err(GraphicsException::new(format!("A timer has 1 to {} slots; {capacity} were asked for.", GraphicsContext::MAX_QUERIES_PER_SET / 2)));
        }
        let query_set = context.create_timestamp_query_set(capacity * 2, label)?;
        let size = capacity as u64 * 16;
        let resolve_buffer = context.create_buffer(size, wgpu::BufferUsages::QUERY_RESOLVE | wgpu::BufferUsages::COPY_SRC, label)?;
        let readback_buffer = context.create_buffer(size, wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST, label)?;
        Ok(GpuTimer { inner: Rc::new(Inner { context: context.clone(), capacity, query_set, resolve_buffer, readback_buffer, resolved: Cell::new(false), reading: Cell::new(false), ticks: RefCell::new(vec![0; capacity as usize * 2]) }) })
    }

    /// The number of slots (C#: `Capacity`).
    pub fn capacity(&self) -> u32 {
        self.inner.capacity
    }

    /// The timestamp query set: two queries per slot, start and end (C#: `QuerySet`).
    pub fn query_set(&self) -> &GpuQuerySet {
        &self.inner.query_set
    }

    /// The timestamps that measure a whole pass in `slot` (C#: `Pass(slot)`), for
    /// `RenderPassOptions::timestamps` or `ComputePassOptions::timestamps`.
    pub fn pass(&self, slot: u32) -> Result<PassTimestamps, GraphicsException> {
        self.check_slot(slot)?;
        Ok(PassTimestamps::new(&self.inner.query_set, Some(slot * 2), Some(slot * 2 + 1)))
    }

    /// Records the resolve of every slot (C#: `Resolve(frame)`): once per frame after the last
    /// measured pass, outside any pass. Returns false when a read was in flight, so the results of
    /// this frame will not be read (the next frame's will).
    pub fn resolve(&self, frame: &mut GraphicsFrame<'_>) -> bool {
        let inner = &self.inner;
        frame.encoder().resolve_query_set(inner.query_set.handle(), 0..inner.capacity * 2, inner.resolve_buffer.handle(), 0);
        if inner.reading.get() {
            return false;
        }
        frame.copy_buffer(&inner.resolve_buffer, 0, &inner.readback_buffer, 0, inner.resolve_buffer.size());
        inner.resolved.set(true);
        true
    }

    /// True while a [`GpuTimer::read_milliseconds`] waits for the GPU.
    pub fn is_reading(&self) -> bool {
        self.inner.reading.get()
    }

    /// Reads every slot's duration in milliseconds (0 for a slot no pass wrote) after the frame
    /// that resolved them has run on the GPU (C#: `ReadMilliseconds` / `TryReadMilliseconds`;
    /// async here). Ok(None) when nothing has been resolved yet, or a read is already in flight.
    pub async fn read_milliseconds(&self) -> Result<Option<Vec<f64>>, GraphicsException> {
        let inner = &self.inner;
        if !inner.resolved.get() || inner.reading.get() {
            return Ok(None);
        }
        inner.reading.set(true);
        let buffer = inner.readback_buffer.handle().clone();
        let (sender, receiver) = oneshot::channel();
        buffer.map_async(wgpu::MapMode::Read, .., move |result| sender.send(result));
        let mapped = receiver.await;
        let result = match mapped {
            Ok(()) => match buffer.get_mapped_range(..) {
                Ok(view) => {
                    let values: Vec<u64> = bytemuck::pod_collect_to_vec(&view[..]);
                    drop(view);
                    *inner.ticks.borrow_mut() = values;
                    Ok(())
                }
                Err(error) => Err(GraphicsException::new(format!("The timer's results could not be read ({error}); read again."))),
            },
            Err(_) => Err(GraphicsException::new("The timer's read-back buffer could not be mapped (the device may be lost); check GraphicsContext::lost_reason.")),
        };
        if mapped.is_ok() {
            buffer.unmap();
        }
        inner.reading.set(false);
        result?;
        let period = inner.context.queue().get_timestamp_period() as f64;
        Ok(Some(milliseconds_from_ticks(&inner.ticks.borrow(), period)))
    }

    /// The raw start and end ticks of the last read, two per slot (C#: `GetLastTicks`).
    pub fn get_last_ticks(&self) -> Vec<u64> {
        self.inner.ticks.borrow().clone()
    }

    fn check_slot(&self, slot: u32) -> Result<(), GraphicsException> {
        if slot >= self.inner.capacity {
            return Err(GraphicsException::new(format!("The timer has slots 0 to {}; slot {slot} does not exist.", self.inner.capacity - 1)));
        }
        Ok(())
    }
}

/// Milliseconds per slot from start/end tick pairs and the tick period in nanoseconds (C#'s
/// arithmetic: 0 for an unwritten slot or an end not after the start).
pub(crate) fn milliseconds_from_ticks(ticks: &[u64], period_nanoseconds: f64) -> Vec<f64> {
    ticks
        .chunks_exact(2)
        .map(|pair| {
            let (start, end) = (pair[0], pair[1]);
            if start != 0 && end > start { (end - start) as f64 * period_nanoseconds / 1_000_000.0 } else { 0.0 }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::milliseconds_from_ticks;

    #[test]
    fn ticks_become_milliseconds_as_in_csharp() {
        let ticks = [1_000_000, 3_500_000, 0, 9, 50, 40];
        assert_eq!(milliseconds_from_ticks(&ticks, 1.0), vec![2.5, 0.0, 0.0]);
        assert_eq!(milliseconds_from_ticks(&[10, 20], 1_000_000.0), vec![10.0]);
    }
}
