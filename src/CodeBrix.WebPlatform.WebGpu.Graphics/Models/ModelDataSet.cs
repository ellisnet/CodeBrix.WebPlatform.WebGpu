using System.Collections.Generic;

namespace CodeBrix.WebPlatform.WebGpu.Graphics.Models;

/// <summary>
/// A model set: what one model data blob holds (<see cref="ModelDataWriter" />). One or more models and the textures
/// they use; a texture several models use is stored once and each model names it by its position in
/// <see cref="Textures" />.
/// </summary>
public sealed class ModelDataSet
{
    /// <summary>The models, in order (a reader finds them by position or by name).</summary>
    public List<ModelData> Models { get; set; } = new List<ModelData>();

    /// <summary>Every texture any model of the set uses, each once (<see cref="ModelData.Textures" /> holds positions in this list).</summary>
    public List<ModelTextureData> Textures { get; set; } = new List<ModelTextureData>();
}
