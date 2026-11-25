using System;
using System.Collections.Generic;

namespace CodeBrix.WebPlatform.WebGpu.Graphics.Tests.GltfReference;

/// <summary>
/// One case of the reference folder (<see cref="GltfReferenceFolder" />): one model data set converted by the
/// reference converter from one or more model files with one set of options, and either the blob it produced or the
/// message with which the reference converter refused the files. One line of cases.txt.
/// </summary>
public sealed class GltfReferenceCase
{
    /// <summary>Creates a case.</summary>
    /// <param name="name">The case name ([a-z0-9-_.], unique in the folder).</param>
    /// <param name="modelPaths">The model files, relative to the folder's models/ with '/' separators, in set order.</param>
    /// <param name="options">The conversion options.</param>
    /// <param name="blobFileName">The blob's file name relative to the folder's blobs/, or null when refused.</param>
    /// <param name="refusal">The reference converter's refusal message, or null when converted.</param>
    /// <param name="statistics">The blob's statistics, or null when refused.</param>
    /// <exception cref="ArgumentNullException"><paramref name="name" />, <paramref name="modelPaths" /> or <paramref name="options" /> is null.</exception>
    public GltfReferenceCase(string name, IReadOnlyList<string> modelPaths, ModelConversionOptions options, string blobFileName, string refusal, ModelBlobStatistics statistics)
    {
        ArgumentNullException.ThrowIfNull(name);
        ArgumentNullException.ThrowIfNull(modelPaths);
        ArgumentNullException.ThrowIfNull(options);
        Name = name;
        ModelPaths = modelPaths;
        Options = options;
        BlobFileName = blobFileName;
        Refusal = refusal;
        Statistics = statistics;
    }

    /// <summary>The case name ([a-z0-9-_.], unique in the folder).</summary>
    public string Name { get; }

    /// <summary>The model files, relative to the folder's models/ with '/' separators, in set order.</summary>
    public IReadOnlyList<string> ModelPaths { get; }

    /// <summary>The conversion options of every model of the case.</summary>
    public ModelConversionOptions Options { get; }

    /// <summary>The options as cases.txt writes them (see <see cref="GltfReferenceFolder.FormatOptions" />).</summary>
    public string OptionsText => GltfReferenceFolder.FormatOptions(Options);

    /// <summary>The blob's file name relative to the folder's blobs/, or null when the reference converter refused the files.</summary>
    public string BlobFileName { get; }

    /// <summary>The reference converter's refusal message (an <see cref="System.IO.InvalidDataException" />'s), or null.</summary>
    public string Refusal { get; }

    /// <summary>True when the reference converter refused the files (no blob was written).</summary>
    public bool IsRefused => Refusal != null;

    /// <summary>The blob's statistics, or null when refused.</summary>
    public ModelBlobStatistics Statistics { get; }

    /// <summary>
    /// The case's line of cases.txt (without the line end): name, model paths joined by '|', options, and the blob file
    /// name or '!' and the refusal message with tabs and line breaks replaced by spaces, separated by one tab each.
    /// </summary>
    /// <returns>The line.</returns>
    public string ToLine()
    {
        string last = IsRefused ? "!" + Refusal.Replace('\t', ' ').Replace('\r', ' ').Replace('\n', ' ') : BlobFileName;
        return Name + "\t" + string.Join("|", ModelPaths) + "\t" + OptionsText + "\t" + last;
    }
}
