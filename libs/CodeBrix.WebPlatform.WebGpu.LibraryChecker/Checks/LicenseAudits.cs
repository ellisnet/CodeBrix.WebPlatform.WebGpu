using CodeBrix.RustTools.Licensing;

namespace CodeBrix.WebPlatform.WebGpu.LibraryChecker.Checks;

/// <summary>
/// The license audits of the vendored crates, kept so that the notices can be written from the
/// same findings the gate judged.
/// </summary>
public sealed class LicenseAudits
{
    /// <summary>The crates linked into a platform application's WebAssembly file that uses wgpu (strictest tier).</summary>
    public LicenseAuditReport Application { get; init; }

    /// <summary>Every vendored crate (the vendored tier).</summary>
    public LicenseAuditReport Vendored { get; init; }
}
