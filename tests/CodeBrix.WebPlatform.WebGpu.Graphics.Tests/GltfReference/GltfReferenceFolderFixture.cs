using System;
using System.Collections.Generic;
using System.IO;
using Xunit;

namespace CodeBrix.WebPlatform.WebGpu.Graphics.Tests.GltfReference;

/// <summary>
/// The reference folder written twice from the packs beside the test assembly, into two temporary folders, once for
/// all of <see cref="GltfReferenceFolderTests" /> (the whole run converts every model several times).
/// </summary>
public sealed class GltfReferenceFolderFixture : IDisposable
{
    /// <summary>Writes both folders.</summary>
    public GltfReferenceFolderFixture()
    {
        FirstFolder = Path.Combine(Path.GetTempPath(), "codebrix-gltf-reference-" + Guid.NewGuid().ToString("N"));
        SecondFolder = Path.Combine(Path.GetTempPath(), "codebrix-gltf-reference-" + Guid.NewGuid().ToString("N"));
        FirstCases = GltfReferenceFolder.Write(FirstFolder, TestContext.Current.CancellationToken);
        SecondCases = GltfReferenceFolder.Write(SecondFolder, TestContext.Current.CancellationToken);
    }

    /// <summary>The first reference folder.</summary>
    public string FirstFolder { get; }

    /// <summary>The second reference folder, written the same way.</summary>
    public string SecondFolder { get; }

    /// <summary>The cases the first write returned.</summary>
    public IReadOnlyList<GltfReferenceCase> FirstCases { get; }

    /// <summary>The cases the second write returned.</summary>
    public IReadOnlyList<GltfReferenceCase> SecondCases { get; }

    /// <summary>Deletes both folders.</summary>
    public void Dispose()
    {
        if (Directory.Exists(FirstFolder)) Directory.Delete(FirstFolder, true);
        if (Directory.Exists(SecondFolder)) Directory.Delete(SecondFolder, true);
    }
}
