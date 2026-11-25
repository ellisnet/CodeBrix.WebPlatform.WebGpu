using System;
using System.Collections.Concurrent;
using System.Collections.Generic;
using System.IO;
using System.Reflection;
using System.Threading;
using CodeBrix.WebPlatform.WebGpu.Graphics.Models;
using GraphicsDemo.Assets.BuiltModels;

namespace GraphicsDemo.Assets;

/// <summary>
/// The sample's models. Two kinds:
/// <list type="bullet">
/// <item>MODEL FILES, embedded in this assembly (Models/&lt;pack&gt;/&lt;name&gt;.glb, each pack's shared
/// Textures/colormap.png beside them) and SERVED UNCHANGED (<see cref="GetFile" />): the frontend fetches them and the
/// graphics crate's glTF loader reads them in the browser. Nothing is converted here.</item>
/// <item>MODELS BUILT IN CODE, which have no file: the ground (<see cref="GroundModel" />) and the morph-target shape
/// (<see cref="MorphShapeModel" />), made as model data and written as model data blobs on first use
/// (<see cref="GetModelBlob" />), which the frontend reads with the graphics crate's blob reader.</item>
/// </list>
/// Every member is safe to call from several threads at once; each file is read and each blob written once.
/// </summary>
public sealed class ModelLibrary
{
    /// <summary>The names of the models built in code (served as blobs), in ordinal order.</summary>
    public static IReadOnlyList<string> BuiltModelNames { get; } = new[] { GroundModel.Name, MorphShapeModel.Name };

    private const string ResourcePrefix = "Models/";

    /// <summary>The content types of the files a model is made of; a file of any other kind (a licence file) is not served.</summary>
    private static readonly Dictionary<string, string> ContentTypes = new Dictionary<string, string>(StringComparer.OrdinalIgnoreCase)
    {
        [".glb"] = "model/gltf-binary",
        [".gltf"] = "model/gltf+json",
        [".bin"] = "application/octet-stream",
        [".png"] = "image/png",
        [".jpg"] = "image/jpeg",
        [".jpeg"] = "image/jpeg",
    };

    private readonly Assembly assembly = typeof(ModelLibrary).Assembly;
    private readonly Dictionary<string, string> resources = new Dictionary<string, string>(StringComparer.Ordinal);
    private readonly Dictionary<string, string> modelFiles = new Dictionary<string, string>(StringComparer.Ordinal);
    private readonly ConcurrentDictionary<string, Lazy<ModelFile>> files = new ConcurrentDictionary<string, Lazy<ModelFile>>(StringComparer.Ordinal);
    private readonly ConcurrentDictionary<string, Lazy<ModelBlob>> blobs = new ConcurrentDictionary<string, Lazy<ModelBlob>>(StringComparer.Ordinal);

    /// <summary>Finds the embedded model files (nothing is read yet).</summary>
    /// <exception cref="InvalidOperationException">Two embedded models have the same name.</exception>
    public ModelLibrary()
    {
        List<string> names = new List<string>(BuiltModelNames);
        foreach (string resource in assembly.GetManifestResourceNames())
        {
            // The resource name is the path below Models/; '/' on every build machine.
            string path = resource.Replace('\\', '/');
            if (!path.StartsWith(ResourcePrefix, StringComparison.Ordinal)) continue;
            path = path.Substring(ResourcePrefix.Length);
            resources[path] = resource;
            // A model is a .glb or .gltf file directly in its pack's folder; the other files are what models name.
            int slash = path.IndexOf('/');
            string extension = Path.GetExtension(path);
            bool model = extension.Equals(".glb", StringComparison.OrdinalIgnoreCase) || extension.Equals(".gltf", StringComparison.OrdinalIgnoreCase);
            if (slash <= 0 || path.IndexOf('/', slash + 1) >= 0 || !model) continue;
            string name = Path.GetFileNameWithoutExtension(path);
            if (modelFiles.ContainsKey(name) || IsBuilt(name))
            {
                throw new InvalidOperationException("Two models are named '" + name + "' ('" + path + "' and '" + (modelFiles.TryGetValue(name, out string other) ? other : "a model built in code") + "'). Rename one: the library finds models by name.");
            }

            modelFiles[name] = path;
            names.Add(name);
        }

        names.Sort(StringComparer.Ordinal);
        ModelNames = names;
    }

    /// <summary>The names of every model - the model files and the models built in code - in ordinal order.</summary>
    public IReadOnlyList<string> ModelNames { get; }

    /// <summary>True when <paramref name="name" /> is one of <see cref="ModelNames" />.</summary>
    /// <param name="name">The model name.</param>
    /// <returns>True for a known model.</returns>
    public bool Contains(string name) => name != null && (IsBuilt(name) || modelFiles.ContainsKey(name));

    /// <summary>True when <paramref name="name" /> is a model built in code (served as a blob, not as a file).</summary>
    /// <param name="name">The model name.</param>
    /// <returns>True for the ground and the morph shape.</returns>
    public static bool IsBuilt(string name) => name == GroundModel.Name || name == MorphShapeModel.Name;

    /// <summary>The path below Models/ of a model's file (for example "survival-kit/tree.glb"), or null for a model built in code or an unknown name.</summary>
    /// <param name="name">The model name.</param>
    /// <returns>The file's path, or null.</returns>
    public string FileOf(string name) => name != null && modelFiles.TryGetValue(name, out string path) ? path : null;

    /// <summary>The pack (the folder below Models/, the base address of the file's own references) a model file is in, or null for a model built in code or an unknown name.</summary>
    /// <param name="name">The model name.</param>
    /// <returns>The pack's folder name, or null.</returns>
    public string PackOf(string name)
    {
        string path = FileOf(name);
        return path == null ? null : path.Substring(0, path.IndexOf('/'));
    }

    /// <summary>
    /// A file of a model, by its path below Models/ (for example "survival-kit/tree.glb" or
    /// "survival-kit/Textures/colormap.png"), read once and kept; null when there is no such embedded file or it is not
    /// a kind a model is made of (.glb, .gltf, .bin, .png, .jpg, .jpeg - a pack's licence file is not served). The path
    /// is looked up among the embedded files as it is written (case-sensitive, '/' separators); no file system path is
    /// ever formed from it.
    /// </summary>
    /// <param name="path">The file's path below Models/.</param>
    /// <returns>The file, or null.</returns>
    public ModelFile GetFile(string path)
    {
        if (path == null || !resources.ContainsKey(path) || !ContentTypes.TryGetValue(Path.GetExtension(path), out string contentType)) return null;
        return files.GetOrAdd(path, key => new Lazy<ModelFile>(() => new ModelFile(key, ReadResource(key), contentType), LazyThreadSafetyMode.ExecutionAndPublication)).Value;
    }

    /// <summary>The blob of a model built in code (a set of one), or null for a model file or an unknown name.</summary>
    /// <param name="name">The model name (<see cref="BuiltModelNames" />).</param>
    /// <returns>The blob, or null.</returns>
    public ModelBlob GetModelBlob(string name)
    {
        if (!IsBuilt(name)) return null;
        return blobs.GetOrAdd(name, key => new Lazy<ModelBlob>(() => Write(BuildModelSet(key)), LazyThreadSafetyMode.ExecutionAndPublication)).Value;
    }

    /// <summary>Builds a model built in code into a new set of its own.</summary>
    /// <param name="name">The model name (<see cref="BuiltModelNames" />).</param>
    /// <returns>The set.</returns>
    /// <exception cref="ArgumentException">There is no model of that name built in code.</exception>
    public static ModelDataSet BuildModelSet(string name)
    {
        ModelDataSet set = new ModelDataSet();
        if (name == GroundModel.Name) GroundModel.AddTo(set);
        else if (name == MorphShapeModel.Name) MorphShapeModel.AddTo(set);
        else throw new ArgumentException("There is no model named '" + name + "' built in code. Use one of ModelLibrary.BuiltModelNames.", nameof(name));
        return set;
    }

    /// <summary>Reads an embedded file by its path below Models/ (for example "survival-kit/Textures/colormap.png").</summary>
    /// <param name="path">The path.</param>
    /// <returns>The bytes, or null when there is no such file.</returns>
    public byte[] ReadResource(string path)
    {
        if (path == null || !resources.TryGetValue(path, out string resource)) return null;
        using Stream stream = assembly.GetManifestResourceStream(resource);
        if (stream == null) return null;
        using MemoryStream copy = new MemoryStream();
        stream.CopyTo(copy);
        return copy.ToArray();
    }

    private static ModelBlob Write(ModelDataSet set)
    {
        byte[] bytes = ModelDataWriter.Write(set);
        List<ModelBlobEntry> entries = new List<ModelBlobEntry>(set.Models.Count);
        for (int index = 0; index < set.Models.Count; index++)
        {
            ModelData model = set.Models[index];
            long triangles = 0;
            foreach (ModelMeshData mesh in model.Meshes)
            {
                foreach (ModelPrimitiveData primitive in mesh.Primitives) triangles += primitive.IndexCount / 3;
            }

            entries.Add(new ModelBlobEntry(model.Name, index, model.Vertices.Length, (int)triangles, model.Warnings.ToArray()));
        }

        return new ModelBlob(bytes, entries, set.Textures.Count);
    }
}
