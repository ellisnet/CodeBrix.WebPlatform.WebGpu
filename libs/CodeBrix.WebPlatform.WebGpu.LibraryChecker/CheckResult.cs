using System;
using System.Collections.Generic;
using System.Linq;

namespace CodeBrix.WebPlatform.WebGpu.LibraryChecker;

/// <summary>
/// The outcome of one check: its name, whether it passed, and what it has to say.
/// </summary>
public sealed class CheckResult
{
    /// <summary>
    /// Creates a result.
    /// </summary>
    /// <param name="name">The check's name.</param>
    /// <param name="problems">What is wrong; empty when the check passed.</param>
    /// <param name="notes">Information that is not a failure.</param>
    public CheckResult(string name, IEnumerable<string> problems, IEnumerable<string> notes = null)
    {
        Name = name ?? string.Empty;
        Problems = (problems ?? Enumerable.Empty<string>()).ToList();
        Notes = (notes ?? Enumerable.Empty<string>()).ToList();
    }

    /// <summary>The check's name.</summary>
    public string Name { get; }

    /// <summary>What is wrong, one line each; empty when the check passed.</summary>
    public IReadOnlyList<string> Problems { get; }

    /// <summary>Information that is not a failure (counts, what was covered).</summary>
    public IReadOnlyList<string> Notes { get; }

    /// <summary>True when there are no problems.</summary>
    public bool Passed => Problems.Count == 0;

    /// <summary>
    /// Builds the text shown for this check: a PASS / FAIL line, then the notes and problems.
    /// </summary>
    public string ToText()
    {
        var lines = new List<string> { (Passed ? "PASS  " : "FAIL  ") + Name };
        lines.AddRange(Notes.Select(n => "        " + n));
        lines.AddRange(Problems.Select(p => "      ! " + p));
        return string.Join(Environment.NewLine, lines);
    }

    /// <inheritdoc />
    public override string ToString() => ToText();
}
