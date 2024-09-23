using UnityEditor.TestTools.TestRunner.Api;

namespace UnityEditor.TestTools.CodeCoverage
{
    interface ICoverageReporter
    {
        ICoverageReporterFilter GetReporterFilter();
        void OnInitialise(CoverageSettings settings);
        void OnRunStarted(ITestAdaptor testsToRun);
        void OnRunFinished(ITestResultAdaptor testResults);
        void OnTestStarted(ITestAdaptor test);
        void OnTestFinished(ITestResultAdaptor result);
        void OnBeforeAssemblyReload();
        void OnCoverageRecordingPaused();
    }
}