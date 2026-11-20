using System;
using System.Collections.Generic;
using System.IO;
using System.Numerics;
using System.Threading.Tasks;
using CodeBrix.WebPlatform.WebGpu.Graphics.Models;
using Schema = CodeBrix.Graphics3D.Gltf2.Schema2;

namespace GraphicsDemo.Assets.Conversion.Internal;

/// <summary>
/// The conversion of one glTF document into a <see cref="ModelData" />: the CPU half of the C# library's internal
/// glTF loader (CodeBrix.WebGpu.Graphics GltfModelLoader), step for step, with the GPU uploads replaced by entries in
/// the model data set. Nodes (with the in-scene flags), materials with every channel and extension the C# library
/// reads (and its list of approximated features and warnings), texture slots with KHR_texture_transform and the
/// texture-coordinate override, samplers shared by content within the model, textures per (image, colour space)
/// with the colour-channel rule, every primitive packed into one vertex and one index array (strips and fans turned
/// into lists, sparse accessors read, default morph weights baked into positions, normals and tangents generated as
/// the C# library generates them, 16-bit indices when every primitive has at most 65536 vertices), skins (identity
/// inverse bind matrices when the file has none) and animations decoded into packed per-channel arrays.
/// </summary>
internal sealed class GltfModelBuilder
{
    private readonly ModelDataSet set;
    private readonly Schema.ModelRoot root;
    private readonly string name;
    private readonly string baseKey;
    private readonly ModelTextureCache cache;
    private readonly ModelConversionOptions options;
    private readonly List<string> warnings = new List<string>();
    private readonly Dictionary<(int Texture, TextureColorSpace Space), int> textures = new Dictionary<(int, TextureColorSpace), int>();
    private readonly Dictionary<(int Image, TextureColorSpace Space), int> imageTextures = new Dictionary<(int, TextureColorSpace), int>();
    private readonly List<int> distinctTextures = new List<int>();
    private readonly Dictionary<(SamplerAddressMode U, SamplerAddressMode V, SamplerFilter Mag, SamplerFilter Min, SamplerFilter Mip, bool NoMips), int> samplers = new Dictionary<(SamplerAddressMode, SamplerAddressMode, SamplerFilter, SamplerFilter, SamplerFilter, bool), int>();
    private readonly List<ModelSamplerData> distinctSamplers = new List<ModelSamplerData>();
    private ModelVertex[] vertices = new ModelVertex[1024];
    private uint[] indices = new uint[4096];
    private ModelSkinVertex[] skinVertices = Array.Empty<ModelSkinVertex>();
    private int vertexCount;
    private int indexCount;
    private int skinCount;
    private int largestPrimitive;

    /// <summary>Creates the builder.</summary>
    /// <param name="set">The set the model's textures go into.</param>
    /// <param name="root">The glTF document.</param>
    /// <param name="name">The model name.</param>
    /// <param name="baseKey">The key external images resolve against for the texture cache, or null.</param>
    /// <param name="cache">The texture cache of <paramref name="set" />, or null.</param>
    /// <param name="options">The options.</param>
    public GltfModelBuilder(ModelDataSet set, Schema.ModelRoot root, string name, string baseKey, ModelTextureCache cache, ModelConversionOptions options)
    {
        this.set = set;
        this.root = root;
        this.name = name;
        this.baseKey = baseKey;
        this.cache = cache;
        this.options = options;
    }

    /// <summary>Adds a warning.</summary>
    /// <param name="warning">The warning.</param>
    public void Warn(string warning) => warnings.Add(warning);

    /// <summary>Builds the model.</summary>
    /// <returns>The model (not yet added to the set's model list).</returns>
    public ModelData Build()
    {
        ModelData model = new ModelData { Name = name };
        ReadNodes(model);

        if (options.LoadTextures) LoadTextures();
        for (int index = 0; index < root.LogicalMaterials.Count; index++) model.Materials.Add(ReadMaterial(root.LogicalMaterials[index]));
        model.DefaultMaterial = new ModelMaterialData { Index = -1, Name = "default" };

        for (int index = 0; index < root.LogicalMeshes.Count; index++) model.Meshes.Add(ReadMesh(root.LogicalMeshes[index], model.Materials));
        Pack(model);

        ReadSkins(model);
        ReadAnimations(model);
        model.Textures.AddRange(distinctTextures);
        model.Samplers.AddRange(distinctSamplers);

        for (int node = 0; node < model.Nodes.Count; node++)
        {
            ModelNodeData data = model.Nodes[node];
            if (data.Mesh < 0 || data.Mesh >= model.Meshes.Count) continue;
            int morphCount = model.Meshes[data.Mesh].DefaultMorphWeights.Length;
            if (morphCount > 0) data.DefaultMorphWeights = NodeMorphWeights(root.LogicalNodes[node], morphCount);
        }

        model.Warnings.AddRange(warnings);
        return model;
    }

    private void ReadNodes(ModelData model)
    {
        IReadOnlyList<Schema.Node> source = root.LogicalNodes;
        for (int index = 0; index < source.Count; index++)
        {
            Schema.Node node = source[index];
            ModelNodeData data = new ModelNodeData { Name = node.Name };
            CodeBrix.Graphics3D.Gltf2.Transforms.AffineTransform local = node.LocalTransform;
            if (local.TryDecompose(out Vector3 scale, out Quaternion rotation, out Vector3 translation))
            {
                data.Translation = translation;
                data.Rotation = NormalizeOrIdentity(rotation);
                data.Scale = scale;
                data.LocalMatrix = TrsMatrix(data.Translation, data.Rotation, data.Scale);
            }
            else
            {
                data.HasFixedMatrix = true;
                data.LocalMatrix = local.Matrix;
            }

            data.Mesh = node.Mesh?.LogicalIndex ?? -1;
            data.Skin = node.Skin?.LogicalIndex ?? -1;
            model.Nodes.Add(data);
        }

        for (int index = 0; index < model.Nodes.Count; index++)
        {
            foreach (Schema.Node child in source[index].VisualChildren)
            {
                model.Nodes[index].Children.Add(child.LogicalIndex);
                model.Nodes[child.LogicalIndex].Parent = index;
            }
        }

        Schema.Scene scene = root.DefaultScene ?? (root.LogicalScenes.Count > 0 ? root.LogicalScenes[0] : null);
        if (scene != null)
        {
            foreach (Schema.Node node in scene.VisualChildren) model.RootNodes.Add(node.LogicalIndex);
        }
        else
        {
            for (int index = 0; index < model.Nodes.Count; index++)
            {
                if (model.Nodes[index].Parent < 0) model.RootNodes.Add(index);
            }
        }

        Stack<int> pending = new Stack<int>(model.RootNodes);
        while (pending.Count > 0)
        {
            ModelNodeData node = model.Nodes[pending.Pop()];
            if (node.InScene) continue;
            node.InScene = true;
            foreach (int child in node.Children) pending.Push(child);
        }
    }

    private static float[] NodeMorphWeights(Schema.Node node, int count)
    {
        float[] weights = new float[count];
        IReadOnlyList<float> source = node.MorphWeights;
        if (source != null)
        {
            for (int index = 0; index < Math.Min(count, source.Count); index++) weights[index] = source[index];
        }

        return weights;
    }

    private void LoadTextures()
    {
        HashSet<(int Texture, TextureColorSpace Space)> wanted = new HashSet<(int, TextureColorSpace)>();
        TextureColorSpace colorSpace = options.ColorTexturesSrgb ? TextureColorSpace.Srgb : TextureColorSpace.Linear;
        foreach (Schema.Material material in root.LogicalMaterials)
        {
            foreach (Schema.MaterialChannel channel in material.Channels)
            {
                Schema.Texture texture = channel.Texture;
                if (texture == null) continue;
                wanted.Add((texture.LogicalIndex, IsColorChannel(channel.Key) ? colorSpace : TextureColorSpace.Linear));
            }
        }

        List<(int Image, TextureColorSpace Space, string CacheKey, ReadOnlyMemory<byte> Bytes, string What)> decodes = new List<(int, TextureColorSpace, string, ReadOnlyMemory<byte>, string)>();
        Dictionary<(int Texture, TextureColorSpace Space), (int Image, TextureColorSpace Space)> textureImages = new Dictionary<(int, TextureColorSpace), (int, TextureColorSpace)>();
        HashSet<(int, TextureColorSpace)> queued = new HashSet<(int, TextureColorSpace)>();
        foreach ((int textureIndex, TextureColorSpace space) in wanted)
        {
            Schema.Image image = ChooseImage(root.LogicalTextures[textureIndex]);
            if (image == null) continue;
            textureImages[(textureIndex, space)] = (image.LogicalIndex, space);
            if (!queued.Add((image.LogicalIndex, space))) continue;
            string cacheKey = CacheKey(image);
            if (cacheKey != null && cache != null && cache.TryGet(cacheKey, space, options.Mipmaps, out int cached))
            {
                imageTextures[(image.LogicalIndex, space)] = cached;
                continue;
            }

            decodes.Add((image.LogicalIndex, space, cacheKey, image.Content.Content, Describe(image)));
        }

        ModelTextureData[] decoded = new ModelTextureData[decodes.Count];
        string[] failures = new string[decodes.Count];
        void DecodeOne(int index)
        {
            try
            {
                byte[] pixels = TextureDecoder.Decode(decodes[index].Bytes, decodes[index].What, out int width, out int height);
                decoded[index] = new ModelTextureData
                {
                    Name = decodes[index].CacheKey ?? (name + " " + decodes[index].What),
                    Width = width,
                    Height = height,
                    ColorSpace = decodes[index].Space,
                    Mipmaps = options.Mipmaps,
                    Pixels = pixels,
                };
            }
            catch (InvalidDataException exception)
            {
                failures[index] = exception.Message;
            }
        }

        if (options.DecodeTexturesInParallel && decodes.Count > 1) Parallel.For(0, decodes.Count, DecodeOne);
        else
        {
            for (int index = 0; index < decodes.Count; index++) DecodeOne(index);
        }

        for (int index = 0; index < decodes.Count; index++)
        {
            ModelTextureData texture = decoded[index];
            if (texture == null)
            {
                warnings.Add(failures[index] + " The texture is left out.");
                continue;
            }

            if (texture.Width > ModelDataFormat.MaxTextureDimension || texture.Height > ModelDataFormat.MaxTextureDimension)
            {
                warnings.Add("The image " + decodes[index].What + " is " + texture.Width + " x " + texture.Height + " pixels, larger than the model data format's " + ModelDataFormat.MaxTextureDimension + " x " + ModelDataFormat.MaxTextureDimension + "; the texture is left out. Scale the image down.");
                continue;
            }

            int absolute;
            if (decodes[index].CacheKey != null && cache != null) absolute = cache.Add(decodes[index].CacheKey, texture);
            else
            {
                absolute = set.Textures.Count;
                set.Textures.Add(texture);
            }

            imageTextures[(decodes[index].Image, decodes[index].Space)] = absolute;
        }

        foreach (KeyValuePair<(int Texture, TextureColorSpace Space), (int Image, TextureColorSpace Space)> pair in textureImages)
        {
            if (!imageTextures.TryGetValue(pair.Value, out int texture)) continue;
            textures[pair.Key] = texture;
            if (!distinctTextures.Contains(texture)) distinctTextures.Add(texture);
        }
    }

    private Schema.Image ChooseImage(Schema.Texture texture)
    {
        Schema.Image primary = texture.PrimaryImage;
        if (primary != null && IsDecodable(primary)) return primary;
        Schema.Image fallback = texture.FallbackImage;
        if (fallback != null && IsDecodable(fallback)) return fallback;
        Schema.Image any = primary ?? fallback;
        warnings.Add("Texture " + texture.LogicalIndex + (any == null ? " has no image." : " uses " + Describe(any) + ", a format CodeBrix.Imaging cannot decode (KTX2, DDS and similar GPU formats are not supported); the texture is left out."));
        return null;
    }

    private static bool IsDecodable(Schema.Image image)
    {
        CodeBrix.Graphics3D.Gltf2.Memory.MemoryImage content = image.Content;
        if (content.IsEmpty) return false;
        return !content.IsKtx2 && !content.IsDds && !content.IsXnb;
    }

    private string CacheKey(Schema.Image image)
    {
        string hint = image.Content.SourcePath;
        if (string.IsNullOrEmpty(hint) || baseKey == null) return null;
        return GltfModelConverter.CombineKey(baseKey, hint);
    }

    private static string Describe(Schema.Image image)
    {
        string hint = image.Content.SourcePath;
        if (!string.IsNullOrEmpty(hint)) return "'" + hint + "'";
        return "image " + image.LogicalIndex + (string.IsNullOrEmpty(image.Name) ? string.Empty : " '" + image.Name + "'");
    }

    private static bool IsColorChannel(string key) => key is "BaseColor" or "Emissive" or "Diffuse" or "SpecularColor" or "SheenColor";

    private ModelMaterialData ReadMaterial(Schema.Material material)
    {
        ModelMaterialData result = new ModelMaterialData { Index = material.LogicalIndex, Name = material.Name };
        List<string> approximated = new List<string>();
        result.AlphaMode = material.Alpha switch
        {
            Schema.AlphaMode.MASK => MaterialAlphaMode.Mask,
            Schema.AlphaMode.BLEND => MaterialAlphaMode.Blend,
            _ => MaterialAlphaMode.Opaque,
        };
        result.AlphaCutoff = material.AlphaCutoff;
        result.DoubleSided = material.DoubleSided;
        result.Unlit = material.Unlit;

        Schema.MaterialChannel? baseColor = material.FindChannel("BaseColor");
        if (baseColor.HasValue)
        {
            result.BaseColorFactor = ColorOf(baseColor.Value, Vector4.One);
            result.BaseColorTexture = Slot(baseColor.Value, true);
        }
        else
        {
            Schema.MaterialChannel? diffuse = material.FindChannel("Diffuse");
            if (diffuse.HasValue)
            {
                result.BaseColorFactor = ColorOf(diffuse.Value, Vector4.One);
                result.BaseColorTexture = Slot(diffuse.Value, true);
                result.MetallicFactor = 0f;
                approximated.Add("KHR_materials_pbrSpecularGlossiness");
            }
        }

        Schema.MaterialChannel? metallicRoughness = material.FindChannel("MetallicRoughness");
        if (metallicRoughness.HasValue)
        {
            result.MetallicFactor = Factor(metallicRoughness.Value, "MetallicFactor", 1f);
            result.RoughnessFactor = Factor(metallicRoughness.Value, "RoughnessFactor", 1f);
            result.MetallicRoughnessTexture = Slot(metallicRoughness.Value, false);
        }

        Schema.MaterialChannel? normal = material.FindChannel("Normal");
        if (normal.HasValue)
        {
            result.NormalTexture = Slot(normal.Value, false);
            result.NormalScale = Factor(normal.Value, "NormalScale", 1f);
        }

        Schema.MaterialChannel? occlusion = material.FindChannel("Occlusion");
        if (occlusion.HasValue)
        {
            result.OcclusionTexture = Slot(occlusion.Value, false);
            result.OcclusionStrength = Factor(occlusion.Value, "OcclusionStrength", 1f);
        }

        Schema.MaterialChannel? emissive = material.FindChannel("Emissive");
        if (emissive.HasValue)
        {
            Vector4 color = ColorOf(emissive.Value, new Vector4(0, 0, 0, 1));
            result.EmissiveFactor = new Vector3(color.X, color.Y, color.Z);
            result.EmissiveStrength = Factor(emissive.Value, "EmissiveStrength", 1f);
            result.EmissiveTexture = Slot(emissive.Value, true);
        }

        result.IndexOfRefraction = material.IndexOfRefraction;
        if (Math.Abs(result.IndexOfRefraction - 1.5f) > 1e-6f) approximated.Add("KHR_materials_ior");

        Schema.MaterialChannel? clearCoat = material.FindChannel("ClearCoat");
        if (clearCoat.HasValue)
        {
            result.ClearCoatFactor = Factor(clearCoat.Value, "ClearCoatFactor", 0f);
            Schema.MaterialChannel? clearCoatRoughness = material.FindChannel("ClearCoatRoughness");
            if (clearCoatRoughness.HasValue) result.ClearCoatRoughness = Factor(clearCoatRoughness.Value, "RoughnessFactor", 0f);
            approximated.Add("KHR_materials_clearcoat");
        }

        Schema.MaterialChannel? specularFactor = material.FindChannel("SpecularFactor");
        Schema.MaterialChannel? specularColor = material.FindChannel("SpecularColor");
        if (specularFactor.HasValue || specularColor.HasValue)
        {
            if (specularFactor.HasValue) result.SpecularFactor = Factor(specularFactor.Value, "SpecularFactor", 1f);
            if (specularColor.HasValue)
            {
                Vector4 color = ColorOf(specularColor.Value, Vector4.One);
                result.SpecularColor = new Vector3(color.X, color.Y, color.Z);
            }

            approximated.Add("KHR_materials_specular");
        }

        Schema.MaterialChannel? transmission = material.FindChannel("Transmission");
        if (transmission.HasValue)
        {
            result.TransmissionFactor = Factor(transmission.Value, "TransmissionFactor", 0f);
            approximated.Add("KHR_materials_transmission");
        }

        if (material.FindChannel("SheenColor").HasValue) approximated.Add("KHR_materials_sheen");
        if (material.FindChannel("VolumeThickness").HasValue) approximated.Add("KHR_materials_volume");
        if (material.FindChannel("Iridescence").HasValue) approximated.Add("KHR_materials_iridescence");
        if (material.FindChannel("Anisotropy").HasValue) approximated.Add("KHR_materials_anisotropy");
        if (material.FindChannel("DiffuseTransmissionFactor").HasValue) approximated.Add("KHR_materials_diffuse_transmission");
        if (approximated.Count > 0)
        {
            result.ApproximatedFeatures.AddRange(approximated);
            warnings.Add("Material " + material.LogicalIndex + (material.Name == null ? string.Empty : " '" + material.Name + "'") + " uses " + string.Join(", ", approximated) + " (captured, approximated when drawn).");
        }

        return result;
    }

    private ModelTextureSlotData Slot(Schema.MaterialChannel channel, bool color)
    {
        Schema.Texture texture = channel.Texture;
        if (texture == null) return ModelTextureSlotData.Empty;
        TextureColorSpace space = color && options.ColorTexturesSrgb ? TextureColorSpace.Srgb : TextureColorSpace.Linear;
        int local = textures.TryGetValue((texture.LogicalIndex, space), out int absolute) ? distinctTextures.IndexOf(absolute) : -1;
        int texCoord = channel.TextureCoordinate;
        Matrix3x2 transform = Matrix3x2.Identity;
        Schema.TextureTransform textureTransform = channel.TextureTransform;
        bool hasTransform = textureTransform != null;
        if (hasTransform)
        {
            transform = textureTransform.Matrix;
            if (textureTransform.TextureCoordinateOverride.HasValue) texCoord = textureTransform.TextureCoordinateOverride.Value;
        }

        if (texCoord > 1) warnings.Add("A texture uses TEXCOORD_" + texCoord + "; only TEXCOORD_0 and TEXCOORD_1 are loaded, so it samples TEXCOORD_0.");
        return new ModelTextureSlotData
        {
            Texture = local,
            Sampler = local < 0 ? -1 : SamplerFor(texture.Sampler),
            TexCoord = texCoord > 1 ? 0 : texCoord,
            Transform = transform,
            HasTransform = hasTransform,
            TextureIndex = texture.LogicalIndex,
        };
    }

    private int SamplerFor(Schema.TextureSampler sampler)
    {
        SamplerAddressMode u = sampler == null ? SamplerAddressMode.Repeat : Wrap(sampler.WrapS);
        SamplerAddressMode v = sampler == null ? SamplerAddressMode.Repeat : Wrap(sampler.WrapT);
        SamplerFilter mag = sampler != null && sampler.MagFilter == Schema.TextureInterpolationFilter.NEAREST ? SamplerFilter.Nearest : SamplerFilter.Linear;
        Schema.TextureMipMapFilter minFilter = sampler?.MinFilter ?? Schema.TextureMipMapFilter.DEFAULT;
        (SamplerFilter min, SamplerFilter mip, bool noMips) = minFilter switch
        {
            Schema.TextureMipMapFilter.NEAREST => (SamplerFilter.Nearest, SamplerFilter.Nearest, true),
            Schema.TextureMipMapFilter.LINEAR => (SamplerFilter.Linear, SamplerFilter.Nearest, true),
            Schema.TextureMipMapFilter.NEAREST_MIPMAP_NEAREST => (SamplerFilter.Nearest, SamplerFilter.Nearest, false),
            Schema.TextureMipMapFilter.LINEAR_MIPMAP_NEAREST => (SamplerFilter.Linear, SamplerFilter.Nearest, false),
            Schema.TextureMipMapFilter.NEAREST_MIPMAP_LINEAR => (SamplerFilter.Nearest, SamplerFilter.Linear, false),
            _ => (SamplerFilter.Linear, SamplerFilter.Linear, false),
        };
        if (samplers.TryGetValue((u, v, mag, min, mip, noMips), out int existing)) return existing;
        int created = distinctSamplers.Count;
        distinctSamplers.Add(new ModelSamplerData
        {
            AddressModeU = u,
            AddressModeV = v,
            MagFilter = mag,
            MinFilter = min,
            MipmapFilter = mip,
            MaxLod = noMips ? 0f : 32f,
        });
        samplers[(u, v, mag, min, mip, noMips)] = created;
        return created;
    }

    private static SamplerAddressMode Wrap(Schema.TextureWrapMode mode) => mode switch
    {
        Schema.TextureWrapMode.CLAMP_TO_EDGE => SamplerAddressMode.ClampToEdge,
        Schema.TextureWrapMode.MIRRORED_REPEAT => SamplerAddressMode.MirrorRepeat,
        _ => SamplerAddressMode.Repeat,
    };

    private static Vector4 ColorOf(Schema.MaterialChannel channel, Vector4 fallback)
    {
        try
        {
            return channel.Color;
        }
        catch (InvalidOperationException)
        {
            return fallback;
        }
    }

    private static float Factor(Schema.MaterialChannel channel, string key, float fallback)
    {
        foreach (Schema.IMaterialParameter parameter in channel.Parameters)
        {
            if (parameter.Name == key && parameter.Value is float value) return value;
        }

        return fallback;
    }

    private ModelMeshData ReadMesh(Schema.Mesh mesh, List<ModelMaterialData> materials)
    {
        IReadOnlyList<float> weightsSource = mesh.MorphWeights;
        int morphCount = 0;
        foreach (Schema.MeshPrimitive primitive in mesh.Primitives) morphCount = Math.Max(morphCount, primitive.MorphTargetsCount);
        float[] defaultWeights = new float[morphCount];
        if (weightsSource != null)
        {
            for (int index = 0; index < Math.Min(morphCount, weightsSource.Count); index++) defaultWeights[index] = weightsSource[index];
        }

        ModelMeshData result = new ModelMeshData { Name = mesh.Name, DefaultMorphWeights = defaultWeights };
        foreach (Schema.MeshPrimitive primitive in mesh.Primitives)
        {
            int material = primitive.Material != null && primitive.Material.LogicalIndex < materials.Count ? primitive.Material.LogicalIndex : -1;
            ModelMaterialData materialData = material >= 0 ? materials[material] : null;
            ModelPrimitiveData loaded = ReadPrimitive(mesh, primitive, material, materialData, defaultWeights);
            if (loaded != null) result.Primitives.Add(loaded);
        }

        return result;
    }

    private ModelPrimitiveData ReadPrimitive(Schema.Mesh mesh, Schema.MeshPrimitive primitive, int material, ModelMaterialData materialData, float[] defaultWeights)
    {
        string where = "Mesh " + mesh.LogicalIndex + (mesh.Name == null ? string.Empty : " '" + mesh.Name + "'") + " primitive " + primitive.LogicalIndex;
        Schema.PrimitiveType mode = primitive.DrawPrimitiveType;
        if (mode is not (Schema.PrimitiveType.TRIANGLES or Schema.PrimitiveType.TRIANGLE_STRIP or Schema.PrimitiveType.TRIANGLE_FAN))
        {
            warnings.Add(where + " draws " + mode + "; only triangles are supported, so it is left out.");
            return null;
        }

        Schema.Accessor positionAccessor = primitive.GetVertexAccessor("POSITION");
        if (positionAccessor == null || positionAccessor.Count == 0)
        {
            warnings.Add(where + " has no POSITION; it is left out.");
            return null;
        }

        int count = positionAccessor.Count;
        IList<Vector3> positions = positionAccessor.AsVector3Array();
        IList<Vector3> normals = primitive.GetVertexAccessor("NORMAL")?.AsVector3Array();
        IList<Vector4> tangents = primitive.GetVertexAccessor("TANGENT")?.AsVector4Array();
        IList<Vector2> uv0 = primitive.GetVertexAccessor("TEXCOORD_0")?.AsVector2Array();
        IList<Vector2> uv1 = primitive.GetVertexAccessor("TEXCOORD_1")?.AsVector2Array();
        IList<Vector4> colors = primitive.GetVertexAccessor("COLOR_0")?.AsColorArray(1f);
        IList<Vector4> joints = primitive.GetVertexAccessor("JOINTS_0")?.AsVector4Array();
        IList<Vector4> weights = primitive.GetVertexAccessor("WEIGHTS_0")?.AsVector4Array();

        uint[] local = ReadIndices(primitive, mode, count, where);
        if (local == null) return null;

        EnsureVertices(count);
        int baseVertex = vertexCount;
        Span<ModelVertex> span = vertices.AsSpan(baseVertex, count);
        Vector3 min = new Vector3(float.MaxValue);
        Vector3 max = new Vector3(float.MinValue);
        for (int index = 0; index < count; index++)
        {
            ref ModelVertex vertex = ref span[index];
            vertex.Position = positions[index];
            vertex.Normal = normals != null && index < normals.Count ? normals[index] : Vector3.Zero;
            vertex.Tangent = tangents != null && index < tangents.Count ? tangents[index] : Vector4.Zero;
            vertex.TexCoord0 = uv0 != null && index < uv0.Count ? uv0[index] : Vector2.Zero;
            vertex.TexCoord1 = uv1 != null && index < uv1.Count ? uv1[index] : Vector2.Zero;
            vertex.Color = colors != null && index < colors.Count ? colors[index] : Vector4.One;
        }

        List<ModelMorphTargetData> targets = ReadMorphTargets(primitive, count, defaultWeights, span);
        for (int index = 0; index < count; index++)
        {
            min = Vector3.Min(min, span[index].Position);
            max = Vector3.Max(max, span[index].Position);
        }

        bool normalsGenerated = normals == null;
        if (normalsGenerated) GenerateNormals(span, local);
        bool tangentsGenerated = false;
        ModelTextureSlotData normalTexture = materialData?.NormalTexture;
        bool hasNormalMap = normalTexture != null && normalTexture.TextureIndex >= 0;
        if (tangents == null && options.GenerateTangents && hasNormalMap)
        {
            int set = normalTexture.TexCoord;
            if ((set == 0 && uv0 != null) || (set == 1 && uv1 != null))
            {
                GenerateTangents(span, local, set);
                tangentsGenerated = true;
            }
            else
            {
                warnings.Add(where + " has a normal map but no TEXCOORD_" + set + "; no tangents were generated.");
            }
        }

        int skinBase = -1;
        if (joints != null && weights != null)
        {
            skinBase = skinCount;
            EnsureSkin(count);
            for (int index = 0; index < count; index++)
            {
                Vector4 joint = index < joints.Count ? joints[index] : Vector4.Zero;
                Vector4 weight = index < weights.Count ? weights[index] : new Vector4(1, 0, 0, 0);
                float sum = weight.X + weight.Y + weight.Z + weight.W;
                weight = sum > 1e-8f ? weight / sum : new Vector4(1, 0, 0, 0);
                skinVertices[skinCount + index] = new ModelSkinVertex
                {
                    Joint0 = (ushort)Math.Clamp(joint.X, 0, ushort.MaxValue),
                    Joint1 = (ushort)Math.Clamp(joint.Y, 0, ushort.MaxValue),
                    Joint2 = (ushort)Math.Clamp(joint.Z, 0, ushort.MaxValue),
                    Joint3 = (ushort)Math.Clamp(joint.W, 0, ushort.MaxValue),
                    Weights = weight,
                };
            }

            skinCount += count;
        }

        EnsureIndices(local.Length);
        uint firstIndex = (uint)indexCount;
        local.CopyTo(indices, indexCount);
        indexCount += local.Length;
        vertexCount += count;
        largestPrimitive = Math.Max(largestPrimitive, count);
        return new ModelPrimitiveData
        {
            Material = material,
            FirstIndex = firstIndex,
            IndexCount = (uint)local.Length,
            BaseVertex = baseVertex,
            VertexCount = count,
            SkinBaseVertex = skinBase,
            BoundsMin = min,
            BoundsMax = max,
            HasVertexColors = colors != null,
            HasTexCoord1 = uv1 != null,
            HasTangents = tangents != null || tangentsGenerated,
            TangentsGenerated = tangentsGenerated,
            NormalsGenerated = normalsGenerated,
            MorphTargets = targets,
        };
    }

    private uint[] ReadIndices(Schema.MeshPrimitive primitive, Schema.PrimitiveType mode, int count, string where)
    {
        Schema.Accessor accessor = primitive.IndexAccessor;
        uint[] raw;
        if (accessor == null)
        {
            raw = new uint[count];
            for (int index = 0; index < count; index++) raw[index] = (uint)index;
        }
        else
        {
            IList<uint> source = accessor.IsSparse ? accessor.AsIndexArray() : accessor.AsIndicesArray();
            raw = new uint[source.Count];
            source.CopyTo(raw, 0);
        }

        uint[] list = mode switch
        {
            Schema.PrimitiveType.TRIANGLE_STRIP => FromStrip(raw),
            Schema.PrimitiveType.TRIANGLE_FAN => FromFan(raw),
            _ => raw,
        };
        int usable = list.Length - (list.Length % 3);
        if (usable != list.Length)
        {
            warnings.Add(where + " has " + list.Length + " indices (not a multiple of 3); the incomplete triangle is dropped.");
            Array.Resize(ref list, usable);
        }

        foreach (uint index in list)
        {
            if (index >= count)
            {
                warnings.Add(where + " has index " + index + " for " + count + " vertices; it is left out.");
                return null;
            }
        }

        if (list.Length == 0)
        {
            warnings.Add(where + " has no triangles; it is left out.");
            return null;
        }

        return list;
    }

    private static uint[] FromStrip(uint[] strip)
    {
        if (strip.Length < 3) return Array.Empty<uint>();
        uint[] list = new uint[(strip.Length - 2) * 3];
        for (int index = 0; index < strip.Length - 2; index++)
        {
            bool even = (index & 1) == 0;
            list[(index * 3) + 0] = strip[index];
            list[(index * 3) + 1] = even ? strip[index + 1] : strip[index + 2];
            list[(index * 3) + 2] = even ? strip[index + 2] : strip[index + 1];
        }

        return list;
    }

    private static uint[] FromFan(uint[] fan)
    {
        if (fan.Length < 3) return Array.Empty<uint>();
        uint[] list = new uint[(fan.Length - 2) * 3];
        for (int index = 0; index < fan.Length - 2; index++)
        {
            list[(index * 3) + 0] = fan[index + 1];
            list[(index * 3) + 1] = fan[index + 2];
            list[(index * 3) + 2] = fan[0];
        }

        return list;
    }

    private static List<ModelMorphTargetData> ReadMorphTargets(Schema.MeshPrimitive primitive, int count, float[] defaultWeights, Span<ModelVertex> span)
    {
        int targets = primitive.MorphTargetsCount;
        List<ModelMorphTargetData> result = new List<ModelMorphTargetData>(targets);
        if (targets == 0) return result;
        bool changedNormals = false;
        for (int target = 0; target < targets; target++)
        {
            IReadOnlyDictionary<string, Schema.Accessor> accessors = primitive.GetMorphTargetAccessors(target);
            Vector3[] positions = Deltas(accessors, "POSITION", count);
            Vector3[] normals = Deltas(accessors, "NORMAL", count);
            Vector3[] tangents = Deltas(accessors, "TANGENT", count);
            result.Add(new ModelMorphTargetData { PositionDeltas = positions, NormalDeltas = normals, TangentDeltas = tangents });
            float weight = target < defaultWeights.Length ? defaultWeights[target] : 0f;
            if (weight == 0f) continue;
            for (int index = 0; index < count; index++)
            {
                ref ModelVertex vertex = ref span[index];
                if (positions.Length > 0) vertex.Position += positions[index] * weight;
                if (normals.Length > 0)
                {
                    vertex.Normal += normals[index] * weight;
                    changedNormals = true;
                }

                if (tangents.Length > 0) vertex.Tangent += new Vector4(tangents[index] * weight, 0f);
            }
        }

        if (changedNormals)
        {
            for (int index = 0; index < count; index++)
            {
                if (span[index].Normal.LengthSquared() > 0) span[index].Normal = Vector3.Normalize(span[index].Normal);
            }
        }

        return result;
    }

    private static Vector3[] Deltas(IReadOnlyDictionary<string, Schema.Accessor> accessors, string key, int count)
    {
        if (accessors == null || !accessors.TryGetValue(key, out Schema.Accessor accessor) || accessor == null) return Array.Empty<Vector3>();
        IList<Vector3> source = accessor.AsVector3Array();
        Vector3[] result = new Vector3[count];
        for (int index = 0; index < Math.Min(count, source.Count); index++) result[index] = source[index];
        return result;
    }

    private static void GenerateNormals(Span<ModelVertex> span, uint[] local)
    {
        for (int index = 0; index + 2 < local.Length; index += 3)
        {
            ref ModelVertex a = ref span[(int)local[index]];
            ref ModelVertex b = ref span[(int)local[index + 1]];
            ref ModelVertex c = ref span[(int)local[index + 2]];
            Vector3 face = Vector3.Cross(b.Position - a.Position, c.Position - a.Position);
            a.Normal += face;
            b.Normal += face;
            c.Normal += face;
        }

        for (int index = 0; index < span.Length; index++)
        {
            span[index].Normal = span[index].Normal.LengthSquared() > 1e-20f ? Vector3.Normalize(span[index].Normal) : Vector3.UnitY;
        }
    }

    private static void GenerateTangents(Span<ModelVertex> span, uint[] local, int set)
    {
        Vector3[] tangent = new Vector3[span.Length];
        Vector3[] bitangent = new Vector3[span.Length];
        for (int index = 0; index + 2 < local.Length; index += 3)
        {
            int i0 = (int)local[index];
            int i1 = (int)local[index + 1];
            int i2 = (int)local[index + 2];
            Vector2 t0 = set == 1 ? span[i0].TexCoord1 : span[i0].TexCoord0;
            Vector2 t1 = set == 1 ? span[i1].TexCoord1 : span[i1].TexCoord0;
            Vector2 t2 = set == 1 ? span[i2].TexCoord1 : span[i2].TexCoord0;
            Vector3 e1 = span[i1].Position - span[i0].Position;
            Vector3 e2 = span[i2].Position - span[i0].Position;
            float du1 = t1.X - t0.X;
            float du2 = t2.X - t0.X;
            float dv1 = -(t1.Y - t0.Y);
            float dv2 = -(t2.Y - t0.Y);
            float determinant = (du1 * dv2) - (du2 * dv1);
            if (MathF.Abs(determinant) < 1e-12f) continue;
            float inverse = 1f / determinant;
            Vector3 s = ((e1 * dv2) - (e2 * dv1)) * inverse;
            Vector3 t = ((e2 * du1) - (e1 * du2)) * inverse;
            tangent[i0] += s;
            tangent[i1] += s;
            tangent[i2] += s;
            bitangent[i0] += t;
            bitangent[i1] += t;
            bitangent[i2] += t;
        }

        for (int index = 0; index < span.Length; index++)
        {
            Vector3 n = span[index].Normal;
            Vector3 t = tangent[index] - (n * Vector3.Dot(n, tangent[index]));
            if (t.LengthSquared() < 1e-20f)
            {
                Vector3 axis = MathF.Abs(n.X) < 0.9f ? Vector3.UnitX : Vector3.UnitY;
                t = Vector3.Cross(n, Vector3.Cross(axis, n));
            }

            t = Vector3.Normalize(t);
            float handedness = Vector3.Dot(Vector3.Cross(n, t), bitangent[index]) < 0f ? -1f : 1f;
            span[index].Tangent = new Vector4(t, handedness);
        }
    }

    private void Pack(ModelData model)
    {
        if (vertexCount == 0)
        {
            model.Vertices = Array.Empty<ModelVertex>();
            model.Indices = Array.Empty<uint>();
            model.IndexFormat = ModelIndexFormat.Uint32;
            return;
        }

        model.Vertices = vertices.AsSpan(0, vertexCount).ToArray();
        model.Indices = indices.AsSpan(0, indexCount).ToArray();
        model.IndexFormat = largestPrimitive <= 65536 ? ModelIndexFormat.Uint16 : ModelIndexFormat.Uint32;
        if (skinCount > 0) model.SkinVertices = skinVertices.AsSpan(0, skinCount).ToArray();
    }

    private void ReadSkins(ModelData model)
    {
        foreach (Schema.Skin skin in root.LogicalSkins)
        {
            ModelSkinData data = new ModelSkinData { Name = skin.Name, Skeleton = skin.Skeleton?.LogicalIndex ?? -1 };
            for (int joint = 0; joint < skin.JointsCount; joint++)
            {
                (Schema.Node node, Matrix4x4 matrix) = skin.GetJoint(joint);
                data.Joints.Add(node.LogicalIndex);
                data.InverseBindMatrices.Add(matrix);
            }

            model.Skins.Add(data);
        }
    }

    private void ReadAnimations(ModelData model)
    {
        foreach (Schema.Animation animation in root.LogicalAnimations)
        {
            ModelAnimationData data = new ModelAnimationData { Name = animation.Name };
            foreach (Schema.AnimationChannel channel in animation.Channels)
            {
                ModelAnimationChannelData decoded = ReadChannel(animation, channel);
                if (decoded != null) data.Channels.Add(decoded);
            }

            model.Animations.Add(data);
        }
    }

    private ModelAnimationChannelData ReadChannel(Schema.Animation animation, Schema.AnimationChannel channel)
    {
        string where = "Animation " + animation.LogicalIndex + (animation.Name == null ? string.Empty : " '" + animation.Name + "'") + " channel " + channel.LogicalIndex;
        Schema.Node node = channel.TargetNode;
        Schema.PropertyPath property = channel.TargetNodePath;
        if (node == null || property == Schema.PropertyPath.pointer)
        {
            warnings.Add(where + " targets '" + channel.TargetPointerPath + "' (KHR_animation_pointer); only node channels are loaded.");
            return null;
        }

        List<float> times = new List<float>();
        List<float> values = new List<float>();
        AnimationInterpolation interpolation;
        AnimationPath path;
        int components;
        switch (property)
        {
            case Schema.PropertyPath.translation:
            case Schema.PropertyPath.scale:
            {
                path = property == Schema.PropertyPath.translation ? AnimationPath.Translation : AnimationPath.Scale;
                components = 3;
                Schema.IAnimationSampler<Vector3> sampler = property == Schema.PropertyPath.translation ? channel.GetTranslationSampler() : channel.GetScaleSampler();
                if (sampler == null) return Missing(where);
                interpolation = Interpolation(sampler.InterpolationMode);
                if (interpolation == AnimationInterpolation.CubicSpline)
                {
                    foreach ((float key, (Vector3 inTangent, Vector3 value, Vector3 outTangent)) in sampler.GetCubicKeys())
                    {
                        times.Add(key);
                        Add(values, inTangent);
                        Add(values, value);
                        Add(values, outTangent);
                    }
                }
                else
                {
                    foreach ((float key, Vector3 value) in sampler.GetLinearKeys())
                    {
                        times.Add(key);
                        Add(values, value);
                    }
                }

                break;
            }

            case Schema.PropertyPath.rotation:
            {
                path = AnimationPath.Rotation;
                components = 4;
                Schema.IAnimationSampler<Quaternion> sampler = channel.GetRotationSampler();
                if (sampler == null) return Missing(where);
                interpolation = Interpolation(sampler.InterpolationMode);
                if (interpolation == AnimationInterpolation.CubicSpline)
                {
                    foreach ((float key, (Quaternion inTangent, Quaternion value, Quaternion outTangent)) in sampler.GetCubicKeys())
                    {
                        times.Add(key);
                        Add(values, inTangent);
                        Add(values, NormalizeOrIdentity(value));
                        Add(values, outTangent);
                    }
                }
                else
                {
                    foreach ((float key, Quaternion value) in sampler.GetLinearKeys())
                    {
                        times.Add(key);
                        Add(values, NormalizeOrIdentity(value));
                    }
                }

                break;
            }

            default:
            {
                path = AnimationPath.Weights;
                Schema.IAnimationSampler<float[]> sampler = channel.GetMorphSampler();
                if (sampler == null) return Missing(where);
                interpolation = Interpolation(sampler.InterpolationMode);
                components = 0;
                if (interpolation == AnimationInterpolation.CubicSpline)
                {
                    foreach ((float key, (float[] inTangent, float[] value, float[] outTangent)) in sampler.GetCubicKeys())
                    {
                        times.Add(key);
                        components = value.Length;
                        values.AddRange(inTangent);
                        values.AddRange(value);
                        values.AddRange(outTangent);
                    }
                }
                else
                {
                    foreach ((float key, float[] value) in sampler.GetLinearKeys())
                    {
                        times.Add(key);
                        components = value.Length;
                        values.AddRange(value);
                    }
                }

                break;
            }
        }

        int perKey = interpolation == AnimationInterpolation.CubicSpline ? 3 : 1;
        if (times.Count == 0 || components <= 0 || values.Count != times.Count * perKey * components)
        {
            warnings.Add(where + " has " + values.Count + " output values for " + times.Count + " keys; it is left out.");
            return null;
        }

        return new ModelAnimationChannelData
        {
            Node = node.LogicalIndex,
            Path = path,
            Interpolation = interpolation,
            ComponentCount = components,
            Times = times.ToArray(),
            Values = values.ToArray(),
        };
    }

    private ModelAnimationChannelData Missing(string where)
    {
        warnings.Add(where + " has no sampler; it is left out.");
        return null;
    }

    private static AnimationInterpolation Interpolation(Schema.AnimationInterpolationMode mode) => mode switch
    {
        Schema.AnimationInterpolationMode.STEP => AnimationInterpolation.Step,
        Schema.AnimationInterpolationMode.CUBICSPLINE => AnimationInterpolation.CubicSpline,
        _ => AnimationInterpolation.Linear,
    };

    private static void Add(List<float> values, Vector3 value)
    {
        values.Add(value.X);
        values.Add(value.Y);
        values.Add(value.Z);
    }

    private static void Add(List<float> values, Quaternion value)
    {
        values.Add(value.X);
        values.Add(value.Y);
        values.Add(value.Z);
        values.Add(value.W);
    }

    /// <summary>The matrix scale, then rotation, then translation (System.Numerics row-vector order), as the C# library's NodeTransform.ToMatrix.</summary>
    private static Matrix4x4 TrsMatrix(Vector3 translation, Quaternion rotation, Vector3 scale)
    {
        Matrix4x4 matrix = Matrix4x4.CreateFromQuaternion(rotation);
        matrix.M11 *= scale.X;
        matrix.M12 *= scale.X;
        matrix.M13 *= scale.X;
        matrix.M21 *= scale.Y;
        matrix.M22 *= scale.Y;
        matrix.M23 *= scale.Y;
        matrix.M31 *= scale.Z;
        matrix.M32 *= scale.Z;
        matrix.M33 *= scale.Z;
        matrix.M41 = translation.X;
        matrix.M42 = translation.Y;
        matrix.M43 = translation.Z;
        return matrix;
    }

    private void EnsureVertices(int more)
    {
        if (vertexCount + more <= vertices.Length) return;
        Array.Resize(ref vertices, Math.Max(vertices.Length * 2, vertexCount + more));
    }

    private void EnsureIndices(int more)
    {
        if (indexCount + more <= indices.Length) return;
        Array.Resize(ref indices, Math.Max(indices.Length * 2, indexCount + more));
    }

    private void EnsureSkin(int more)
    {
        if (skinCount + more <= skinVertices.Length) return;
        Array.Resize(ref skinVertices, Math.Max(Math.Max(skinVertices.Length * 2, 256), skinCount + more));
    }

    private static Quaternion NormalizeOrIdentity(Quaternion value)
    {
        float length = value.Length();
        return length > 1e-12f ? Quaternion.Normalize(value) : Quaternion.Identity;
    }
}
