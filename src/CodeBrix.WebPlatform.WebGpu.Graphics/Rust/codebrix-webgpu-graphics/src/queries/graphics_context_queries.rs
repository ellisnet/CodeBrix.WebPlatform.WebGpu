//! The query factories of [`GraphicsContext`] (C#: `GraphicsContext.Queries.cs`).

use crate::context::{label_text, GraphicsContext, GraphicsException};
use crate::queries::{GpuQuerySet, GpuTimer};
use crate::resources::GpuBuffer;

impl GraphicsContext {
    /// The most queries one set may hold (C#: `MaxQueriesPerSet`; WebGPU's limit too).
    pub const MAX_QUERIES_PER_SET: u32 = 4096;

    /// Creates a timestamp query set of `count` queries (C#: `CreateQuerySet(type, count, label)`).
    /// A timestamp set needs the device feature `TIMESTAMP_QUERY`: check
    /// [`GraphicsContext::has_feature`] first (a browser grants it only where the adapter has it -
    /// ask for it with `GraphicsContextOptions::optional_features`). C#'s pipeline-statistics sets
    /// have no counterpart in a browser.
    ///
    /// OCCLUSION sets are REFUSED, with a message that says why: `wgpu`'s browser backend (the
    /// version this package carries) does not hand a render pass's occlusion query set to the
    /// browser, so every pass that began an occlusion query would fail validation and its whole
    /// frame would be lost. The API is kept (C#'s shape) for when a `wgpu` upgrade passes the set.
    pub fn create_query_set(&self, query_type: wgpu::QueryType, count: u32, label: Option<&str>) -> Result<GpuQuerySet, GraphicsException> {
        check_query_set(query_type, count, self.has_feature(wgpu::Features::TIMESTAMP_QUERY), label)?;
        let handle = self.device().create_query_set(&wgpu::QuerySetDescriptor { label, ty: query_type, count });
        Ok(GpuQuerySet::new(handle, query_type, count, label))
    }

    /// Creates a timestamp query set (C#: `CreateTimestampQuerySet`).
    pub fn create_timestamp_query_set(&self, count: u32, label: Option<&str>) -> Result<GpuQuerySet, GraphicsException> {
        self.create_query_set(wgpu::QueryType::Timestamp, count, label)
    }

    /// Creates an occlusion query set (C#: `CreateOcclusionQuerySet`) - REFUSED in this version,
    /// see [`GraphicsContext::create_query_set`].
    pub fn create_occlusion_query_set(&self, count: u32, label: Option<&str>) -> Result<GpuQuerySet, GraphicsException> {
        self.create_query_set(wgpu::QueryType::Occlusion, count, label)
    }

    /// A buffer to resolve `query_set` into: `QUERY_RESOLVE` and the copy usages, the set's
    /// [`GpuQuerySet::resolved_size`] (C#: `CreateQueryResolveBuffer`). Read it with
    /// `read::<u64>(None, 0).await` after the frame that resolved it was submitted.
    pub fn create_query_resolve_buffer(&self, query_set: &GpuQuerySet, label: Option<&str>) -> Result<GpuBuffer, GraphicsException> {
        self.create_buffer(query_set.resolved_size(), wgpu::BufferUsages::QUERY_RESOLVE, label)
    }

    /// Wraps a `wgpu` query set made elsewhere (C#: `WrapQuerySet`).
    pub fn wrap_query_set(&self, query_set: wgpu::QuerySet, label: Option<&str>) -> GpuQuerySet {
        let (query_type, count) = (query_set.ty(), query_set.count());
        GpuQuerySet::new(query_set, query_type, count, label)
    }

    /// A GPU timer of `capacity` slots, each measuring one pass in milliseconds (C#:
    /// `CreateTimer(capacity = 16, label)`). Needs `TIMESTAMP_QUERY`.
    pub fn create_timer(&self, capacity: u32, label: Option<&str>) -> Result<GpuTimer, GraphicsException> {
        GpuTimer::new(self, capacity, label)
    }
}

/// Why occlusion query sets are refused (see [`GraphicsContext::create_query_set`]).
pub(crate) const OCCLUSION_UNAVAILABLE: &str = "Occlusion queries are not available in the browser with this version of wgpu: its WebGPU backend does not pass a render pass's occlusion query set to the browser, so a pass that begins an occlusion query fails validation and its frame is lost. Leave occlusion queries out (timestamp queries work where the browser grants timestamp-query).";

/// The checks of a query set the library makes before WebGPU sees it.
pub(crate) fn check_query_set(query_type: wgpu::QueryType, count: u32, has_timestamps: bool, label: Option<&str>) -> Result<(), GraphicsException> {
    if count < 1 || count > GraphicsContext::MAX_QUERIES_PER_SET {
        return Err(GraphicsException::new(format!("A query set holds 1 to {} queries; the set{} was asked for {count}.", GraphicsContext::MAX_QUERIES_PER_SET, label_text(label))));
    }
    match query_type {
        wgpu::QueryType::Occlusion => Err(GraphicsException::new(OCCLUSION_UNAVAILABLE)),
        wgpu::QueryType::Timestamp if has_timestamps => Ok(()),
        wgpu::QueryType::Timestamp => Err(GraphicsException::new(format!("Timestamp queries need the device feature TIMESTAMP_QUERY, which this device does not have{}; check GraphicsContext::has_feature(wgpu::Features::TIMESTAMP_QUERY) and leave GPU timing out where it is missing (ask for it with GraphicsContextOptions::optional_features).", label_text(label)))),
        _ => Err(GraphicsException::new("A browser has occlusion and timestamp queries only; pipeline-statistics queries are a native feature.")),
    }
}

#[cfg(test)]
mod tests {
    use super::check_query_set;

    #[test]
    fn a_set_holds_one_to_4096_queries() {
        assert!(check_query_set(wgpu::QueryType::Timestamp, 1, true, None).is_ok());
        assert!(check_query_set(wgpu::QueryType::Timestamp, 4096, true, None).is_ok());
        assert!(check_query_set(wgpu::QueryType::Timestamp, 0, true, None).unwrap_err().message().contains("1 to 4096"));
        assert!(check_query_set(wgpu::QueryType::Timestamp, 4097, true, Some("q")).unwrap_err().message().contains("'q'"));
    }

    #[test]
    fn occlusion_sets_are_refused_with_the_reason() {
        let error = check_query_set(wgpu::QueryType::Occlusion, 2, true, None).unwrap_err();
        assert!(error.message().contains("not available in the browser with this version of wgpu"));
        assert!(error.message().contains("Leave occlusion queries out"));
    }

    #[test]
    fn timestamps_need_the_feature() {
        assert!(check_query_set(wgpu::QueryType::Timestamp, 2, true, None).is_ok());
        assert!(check_query_set(wgpu::QueryType::Timestamp, 2, false, None).unwrap_err().message().contains("TIMESTAMP_QUERY"));
    }
}
