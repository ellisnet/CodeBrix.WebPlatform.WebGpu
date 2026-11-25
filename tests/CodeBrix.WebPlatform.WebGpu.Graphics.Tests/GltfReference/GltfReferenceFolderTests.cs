using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using System.Text;
using System.Text.RegularExpressions;
using CodeBrix.WebPlatform.WebGpu.Graphics.Tests.Models;
using SilverAssertions;
using Xunit;

namespace CodeBrix.WebPlatform.WebGpu.Graphics.Tests.GltfReference;

public class GltfReferenceFolderTests : IClassFixture<GltfReferenceFolderFixture>
{
    /// <summary>
    /// The environment variable that names a folder to write the reference folder into (for the Rust tests or a look
    /// by hand); when it is not set, the test that writes there is skipped.
    /// </summary>
    private const string FolderVariable = "CODEBRIX_GLTF_REFERENCE_FOLDER";

    private const string LinePattern = @"^[a-z0-9\-_.]+\t[^\t|]+(\|[^\t|]+)*\ttextures=(true|false),mipmaps=(true|false),srgb=(true|false),tangents=(true|false),maxsize=[0-9]+\t([a-z0-9\-_.]+\.cbmd|![^\t]*)$";

    private readonly GltfReferenceFolderFixture fixture;

    public GltfReferenceFolderTests(GltfReferenceFolderFixture fixture)
    {
        this.fixture = fixture;
    }

    [Fact]
    public void Write_cases_txt_is_utf8_without_bom_with_lf_line_ends_and_the_header_first()
    {
        //Arrange
        var bytes = File.ReadAllBytes(Path.Combine(fixture.FirstFolder, GltfReferenceFolder.CasesFileName));

        //Act
        var text = Encoding.UTF8.GetString(bytes);

        //Assert
        bytes[0].Should().Be((byte)'#');
        text.Should().NotContain("\r");
        text.Should().EndWith("\n");
        text.Split('\n')[0].Should().Be(GltfReferenceFolder.HeaderLine);
        GltfReferenceFolder.HeaderLine.Should().StartWith("# ");
    }

    [Fact]
    public void Write_every_case_line_has_the_four_tab_separated_fields()
    {
        //Arrange
        var lines = CaseLines(fixture.FirstFolder);

        //Act
        var names = lines.Select(line => line.Split('\t')[0]).ToList();

        //Assert
        lines.Should().NotBeEmpty();
        lines.Should().OnlyContain(line => Regex.IsMatch(line, LinePattern));
        names.Should().OnlyHaveUniqueItems();
        names.Should().Equal(names.OrderBy(name => name, StringComparer.Ordinal));
        lines.Should().Equal(fixture.FirstCases.Select(item => item.ToLine()));
    }

    [Fact]
    public void Write_every_model_file_has_a_default_case_and_a_no_textures_case()
    {
        //Arrange
        var models = ModelFiles(GltfReferenceFolder.DefaultPacksFolder);
        var single = fixture.FirstCases.Where(item => item.ModelPaths.Count == 1).ToList();

        //Act
        var simple = models.Select(path => GltfReferenceFolder.Sanitize(path.Split('/')[0]) + "." + GltfReferenceFolder.Sanitize(Path.GetFileNameWithoutExtension(path))).ToList();

        //Assert
        models.Should().Contain("survival-kit/barrel.glb");
        for (var index = 0; index < models.Count; index++)
        {
            // The case's name is the pack and the file name; two models of a pack with one file name (the
            // variants of one model in sibling folders) also name their folders, so the name only STARTS with the pack.
            var ofModel = single.Where(item => item.ModelPaths[0] == models[index]).ToList();
            var plain = ofModel.Single(item => item.OptionsText == "textures=true,mipmaps=true,srgb=true,tangents=true,maxsize=0");
            plain.Name.Should().StartWith(GltfReferenceFolder.Sanitize(models[index].Split('/')[0]) + ".");
            if (simple.Count(name => name == simple[index]) == 1)
            {
                plain.Name.Should().Be(simple[index]);
            }

            var bare = ofModel.Single(item => item.OptionsText == "textures=false,mipmaps=true,srgb=true,tangents=true,maxsize=0");
            bare.Name.Should().Be(plain.Name + ".no-textures");
        }

        fixture.FirstCases.Select(item => item.Name).Should().OnlyHaveUniqueItems();
    }

    [Fact]
    public void Write_each_pack_with_several_models_has_one_set_case_of_all_of_them_in_ordinal_order()
    {
        //Arrange
        var models = ModelFiles(GltfReferenceFolder.DefaultPacksFolder);
        var packs = models.GroupBy(path => path.Split('/')[0]).Where(group => group.Count() >= 2).ToList();
        var refused = fixture.FirstCases.Where(item => item.ModelPaths.Count == 1 && item.IsRefused).Select(item => item.ModelPaths[0]).ToHashSet(StringComparer.Ordinal);

        //Act
        var sets = fixture.FirstCases.Where(item => item.Name.EndsWith(".set", StringComparison.Ordinal)).ToList();

        //Assert
        sets.Should().HaveCount(packs.Count);
        foreach (var pack in packs)
        {
            var set = sets.Single(item => item.Name == pack.Key + ".set");
            set.ModelPaths.Should().Equal(pack.OrderBy(path => path, StringComparer.Ordinal));
            // A set is refused exactly when one of its models is (the pack of Draco-compressed models is).
            set.IsRefused.Should().Be(pack.Any(refused.Contains));
        }

        sets.Single(item => item.Name == "survival-kit.set").Statistics.TextureCount.Should().Be(1);
        sets.Single(item => item.Name == "khronos-sample-assets.set").IsRefused.Should().BeFalse();
        sets.Single(item => item.Name == "khronos-sample-assets-draco.set").Refusal.Should().Contain("KHR_draco_mesh_compression");
    }

    [Fact]
    public void Write_the_first_textured_model_of_a_pack_has_the_three_option_cases()
    {
        //Arrange
        var cases = fixture.FirstCases.ToDictionary(item => item.Name, StringComparer.Ordinal);

        //Act
        var max64 = cases["survival-kit.barrel.max64"];
        var linear = cases["survival-kit.barrel.linear-no-mips"];
        var noTangents = cases["survival-kit.barrel.no-tangents"];

        //Assert
        max64.OptionsText.Should().Be("textures=true,mipmaps=true,srgb=true,tangents=true,maxsize=64");
        linear.OptionsText.Should().Be("textures=true,mipmaps=false,srgb=false,tangents=true,maxsize=0");
        noTangents.OptionsText.Should().Be("textures=true,mipmaps=true,srgb=true,tangents=false,maxsize=0");
        cases.Keys.Where(name => name.EndsWith(".max64", StringComparison.Ordinal)).Select(name => name.Split('.')[0]).Should().OnlyHaveUniqueItems();
        cases.Should().NotContainKey("survival-kit.campfire-pit.max64");
    }

    [Fact]
    public void Write_every_converted_case_has_a_blob_the_test_reader_reads_back()
    {
        //Arrange
        var blobs = Path.Combine(fixture.FirstFolder, GltfReferenceFolder.BlobsFolderName);
        var converted = fixture.FirstCases.Where(item => !item.IsRefused).ToList();

        //Act
        var files = Directory.GetFiles(blobs).Select(Path.GetFileName).OrderBy(name => name, StringComparer.Ordinal).ToList();

        //Assert
        files.Should().Equal(converted.Select(item => item.BlobFileName).OrderBy(name => name, StringComparer.Ordinal));
        foreach (var item in converted)
        {
            var bytes = File.ReadAllBytes(Path.Combine(blobs, item.BlobFileName));
            bytes.Take(4).Should().Equal("CBMD"u8.ToArray());
            bytes.Length.Should().Be((int)item.Statistics.BlobBytes);
            var set = new ModelDataTestReader(bytes).Read();
            set.Models.Select(model => model.Name).Should().Equal(item.ModelPaths.Select(Path.GetFileNameWithoutExtension));
            set.Models.Should().OnlyContain(model => model.Vertices.Length > 0);
            set.Textures.Should().HaveCount(item.Statistics.TextureCount);
        }
    }

    [Fact]
    public void Write_twice_gives_byte_identical_cases_and_blobs()
    {
        //Arrange
        var firstBlobs = Path.Combine(fixture.FirstFolder, GltfReferenceFolder.BlobsFolderName);
        var secondBlobs = Path.Combine(fixture.SecondFolder, GltfReferenceFolder.BlobsFolderName);

        //Act
        var firstCases = File.ReadAllBytes(Path.Combine(fixture.FirstFolder, GltfReferenceFolder.CasesFileName));
        var secondCases = File.ReadAllBytes(Path.Combine(fixture.SecondFolder, GltfReferenceFolder.CasesFileName));

        //Assert
        secondCases.Should().Equal(firstCases);
        Directory.GetFiles(secondBlobs).Select(Path.GetFileName).Should().BeEquivalentTo(Directory.GetFiles(firstBlobs).Select(Path.GetFileName));
        foreach (var file in Directory.GetFiles(firstBlobs))
        {
            File.ReadAllBytes(Path.Combine(secondBlobs, Path.GetFileName(file))).Should().Equal(File.ReadAllBytes(file));
        }
    }

    [Fact]
    public void Write_copies_every_pack_file_into_models()
    {
        //Arrange
        var packs = GltfReferenceFolder.DefaultPacksFolder;
        var models = Path.Combine(fixture.FirstFolder, GltfReferenceFolder.ModelsFolderName);

        //Act
        var copied = RelativeFiles(models);

        //Assert
        copied.Should().Equal(RelativeFiles(packs));
        copied.Should().Contain("survival-kit/Textures/colormap.png");
        copied.Should().Contain("survival-kit/LICENSE-Kenney-survival-kit.txt");
        foreach (var file in copied)
        {
            File.ReadAllBytes(Path.Combine(models, file)).Should().Equal(File.ReadAllBytes(Path.Combine(packs, file)));
        }
    }

    [Fact]
    public void Write_refused_models_are_bang_lines_and_repeated_names_take_their_folders()
    {
        //Arrange
        var root = Path.Combine(Path.GetTempPath(), "codebrix-gltf-reference-" + Guid.NewGuid().ToString("N"));
        var packs = Path.Combine(root, "packs");
        var folder = Path.Combine(root, "reference");
        Directory.CreateDirectory(Path.Combine(packs, "Box", "glTF"));
        Directory.CreateDirectory(Path.Combine(packs, "Box", "glTF-Binary"));
        File.WriteAllText(Path.Combine(packs, "Box", "glTF", "Box.gltf"), "{ not\tjson");
        File.WriteAllBytes(Path.Combine(packs, "Box", "glTF-Binary", "Box.glb"), new byte[] { 1, 2, 3, 4 });
        File.WriteAllText(Path.Combine(packs, "Box", "SOURCE.txt"), "invented for this test");

        try
        {
            //Act
            var cases = GltfReferenceFolder.Write(folder, packs, TestContext.Current.CancellationToken);

            //Assert
            cases.Select(item => item.Name).Should().Equal("box.gltf-binary.box", "box.gltf-binary.box.no-textures", "box.gltf.box", "box.gltf.box.no-textures", "box.set");
            cases.Should().OnlyContain(item => item.IsRefused && item.BlobFileName == null);
            var lines = CaseLines(folder);
            lines.Should().OnlyContain(line => Regex.IsMatch(line, LinePattern));
            lines[0].Should().StartWith("box.gltf-binary.box\tBox/glTF-Binary/Box.glb\ttextures=true,mipmaps=true,srgb=true,tangents=true,maxsize=0\t!");
            lines[4].Should().StartWith("box.set\tBox/glTF-Binary/Box.glb|Box/glTF/Box.gltf\t");
            Directory.GetFiles(Path.Combine(folder, GltfReferenceFolder.BlobsFolderName)).Should().BeEmpty();
            File.Exists(Path.Combine(folder, GltfReferenceFolder.ModelsFolderName, "Box", "SOURCE.txt")).Should().BeTrue();
        }
        finally
        {
            Directory.Delete(root, true);
        }
    }

    [Fact]
    public void Write_to_the_folder_named_by_the_environment_variable()
    {
        //Arrange
        var folder = Environment.GetEnvironmentVariable(FolderVariable);
        Assert.SkipWhen(string.IsNullOrEmpty(folder), FolderVariable + " is not set: set it to a folder to write the reference folder there.");

        //Act
        var cases = GltfReferenceFolder.Write(folder, TestContext.Current.CancellationToken);

        //Assert
        cases.Should().NotBeEmpty();
        File.Exists(Path.Combine(folder, GltfReferenceFolder.CasesFileName)).Should().BeTrue();
    }

    private static List<string> CaseLines(string folder)
    {
        var text = File.ReadAllText(Path.Combine(folder, GltfReferenceFolder.CasesFileName), Encoding.UTF8);
        return text.Split('\n').Where(line => line.Length > 0 && !line.StartsWith('#')).ToList();
    }

    private static List<string> ModelFiles(string packs) => RelativeFiles(packs)
        .Where(path => path.EndsWith(".glb", StringComparison.OrdinalIgnoreCase) || path.EndsWith(".gltf", StringComparison.OrdinalIgnoreCase))
        .ToList();

    private static List<string> RelativeFiles(string folder) => Directory.GetFiles(folder, "*", SearchOption.AllDirectories)
        .Select(file => Path.GetRelativePath(folder, file).Replace(Path.DirectorySeparatorChar, '/'))
        .OrderBy(path => path, StringComparer.Ordinal)
        .ToList();
}
