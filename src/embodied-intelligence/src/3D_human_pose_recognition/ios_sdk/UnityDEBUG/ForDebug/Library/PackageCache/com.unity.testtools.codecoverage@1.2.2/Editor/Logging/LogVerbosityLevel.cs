namespace UnityEditor.TestTools.CodeCoverage
{
    /// <summary>
    /// The verbosity level used in editor and console logs.
    /// </summary>
    public enum LogVerbosityLevel
    {
        /// <summary>
        /// All logs will be printed in Verbose.
        /// </summary>
        Verbose = 0,
        /// <summary>
        /// Logs, Warnings and Errors will be printed in Info.
        /// </summary>
        Info = 1,
        /// <summary>
        /// Warnings and Errors will be printed in Warning.
        /// </summary>
        Warning = 2,
        /// <summary>
        /// Only Errors will be printed in Error.
        /// </summary>
        Error = 3,
        /// <summary>
        /// No logs will be printed in Off.
        /// </summary>
        Off = 4
    }
}
