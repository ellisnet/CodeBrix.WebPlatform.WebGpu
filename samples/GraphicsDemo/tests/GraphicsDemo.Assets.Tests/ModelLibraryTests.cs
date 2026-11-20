using System;
using System.Buffers.Binary;
using System.Collections.Generic;
using System.Linq;
using System.Numerics;
using CodeBrix.WebPlatform.WebGpu.Graphics.Models;
using GraphicsDemo.Assets;
using GraphicsDemo.Assets.Conversion;
using GraphicsDemo.Assets.Scene;
using SilverAssertions;
using Xunit;

namespace GraphicsDemo.Assets.Tests;

/// <summary>The embedded models, their conversion to model data, the scene set and the blobs.</summary>
public class ModelLibraryTests
{
    private static readonly ModelLibrary Library = new ModelLibrary();

    private static int Triangles(ModelData model) => (int)model.Meshes.SelectMany(mesh => mesh.Primitives).Sum(primitive => primitive.IndexCount / 3);

    [Fact]
    public void ModelNames_lists_every_embedded_model_the_ground_and_the_morph_shape()
    {
        Library.ModelNames.Should().Contain(new[] { "ground", "morph-shape", "character-archer", "tree", "tree-tall", "tree-autumn", "rock-a", "rock-b", "rock-c", "barrel", "campfire-pit", "patch-grass-large", "fence", "chest", "avian", "animal-cat", "animal-bunny" });
        Library.ModelNames.Count.Should().Be(23);
        Library.PackOf("character-archer").Should().Be("kenney-mini-forest");
        Library.PackOf("morph-shape").Should().BeNull();
        Library.Contains("morph-shape").Should().BeTrue();
        Library.PackOf("tree").Should().Be("survival-kit");
        Library.PackOf("avian").Should().Be("crystals-and-avian");
        Library.PackOf("animal-fox").Should().Be("cube-pets");
        Library.Contains("no-such-model").Should().BeFalse();
        Library.GetModelBlob("no-such-model").Should().BeNull();
    }

    [Fact]
    public void Tree_converts_with_its_geometry_material_texture_and_sampler()
    {
        //Arrange
        ModelDataSet set = Library.BuildModelSet("tree");

        //Act
        ModelData tree = set.Models.Single();

        //Assert
        tree.Name.Should().Be("tree");
        tree.Vertices.Length.Should().Be(396);
        Triangles(tree).Should().Be(226);
        tree.IndexFormat.Should().Be(ModelIndexFormat.Uint16);
        tree.Nodes.Count.Should().Be(1);
        tree.Nodes[0].InScene.Should().BeTrue();
        tree.RootNodes.Should().Equal(0);
        tree.Warnings.Should().BeEmpty();

        ModelPrimitiveData primitive = tree.Meshes.Single().Primitives.Single();
        primitive.Material.Should().Be(0);
        primitive.HasTangents.Should().BeTrue();
        primitive.NormalsGenerated.Should().BeFalse();

        ModelMaterialData material = tree.Materials.Single();
        material.Name.Should().Be("colormap");
        material.BaseColorTexture.TextureIndex.Should().Be(0);
        material.BaseColorTexture.Texture.Should().Be(0);
        material.BaseColorTexture.HasTransform.Should().BeTrue();
        tree.Textures.Should().Equal(0);

        ModelTextureData texture = set.Textures.Single();
        texture.Name.Should().Be("survival-kit/Textures/colormap.png");
        texture.ColorSpace.Should().Be(TextureColorSpace.Srgb);
        texture.Mipmaps.Should().BeTrue();
        texture.Width.Should().Be(512);
        texture.Height.Should().Be(512);
        texture.Pixels.Length.Should().Be(512 * 512 * 4);

        ModelSamplerData sampler = tree.Samplers[material.BaseColorTexture.Sampler];
        sampler.MagFilter.Should().Be(SamplerFilter.Nearest);
        sampler.MinFilter.Should().Be(SamplerFilter.Nearest);
        sampler.MipmapFilter.Should().Be(SamplerFilter.Nearest);
        sampler.AddressModeU.Should().Be(SamplerAddressMode.Repeat);
        sampler.AddressModeV.Should().Be(SamplerAddressMode.Repeat);
        sampler.MaxLod.Should().Be(32f);
    }

    [Fact]
    public void Avian_converts_with_its_nodes_meshes_and_clips()
    {
        //Arrange
        ModelDataSet set = Library.BuildModelSet("avian");

        //Act
        ModelData avian = set.Models.Single();

        //Assert
        avian.Nodes.Count.Should().Be(13);
        avian.Meshes.Count.Should().Be(11);
        avian.Vertices.Length.Should().Be(3347);
        Triangles(avian).Should().Be(1798);
        avian.Animations.Select(animation => animation.Name).Should().Equal("static", "idle", "fly", "take-off", "land", "create", "nod-yes", "nod-no");
        avian.Animations.Single(animation => animation.Name == "fly").Channels.Should().NotBeEmpty();
        avian.Skins.Should().BeEmpty();
        avian.SkinVertices.Should().BeNull();
        set.Textures.Single().Name.Should().Be("crystals-and-avian/Textures/colormap.png");
        avian.Samplers.Single().MinFilter.Should().Be(SamplerFilter.Linear);
        avian.Samplers.Single().MipmapFilter.Should().Be(SamplerFilter.Linear);
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
        ground.Vertices.Should().OnlyContain(vertex => vertex.Normal == Vector3.UnitY && vertex.Position.Y == 0f);
        ground.Vertices.Max(vertex => vertex.Position.X).Should().Be(GroundModel.HalfSize);
        ground.Meshes.Single().Primitives.Single().NormalsGenerated.Should().BeFalse();
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
    public void Scene_set_holds_the_ground_the_actors_and_the_props_models_with_one_texture_per_pack()
    {
        //Arrange
        IReadOnlyList<string> models = ViewerSceneLayout.SceneModels();

        //Act
        ModelDataSet set = Library.BuildSceneSet();

        //Assert
        models.Take(11).Should().Equal("chest", "animal-cat", "animal-dog", "animal-fox", "animal-pig", "animal-cow", "animal-panda", "animal-penguin", "animal-bunny", "avian", "tree");
        set.Models.Select(model => model.Name).Should().Equal(new[] { "ground" }.Concat(models));
        set.Models.Count.Should().Be(21);
        set.Textures.Select(texture => texture.Name).Should().Equal("survival-kit/Textures/colormap.png", "cube-pets/Textures/colormap.png", "crystals-and-avian/Textures/colormap.png");
        set.Models[0].Textures.Should().BeEmpty();
        set.Models.Skip(1).Should().OnlyContain(model => model.Textures.Count == 1);
        set.Models.Single(model => model.Name == "avian").Textures.Should().Equal(2);
        set.Models.Where(model => model.Name.StartsWith("animal-", StringComparison.Ordinal)).Should().OnlyContain(model => model.Textures[0] == 1);
        set.Models.Single(model => model.Name == "chest").Animations.Select(clip => clip.Name).Should().Contain("open-close");
    }

    [Fact]
    public void Texture_cache_shares_one_image_per_colour_space_across_models_of_a_pack()
    {
        //Arrange
        ModelDataSet set = new ModelDataSet();
        ModelTextureCache cache = new ModelTextureCache(set);

        //Act
        Library.AddModel(set, cache, "tree");
        Library.AddModel(set, cache, "rock-a");
        Library.AddModel(set, cache, "animal-cat");
        Library.AddModel(set, cache, "animal-dog");

        //Assert
        set.Textures.Select(texture => texture.Name).Should().Equal("survival-kit/Textures/colormap.png", "cube-pets/Textures/colormap.png");
        cache.Count.Should().Be(2);
        set.Models[2].Textures.Should().Equal(1);
        set.Models[3].Textures.Should().Equal(1);
    }

    [Fact]
    public void Converter_refuses_a_cache_of_another_set()
    {
        //Arrange
        ModelDataSet set = new ModelDataSet();
        ModelTextureCache other = new ModelTextureCache(new ModelDataSet());

        //Act
        Action act = () => GltfModelConverter.Convert(set, GroundModel.CreateGlb(), "ground", null, null, other);

        //Assert
        act.Should().Throw<ArgumentException>().WithMessage("*belongs to another model data set*");
    }

    [Fact]
    public void Blobs_carry_the_format_header_and_are_built_once()
    {
        //Arrange
        ModelBlob scene = Library.SceneBlob;
        ModelBlob tree = Library.GetModelBlob("tree");

        //Act
        ReadOnlySpan<byte> bytes = scene.Bytes;

        //Assert
        bytes.Slice(0, 4).ToArray().Should().Equal(ModelDataFormat.Magic.ToArray());
        BinaryPrimitives.ReadUInt16LittleEndian(bytes.Slice(4)).Should().Be((ushort)ModelDataFormat.MajorVersion);
        BinaryPrimitives.ReadUInt16LittleEndian(bytes.Slice(6)).Should().Be((ushort)ModelDataFormat.MinorVersion);
        BinaryPrimitives.ReadUInt32LittleEndian(bytes.Slice(16)).Should().Be((uint)scene.Length);
        (scene.Length % 16).Should().Be(0);
        Library.SceneBlob.Should().BeSameAs(scene);
        Library.GetModelBlob("tree").Should().BeSameAs(tree);
        scene.ETag.Should().MatchRegex("^\"[0-9a-f]{64}\"$");
        scene.TextureCount.Should().Be(3);
        scene.Models.Select(entry => entry.Index).Should().Equal(Enumerable.Range(0, 21));
        scene.Models[11].Name.Should().Be("tree");
        scene.Models[11].VertexCount.Should().Be(396);
        scene.Models[11].TriangleCount.Should().Be(226);
        tree.Models.Single().Name.Should().Be("tree");
        tree.TextureCount.Should().Be(1);
    }

    [Fact]
    public void Every_embedded_model_converts_without_an_exception_or_a_warning()
    {
        //Arrange
        List<string> warnings = new List<string>();

        //Act
        foreach (string name in Library.ModelNames)
        {
            ModelData model = Library.BuildModelSet(name).Models.Single();
            model.Vertices.Should().NotBeEmpty();
            foreach (string warning in model.Warnings) warnings.Add(name + ": " + warning);
        }

        //Assert
        warnings.Should().BeEmpty();
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
    public void Every_actor_names_a_clip_its_model_has()
    {
        //Arrange
        List<string> missing = new List<string>();

        //Act
        foreach (ViewerActor actor in ViewerSceneLayout.Actors)
        {
            ModelData model = Library.BuildModelSet(actor.Model).Models.Single();
            HashSet<string> clips = model.Animations.Select(clip => clip.Name).ToHashSet(StringComparer.Ordinal);
            foreach (string clip in actor.Routine.Append(actor.Clip).Where(clip => clip != null))
            {
                if (!clips.Contains(clip)) missing.Add(actor.Model + ": " + clip);
            }
        }

        //Assert
        missing.Should().BeEmpty();
    }

    [Fact]
    public void Archer_converts_with_its_skins_and_clips()
    {
        //Arrange
        ModelDataSet set = Library.BuildModelSet("character-archer");

        //Act
        ModelData archer = set.Models.Single();

        //Assert
        archer.Skins.Should().NotBeEmpty();
        archer.SkinVertices.Should().NotBeNull();
        archer.SkinVertices.Length.Should().Be(archer.Vertices.Length);
        archer.Animations.Select(clip => clip.Name).Should().Contain("walk");
        archer.Meshes.SelectMany(mesh => mesh.Primitives).Should().Contain(primitive => primitive.SkinBaseVertex >= 0);
        archer.Warnings.Should().BeEmpty();
        set.Textures.Single().Name.Should().Be("kenney-mini-forest/Textures/colormap.png");
    }
}
