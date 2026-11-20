namespace CodeBrix.WebPlatform.WebGpu.Graphics.Models.Internal;

/// <summary>The sections of a model data blob and their record sizes, in the order the writer writes them (version 1.0).</summary>
internal static class ModelDataSection
{
    /// <summary>The size of the header in bytes.</summary>
    public const int HeaderSize = 32;

    /// <summary>The size of one section table entry in bytes.</summary>
    public const int TableEntrySize = 24;

    /// <summary>The alignment of every section and of every model's or texture's range in the bulk sections.</summary>
    public const int Alignment = 16;

    /// <summary>The largest blob: every offset and length is a 32-bit unsigned value.</summary>
    public const long MaxBlobLength = uint.MaxValue;

    /// <summary>The section tags, in the order the writer writes them.</summary>
    public static readonly string[] Tags =
    {
        "STRG", "U32S", "F32S", "MODL", "MESH", "PRIM", "MTGT", "MTRL", "TXTR",
        "SMPL", "NODE", "SKIN", "ANIM", "CHAN", "VTX0", "IDX0", "SKV0", "PIX0",
    };

    /// <summary>The record size of each section of <see cref="Tags" /> (1 for byte sections, 4 for the pools).</summary>
    public static readonly int[] RecordSizes =
    {
        1, 4, 4, 120, 24, 64, 24, 336, 40,
        24, 148, 28, 16, 32, 1, 1, 1, 1,
    };

    /// <summary>A tag as the little-endian u32 the section table stores.</summary>
    /// <param name="tag">Four ASCII characters.</param>
    /// <returns>The value.</returns>
    public static uint TagValue(string tag) => tag[0] | ((uint)tag[1] << 8) | ((uint)tag[2] << 16) | ((uint)tag[3] << 24);

    /// <summary>A position rounded up to the alignment.</summary>
    /// <param name="position">The position.</param>
    /// <returns>The aligned position.</returns>
    public static long Align(long position) => (position + Alignment - 1) / Alignment * Alignment;
}
