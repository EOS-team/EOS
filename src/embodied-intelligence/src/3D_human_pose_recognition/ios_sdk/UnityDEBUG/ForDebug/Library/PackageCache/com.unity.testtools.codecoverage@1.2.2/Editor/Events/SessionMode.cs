namespace UnityEditor.TestTools.CodeCoverage
{
    /// <summary>
    /// The code coverage session mode.
    /// </summary>
    public enum SessionMode
    {
        /// <summary>
        /// Describes a code coverage session triggered by automated testing, using the Test Runner.
        /// </summary>
        TestRunner = 0,
        /// <summary>
        /// Describes a code coverage session triggered by Coverage Recording.
        /// </summary>
        Recording = 1
    }
}
