using System.Linq;
using CodeBrix.WebPlatform.WebGpu.LibraryChecker.Checks;
using SilverAssertions;
using Xunit;

namespace CodeBrix.WebPlatform.WebGpu.LibraryChecker.Tests.Checks;

public class SampleNoticesCheckTests
{
    private const string Notices = """
        Survival Kit - created and distributed by Kenney
          Licence:  CC0 1.0 Universal.
                    backend/GraphicsDemo.Assets/Models/survival-kit/LICENSE-Kenney-survival-kit.txt
          Files:    backend/GraphicsDemo.Assets/Models/survival-kit/: barrel, tree (.glb)
        """;

    private static readonly SampleNoticesCheck.Pack Kit = new("survival-kit", new[] { "LICENSE-Kenney-survival-kit.txt", "Textures/colormap.png", "barrel.glb", "tree.glb" });

    [Fact]
    public void Check_passes_when_every_pack_its_licence_file_and_its_models_are_named()
    {
        //Act
        var result = SampleNoticesCheck.Check(Notices, "notices.txt", new[] { Kit });

        //Assert
        result.Passed.Should().BeTrue(string.Join("; ", result.Problems));
        result.Notes.Single().Should().Be("survival-kit: 2 model or panorama file(s), licence LICENSE-Kenney-survival-kit.txt");
    }

    [Fact]
    public void Check_fails_for_a_pack_without_a_licence_file_and_says_to_add_it()
    {
        //Arrange
        var pack = new SampleNoticesCheck.Pack("survival-kit", new[] { "barrel.glb", "tree.glb" });

        //Act
        var result = SampleNoticesCheck.Check(Notices, "notices.txt", new[] { pack });

        //Assert
        result.Problems.Single().Should().Contain("has no licence file").And.Contain("add it, or remove the pack");
    }

    [Fact]
    public void Check_fails_for_a_model_the_notices_do_not_list()
    {
        //Arrange
        var pack = Kit with { Files = Kit.Files.Append("rock-a.glb").ToList() };

        //Act
        var result = SampleNoticesCheck.Check(Notices, "notices.txt", new[] { pack });

        //Assert
        result.Problems.Single().Should().Contain("does not name the file survival-kit/rock-a.glb");
    }

    [Fact]
    public void Check_fails_for_an_unnamed_pack_and_an_unnamed_licence_path()
    {
        //Arrange
        var pack = new SampleNoticesCheck.Pack("cube-pets", new[] { "LICENSE-Kenney-cube-pets.txt" });

        //Act
        var result = SampleNoticesCheck.Check(Notices, "notices.txt", new[] { pack });

        //Assert
        result.Problems.Should().HaveCount(2);
        result.Problems[0].Should().Contain("does not name the pack 'cube-pets'");
        result.Problems[1].Should().Contain("Models/cube-pets/LICENSE-Kenney-cube-pets.txt");
    }

    [Fact]
    public void Check_covers_a_panorama_pack_in_the_environments_folder()
    {
        //Arrange
        var panorama = new SampleNoticesCheck.Pack("polyhaven-small-cathedral", new[] { "LICENSE-PolyHaven-small_cathedral.txt", "small_cathedral_1k.hdr" }, SampleNoticesCheck.EnvironmentsFolder);
        var listed = Notices + "Small Cathedral: backend/GraphicsDemo.Assets/Environments/polyhaven-small-cathedral/LICENSE-PolyHaven-small_cathedral.txt, small_cathedral_1k.hdr";

        //Act
        var passed = SampleNoticesCheck.Check(listed, "notices.txt", new[] { Kit, panorama });
        var failed = SampleNoticesCheck.Check(Notices, "notices.txt", new[] { Kit, panorama });

        //Assert
        passed.Passed.Should().BeTrue(string.Join("; ", passed.Problems));
        failed.Problems.Should().HaveCount(3);
        failed.Problems[1].Should().Contain("Environments/polyhaven-small-cathedral/LICENSE-PolyHaven-small_cathedral.txt");
        failed.Problems[2].Should().Contain("does not name the file polyhaven-small-cathedral/small_cathedral_1k.hdr");
    }

    [Fact]
    public void Check_asks_nothing_of_the_owners_own_pack_and_notes_it()
    {
        //Arrange
        var avian = new SampleNoticesCheck.Pack("crystals-and-avian", new[] { "Textures/colormap.png", "avian.glb" });

        //Act
        var result = SampleNoticesCheck.Check(Notices, "notices.txt", new[] { Kit, avian });

        //Assert
        result.Passed.Should().BeTrue(string.Join("; ", result.Problems));
        result.Notes.Should().Contain("crystals-and-avian: the repository owner's own work (OwnersOwnPacks) - no licence file or notices entry needed");
    }

    [Fact]
    public void Only_the_named_packs_are_the_owners_own()
    {
        //Arrange
        var other = new SampleNoticesCheck.Pack("crystals-and-other", new[] { "other.glb" });

        //Act
        var result = SampleNoticesCheck.Check(Notices, "notices.txt", new[] { other });

        //Assert
        SampleNoticesCheck.OwnersOwnPacks.Should().Equal("crystals-and-avian");
        result.Passed.Should().BeFalse();
        result.Problems.Should().Contain(problem => problem.Contains("has no licence file"));
    }

    [Fact]
    public void Check_fails_when_the_notices_file_is_missing()
        => SampleNoticesCheck.Check(null, "notices.txt", new[] { Kit }).Problems.Single().Should().Contain("does not exist").And.Contain("write it");

    [Fact]
    public void Check_passes_with_a_note_when_the_sample_carries_no_packs()
        => SampleNoticesCheck.Check(null, "notices.txt", System.Array.Empty<SampleNoticesCheck.Pack>()).Passed.Should().BeTrue();

    [Fact]
    public void Check_covers_a_font_pack_whose_licence_file_is_its_original_OFL_txt()
    {
        //Arrange
        const string notices = """
            Roboto Regular
              Licence:  SIL Open Font License, Version 1.1.
                        backend/GraphicsDemo.Assets/Fonts/roboto/OFL.txt
              Files:    backend/GraphicsDemo.Assets/Fonts/roboto/: Roboto-Regular.ttf.
            """;
        var pack = new SampleNoticesCheck.Pack("roboto", new[] { "OFL.txt", "Roboto-Regular.ttf" }, SampleNoticesCheck.FontsFolder);

        //Act
        var result = SampleNoticesCheck.Check(notices, "notices.txt", new[] { pack });
        var unlisted = SampleNoticesCheck.Check(notices, "notices.txt", new[] { pack with { Files = pack.Files.Append("Roboto-Bold.ttf").ToList() } });
        var bare = SampleNoticesCheck.Check(notices, "notices.txt", new[] { pack with { Files = new[] { "Roboto-Regular.ttf" } } });

        //Assert
        result.Passed.Should().BeTrue(string.Join("; ", result.Problems));
        result.Notes.Single().Should().Be("roboto: 1 font file(s), licence OFL.txt");
        unlisted.Problems.Single().Should().Contain("does not name the file roboto/Roboto-Bold.ttf");
        bare.Problems.Single().Should().Contain("has no licence file beside its fonts").And.Contain("ORIGINAL licence file");
    }
}
