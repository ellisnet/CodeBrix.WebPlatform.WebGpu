using System;
using System.IO;
using System.IO.Compression;
using System.Linq;
using System.Text.RegularExpressions;
using System.Threading.Tasks;
using CodeBrix.WebPlatform.FrontendBuilder;
using CodeBrix.WebPlatform.FrontendBuilder.Crates;
using Xunit;

namespace CodeBrix.WebPlatform.WebGpu.Graphics.Tests.Materialize;

/// <summary>
/// ONE frontend, materialized for real by the platform's <see cref="PlatformFrontend.MaterializeAsync"/>
/// after <see cref="GraphicsCrates.Register"/> (the WebGpu bundle, then this package's), in a
/// temporary folder: a Cargo.toml that names the platform's shared crate, this package's crate
/// <c>codebrix-webgpu-graphics</c> at its version, <c>codebrix-webgpu</c> at its version and
/// <c>dioxus</c> at the version that crate names, and a main.rs (Materialize/frontend-main.rs);
/// the client is read from the WebGpu tests' fixture backend. The platform's crates archive comes
/// from the FrontendBuilder package (beside the test assembly, as beside a build program); the
/// WebGpu and Graphics archives from the projects' output. Created once for the collection
/// <see cref="MaterializeCollection"/>.
/// </summary>
public sealed class MaterializedFrontend : IAsyncLifetime
{
    /// <summary>The temporary folder that holds the frontend; deleted afterwards.</summary>
    public string Folder { get; } = Path.Combine(Path.GetTempPath(), "webgpu-graphics-materialize-tests-" + Guid.NewGuid().ToString("N"));

    /// <summary>The frontend folder.</summary>
    public string FrontendDirectory => Path.Combine(Folder, "frontend");

    /// <summary>The version of this package's crate codebrix-webgpu-graphics (from its Cargo.toml).</summary>
    public string GraphicsCrateVersion { get; private set; }

    /// <summary>The version of the WebGpu package's crate codebrix-webgpu (from its Cargo.toml).</summary>
    public string WebGpuCrateVersion { get; private set; }

    /// <summary>The options the frontend was materialized with.</summary>
    public PlatformFrontendOptions Options { get; private set; }

    /// <summary>The builder; a test may materialize again with it.</summary>
    public PlatformFrontend Frontend { get; private set; }

    /// <summary>The result of the first Materialize.</summary>
    public MaterializeResult First { get; private set; }

    /// <summary>
    /// Why nothing was materialized: the WebGpu archive holds no wgpu crate (a build without the
    /// WebGpu project's Rust/vendor), so a frontend could not resolve. <c>null</c> when the
    /// frontend was materialized.
    /// </summary>
    public string NoCratesReason { get; private set; }

    /// <summary>What the frontend build wrote to its output.</summary>
    public StringWriter Log { get; } = new StringWriter();

    /// <summary>The fixture backend, built beside the test assembly.</summary>
    public static string SampleBackendAssembly => Path.Combine(AppContext.BaseDirectory, "CodeBrix.WebPlatform.WebGpu.Tests.SampleBackend.dll");

    /// <summary>This package's crate's Cargo.toml, copied beside the test assembly by the project file.</summary>
    public static string GraphicsCrateManifest => Path.Combine(AppContext.BaseDirectory, "codebrix-webgpu-graphics", "Cargo.toml");

    /// <summary>The Cargo.toml of codebrix-webgpu, copied beside the test assembly by the project file.</summary>
    public static string WebGpuCrateManifest => Path.Combine(AppContext.BaseDirectory, "codebrix-webgpu", "Cargo.toml");

    /// <summary>The test frontend's main.rs, copied beside the test assembly by the project file.</summary>
    public static string FrontendMain => Path.Combine(AppContext.BaseDirectory, "Materialize", "frontend-main.rs");

    /// <inheritdoc />
    public async ValueTask InitializeAsync()
    {
        GraphicsCrateVersion = Regex.Match(File.ReadAllText(GraphicsCrateManifest), @"(?m)^version\s*=\s*""([^""]+)""").Groups[1].Value;
        var webGpu = File.ReadAllText(WebGpuCrateManifest);
        WebGpuCrateVersion = Regex.Match(webGpu, @"(?m)^version\s*=\s*""([^""]+)""").Groups[1].Value;
        var dioxusVersion = Regex.Match(webGpu, @"(?m)^dioxus\s*=\s*\{\s*version\s*=\s*""([^""]+)""").Groups[1].Value;
        if (GraphicsCrateVersion.Length == 0 || WebGpuCrateVersion.Length == 0 || dioxusVersion.Length == 0)
        {
            throw new InvalidOperationException($"The Cargo.toml of codebrix-webgpu-graphics ({GraphicsCrateManifest}) or of codebrix-webgpu ({WebGpuCrateManifest}) names no version on a line of its own, or codebrix-webgpu's no `dioxus = {{ version = ... }}` line; write them so.");
        }

        Directory.CreateDirectory(Path.Combine(FrontendDirectory, "src"));
        File.WriteAllText(Path.Combine(FrontendDirectory, "Cargo.toml"),
            "[package]\nname = \"webgpu-graphics-test-frontend\"\nversion = \"0.1.0\"\nedition = \"2021\"\npublish = false\n\n"
            + "[dependencies]\ncodebrix-webplatform = { path = \"platform/codebrix-webplatform\" }\n"
            + "codebrix-webgpu-graphics = \"" + GraphicsCrateVersion + "\"\n"
            + "codebrix-webgpu = \"" + WebGpuCrateVersion + "\"\n"
            + "dioxus = { version = \"" + dioxusVersion + "\", default-features = false, features = [\"minimal\", \"web\", \"mounted\"] }\n");
        File.Copy(FrontendMain, Path.Combine(FrontendDirectory, "src", "main.rs"));

        using (var archive = ZipFile.OpenRead(WebGpuCrates.GetArchivePath()))
        {
            if (!archive.Entries.Any(e => e.FullName.StartsWith("wgpu-", StringComparison.Ordinal)))
            {
                NoCratesReason = "The WebGpu archive holds no wgpu crate: src/CodeBrix.WebPlatform.WebGpu/Rust/vendor is empty or missing, so a frontend cannot resolve.";
                return;
            }
        }

        GraphicsCrates.Register();
        Options = new PlatformFrontendOptions
        {
            FrontendDirectory = FrontendDirectory,
            BackendAssemblyPath = SampleBackendAssembly,
            CrateMode = CrateMode.PlatformCratesOnly,
            Output = Log,
        };
        Frontend = new PlatformFrontend(Options);
        First = await Frontend.MaterializeAsync(TestContext.Current.CancellationToken);
    }

    /// <inheritdoc />
    public ValueTask DisposeAsync()
    {
        if (Directory.Exists(Folder))
        {
            Directory.Delete(Folder, recursive: true);
        }

        return ValueTask.CompletedTask;
    }
}
