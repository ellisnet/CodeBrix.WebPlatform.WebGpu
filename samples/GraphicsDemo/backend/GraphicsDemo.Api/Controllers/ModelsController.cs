using System;
using System.Collections.Generic;
using System.Linq;
using System.Numerics;
using CodeBrix.WebPlatform.Backend.Problems;
using CodeBrix.WebPlatform.Core.Mapping;
using CodeBrix.WebPlatform.WebGpu.Graphics.Models;
using GraphicsDemo.Api.Contracts;
using GraphicsDemo.Assets;
using GraphicsDemo.Assets.BuiltModels;
using GraphicsDemo.Assets.Scene;
using Microsoft.AspNetCore.Authorization;
using Microsoft.AspNetCore.Mvc;
using Microsoft.Net.Http.Headers;

namespace GraphicsDemo.Api.Controllers;

/// <summary>
/// The models of the "graphics" application (GraphicsDemo.Assets). The MODEL FILES - .glb models and the textures they
/// name - are served UNCHANGED (<see cref="ModelFile" /> and <see cref="ModelFolderFile" />): the frontend fetches them
/// with the generated <c>model_file_bytes</c> and <c>model_folder_file_bytes</c> functions and the graphics crate's glTF
/// loader reads them in the browser; nothing is converted on the backend. The two models BUILT IN CODE (the ground and
/// the morph-target shape) have no file and are served as model data blobs (<see cref="ModelData" />, generated
/// <c>model_data_bytes</c>). The Viewer scene's description (<see cref="Scene" />) says which model is which.
/// </summary>
[ApiController]
[AllowAnonymous]
[Route("api/models")]
public sealed class ModelsController : ControllerBase
{
    /// <summary>
    /// The caching of every file answer of this backend (the model files and blobs, and the other controllers' files):
    /// any cache may keep it, but must ask again before each use ("no-cache"), so a rebuilt backend is never answered
    /// from a stale copy. Asking again costs one request answered 304 with no body, because every file carries a
    /// strong ETag.
    /// </summary>
    public const string BlobCacheControl = "public, no-cache";

    private readonly ModelLibrary library;

    /// <summary>Creates the controller.</summary>
    /// <param name="library">The model library (one for the backend).</param>
    public ModelsController(ModelLibrary library)
    {
        this.library = library;
    }

    /// <summary>
    /// The Viewer scene: its models (the ground, built in code, first; then the actors' and the props' model files in
    /// the order of their first use), the static placements, the animated actors, the camera and the lighting.
    /// </summary>
    /// <returns>The scene.</returns>
    [HttpGet("scene")]
    public ActionResult<ViewerScene> Scene()
    {
        ViewerScene scene = new ViewerScene();
        Dictionary<string, int> indices = new Dictionary<string, int>(StringComparer.Ordinal);
        List<string> names = new List<string> { GroundModel.Name };
        names.AddRange(ViewerSceneLayout.SceneModels());
        foreach (string name in names)
        {
            indices[name] = scene.Models.Count;
            scene.Models.Add(new SceneModel { Name = name, Index = scene.Models.Count, File = library.FileOf(name) ?? string.Empty, Pack = library.PackOf(name) ?? string.Empty });
        }

        scene.Placements.Add(new ScenePlacement { Model = GroundModel.Name, ModelIndex = indices[GroundModel.Name], Translation = Vector(Vector3.Zero), RotationY = 0f, Scale = 1f });
        foreach (ViewerProp prop in ViewerSceneLayout.Props)
        {
            scene.Placements.Add(new ScenePlacement { Model = prop.Model, ModelIndex = indices[prop.Model], Translation = Vector(prop.Translation), RotationY = prop.RotationY, Scale = prop.Scale });
        }

        foreach (ViewerActor actor in ViewerSceneLayout.Actors)
        {
            SceneActor item = new SceneActor
            {
                Model = actor.Model,
                ModelIndex = indices[actor.Model],
                Motion = actor.Motion.ToString().ToLowerInvariant(),
                Clip = actor.Clip ?? string.Empty,
                TimeOffset = actor.TimeOffset,
                Scale = actor.Scale,
                RotationY = actor.RotationY,
                Translation = Vector(actor.Translation),
                BaseAngle = actor.BaseAngle,
                Radius = actor.Radius,
                AngularSpeed = actor.AngularSpeed,
                Height = actor.Height,
                BobAmplitude = actor.BobAmplitude,
                BobRate = actor.BobRate,
            };
            item.Routine.AddRange(actor.Routine);
            scene.Actors.Add(item);
        }

        scene.RoutineSeconds = ViewerSceneLayout.RoutineSeconds;
        scene.CrossFadeSeconds = ViewerSceneLayout.CrossFadeSeconds;
        scene.Camera = new SceneCamera
        {
            FieldOfViewDegrees = ViewerSceneLayout.FieldOfViewDegrees,
            NearPlane = ViewerSceneLayout.NearPlane,
            FarPlane = ViewerSceneLayout.FarPlane,
            Target = Vector(ViewerSceneLayout.OrbitTarget),
            Distance = ViewerSceneLayout.OrbitDistance,
            Pitch = ViewerSceneLayout.OrbitPitch,
            YawStart = ViewerSceneLayout.OrbitYawStart,
            YawPerSecond = ViewerSceneLayout.OrbitYawPerSecond,
        };
        ReadOnlySpan<byte> sky = ViewerSceneLayout.SkyColorSrgb;
        scene.Lighting = new SceneLighting
        {
            SunDirection = Vector(ViewerSceneLayout.SunDirection),
            LightColor = Vector(ViewerSceneLayout.LightColor),
            LightIntensity = ViewerSceneLayout.LightIntensity,
            SkyZenithColor = Vector(ViewerSceneLayout.SkyZenithColor),
            SkyHorizonColor = Vector(ViewerSceneLayout.SkyHorizonColor),
            SkyGroundColor = Vector(ViewerSceneLayout.SkyGroundColor),
            SkyTurbidity = ViewerSceneLayout.SkyTurbidity,
            PanoramaExposure = ViewerSceneLayout.PanoramaExposure,
            SkyRed = sky[0],
            SkyGreen = sky[1],
            SkyBlue = sky[2],
        };
        return scene;
    }

    /// <summary>
    /// The model data blob of a model BUILT IN CODE (a set of one model): api/models/ground.cbmd (the Viewer scene's
    /// ground) and api/models/morph-shape.cbmd (the morph-target shape of the "Morph targets" scene). Every other model
    /// is a file (<see cref="ModelFile" />).
    /// </summary>
    /// <param name="name">The model's name.</param>
    /// <returns>The blob (application/octet-stream), 304 when the caller's copy is current, or 404 for any other name.</returns>
    [HttpGet("{name}.cbmd")]
    [WireFile]
    public IActionResult ModelData(string name)
    {
        ModelBlob blob = library.GetModelBlob(name);
        if (blob == null) return new ApiProblemResult(ApiProblems.NotFound("There is no model built in code named '" + name + "'. The models built in code are " + string.Join(" and ", ModelLibrary.BuiltModelNames) + "; every other model is a file below api/models/files/."));
        Response.Headers.CacheControl = BlobCacheControl;
        return File(blob.Bytes, ModelDataFormat.ContentType, null, new EntityTagHeaderValue(blob.ETag));
    }

    /// <summary>
    /// A model file in a pack's folder, served unchanged: api/models/files/&lt;pack&gt;/&lt;file&gt;, such as
    /// api/models/files/survival-kit/tree.glb (model/gltf-binary). The path is looked up among the embedded model files
    /// as it is (no file system path is formed from it); only the kinds a model is made of are served (.glb, .gltf,
    /// .bin, .png, .jpg) - never a pack's licence file.
    /// </summary>
    /// <param name="pack">The pack's folder, such as "survival-kit".</param>
    /// <param name="file">The file's name, such as "tree.glb".</param>
    /// <returns>The file, 304 when the caller's copy is current, or 404 with the standard problem body.</returns>
    [HttpGet("files/{pack}/{file}")]
    [WireFile]
    public IActionResult ModelFile(string pack, string file) => Send(pack, null, file);

    /// <summary>
    /// A file one folder deeper in a pack, served unchanged: api/models/files/&lt;pack&gt;/&lt;folder&gt;/&lt;file&gt;, such
    /// as the colour map the pack's models name, api/models/files/survival-kit/Textures/colormap.png (image/png). The
    /// same rules as <see cref="ModelFile" />.
    /// </summary>
    /// <param name="pack">The pack's folder, such as "survival-kit".</param>
    /// <param name="folder">The folder inside the pack, such as "Textures".</param>
    /// <param name="file">The file's name, such as "colormap.png".</param>
    /// <returns>The file, 304 when the caller's copy is current, or 404 with the standard problem body.</returns>
    [HttpGet("files/{pack}/{folder}/{file}")]
    [WireFile]
    public IActionResult ModelFolderFile(string pack, string folder, string file) => Send(pack, folder, file);

    /// <summary>
    /// Answers with a model file. Each part of the path must be a plain name (no separator, no "." or ".."): the parts
    /// are joined with '/' and looked up among the embedded files, so nothing outside the models can be named. The file
    /// result compares the request's If-None-Match with the file's ETag itself and answers 304 with no body when they
    /// match; the platform compresses the answer for the kinds it compresses (glTF and binary data, not PNG or JPEG).
    /// </summary>
    private IActionResult Send(string pack, string folder, string file)
    {
        Assets.ModelFile found = IsPlainName(pack) && (folder == null || IsPlainName(folder)) && IsPlainName(file)
            ? library.GetFile(folder == null ? pack + "/" + file : pack + "/" + folder + "/" + file)
            : null;
        if (found == null) return new ApiProblemResult(ApiProblems.NotFound("There is no model file '" + string.Join("/", new[] { pack, folder, file }.Where(part => part != null)) + "'. The Viewer scene's description (api/models/scene) names the model files."));
        Response.Headers.CacheControl = BlobCacheControl;
        return File(found.Bytes, found.ContentType, null, new EntityTagHeaderValue(found.ETag));
    }

    private static bool IsPlainName(string part)
        => !string.IsNullOrEmpty(part) && part != "." && part != ".." && part.IndexOfAny(new[] { '/', '\\', ':' }) < 0;

    private static SceneVector Vector(Vector3 value) => new SceneVector { X = value.X, Y = value.Y, Z = value.Z };
}
