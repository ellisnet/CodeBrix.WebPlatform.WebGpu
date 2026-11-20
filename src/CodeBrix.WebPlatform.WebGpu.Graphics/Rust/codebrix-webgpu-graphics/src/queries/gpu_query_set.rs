//! [`GpuQuerySet`]: a set of occlusion or timestamp queries (C#: `GpuQuerySet`), and
//! [`PassTimestamps`]: the timestamps a pass writes when it begins and ends (C#: `PassTimestamps`).

/// A set of GPU queries (C#: `GpuQuerySet`): TIMESTAMP (GPU clock ticks in nanoseconds, written at
/// the beginning and end of passes - needs the device feature `TIMESTAMP_QUERY`), or OCCLUSION
/// (how many samples of the draws between `begin_occlusion_query` and `end_occlusion_query`
/// passed the depth test - REFUSED by this version, see `GraphicsContext::create_query_set`; a
/// wrapped set of that type would fail the same way). Write it from passes, resolve it into a buffer with
/// [`GraphicsFrame::resolve_query_set`](crate::GraphicsFrame::resolve_query_set) and read that
/// buffer with `read::<u64>(..).await`. [`GpuTimer`](crate::GpuTimer) wraps the timestamp case into
/// milliseconds. A cheap handle: clones are the same set; dropping the last one releases it.
///
/// C#'s pipeline-statistics sets have no counterpart: the browser has no such queries.
#[derive(Clone, Debug)]
pub struct GpuQuerySet {
    handle: wgpu::QuerySet,
    query_type: wgpu::QueryType,
    count: u32,
    label: Option<String>,
}

impl GpuQuerySet {
    pub(crate) fn new(handle: wgpu::QuerySet, query_type: wgpu::QueryType, count: u32, label: Option<&str>) -> GpuQuerySet {
        GpuQuerySet { handle, query_type, count, label: label.map(str::to_string) }
    }

    /// The `wgpu` query set (C#: `Handle`).
    pub fn handle(&self) -> &wgpu::QuerySet {
        &self.handle
    }

    /// The query type, `Occlusion` or `Timestamp` (C#: `Type`).
    pub fn query_type(&self) -> wgpu::QueryType {
        self.query_type
    }

    /// The number of queries (C#: `Count`).
    pub fn count(&self) -> u32 {
        self.count
    }

    /// How many 64-bit values one query resolves to: always 1 in the browser (C#:
    /// `ValuesPerQuery`, which is more for pipeline statistics).
    pub fn values_per_query(&self) -> u32 {
        1
    }

    /// The bytes every query of the set resolves to: count x 8 (C#: `ResolvedSize`).
    pub fn resolved_size(&self) -> u64 {
        self.count as u64 * self.values_per_query() as u64 * 8
    }

    /// The label (C#: `Label`).
    pub fn label(&self) -> Option<&str> {
        self.label.as_deref()
    }

    /// Releases the set's GPU memory now (dropping the last clone does it too).
    pub fn destroy(&self) {
        self.handle.destroy();
    }
}

/// The timestamps a render or compute pass writes when it begins and when it ends (C#:
/// `PassTimestamps`): set it as [`RenderPassOptions::timestamps`](crate::RenderPassOptions) or
/// [`ComputePassOptions::timestamps`](crate::ComputePassOptions). Needs a timestamp query set.
#[derive(Clone, Debug)]
pub struct PassTimestamps {
    /// The timestamp query set (C#: `QuerySet`).
    pub query_set: GpuQuerySet,
    /// The query written when the pass begins, or None (C#: `BeginningIndex`, `NotWritten`).
    pub beginning_index: Option<u32>,
    /// The query written when the pass ends, or None (C#: `EndIndex`).
    pub end_index: Option<u32>,
}

impl PassTimestamps {
    /// Timestamps written into queries `beginning_index` and `end_index` (C#: `new
    /// PassTimestamps(querySet, beginningIndex, endIndex)`).
    pub fn new(query_set: &GpuQuerySet, beginning_index: Option<u32>, end_index: Option<u32>) -> PassTimestamps {
        PassTimestamps { query_set: query_set.clone(), beginning_index, end_index }
    }

    /// The render pass's `wgpu` form.
    pub(crate) fn to_render_pass(&self) -> wgpu::RenderPassTimestampWrites<'_> {
        wgpu::RenderPassTimestampWrites { query_set: &self.query_set.handle, beginning_of_pass_write_index: self.beginning_index, end_of_pass_write_index: self.end_index }
    }

    /// The compute pass's `wgpu` form.
    pub(crate) fn to_compute_pass(&self) -> wgpu::ComputePassTimestampWrites<'_> {
        wgpu::ComputePassTimestampWrites { query_set: &self.query_set.handle, beginning_of_pass_write_index: self.beginning_index, end_of_pass_write_index: self.end_index }
    }
}
