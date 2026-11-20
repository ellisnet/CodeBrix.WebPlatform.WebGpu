using CodeBrix.WebPlatform.Backend;
using GraphicsDemo.Assets;
using Microsoft.AspNetCore.Builder;
using Microsoft.Extensions.DependencyInjection;

namespace GraphicsDemo.Api;

/// <summary>The one backend of the GraphicsDemo sample: the host of its two frontend applications and of the developer's own page at the site root, the models and environments API (api/models, api/environments) of the "graphics" application, and the fonts (api/fonts) and pictures (api/pictures) of the "animation" application.</summary>
public partial class Program
{
    /// <summary>
    /// The Content-Security-Policy every answer carries. Scripts, styles and connections come
    /// from this site only; <c>'wasm-unsafe-eval'</c> is what lets a browser compile
    /// WebAssembly. WebGPU needs nothing more: the frontends draw through the browser's WebGPU
    /// API from their own WebAssembly, with no script of their own.
    /// </summary>
    public const string ContentSecurityPolicy =
        "default-src 'self'; script-src 'self' 'wasm-unsafe-eval'; style-src 'self'; img-src 'self' data:; "
        + "connect-src 'self'; base-uri 'self'; form-action 'self'; frame-ancestors 'none'";

    /// <summary>Starts the backend.</summary>
    /// <param name="args">Command-line arguments.</param>
    public static void Main(string[] args)
    {
        var builder = WebApplication.CreateBuilder(args);
        builder.Services.AddControllers();
        // The models (GraphicsDemo.Assets): one library for the backend, which converts each
        // model data blob on first use and keeps it.
        builder.Services.AddSingleton<ModelLibrary>();
        // The environment panoramas (Radiance .hdr files, served unchanged).
        builder.Services.AddSingleton<EnvironmentLibrary>();
        // The fonts of the "animation" application (TrueType files, served unchanged).
        builder.Services.AddSingleton<FontLibrary>();
        // The pictures of the "animation" application (decoded on the backend, served as pixels).
        builder.Services.AddSingleton<PictureLibrary>();
        // The caption files of the "animation" application (SRT text, served unchanged).
        builder.Services.AddSingleton<CaptionLibrary>();
        // The SVG documents of the "animation" application (SVG text, served unchanged).
        builder.Services.AddSingleton<SvgDocumentLibrary>();
        // The Lottie animations of the "animation" application (Lottie JSON, served unchanged).
        builder.Services.AddSingleton<LottieLibrary>();
        // The platform's backend library, first call: the standard error bodies for requests
        // that cannot be read, and the options of everything the second call sets up.
        // CompressResponses: the model data, the panoramas, the fonts, the pictures and the caption files ([WireFile] actions) are public
        // assets that hold no user's data, so they are compressed as they are sent.
        builder.Services.AddPlatformBackend(options =>
        {
            options.ContentSecurityPolicy = ContentSecurityPolicy;
            options.CompressResponses = ResponseCompression.FileAnswers;
        });
        var app = builder.Build();

        // Second call: the security headers, the request-marker check, and the frontend
        // applications. It reads the manifest the frontend build wrote into wwwroot and serves
        // what was built: "graphics" below /graphics and "animation" below /animation. Every
        // application has a name, so "/" answers the developer's own wwwroot/index.html. This
        // backend lists no application anywhere. The backend's own API goes after this call.
        app.UsePlatformBackend();

        // The controllers: ModelsController (api/models), the scene description and the model
        // data blobs of the "graphics" application's Viewer scene; EnvironmentsController
        // (api/environments), the scene's HDR panorama; FontsController (api/fonts), the fonts of
        // the "animation" application; PicturesController (api/pictures), its decoded pictures.
        app.UseRouting();
        app.MapControllers();
        app.Run();
    }
}
