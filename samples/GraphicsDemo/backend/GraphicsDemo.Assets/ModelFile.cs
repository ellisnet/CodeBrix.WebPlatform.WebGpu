using System;
using System.Security.Cryptography;

namespace GraphicsDemo.Assets;

/// <summary>
/// One model file the backend serves exactly as it is stored - a .glb or .gltf model, a .bin buffer, or a PNG or
/// JPEG texture - for the graphics crate's glTF loader in the browser: its path below Models/, its bytes, its content
/// type and a strong entity tag for HTTP caching.
/// </summary>
public sealed class ModelFile
{
    /// <summary>Wraps a model file's bytes.</summary>
    /// <param name="path">The file's path below Models/ (for example "survival-kit/Textures/colormap.png").</param>
    /// <param name="bytes">The file's bytes.</param>
    /// <param name="contentType">The file's content type.</param>
    /// <exception cref="ArgumentNullException">An argument is null.</exception>
    public ModelFile(string path, byte[] bytes, string contentType)
    {
        ArgumentNullException.ThrowIfNull(path);
        ArgumentNullException.ThrowIfNull(bytes);
        ArgumentNullException.ThrowIfNull(contentType);
        Path = path;
        Bytes = bytes;
        ContentType = contentType;
        ETag = "\"" + Convert.ToHexStringLower(SHA256.HashData(bytes)) + "\"";
    }

    /// <summary>The file's path below Models/, with '/' separators.</summary>
    public string Path { get; }

    /// <summary>The file's bytes. Never modify them: they are shared by every request.</summary>
    public byte[] Bytes { get; }

    /// <summary>The file's size in bytes.</summary>
    public int Length => Bytes.Length;

    /// <summary>The file's content type (for example "model/gltf-binary" or "image/png").</summary>
    public string ContentType { get; }

    /// <summary>A strong entity tag (the quoted lower-case hex SHA-256 of the bytes), for an HTTP ETag header.</summary>
    public string ETag { get; }
}
