using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using System.Text.Json;
using CodeBrix.RustTools.Licensing;

namespace CodeBrix.WebPlatform.WebGpu.LibraryChecker;

/// <summary>
/// The licenses the repository accepts in the third-party code it carries outside
/// <c>samples/</c> (the vendored wgpu crates), read from
/// <c>tools/licensing/license-allowlist.json</c>, in two tiers: anywhere (including what is
/// compiled into a platform application), and vendored but not compiled into an application.
/// The repository carries no tools, so the platform's tooling tiers do not exist here; the file
/// is rejected when it names one.
/// </summary>
public sealed class LicenseAllowlist
{
    /// <summary>The properties of the platform's allowlist that belong to its tools and have no meaning here.</summary>
    public static readonly IReadOnlyList<string> ToolingProperties = new[] { "toolingOnly", "toolingOnlyExceptions", "compiledIntoTheDistributedTools" };

    /// <summary>Licenses accepted anywhere, including in the compiled application.</summary>
    public IReadOnlyList<OpenSourceLicense> Everywhere { get; init; } = Array.Empty<OpenSourceLicense>();

    /// <summary>Licenses accepted in a vendored crate that is not compiled into the application.</summary>
    public IReadOnlyList<OpenSourceLicense> VendoredOnly { get; init; } = Array.Empty<OpenSourceLicense>();

    /// <summary>Which license to elect when a crate offers a choice, most preferred first.</summary>
    public IReadOnlyList<OpenSourceLicense> Preferred { get; init; } = Array.Empty<OpenSourceLicense>();

    /// <summary>Vendored crates whose bundled license files were reviewed by hand (crate name to reason).</summary>
    public IReadOnlyDictionary<string, string> BundledOverrides { get; init; } = new Dictionary<string, string>();

    /// <summary>
    /// Reads the allowlist file.
    /// </summary>
    /// <param name="path">The JSON file.</param>
    /// <exception cref="FileNotFoundException">The file does not exist.</exception>
    /// <exception cref="InvalidDataException">The file is not valid, names a license that is not
    /// a known SPDX identifier, or names a tooling tier.</exception>
    public static LicenseAllowlist Load(string path)
    {
        if (!File.Exists(path))
        {
            throw new FileNotFoundException($"The license allowlist '{path}' does not exist.", path);
        }

        return Parse(File.ReadAllText(path));
    }

    /// <summary>
    /// Reads the allowlist from JSON text.
    /// </summary>
    /// <param name="json">The JSON text.</param>
    /// <exception cref="InvalidDataException">The text is not valid, names a license that is not
    /// a known SPDX identifier, or names a tooling tier.</exception>
    public static LicenseAllowlist Parse(string json)
    {
        JsonDocument document;
        try
        {
            document = JsonDocument.Parse(json, new JsonDocumentOptions { CommentHandling = JsonCommentHandling.Skip, AllowTrailingCommas = true });
        }
        catch (JsonException exception)
        {
            throw new InvalidDataException("The license allowlist is not valid JSON: " + exception.Message, exception);
        }

        using (document)
        {
            var root = document.RootElement;
            if (root.ValueKind != JsonValueKind.Object)
            {
                throw new InvalidDataException("The license allowlist must be a JSON object.");
            }

            foreach (var name in ToolingProperties)
            {
                if (root.TryGetProperty(name, out _))
                {
                    throw new InvalidDataException($"The license allowlist names '{name}', a tier of the CodeBrix.WebPlatform repository's build tools. This repository carries no tools; remove it, or put the licence in 'everywhere' or 'vendoredButNotCompiledIntoTheApplication' (the repository owner decides).");
                }
            }

            return new LicenseAllowlist
            {
                Everywhere = ReadLicenses(root, "everywhere"),
                VendoredOnly = ReadLicenses(root, "vendoredButNotCompiledIntoTheApplication"),
                Preferred = ReadLicenses(root, "preferredWhenACrateOffersAChoice"),
                BundledOverrides = ReadMap(root, "bundledLicenseOverrides"),
            };
        }
    }

    /// <summary>
    /// The policy for crates compiled into the application: the "everywhere" tier only.
    /// </summary>
    public LicensePolicy BuildApplicationPolicy() => BuildPolicy(Everywhere);

    /// <summary>
    /// The policy for every vendored crate: the "everywhere" and "vendored" tiers.
    /// </summary>
    public LicensePolicy BuildVendoredPolicy() => BuildPolicy(Everywhere.Concat(VendoredOnly));

    private LicensePolicy BuildPolicy(IEnumerable<OpenSourceLicense> allowed)
    {
        var policy = new LicensePolicy
        {
            UnresolvedLicenseHandling = UnresolvedLicenseHandling.Reject,
            BundledLicenseHandling = BundledLicenseHandling.Enforce,
            ExemptWorkspaceMembers = true,
        };
        policy.Allow(allowed.ToArray());
        policy.Prefer(Preferred.ToArray());
        foreach (var pair in BundledOverrides)
        {
            policy.AcceptBundledLicenses(pair.Key, pair.Value);
        }

        return policy;
    }

    private static IReadOnlyList<OpenSourceLicense> ReadLicenses(JsonElement root, string name)
    {
        var result = new List<OpenSourceLicense>();
        foreach (var id in ReadStrings(root, name))
        {
            if (!OpenSourceLicenseInfo.TryParse(id, out var license))
            {
                throw new InvalidDataException($"The license allowlist names '{id}' under '{name}', which is not a known SPDX license identifier.");
            }

            result.Add(license);
        }

        return result;
    }

    private static IReadOnlyList<string> ReadStrings(JsonElement root, string name)
    {
        if (!root.TryGetProperty(name, out var element))
        {
            return Array.Empty<string>();
        }

        if (element.ValueKind != JsonValueKind.Array || element.EnumerateArray().Any(e => e.ValueKind != JsonValueKind.String))
        {
            throw new InvalidDataException($"'{name}' in the license allowlist must be an array of strings.");
        }

        return element.EnumerateArray().Select(e => e.GetString().Trim()).Where(s => s.Length > 0).ToList();
    }

    private static IReadOnlyDictionary<string, string> ReadMap(JsonElement root, string name)
    {
        var result = new Dictionary<string, string>(StringComparer.OrdinalIgnoreCase);
        if (!root.TryGetProperty(name, out var element))
        {
            return result;
        }

        if (element.ValueKind != JsonValueKind.Object)
        {
            throw new InvalidDataException($"'{name}' in the license allowlist must be an object of crate names and reasons.");
        }

        foreach (var property in element.EnumerateObject())
        {
            if (property.Value.ValueKind != JsonValueKind.String || string.IsNullOrWhiteSpace(property.Value.GetString()))
            {
                throw new InvalidDataException($"The bundled-license override for '{property.Name}' must give a reason (a string).");
            }

            result[property.Name] = property.Value.GetString();
        }

        return result;
    }
}
