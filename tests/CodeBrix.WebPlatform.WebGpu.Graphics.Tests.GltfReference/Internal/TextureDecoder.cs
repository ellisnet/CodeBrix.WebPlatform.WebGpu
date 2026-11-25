using System;
using System.IO;
using CodeBrix.Imaging;
using CodeBrix.Imaging.Formats.Jxl;
using CodeBrix.Imaging.PixelFormats;
using CodeBrix.Imaging.Processing;

namespace CodeBrix.WebPlatform.WebGpu.Graphics.Tests.GltfReference.Internal;

/// <summary>
/// Decodes an encoded image with CodeBrix.Imaging to tightly packed RGBA8 pixels (rows top to bottom), as the C#
/// library's ImageUpload.Decode does: JPEG XL through a decoder that converts to sRGB for a 255-nit display, every
/// other format through the default decoders. Part of the reference converter (<see cref="GltfModelConverter" />).
/// </summary>
internal static class TextureDecoder
{
    private static readonly Configuration Decoding = CreateDecodingConfiguration();

    private static readonly JxlDecoder JxlTextureDecoder = new JxlDecoder { TargetColor = JxlTargetColor.Srgb, TargetDisplayLuminance = 255 };

    private static readonly JxlImageFormatDetector JxlDetector = new JxlImageFormatDetector();

    /// <summary>Decodes an image.</summary>
    /// <param name="encoded">The file bytes.</param>
    /// <param name="what">What is decoded (for messages).</param>
    /// <param name="maxSize">The largest side allowed (0: any); a larger image is halved until it fits.</param>
    /// <param name="width">The width in pixels.</param>
    /// <param name="height">The height in pixels.</param>
    /// <returns>The RGBA8 pixels, width x height x 4 bytes.</returns>
    /// <exception cref="InvalidDataException">The bytes are not an image CodeBrix.Imaging can decode.</exception>
    public static byte[] Decode(ReadOnlyMemory<byte> encoded, string what, int maxSize, out int width, out int height)
    {
        try
        {
            using Image<Rgba32> image = IsJpegXl(encoded.Span) ? DecodeJpegXl(encoded) : Image.Load<Rgba32>(Decoding, encoded.Span);
            if (maxSize > 0 && (image.Width > maxSize || image.Height > maxSize))
            {
                int targetWidth = image.Width;
                int targetHeight = image.Height;
                while ((targetWidth > maxSize || targetHeight > maxSize) && targetWidth > 1 && targetHeight > 1)
                {
                    targetWidth = Math.Max(1, targetWidth / 2);
                    targetHeight = Math.Max(1, targetHeight / 2);
                }

                image.Mutate(Decoding, context => context.Resize(targetWidth, targetHeight));
            }

            ImageFrame<Rgba32> frame = image.Frames.RootFrame;
            width = frame.Width;
            height = frame.Height;
            byte[] pixels = new byte[checked(width * height * 4)];
            frame.CopyPixelDataTo(pixels);
            return pixels;
        }
        catch (Exception exception) when (exception is not OutOfMemoryException)
        {
            throw new InvalidDataException("The image " + what + " could not be decoded: " + exception.Message, exception);
        }
    }

    private static bool IsJpegXl(ReadOnlySpan<byte> data) => JxlDetector.DetectFormat(data) != null;

    private static Image<Rgba32> DecodeJpegXl(ReadOnlyMemory<byte> encoded)
    {
        using MemoryStream stream = new MemoryStream(encoded.ToArray(), false);
        return JxlTextureDecoder.Decode<Rgba32>(Decoding, stream, default);
    }

    private static Configuration CreateDecodingConfiguration()
    {
        Configuration configuration = Configuration.Default.Clone();
        configuration.PreferContiguousImageBuffers = true;
        return configuration;
    }
}
