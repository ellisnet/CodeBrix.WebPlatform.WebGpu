using System;
using System.IO;
using System.Reflection;
using System.Text;
using CodeBrix.WebPlatform.Core.Crates;

namespace CodeBrix.WebPlatform.WebGpu;

/// <summary>
/// The Rust <c>wgpu</c> crates for WebGPU in the browser and the <c>glam</c> maths crate, and
/// every crate they need, exactly as crates.io published them, with the third-party notices for
/// them; and this repository's own crate <c>codebrix-webgpu</c> (the WebGPU context, the canvas
/// host with its layers and clock, the maths), in vendored form. The package carries them as
/// ONE archive, which it copies beside the build program as
/// <c>&lt;build program folder&gt;/webgpu-crates/webgpu-crates.zip</c>, with
/// <c>THIRD-PARTY-NOTICES-WebGpuCrates.txt</c> beside it. The build program calls
/// <see cref="Register"/> before it materializes the frontend; the frontend build then writes the
/// crates into the frontend beside the platform's, and the frontend names <c>codebrix-webgpu</c>
/// (or <c>wgpu</c>) in its own <c>Cargo.toml</c>.
/// </summary>
/// <example>
/// <code>
/// WebGpuCrates.Register();
/// await PlatformFrontend.MaterializeAsync(options);
/// </code>
/// </example>
public static class WebGpuCrates
{
    /// <summary>The name of the crate bundle this package registers, unique among the bundles of one build.</summary>
    public const string BundleName = "webgpu";

    /// <summary>The id of the NuGet package that carries the crates; error messages name it.</summary>
    public const string PackageId = "CodeBrix.WebPlatform.WebGpu.ApacheLicenseForever";

    /// <summary>The folder, beside the build program, that holds the archive and the notices.</summary>
    public const string FolderName = "webgpu-crates";

    /// <summary>The file name of the archive of the crates.</summary>
    public const string ArchiveFileName = "webgpu-crates.zip";

    /// <summary>
    /// The file name of the generated third-party notices for the crates in the archive. Every
    /// crate this package brings into an application's WebAssembly file is recorded in it; a
    /// built site that uses the crates should carry it.
    /// </summary>
    public const string NoticesFileName = "THIRD-PARTY-NOTICES-WebGpuCrates.txt";

    /// <summary>
    /// The version of this package, read from the assembly (the date-stamp version it was built
    /// with), in the form NuGet gives a package version.
    /// </summary>
    internal static string PackageVersion { get; } = NormalizeVersion(
        typeof(WebGpuCrates).GetTypeInfo().Assembly.GetCustomAttribute<AssemblyInformationalVersionAttribute>()?.InformationalVersion
        ?? typeof(WebGpuCrates).GetTypeInfo().Assembly.GetName().Version.ToString());

    /// <summary>
    /// Registers this package's crates with the platform's frontend build
    /// (<see cref="CrateBundles.Register(CrateBundle)"/>), as the bundle named
    /// <see cref="BundleName"/>. Call it in the build program, before the frontend is
    /// materialized; calling it again changes nothing. The archive is opened from
    /// <see cref="GetArchivePath"/> when the build needs it.
    /// </summary>
    /// <exception cref="InvalidOperationException">Another package, or another version of this
    /// package, has already registered a bundle named <see cref="BundleName"/>.</exception>
    public static void Register() => CrateBundles.Register(CreateBundle(null));

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
    /// The third-party notices for the crates this package brings into an application's
    /// WebAssembly file, each with its licence and licence text. The platform's own notices do
    /// not cover them: an application serves these beside the platform's and its own.
    /// </summary>
    /// <param name="applicationBaseDirectory">The build program's base folder; <c>null</c> means
    /// <see cref="AppContext.BaseDirectory"/>.</param>
    /// <returns>The text (UTF-8, line feeds).</returns>
    /// <exception cref="FileNotFoundException">The notices file is not beside the build program
    /// (the package is not referenced from the build program, was not restored, or the program
    /// was copied without its webgpu-crates folder).</exception>
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
