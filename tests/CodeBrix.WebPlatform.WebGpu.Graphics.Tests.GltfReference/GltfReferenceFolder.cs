using System;
using System.Collections.Generic;
using System.Globalization;
using System.IO;
using System.Text;
using System.Threading;

namespace CodeBrix.WebPlatform.WebGpu.Graphics.Tests.GltfReference;

/// <summary>
/// Writes THE REFERENCE FOLDER that the Rust tests of the crate codebrix-webgpu-graphics read to check its glTF loader
/// (src/gltf) against the reference converter (<see cref="GltfModelConverter" />):
/// <para><c>&lt;folder&gt;/models/&lt;pack&gt;/...</c>: every model pack copied as it is (the model files with their
/// external files and licence files in place).</para>
/// <para><c>&lt;folder&gt;/blobs/&lt;case&gt;.cbmd</c>: the ModelDataWriter blob of the one model data set the reference
/// converter produced for that case.</para>
/// <para><c>&lt;folder&gt;/cases.txt</c>: UTF-8 without a byte order mark, LF line ends, a first comment line ('#') that
/// names the fields, then one case per line (<see cref="GltfReferenceCase.ToLine" />):
/// <c>name TAB model paths ('|' between the models of one set) TAB options TAB blob file name</c>, or, when the
/// reference converter refused the files, <c>!</c> and its message in place of the blob file name.</para>
/// The cases: for every model file (.glb, .gltf) of every pack, one with the default options
/// (<c>&lt;pack&gt;.&lt;file name without extension&gt;</c>) and one without textures (<c>.no-textures</c>); for every
/// pack with two or more models one set of all of them (<c>&lt;pack&gt;.set</c>); and for the first model (ordinal
/// order) of each pack whose default conversion holds a texture, one with the largest texture side 64
/// (<c>.max64</c>), one with linear colour textures and no mipmaps (<c>.linear-no-mips</c>) and one without generated
/// tangents (<c>.no-tangents</c>). Every model is converted with its file name (without extension) as its name, its
/// folder below models/ as its texture base key, external files read beside it, one texture cache per case and
/// textures decoded on one thread, so that the output is the same on every run.
/// </summary>
public static class GltfReferenceFolder
{
    /// <summary>The folder beside the assembly of a project that references this library that holds the packs (gltf-reference-models).</summary>
    public const string PacksFolderName = "gltf-reference-models";

    /// <summary>The sub-folder of the reference folder that holds the copied packs.</summary>
    public const string ModelsFolderName = "models";

    /// <summary>The sub-folder of the reference folder that holds the blobs.</summary>
    public const string BlobsFolderName = "blobs";

    /// <summary>The case list's file name in the reference folder.</summary>
    public const string CasesFileName = "cases.txt";

    /// <summary>The extension of a blob file.</summary>
    public const string BlobExtension = ".cbmd";

    /// <summary>The first line of cases.txt: a comment that names the fields.</summary>
    public const string HeaderLine = "# case name TAB model paths (relative to models/, '/' separators, '|' between the models of one set, converted in that order with one shared texture cache) TAB options (textures=<true|false>,mipmaps=<true|false>,srgb=<true|false>,tangents=<true|false>,maxsize=<int>) TAB blob file (relative to blobs/), or '!' and the reference converter's refusal message";

    /// <summary>The packs beside this assembly: <c>&lt;AppContext.BaseDirectory&gt;/gltf-reference-models</c>.</summary>
    public static string DefaultPacksFolder => Path.Combine(AppContext.BaseDirectory, PacksFolderName);

    /// <summary>Writes the reference folder from the packs beside this assembly (<see cref="DefaultPacksFolder" />).</summary>
    /// <param name="folder">The reference folder (created when missing; its models/, blobs/ and cases.txt are replaced).</param>
    /// <param name="cancellationToken">Stops the run between files and between cases.</param>
    /// <returns>The cases, in ordinal order of their names (the order of cases.txt).</returns>
    /// <exception cref="ArgumentNullException"><paramref name="folder" /> is null.</exception>
    /// <exception cref="DirectoryNotFoundException">The packs folder does not exist.</exception>
    /// <exception cref="InvalidOperationException">Two cases would have the same name (rename a model or a pack).</exception>
    public static IReadOnlyList<GltfReferenceCase> Write(string folder, CancellationToken cancellationToken) => Write(folder, DefaultPacksFolder, cancellationToken);

    /// <summary>Writes the reference folder from the packs of <paramref name="packsFolder" /> (one sub-folder per pack).</summary>
    /// <param name="folder">The reference folder (created when missing; its models/, blobs/ and cases.txt are replaced).</param>
    /// <param name="packsFolder">The folder whose sub-folders are the packs (files directly in it are not copied).</param>
    /// <param name="cancellationToken">Stops the run between files and between cases.</param>
    /// <returns>The cases, in ordinal order of their names (the order of cases.txt).</returns>
    /// <exception cref="ArgumentNullException">An argument is null.</exception>
    /// <exception cref="DirectoryNotFoundException"><paramref name="packsFolder" /> does not exist.</exception>
    /// <exception cref="InvalidOperationException">Two cases would have the same name (rename a model or a pack).</exception>
    public static IReadOnlyList<GltfReferenceCase> Write(string folder, string packsFolder, CancellationToken cancellationToken)
    {
        ArgumentNullException.ThrowIfNull(folder);
        ArgumentNullException.ThrowIfNull(packsFolder);
        if (!Directory.Exists(packsFolder)) throw new DirectoryNotFoundException("The model packs folder '" + packsFolder + "' does not exist. Reference the library CodeBrix.WebPlatform.WebGpu.Graphics.Tests.GltfReference, which copies its packs beside the referencing assembly as " + PacksFolderName + "/, or name an existing folder.");
        string models = Path.Combine(folder, ModelsFolderName);
        string blobs = Path.Combine(folder, BlobsFolderName);
        string casesFile = Path.Combine(folder, CasesFileName);
        Directory.CreateDirectory(folder);
        if (Directory.Exists(models)) Directory.Delete(models, true);
        if (Directory.Exists(blobs)) Directory.Delete(blobs, true);
        if (File.Exists(casesFile)) File.Delete(casesFile);
        CopyPacks(packsFolder, models, cancellationToken);
        Directory.CreateDirectory(blobs);

        HashSet<string> used = new HashSet<string>(StringComparer.Ordinal);
        List<GltfReferenceCase> cases = new List<GltfReferenceCase>();
        foreach (string pack in SortedDirectories(models))
        {
            string packName = Sanitize(Path.GetFileName(pack));
            List<string> modelPaths = ModelPaths(models, pack);
            Dictionary<string, string> names = ModelCaseNames(packName, modelPaths);
            string firstTextured = null;
            foreach (string path in modelPaths)
            {
                string[] one = { path };
                GltfReferenceCase main = Convert(models, blobs, names[path], one, DefaultOptions(), used, cancellationToken);
                cases.Add(main);
                ModelConversionOptions noTextures = DefaultOptions();
                noTextures.LoadTextures = false;
                cases.Add(Convert(models, blobs, names[path] + ".no-textures", one, noTextures, used, cancellationToken));
                if (firstTextured == null && !main.IsRefused && main.Statistics.TextureCount > 0) firstTextured = path;
            }

            if (modelPaths.Count >= 2) cases.Add(Convert(models, blobs, packName + ".set", modelPaths, DefaultOptions(), used, cancellationToken));
            if (firstTextured != null)
            {
                string[] one = { firstTextured };
                ModelConversionOptions max64 = DefaultOptions();
                max64.MaxTextureSize = 64;
                cases.Add(Convert(models, blobs, names[firstTextured] + ".max64", one, max64, used, cancellationToken));
                ModelConversionOptions linearNoMips = DefaultOptions();
                linearNoMips.ColorTexturesSrgb = false;
                linearNoMips.Mipmaps = false;
                cases.Add(Convert(models, blobs, names[firstTextured] + ".linear-no-mips", one, linearNoMips, used, cancellationToken));
                ModelConversionOptions noTangents = DefaultOptions();
                noTangents.GenerateTangents = false;
                cases.Add(Convert(models, blobs, names[firstTextured] + ".no-tangents", one, noTangents, used, cancellationToken));
            }
        }

        cases.Sort((left, right) => string.CompareOrdinal(left.Name, right.Name));
        StringBuilder text = new StringBuilder();
        text.Append(HeaderLine).Append('\n');
        foreach (GltfReferenceCase item in cases) text.Append(item.ToLine()).Append('\n');
        File.WriteAllText(casesFile, text.ToString(), new UTF8Encoding(false));
        return cases;
    }

    /// <summary>
    /// The options as cases.txt writes them:
    /// <c>textures=&lt;true|false&gt;,mipmaps=&lt;true|false&gt;,srgb=&lt;true|false&gt;,tangents=&lt;true|false&gt;,maxsize=&lt;int&gt;</c>.
    /// </summary>
    /// <param name="options">The options.</param>
    /// <returns>The text.</returns>
    /// <exception cref="ArgumentNullException"><paramref name="options" /> is null.</exception>
    public static string FormatOptions(ModelConversionOptions options)
    {
        ArgumentNullException.ThrowIfNull(options);
        return "textures=" + Flag(options.LoadTextures) + ",mipmaps=" + Flag(options.Mipmaps) + ",srgb=" + Flag(options.ColorTexturesSrgb) + ",tangents=" + Flag(options.GenerateTangents) + ",maxsize=" + options.MaxTextureSize.ToString(CultureInfo.InvariantCulture);
    }

    /// <summary>The default options of a case: the converter's defaults, textures decoded on one thread.</summary>
    /// <returns>New options.</returns>
    public static ModelConversionOptions DefaultOptions() => new ModelConversionOptions { DecodeTexturesInParallel = false };

    /// <summary>
    /// The source of one model of a case: its file name without extension as its name, its folder below
    /// <paramref name="modelsFolder" /> ('/' separators) as its texture base key, and its external files read beside
    /// it (the relative URI with its percent escapes undone; null when the file does not exist).
    /// </summary>
    /// <param name="modelsFolder">The folder the model path is relative to.</param>
    /// <param name="modelPath">The model's path relative to <paramref name="modelsFolder" />, '/' separators.</param>
    /// <returns>The source.</returns>
    /// <exception cref="ArgumentNullException">An argument is null.</exception>
    public static ModelSource Source(string modelsFolder, string modelPath)
    {
        ArgumentNullException.ThrowIfNull(modelsFolder);
        ArgumentNullException.ThrowIfNull(modelPath);
        string file = Path.Combine(modelsFolder, modelPath.Replace('/', Path.DirectorySeparatorChar));
        string directory = Path.GetDirectoryName(Path.GetFullPath(file));
        int slash = modelPath.LastIndexOf('/');
        string baseKey = slash < 0 ? string.Empty : modelPath.Substring(0, slash);
        byte[] Resolve(string uri)
        {
            string local = Uri.UnescapeDataString(uri).Replace('/', Path.DirectorySeparatorChar).Replace('\\', Path.DirectorySeparatorChar);
            string path = Path.GetFullPath(Path.Combine(directory, local));
            return File.Exists(path) ? File.ReadAllBytes(path) : null;
        }

        return new ModelSource(Path.GetFileNameWithoutExtension(file), File.ReadAllBytes(file), Resolve, baseKey);
    }

    /// <summary>A name made of [a-z0-9-_.]: lower case, every other character replaced by '-'.</summary>
    /// <param name="text">The text.</param>
    /// <returns>The name.</returns>
    /// <exception cref="ArgumentNullException"><paramref name="text" /> is null.</exception>
    public static string Sanitize(string text)
    {
        ArgumentNullException.ThrowIfNull(text);
        StringBuilder result = new StringBuilder(text.Length);
        foreach (char character in text.ToLowerInvariant())
        {
            bool allowed = (character >= 'a' && character <= 'z') || (character >= '0' && character <= '9') || character == '-' || character == '_' || character == '.';
            result.Append(allowed ? character : '-');
        }

        return result.ToString();
    }

    private static string Flag(bool value) => value ? "true" : "false";

    private static GltfReferenceCase Convert(string models, string blobs, string name, IReadOnlyList<string> paths, ModelConversionOptions options, HashSet<string> used, CancellationToken cancellationToken)
    {
        cancellationToken.ThrowIfCancellationRequested();
        if (!used.Add(name)) throw new InvalidOperationException("Two cases of the reference folder would be named '" + name + "' (the second from " + string.Join("|", paths) + "). Rename the model file or the pack.");
        List<ModelSource> sources = new List<ModelSource>(paths.Count);
        foreach (string path in paths) sources.Add(Source(models, path));
        try
        {
            ModelBlob blob = ModelSetConverter.Convert(sources, options);
            string blobFile = name + BlobExtension;
            File.WriteAllBytes(Path.Combine(blobs, blobFile), blob.Bytes);
            return new GltfReferenceCase(name, paths, options, blobFile, null, blob.Statistics);
        }
        catch (InvalidDataException exception)
        {
            return new GltfReferenceCase(name, paths, options, null, exception.Message, null);
        }
    }

    private static void CopyPacks(string packsFolder, string models, CancellationToken cancellationToken)
    {
        Directory.CreateDirectory(models);
        foreach (string pack in SortedDirectories(packsFolder))
        {
            string target = Path.Combine(models, Path.GetFileName(pack));
            foreach (string file in SortedFiles(pack))
            {
                cancellationToken.ThrowIfCancellationRequested();
                string destination = Path.Combine(target, Path.GetRelativePath(pack, file));
                Directory.CreateDirectory(Path.GetDirectoryName(destination));
                File.Copy(file, destination, true);
            }
        }
    }

    private static List<string> ModelPaths(string models, string pack)
    {
        List<string> paths = new List<string>();
        foreach (string file in SortedFiles(pack))
        {
            string extension = Path.GetExtension(file);
            if (!string.Equals(extension, ".glb", StringComparison.OrdinalIgnoreCase) && !string.Equals(extension, ".gltf", StringComparison.OrdinalIgnoreCase)) continue;
            paths.Add(Path.GetRelativePath(models, file).Replace(Path.DirectorySeparatorChar, '/'));
        }

        paths.Sort(StringComparer.Ordinal);
        return paths;
    }

    /// <summary>
    /// The case names of a pack's models: pack + "." + file name without extension; when two models of the pack get the
    /// same name, each of them also names its folders below the pack ("pack.gltf-binary.box"); when that still repeats,
    /// each of those also takes its extension ("-glb", "-gltf").
    /// </summary>
    private static Dictionary<string, string> ModelCaseNames(string packName, List<string> paths)
    {
        Dictionary<string, string> names = new Dictionary<string, string>(StringComparer.Ordinal);
        foreach (string path in paths) names[path] = packName + "." + Sanitize(FileStem(path));
        RenameRepeats(names, path =>
        {
            string[] parts = path.Split('/');
            StringBuilder name = new StringBuilder(packName);
            for (int index = 1; index < parts.Length - 1; index++) name.Append('.').Append(Sanitize(parts[index]));
            return name.Append('.').Append(Sanitize(FileStem(path))).ToString();
        });
        RenameRepeats(names, path => names[path] + "-" + Sanitize(Path.GetExtension(path).TrimStart('.')));
        return names;
    }

    private static void RenameRepeats(Dictionary<string, string> names, Func<string, string> rename)
    {
        Dictionary<string, int> counts = new Dictionary<string, int>(StringComparer.Ordinal);
        foreach (string name in names.Values) counts[name] = counts.TryGetValue(name, out int count) ? count + 1 : 1;
        List<string> repeated = new List<string>();
        foreach (KeyValuePair<string, string> pair in names)
        {
            if (counts[pair.Value] > 1) repeated.Add(pair.Key);
        }

        Dictionary<string, string> renamed = new Dictionary<string, string>(StringComparer.Ordinal);
        foreach (string path in repeated) renamed[path] = rename(path);
        foreach (KeyValuePair<string, string> pair in renamed) names[pair.Key] = pair.Value;
    }

    private static string FileStem(string path) => Path.GetFileNameWithoutExtension(path.Substring(path.LastIndexOf('/') + 1));

    private static List<string> SortedDirectories(string folder)
    {
        List<string> directories = new List<string>(Directory.GetDirectories(folder));
        directories.Sort(StringComparer.Ordinal);
        return directories;
    }

    private static List<string> SortedFiles(string folder)
    {
        List<string> files = new List<string>(Directory.GetFiles(folder, "*", SearchOption.AllDirectories));
        files.Sort(StringComparer.Ordinal);
        return files;
    }
}
