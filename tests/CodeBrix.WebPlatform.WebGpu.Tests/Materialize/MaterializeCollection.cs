using Xunit;

namespace CodeBrix.WebPlatform.WebGpu.Tests.Materialize;

/// <summary>
/// The tests that share one really materialized frontend (<see cref="MaterializedFrontend"/>).
/// They run one after another; the fixture is created once for all of them. The crate-bundle
/// registry is process-wide, and the only bundle any test of this assembly registers is this
/// package's own, so the other test classes may run beside these.
/// </summary>
[CollectionDefinition(Name)]
public sealed class MaterializeCollection : ICollectionFixture<MaterializedFrontend>
{
    /// <summary>The collection's name.</summary>
    public const string Name = "Materialize";
}
