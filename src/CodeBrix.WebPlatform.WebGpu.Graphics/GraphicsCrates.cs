using System;
using System.IO;
using System.Reflection;
using System.Text;
using CodeBrix.WebPlatform.Core.Crates;

namespace CodeBrix.WebPlatform.WebGpu.Graphics;

/// <summary>
/// The Rust crate of this package, <c>codebrix-webgpu-graphics</c> (graphics through WebGPU as a
/// layer on the canvas host of <c>codebrix-webgpu</c>), in vendored form, and the third-party
/// crates only it needs (<c>gltf</c>, <c>gltf-json</c>, <c>gltf-derive</c> and what they bring
/// beyond the platform's and the WebGpu package's crates), exactly as crates.io published them,
/// with the notices for them. The package carries them as ONE archive, which it copies beside the
/// build program as <c>&lt;build program folder&gt;/webgpu-graphics-crates/webgpu-graphics-crates.zip</c>,
/// with <c>THIRD-PARTY-NOTICES-WebGpuGraphicsCrates.txt</c> beside it. The archive holds only what
/// this package adds: the crates the Graphics crate builds on (<c>wgpu</c>, <c>glam</c>,
/// <c>codebrix-webgpu</c>) and those it shares with the platform (<c>serde</c>,
/// <c>serde_json</c>, ...) come from the WebGpu package's bundle and the platform, and
/// <see cref="Register"/> registers the WebGpu bundle first. The build program calls <see cref="Register"/> before it materializes the
/// frontend; the frontend build then writes the crates into the frontend beside the platform's,
/// and the frontend names <c>codebrix-webgpu-graphics</c> in its own <c>Cargo.toml</c>.
/// </summary>
/// <example>
/// <code>
/// GraphicsCrates.Register();
/// await PlatformFrontend.MaterializeAsync(options);
/// </code>
/// </example>
public static class GraphicsCrates
{
    /// <summary>The name of the crate bundle this package registers, unique among the bundles of one build.</summary>
    public const string BundleName = "webgpu-graphics";

    /// <summary>The id of the NuGet package that carries the crates; error messages name it.</summary>
    public const string PackageId = "CodeBrix.WebPlatform.WebGpu.Graphics.ApacheLicenseForever";

    /// <summary>The folder, beside the build program, that holds the archive and the notices.</summary>
    public const string FolderName = "webgpu-graphics-crates";

    /// <summary>The file name of the archive of the crates.</summary>
    public const string ArchiveFileName = "webgpu-graphics-crates.zip";

    /// <summary>
    /// The file name of the notices for what the archive brings: the Graphics crate is this
    /// repository's own code, and the notices name the outside sources of parts of it (its shaders
    /// and its .hdr decoder) and list the third-party crates of the archive with their licences. A
    /// built site that uses the crate should carry it, beside the WebGpu package's notices
    /// (<see cref="WebGpuCrates.NoticesFileName"/>).
    /// </summary>
    public const string NoticesFileName = "THIRD-PARTY-NOTICES-WebGpuGraphicsCrates.txt";

    /// <summary>
    /// The version of this package, read from the assembly (the date-stamp version it was built
    /// with), in the form NuGet gives a package version.
    /// </summary>
    internal static string PackageVersion { get; } = NormalizeVersion(
        typeof(GraphicsCrates).GetTypeInfo().Assembly.GetCustomAttribute<AssemblyInformationalVersionAttribute>()?.InformationalVersion
        ?? typeof(GraphicsCrates).GetTypeInfo().Assembly.GetName().Version.ToString());

    /// <summary>
    /// Registers the WebGpu package's crates (<see cref="WebGpuCrates.Register"/>) FIRST, because
    /// this package's crate builds on them, and then this package's crates, as the bundle named
    /// <see cref="BundleName"/> (<see cref="CrateBundles.Register(CrateBundle)"/>). Call it in the
    /// build program, before the frontend is materialized; calling it again (or after
    /// <see cref="WebGpuCrates.Register"/>) changes nothing. The archive is opened from
    /// <see cref="GetArchivePath"/> when the build needs it.
    /// </summary>
    /// <exception cref="InvalidOperationException">Another package, or another version of this
    /// package or of the WebGpu package, has already registered a bundle of the same name.</exception>
    public static void Register()
    {
        WebGpuCrates.Register();
        CrateBundles.Register(CreateBundle(null));
    }

    /// <summary>
    /// The folder that holds the archive and the notices below an application's base folder.
    /// </summary>
    /// <param name="applicationBaseDirectory">The build program's base folder; <c>null</c> means
    /// <see cref="AppContext.BaseDirectory"/>.</param>
    public static string GetDirectory(string applicationBaseDirectory = null)
        => Path.Combine(string.IsNullOrWhiteSpace(applicationBaseDirectory) ? AppContext.BaseDirectory : applicationBaseDirectory, FolderName);

    /// <summary>The archive of the crates below an application's base folder.</summary>
    /// <param name="applicationBaseDirectory">The build program's base folder; <c>null</c> means
    /// <see cref="AppContext.BaseDirectory"/>.</param>
    public static string GetArchivePath(string applicationBaseDirectory = null) => Path.Combine(GetDirectory(applicationBaseDirectory), ArchiveFileName);

    /// <summary>The notices file below an application's base folder.</summary>
    /// <param name="applicationBaseDirectory">The build program's base folder; <c>null</c> means
    /// <see cref="AppContext.BaseDirectory"/>.</param>
    public static string GetNoticesPath(string applicationBaseDirectory = null) => Path.Combine(GetDirectory(applicationBaseDirectory), NoticesFileName);

    /// <summary>
    /// The notices for what this package's crates bring into an application's WebAssembly file.
    /// Neither the platform's notices nor the WebGpu package's cover it: an application serves
    /// these beside those and its own.
    /// </summary>
    /// <param name="applicationBaseDirectory">The build program's base folder; <c>null</c> means
    /// <see cref="AppContext.BaseDirectory"/>.</param>
    /// <returns>The text (UTF-8, line feeds).</returns>
    /// <exception cref="FileNotFoundException">The notices file is not beside the build program
    /// (the package is not referenced from the build program, was not restored, or the program
    /// was copied without its webgpu-graphics-crates folder).</exception>
    public static string ReadNotices(string applicationBaseDirectory = null)
    {
        var path = GetNoticesPath(applicationBaseDirectory);
        if (!File.Exists(path))
        {
            throw new FileNotFoundException(MissingMessage(path), path);
        }

        return File.ReadAllText(path, Encoding.UTF8);
    }

    /// <summary>The bundle <see cref="Register"/> hands over, opening the archive below a base folder.</summary>
    /// <param name="applicationBaseDirectory">The build program's base folder; <c>null</c> means
    /// <see cref="AppContext.BaseDirectory"/> at the time the archive is opened.</param>
    internal static CrateBundle CreateBundle(string applicationBaseDirectory)
        => new CrateBundle(BundleName, PackageId, PackageVersion, () => OpenArchive(applicationBaseDirectory));

    /// <summary>Opens the archive below a base folder for reading.</summary>
    /// <param name="applicationBaseDirectory">The build program's base folder; <c>null</c> means
    /// <see cref="AppContext.BaseDirectory"/>.</param>
    /// <exception cref="FileNotFoundException">The archive is not there.</exception>
    internal static Stream OpenArchive(string applicationBaseDirectory)
    {
        var path = GetArchivePath(applicationBaseDirectory);
        if (!File.Exists(path))
        {
            throw new FileNotFoundException(MissingMessage(path), path);
        }

        return File.OpenRead(path);
    }

    /// <summary>
    /// A version as NuGet writes a package version: without build metadata (the text after a
    /// <c>+</c>) and without a fourth part that is zero.
    /// </summary>
    /// <param name="version">The version text.</param>
    internal static string NormalizeVersion(string version)
    {
        var text = version.Trim();
        var plus = text.IndexOf('+');
        if (plus >= 0)
        {
            text = text.Substring(0, plus);
        }

        var parts = text.Split('.');
        return parts.Length == 4 && parts[3] == "0" ? string.Join(".", parts, 0, 3) : text;
    }

    internal static string MissingMessage(string path)
        => $"'{path}' does not exist. The {PackageId} package copies its {FolderName} folder beside the build program when the program is built: reference the package from the build program (not only from the frontend's other projects), then restore and build the build program again.";
}
