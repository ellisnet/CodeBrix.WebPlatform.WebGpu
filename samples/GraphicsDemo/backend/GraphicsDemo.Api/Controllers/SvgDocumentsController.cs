using CodeBrix.WebPlatform.Core.Mapping;
using GraphicsDemo.Assets;
using Microsoft.AspNetCore.Authorization;
using Microsoft.AspNetCore.Mvc;
using Microsoft.Net.Http.Headers;

namespace GraphicsDemo.Api.Controllers;

/// <summary>
/// The SVG documents of the "animation" application (GraphicsDemo.Assets): SVG text served unchanged, which the frontend
/// fetches with the generated <c>..._svg_bytes</c> functions and loads with the animation crate's DrawingSvg. SVG
/// documents stay on the backend in this sample (its own rule); the crate takes SVG text from anywhere.
/// </summary>
[ApiController]
[AllowAnonymous]
[Route("api/svg-documents")]
public sealed class SvgDocumentsController : ControllerBase
{
    private readonly SvgDocumentLibrary library;

    /// <summary>Creates the controller.</summary>
    /// <param name="library">The SVG document library (one for the backend).</param>
    public SvgDocumentsController(SvgDocumentLibrary library)
    {
        this.library = library;
    }

    /// <summary>The gear train of the "svg-parts" scene.</summary>
    /// <returns>The file, or 304 when the caller's copy is current.</returns>
    [HttpGet("gear-train.svg")]
    [WireFile]
    public IActionResult GearTrainSvg() => Document(SvgDocumentLibrary.GearTrain);

    /// <summary>The clock of the "svg-parts" scene.</summary>
    /// <returns>The file, or 304 when the caller's copy is current.</returns>
    [HttpGet("clock.svg")]
    [WireFile]
    public IActionResult ClockSvg() => Document(SvgDocumentLibrary.Clock);

    /// <summary>The document of the "svg-smil" scene.</summary>
    /// <returns>The file, or 304 when the caller's copy is current.</returns>
    [HttpGet("smil-scene.svg")]
    [WireFile]
    public IActionResult SmilSceneSvg() => Document(SvgDocumentLibrary.SmilScene);

    /// <summary>The document of the "svg-css" scene.</summary>
    /// <returns>The file, or 304 when the caller's copy is current.</returns>
    [HttpGet("css-scene.svg")]
    [WireFile]
    public IActionResult CssSceneSvg() => Document(SvgDocumentLibrary.CssScene);

    /// <summary>The document of the "svg-features" scene.</summary>
    /// <returns>The file, or 304 when the caller's copy is current.</returns>
    [HttpGet("features.svg")]
    [WireFile]
    public IActionResult FeaturesSvg() => Document(SvgDocumentLibrary.Features);

    private IActionResult Document(string name)
    {
        Response.Headers.CacheControl = ModelsController.BlobCacheControl;
        return File(library.Get(name), SvgDocumentLibrary.SvgContentType, null, new EntityTagHeaderValue(library.GetETag(name)));
    }
}
