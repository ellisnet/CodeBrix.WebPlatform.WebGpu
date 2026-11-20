using System;
using System.Collections.Generic;
using System.Numerics;
using CodeBrix.WebPlatform.Backend.Problems;
using CodeBrix.WebPlatform.Core.Mapping;
using CodeBrix.WebPlatform.WebGpu.Graphics.Models;
using GraphicsDemo.Api.Contracts;
using GraphicsDemo.Assets;
using GraphicsDemo.Assets.Conversion;
using GraphicsDemo.Assets.Scene;
using Microsoft.AspNetCore.Authorization;
using Microsoft.AspNetCore.Mvc;
using Microsoft.Net.Http.Headers;

namespace GraphicsDemo.Api.Controllers;

/// <summary>
/// The models of the "graphics" application's Viewer scene, converted from glTF on this backend (GraphicsDemo.Assets)
/// into the model data format of CodeBrix.WebPlatform.WebGpu.Graphics: the scene's description as JSON, and the model
/// data blobs, which the frontend fetches with the generated <c>scene_data_bytes</c> and <c>model_data_bytes</c>
/// functions and hands to the graphics crate.
/// </summary>
[ApiController]
[AllowAnonymous]
[Route("api/models")]
public sealed class ModelsController : ControllerBase
{
    /// <summary>
    /// The caching of a blob: any cache may keep it, but must ask again before each use ("no-cache"), so a rebuilt
    /// backend is never answered from a stale copy that no longer agrees with the scene description. Asking again
    /// costs one request answered 304 with no body, because every blob carries a strong ETag.
    /// </summary>
    public const string BlobCacheControl = "public, no-cache";

    private readonly ModelLibrary library;

    /// <summary>Creates the controller.</summary>
    /// <param name="library">The model library (one for the backend).</param>
    public ModelsController(ModelLibrary library)
    {
        this.library = library;
    }

    /// <summary>The Viewer scene: the models its blob holds, the static placements, the animated actors, the camera and the lighting.</summary>
    /// <returns>The scene.</returns>
    [HttpGet("scene")]
    public ActionResult<ViewerScene> Scene()
    {
        ModelBlob blob = library.SceneBlob;
        ViewerScene scene = new ViewerScene { BlobSize = blob.Length, TextureCount = blob.TextureCount };
        Dictionary<string, int> indices = new Dictionary<string, int>(StringComparer.Ordinal);
        foreach (ModelBlobEntry entry in blob.Models)
        {
            scene.Models.Add(new SceneModel { Name = entry.Name, Index = entry.Index, VertexCount = entry.VertexCount, TriangleCount = entry.TriangleCount });
            indices[entry.Name] = entry.Index;
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

    /// <summary>The Viewer scene's model data blob: the ground and every model the scene places (actors and props), each shared texture stored once.</summary>
    /// <returns>The blob (application/octet-stream), or 304 when the caller's copy is current.</returns>
    [HttpGet("scene.cbmd")]
    [WireFile]
    public IActionResult SceneData() => Blob(library.SceneBlob);

    /// <summary>
    /// One model's model data blob (a set of one model), such as api/models/tree.cbmd - also the blobs of the two validation
    /// scenes, api/models/character-archer.cbmd (the skinned Kenney archer) and api/models/morph-shape.cbmd (the morph-target
    /// shape built in code).
    /// </summary>
    /// <param name="name">The model's name.</param>
    /// <returns>The blob (application/octet-stream), 304 when the caller's copy is current, or 404 for an unknown name.</returns>
    [HttpGet("{name}.cbmd")]
    [WireFile]
    public IActionResult ModelData(string name)
    {
        ModelBlob blob = library.GetModelBlob(name);
        if (blob == null) return new ApiProblemResult(ApiProblems.NotFound("There is no model named '" + name + "'. The models are listed by the scene description (api/models/scene)."));
        return Blob(blob);
    }

    /// <summary>
    /// Answers with a blob. The file result compares the request's If-None-Match with the blob's ETag itself and
    /// answers 304 with no body when they match.
    /// </summary>
    private IActionResult Blob(ModelBlob blob)
    {
        Response.Headers.CacheControl = BlobCacheControl;
        return File(blob.Bytes, ModelDataFormat.ContentType, null, new EntityTagHeaderValue(blob.ETag));
    }

    private static SceneVector Vector(Vector3 value) => new SceneVector { X = value.X, Y = value.Y, Z = value.Z };
}
