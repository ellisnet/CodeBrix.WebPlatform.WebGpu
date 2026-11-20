namespace CodeBrix.WebPlatform.WebGpu.Graphics.Models;

/// <summary>The size of one index of a <see cref="ModelData" /> (the numeric value is the size in bytes, as the model data format stores it).</summary>
public enum ModelIndexFormat
{
    /// <summary>16-bit indices (every index below 65 536).</summary>
    Uint16 = 2,

    /// <summary>32-bit indices.</summary>
    Uint32 = 4,
}
