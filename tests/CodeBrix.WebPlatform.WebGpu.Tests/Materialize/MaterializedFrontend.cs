using System;
using System.Diagnostics;
using System.IO;
using System.IO.Compression;
using System.Linq;
using System.Text.RegularExpressions;
using System.Threading.Tasks;
using CodeBrix.WebPlatform.FrontendBuilder;
using CodeBrix.WebPlatform.FrontendBuilder.Crates;
using Xunit;

namespace CodeBrix.WebPlatform.WebGpu.Tests.Materialize;

/// <summary>
/// ONE frontend, materialized for real by the platform's <see cref="PlatformFrontend.MaterializeAsync"/>
/// with this package's bundle registered (<see cref="WebGpuCrates.Register"/>), in a temporary
/// folder: a Cargo.toml that names the platform's shared crate, <c>wgpu</c> exactly as the crate
/// set names it, the first-party crate <c>codebrix-webgpu</c> at its version and <c>dioxus</c> at
/// the version that crate names, and a main.rs (Materialize/frontend-main.rs) that uses the
/// crate's public API; the client is read from the fixture backend. The platform's crates archive comes from
/// the FrontendBuilder package (beside the test assembly, as beside a build program); this
/// package's archive from the WebGpu project's output. Created once for the collection
/// <see cref="MaterializeCollection"/>; that one Materialize writes both archives' crates
/// (several hundred megabytes, a few seconds).
/// </summary>
public sealed class MaterializedFrontend : IAsyncLifetime
{
    /// <summary>The temporary folder that holds the frontend; deleted afterwards.</summary>
    public string Folder { get; } = Path.Combine(Path.GetTempPath(), "webgpu-materialize-tests-" + Guid.NewGuid().ToString("N"));

    /// <summary>The frontend folder.</summary>
    public string FrontendDirectory => Path.Combine(Folder, "frontend");

    /// <summary>The <c>wgpu = ...</c> line of the crate set's Cargo.toml, as the frontend names it.</summary>
    public string WgpuDependency { get; private set; }

    /// <summary>The version of wgpu the crate set names (without the <c>=</c>).</summary>
    public string WgpuVersion { get; private set; }

    /// <summary>The version of the first-party crate codebrix-webgpu (from its Cargo.toml).</summary>
    public string FirstPartyCrateVersion { get; private set; }

    /// <summary>The options the frontend was materialized with.</summary>
    public PlatformFrontendOptions Options { get; private set; }

    /// <summary>The builder; a test may materialize again with it.</summary>
    public PlatformFrontend Frontend { get; private set; }

    /// <summary>The result of the first Materialize.</summary>
    public MaterializeResult First { get; private set; }

    /// <summary>How long the first Materialize took.</summary>
    public TimeSpan FirstDuration { get; private set; }

    /// <summary>
    /// Why nothing was materialized: the WebGpu archive holds no wgpu crate (a build without
    /// Rust/vendor), so a frontend could not resolve. <c>null</c> when the frontend was
    /// materialized.
    /// </summary>
    public string NoCratesReason { get; private set; }

    /// <summary>What the frontend build wrote to its output.</summary>
    public StringWriter Log { get; } = new StringWriter();

    /// <summary>The fixture backend, built beside the test assembly.</summary>
    public static string SampleBackendAssembly => Path.Combine(AppContext.BaseDirectory, "CodeBrix.WebPlatform.WebGpu.Tests.SampleBackend.dll");

    /// <summary>The crate set's Cargo.toml, copied beside the test assembly by the project file.</summary>
    public static string CrateSetManifest => Path.Combine(AppContext.BaseDirectory, "webgpu-crate-set", "Cargo.toml");

    /// <summary>The first-party crate's Cargo.toml, copied beside the test assembly by the project file.</summary>
    public static string FirstPartyManifest => Path.Combine(AppContext.BaseDirectory, "codebrix-webgpu", "Cargo.toml");

    /// <summary>The test frontend's main.rs, copied beside the test assembly by the project file.</summary>
    public static string FrontendMain => Path.Combine(AppContext.BaseDirectory, "Materialize", "frontend-main.rs");

    /// <inheritdoc />
    public async ValueTask InitializeAsync()
    {
        WgpuDependency = File.ReadAllLines(CrateSetManifest).Select(l => l.Trim()).Single(l => l.StartsWith("wgpu = ", StringComparison.Ordinal));
        var version = Regex.Match(WgpuDependency, @"version\s*=\s*""=([^""]+)""");
        if (!version.Success)
        {
            throw new InvalidOperationException($"The crate set's wgpu line does not name an exact version: {WgpuDependency}");
        }

        WgpuVersion = version.Groups[1].Value;
        var firstParty = File.ReadAllText(FirstPartyManifest);
        FirstPartyCrateVersion = Regex.Match(firstParty, @"(?m)^version\s*=\s*""([^""]+)""").Groups[1].Value;
        var dioxusVersion = Regex.Match(firstParty, @"(?m)^dioxus\s*=\s*\{\s*version\s*=\s*""([^""]+)""").Groups[1].Value;
        if (FirstPartyCrateVersion.Length == 0 || dioxusVersion.Length == 0)
        {
            throw new InvalidOperationException($"The Cargo.toml of codebrix-webgpu ({FirstPartyManifest}) names no version on a line of its own, or no `dioxus = {{ version = ... }}` line; write them so.");
        }

        Directory.CreateDirectory(Path.Combine(FrontendDirectory, "src"));
        File.WriteAllText(Path.Combine(FrontendDirectory, "Cargo.toml"),
            "[package]\nname = \"webgpu-test-frontend\"\nversion = \"0.1.0\"\nedition = \"2021\"\npublish = false\n\n"
            + "[dependencies]\ncodebrix-webplatform = { path = \"platform/codebrix-webplatform\" }\n" + WgpuDependency + "\n"
            + "codebrix-webgpu = \"" + FirstPartyCrateVersion + "\"\n"
            + "dioxus = { version = \"" + dioxusVersion + "\", default-features = false, features = [\"minimal\", \"web\", \"mounted\"] }\n");
        File.Copy(FrontendMain, Path.Combine(FrontendDirectory, "src", "main.rs"));

        using (var archive = ZipFile.OpenRead(WebGpuCrates.GetArchivePath()))
        {
            if (!archive.Entries.Any(e => e.FullName.StartsWith("wgpu-" + WgpuVersion + "/", StringComparison.Ordinal)))
            {
                NoCratesReason = "The WebGpu archive holds no wgpu crate: src/CodeBrix.WebPlatform.WebGpu/Rust/vendor is empty or missing, so a frontend cannot resolve.";
                return;
            }
        }

        WebGpuCrates.Register();
        Options = new PlatformFrontendOptions
        {
            FrontendDirectory = FrontendDirectory,
            BackendAssemblyPath = SampleBackendAssembly,
            CrateMode = CrateMode.PlatformCratesOnly,
            Output = Log,
        };
        Frontend = new PlatformFrontend(Options);
        var watch = Stopwatch.StartNew();
        First = await Frontend.MaterializeAsync(TestContext.Current.CancellationToken);
        FirstDuration = watch.Elapsed;
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
