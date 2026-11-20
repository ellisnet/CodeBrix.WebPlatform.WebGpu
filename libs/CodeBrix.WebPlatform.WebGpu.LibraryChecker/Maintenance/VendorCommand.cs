using System;
using System.IO;
using System.Text;
using System.Threading;
using System.Threading.Tasks;
using CodeBrix.RustTools;
using CodeBrix.RustTools.Projects;
using CodeBrix.RustTools.Vendoring;
using CodeBrix.WebPlatform.WebGpu.LibraryChecker.Platform;

namespace CodeBrix.WebPlatform.WebGpu.LibraryChecker.Maintenance;

/// <summary>
/// Re-resolves the crate set against the platform's Cargo.lock and re-takes the vendored wgpu
/// crates with <c>cargo vendor</c>, the way Rust/vendoring/wgpu/UPGRADE-README.txt describes it
/// (THE LOCK IS RESOLVED AGAINST THE PLATFORM'S LOCK). This is the one step that needs network
/// access to crates.io; it is run by hand when the pinned version or a feature changes, or when
/// the platform release moves, never as part of a build.
/// </summary>
public sealed class VendorCommand
{
    private readonly RepositoryLayout _layout;
    private readonly TextWriter _output;

    /// <summary>
    /// Creates the command.
    /// </summary>
    /// <param name="layout">The repository's folders.</param>
    /// <param name="output">Where progress lines go.</param>
    public VendorCommand(RepositoryLayout layout, TextWriter output)
    {
        ArgumentNullException.ThrowIfNull(layout);
        _layout = layout;
        _output = output ?? TextWriter.Null;
    }

    /// <summary>
    /// Re-resolves and re-vendors: (1) copies the crate set's Cargo.toml and src/ into a scratch
    /// folder (without its .cargo folder, so that cargo reads crates.io) with the platform's
    /// Cargo.lock - the one the referenced FrontendBuilder carries - as its lock; (2) lets cargo
    /// resolve there, which keeps every platform version that satisfies the requirements, adds
    /// what is missing and drops what the crate set does not use; (3) runs
    /// <c>cargo vendor --locked --versioned-dirs</c> into a fresh folder; (4) replaces
    /// <c>Rust/vendor</c> and the crate set's Cargo.lock with the result.
    /// </summary>
    /// <param name="cancellationToken">Stops cargo.</param>
    /// <exception cref="MaintenanceException">cargo failed, or the crate set is incomplete.</exception>
    public async Task VendorAsync(CancellationToken cancellationToken = default)
    {
        var manifest = Path.Combine(_layout.CrateSet, "Cargo.toml");
        if (!File.Exists(manifest))
        {
            throw new MaintenanceException($"The crate set's manifest '{_layout.Relative(manifest)}' does not exist; nothing to vendor.");
        }

        _output.WriteLine($"Re-resolving the wgpu crate set against the platform's Cargo.lock (CodeBrix.WebPlatform {PlatformRelease.Version}) and vendoring it (this contacts crates.io) ...");
        var scratch = Path.Combine(Path.GetTempPath(), "codebrix-webgpu-revendor-" + Guid.NewGuid().ToString("N"));
        var staging = Path.Combine(Path.GetDirectoryName(_layout.Vendor), ".revendor-" + Guid.NewGuid().ToString("N"));
        try
        {
            PrepareScratch(scratch);
            var workbench = new RustWorkbench();
            var project = workbench.OpenProject(scratch);
            try
            {
                await project.GetMetadataAsync(new CargoMetadataOptions(), cancellationToken).ConfigureAwait(false);
            }
            catch (RustToolsException exception)
            {
                throw new MaintenanceException($"cargo could not resolve the crate set against the platform's Cargo.lock: {exception.Message}", exception);
            }

            CargoVendorResult result;
            try
            {
                result = await workbench.VendorAsync(
                    workbench.OpenProject(scratch),
                    new CargoVendorOptions { VendorDirectory = Path.Combine(staging, "vendor"), VersionedDirectories = true, Locked = true },
                    cancellationToken).ConfigureAwait(false);
            }
            catch (RustToolsException exception)
            {
                throw new MaintenanceException($"Vendoring the wgpu crates could not start: {exception.Message}", exception);
            }

            if (!result.Success)
            {
                throw new MaintenanceException($"Vendoring the wgpu crates failed: {result.ErrorMessage}");
            }

            if (Directory.Exists(_layout.Vendor))
            {
                Directory.Delete(_layout.Vendor, recursive: true);
            }

            Directory.Move(result.VendorDirectory, _layout.Vendor);
            File.Copy(Path.Combine(scratch, CargoLockFile.FileName), _layout.CrateSetLock, overwrite: true);
            _output.WriteLine($"  {VendorFolder.Load(_layout.Vendor).Crates.Count} crates in {_layout.VendorRelative}; {_layout.Relative(_layout.CrateSetLock)} replaced.");
            _output.WriteLine("  Now run `LibraryChecker` (every check, the platform check included), read the licence files of every");
            _output.WriteLine("  new crate, regenerate the notices (`LibraryChecker notices --write`) and rebuild the solution.");
        }
        finally
        {
            DeleteQuietly(scratch);
            DeleteQuietly(staging);
        }
    }

    /// <summary>
    /// Writes the scratch project: the crate set's Cargo.toml and src/ folder, and the platform's
    /// Cargo.lock as its lock file. The .cargo folder is not copied.
    /// </summary>
    /// <param name="scratch">The scratch folder (created).</param>
    internal void PrepareScratch(string scratch)
    {
        Directory.CreateDirectory(scratch);
        File.Copy(Path.Combine(_layout.CrateSet, "Cargo.toml"), Path.Combine(scratch, "Cargo.toml"));
        var source = Path.Combine(_layout.CrateSet, "src");
        if (Directory.Exists(source))
        {
            foreach (var file in Directory.EnumerateFiles(source, "*", SearchOption.AllDirectories))
            {
                var target = Path.Combine(scratch, "src", Path.GetRelativePath(source, file));
                Directory.CreateDirectory(Path.GetDirectoryName(target));
                File.Copy(file, target);
            }
        }

        File.WriteAllText(Path.Combine(scratch, CargoLockFile.FileName), PlatformRelease.LockFileText, new UTF8Encoding(false));
    }

    private static void DeleteQuietly(string directory)
    {
        try
        {
            if (Directory.Exists(directory))
            {
                Directory.Delete(directory, recursive: true);
            }
        }
        catch (Exception exception) when (exception is IOException or UnauthorizedAccessException)
        {
            // Best effort: a scratch folder.
        }
    }
}
