using System;
using System.Collections.Generic;

namespace CodeBrix.WebPlatform.WebGpu.Graphics.Models.Internal;

/// <summary>
/// The writer checks: every reference a model set holds is in range and every array has the length the format needs,
/// so that a reader can trust the blob. Each failure is an <see cref="ArgumentException" /> whose message says what is
/// wrong and what to do. Null lists and arrays count as empty; null slots count as empty slots.
/// </summary>
internal static class ModelDataValidator
{
    /// <summary>Checks a model set.</summary>
    /// <param name="set">The set.</param>
    /// <exception cref="ArgumentException">Something in the set is out of range or of the wrong length.</exception>
    public static void Validate(ModelDataSet set)
    {
        var models = List(set.Models);
        var textures = List(set.Textures);
        if (models.Count == 0) Fail("The model set has no models; add at least one ModelData to ModelDataSet.Models.");
        for (var index = 0; index < textures.Count; index++) ValidateTexture(textures[index], index);
        for (var index = 0; index < models.Count; index++)
        {
            if (models[index] == null) Fail("Model " + index + " of the set is null; remove it from ModelDataSet.Models or set it to a ModelData.");
            ValidateModel(models[index], index, textures.Count);
        }
    }

    /// <summary>A list, or an empty one for null.</summary>
    /// <typeparam name="T">The element type.</typeparam>
    /// <param name="list">The list.</param>
    /// <returns>The list or an empty one.</returns>
    public static IReadOnlyList<T> List<T>(IReadOnlyList<T> list) => list ?? Array.Empty<T>();

    /// <summary>The name of a model for a message: <c>Model 'tree'</c>, or <c>Model 0</c> when it has no name.</summary>
    /// <param name="model">The model.</param>
    /// <param name="index">Its position in the set.</param>
    /// <returns>The name.</returns>
    public static string Describe(ModelData model, int index) => string.IsNullOrEmpty(model.Name) ? "Model " + index : "Model '" + model.Name + "'";

    private static void ValidateTexture(ModelTextureData texture, int index)
    {
        if (texture == null) Fail("Texture " + index + " of the set is null; remove it from ModelDataSet.Textures or set it to a ModelTextureData.");
        var name = "Texture " + index + (texture.Name == null ? string.Empty : " ('" + texture.Name + "')");
        const int max = ModelDataFormat.MaxTextureDimension;
        if (texture.Width < 1 || texture.Width > max || texture.Height < 1 || texture.Height > max)
        {
            Fail(name + " is " + texture.Width + " x " + texture.Height + " pixels; give it a width and a height from 1 to " + max + " (scale a larger image down before writing).");
        }

        var expected = (long)texture.Width * texture.Height * 4;
        var actual = texture.Pixels == null ? 0 : texture.Pixels.Length;
        if (actual != expected)
        {
            Fail(name + " has " + actual + " pixel bytes, but " + texture.Width + " x " + texture.Height + " RGBA8 pixels take " + expected + "; set Pixels to width x height x 4 bytes (R, G, B, A per pixel, rows top to bottom).");
        }

        Defined(texture.ColorSpace, name + " ColorSpace");
    }

    private static void ValidateModel(ModelData model, int modelIndex, int setTextureCount)
    {
        var name = Describe(model, modelIndex);
        Defined(model.IndexFormat, name + " IndexFormat");
        var vertexCount = List(model.Vertices).Count;
        var indices = List(model.Indices);
        var skinVertexCount = List(model.SkinVertices).Count;
        if (model.IndexFormat == ModelIndexFormat.Uint16)
        {
            for (var position = 0; position < indices.Count; position++)
            {
                if (indices[position] > ushort.MaxValue)
                {
                    Fail(name + " has IndexFormat Uint16, but index " + position + " is " + indices[position] + "; set IndexFormat to ModelIndexFormat.Uint32 (Uint16 holds indices below 65536 only).");
                }
            }
        }

        var materials = List(model.Materials);
        var textures = List(model.Textures);
        var samplers = List(model.Samplers);
        var nodes = List(model.Nodes);
        var meshes = List(model.Meshes);
        var skins = List(model.Skins);

        for (var position = 0; position < textures.Count; position++)
        {
            if (textures[position] < 0 || textures[position] >= setTextureCount)
            {
                Fail(name + " texture " + position + " names set texture " + textures[position] + ", but the set has " + Count(setTextureCount, "texture") + "; add the texture to ModelDataSet.Textures or name one below " + setTextureCount + ".");
            }
        }

        for (var position = 0; position < samplers.Count; position++)
        {
            var sampler = samplers[position];
            if (sampler == null) Fail(name + " sampler " + position + " is null; set it to a ModelSamplerData.");
            var where = name + " sampler " + position;
            Defined(sampler.AddressModeU, where + " AddressModeU");
            Defined(sampler.AddressModeV, where + " AddressModeV");
            Defined(sampler.MagFilter, where + " MagFilter");
            Defined(sampler.MinFilter, where + " MinFilter");
            Defined(sampler.MipmapFilter, where + " MipmapFilter");
        }

        for (var position = 0; position < materials.Count; position++)
        {
            var material = materials[position];
            if (material == null) Fail(name + " material " + position + " is null; set it to a ModelMaterialData.");
            if (material.Index != position) Fail(name + " material " + position + " has Index " + material.Index + "; set Index to " + position + " (its position in Materials).");
            ValidateMaterial(material, name + " material " + position, textures.Count, samplers.Count);
        }

        if (model.DefaultMaterial == null) Fail(name + " has no DefaultMaterial; set it to new ModelMaterialData { Index = -1, Name = \"default\" }.");
        if (model.DefaultMaterial.Index != -1) Fail(name + " DefaultMaterial has Index " + model.DefaultMaterial.Index + "; set it to -1 (the glTF default material's index).");
        ValidateMaterial(model.DefaultMaterial, name + " default material", textures.Count, samplers.Count);

        for (var meshIndex = 0; meshIndex < meshes.Count; meshIndex++)
        {
            var mesh = meshes[meshIndex];
            if (mesh == null) Fail(name + " mesh " + meshIndex + " is null; set it to a ModelMeshData.");
            var weightCount = List(mesh.DefaultMorphWeights).Count;
            var primitives = List(mesh.Primitives);
            for (var primitiveIndex = 0; primitiveIndex < primitives.Count; primitiveIndex++)
            {
                var primitive = primitives[primitiveIndex];
                var where = name + " mesh " + meshIndex + " primitive " + primitiveIndex;
                if (primitive == null) Fail(where + " is null; set it to a ModelPrimitiveData.");
                ValidatePrimitive(primitive, where, materials.Count, vertexCount, indices, skinVertexCount, weightCount);
            }
        }

        for (var nodeIndex = 0; nodeIndex < nodes.Count; nodeIndex++)
        {
            var node = nodes[nodeIndex];
            var where = name + " node " + nodeIndex;
            if (node == null) Fail(where + " is null; set it to a ModelNodeData.");
            Optional(node.Parent, nodes.Count, where, "Parent", "node", "-1 for a root");
            var children = List(node.Children);
            for (var position = 0; position < children.Count; position++) Required(children[position], nodes.Count, where + " child " + position, "node");
            Optional(node.Mesh, meshes.Count, where, "Mesh", "mesh", "-1 for no mesh");
            Optional(node.Skin, skins.Count, where, "Skin", "skin", "-1 for no skin");
        }

        var roots = List(model.RootNodes);
        for (var position = 0; position < roots.Count; position++) Required(roots[position], nodes.Count, name + " root node " + position, "node");

        for (var skinIndex = 0; skinIndex < skins.Count; skinIndex++)
        {
            var skin = skins[skinIndex];
            var where = name + " skin " + skinIndex;
            if (skin == null) Fail(where + " is null; set it to a ModelSkinData.");
            var joints = List(skin.Joints);
            for (var position = 0; position < joints.Count; position++) Required(joints[position], nodes.Count, where + " joint " + position, "node");
            var matrices = List(skin.InverseBindMatrices).Count;
            if (matrices != 0 && matrices != joints.Count)
            {
                Fail(where + " has " + Count(matrices, "inverse bind matrix", "inverse bind matrices") + " for " + Count(joints.Count, "joint") + "; give one matrix per joint, or none for identities.");
            }

            Optional(skin.Skeleton, nodes.Count, where, "Skeleton", "node", "-1 for none");
        }

        var animations = List(model.Animations);
        for (var animationIndex = 0; animationIndex < animations.Count; animationIndex++)
        {
            var animation = animations[animationIndex];
            if (animation == null) Fail(name + " animation " + animationIndex + " is null; set it to a ModelAnimationData.");
            var channels = List(animation.Channels);
            for (var channelIndex = 0; channelIndex < channels.Count; channelIndex++)
            {
                var where = name + " animation " + animationIndex + " channel " + channelIndex;
                if (channels[channelIndex] == null) Fail(where + " is null; set it to a ModelAnimationChannelData.");
                ValidateChannel(channels[channelIndex], where, nodes.Count);
            }
        }
    }

    private static void ValidateMaterial(ModelMaterialData material, string where, int textureCount, int samplerCount)
    {
        Defined(material.AlphaMode, where + " AlphaMode");
        ValidateSlot(material.BaseColorTexture, where + " BaseColorTexture", textureCount, samplerCount);
        ValidateSlot(material.MetallicRoughnessTexture, where + " MetallicRoughnessTexture", textureCount, samplerCount);
        ValidateSlot(material.NormalTexture, where + " NormalTexture", textureCount, samplerCount);
        ValidateSlot(material.OcclusionTexture, where + " OcclusionTexture", textureCount, samplerCount);
        ValidateSlot(material.EmissiveTexture, where + " EmissiveTexture", textureCount, samplerCount);
    }

    private static void ValidateSlot(ModelTextureSlotData slot, string where, int textureCount, int samplerCount)
    {
        if (slot == null) return;
        Optional(slot.Texture, textureCount, where, "Texture", "texture", "-1 for no texture");
        Optional(slot.Sampler, samplerCount, where, "Sampler", "sampler", "-1 for none");
        if (slot.Texture < 0 && slot.Sampler >= 0) Fail(where + " names sampler " + slot.Sampler + " but no texture; set Sampler to -1 when Texture is -1.");
        if (slot.TexCoord != 0 && slot.TexCoord != 1) Fail(where + " has TexCoord " + slot.TexCoord + "; set it to 0 (TEXCOORD_0) or 1 (TEXCOORD_1).");
        if (slot.TextureIndex < -1) Fail(where + " has TextureIndex " + slot.TextureIndex + "; set it to the glTF texture index, or -1 for an empty slot.");
    }

    private static void ValidatePrimitive(ModelPrimitiveData primitive, string where, int materialCount, int vertexCount, IReadOnlyList<uint> indices, int skinVertexCount, int weightCount)
    {
        if (primitive.Material < -1 || primitive.Material >= materialCount)
        {
            Fail(where + " names material " + primitive.Material + ", but the model has " + Count(materialCount, "material") + "; set Material to -1 for the default material or to an index below " + materialCount + ".");
        }

        if (primitive.IndexCount % 3 != 0) Fail(where + " has " + primitive.IndexCount + " indices, which is not a multiple of 3; give it whole triangles (3 indices each).");
        if ((long)primitive.FirstIndex + primitive.IndexCount > indices.Count)
        {
            Fail(where + " uses indices " + primitive.FirstIndex + " to " + ((long)primitive.FirstIndex + primitive.IndexCount - 1) + ", but the model has " + Count(indices.Count, "index", "indices") + "; keep FirstIndex + IndexCount within Indices.");
        }

        if (primitive.BaseVertex < 0 || primitive.VertexCount < 0 || (long)primitive.BaseVertex + primitive.VertexCount > vertexCount)
        {
            Fail(where + " uses " + Count(primitive.VertexCount, "vertex", "vertices") + " from vertex " + primitive.BaseVertex + ", but the model has " + Count(vertexCount, "vertex", "vertices") + "; keep BaseVertex and VertexCount non-negative and BaseVertex + VertexCount within Vertices.");
        }

        for (var position = primitive.FirstIndex; position < primitive.FirstIndex + primitive.IndexCount; position++)
        {
            if (indices[(int)position] >= (uint)primitive.VertexCount)
            {
                Fail(where + " index " + position + " is " + indices[(int)position] + ", but the primitive has " + Count(primitive.VertexCount, "vertex", "vertices") + "; every index (relative to BaseVertex) must be below VertexCount.");
            }
        }

        if (primitive.SkinBaseVertex < -1 || (primitive.SkinBaseVertex >= 0 && (long)primitive.SkinBaseVertex + primitive.VertexCount > skinVertexCount))
        {
            Fail(where + " has SkinBaseVertex " + primitive.SkinBaseVertex + " for " + Count(primitive.VertexCount, "vertex", "vertices") + ", but the model has " + Count(skinVertexCount, "skin vertex", "skin vertices") + "; set SkinBaseVertex to -1 when the primitive is not skinned, else keep SkinBaseVertex + VertexCount within SkinVertices.");
        }

        var targets = List(primitive.MorphTargets);
        if (targets.Count != weightCount)
        {
            Fail(where + " has " + Count(targets.Count, "morph target") + ", but its mesh has " + Count(weightCount, "default morph weight") + "; give the mesh one DefaultMorphWeights entry per morph target of each primitive.");
        }

        for (var position = 0; position < targets.Count; position++)
        {
            var target = targets[position];
            var targetWhere = where + " morph target " + position;
            if (target == null) Fail(targetWhere + " is null; set it to a ModelMorphTargetData.");
            Deltas(List(target.PositionDeltas).Count, primitive.VertexCount, targetWhere, "PositionDeltas");
            Deltas(List(target.NormalDeltas).Count, primitive.VertexCount, targetWhere, "NormalDeltas");
            Deltas(List(target.TangentDeltas).Count, primitive.VertexCount, targetWhere, "TangentDeltas");
        }
    }

    private static void ValidateChannel(ModelAnimationChannelData channel, string where, int nodeCount)
    {
        Required(channel.Node, nodeCount, where, "node");
        Defined(channel.Path, where + " Path");
        Defined(channel.Interpolation, where + " Interpolation");
        var expected = channel.Path switch
        {
            AnimationPath.Rotation => 4,
            AnimationPath.Weights => channel.ComponentCount,
            _ => 3,
        };
        if (channel.ComponentCount < 1 || channel.ComponentCount != expected)
        {
            var what = channel.Path == AnimationPath.Weights ? "at least 1 (the morph-target count)" : expected.ToString(System.Globalization.CultureInfo.InvariantCulture);
            Fail(where + " (" + channel.Path + ") has ComponentCount " + channel.ComponentCount + "; set it to " + what + ".");
        }

        var keys = List(channel.Times).Count;
        var values = List(channel.Values).Count;
        var perKey = (long)channel.ComponentCount * (channel.Interpolation == AnimationInterpolation.CubicSpline ? 3 : 1);
        if (values != keys * perKey)
        {
            Fail(where + " has " + Count(values, "value") + " for " + Count(keys, "key") + "; give " + perKey + " floats per key (ComponentCount" + (channel.Interpolation == AnimationInterpolation.CubicSpline ? " x 3 for CubicSpline: in-tangent, value, out-tangent" : string.Empty) + "), " + (keys * perKey) + " in all.");
        }
    }

    private static void Deltas(int count, int vertexCount, string where, string property)
    {
        if (count != 0 && count != vertexCount)
        {
            Fail(where + " has " + Count(count, "entry", "entries") + " in " + property + ", but the primitive has " + Count(vertexCount, "vertex", "vertices") + "; give one delta per vertex, or none.");
        }
    }

    private static void Optional(int value, int count, string where, string property, string what, string none)
    {
        if (value < -1 || value >= count)
        {
            Fail(where + " names " + what + " " + value + " in " + property + ", but the model has " + Count(count, what) + "; set " + property + " to " + none + " or to an index below " + count + ".");
        }
    }

    private static void Required(int value, int count, string where, string what)
    {
        if (value < 0 || value >= count)
        {
            Fail(where + " names " + what + " " + value + ", but the model has " + Count(count, what) + "; name an index from 0 to " + (count - 1) + (count == 0 ? " (add the " + what + " first)" : string.Empty) + ".");
        }
    }

    private static void Defined<TEnum>(TEnum value, string where) where TEnum : struct, Enum
    {
        if (!Enum.IsDefined(value)) Fail(where + " is " + Convert.ToInt64(value, System.Globalization.CultureInfo.InvariantCulture) + ", which is not a " + typeof(TEnum).Name + " value; set it to one of " + string.Join(", ", Enum.GetNames<TEnum>()) + ".");
    }

    private static string Count(long count, string singular, string plural = null) => count + " " + (count == 1 ? singular : plural ?? (singular.EndsWith("sh", StringComparison.Ordinal) ? singular + "es" : singular + "s"));

    private static void Fail(string message) => throw new ArgumentException(message, "set");
}
