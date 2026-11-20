using Xunit;

namespace CodeBrix.WebPlatform.WebGpu.Graphics.Tests.Materialize;

/// <summary>
/// The tests that share one really materialized frontend (<see cref="MaterializedFrontend"/>).
/// They run one after another; the fixture is created once for all of them. The crate-bundle
/// registry is process-wide, and the only bundles any test of this assembly registers are the
/// WebGpu package's and this package's own (through <see cref="GraphicsCrates.Register"/>), so the
/// other test classes may run beside these.
/// </summary>
[CollectionDefinition(Name)]
public sealed class MaterializeCollection : ICollectionFixture<MaterializedFrontend>
{
    /// <summary>The collection's name.</summary>
    public const string Name = "Materialize";
}
