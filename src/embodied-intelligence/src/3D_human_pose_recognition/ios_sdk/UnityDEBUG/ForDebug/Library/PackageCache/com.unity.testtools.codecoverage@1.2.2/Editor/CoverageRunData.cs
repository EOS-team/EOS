using System;
using UnityEditor.TestTools.CodeCoverage.Analytics;
using UnityEngine;

namespace UnityEditor.TestTools.CodeCoverage
{
    [Serializable]
    internal class CoverageRunDataImplementation
    {
        [SerializeField]
        private bool m_IsRunning = false;

        [SerializeField]
        private int m_TestRunCount = 0;

        [SerializeField]
        private string m_LastIgnoredSuite = string.Empty;

        [SerializeField]
        private bool m_IsRecording = false;

        [SerializeField]
        private bool m_IsRecordingPaused = false;

        [SerializeField]
        private bool m_ReportWasGenerated = false;

        [SerializeField]
        private bool m_IsGeneratingReport = false;

        public void Start(bool setupEvents = true)
        {
            m_LastIgnoredSuite = string.Empty;
            m_IsRunning = true;
            m_TestRunCount = 0;

            if (setupEvents)
            {
                CoverageAnalytics.instance.CurrentCoverageEvent.actionID = ActionID.DataOnly;
                CoverageAnalytics.instance.CurrentCoverageEvent.coverageModeId = CoverageModeID.TestRunner;
                CoverageAnalytics.instance.StartTimer();

                CoverageEventData.instance.StartSession(SessionMode.TestRunner);
            }
        }

        public void Stop()
        {
            m_LastIgnoredSuite = string.Empty;
            m_IsRunning = false;
        }

        public void StartRecording(bool setupEvents = true)
        {
            Start(setupEvents);
            IncrementTestRunCount();
            m_IsRecording = true;
            m_IsRecordingPaused = false;

            if (setupEvents)
            {
                CoverageAnalytics.instance.CurrentCoverageEvent.coverageModeId = CoverageModeID.Recording;
                CoverageEventData.instance.StartSession(SessionMode.Recording);
            }
        }

        public void PauseRecording()
        {
            m_IsRecordingPaused = true;
        }

        public void UnpauseRecording()
        {
            m_IsRecordingPaused = false;
        }

        public void StopRecording()
        {
            Stop();
            m_IsRecording = false;
            m_IsRecordingPaused = false;
        }

        public bool isRunning
        {
            get { return m_IsRunning; }
        }

        public bool isRecording
        {
            get { return m_IsRecording; }
        }

        public bool isRecordingPaused
        {
            get { return m_IsRecordingPaused; }
        }

        public bool isGeneratingReport
        {
            get { return m_IsGeneratingReport; }
        }

        public bool reportWasGenerated
        {
            get { return m_ReportWasGenerated; }
        }

        public void ReportGenerationStart()
        {
            m_IsGeneratingReport = true;
            m_ReportWasGenerated = false;
        }

        public void ReportGenerationEnd(bool success)
        {
            m_IsGeneratingReport = false;
            m_ReportWasGenerated = success;
        }

        public void IncrementTestRunCount()
        {
            m_TestRunCount++;
        }

        public bool DidTestsRun()
        {
            return m_TestRunCount > 0;
        }

        public void SetLastIgnoredSuiteID(string id)
        {
            m_LastIgnoredSuite = id;
        }

        public bool HasLastIgnoredSuiteID()
        {
            return m_LastIgnoredSuite.Length > 0;
        }

        public string GetLastIgnoredSuiteID()
        {
            return m_LastIgnoredSuite;
        }
    }

    [Serializable]
    internal class CoverageRunData : ScriptableSingleton<CoverageRunData>
    {
        [SerializeField]
        private CoverageRunDataImplementation m_CoverageRunDataImplementation = null;

        protected CoverageRunData() : base()
        {
            m_CoverageRunDataImplementation = new CoverageRunDataImplementation();
        }

        public bool isRunning
        {
            get { return m_CoverageRunDataImplementation.isRunning; }
        }

        public bool isRecording
        {
            get { return m_CoverageRunDataImplementation.isRecording; }
        }

        public bool isRecordingPaused
        {
            get { return m_CoverageRunDataImplementation.isRecordingPaused; }
        }

        public bool reportWasGenerated
        {
            get { return m_CoverageRunDataImplementation.reportWasGenerated; }
        }

        public void IncrementTestRunCount()
        {
            m_CoverageRunDataImplementation.IncrementTestRunCount();
        }

        public bool DidTestsRun()
        {
            return m_CoverageRunDataImplementation.DidTestsRun();
        }

        public void SetLastIgnoredSuiteID(string id)
        {
            m_CoverageRunDataImplementation.SetLastIgnoredSuiteID(id);
        }

        public bool HasLastIgnoredSuiteID()
        {
            return m_CoverageRunDataImplementation.HasLastIgnoredSuiteID();
        }

        public string GetLastIgnoredSuiteID()
        {
            return m_CoverageRunDataImplementation.GetLastIgnoredSuiteID();
        }

        public void Start()
        {
            m_CoverageRunDataImplementation.Start();
        }

        public void Stop()
        {
            m_CoverageRunDataImplementation.Stop();
        }

        public void StartRecording()
        {
            m_CoverageRunDataImplementation.StartRecording();
        }

        public void StopRecording()
        {
            m_CoverageRunDataImplementation.StopRecording();
        }

        public void PauseRecording()
        {
            m_CoverageRunDataImplementation.PauseRecording();
            Events.InvokeOnCoverageSessionPaused();
        }

        public void UnpauseRecording()
        {
            m_CoverageRunDataImplementation.UnpauseRecording();
            Events.InvokeOnCoverageSessionUnpaused();
        }

        public bool isGeneratingReport
        {
            get { return m_CoverageRunDataImplementation.isGeneratingReport; }
        }

        public void ReportGenerationStart()
        {
            m_CoverageRunDataImplementation.ReportGenerationStart();
        }

        public void ReportGenerationEnd(bool success)
        {
            m_CoverageRunDataImplementation.ReportGenerationEnd(success);
        }
    }
}