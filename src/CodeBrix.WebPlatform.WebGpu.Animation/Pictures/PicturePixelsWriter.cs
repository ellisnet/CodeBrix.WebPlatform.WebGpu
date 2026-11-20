using System;
using System.Buffers.Binary;
using System.IO;

namespace CodeBrix.WebPlatform.WebGpu.Animation.Pictures;

/// <summary>
/// Writes DECODED picture pixels (width, height and straight-alpha RGBA) in the picture-pixels form
/// (<see cref="PicturePixelsFormat" />) for the crate codebrix-webgpu-animation to read in the browser with
/// <c>ImageSource::from_picture_pixels</c>. The backend decodes the picture with whatever it likes (CodeBrix.Imaging,
/// for example); this package has no image dependency and decodes nothing. Little-endian whatever the host's byte
/// order; the same pixels always give the same bytes.
/// </summary>
/// <example>
/// <code>
/// byte[] form = PicturePixelsWriter.Write(rgba, width, height);
/// return File(form, PicturePixelsFormat.ContentType);
/// </code>
/// </example>
public static class PicturePixelsWriter
{
    /// <summary>Writes a picture to a new byte array.</summary>
    /// <param name="rgba">Straight-alpha RGBA pixels, sRGB-encoded, rows top to bottom, exactly width x height x 4 bytes.</param>
    /// <param name="width">The width in pixels, 1 to <see cref="PicturePixelsFormat.MaxDimension" />.</param>
    /// <param name="height">The height in pixels, 1 to <see cref="PicturePixelsFormat.MaxDimension" />.</param>
    /// <returns>The picture in the picture-pixels form.</returns>
    /// <exception cref="ArgumentException">The size is out of range, or the pixels are not width x height x 4 bytes (the message says what).</exception>
    public static byte[] Write(ReadOnlySpan<byte> rgba, int width, int height)
    {
        Check(rgba.Length, width, height);
        byte[] bytes = new byte[PicturePixelsFormat.HeaderLength + rgba.Length];
        WriteHeader(bytes.AsSpan(0, PicturePixelsFormat.HeaderLength), width, height);
        rgba.CopyTo(bytes.AsSpan(PicturePixelsFormat.HeaderLength));
        return bytes;
    }

    /// <summary>Writes a picture to a stream (the stream is left open; nothing is written when a check fails).</summary>
    /// <param name="rgba">Straight-alpha RGBA pixels, sRGB-encoded, rows top to bottom, exactly width x height x 4 bytes.</param>
    /// <param name="width">The width in pixels, 1 to <see cref="PicturePixelsFormat.MaxDimension" />.</param>
    /// <param name="height">The height in pixels, 1 to <see cref="PicturePixelsFormat.MaxDimension" />.</param>
    /// <param name="stream">The stream to write to.</param>
    /// <exception cref="ArgumentNullException"><paramref name="stream" /> is null.</exception>
    /// <exception cref="ArgumentException">The size is out of range, or the pixels are not width x height x 4 bytes (the message says what).</exception>
    public static void Write(ReadOnlySpan<byte> rgba, int width, int height, Stream stream)
    {
        ArgumentNullException.ThrowIfNull(stream);
        Check(rgba.Length, width, height);
        Span<byte> header = stackalloc byte[PicturePixelsFormat.HeaderLength];
        WriteHeader(header, width, height);
        stream.Write(header);
        stream.Write(rgba);
    }

    /// <summary>The length of the form for a picture of this size, in bytes.</summary>
    /// <param name="width">The width in pixels, 1 to <see cref="PicturePixelsFormat.MaxDimension" />.</param>
    /// <param name="height">The height in pixels, 1 to <see cref="PicturePixelsFormat.MaxDimension" />.</param>
    /// <returns>The header length plus width x height x 4.</returns>
    /// <exception cref="ArgumentException">The size is out of range.</exception>
    public static long GetLength(int width, int height)
    {
        CheckSize(width, height);
        return PicturePixelsFormat.HeaderLength + (long)width * height * 4;
    }

    private static void CheckSize(int width, int height)
    {
        if (width < 1 || height < 1 || width > PicturePixelsFormat.MaxDimension || height > PicturePixelsFormat.MaxDimension)
        {
            throw new ArgumentException("A picture must be between 1 x 1 and " + PicturePixelsFormat.MaxDimension + " x " + PicturePixelsFormat.MaxDimension + " pixels (it was " + width + " x " + height + "): scale it before writing it.");
        }
    }

    private static void Check(int length, int width, int height)
    {
        CheckSize(width, height);
        long required = (long)width * height * 4;
        if (length != required)
        {
            throw new ArgumentException("The pixels are " + length + " bytes but a " + width + " x " + height + " picture is " + required + ": pass width x height x 4 bytes (R, G, B, A per pixel, rows top to bottom, no padding).", "rgba");
        }
    }

    private static void WriteHeader(Span<byte> header, int width, int height)
    {
        PicturePixelsFormat.Magic.CopyTo(header);
        BinaryPrimitives.WriteUInt16LittleEndian(header.Slice(4, 2), PicturePixelsFormat.MajorVersion);
        BinaryPrimitives.WriteUInt16LittleEndian(header.Slice(6, 2), PicturePixelsFormat.MinorVersion);
        BinaryPrimitives.WriteUInt32LittleEndian(header.Slice(8, 4), (uint)width);
        BinaryPrimitives.WriteUInt32LittleEndian(header.Slice(12, 4), (uint)height);
    }
}
