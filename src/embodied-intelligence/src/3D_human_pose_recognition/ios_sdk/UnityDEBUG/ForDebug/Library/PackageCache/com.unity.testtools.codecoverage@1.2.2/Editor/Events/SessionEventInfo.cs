using System.Collections.Generic;

namespace UnityEditor.TestTools.CodeCoverage
{
    /// <summary> 
    /// The code coverage session information retuned by the coverage session <see cref="Events"/>.
    /// </summary>
    public class SessionEventInfo
    {
        /// <summary> 
        /// The code coverage session mode.
        /// </summary>
        public SessionMode SessionMode { get; internal set; }
        /// <summary> 
        /// The coverage results paths of the files or folders created during the code coverage session.
        /// </summary>
        public List<string> SessionResultPaths { get; internal set; }

        internal SessionEventInfo(SessionMode mode, List<string> resultPaths)
        {
            SessionMode = mode;
            SessionResultPaths = resultPaths;
        }
    }
}

