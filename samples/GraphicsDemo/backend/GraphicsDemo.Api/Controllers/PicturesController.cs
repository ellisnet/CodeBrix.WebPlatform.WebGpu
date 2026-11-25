using CodeBrix.WebPlatform.Core.Mapping;
using GraphicsDemo.Assets;
using Microsoft.AspNetCore.Authorization;
using Microsoft.AspNetCore.Mvc;
using Microsoft.Net.Http.Headers;

namespace GraphicsDemo.Api.Controllers;

/// <summary>
/// The pictures of the "animation" application (GraphicsDemo.Assets), DECODED on the backend with CodeBrix.Imaging and
/// served as pixels in the Animation package's picture-pixels form (<see cref="PicturePixels"/>, written by
/// PicturePixelsWriter). The frontend fetches them with the generated <c>cube_pets_colormap_bytes</c> function and hands
/// them to the animation crate's <c>ImageSource::from_picture_pixels</c>. The same picture is also served as the PNG
/// FILE itself (<c>cube_pets_colormap_png_bytes</c>), which the crate decodes in the browser (<c>ImageSource::from_png</c>):
/// the two ways of supplying a picture, side by side.
/// </summary>
[ApiController]
[AllowAnonymous]
[Route("api/pictures")]
public sealed class PicturesController : ControllerBase
{
    private readonly PictureLibrary library;

    /// <summary>Creates the controller.</summary>
    /// <param name="library">The picture library (one for the backend).</param>
    public PicturesController(PictureLibrary library)
    {
        this.library = library;
    }

    /// <summary>The Kenney "Cube Pets" colour map (CC0), 512 x 512, as decoded pixels.</summary>
    /// <returns>The pixels (application/octet-stream), or 304 when the caller's copy is current.</returns>
    [HttpGet("cube-pets-colormap.rgba")]
    [WireFile]
    public IActionResult CubePetsColormap() => Send(library.CubePetsColormap);

    /// <summary>The same colour map as the PNG FILE itself (CC0), which the frontend's crate decodes (ImageSource::from_png).</summary>
    /// <returns>The PNG file (image/png), or 304 when the caller's copy is current.</returns>
    [HttpGet("cube-pets-colormap.png")]
    [WireFile]
    public IActionResult CubePetsColormapPng()
    {
        Response.Headers.CacheControl = ModelsController.BlobCacheControl;
        PngFile png = library.CubePetsColormapPng;
        return File(png.Bytes, PictureLibrary.PngContentType, null, new EntityTagHeaderValue(png.ETag));
    }

    /// <summary>The same colour map as a JPEG FILE made from it on the backend (CC0), which the frontend's crate decodes (ImageSource::from_jpeg).</summary>
    /// <returns>The JPEG file (image/jpeg), or 304 when the caller's copy is current.</returns>
    [HttpGet("cube-pets-colormap.jpg")]
    [WireFile]
    public IActionResult CubePetsColormapJpeg()
    {
        Response.Headers.CacheControl = ModelsController.BlobCacheControl;
        JpegFile jpeg = library.CubePetsColormapJpeg;
        return File(jpeg.Bytes, PictureLibrary.JpegContentType, null, new EntityTagHeaderValue(jpeg.ETag));
    }

    /// <summary>The badge of the Lottie image layer, MADE IN CODE on the backend, as pixels in the picture-pixels form.</summary>
    /// <returns>The pixels (application/octet-stream), or 304 when the caller's copy is current.</returns>
    [HttpGet("lottie-badge.rgba")]
    [WireFile]
    public IActionResult LottieBadge() => Send(library.LottieBadge);

    private IActionResult Send(PicturePixels picture)
    {
        Response.Headers.CacheControl = ModelsController.BlobCacheControl;
        return File(picture.Bytes, PictureLibrary.PixelContentType, null, new EntityTagHeaderValue(picture.ETag));
    }
}
