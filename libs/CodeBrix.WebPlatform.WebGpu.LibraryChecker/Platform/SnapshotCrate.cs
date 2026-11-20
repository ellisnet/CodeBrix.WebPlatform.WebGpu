using System;

namespace CodeBrix.WebPlatform.WebGpu.LibraryChecker.Platform;

/// <summary>
/// One vendored crate of a <see cref="CrateSnapshot"/>: its folder, the name and version its
/// Cargo.toml declares, and the bytes of its <c>.cargo-checksum.json</c> (what the platform's
/// strict rule compares).
/// </summary>
public sealed class SnapshotCrate
{
    /// <summary>
    /// Creates a crate entry.
    /// </summary>
    /// <param name="folder">The crate's folder name (name-version).</param>
    /// <param name="name">The crate's name.</param>
    /// <param name="version">The crate's version.</param>
    /// <param name="checksum">The bytes of the crate's <c>.cargo-checksum.json</c>.</param>
    public SnapshotCrate(string folder, string name, string version, byte[] checksum)
    {
        Folder = folder ?? string.Empty;
        Name = name ?? string.Empty;
        Version = version ?? string.Empty;
        Checksum = checksum ?? Array.Empty<byte>();
    }

    /// <summary>The crate's folder name (name-version).</summary>
    public string Folder { get; }

    /// <summary>The crate's name.</summary>
    public string Name { get; }

    /// <summary>The crate's version.</summary>
    public string Version { get; }

    /// <summary>The bytes of the crate's <c>.cargo-checksum.json</c>.</summary>
    public byte[] Checksum { get; }

    /// <summary>True when another crate entry has exactly the same checksum file.</summary>
    /// <param name="other">The other crate.</param>
    public bool HasSameContentAs(SnapshotCrate other)
        => other != null && Checksum.AsSpan().SequenceEqual(other.Checksum);

    /// <inheritdoc />
    public override string ToString() => $"{Name} {Version}";
}
