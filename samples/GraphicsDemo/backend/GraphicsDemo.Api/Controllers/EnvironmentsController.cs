using CodeBrix.WebPlatform.Core.Mapping;
using GraphicsDemo.Assets;
using Microsoft.AspNetCore.Authorization;
using Microsoft.AspNetCore.Mvc;
using Microsoft.Net.Http.Headers;

namespace GraphicsDemo.Api.Controllers;

/// <summary>
/// The environment panoramas of the "graphics" application's Viewer scene (GraphicsDemo.Assets): Radiance .hdr files
/// served unchanged, which the frontend fetches with the generated <c>small_cathedral_bytes</c> function and hands to
/// the graphics crate, whose decoder turns them into an image-based-lighting environment in the browser.
/// </summary>
[ApiController]
[AllowAnonymous]
[Route("api/environments")]
public sealed class EnvironmentsController : ControllerBase
{
    private readonly EnvironmentLibrary library;

    /// <summary>Creates the controller.</summary>
    /// <param name="library">The environment library (one for the backend).</param>
    public EnvironmentsController(EnvironmentLibrary library)
    {
        this.library = library;
    }

    /// <summary>The Poly Haven "Small Cathedral" panorama (1024 x 512 Radiance .hdr, CC0).</summary>
    /// <returns>The file (image/vnd.radiance), or 304 when the caller's copy is current.</returns>
    [HttpGet("small-cathedral.hdr")]
    [WireFile]
    public IActionResult SmallCathedral()
    {
        EnvironmentFile file = library.SmallCathedral;
        Response.Headers.CacheControl = ModelsController.BlobCacheControl;
        return File(file.Bytes, EnvironmentLibrary.RadianceContentType, null, new EntityTagHeaderValue(file.ETag));
    }
}
