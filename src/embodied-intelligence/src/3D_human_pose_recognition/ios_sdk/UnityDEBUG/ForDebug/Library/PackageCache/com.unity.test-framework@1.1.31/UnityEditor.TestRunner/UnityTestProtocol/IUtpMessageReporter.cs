using System.Collections.Generic;
using UnityEditor.Compilation;
using UnityEditor.TestTools.TestRunner.Api;

namespace UnityEditor.TestTools.TestRunner.UnityTestProtocol
{
    internal interface IUtpMessageReporter
    {
        void ReportTestFinished(ITestResultAdaptor result);
        void ReportTestRunStarted(ITestAdaptor testsToRun);
        void ReportTestStarted(ITestAdaptor test);
    }
}
