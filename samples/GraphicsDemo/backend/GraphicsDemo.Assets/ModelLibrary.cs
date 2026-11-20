using System;
using System.Collections.Concurrent;
using System.Collections.Generic;
using System.IO;
using System.Reflection;
using System.Threading;
using CodeBrix.WebPlatform.WebGpu.Graphics.Models;
using GraphicsDemo.Assets.Conversion;
using GraphicsDemo.Assets.Scene;

namespace GraphicsDemo.Assets;

/// <summary>
/// The sample's models: the .glb files embedded in this assembly (Models/&lt;pack&gt;/&lt;name&gt;.glb, each pack's
/// shared Textures/colormap.png beside them), the ground built in memory as a glTF file, and the morph-target shape
/// built in code as model data (<see cref="MorphShapeModel" />), converted to model data blobs on first use and kept. Every member is safe to call from several threads at once; each blob is built once.
/// </summary>
public sealed class ModelLibrary
{
    private const string ResourcePrefix = "Models/";

    private readonly Assembly assembly = typeof(ModelLibrary).Assembly;
    private readonly Dictionary<string, string> resources = new Dictionary<string, string>(StringComparer.Ordinal);
    private readonly Dictionary<string, string> packs = new Dictionary<string, string>(StringComparer.Ordinal);
    private readonly ConcurrentDictionary<string, Lazy<ModelBlob>> modelBlobs = new ConcurrentDictionary<string, Lazy<ModelBlob>>(StringComparer.Ordinal);
    private readonly Lazy<ModelBlob> sceneBlob;

    /// <summary>Finds the embedded models (nothing is converted yet).</summary>
    public ModelLibrary()
    {
        List<string> names = new List<string> { GroundModel.Name, MorphShapeModel.Name };
        foreach (string resource in assembly.GetManifestResourceNames())
        {
            // The resource name is the path below Models/; '/' on every build machine.
            string path = resource.Replace('\\', '/');
            if (!path.StartsWith(ResourcePrefix, StringComparison.Ordinal)) continue;
            path = path.Substring(ResourcePrefix.Length);
            resources[path] = resource;
            int slash = path.IndexOf('/');
            if (slash <= 0 || path.IndexOf('/', slash + 1) >= 0 || !path.EndsWith(".glb", StringComparison.Ordinal)) continue;
            string name = path.Substring(slash + 1, path.Length - slash - 1 - ".glb".Length);
            if (packs.ContainsKey(name)) throw new InvalidOperationException("Two embedded models are named '" + name + "' (in '" + packs[name] + "' and '" + path.Substring(0, slash) + "'). Rename one: the library finds models by name.");
            packs[name] = path.Substring(0, slash);
            names.Add(name);
        }

        names.Sort(StringComparer.Ordinal);
        ModelNames = names;
        sceneBlob = new Lazy<ModelBlob>(() => Write(BuildSceneSet()), LazyThreadSafetyMode.ExecutionAndPublication);
    }

    /// <summary>The names of every model, the ground included, in ordinal order.</summary>
    public IReadOnlyList<string> ModelNames { get; }

    /// <summary>The blob of the Viewer scene: the ground and every model the scene places (actors and props), each texture stored once.</summary>
    public ModelBlob SceneBlob => sceneBlob.Value;

    /// <summary>True when <paramref name="name" /> is one of <see cref="ModelNames" />.</summary>
    /// <param name="name">The model name.</param>
    /// <returns>True for a known model.</returns>
    public bool Contains(string name) => name != null && (name == GroundModel.Name || name == MorphShapeModel.Name || packs.ContainsKey(name));

    /// <summary>The pack (the folder below Models/) a model comes from, or null for the ground, the morph shape or an unknown name.</summary>
    /// <param name="name">The model name.</param>
    /// <returns>The pack's folder name, or null.</returns>
    public string PackOf(string name) => name != null && packs.TryGetValue(name, out string pack) ? pack : null;

    /// <summary>The blob of one model (a set of one), or null when there is no model of that name.</summary>
    /// <param name="name">The model name.</param>
    /// <returns>The blob, or null.</returns>
    public ModelBlob GetModelBlob(string name)
    {
        if (!Contains(name)) return null;
        return modelBlobs.GetOrAdd(name, key => new Lazy<ModelBlob>(() => Write(BuildModelSet(key)), LazyThreadSafetyMode.ExecutionAndPublication)).Value;
    }

    /// <summary>Converts one model into a new set of its own.</summary>
    /// <param name="name">The model name.</param>
    /// <returns>The set.</returns>
    /// <exception cref="ArgumentException">There is no model of that name.</exception>
    public ModelDataSet BuildModelSet(string name)
    {
        ModelDataSet set = new ModelDataSet();
        AddModel(set, new ModelTextureCache(set), name);
        return set;
    }

    /// <summary>
    /// Converts the Viewer scene's models into one set: the ground first, then the actors' and the props' models in the
    /// order of their first use (<see cref="ViewerSceneLayout.SceneModels" />), their shared textures stored once.
    /// </summary>
    /// <returns>The set.</returns>
    public ModelDataSet BuildSceneSet()
    {
        ModelDataSet set = new ModelDataSet();
        ModelTextureCache cache = new ModelTextureCache(set);
        GroundModel.AddTo(set);
        foreach (string name in ViewerSceneLayout.SceneModels()) AddModel(set, cache, name);
        return set;
    }

    /// <summary>Converts one model into <paramref name="set" />, sharing textures through <paramref name="cache" />.</summary>
    /// <param name="set">The set.</param>
    /// <param name="cache">The set's texture cache, or null.</param>
    /// <param name="name">The model name.</param>
    /// <returns>The model.</returns>
    /// <exception cref="ArgumentException">There is no model of that name.</exception>
    public ModelData AddModel(ModelDataSet set, ModelTextureCache cache, string name)
    {
        ArgumentNullException.ThrowIfNull(set);
        if (name == GroundModel.Name) return GroundModel.AddTo(set);
        if (name == MorphShapeModel.Name) return MorphShapeModel.AddTo(set);
        string pack = PackOf(name);
        if (pack == null) throw new ArgumentException("There is no model named '" + name + "'. Use one of ModelLibrary.ModelNames.", nameof(name));
        byte[] glb = ReadResource(pack + "/" + name + ".glb");
        return GltfModelConverter.Convert(set, glb, name, uri => ReadResource(GltfModelConverter.CombineKey(pack, uri)), pack, cache);
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
