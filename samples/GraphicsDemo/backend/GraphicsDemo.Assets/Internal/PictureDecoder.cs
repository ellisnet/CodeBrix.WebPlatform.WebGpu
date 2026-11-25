using System;
using System.IO;
using CodeBrix.Imaging;
using CodeBrix.Imaging.Formats.Jpeg;
using CodeBrix.Imaging.PixelFormats;

namespace GraphicsDemo.Assets.Internal;

/// <summary>
/// Decodes an encoded picture (a PNG file) with CodeBrix.Imaging to tightly packed straight-alpha RGBA8 pixels, rows
/// top to bottom - the pixels <see cref="PictureLibrary" /> serves in the picture-pixels form - and re-encodes one as a
/// JPEG file (the "images" scene's picture for the animation crate's <c>ImageSource::from_jpeg</c>). (The models' textures are
/// NOT decoded here: the graphics crate's glTF loader decodes them in the browser.)
/// </summary>
internal static class PictureDecoder
{
    private static readonly Configuration Decoding = CreateDecodingConfiguration();

    /// <summary>Decodes a picture.</summary>
    /// <param name="encoded">The file bytes.</param>
    /// <param name="what">What is decoded (for messages).</param>
    /// <param name="width">The width in pixels.</param>
    /// <param name="height">The height in pixels.</param>
    /// <returns>The RGBA8 pixels, width x height x 4 bytes.</returns>
    /// <exception cref="InvalidDataException">The bytes are not a picture CodeBrix.Imaging can decode.</exception>
    public static byte[] Decode(ReadOnlyMemory<byte> encoded, string what, out int width, out int height)
    {
        try
        {
            using Image<Rgba32> image = Image.Load<Rgba32>(Decoding, encoded.Span);
            ImageFrame<Rgba32> frame = image.Frames.RootFrame;
            width = frame.Width;
            height = frame.Height;
            byte[] pixels = new byte[checked(width * height * 4)];
            frame.CopyPixelDataTo(pixels);
            return pixels;
        }
        catch (Exception exception) when (exception is not OutOfMemoryException)
        {
            throw new InvalidDataException("The picture " + what + " could not be decoded: " + exception.Message, exception);
        }
    }

    /// <summary>Decodes a picture and encodes it again as a baseline JPEG file (alpha is dropped: JPEG has none).</summary>
    /// <param name="encoded">The file bytes.</param>
    /// <param name="what">What is converted (for messages).</param>
    /// <param name="quality">The JPEG quality, 1 to 100.</param>
    /// <returns>The JPEG file's bytes.</returns>
    /// <exception cref="InvalidDataException">The bytes are not a picture CodeBrix.Imaging can decode.</exception>
    public static byte[] ToJpeg(ReadOnlyMemory<byte> encoded, string what, int quality)
    {
        try
        {
            using Image<Rgba32> image = Image.Load<Rgba32>(Decoding, encoded.Span);
            using MemoryStream stream = new MemoryStream();
            image.SaveAsJpeg(stream, new JpegEncoder { Quality = quality });
            return stream.ToArray();
        }
        catch (Exception exception) when (exception is not OutOfMemoryException)
        {
            throw new InvalidDataException("The picture " + what + " could not be converted to JPEG: " + exception.Message, exception);
        }
    }

    private static Configuration CreateDecodingConfiguration()
    {
        Configuration configuration = Configuration.Default.Clone();
        configuration.PreferContiguousImageBuffers = true;
        return configuration;
    }
}
