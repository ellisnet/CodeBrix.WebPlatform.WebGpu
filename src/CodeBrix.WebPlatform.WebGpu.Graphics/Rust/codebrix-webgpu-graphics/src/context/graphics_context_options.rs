//! [`GraphicsContextOptions`]: how [`GraphicsContext::create`](crate::GraphicsContext::create)
//! asks the browser for an adapter and a device (C#: `GraphicsContextOptions`).

/// How [`GraphicsContext::create`](crate::GraphicsContext::create) asks for its adapter and
/// device (C#: `GraphicsContextOptions`, with the options a browser offers).
///
/// Left out, because a browser has no equivalent: `Backend`, `AdapterSelector` and adapter
/// enumeration (a browser hands out ONE adapter per request), `RequiredNativeFeatures` (a browser
/// has only the WebGPU features), `InstanceFlags`, and `CompatibleWindow` (any adapter of the page
/// presents to its canvases). ADDED for the browser: `optional_features` - features asked for
/// only where the adapter has them, so that a page can use one (timestamp queries, say) where it
/// is granted and do without it elsewhere.
#[derive(Clone, Debug)]
pub struct GraphicsContextOptions {
    /// Which kind of adapter to prefer (C#: `PowerPreference`; default high performance, as in C#).
    pub power_preference: wgpu::PowerPreference,
    /// True to ask for the browser's software (fallback) adapter (C#: `ForceFallbackAdapter`).
    pub force_fallback_adapter: bool,
    /// The device's label (C#: `Label`).
    pub label: Option<String>,
    /// Optional WebGPU features the device must have (C#: `RequiredFeatures`); a device the
    /// adapter cannot open with them makes `create` fail with the browser's reason.
    pub required_features: wgpu::Features,
    /// Limits the device must have (C#: `RequiredLimits`); None for WebGPU's default limits.
    pub required_limits: Option<wgpu::Limits>,
    /// True to ask for every limit the adapter offers (C#: `RequestAdapterLimits`); it takes
    /// precedence over `required_limits`.
    pub request_adapter_limits: bool,
    /// Features asked for WHERE THE ADAPTER HAS THEM (no C# counterpart): the device gets
    /// `required_features` plus those of these the adapter offers. Check what was granted with
    /// `GraphicsContext::has_feature`.
    pub optional_features: wgpu::Features,
    /// True to ask for every feature the adapter offers (C#: `RequestAllAdapterFeatures`).
    pub request_all_adapter_features: bool,
}

impl GraphicsContextOptions {
    /// The features to ask a device for on an adapter that offers `adapter_features`.
    pub(crate) fn features_for(&self, adapter_features: wgpu::Features) -> wgpu::Features {
        if self.request_all_adapter_features {
            return adapter_features | self.required_features;
        }
        self.required_features | (self.optional_features & adapter_features)
    }
}

impl Default for GraphicsContextOptions {
    fn default() -> GraphicsContextOptions {
        GraphicsContextOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            force_fallback_adapter: false,
            label: None,
            required_features: wgpu::Features::empty(),
            required_limits: None,
            request_adapter_limits: false,
            optional_features: wgpu::Features::empty(),
            request_all_adapter_features: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::GraphicsContextOptions;

    #[test]
    fn the_defaults_are_the_csharp_defaults() {
        let options = GraphicsContextOptions::default();
        assert_eq!(options.power_preference, wgpu::PowerPreference::HighPerformance);
        assert!(!options.force_fallback_adapter);
        assert!(options.required_features.is_empty());
        assert!(options.required_limits.is_none());
        assert!(!options.request_adapter_limits);
        assert!(options.optional_features.is_empty());
        assert!(!options.request_all_adapter_features);
    }

    #[test]
    fn optional_features_are_asked_for_only_where_the_adapter_has_them() {
        let adapter = wgpu::Features::TIMESTAMP_QUERY | wgpu::Features::DEPTH_CLIP_CONTROL;
        let mut options = GraphicsContextOptions { optional_features: wgpu::Features::TIMESTAMP_QUERY | wgpu::Features::SHADER_F16, ..GraphicsContextOptions::default() };
        assert_eq!(options.features_for(adapter), wgpu::Features::TIMESTAMP_QUERY);
        assert_eq!(options.features_for(wgpu::Features::empty()), wgpu::Features::empty());
        options.required_features = wgpu::Features::SHADER_F16;
        assert_eq!(options.features_for(adapter), wgpu::Features::TIMESTAMP_QUERY | wgpu::Features::SHADER_F16);
        options.request_all_adapter_features = true;
        assert_eq!(options.features_for(adapter), adapter | wgpu::Features::SHADER_F16);
    }
}
