using System;
using System.Collections.Generic;
using UnityEngine;

namespace UnityEditor.TestTools.CodeCoverage
{
    [Serializable]
    internal class CoverageEventDataImplementation
    {
        [SerializeField]
        private SessionMode m_CoverageSessionMode;

        [SerializeField]
        private List<string> m_CoverageSessionResultPaths;

        public void StartSession(SessionMode coverageSessionMode)
        {
            m_CoverageSessionMode = coverageSessionMode;
            m_CoverageSessionResultPaths = new List<string>();
        }

        public void AddSessionResultPath(string path)
        {
            if (m_CoverageSessionResultPaths != null)
            {
                m_CoverageSessionResultPaths.Add(path);
            }
        }

        public SessionMode CoverageSessionMode
        {
            get { return m_CoverageSessionMode; }
        }

        public List<string> CoverageSessionResultPaths
        {
            get { return m_CoverageSessionResultPaths; }
        }
    }

    [Serializable]
    internal class CoverageEventData : ScriptableSingleton<CoverageEventData>
    {
        [SerializeField]
        private CoverageEventDataImplementation m_CoverageEventDataImplementation = null;

        protected CoverageEventData() : base()
        {
            m_CoverageEventDataImplementation = new CoverageEventDataImplementation();
        }

        public void StartSession(SessionMode coverageSessionMode)
        {
            m_CoverageEventDataImplementation.StartSession(coverageSessionMode);
        }

        public void AddSessionResultPath(string path)
        {
            m_CoverageEventDataImplementation.AddSessionResultPath(path);
        }

        public SessionEventInfo GetCoverageSessionInfo()
        {
            SessionEventInfo info = new SessionEventInfo(m_CoverageEventDataImplementation.CoverageSessionMode, m_CoverageEventDataImplementation.CoverageSessionResultPaths);
            return info;
        }
    }
}