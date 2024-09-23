using UnityEditor.TestTools.CodeCoverage.Analytics;
using UnityEditor.TestTools.CodeCoverage.Utils;
using UnityEditor.TestTools.TestRunner.Api;
using UnityEngine;
using UnityEngine.TestTools;

#if NO_COV_EDITORPREF
using System.Linq;
using UnityEditor.PackageManager;
#endif

namespace UnityEditor.TestTools.CodeCoverage
{
    internal class CoverageReporterListener : ScriptableObject, ICallbacks
    {
        private CoverageReporterManager m_CoverageReporterManager;
        private bool m_IsConnectedToPlayer;

#if TEST_FRAMEWORK_1_3_OR_NEWER
        private bool m_Temp_RunFinishedCalled;
#endif

        public void SetCoverageReporterManager(CoverageReporterManager manager)
        {
            m_CoverageReporterManager = manager;
        }

        public void RunStarted(ITestAdaptor testsToRun)
        {
            if (!Coverage.enabled)
                return;

#if TEST_FRAMEWORK_1_3_OR_NEWER
            m_Temp_RunFinishedCalled = false;
#endif
            m_IsConnectedToPlayer = CoverageUtils.IsConnectedToPlayer;

            if (m_IsConnectedToPlayer)
            {
                ResultsLogger.Log(ResultID.Warning_StandaloneUnsupported);
                return;
            }

            if (CoverageRunData.instance.isRunning || EditorApplication.isCompiling)
                return;

            CoverageRunData.instance.Start();
            m_CoverageReporterManager.CreateCoverageReporter();
            ICoverageReporter coverageReporter = m_CoverageReporterManager.CoverageReporter;
            if (coverageReporter != null)
                coverageReporter.OnRunStarted(testsToRun);
        }

        public void RunFinished(ITestResultAdaptor result)
        {
#if TEST_FRAMEWORK_1_3_OR_NEWER
            if (m_Temp_RunFinishedCalled)
                return;
#endif
            if (!Coverage.enabled)
                return;

            if (CoverageRunData.instance.isRecording || m_IsConnectedToPlayer)
                return;

            CoverageRunData.instance.Stop();

            if (!CoverageRunData.instance.DidTestsRun())
                return;

            ICoverageReporter coverageReporter = m_CoverageReporterManager.CoverageReporter;
            if (coverageReporter != null)
                coverageReporter.OnRunFinished(result);

            m_CoverageReporterManager.GenerateReport();

#if TEST_FRAMEWORK_1_3_OR_NEWER
            m_Temp_RunFinishedCalled = true;
#endif
        }

        public void TestStarted(ITestAdaptor test)
        {
            if (!Coverage.enabled)
                return;

            if (CoverageRunData.instance.HasLastIgnoredSuiteID() || m_IsConnectedToPlayer)
                return;

            if (test.RunState == RunState.Ignored)
            {
                if (test.IsSuite)
                    CoverageRunData.instance.SetLastIgnoredSuiteID(test.Id);

                return;
            }

            if (test.IsSuite)
                return;

            CoverageRunData.instance.IncrementTestRunCount();
            ICoverageReporter coverageReporter = m_CoverageReporterManager.CoverageReporter;
            if (coverageReporter != null)
                coverageReporter.OnTestStarted(test);
        }

        public void TestFinished(ITestResultAdaptor result)
        {
            if (!Coverage.enabled)
                return;

            if (m_IsConnectedToPlayer)
                return;

            if (result.Test.RunState == RunState.Ignored)
            {
                if (result.Test.IsSuite && string.Equals(CoverageRunData.instance.GetLastIgnoredSuiteID(), result.Test.Id))
                    CoverageRunData.instance.SetLastIgnoredSuiteID(string.Empty);
            } 
            else if (!CoverageRunData.instance.HasLastIgnoredSuiteID() && !result.Test.IsSuite)
            {
                ICoverageReporter coverageReporter = m_CoverageReporterManager.CoverageReporter;
                if (coverageReporter != null)
                    coverageReporter.OnTestFinished(result);
            }

#if TEST_FRAMEWORK_1_3_OR_NEWER
            // Temporary fix for UTF issue https://issuetracker.unity3d.com/issues/registered-callbacks-dont-work-after-domain-reload 
            // so that RunFinished is called on the last TestFinished
            if (result.Test.IsSuite && result.Test.Parent == null)
            {
                RunFinished(result);
            }
#endif
        }
    }

    [InitializeOnLoad]
    internal static class CoverageReporterStarter
    {
        public readonly static CoverageReporterManager CoverageReporterManager;

        static CoverageReporterStarter()
        {
#if NO_COV_EDITORPREF
            if (!CommandLineManager.instance.runFromCommandLine)
            {
                bool localCoverageEnabled = CoveragePreferences.instance.GetBool("EnableCodeCoverage", false);
                if (localCoverageEnabled != Coverage.enabled)
                    Coverage.enabled = localCoverageEnabled;

                PackageManager.Events.registeringPackages += OnRegisteringPackages;
            } 
#endif
            if (!Coverage.enabled)
                return;

#if CONDITIONAL_IGNORE_SUPPORTED
            ConditionalIgnoreAttribute.AddConditionalIgnoreMapping("IgnoreForCoverage", true);
#endif
            CoverageReporterListener listener = ScriptableObject.CreateInstance<CoverageReporterListener>();

#if TEST_FRAMEWORK_1_3_OR_NEWER
            TestRunnerApi.RegisterTestCallback(listener);
#else
            TestRunnerApi api = ScriptableObject.CreateInstance<TestRunnerApi>();
            api.RegisterCallbacks(listener);
#endif

            CoverageSettings coverageSettings = new CoverageSettings()
            {
                resultsPathFromCommandLine = CommandLineManager.instance.coverageResultsPath,
                historyPathFromCommandLine = CommandLineManager.instance.coverageHistoryPath
            };

            CoverageReporterManager = new CoverageReporterManager(coverageSettings);

            listener.SetCoverageReporterManager(CoverageReporterManager);

            AssemblyReloadEvents.beforeAssemblyReload += OnBeforeAssemblyReload;
            AssemblyReloadEvents.afterAssemblyReload += OnAfterAssemblyReload;

            // Generate a report if running from the command line,
            // generateHTMLReport or generateBadgeReport or generateAdditionalReports is passed to -coverageOptions
            // and -runTests has not been passed to the command line,
            if (CommandLineManager.instance.runFromCommandLine &&
                CoverageReporterManager.ShouldAutoGenerateReport() &&
                !CommandLineManager.instance.runTests &&
                !CoverageRunData.instance.reportWasGenerated)
            {
                // Start the timer for analytics for Report only
                CoverageAnalytics.instance.StartTimer();
                CoverageAnalytics.instance.CurrentCoverageEvent.actionID = ActionID.ReportOnly;

                coverageSettings.rootFolderPath = CoverageUtils.GetRootFolderPath(coverageSettings);
                coverageSettings.historyFolderPath = CoverageUtils.GetHistoryFolderPath(coverageSettings);

                CoverageReporterManager.ReportGenerator.Generate(coverageSettings);
            }
        }

#if NO_COV_EDITORPREF
        static void OnRegisteringPackages(PackageRegistrationEventArgs args)
        {
            if (args.removed.Any(info => info.name == "com.unity.testtools.codecoverage"))
            {
                Coverage.enabled = false;
            }
        }
#endif

        static void OnBeforeAssemblyReload()
        {
            if (!CoverageRunData.instance.isRunning)
                return;

            if (!CoverageRunData.instance.DidTestsRun())
                return;

            if (CoverageRunData.instance.isRecording && CoverageRunData.instance.isRecordingPaused)
                return;

            ICoverageReporter coverageReporter = CoverageReporterManager.CoverageReporter;
            if (coverageReporter != null)
                coverageReporter.OnBeforeAssemblyReload();
        }

        static void OnAfterAssemblyReload()
        {
            if (!CoverageRunData.instance.isRunning)
                return;

            CoverageReporterManager.CreateCoverageReporter();
        }
    }
}
