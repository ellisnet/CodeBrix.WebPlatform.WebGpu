using CodeBrix.WebPlatform.Core.Mapping;
using GraphicsDemo.Assets;
using Microsoft.AspNetCore.Authorization;
using Microsoft.AspNetCore.Mvc;
using Microsoft.Net.Http.Headers;

namespace GraphicsDemo.Api.Controllers;

/// <summary>
/// The Lottie animations of the "animation" application (GraphicsDemo.Assets): Lottie JSON served unchanged, which the
/// frontend fetches with the generated <c>..._json_bytes</c> functions and reads with the animation crate's
/// DrawingLottie. Lottie files stay on the backend in this sample (its own rule); the crate takes Lottie JSON from
/// anywhere.
/// </summary>
[ApiController]
[AllowAnonymous]
[Route("api/lottie")]
public sealed class LottieController : ControllerBase
{
    private readonly LottieLibrary library;

    /// <summary>Creates the controller.</summary>
    /// <param name="library">The Lottie library (one for the backend).</param>
    public LottieController(LottieLibrary library)
    {
        this.library = library;
    }

    /// <summary>The trial file with three hundred shape layers.</summary>
    /// <returns>The file, or 304 when the caller's copy is current.</returns>
    [HttpGet("many-shapes.json")]
    [WireFile]
    public IActionResult ManyShapesJson() => Serve(LottieLibrary.ManyShapes);

    /// <summary>The trial file with gradients, a mask, a matte and morphing paths.</summary>
    /// <returns>The file, or 304 when the caller's copy is current.</returns>
    [HttpGet("painterly.json")]
    [WireFile]
    public IActionResult PainterlyJson() => Serve(LottieLibrary.Painterly);

    /// <summary>The trial file with nested precompositions and repeaters.</summary>
    /// <returns>The file, or 304 when the caller's copy is current.</returns>
    [HttpGet("busy-scene.json")]
    [WireFile]
    public IActionResult BusySceneJson() => Serve(LottieLibrary.BusyScene);

    /// <summary>The sample's own file: parenting, a trim path, a gradient, a matte and three markers.</summary>
    /// <returns>The file, or 304 when the caller's copy is current.</returns>
    [HttpGet("features.json")]
    [WireFile]
    public IActionResult FeaturesJson() => Serve(LottieLibrary.Features);

    /// <summary>The sample's own file with one image layer, whose pixels the page supplies (the backend's badge).</summary>
    /// <returns>The file, or 304 when the caller's copy is current.</returns>
    [HttpGet("image-layer.json")]
    [WireFile]
    public IActionResult ImageLayerJson() => Serve(LottieLibrary.ImageLayer);

    private IActionResult Serve(string name)
    {
        Response.Headers.CacheControl = ModelsController.BlobCacheControl;
        return File(library.GetFile(name), LottieLibrary.LottieContentType, null, new EntityTagHeaderValue(library.GetETag(name)));
    }
}
