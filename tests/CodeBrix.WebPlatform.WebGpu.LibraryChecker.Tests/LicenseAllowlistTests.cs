using System;
using System.IO;
using System.Linq;
using CodeBrix.RustTools.Licensing;
using CodeBrix.WebPlatform.WebGpu.LibraryChecker.Tests.Helpers;
using SilverAssertions;
using Xunit;

namespace CodeBrix.WebPlatform.WebGpu.LibraryChecker.Tests;

public class LicenseAllowlistTests
{
    private const string Json = """
        {
          "comment": "test",
          "everywhere": ["Apache-2.0", "MIT"],
          "vendoredButNotCompiledIntoTheApplication": ["ISC"],
          "preferredWhenACrateOffersAChoice": ["Apache-2.0"],
          "bundledLicenseOverrides": { "some-crate": "reviewed" }
        }
        """;

    [Fact]
    public void Parse_reads_the_two_tiers_the_preference_and_the_overrides()
    {
        //Act
        var allowlist = LicenseAllowlist.Parse(Json);

        //Assert
        allowlist.Everywhere.Should().BeEquivalentTo(new[] { OpenSourceLicense.Apache20, OpenSourceLicense.Mit });
        allowlist.VendoredOnly.Should().ContainSingle().Which.Should().Be(OpenSourceLicense.Isc);
        allowlist.Preferred.Should().ContainSingle().Which.Should().Be(OpenSourceLicense.Apache20);
        allowlist.BundledOverrides["SOME-CRATE"].Should().Be("reviewed");
    }

    [Fact]
    public void the_vendored_policy_is_wider_than_the_application_policy()
    {
        //Arrange
        var allowlist = LicenseAllowlist.Parse(Json);

        //Act
        var application = allowlist.BuildApplicationPolicy();
        var vendored = allowlist.BuildVendoredPolicy();

        //Assert
        application.IsAllowed(OpenSourceLicense.Mit).Should().BeTrue();
        application.IsAllowed(OpenSourceLicense.Isc).Should().BeFalse();
        vendored.IsAllowed(OpenSourceLicense.Isc).Should().BeTrue();
        vendored.IsAllowed(OpenSourceLicense.Mpl20).Should().BeFalse();
        application.AllowedExceptions.Should().BeEmpty();
        vendored.AllowedExceptions.Should().BeEmpty();
        application.BundledLicenseOverrides.Should().ContainKey("some-crate");
        vendored.BundledLicenseOverrides.Should().ContainKey("some-crate");
    }

    [Fact]
    public void every_policy_enforces_the_bundled_scan_rejects_unresolved_licenses_and_prefers_apache()
    {
        //Arrange
        var allowlist = LicenseAllowlist.Parse(Json);

        //Assert
        foreach (var policy in new[] { allowlist.BuildApplicationPolicy(), allowlist.BuildVendoredPolicy() })
        {
            policy.BundledLicenseHandling.Should().Be(BundledLicenseHandling.Enforce);
            policy.UnresolvedLicenseHandling.Should().Be(UnresolvedLicenseHandling.Reject);
            policy.PreferredLicenses.Should().ContainSingle().Which.Should().Be(OpenSourceLicense.Apache20);
        }
    }

    [Theory]
    [InlineData("""{ "toolingOnly": ["MPL-2.0"] }""", "toolingOnly")]
    [InlineData("""{ "toolingOnlyExceptions": ["LLVM-exception"] }""", "toolingOnlyExceptions")]
    [InlineData("""{ "compiledIntoTheDistributedTools": {} }""", "compiledIntoTheDistributedTools")]
    public void Parse_rejects_the_platform_s_tooling_tiers(string json, string property)
    {
        //Arrange
        Action act = () => LicenseAllowlist.Parse(json);

        //Assert
        act.Should().Throw<InvalidDataException>().WithMessage($"*'{property}'*no tools*");
    }

    [Fact]
    public void Parse_rejects_an_unknown_license_a_bad_shape_and_an_override_without_a_reason()
    {
        //Arrange
        Action unknown = () => LicenseAllowlist.Parse("""{ "everywhere": ["Not-A-License-1.0"] }""");
        Action notArray = () => LicenseAllowlist.Parse("""{ "everywhere": "MIT" }""");
        Action notJson = () => LicenseAllowlist.Parse("{ nope");
        Action noReason = () => LicenseAllowlist.Parse("""{ "bundledLicenseOverrides": { "x": "" } }""");
        Action platformShape = () => LicenseAllowlist.Parse("""{ "bundledLicenseOverrides": { "application": {} } }""");

        //Assert
        unknown.Should().Throw<InvalidDataException>().WithMessage("*Not-A-License-1.0*");
        notArray.Should().Throw<InvalidDataException>();
        notJson.Should().Throw<InvalidDataException>();
        noReason.Should().Throw<InvalidDataException>().WithMessage("*reason*");
        platformShape.Should().Throw<InvalidDataException>().WithMessage("*reason*");
    }

    [Fact]
    public void Load_throws_for_a_missing_file()
    {
        //Arrange
        Action act = () => LicenseAllowlist.Load(Path.Combine(Path.GetTempPath(), "no-such-allowlist-" + Guid.NewGuid().ToString("N") + ".json"));

        //Assert
        act.Should().Throw<FileNotFoundException>();
    }

    [Fact]
    public void the_repository_s_allowlist_holds_exactly_the_platform_s_two_tiers_and_no_overrides()
    {
        //Arrange
        var layout = RepositoryUnderTest.Find();
        layout.Should().NotBeNull("the test project sits under the repository's tests folder");

        //Act
        var allowlist = LicenseAllowlist.Load(layout.AllowlistFile);

        //Assert
        allowlist.Everywhere.Select(OpenSourceLicenseInfo.ToSpdxId).Should().Equal(
            "Apache-2.0", "MIT", "BSD-2-Clause", "BSD-3-Clause", "CC0-1.0", "ISC", "PostgreSQL", "Unicode-3.0", "Unicode-DFS-2016", "Zlib");
        allowlist.VendoredOnly.Select(OpenSourceLicenseInfo.ToSpdxId).Should().Equal("CDLA-Permissive-2.0", "BSL-1.0");
        allowlist.Preferred.Select(OpenSourceLicenseInfo.ToSpdxId).Should().Equal("Apache-2.0", "MIT");
        allowlist.BundledOverrides.Should().BeEmpty();
        allowlist.Everywhere.Intersect(allowlist.VendoredOnly).Should().BeEmpty();
    }
}
