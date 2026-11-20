using System.Collections.Generic;
using System.Numerics;

namespace CodeBrix.WebPlatform.WebGpu.Graphics.Models;

/// <summary>
/// A glTF material of a <see cref="ModelData" />, captured completely as metallic-roughness parameters. Renderers read
/// what they support; <see cref="ApproximatedFeatures" /> lists the extensions that are captured but only
/// approximated.
/// </summary>
public sealed class ModelMaterialData
{
    /// <summary>The position in <see cref="ModelData.Materials" />, or -1 for the glTF default material (<see cref="ModelData.DefaultMaterial" />).</summary>
    public int Index { get; set; }

    /// <summary>The name, or null.</summary>
    public string Name { get; set; }

    /// <summary>baseColorFactor (linear RGBA). Default white.</summary>
    public Vector4 BaseColorFactor { get; set; } = Vector4.One;

    /// <summary>baseColorTexture (sRGB texture).</summary>
    public ModelTextureSlotData BaseColorTexture { get; set; } = ModelTextureSlotData.Empty;

    /// <summary>metallicFactor. Default 1.</summary>
    public float MetallicFactor { get; set; } = 1f;

    /// <summary>roughnessFactor. Default 1.</summary>
    public float RoughnessFactor { get; set; } = 1f;

    /// <summary>metallicRoughnessTexture (linear; roughness in G, metallic in B).</summary>
    public ModelTextureSlotData MetallicRoughnessTexture { get; set; } = ModelTextureSlotData.Empty;

    /// <summary>normalTexture (linear, tangent space).</summary>
    public ModelTextureSlotData NormalTexture { get; set; } = ModelTextureSlotData.Empty;

    /// <summary>normalTexture.scale. Default 1.</summary>
    public float NormalScale { get; set; } = 1f;

    /// <summary>occlusionTexture (linear; occlusion in R).</summary>
    public ModelTextureSlotData OcclusionTexture { get; set; } = ModelTextureSlotData.Empty;

    /// <summary>occlusionTexture.strength. Default 1.</summary>
    public float OcclusionStrength { get; set; } = 1f;

    /// <summary>emissiveFactor (linear RGB). Default black.</summary>
    public Vector3 EmissiveFactor { get; set; }

    /// <summary>KHR_materials_emissive_strength. Default 1.</summary>
    public float EmissiveStrength { get; set; } = 1f;

    /// <summary>emissiveTexture (sRGB).</summary>
    public ModelTextureSlotData EmissiveTexture { get; set; } = ModelTextureSlotData.Empty;

    /// <summary>alphaMode (OPAQUE, MASK, BLEND). Default <see cref="MaterialAlphaMode.Opaque" />.</summary>
    public MaterialAlphaMode AlphaMode { get; set; } = MaterialAlphaMode.Opaque;

    /// <summary>alphaCutoff (used by MASK). Default 0.5.</summary>
    public float AlphaCutoff { get; set; } = 0.5f;

    /// <summary>doubleSided. Default false.</summary>
    public bool DoubleSided { get; set; }

    /// <summary>KHR_materials_unlit: the base colour is shown without lighting.</summary>
    public bool Unlit { get; set; }

    /// <summary>KHR_materials_ior. Default 1.5.</summary>
    public float IndexOfRefraction { get; set; } = 1.5f;

    /// <summary>KHR_materials_clearcoat factor (0 = none). Approximated.</summary>
    public float ClearCoatFactor { get; set; }

    /// <summary>KHR_materials_clearcoat roughness. Approximated.</summary>
    public float ClearCoatRoughness { get; set; }

    /// <summary>KHR_materials_specular factor. Default 1. Approximated.</summary>
    public float SpecularFactor { get; set; } = 1f;

    /// <summary>KHR_materials_specular colour. Default white. Approximated.</summary>
    public Vector3 SpecularColor { get; set; } = Vector3.One;

    /// <summary>KHR_materials_transmission factor (0 = none). Approximated.</summary>
    public float TransmissionFactor { get; set; }

    /// <summary>The extensions on this material that are captured but only approximated by the renderers (for example "KHR_materials_clearcoat").</summary>
    public List<string> ApproximatedFeatures { get; set; } = new List<string>();
}
