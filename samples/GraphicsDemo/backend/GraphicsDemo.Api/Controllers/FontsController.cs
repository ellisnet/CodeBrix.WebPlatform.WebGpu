using CodeBrix.WebPlatform.Core.Mapping;
using GraphicsDemo.Assets;
using Microsoft.AspNetCore.Authorization;
using Microsoft.AspNetCore.Mvc;
using Microsoft.Net.Http.Headers;

namespace GraphicsDemo.Api.Controllers;

/// <summary>
/// The fonts of the "animation" application (GraphicsDemo.Assets): TrueType files served unchanged, which the
/// frontend fetches with the generated <c>roboto_regular_bytes</c> and <c>open_sans_regular_bytes</c> functions and
/// registers with the animation crate's FontRegistry. Fonts stay on the backend in this sample (its own rule); the
/// crate takes font bytes from anywhere.
/// </summary>
[ApiController]
[AllowAnonymous]
[Route("api/fonts")]
public sealed class FontsController : ControllerBase
{
    private readonly FontLibrary library;

    /// <summary>Creates the controller.</summary>
    /// <param name="library">The font library (one for the backend).</param>
    public FontsController(FontLibrary library)
    {
        this.library = library;
    }

    /// <summary>Roboto Regular (SIL OFL 1.1), the font of every header, label and read-out of the demo.</summary>
    /// <returns>The file (font/ttf), or 304 when the caller's copy is current.</returns>
    [HttpGet("roboto-regular.ttf")]
    [WireFile]
    public IActionResult RobotoRegular() => Send(library.RobotoRegular);

    /// <summary>Open Sans Regular (SIL OFL 1.1), the font whose .notdef glyph has no ink (the missing-characters scene).</summary>
    /// <returns>The file (font/ttf), or 304 when the caller's copy is current.</returns>
    [HttpGet("open-sans-regular.ttf")]
    [WireFile]
    public IActionResult OpenSansRegular() => Send(library.OpenSansRegular);

    private IActionResult Send(FontFile file)
    {
        Response.Headers.CacheControl = ModelsController.BlobCacheControl;
        return File(file.Bytes, FontLibrary.TrueTypeContentType, null, new EntityTagHeaderValue(file.ETag));
    }
}
