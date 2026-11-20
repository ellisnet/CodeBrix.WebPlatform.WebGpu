using System;
using System.Diagnostics;
using System.IO;
using System.Linq;
using System.Threading;
using System.Threading.Tasks;
using CodeBrix.WebPlatform.FrontendBuilder;
using CodeBrix.WebPlatform.FrontendBuilder.Applications;
using CodeBrix.WebPlatform.FrontendBuilder.Crates;
using CodeBrix.WebPlatform.FrontendBuilder.Prerequisites;
using CodeBrix.WebPlatform.WebGpu;
using CodeBrix.WebPlatform.WebGpu.Animation;
using CodeBrix.WebPlatform.WebGpu.Graphics;

namespace GraphicsDemo.BuildFrontend;

/// <summary>
/// The build program of the GraphicsDemo sample. It registers the crate bundles of the
/// CodeBrix.WebPlatform.WebGpu.Graphics package (codebrix-webgpu-graphics), of the
/// CodeBrix.WebPlatform.WebGpu.Animation package (codebrix-webgpu-animation, kurbo, peniko) and,
/// through either, of the CodeBrix.WebPlatform.WebGpu package (wgpu, glam, codebrix-webgpu) - all
/// three packages, so that it is also the proof that bundles carrying the same crates are merged
/// into one copy - tells the platform's FrontendBuilder
/// about the sample's TWO named frontend applications - they share the one backend - and lets
/// it check the machine, materialize them, build them into the backend's web root, or serve one
/// of them with hot reload.
/// </summary>
public static class Program
{
    private const string Url = "http://localhost:5220";

    private const string Usage = """
        Usage: dotnet run --project build/GraphicsDemo.BuildFrontend -- <command>

          check            say what this machine has and what it lacks for a build
          generate         write the generated client code, the shared crate and the crates
                           (the platform's and the bundles') into the two frontends (after
                           this, plain cargo works there)
          build [--debug]  generate, then build the two frontends into the backend's wwwroot
          run [--debug]    build, then start the backend (http://localhost:5220/)
          serve <name>     generate, then run `dx serve` (hot reload) for one application:
                           graphics or animation
        """;

    /// <summary>Runs one command.</summary>
    /// <param name="args">The command and its options.</param>
    /// <returns>0 when it worked.</returns>
    public static async Task<int> Main(string[] args)
    {
        using var stop = new CancellationTokenSource();
        Console.CancelKeyPress += (_, e) =>
        {
            e.Cancel = true;
            stop.Cancel();
        };

        // The crate bundles: registered BEFORE anything is materialized, or a frontend's
        // dependency on them is "no matching package". The Graphics and Animation packages each
        // register the WebGpu package's bundle first (wgpu, glam, codebrix-webgpu; once), then
        // their own (codebrix-webgpu-graphics; codebrix-webgpu-animation with kurbo and peniko).
        GraphicsCrates.Register();
        AnimationCrates.Register();

        var root = FindSampleRoot();
        var frontend = new PlatformFrontend(CreateOptions(root, Console.Out));
        var configuration = args.Contains("--debug") ? FrontendConfiguration.Debug : FrontendConfiguration.Release;
        try
        {
            switch (args.FirstOrDefault())
            {
                case "check":
                    var report = await frontend.CheckPrerequisitesAsync(stop.Token);
                    foreach (var item in report.Items)
                    {
                        Console.WriteLine($"  {item.Kind,-12} {item.Status,-8} {item.Message}");
                    }

                    return report.Items.All(i => i.Status != PrerequisiteStatus.Missing) ? 0 : 1;

                case "generate":
                    await MaterializeAsync(frontend, stop.Token);
                    return 0;

                case "build":
                    await BuildAsync(frontend, root, configuration, stop.Token);
                    return 0;

                case "run":
                    await BuildAsync(frontend, root, configuration, stop.Token);
                    return Run("dotnet", $"run --project \"{BackendProject(root)}\" --configuration Release --no-launch-profile --urls {Url}", root);

                case "serve":
                    await MaterializeAsync(frontend, stop.Token);
                    return await frontend.ServeAsync(args.Skip(1).FirstOrDefault(a => !a.StartsWith('-')), stop.Token);

                default:
                    Console.WriteLine(Usage);
                    return args.Length == 0 ? 0 : 2;
            }
        }
        catch (FrontendBuildException exception)
        {
            Console.Error.WriteLine();
            Console.Error.WriteLine(exception.Message);
            return 1;
        }
    }

    /// <summary>
    /// The sample's frontend applications. Both have a name, so nothing of the platform's is at
    /// the site root: "/" answers the backend's own wwwroot/index.html, which links to them.
    /// "graphics" is served below /graphics and "animation" below /animation; neither is built
    /// with bundle splitting.
    /// </summary>
    /// <param name="root">The sample's folder.</param>
    /// <param name="output">Where progress is written.</param>
    /// <returns>The options for the platform's FrontendBuilder.</returns>
    public static PlatformFrontendOptions CreateOptions(string root, TextWriter output)
    {
        var options = new PlatformFrontendOptions
        {
            BackendProjectPath = BackendProject(root),
            CrateMode = CrateMode.PlatformCratesOnly,
            Output = output,
        };
        options.Applications.Add(new FrontendApplicationOptions { Name = "graphics", FrontendDirectory = Path.Combine(root, "frontends", "graphics"), WasmSplit = false });
        options.Applications.Add(new FrontendApplicationOptions { Name = "animation", FrontendDirectory = Path.Combine(root, "frontends", "animation"), WasmSplit = false });
        return options;
    }

    private static async Task MaterializeAsync(PlatformFrontend frontend, CancellationToken cancellationToken)
    {
        var materialized = await frontend.MaterializeAsync(cancellationToken);
        // What the crate bundles brought: the crates each one added to the vendor folder, and the
        // ones the platform (or an earlier bundle) already carried identically, of which one
        // copy is kept.
        foreach (var bundle in materialized.CrateBundles)
        {
            Console.WriteLine($"  crate bundle \"{bundle.Name}\" from {bundle.Origin}: {bundle.AddedCrates.Count} crates added, {bundle.SharedCrates.Count} shared");
            Console.WriteLine("    added:  " + string.Join(", ", bundle.AddedCrates));
            Console.WriteLine("    shared: " + string.Join(", ", bundle.SharedCrates));
        }
    }

    private static async Task BuildAsync(PlatformFrontend frontend, string root, FrontendConfiguration configuration, CancellationToken cancellationToken)
    {
        await MaterializeAsync(frontend, cancellationToken);
        var webRoot = Path.Combine(root, "backend", "GraphicsDemo.Api", "wwwroot");
        var built = await frontend.BuildAsync(configuration, webRoot, cancellationToken);
        // One notices file for the whole site, written after the build: the platform's crates, the
        // wgpu crates, the graphics crate and the animation crates are compiled into the
        // applications' WebAssembly.
        File.WriteAllText(Path.Combine(webRoot, "THIRD-PARTY-NOTICES.txt"),
            PlatformCrates.ReadNotices() + "\n\n\n" + WebGpuCrates.ReadNotices() + "\n\n\n" + GraphicsCrates.ReadNotices() + "\n\n\n" + AnimationCrates.ReadNotices());
        // Beside it, the sample's own notices: the models and the HDR panorama the backend serves.
        File.Copy(Path.Combine(root, "THIRD-PARTY-NOTICES.txt"), Path.Combine(webRoot, "SAMPLE-THIRD-PARTY-NOTICES.txt"), overwrite: true);
        foreach (var application in built.Applications)
        {
            Console.WriteLine($"  {application.ApplicationName,-9} {application.WebRootFileCount,3} files in {application.WebRootApplicationDirectory}");
        }
    }

    private static string BackendProject(string root)
        => Path.Combine(root, "backend", "GraphicsDemo.Api", "GraphicsDemo.Api.csproj");

    /// <summary>
    /// The sample's folder (the one with GraphicsDemo.slnx): above the program's own folder, or -
    /// when the program was built elsewhere, with --artifacts-path - the current folder or one
    /// above it.
    /// </summary>
    private static string FindSampleRoot()
    {
        foreach (var start in new[] { AppContext.BaseDirectory, Directory.GetCurrentDirectory() })
        {
            for (var directory = new DirectoryInfo(start); directory != null; directory = directory.Parent)
            {
                if (File.Exists(Path.Combine(directory.FullName, "GraphicsDemo.slnx")))
                {
                    return directory.FullName;
                }
            }
        }

        throw new InvalidOperationException("The sample's folder (the one with GraphicsDemo.slnx) was not found above " + AppContext.BaseDirectory + " or the current folder. Run the build program from inside the sample's folder.");
    }

    private static int Run(string program, string arguments, string workingDirectory)
    {
        using var process = Process.Start(new ProcessStartInfo(program, arguments) { WorkingDirectory = workingDirectory, UseShellExecute = false });
        process.WaitForExit();
        return process.ExitCode;
    }
}
