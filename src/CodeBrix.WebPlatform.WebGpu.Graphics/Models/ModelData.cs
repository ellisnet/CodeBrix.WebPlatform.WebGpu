using System;
using System.Collections.Generic;

namespace CodeBrix.WebPlatform.WebGpu.Graphics.Models;

/// <summary>
/// The CPU-side description of one model of a <see cref="ModelDataSet" /> (what a glTF 2.0 file loads to): one
/// shared vertex array, one shared index array, an optional skin vertex array, and the meshes, materials, node
/// hierarchy, skins, animation clips, textures and samplers that use them. Every index a record holds (material, node,
/// mesh, skin, sampler, texture) is local to this model unless its documentation says otherwise.
/// </summary>
public sealed class ModelData
{
    /// <summary>The name (for example the file name without its extension), or null.</summary>
    public string Name { get; set; }

    /// <summary>The shared vertex array every primitive draws from.</summary>
    public ModelVertex[] Vertices { get; set; } = Array.Empty<ModelVertex>();

    /// <summary>The shared index array (each index relative to its primitive's <see cref="ModelPrimitiveData.BaseVertex" />).</summary>
    public uint[] Indices { get; set; } = Array.Empty<uint>();

    /// <summary>How the indices are stored: <see cref="ModelIndexFormat.Uint16" /> only when every index is below 65 536. Default <see cref="ModelIndexFormat.Uint32" />.</summary>
    public ModelIndexFormat IndexFormat { get; set; } = ModelIndexFormat.Uint32;

    /// <summary>The skin vertex array of the skinned primitives, or null when the model has none.</summary>
    public ModelSkinVertex[] SkinVertices { get; set; }

    /// <summary>The meshes (glTF mesh order).</summary>
    public List<ModelMeshData> Meshes { get; set; } = new List<ModelMeshData>();

    /// <summary>The materials (glTF material order); material <c>i</c> has <see cref="ModelMaterialData.Index" /> <c>i</c>.</summary>
    public List<ModelMaterialData> Materials { get; set; } = new List<ModelMaterialData>();

    /// <summary>The glTF default material, used by primitives that name none (<see cref="ModelMaterialData.Index" /> -1, name "default").</summary>
    public ModelMaterialData DefaultMaterial { get; set; } = new ModelMaterialData { Index = -1, Name = "default" };

    /// <summary>Every node (glTF node order).</summary>
    public List<ModelNodeData> Nodes { get; set; } = new List<ModelNodeData>();

    /// <summary>The root nodes of the model's scene.</summary>
    public List<int> RootNodes { get; set; } = new List<int>();

    /// <summary>The skins.</summary>
    public List<ModelSkinData> Skins { get; set; } = new List<ModelSkinData>();

    /// <summary>The animation clips.</summary>
    public List<ModelAnimationData> Animations { get; set; } = new List<ModelAnimationData>();

    /// <summary>The distinct textures the materials use, as positions in <see cref="ModelDataSet.Textures" />; a texture slot's <see cref="ModelTextureSlotData.Texture" /> is a position in this list.</summary>
    public List<int> Textures { get; set; } = new List<int>();

    /// <summary>The distinct samplers the materials use; a texture slot's <see cref="ModelTextureSlotData.Sampler" /> is a position in this list.</summary>
    public List<ModelSamplerData> Samplers { get; set; } = new List<ModelSamplerData>();

    /// <summary>What was skipped or approximated while loading the model. Empty for a fully supported file.</summary>
    public List<string> Warnings { get; set; } = new List<string>();
}
