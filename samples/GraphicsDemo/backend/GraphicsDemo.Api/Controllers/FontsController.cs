using CodeBrix.WebPlatform.Backend.Problems;
using CodeBrix.WebPlatform.Core.Mapping;
using GraphicsDemo.Assets;
using Microsoft.AspNetCore.Authorization;
using Microsoft.AspNetCore.Mvc;
using Microsoft.Net.Http.Headers;

namespace GraphicsDemo.Api.Controllers;

/// <summary>
/// The fonts of the "animation" application (GraphicsDemo.Assets): every embedded TrueType or OpenType file, served
/// unchanged by its file name in lower case (api/fonts/roboto-regular.ttf, api/fonts/opensans-regular.ttf), which the frontend fetches with the
/// generated <c>font_file_bytes</c> function and registers with the animation crate's FontRegistry. The frontend
/// chooses which fonts it asks for; adding a font to the sample is adding its file to GraphicsDemo.Assets/Fonts/. Fonts
/// stay on the backend in this sample (its own rule); the crate takes font bytes from anywhere.
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

    /// <summary>A font file by its name (its file name, such as "roboto-regular.ttf" or "opensans-regular.ttf").</summary>
    /// <param name="file">The font's file name.</param>
    /// <returns>The file (font/ttf or font/otf), 304 when the caller's copy is current, or 404 with the standard problem body.</returns>
    [HttpGet("{file}")]
    [WireFile]
    public IActionResult FontFile(string file)
    {
        Assets.FontFile font = library.GetFont(file);
        if (font == null) return new ApiProblemResult(ApiProblems.NotFound("There is no font named '" + file + "'. The fonts are: " + string.Join(", ", library.FontNames) + "."));
        Response.Headers.CacheControl = ModelsController.BlobCacheControl;
        return File(font.Bytes, font.ContentType, null, new EntityTagHeaderValue(font.ETag));
    }
}
