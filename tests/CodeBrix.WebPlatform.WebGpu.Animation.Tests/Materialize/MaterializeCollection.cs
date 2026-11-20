using Xunit;

namespace CodeBrix.WebPlatform.WebGpu.Animation.Tests.Materialize;

/// <summary>
/// The tests that share the really materialized frontends (<see cref="MaterializedFrontend"/>).
/// They run one after another; the fixture is created once for all of them. The crate-bundle
/// registry is process-wide: the fixture materializes the Animation-only frontend FIRST (with the
/// WebGpu and Animation bundles registered) and only then registers the Graphics package for the
/// second frontend, which names both crates; the other test classes of this assembly register
/// nothing but the WebGpu and Animation bundles, so they may run beside these.
/// </summary>
[CollectionDefinition(Name)]
public sealed class MaterializeCollection : ICollectionFixture<MaterializedFrontend>
{
    /// <summary>The collection's name.</summary>
    public const string Name = "Materialize";
}
