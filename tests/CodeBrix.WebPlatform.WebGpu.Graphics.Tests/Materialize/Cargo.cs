using System;
using System.Collections.Generic;
using System.ComponentModel;
using System.Diagnostics;
using System.IO;
using System.Threading;
using System.Threading.Tasks;

namespace CodeBrix.WebPlatform.WebGpu.Graphics.Tests.Materialize;

/// <summary>
/// Runs cargo (found on PATH) for the tests that check the materialized frontend with it. Those
/// tests are skipped on a machine without cargo or without the wasm32 standard library.
/// </summary>
internal static class Cargo
{
    /// <summary>The target a browser frontend is built for.</summary>
    public const string WasmTarget = "wasm32-unknown-unknown";

    private static readonly Lazy<string> Version = new Lazy<string>(() => RunQuietly("cargo", "--version"));

    private static readonly Lazy<bool> WasmTargetInstalled = new Lazy<bool>(() =>
    {
        var sysroot = RunQuietly("rustc", "--print", "sysroot");
        return sysroot != null && Directory.Exists(Path.Combine(sysroot, "lib", "rustlib", WasmTarget));
    });

    /// <summary>True when cargo can be started.</summary>
    public static bool IsAvailable => Version.Value != null;

    /// <summary>True when rustc has the standard library for <see cref="WasmTarget"/>.</summary>
    public static bool HasWasmTarget => IsAvailable && WasmTargetInstalled.Value;

    /// <summary>Runs cargo in a folder, with its build output in <paramref name="targetDirectory"/>.</summary>
    /// <returns>The exit code and what cargo wrote to its standard output and its standard error.</returns>
    public static async Task<(int ExitCode, string Output, string Errors)> RunAsync(string workingDirectory, string targetDirectory, IEnumerable<string> arguments, CancellationToken cancellationToken)
    {
        var start = new ProcessStartInfo("cargo")
        {
            WorkingDirectory = workingDirectory,
            RedirectStandardOutput = true,
            RedirectStandardError = true,
            UseShellExecute = false,
        };
        foreach (var argument in arguments)
        {
            start.ArgumentList.Add(argument);
        }

        start.Environment["CARGO_TARGET_DIR"] = targetDirectory;
        start.Environment["CARGO_TERM_COLOR"] = "never";
        using var process = Process.Start(start);
        var output = process.StandardOutput.ReadToEndAsync(cancellationToken);
        var errors = process.StandardError.ReadToEndAsync(cancellationToken);
        try
        {
            await process.WaitForExitAsync(cancellationToken);
        }
        catch (OperationCanceledException)
        {
            process.Kill(entireProcessTree: true);
            throw;
        }

        return (process.ExitCode, await output, await errors);
    }

    /// <summary>Runs a program and returns its trimmed standard output, or null when it cannot be started or fails.</summary>
    private static string RunQuietly(string program, params string[] arguments)
    {
        try
        {
            var start = new ProcessStartInfo(program) { RedirectStandardOutput = true, RedirectStandardError = true, UseShellExecute = false };
            foreach (var argument in arguments)
            {
                start.ArgumentList.Add(argument);
            }

            using var process = Process.Start(start);
            var errors = process.StandardError.ReadToEndAsync();
            var output = process.StandardOutput.ReadToEnd();
            process.WaitForExit();
            errors.Wait();
            return process.ExitCode == 0 ? output.Trim() : null;
        }
        catch (Win32Exception)
        {
            return null;
        }
    }
}
