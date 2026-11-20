using System;
using System.Buffers.Binary;
using System.IO;
using System.Text;

namespace CodeBrix.WebPlatform.WebGpu.Graphics.Models.Internal;

/// <summary>A growable byte buffer that appends values little-endian whatever the host's byte order (the model data format's tables and pools).</summary>
internal sealed class LittleEndianBuffer
{
    private byte[] bytes = new byte[256];

    /// <summary>The number of bytes appended.</summary>
    public int Length { get; private set; }

    /// <summary>Appends a 16-bit unsigned value.</summary>
    /// <param name="value">The value.</param>
    public void U16(ushort value) => BinaryPrimitives.WriteUInt16LittleEndian(Grow(2), value);

    /// <summary>Appends a 32-bit unsigned value.</summary>
    /// <param name="value">The value.</param>
    public void U32(uint value) => BinaryPrimitives.WriteUInt32LittleEndian(Grow(4), value);

    /// <summary>Appends a 32-bit signed value.</summary>
    /// <param name="value">The value.</param>
    public void I32(int value) => BinaryPrimitives.WriteInt32LittleEndian(Grow(4), value);

    /// <summary>Appends a 32-bit float.</summary>
    /// <param name="value">The value.</param>
    public void F32(float value) => BinaryPrimitives.WriteSingleLittleEndian(Grow(4), value);

    /// <summary>Appends 0 or 1.</summary>
    /// <param name="value">The value.</param>
    public void Bool(bool value) => U32(value ? 1u : 0u);

    /// <summary>Appends a string's UTF-8 bytes (no terminator).</summary>
    /// <param name="value">The string.</param>
    /// <returns>The number of bytes appended.</returns>
    public int Utf8(string value)
    {
        var count = Encoding.UTF8.GetByteCount(value);
        Encoding.UTF8.GetBytes(value, Grow(count));
        return count;
    }

    /// <summary>Writes the appended bytes to a stream.</summary>
    /// <param name="destination">The stream.</param>
    public void WriteTo(Stream destination) => destination.Write(bytes, 0, Length);

    private Span<byte> Grow(int count)
    {
        if (Length + count > bytes.Length)
        {
            Array.Resize(ref bytes, Math.Max(bytes.Length * 2, Length + count));
        }

        var span = bytes.AsSpan(Length, count);
        Length += count;
        return span;
    }
}
