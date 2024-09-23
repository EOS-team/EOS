namespace UnityEditor.TestTools.CodeCoverage.Analytics
{
    internal enum EventName
    {
        codeCoverage
    }

    internal enum ActionID
    {
        Other = 0,
        DataOnly = 1,
        ReportOnly = 2,
        DataReport = 3   
    }

    internal enum CoverageModeID
    {
        None = 0,
        TestRunner = 1,
        Recording = 2
    }
}
