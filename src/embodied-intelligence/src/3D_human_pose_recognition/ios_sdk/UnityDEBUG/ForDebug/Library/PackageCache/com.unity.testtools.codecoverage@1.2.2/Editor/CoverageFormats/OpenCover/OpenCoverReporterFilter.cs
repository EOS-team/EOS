namespace UnityEditor.TestTools.CodeCoverage.OpenCover
{
    internal class OpenCoverReporterFilter : ICoverageReporterFilter
    {
        private AssemblyFiltering m_AssemblyFiltering;
        private PathFiltering m_PathFiltering;

        public void SetupFiltering()
        {
            if (!CommandLineManager.instance.runFromCommandLine || !CommandLineManager.instance.assemblyFiltersSpecified)
            {
                m_AssemblyFiltering = new AssemblyFiltering();

                string includeAssemblies = CoveragePreferences.instance.GetString("IncludeAssemblies", AssemblyFiltering.GetUserOnlyAssembliesString());
                m_AssemblyFiltering.Parse(includeAssemblies, AssemblyFiltering.kDefaultExcludedAssemblies);
            }

            if (!CommandLineManager.instance.runFromCommandLine || !CommandLineManager.instance.pathFiltersSpecified)
            {
                m_PathFiltering = new PathFiltering();

                string pathsToInclude = CoveragePreferences.instance.GetStringForPaths("PathsToInclude", string.Empty);
                string pathsToExclude = CoveragePreferences.instance.GetStringForPaths("PathsToExclude", string.Empty);

                m_PathFiltering.Parse(pathsToInclude, pathsToExclude);
            }
        }

        public AssemblyFiltering GetAssemblyFiltering()
        {
            if (CommandLineManager.instance.batchmode && !CommandLineManager.instance.useProjectSettings)
                return CommandLineManager.instance.assemblyFiltering;
            else
                return CommandLineManager.instance.assemblyFiltersSpecified ?
                    CommandLineManager.instance.assemblyFiltering :
                    m_AssemblyFiltering;
        }

        public bool ShouldProcessAssembly(string assemblyName)
        {
            return GetAssemblyFiltering().IsAssemblyIncluded(assemblyName);
        }

        public PathFiltering GetPathFiltering()
        {
            if (CommandLineManager.instance.batchmode && !CommandLineManager.instance.useProjectSettings)
                return CommandLineManager.instance.pathFiltering;
            else
                return CommandLineManager.instance.pathFiltersSpecified ?
                    CommandLineManager.instance.pathFiltering :
                    m_PathFiltering;
        }

        public bool ShouldProcessFile(string filename)
        {
            return GetPathFiltering().IsPathIncluded(filename);
        }

        public bool ShouldGenerateAdditionalMetrics()
        {
            if (CommandLineManager.instance.batchmode && !CommandLineManager.instance.useProjectSettings)
                return CommandLineManager.instance.generateAdditionalMetrics;
            else
                return CommandLineManager.instance.generateAdditionalMetrics || CoveragePreferences.instance.GetBool("GenerateAdditionalMetrics", false);
        }

        public bool ShouldGenerateTestReferences()
        {
            if (CommandLineManager.instance.batchmode && !CommandLineManager.instance.useProjectSettings)
                return CommandLineManager.instance.generateTestReferences;
            else
                return CommandLineManager.instance.generateTestReferences || CoveragePreferences.instance.GetBool("GenerateTestReferences", false);
        }
    }
}
