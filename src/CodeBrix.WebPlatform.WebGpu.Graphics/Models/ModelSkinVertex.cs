using System.Numerics;
using System.Runtime.InteropServices;

namespace CodeBrix.WebPlatform.WebGpu.Graphics.Models;

/// <summary>
/// The skinning data of one vertex of a skinned primitive (24 bytes, exactly the record the model data format
/// stores), in the model's separate skin vertex array (<see cref="ModelData.SkinVertices" />, starting at
/// <see cref="ModelPrimitiveData.SkinBaseVertex" />): four joint indices (into the skin's joint list) and their
/// weights (normalised to sum to 1).
/// </summary>
[StructLayout(LayoutKind.Sequential)]
public struct ModelSkinVertex
{
    /// <summary>The size in bytes.</summary>
    public const int Size = 24;

    /// <summary>Joint 0 (offset 0; the four joints read as Uint16x4).</summary>
    public ushort Joint0;

    /// <summary>Joint 1.</summary>
    public ushort Joint1;

    /// <summary>Joint 2.</summary>
    public ushort Joint2;

    /// <summary>Joint 3.</summary>
    public ushort Joint3;

    /// <summary>The four weights (offset 8, Float32x4).</summary>
    public Vector4 Weights;
}
