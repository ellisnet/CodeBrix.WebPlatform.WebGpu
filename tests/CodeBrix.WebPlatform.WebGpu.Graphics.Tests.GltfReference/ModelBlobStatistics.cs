namespace CodeBrix.WebPlatform.WebGpu.Graphics.Tests.GltfReference;

/// <summary>
/// What a model data blob costs: its bytes, what of it is geometry and what is texture, and an estimate of the GPU
/// memory the graphics crate gives it once loaded. Filled in when the blob is written.
/// </summary>
public sealed class ModelBlobStatistics
{
    /// <summary>The blob's size in bytes.</summary>
    public long BlobBytes { get; set; }

    /// <summary>The bytes of the vertex arrays (64 bytes a vertex, ModelVertex).</summary>
    public long VertexBytes { get; set; }

    /// <summary>The bytes of the skin vertex arrays (24 bytes a skinned vertex, ModelSkinVertex).</summary>
    public long SkinVertexBytes { get; set; }

    /// <summary>The bytes of the index arrays (2 or 4 bytes an index).</summary>
    public long IndexBytes { get; set; }

    /// <summary>The bytes of the morph-target deltas (12 bytes a delta).</summary>
    public long MorphBytes { get; set; }

    /// <summary>The bytes of the decoded textures (RGBA8, 4 bytes a texel, without mip levels).</summary>
    public long TextureBytes { get; set; }

    /// <summary>The number of textures (each stored once however many models use it).</summary>
    public int TextureCount { get; set; }

    /// <summary>The number of models.</summary>
    public int ModelCount { get; set; }

    /// <summary>The triangles of one instance of every model together.</summary>
    public long Triangles { get; set; }

    /// <summary>The primitives of every model together (each is at least one draw call per frame).</summary>
    public int Primitives { get; set; }

    /// <summary>The blended (alpha BLEND) primitives: each is ONE draw call per instance per frame.</summary>
    public int BlendedPrimitives { get; set; }

    /// <summary>
    /// The estimated GPU memory once loaded: geometry as it is, textures with their full mip chains (4/3 of the base
    /// level). The renderer's per-instance buffers, the environments and the canvas targets are not included.
    /// </summary>
    public long GpuBytesEstimate { get; set; }

    /// <summary>The milliseconds the conversion took (glTF reading, image decoding, writing).</summary>
    public double ConversionMilliseconds { get; set; }
}
