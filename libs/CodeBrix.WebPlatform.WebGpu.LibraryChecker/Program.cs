using System;
using System.IO;
using System.Linq;
using System.Threading;
using System.Threading.Tasks;
using CodeBrix.WebPlatform.WebGpu.LibraryChecker.Maintenance;

namespace CodeBrix.WebPlatform.WebGpu.LibraryChecker;

/// <summary>
/// The LibraryChecker command line: the maintainer tool of the CodeBrix.WebPlatform.WebGpu repository.
/// </summary>
public static class Program
{
    private const string Usage = """
        LibraryChecker - the maintainer gate of the CodeBrix.WebPlatform.WebGpu repository. It checks what
        the repository holds outside samples/; it is not published and is no part of any package.

        Usage:  LibraryChecker [<check>] [--write] [--root <repository root>]
                LibraryChecker revendor

        Checks:
          all          every check below (the default)
          no-node      no package.json, node_modules or JavaScript of ours outside samples/
          vendor       the vendored wgpu crates are untouched and are exactly the crate set's derived
                       set; the Graphics and the Animation packages' vendored crates are exactly
                       their crate sets' own crates; git hides no vendored file; no GitHub workflow
                       files
          platform     every vendored crate the platform also carries is at the platform's version with
                       the same content (the strict rule), and the Graphics and the Animation crate
                       sets share every crate the WebGpu package or the platform carries at its
                       version; no crate is vendored by both of them in two versions; the
                       packages pin Core to that platform
                       version; Graphics and Animation pin the WebGpu package of the same build
                       exactly (the family rule)
          first-party  the first-party crates codebrix-webgpu, codebrix-webgpu-graphics and
                       codebrix-webgpu-animation: ours
                       (Apache-2.0, not published), every dependency at an exact version the vendored
                       or the platform's crates carry (or codebrix-webgpu at its own version; the
                       Graphics and Animation crates also their own package's vendored crates)
          dependency-rule
                       Graphics and Animation (project and crate) both depend on WebGpu and never
                       name each other
          licenses     every crate's license is on the allowlist for its tier (the WebGpu, the
                       Graphics and the Animation crate sets)
          notices      the generated notices (the WebGpu, the Graphics and the Animation packages')
                       are up to date (--write regenerates them); the Graphics and the Animation
                       packages' hand-written headers are complete and their notices packed

        Maintenance (not a check):
          revendor     re-take the wgpu crates from crates.io (needs the network)

        Exit code 0 when every check passes, 1 when one fails, 2 for a usage error.
        """;

    /// <summary>
    /// Runs the tool.
    /// </summary>
    /// <param name="args">The command-line arguments.</param>
    /// <returns>0 when every check passed, 1 when one failed, 2 for a usage error.</returns>
    public static async Task<int> Main(string[] args)
    {
        var group = "all";
        var write = false;
        string root = null;
        for (var i = 0; i < args.Length; i++)
        {
            switch (args[i])
            {
                case "-h" or "--help":
                    Console.WriteLine(Usage);
                    return 0;
                case "--write":
                    write = true;
                    break;
                case "--root" when i + 1 < args.Length:
                    root = args[++i];
                    break;
                default:
                    if (args[i].StartsWith('-') || group != "all")
                    {
                        Console.Error.WriteLine($"Unexpected argument '{args[i]}'.{Environment.NewLine}{Environment.NewLine}{Usage}");
                        return 2;
                    }

                    group = args[i];
                    break;
            }
        }

        var layout = root != null
            ? (RepositoryLayout.IsRoot(root) ? new RepositoryLayout(root) : null)
            : RepositoryLayout.Find(Directory.GetCurrentDirectory()) ?? RepositoryLayout.Find(AppContext.BaseDirectory);
        if (layout == null)
        {
            Console.Error.WriteLine($"The CodeBrix.WebPlatform.WebGpu repository root (the folder with {RepositoryLayout.SolutionFile} and {RepositoryLayout.MarkerFile}) was not found. Run from inside it or pass --root.");
            return 2;
        }

        using var cancellation = new CancellationTokenSource();
        Console.CancelKeyPress += (_, e) =>
        {
            e.Cancel = true;
            cancellation.Cancel();
        };

        try
        {
            if (string.Equals(group, "revendor", StringComparison.OrdinalIgnoreCase))
            {
                await new VendorCommand(layout, Console.Out).VendorAsync(cancellation.Token).ConfigureAwait(false);
                return 0;
            }

            var results = await CheckRunner.RunAsync(layout, group, write, cancellation.Token).ConfigureAwait(false);
            foreach (var result in results)
            {
                Console.WriteLine(result.ToText());
            }

            var failed = results.Count(r => !r.Passed);
            Console.WriteLine();
            Console.WriteLine(failed == 0 ? $"All {results.Count} checks passed." : $"{failed} of {results.Count} checks FAILED.");
            return failed == 0 ? 0 : 1;
        }
        catch (ArgumentException exception)
        {
            Console.Error.WriteLine(exception.Message);
            return 2;
        }
        catch (MaintenanceException exception)
        {
            Console.Error.WriteLine("FAILED: " + exception.Message);
            return 1;
        }
        catch (OperationCanceledException)
        {
            Console.Error.WriteLine("Cancelled.");
            return 1;
        }
        catch (Exception exception) when (exception is IOException or InvalidDataException or CodeBrix.RustTools.RustToolsException)
        {
            Console.Error.WriteLine("LibraryChecker could not run: " + exception.Message);
            return 1;
        }
    }
}
