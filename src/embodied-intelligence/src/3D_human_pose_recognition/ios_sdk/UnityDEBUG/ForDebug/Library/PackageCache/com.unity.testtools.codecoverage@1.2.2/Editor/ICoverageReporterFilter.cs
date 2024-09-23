namespace UnityEditor.TestTools.CodeCoverage
{
    interface ICoverageReporterFilter
    {
        void SetupFiltering();
        AssemblyFiltering GetAssemblyFiltering();
        bool ShouldProcessAssembly(string assemblyName);
        PathFiltering GetPathFiltering();
        bool ShouldProcessFile(string filename);
        bool ShouldGenerateAdditionalMetrics();
        bool ShouldGenerateTestReferences();
    }
}
