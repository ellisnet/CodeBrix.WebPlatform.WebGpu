using System;
using System.Buffers.Binary;
using System.IO;

namespace CodeBrix.WebPlatform.WebGpu.Graphics.Models.Internal;

/// <summary>Streams the bulk sections of a model data blob through one reusable buffer, little-endian whatever the host's byte order, and keeps the absolute position for the zero padding.</summary>
internal sealed class ChunkWriter
{
    private readonly Stream destination;
    private readonly byte[] buffer = new byte[64 * 1024];
    private int used;

    /// <summary>Starts at a position of the blob.</summary>
    /// <param name="destination">The stream.</param>
    /// <param name="position">The number of bytes already written.</param>
    public ChunkWriter(Stream destination, long position)
    {
        this.destination = destination;
        Position = position;
    }

    /// <summary>The absolute position in the blob.</summary>
    public long Position { get; private set; }

    /// <summary>Accounts for bytes written to the stream directly (after <see cref="Flush" />).</summary>
    /// <param name="count">The number of bytes.</param>
    public void Advance(int count) => Position += count;

    /// <summary>Writes zero bytes up to an absolute position.</summary>
    /// <param name="position">The position.</param>
    public void PadTo(long position)
    {
        if (position < Position) throw new InvalidOperationException("Internal error: the model data layout went backwards; report this as a bug of CodeBrix.WebPlatform.WebGpu.Graphics.");
        while (Position < position) Take((int)Math.Min(position - Position, 16)).Clear();
    }

    /// <summary>Writes bytes as they are.</summary>
    /// <param name="bytes">The bytes.</param>
    public void Bytes(byte[] bytes)
    {
        Flush();
        destination.Write(bytes, 0, bytes.Length);
        Position += bytes.Length;
    }

    /// <summary>Writes a 16-bit unsigned value.</summary>
    /// <param name="value">The value.</param>
    public void U16(ushort value) => BinaryPrimitives.WriteUInt16LittleEndian(Take(2), value);

    /// <summary>Writes a 32-bit unsigned value.</summary>
    /// <param name="value">The value.</param>
    public void U32(uint value) => BinaryPrimitives.WriteUInt32LittleEndian(Take(4), value);

    /// <summary>Writes one <see cref="ModelVertex" /> (64 bytes).</summary>
    /// <param name="vertex">The vertex.</param>
    public void Vertex(in ModelVertex vertex)
    {
        var span = Take(ModelVertex.Size);
        F32(span, 0, vertex.Position.X);
        F32(span, 4, vertex.Position.Y);
        F32(span, 8, vertex.Position.Z);
        F32(span, 12, vertex.Normal.X);
        F32(span, 16, vertex.Normal.Y);
        F32(span, 20, vertex.Normal.Z);
        F32(span, 24, vertex.Tangent.X);
        F32(span, 28, vertex.Tangent.Y);
        F32(span, 32, vertex.Tangent.Z);
        F32(span, 36, vertex.Tangent.W);
        F32(span, 40, vertex.TexCoord0.X);
        F32(span, 44, vertex.TexCoord0.Y);
        F32(span, 48, vertex.TexCoord1.X);
        F32(span, 52, vertex.TexCoord1.Y);
        BinaryPrimitives.WriteUInt16LittleEndian(span.Slice(56), vertex.ColorR);
        BinaryPrimitives.WriteUInt16LittleEndian(span.Slice(58), vertex.ColorG);
        BinaryPrimitives.WriteUInt16LittleEndian(span.Slice(60), vertex.ColorB);
        BinaryPrimitives.WriteUInt16LittleEndian(span.Slice(62), vertex.ColorA);
    }

    /// <summary>Writes one <see cref="ModelSkinVertex" /> (24 bytes).</summary>
    /// <param name="vertex">The skin vertex.</param>
    public void SkinVertex(in ModelSkinVertex vertex)
    {
        var span = Take(ModelSkinVertex.Size);
        BinaryPrimitives.WriteUInt16LittleEndian(span, vertex.Joint0);
        BinaryPrimitives.WriteUInt16LittleEndian(span.Slice(2), vertex.Joint1);
        BinaryPrimitives.WriteUInt16LittleEndian(span.Slice(4), vertex.Joint2);
        BinaryPrimitives.WriteUInt16LittleEndian(span.Slice(6), vertex.Joint3);
        F32(span, 8, vertex.Weights.X);
        F32(span, 12, vertex.Weights.Y);
        F32(span, 16, vertex.Weights.Z);
        F32(span, 20, vertex.Weights.W);
    }

    /// <summary>Writes the buffered bytes to the stream.</summary>
    public void Flush()
    {
        destination.Write(buffer, 0, used);
        used = 0;
    }

    private static void F32(Span<byte> span, int offset, float value) => BinaryPrimitives.WriteSingleLittleEndian(span.Slice(offset), value);

    private Span<byte> Take(int count)
    {
        if (used + count > buffer.Length) Flush();
        var span = buffer.AsSpan(used, count);
        used += count;
        Position += count;
        return span;
    }
}
