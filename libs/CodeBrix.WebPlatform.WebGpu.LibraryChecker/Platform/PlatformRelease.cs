using System;
using System.IO;
using System.Reflection;
using CodeBrix.WebPlatform.FrontendBuilder.Crates;

namespace CodeBrix.WebPlatform.WebGpu.LibraryChecker.Platform;

/// <summary>
/// The CodeBrix.WebPlatform release this repository's crates are checked against: the
/// FrontendBuilder package this tool references, whose archive of the platform's crates is copied
/// beside the tool (<c>frontend-crates/platform-crates.zip</c>) and whose assembly embeds the
/// platform's Cargo.lock.
/// </summary>
public static class PlatformRelease
{
    /// <summary>
    /// The platform version: the version of the FrontendBuilder assembly this tool was built
    /// against (the family's date-stamp version, which is also the package version).
    /// </summary>
    public static string Version
    {
        get
        {
            var version = typeof(PlatformCrates).GetTypeInfo().Assembly.GetName().Version;
            return version == null ? string.Empty : version.ToString();
        }
    }

    /// <summary>The platform's crates archive beside this tool.</summary>
    public static string ArchivePath => PlatformCrates.GetArchivePath(AppContext.BaseDirectory);

    /// <summary>The platform's Cargo.lock, as the FrontendBuilder assembly carries it.</summary>
    public static string LockFileText => PlatformCrates.LockFileText;

    /// <summary>
    /// Reads the platform's crates archive.
    /// </summary>
    /// <exception cref="FileNotFoundException">The archive is not beside the tool (the
    /// FrontendBuilder package was not restored, or the tool's output was copied without its
    /// frontend-crates folder).</exception>
    /// <exception cref="InvalidDataException">The archive cannot be read.</exception>
    public static CrateSnapshot LoadCrates()
    {
        var path = ArchivePath;
        if (!File.Exists(path))
        {
            throw new FileNotFoundException($"The platform's crates archive '{path}' does not exist. The CodeBrix.WebPlatform.FrontendBuilder package copies it beside LibraryChecker when the tool is built; restore and build the tool again.", path);
        }

        return CrateSnapshot.FromArchive(path);
    }
}
