using Microsoft.AspNetCore.Authorization;
using Microsoft.AspNetCore.Mvc;

namespace CodeBrix.WebPlatform.WebGpu.Tests.SampleBackend.Controllers;

/// <summary>The one controller of the fixture backend; it is read by the frontend build, never run.</summary>
[ApiController]
[Route("api/gpu")]
[AllowAnonymous]
public sealed class GpuController : ControllerBase
{
    /// <summary>A small answer, so that the generated client code has one function.</summary>
    [HttpGet("greeting")]
    public ActionResult<string> Greeting() => "WebGPU";
}
