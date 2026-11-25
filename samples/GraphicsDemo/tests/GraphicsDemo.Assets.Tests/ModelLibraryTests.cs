using System;
using System.Buffers.Binary;
using System.Collections.Generic;
using System.Linq;
using System.Numerics;
using System.Text.Json;
using CodeBrix.WebPlatform.WebGpu.Graphics.Models;
using GraphicsDemo.Assets;
using GraphicsDemo.Assets.BuiltModels;
using GraphicsDemo.Assets.Scene;
using SilverAssertions;
using Xunit;

namespace GraphicsDemo.Assets.Tests;

/// <summary>
/// The embedded model files (served unchanged, read in the browser by the graphics crate's glTF loader), the two models
/// built in code and their blobs, and the Viewer scene's layout. What the glTF loader makes of the files is checked on
/// the repository's test side (the reference converter, which links this sample's Models folder).
/// </summary>
public class ModelLibraryTests
{
    private static readonly ModelLibrary Library = new ModelLibrary();

    private static int Triangles(ModelData model) => (int)model.Meshes.SelectMany(mesh => mesh.Primitives).Sum(primitive => primitive.IndexCount / 3);

    /// <summary>The JSON chunk of a .glb file (glTF 2.0 binary container: a 12-byte header, then the JSON chunk).</summary>
    private static JsonDocument GlbJson(byte[] glb)
    {
        BinaryPrimitives.ReadUInt32LittleEndian(glb).Should().Be(0x46546C67u, "a .glb file starts with 'glTF'");
        BinaryPrimitives.ReadUInt32LittleEndian(glb.AsSpan(4)).Should().Be(2u, "the container is glTF 2.0");
        int length = (int)BinaryPrimitives.ReadUInt32LittleEndian(glb.AsSpan(12));
        BinaryPrimitives.ReadUInt32LittleEndian(glb.AsSpan(16)).Should().Be(0x4E4F534Au, "the first chunk is JSON");
        return JsonDocument.Parse(glb.AsMemory(20, length));
    }

    private static IEnumerable<string> Names(JsonElement root, string array)
        => root.TryGetProperty(array, out JsonElement items) ? items.EnumerateArray().Select(item => item.TryGetProperty("name", out JsonElement name) ? name.GetString() : null) : Enumerable.Empty<string>();

    [Fact]
    public void ModelNames_lists_every_embedded_model_file_the_ground_and_the_morph_shape()
    {
        Library.ModelNames.Should().Contain(new[] { "ground", "morph-shape", "character-archer", "tree", "tree-tall", "tree-autumn", "rock-a", "rock-b", "rock-c", "barrel", "campfire-pit", "patch-grass-large", "fence", "chest", "avian", "animal-cat", "animal-bunny" });
        Library.ModelNames.Count.Should().Be(23);
        ModelLibrary.BuiltModelNames.Should().Equal("ground", "morph-shape");
        Library.FileOf("character-archer").Should().Be("kenney-mini-forest/character-archer.glb");
        Library.PackOf("character-archer").Should().Be("kenney-mini-forest");
        Library.FileOf("tree").Should().Be("survival-kit/tree.glb");
        Library.PackOf("tree").Should().Be("survival-kit");
        Library.PackOf("avian").Should().Be("crystals-and-avian");
        Library.PackOf("animal-fox").Should().Be("cube-pets");
        Library.FileOf("morph-shape").Should().BeNull();
        Library.FileOf("ground").Should().BeNull();
        Library.Contains("morph-shape").Should().BeTrue();
        ModelLibrary.IsBuilt("ground").Should().BeTrue();
        ModelLibrary.IsBuilt("tree").Should().BeFalse();
        Library.Contains("no-such-model").Should().BeFalse();
        Library.FileOf("no-such-model").Should().BeNull();
    }

    [Fact]
    public void GetFile_serves_a_model_file_and_its_texture_unchanged_with_their_content_types()
    {
        //Arrange
        ModelFile tree = Library.GetFile("survival-kit/tree.glb");

        //Act
        ModelFile colormap = Library.GetFile("survival-kit/Textures/colormap.png");

        //Assert
        tree.Path.Should().Be("survival-kit/tree.glb");
        tree.ContentType.Should().Be("model/gltf-binary");
        tree.Bytes.Should().Equal(Library.ReadResource("survival-kit/tree.glb"));
        colormap.ContentType.Should().Be("image/png");
        colormap.Length.Should().BeGreaterThan(8);
        colormap.Bytes.AsSpan(1, 3).ToArray().Should().Equal(new[] { (byte)'P', (byte)'N', (byte)'G' });
    }

    [Fact]
    public void GetFile_refuses_licence_files_unknown_paths_and_paths_not_written_as_embedded()
    {
        //Arrange
        string[] paths =
        {
            "survival-kit/LICENSE-Kenney-survival-kit.txt",
            "survival-kit/no-such.glb",
            "Survival-Kit/tree.glb",
            "survival-kit/../survival-kit/tree.glb",
            "survival-kit\\tree.glb",
            "/survival-kit/tree.glb",
            "tree.glb",
            "",
            null,
        };

        //Act
        List<string> served = paths.Where(path => Library.GetFile(path) != null).ToList();

        //Assert
        served.Should().BeEmpty();
    }

    [Fact]
    public void A_model_file_is_read_once_and_carries_a_strong_etag_of_its_bytes()
    {
        //Arrange
        ModelFile first = Library.GetFile("cube-pets/animal-cat.glb");

        //Act
        ModelFile again = Library.GetFile("cube-pets/animal-cat.glb");

        //Assert
        ReferenceEquals(first, again).Should().BeTrue();
        first.ETag.Should().MatchRegex("^\"[0-9a-f]{64}\"$");
        first.ETag.Should().NotBe(Library.GetFile("cube-pets/animal-dog.glb").ETag);
    }

    [Fact]
    public void Every_model_file_is_a_gltf_2_binary_whose_external_files_are_served_from_its_pack()
    {
        //Arrange
        List<string> problems = new List<string>();

        //Act
        foreach (string name in Library.ModelNames.Where(name => !ModelLibrary.IsBuilt(name)))
        {
            string pack = Library.PackOf(name);
            using JsonDocument json = GlbJson(Library.GetFile(Library.FileOf(name)).Bytes);
            JsonElement root = json.RootElement;
            if (root.GetProperty("asset").GetProperty("version").GetString() != "2.0") problems.Add(name + ": not glTF 2.0");
            foreach (string array in new[] { "images", "buffers" })
            {
                if (!root.TryGetProperty(array, out JsonElement items)) continue;
                foreach (JsonElement item in items.EnumerateArray())
                {
                    if (!item.TryGetProperty("uri", out JsonElement uri)) continue;
                    string path = pack + "/" + Uri.UnescapeDataString(uri.GetString());
                    if (Library.GetFile(path) == null) problems.Add(name + ": " + array + " names '" + uri.GetString() + "', which is not served as " + path);
                }
            }
        }

        //Assert
        problems.Should().BeEmpty();
    }

    [Fact]
    public void Ground_is_the_desktop_viewers_grass_plane()
    {
        //Arrange
        ModelDataSet set = new ModelDataSet();

        //Act
        ModelData ground = GroundModel.AddTo(set);

        //Assert
        ground.Name.Should().Be("ground");
        ground.Vertices.Length.Should().Be(4);
        Triangles(ground).Should().Be(2);
        ground.Vertices.Should().OnlyContain(vertex => vertex.Normal == Vector3.UnitY && vertex.Position.Y == 0f && vertex.Color == Vector4.One);
        ground.Vertices.Max(vertex => vertex.Position.X).Should().Be(GroundModel.HalfSize);
        ground.Indices.Should().Equal(0u, 1u, 2u, 0u, 2u, 3u);
        ModelPrimitiveData primitive = ground.Meshes.Single().Primitives.Single();
        primitive.NormalsGenerated.Should().BeFalse();
        primitive.HasTangents.Should().BeFalse();
        primitive.BoundsMin.Should().Be(new Vector3(-GroundModel.HalfSize, 0, -GroundModel.HalfSize));
        primitive.BoundsMax.Should().Be(new Vector3(GroundModel.HalfSize, 0, GroundModel.HalfSize));
        ground.Nodes.Single().Mesh.Should().Be(0);
        ground.RootNodes.Should().Equal(0);
        ModelMaterialData grass = ground.Materials.Single();
        grass.Name.Should().Be("grass");
        grass.MetallicFactor.Should().Be(0f);
        grass.RoughnessFactor.Should().Be(0.95f);
        grass.BaseColorFactor.X.Should().BeApproximately(0.1559f, 0.0001f);
        grass.BaseColorFactor.Y.Should().BeApproximately(0.3916f, 0.0001f);
        grass.BaseColorFactor.Z.Should().BeApproximately(0.0887f, 0.0001f);
        grass.BaseColorTexture.TextureIndex.Should().Be(-1);
        set.Textures.Should().BeEmpty();
    }

    [Fact]
    public void Blobs_are_written_only_for_the_models_built_in_code_carry_the_format_header_and_are_built_once()
    {
        //Arrange
        ModelBlob ground = Library.GetModelBlob("ground");

        //Act
        ReadOnlySpan<byte> bytes = ground.Bytes;

        //Assert
        bytes.Slice(0, 4).ToArray().Should().Equal(ModelDataFormat.Magic.ToArray());
        BinaryPrimitives.ReadUInt16LittleEndian(bytes.Slice(4)).Should().Be((ushort)ModelDataFormat.MajorVersion);
        BinaryPrimitives.ReadUInt16LittleEndian(bytes.Slice(6)).Should().Be((ushort)ModelDataFormat.MinorVersion);
        BinaryPrimitives.ReadUInt32LittleEndian(bytes.Slice(16)).Should().Be((uint)ground.Length);
        (ground.Length % 16).Should().Be(0);
        Library.GetModelBlob("ground").Should().BeSameAs(ground);
        ground.ETag.Should().MatchRegex("^\"[0-9a-f]{64}\"$");
        ground.TextureCount.Should().Be(0);
        ground.Models.Single().Name.Should().Be("ground");
        ground.Models.Single().VertexCount.Should().Be(4);
        ground.Models.Single().TriangleCount.Should().Be(2);
        Library.GetModelBlob("tree").Should().BeNull();
        Library.GetModelBlob("no-such-model").Should().BeNull();
    }

    [Fact]
    public void Scene_models_are_the_actors_then_the_props_models_each_a_file()
    {
        //Arrange
        IReadOnlyList<string> models = ViewerSceneLayout.SceneModels();

        //Act
        List<string> files = models.Select(Library.FileOf).ToList();

        //Assert
        models.Take(11).Should().Equal("chest", "animal-cat", "animal-dog", "animal-fox", "animal-pig", "animal-cow", "animal-panda", "animal-penguin", "animal-bunny", "avian", "tree");
        models.Count.Should().Be(20);
        files.Should().OnlyContain(file => file != null);
        models.Select(Library.PackOf).Distinct().Should().Equal("survival-kit", "cube-pets", "crystals-and-avian");
    }

    [Fact]
    public void Props_are_placed_as_the_desktop_viewer_places_them()
    {
        //Arrange
        ViewerProp patch = ViewerSceneLayout.Props.First(prop => prop.Model == "patch-grass-large");
        ViewerProp tree = ViewerSceneLayout.Props[0];

        //Act
        Matrix4x4 expected = Matrix4x4.CreateScale(2.6f) * Matrix4x4.CreateRotationY(0.3f + MathF.PI) * Matrix4x4.CreateTranslation(MathF.Sin(0.3f) * 10.5f, 0f, MathF.Cos(0.3f) * 10.5f);

        //Assert
        ViewerSceneLayout.Props.Count.Should().Be(21);
        tree.Model.Should().Be("tree");
        tree.World.Should().Be(expected);
        tree.Lift.Should().Be(0f);
        patch.Lift.Should().Be(0.03f);
        patch.Translation.Y.Should().Be(0.03f);
        ViewerSceneLayout.PropModels().Should().Equal("tree", "tree-tall", "tree-autumn", "rock-a", "rock-b", "rock-c", "barrel", "campfire-pit", "patch-grass-large", "fence");
    }

    [Fact]
    public void Actors_are_placed_and_play_as_the_desktop_viewer_has_them()
    {
        //Arrange
        IReadOnlyList<ViewerActor> actors = ViewerSceneLayout.Actors;
        ViewerActor walker = actors[1];
        ViewerActor stander = actors[2];
        ViewerActor bird = actors[^1];
        float angle = 0.13f * 0.28f;

        //Act
        Matrix4x4 walking = Matrix4x4.CreateScale(0.55f) * Matrix4x4.CreateRotationY(angle + (MathF.PI / 2)) * Matrix4x4.CreateTranslation(new Vector3(MathF.Sin(angle), 0, MathF.Cos(angle)) * 5.4f);
        float birdAngle = -2f * 0.45f;
        Vector3 birdAt = new Vector3(MathF.Sin(birdAngle) * 4.2f, 3.0f + (MathF.Sin(2f * 1.7f) * 0.25f), MathF.Cos(birdAngle) * 4.2f);

        //Assert
        actors.Count.Should().Be(10);
        actors[0].Model.Should().Be("chest");
        actors[0].Clip.Should().Be("open-close");
        actors[0].WorldAt(5f).Should().Be(Matrix4x4.CreateScale(3.2f) * Matrix4x4.CreateRotationY(0.6f));
        walker.Model.Should().Be("animal-cat");
        walker.Motion.Should().Be(ViewerActorMotion.Walk);
        walker.Clip.Should().Be("walk");
        walker.TimeOffset.Should().Be(0f);
        walker.WorldAt(0.13f).Should().Be(walking);
        stander.Model.Should().Be("animal-dog");
        stander.Clip.Should().BeNull();
        stander.Routine.Should().Equal("idle", "dance", "eat", "gesture-positive", "idle", "dance");
        stander.TimeOffset.Should().Be(0.9f);
        stander.WorldAt(0f).Should().Be(stander.WorldAt(9f));
        actors[3].TimeOffset.Should().Be(2 * 0.13f);
        bird.Model.Should().Be("avian");
        bird.Clip.Should().Be("fly");
        (bird.WorldAt(2f).Translation - birdAt).Length().Should().BeLessThan(1e-5f);
        actors.Count(actor => actor.Motion == ViewerActorMotion.Walk).Should().Be(4);
        actors.Count(actor => actor.Routine.Count > 0).Should().Be(4);
        ViewerSceneLayout.RoutineSeconds.Should().Be(3.5f);
        ViewerSceneLayout.CrossFadeSeconds.Should().Be(0.4f);
    }

    [Fact]
    public void Every_actor_names_a_clip_its_model_file_has()
    {
        //Arrange
        List<string> missing = new List<string>();

        //Act
        foreach (ViewerActor actor in ViewerSceneLayout.Actors)
        {
            using JsonDocument json = GlbJson(Library.GetFile(Library.FileOf(actor.Model)).Bytes);
            HashSet<string> clips = Names(json.RootElement, "animations").ToHashSet(StringComparer.Ordinal);
            foreach (string clip in actor.Routine.Append(actor.Clip).Where(clip => clip != null))
            {
                if (!clips.Contains(clip)) missing.Add(actor.Model + ": " + clip);
            }
        }

        //Assert
        missing.Should().BeEmpty();
    }

    [Fact]
    public void Archer_file_has_skins_its_clips_and_the_node_the_skinned_scene_reads_out()
    {
        //Arrange
        ModelFile archer = Library.GetFile(Library.FileOf("character-archer"));

        //Act
        using JsonDocument json = GlbJson(archer.Bytes);

        //Assert
        json.RootElement.GetProperty("skins").GetArrayLength().Should().BeGreaterThan(0);
        Names(json.RootElement, "animations").Should().Contain("walk");
        Names(json.RootElement, "nodes").Should().Contain("arm-left");
        json.RootElement.GetProperty("images").EnumerateArray().Select(image => image.GetProperty("uri").GetString()).Should().Equal("Textures/colormap.png");
    }
}
