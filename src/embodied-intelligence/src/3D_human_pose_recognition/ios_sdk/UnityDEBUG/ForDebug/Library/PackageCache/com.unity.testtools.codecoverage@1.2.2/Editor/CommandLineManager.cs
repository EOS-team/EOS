using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using UnityEditor.TestTools.CodeCoverage.CommandLineParser;
using UnityEditor.TestTools.CodeCoverage.Utils;

namespace UnityEditor.TestTools.CodeCoverage
{
    internal class CommandLineManager : CommandLineManagerImplementation
    {
        private static CommandLineManager s_Instance = null;

        public static CommandLineManager instance
        {
            get
            {
                if (s_Instance == null)
                    s_Instance = new CommandLineManager();

                return s_Instance;
            }
        }

        protected CommandLineManager() : base(Environment.GetCommandLineArgs())
        {
        }
    }

    internal class CommandLineManagerImplementation
    {
        public bool runFromCommandLine
        {
            get;
            private set;
        }

        public string coverageResultsPath
        {
            get;
            private set;
        }

        public string coverageHistoryPath
        {
            get;
            private set;
        }

        public bool generateAdditionalMetrics
        {
            get;
            private set;
        }

        public bool generateTestReferences
        {
            get;
            private set;
        }

        public bool generateHTMLReportHistory
        {
            get;
            private set;
        }

        public bool generateHTMLReport
        {
            get;
            private set;
        }

        public bool generateBadgeReport
        {
            get;
            private set;
        }

        public bool generateAdditionalReports
        {
            get;
            private set;
        }

        public bool useProjectSettings
        {
            get;
            private set;
        }

        public bool generateRootEmptyReport
        {
            get;
            private set;
        }

        public bool dontClear
        {
            get;
            private set;
        }

        public bool verbosityLevelSpecified
        {
            get;
            private set;
        }

        public bool assemblyFiltersSpecified
        {
            get;
            private set;
        }

        public bool pathFiltersSpecified
        {
            get;
            private set;
        }

        public bool pathFiltersFromFileSpecified
        {
            get;
            private set;
        }

        public bool pathReplacingSpecified
        {
            get;
            private set;
        }

        public string sourcePaths
        {
            get;
            private set;
        }

        public bool sourcePathsSpecified
        {
            get;
            private set;
        }

        public AssemblyFiltering assemblyFiltering
        {
            get;
            private set;
        }

        public PathFiltering pathFiltering
        {
            get;
            private set;
        }

        public PathReplacing pathReplacing
        {
            get;
            private set;
        }

        public bool runTests
        {
            get;
            private set;
        }

        public bool batchmode
        {
            get;
            private set;
        }

        public bool burstDisabled
        {
            get;
            private set;
        }

        private string m_CoverageOptionsArg;
        private string m_IncludeAssemblies;
        private string m_ExcludeAssemblies;
        private string m_IncludePaths;
        private string m_ExcludePaths;
        private string m_PathReplacePatterns;

        public CommandLineManagerImplementation(string[] commandLineArgs)
        {
            runFromCommandLine = false;
            coverageResultsPath = string.Empty;
            coverageHistoryPath = string.Empty;
            sourcePaths = string.Empty;
            generateAdditionalMetrics = false;
            generateTestReferences = false;
            generateHTMLReportHistory = false;
            generateHTMLReport = false;
            generateBadgeReport = false;
            generateAdditionalReports = false;
            useProjectSettings = false;
            generateRootEmptyReport = false;
            dontClear = false;
            verbosityLevelSpecified = false;
            assemblyFiltersSpecified = false;
            pathFiltersSpecified = false;
            pathReplacingSpecified = false;
            sourcePathsSpecified = false;
            pathFiltersFromFileSpecified = false;
            assemblyFiltering = new AssemblyFiltering();
            pathFiltering = new PathFiltering();
            pathReplacing = new PathReplacing();
            runTests = false;
            batchmode = false;
            burstDisabled = false;

            m_CoverageOptionsArg = string.Empty;
            m_IncludeAssemblies = string.Empty;
            m_ExcludeAssemblies = string.Empty;
            m_IncludePaths = string.Empty;
            m_ExcludePaths = string.Empty;
            m_PathReplacePatterns = string.Empty;

            CommandLineOptionSet optionSet = new CommandLineOptionSet(
                new CommandLineOption("enableCodeCoverage", () => { runFromCommandLine = true; }),
                new CommandLineOption("coverageResultsPath", filePathArg => { SetCoverageResultsPath(filePathArg); }),
                new CommandLineOption("coverageHistoryPath", filePathArg => { SetCoverageHistoryPath(filePathArg); }),
                new CommandLineOption("coverageOptions", optionsArg => { AddCoverageOptions(optionsArg); }),
                new CommandLineOption("runTests", () => { runTests = true; }),
                new CommandLineOption("batchmode", () => { batchmode = true; }),
                new CommandLineOption("burst-disable-compilation", () => { burstDisabled = true; })
            );
            optionSet.Parse(commandLineArgs);

            ValidateCoverageResultsPath();
            ValidateCoverageHistoryPath();

            if (runFromCommandLine)
                ParseCoverageOptions();
        }

        private void SetCoverageResultsPath(string filePathArg)
        {
            if (coverageResultsPath != string.Empty)
            {
                ResultsLogger.Log(ResultID.Warning_MultipleResultsPaths, coverageResultsPath);
            }
            else
            {
                if (filePathArg != null)
                {
                    coverageResultsPath = CoverageUtils.NormaliseFolderSeparators(filePathArg);
                }
            }
        }

        private void ValidateCoverageResultsPath()
        {
            if (!CoverageUtils.EnsureFolderExists(coverageResultsPath))
                coverageResultsPath = string.Empty;
        }

        private void SetCoverageHistoryPath(string filePathArg)
        {
            if (coverageHistoryPath != string.Empty)
            {
                ResultsLogger.Log(ResultID.Warning_MultipleHistoryPaths, coverageHistoryPath);
            }
            else
            {
                if (filePathArg != null)
                {
                    coverageHistoryPath = CoverageUtils.NormaliseFolderSeparators(filePathArg);
                }
            }
        }

        private void ValidateCoverageHistoryPath()
        {
            if (!CoverageUtils.EnsureFolderExists(coverageHistoryPath))
                coverageHistoryPath = string.Empty;
        }

        private void AddCoverageOptions(string coverageOptionsArg)
        {
            if (coverageOptionsArg != null)
            {
                coverageOptionsArg = coverageOptionsArg.Trim('\'');

                if (coverageOptionsArg != string.Empty)
                {
                    if (m_CoverageOptionsArg == string.Empty)
                    {
                        m_CoverageOptionsArg = coverageOptionsArg;
                    }
                    else
                    {
                        m_CoverageOptionsArg += ";";
                        m_CoverageOptionsArg += coverageOptionsArg;
                    }
                }
            }
        }

        private void ParseCoverageOptions()
        {
            // Make sure there is no trailing quotes at the end of the options
            m_CoverageOptionsArg = m_CoverageOptionsArg.TrimEnd('"');

            // 'sourcePaths' option is moved at the beginning to ensure it is handled first,
            // since it may be needed for other options
            var options = m_CoverageOptionsArg.Split(new[] { ';' }, StringSplitOptions.RemoveEmptyEntries).ToList();
            if (options.Count > 1)
            {
                var sourcePath = options.FirstOrDefault(option => option.StartsWith("SOURCEPATHS:", StringComparison.InvariantCultureIgnoreCase));
                if (sourcePath != null)
                {
                    var sourcePathIndex = options.IndexOf(sourcePath);
                    var firstElement = options[0];
                    options[sourcePathIndex] = firstElement;
                    options[0] = sourcePath;
                }
            }

            string[] coverageOptions = options.ToArray();

            foreach (string optionArgsStr in coverageOptions)
            {
                if (optionArgsStr.Length == 0)
                    continue;

                string optionName = optionArgsStr;
                string optionArgs = string.Empty;

                int indexOfColon = optionArgsStr.IndexOf(':');
                if (indexOfColon > 0)
                {
                    optionName = optionArgsStr.Substring(0, indexOfColon);
                    optionArgs = optionArgsStr.Substring(indexOfColon+1);
                }

                switch (optionName.ToUpperInvariant())
                {
                    case "GENERATEADDITIONALMETRICS":
                        generateAdditionalMetrics = true;
                        break;

                    case "GENERATEHTMLREPORTHISTORY":
                        generateHTMLReportHistory = true;
                        break;

                    case "GENERATEHTMLREPORT":
                        generateHTMLReport = true;
                        break;

                    case "GENERATEBADGEREPORT":
                        generateBadgeReport = true;
                        break;

                    case "GENERATEADDITIONALREPORTS":
                        generateAdditionalReports = true;
                        break;

                    case "GENERATEROOTEMPTYREPORT":
                        generateRootEmptyReport = true;
                        break;

                    case "DONTCLEAR":
                        dontClear = true;
                        break;

                    case "GENERATETESTREFERENCES":
                        generateTestReferences = true;
                        break;

                    case "USEPROJECTSETTINGS":
                        if (batchmode)
                            useProjectSettings = true;
                        else
                            ResultsLogger.Log(ResultID.Warning_UseProjectSettingsNonBatchmode);
                        break;

                    case "VERBOSITY":
                        if (optionArgs.Length > 0)
                        {
                            verbosityLevelSpecified = true;

                            switch (optionArgs.ToUpperInvariant())
                            {
                                case "VERBOSE":
                                    ResultsLogger.VerbosityLevel = LogVerbosityLevel.Verbose;
                                    break;
                                case "INFO":
                                    ResultsLogger.VerbosityLevel = LogVerbosityLevel.Info;
                                    break;
                                case "WARNING":
                                    ResultsLogger.VerbosityLevel = LogVerbosityLevel.Warning;
                                    break;
                                case "ERROR":
                                    ResultsLogger.VerbosityLevel = LogVerbosityLevel.Error;
                                    break;
                                case "OFF":
                                    ResultsLogger.VerbosityLevel = LogVerbosityLevel.Off;
                                    break;
                            }
                        }
                        break;

                    case "ASSEMBLYFILTERS":
                        if (optionArgs.Length > 0)
                        {
                            assemblyFiltersSpecified = true;

                            string[] assemblyFilters = optionArgs.Split(new[] { ',' }, StringSplitOptions.RemoveEmptyEntries);

                            for (int i = 0; i < assemblyFilters.Length; ++i)
                            {
                                string filter = assemblyFilters[i];
                                string filterBody = filter.Length > 1 ? filter.Substring(1) : string.Empty;

                                if (filter.StartsWith("+", StringComparison.OrdinalIgnoreCase))
                                {
                                    if (m_IncludeAssemblies.Length > 0)
                                        m_IncludeAssemblies += ",";

                                    if (filterBody.StartsWith("<", StringComparison.OrdinalIgnoreCase))
                                    {
                                        if (string.Equals(filterBody, AssemblyFiltering.kAssetsAlias, StringComparison.OrdinalIgnoreCase))
                                            m_IncludeAssemblies += AssemblyFiltering.GetUserOnlyAssembliesString();
                                        else if (string.Equals(filterBody, AssemblyFiltering.kAllAlias, StringComparison.OrdinalIgnoreCase))
                                            m_IncludeAssemblies += AssemblyFiltering.GetAllProjectAssembliesString();
                                        else if (string.Equals(filterBody, AssemblyFiltering.kPackagesAlias, StringComparison.OrdinalIgnoreCase))
                                            m_IncludeAssemblies += AssemblyFiltering.GetPackagesOnlyAssembliesString();
                                        else if (string.Equals(filterBody, AssemblyFiltering.kCoreAlias, StringComparison.OrdinalIgnoreCase))
                                            m_IncludeAssemblies += AssemblyFiltering.kCoreAssemblies;
                                    }
                                    else
                                    {
                                        m_IncludeAssemblies += filterBody;
                                    }
                                }
                                else if (filter.StartsWith("-", StringComparison.OrdinalIgnoreCase))
                                {
                                    if (m_ExcludeAssemblies.Length > 0)
                                        m_ExcludeAssemblies += ",";

                                    if (filterBody.StartsWith("<", StringComparison.OrdinalIgnoreCase))
                                    {
                                        if (string.Equals(filterBody, AssemblyFiltering.kAssetsAlias, StringComparison.OrdinalIgnoreCase))
                                            m_ExcludeAssemblies += AssemblyFiltering.GetUserOnlyAssembliesString();
                                        else if (string.Equals(filterBody, AssemblyFiltering.kAllAlias, StringComparison.OrdinalIgnoreCase))
                                            m_ExcludeAssemblies += AssemblyFiltering.GetAllProjectAssembliesString();
                                        else if (string.Equals(filterBody, AssemblyFiltering.kPackagesAlias, StringComparison.OrdinalIgnoreCase))
                                            m_ExcludeAssemblies += AssemblyFiltering.GetPackagesOnlyAssembliesString();
                                        else if (string.Equals(filterBody, AssemblyFiltering.kCoreAlias, StringComparison.OrdinalIgnoreCase))
                                            m_ExcludeAssemblies += AssemblyFiltering.kCoreAssemblies;
                                    }
                                    else
                                    {
                                        m_ExcludeAssemblies += filterBody;
                                    }
                                }
                                else
                                {
                                    ResultsLogger.Log(ResultID.Warning_AssemblyFiltersNotPrefixed, filter);
                                }
                            }
                        }
                        break;

                    case "PATHFILTERSFROMFILE":
                        if (optionArgs.Length > 0)
                        {
                            pathFiltersFromFileSpecified = true;
                            if (File.Exists(optionArgs))
                            {
                                try
                                {
                                    ParsePathFilters( GetPathFiltersFromFile(optionArgs) );
                                }
                                catch (Exception e)
                                {
                                    ResultsLogger.Log(ResultID.Warning_FailedToExtractPathFiltersFromFile, e.Message, optionArgs);
                                }
                            }
                        }
                        break;

                    case "PATHFILTERS":
                        if (optionArgs.Length > 0)
                        {
                            string[] pathFilters = optionArgs.Split(new[] { ',' }, StringSplitOptions.RemoveEmptyEntries);

                            ParsePathFilters(pathFilters);
                        }
                        break;

                    case "PATHREPLACEPATTERNS":
                        if (optionArgs.Length > 0)
                        {
                            pathReplacingSpecified = true;
                            m_PathReplacePatterns = optionArgs;
                        }
                        break;

                    case "SOURCEPATHS":
                        if (optionArgs.Length > 0)
                        {
                            string[] rawSourcePaths = optionArgs.Split(new[] { ',' }, StringSplitOptions.RemoveEmptyEntries);
                            for (int i = 0; i < rawSourcePaths.Length; ++i)
                            {
                                if (sourcePaths.Length > 0)
                                    sourcePaths += ",";
                                sourcePaths += CoverageUtils.NormaliseFolderSeparators(rawSourcePaths[i]);
                            }

                            if (sourcePaths.Length > 0)
                                sourcePathsSpecified = true;
                        }
                        break;

                    default:
                        ResultsLogger.Log(ResultID.Warning_UnknownCoverageOptionProvided, optionArgsStr);
                        break;
                }
            }

            if (m_IncludeAssemblies.Length == 0)
            {
                // If there are no inlcudedAssemblies specified but there are includedPaths specified
                // then include all project assemblies so path filtering can take precedence over assembly filtering,
                // othewise if there are no includedPaths specified neither then inlcude just the user assemblies (found under the Assets folder)

                if (m_IncludePaths.Length > 0)
                    m_IncludeAssemblies = AssemblyFiltering.GetAllProjectAssembliesString();
                else
                    m_IncludeAssemblies = AssemblyFiltering.GetUserOnlyAssembliesString();
            }

            if (m_ExcludeAssemblies.Length > 0)
                m_ExcludeAssemblies += ",";

            m_ExcludeAssemblies += AssemblyFiltering.kDefaultExcludedAssemblies;

            assemblyFiltering.Parse(m_IncludeAssemblies, m_ExcludeAssemblies);
            pathFiltering.Parse(m_IncludePaths, m_ExcludePaths);
            pathReplacing.Parse(m_PathReplacePatterns);
        }

        private void ParsePathFilters(string[] pathFilters)
        {
            var sources = new string[0];
            if (sourcePathsSpecified)
                sources = sourcePaths.Split(new[] { ',' }, StringSplitOptions.RemoveEmptyEntries);

            for (int i = 0; i < pathFilters.Length; ++i)
            {
                string filter = pathFilters[i];
                string filterBody = filter.Length > 1 ? filter.Substring(1) : string.Empty;

                var isRelative = !filterBody.StartsWith("*") && !filterBody.StartsWith("?") && string.IsNullOrEmpty(Path.GetPathRoot(filterBody));
                //If current path is relative - expand it to an absolute path using specified source paths
                if (isRelative && sourcePathsSpecified)
                {
                    string expandedPaths = string.Empty;
                    foreach (var source in sources)
                    {
                        if (expandedPaths.Length > 0)
                            expandedPaths += ",";
                        expandedPaths += CoverageUtils.NormaliseFolderSeparators(Path.Combine(source, filterBody));
                    }
                    filterBody = expandedPaths;
                }

                if (filter.StartsWith("+", StringComparison.OrdinalIgnoreCase))
                {
                    if (m_IncludePaths.Length > 0)
                        m_IncludePaths += ",";
                    m_IncludePaths += filterBody;
                }
                else if (filter.StartsWith("-", StringComparison.OrdinalIgnoreCase))
                {
                    if (m_ExcludePaths.Length > 0)
                        m_ExcludePaths += ",";
                    m_ExcludePaths += filterBody;
                }
                else
                {
                    ResultsLogger.Log(ResultID.Warning_PathFiltersNotPrefixed, filter);
                }
            }

            if (m_IncludePaths.Length > 0 || m_ExcludePaths.Length > 0)
                pathFiltersSpecified = true;
        }

        internal string[] GetPathFiltersFromFile(string path)
        {
            var paths = new List<string>();

            foreach (var line in File.ReadAllLines(path))
            {
                var entry = line.Trim();
                paths.Add(CoverageUtils.NormaliseFolderSeparators(entry));
            }

            return paths.ToArray();
        }
    }
}
