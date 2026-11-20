using System;

namespace CodeBrix.WebPlatform.WebGpu.LibraryChecker.Maintenance;

/// <summary>
/// A maintenance step (re-vendoring) failed in a way the maintainer can act on; the message says
/// what to do.
/// </summary>
public sealed class MaintenanceException : Exception
{
    /// <summary>
    /// Creates the exception.
    /// </summary>
    /// <param name="message">What failed and what to do about it.</param>
    public MaintenanceException(string message)
        : base(message)
    {
    }

    /// <summary>
    /// Creates the exception with its cause.
    /// </summary>
    /// <param name="message">What failed and what to do about it.</param>
    /// <param name="innerException">The cause.</param>
    public MaintenanceException(string message, Exception innerException)
        : base(message, innerException)
    {
    }
}
