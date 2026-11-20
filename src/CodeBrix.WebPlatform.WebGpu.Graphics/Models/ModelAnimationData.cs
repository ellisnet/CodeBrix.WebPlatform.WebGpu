using System.Collections.Generic;

namespace CodeBrix.WebPlatform.WebGpu.Graphics.Models;

/// <summary>A glTF animation clip of a <see cref="ModelData" />: a name and its channels.</summary>
public sealed class ModelAnimationData
{
    /// <summary>The name, or null.</summary>
    public string Name { get; set; }

    /// <summary>The channels (node-property channels).</summary>
    public List<ModelAnimationChannelData> Channels { get; set; } = new List<ModelAnimationChannelData>();
}
