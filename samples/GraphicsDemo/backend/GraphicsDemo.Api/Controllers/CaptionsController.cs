using CodeBrix.WebPlatform.Core.Mapping;
using GraphicsDemo.Assets;
using Microsoft.AspNetCore.Authorization;
using Microsoft.AspNetCore.Mvc;
using Microsoft.Net.Http.Headers;

namespace GraphicsDemo.Api.Controllers;

/// <summary>
/// The caption files of the "animation" application (GraphicsDemo.Assets): SubRip text served unchanged, which the
/// frontend fetches with the generated <c>caption_scene_srt_bytes</c> function and parses with the animation crate's
/// CaptionTrack. Caption files stay on the backend in this sample (its own rule); the crate takes caption text from
/// anywhere.
/// </summary>
[ApiController]
[AllowAnonymous]
[Route("api/captions")]
public sealed class CaptionsController : ControllerBase
{
    private readonly CaptionLibrary library;

    /// <summary>Creates the controller.</summary>
    /// <param name="library">The caption library (one for the backend).</param>
    public CaptionsController(CaptionLibrary library)
    {
        this.library = library;
    }

    /// <summary>The captions scene's cues (SRT, UTF-8).</summary>
    /// <returns>The file, or 304 when the caller's copy is current.</returns>
    [HttpGet("caption-scene.srt")]
    [WireFile]
    public IActionResult CaptionSceneSrt()
    {
        Response.Headers.CacheControl = ModelsController.BlobCacheControl;
        return File(library.CaptionScene, CaptionLibrary.SubRipContentType, null, new EntityTagHeaderValue(library.CaptionSceneETag));
    }
}
