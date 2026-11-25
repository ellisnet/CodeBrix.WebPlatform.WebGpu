using CodeBrix.WebPlatform.Core.Mapping;

namespace GraphicsDemo.Api.Contracts;

/// <summary>
/// One model of the scene and where the frontend gets it: a MODEL FILE served unchanged (GET
/// api/models/files/&lt;File&gt;, read in the browser by the graphics crate's glTF loader with <see cref="Pack" /> as the
/// base address of the files it names), or - for a model built in code, which has no file - its model data blob (GET
/// api/models/&lt;Name&gt;.cbmd, read by the graphics crate's blob reader).
/// </summary>
[WireRequired]
public sealed class SceneModel
{
    /// <summary>The model's name, such as "tree" or "ground".</summary>
    public string Name { get; set; }

    /// <summary>The model's index in the scene's list of models (what <see cref="ScenePlacement.ModelIndex" /> and <see cref="SceneActor.ModelIndex" /> name).</summary>
    public int Index { get; set; }

    /// <summary>The model file's path below the models folder, such as "survival-kit/tree.glb"; empty for a model built in code (served as a blob).</summary>
    public string File { get; set; }

    /// <summary>The folder of the model file below the models folder (its pack, such as "survival-kit"): the base address of the textures and buffers the file names; empty for a model built in code.</summary>
    public string Pack { get; set; }
}
