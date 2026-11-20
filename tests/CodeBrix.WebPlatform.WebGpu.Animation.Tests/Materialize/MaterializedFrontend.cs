using System;
using System.IO;
using System.IO.Compression;
using System.Linq;
using System.Text.RegularExpressions;
using System.Threading.Tasks;
using CodeBrix.WebPlatform.FrontendBuilder;
using CodeBrix.WebPlatform.FrontendBuilder.Crates;
using CodeBrix.WebPlatform.WebGpu.Graphics;
using Xunit;

namespace CodeBrix.WebPlatform.WebGpu.Animation.Tests.Materialize;

/// <summary>
/// TWO frontends, materialized for real by the platform's <see cref="PlatformFrontend.MaterializeAsync"/>
/// in a temporary folder, one after the other:
/// <list type="number">
/// <item>after <see cref="AnimationCrates.Register"/> ONLY (the WebGpu bundle, then this
/// package's): a Cargo.toml that names the platform's shared crate,
/// <c>codebrix-webgpu-animation</c> and <c>codebrix-webgpu</c> at their versions and
/// <c>dioxus</c> at the version that crate names, and Materialize/frontend-main.rs;</item>
/// <item>then after <see cref="GraphicsCrates.Register"/> as well: a frontend that names BOTH
/// <c>codebrix-webgpu-graphics</c> and <c>codebrix-webgpu-animation</c>, with
/// Materialize/frontend-both-main.rs (a Graphics layer under an Animation layer).</item>
/// </list>
/// The client is read from the WebGpu tests' fixture backend. The platform's crates archive comes
/// from the FrontendBuilder package (beside the test assembly, as beside a build program); the
/// WebGpu, Animation and Graphics archives from the projects' output. Created once for the
/// collection <see cref="MaterializeCollection"/>.
/// </summary>
public sealed class MaterializedFrontend : IAsyncLifetime
{
    /// <summary>The temporary folder that holds the frontends; deleted afterwards.</summary>
    public string Folder { get; } = Path.Combine(Path.GetTempPath(), "webgpu-animation-materialize-tests-" + Guid.NewGuid().ToString("N"));

    /// <summary>The Animation-only frontend's folder.</summary>
    public string FrontendDirectory => Path.Combine(Folder, "frontend");

    /// <summary>The folder of the frontend that names both crates.</summary>
    public string BothFrontendDirectory => Path.Combine(Folder, "frontend-both");

    /// <summary>The version of this package's crate codebrix-webgpu-animation (from its Cargo.toml).</summary>
    public string AnimationCrateVersion { get; private set; }

    /// <summary>The version of the WebGpu package's crate codebrix-webgpu (from its Cargo.toml).</summary>
    public string WebGpuCrateVersion { get; private set; }

    /// <summary>The version of the Graphics package's crate codebrix-webgpu-graphics (from its Cargo.toml).</summary>
    public string GraphicsCrateVersion { get; private set; }

    /// <summary>The options the Animation-only frontend was materialized with.</summary>
    public PlatformFrontendOptions Options { get; private set; }

    /// <summary>The options the frontend that names both crates was materialized with.</summary>
    public PlatformFrontendOptions BothOptions { get; private set; }

    /// <summary>The result of the Animation-only Materialize (the WebGpu and Animation bundles registered).</summary>
    public MaterializeResult First { get; private set; }

    /// <summary>The result of the Materialize with the Graphics bundle registered as well.</summary>
    public MaterializeResult Both { get; private set; }

    /// <summary>
    /// Why nothing was materialized: the WebGpu archive holds no wgpu crate (a build without the
    /// WebGpu project's Rust/vendor), so a frontend could not resolve. <c>null</c> when the
    /// frontends were materialized.
    /// </summary>
    public string NoCratesReason { get; private set; }

    /// <summary>What the frontend builds wrote to their output.</summary>
    public StringWriter Log { get; } = new StringWriter();

    /// <summary>The fixture backend, built beside the test assembly.</summary>
    public static string SampleBackendAssembly => Path.Combine(AppContext.BaseDirectory, "CodeBrix.WebPlatform.WebGpu.Tests.SampleBackend.dll");

    /// <summary>This package's crate's Cargo.toml, copied beside the test assembly by the project file.</summary>
    public static string AnimationCrateManifest => Path.Combine(AppContext.BaseDirectory, "codebrix-webgpu-animation", "Cargo.toml");

    /// <summary>The Cargo.toml of codebrix-webgpu, copied beside the test assembly by the project file.</summary>
    public static string WebGpuCrateManifest => Path.Combine(AppContext.BaseDirectory, "codebrix-webgpu", "Cargo.toml");

    /// <summary>The Cargo.toml of codebrix-webgpu-graphics, copied beside the test assembly by the project file.</summary>
    public static string GraphicsCrateManifest => Path.Combine(AppContext.BaseDirectory, "codebrix-webgpu-graphics", "Cargo.toml");

    /// <summary>The Animation-only test frontend's main.rs, copied beside the test assembly by the project file.</summary>
    public static string FrontendMain => Path.Combine(AppContext.BaseDirectory, "Materialize", "frontend-main.rs");

    /// <summary>The main.rs of the test frontend that names both crates, copied beside the test assembly by the project file.</summary>
    public static string BothFrontendMain => Path.Combine(AppContext.BaseDirectory, "Materialize", "frontend-both-main.rs");

    private static string ReadVersion(string manifest) => Regex.Match(File.ReadAllText(manifest), @"(?m)^version\s*=\s*""([^""]+)""").Groups[1].Value;

    /// <inheritdoc />
    public async ValueTask InitializeAsync()
    {
        AnimationCrateVersion = ReadVersion(AnimationCrateManifest);
        GraphicsCrateVersion = ReadVersion(GraphicsCrateManifest);
        var webGpu = File.ReadAllText(WebGpuCrateManifest);
        WebGpuCrateVersion = Regex.Match(webGpu, @"(?m)^version\s*=\s*""([^""]+)""").Groups[1].Value;
        var dioxusVersion = Regex.Match(webGpu, @"(?m)^dioxus\s*=\s*\{\s*version\s*=\s*""([^""]+)""").Groups[1].Value;
        if (AnimationCrateVersion.Length == 0 || GraphicsCrateVersion.Length == 0 || WebGpuCrateVersion.Length == 0 || dioxusVersion.Length == 0)
        {
            throw new InvalidOperationException($"The Cargo.toml of codebrix-webgpu-animation ({AnimationCrateManifest}), of codebrix-webgpu-graphics ({GraphicsCrateManifest}) or of codebrix-webgpu ({WebGpuCrateManifest}) names no version on a line of its own, or codebrix-webgpu's no `dioxus = {{ version = ... }}` line; write them so.");
        }

        var dioxus = "dioxus = { version = \"" + dioxusVersion + "\", default-features = false, features = [\"minimal\", \"web\", \"mounted\"] }\n";
        WriteFrontend(FrontendDirectory, "webgpu-animation-test-frontend", FrontendMain,
            "codebrix-webgpu-animation = \"" + AnimationCrateVersion + "\"\n" + "codebrix-webgpu = \"" + WebGpuCrateVersion + "\"\n" + dioxus);
        WriteFrontend(BothFrontendDirectory, "webgpu-both-test-frontend", BothFrontendMain,
            "codebrix-webgpu-animation = \"" + AnimationCrateVersion + "\"\n" + "codebrix-webgpu-graphics = \"" + GraphicsCrateVersion + "\"\n" + "codebrix-webgpu = \"" + WebGpuCrateVersion + "\"\n" + dioxus);

        using (var archive = ZipFile.OpenRead(WebGpuCrates.GetArchivePath()))
        {
            if (!archive.Entries.Any(e => e.FullName.StartsWith("wgpu-", StringComparison.Ordinal)))
            {
                NoCratesReason = "The WebGpu archive holds no wgpu crate: src/CodeBrix.WebPlatform.WebGpu/Rust/vendor is empty or missing, so a frontend cannot resolve.";
                return;
            }
        }

        AnimationCrates.Register();
        Options = NewOptions(FrontendDirectory);
        First = await new PlatformFrontend(Options).MaterializeAsync(TestContext.Current.CancellationToken);

        GraphicsCrates.Register();
        BothOptions = NewOptions(BothFrontendDirectory);
        Both = await new PlatformFrontend(BothOptions).MaterializeAsync(TestContext.Current.CancellationToken);
    }

    private PlatformFrontendOptions NewOptions(string frontend) => new PlatformFrontendOptions
    {
        FrontendDirectory = frontend,
        BackendAssemblyPath = SampleBackendAssembly,
        CrateMode = CrateMode.PlatformCratesOnly,
        Output = Log,
    };

    private static void WriteFrontend(string directory, string name, string main, string dependencies)
    {
        Directory.CreateDirectory(Path.Combine(directory, "src"));
        File.WriteAllText(Path.Combine(directory, "Cargo.toml"),
            "[package]\nname = \"" + name + "\"\nversion = \"0.1.0\"\nedition = \"2021\"\npublish = false\n\n"
            + "[dependencies]\ncodebrix-webplatform = { path = \"platform/codebrix-webplatform\" }\n" + dependencies);
        File.Copy(main, Path.Combine(directory, "src", "main.rs"));
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
