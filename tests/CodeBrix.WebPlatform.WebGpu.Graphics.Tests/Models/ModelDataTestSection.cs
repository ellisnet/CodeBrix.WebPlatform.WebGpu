namespace CodeBrix.WebPlatform.WebGpu.Graphics.Tests.Models;

/// <summary>One section table entry of a model data blob, as <see cref="ModelDataTestReader" /> reads it.</summary>
internal sealed record ModelDataTestSection(string Tag, int RecordSize, int Count, int Flags, int Offset, int Length);
