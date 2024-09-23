using System;
using System.IO;
using System.Text;
using Palmmedia.ReportGenerator.Core;
using Palmmedia.ReportGenerator.Core.Logging;
using UnityEditor.TestTools.CodeCoverage.Utils;
using ILogger = Palmmedia.ReportGenerator.Core.Logging.ILogger;
using Palmmedia.ReportGenerator.Core.CodeAnalysis;
using UnityEditor.TestTools.CodeCoverage.Analytics;

namespace UnityEditor.TestTools.CodeCoverage
{
    internal class CoverageReportGenerator
    {
        public void Generate(CoverageSettings coverageSettings)
        {
            CoverageReporterManager coverageReporterManager = CoverageReporterStarter.CoverageReporterManager;
            AssemblyFiltering assemblyFiltering = null;
            PathFiltering pathFiltering = null;
            if (coverageReporterManager != null)
            {
                coverageReporterManager.CoverageReporter.GetReporterFilter().SetupFiltering();
                assemblyFiltering = coverageReporterManager.CoverageReporter.GetReporterFilter().GetAssemblyFiltering();
                pathFiltering = coverageReporterManager.CoverageReporter.GetReporterFilter().GetPathFiltering();
            }
                
            if (assemblyFiltering == null || pathFiltering == null)
            {
                ResultsLogger.Log(ResultID.Warning_FailedReportNullCoverageFilters);
            }

            CoverageRunData.instance.ReportGenerationStart();

            if (coverageSettings == null)
            {
                EditorUtility.ClearProgressBar();
                ResultsLogger.Log(ResultID.Warning_FailedReportNullCoverageSettings);
                CoverageRunData.instance.ReportGenerationEnd(false);
                return;
            }

            string includeAssemblies = assemblyFiltering != null ? assemblyFiltering.includedAssemblies : string.Empty;
            
            // If override for include assemblies is set in coverageSettings, use overrideIncludeAssemblies instead
            if (!String.IsNullOrEmpty(coverageSettings.overrideIncludeAssemblies))
                includeAssemblies = coverageSettings.overrideIncludeAssemblies;

            string[] assemblyFilters = includeAssemblies.Split(new[] { ',' }, StringSplitOptions.RemoveEmptyEntries);

            if (assemblyFilters.Length == 0)
            {
                EditorUtility.ClearProgressBar();
                ResultsLogger.Log(ResultID.Error_FailedReportNoAssemblies, CoverageUtils.GetFilteringLogParams(assemblyFiltering, pathFiltering));
                CoverageRunData.instance.ReportGenerationEnd(false);
                return;
            }

            for (int i = 0; i < assemblyFilters.Length; i++)
            {
                assemblyFilters[i] = "+" + assemblyFilters[i];
            }

            string rootFolderPath = coverageSettings.rootFolderPath;

            if (CoverageUtils.GetNumberOfFilesInFolder(rootFolderPath, "*.xml", SearchOption.AllDirectories) == 0)
            {
                EditorUtility.ClearProgressBar();
                ResultsLogger.Log(ResultID.Error_FailedReportNoCoverageResults, CoverageUtils.GetFilteringLogParams(assemblyFiltering, pathFiltering));            
                CoverageRunData.instance.ReportGenerationEnd(false);
                return;
            }

            // Only include xml files with the correct filename format
            string sourceXmlPath = CoverageUtils.JoinPaths(rootFolderPath, "**");       
            string testResultsXmlPath = CoverageUtils.JoinPaths(sourceXmlPath, "TestCoverageResults_????.xml");
            string recordingResultsXmlPath = CoverageUtils.JoinPaths(sourceXmlPath, "RecordingCoverageResults_????.xml");
            string rootFullEmptyResultsXmlPath = CoverageUtils.JoinPaths(rootFolderPath, "TestCoverageResults_fullEmpty.xml");

            string[] reportFilePatterns = new string[] { testResultsXmlPath, recordingResultsXmlPath, rootFullEmptyResultsXmlPath };

            bool includeHistoryInReport = CommandLineManager.instance.batchmode && !CommandLineManager.instance.useProjectSettings ?
                CommandLineManager.instance.generateHTMLReportHistory :
                CommandLineManager.instance.generateHTMLReportHistory || CoveragePreferences.instance.GetBool("IncludeHistoryInReport", true);

            string historyDirectory = includeHistoryInReport ? coverageSettings.historyFolderPath : null;

            string targetDirectory = CoverageUtils.JoinPaths(rootFolderPath, CoverageSettings.ReportFolderName);

            string[] sourceDirectories = CommandLineManager.instance.sourcePathsSpecified ? CommandLineManager.instance.sourcePaths.Split(',') : new string[] { };

            bool generateHTMLReport = CommandLineManager.instance.batchmode && !CommandLineManager.instance.useProjectSettings ?
                CommandLineManager.instance.generateHTMLReport :
                CommandLineManager.instance.generateHTMLReport || CoveragePreferences.instance.GetBool("GenerateHTMLReport", true);

            if (coverageSettings.overrideGenerateHTMLReport)
                generateHTMLReport = true;

            bool generateBadge = CommandLineManager.instance.batchmode && !CommandLineManager.instance.useProjectSettings ?
                CommandLineManager.instance.generateBadgeReport :
                CommandLineManager.instance.generateBadgeReport || CoveragePreferences.instance.GetBool("GenerateBadge", true);

            bool generateAdditionalReports = CommandLineManager.instance.batchmode && !CommandLineManager.instance.useProjectSettings ?
                CommandLineManager.instance.generateAdditionalReports :
                CommandLineManager.instance.generateAdditionalReports || CoveragePreferences.instance.GetBool("GenerateAdditionalReports", false);

            string reportTypesString = "xmlSummary,MarkdownSummary,JsonSummary,";
            if (generateHTMLReport)
                reportTypesString += "Html,";
            if (generateBadge)
                reportTypesString += "Badges,";
            if (generateAdditionalReports)
                reportTypesString += "SonarQube,lcov,Cobertura,";

            string[] reportTypes = reportTypesString.Split(new[] { ',' }, StringSplitOptions.RemoveEmptyEntries);
            string[] plugins = new string[] { };

            bool includeAdditionalMetrics = coverageReporterManager != null &&
                   coverageReporterManager.CoverageReporter.GetReporterFilter().ShouldGenerateAdditionalMetrics();

            string[] classFilters = new string[] { };
            string[] fileFilters = new string[] { };
            string verbosityLevel = null;
            string tag = null;

            ReportConfiguration config = new ReportConfiguration(
            reportFilePatterns,
            targetDirectory,
            sourceDirectories,
            historyDirectory,
            reportTypes,
            plugins,
            assemblyFilters,
            classFilters,
            fileFilters,
            verbosityLevel,
            tag);

            DebugFactory loggerFactory = new DebugFactory();
            LoggerFactory.Configure(loggerFactory);

            try
            {
                if (!CommandLineManager.instance.batchmode)
                    EditorUtility.DisplayProgressBar(ReportGeneratorStyles.ProgressTitle.text, ReportGeneratorStyles.ProgressInfoCreating.text, 0f);

                if (Directory.Exists(targetDirectory))
                    Directory.Delete(targetDirectory, true);

                Generator generator = new Generator();
                ResultsLogger.LogSessionItem("Initializing report generation..", LogVerbosityLevel.Info);
                if (generator.GenerateReport(config, new Settings() { DisableRiskHotspots = !includeAdditionalMetrics }, new RiskHotspotsAnalysisThresholds()))
                {
                    ResultsLogger.Log(ResultID.Log_ReportSaved, CoverageUtils.GetFilteringLogParams(assemblyFiltering, pathFiltering, new string[] { targetDirectory }));

                    CoverageRunData.instance.ReportGenerationEnd(true);

                    // Send Analytics event (Report Only / Data & Report)
                    CoverageAnalytics.instance.SendCoverageEvent(true);

                    if (!CommandLineManager.instance.batchmode &&
                        coverageSettings.revealReportInFinder &&
                        CoveragePreferences.instance.GetBool("OpenReportWhenGenerated", true))
                    {
                        string indexHtm = CoverageUtils.JoinPaths(targetDirectory, "index.htm");
                        if (File.Exists(indexHtm))
                            EditorUtility.RevealInFinder(indexHtm);
                        else
                            EditorUtility.RevealInFinder(targetDirectory);
                    }
                }
                else
                {
                    ResultsLogger.Log(ResultID.Error_FailedReport, CoverageUtils.GetFilteringLogParams(assemblyFiltering, pathFiltering));
                    CoverageRunData.instance.ReportGenerationEnd(false);
                }
            }
            finally
            {
                EditorUtility.ClearProgressBar();
            }
        }
    }

    class DebugFactory : ILoggerFactory
    {
        public VerbosityLevel VerbosityLevel { get; set; }

        public DebugLogger Logger { get; set; }

        public DebugFactory()
        {
            Logger = new DebugLogger();
        }

        public ILogger GetLogger(Type type)
        {
            return Logger;
        }
    }

    class DebugLogger : ILogger
    {
        public VerbosityLevel VerbosityLevel { get; set; }

        readonly StringBuilder m_StringBuilder = new StringBuilder();

        public void Debug(string message)
        {
            m_StringBuilder.AppendLine(message);
        }

        public void DebugFormat(string format, params object[] args)
        {
            string message = string.Format(format, args);
            m_StringBuilder.AppendLine(message);

            if (string.Equals(format, "Finished parsing \'{0}\' {1}/{2}") ||
                string.Equals(format, "Parsing of {0} files completed") ||
                string.Equals(format, "Coverage report parsing took {0:f1} seconds") ||
                string.Equals(format, "Initializing report builders for report types: {0}") ||
                string.Equals(format, "Analyzing {0} classes") ||
                string.Equals(format, "Report generation took {0:f1} seconds") )
            {
                ResultsLogger.LogSessionItem(message, LogVerbosityLevel.Info);
            }
            else
            {
                ResultsLogger.LogSessionItem(message, LogVerbosityLevel.Verbose);
            }

            if (!CommandLineManager.instance.batchmode)
            {
                if (string.Equals(format, "Creating report {0}/{1} (Assembly: {2}, Class: {3})"))
                {
                    if (args.Length >= 2)
                    {
                        if (float.TryParse(string.Format("{0}", args[0]), out float currentNum) &&
                            float.TryParse(string.Format("{0}", args[1]), out float totalNum) &&
                            currentNum <= totalNum &&
                            currentNum > 0 &&
                            totalNum > 0)
                        {
                            float progress = (currentNum + 1) / totalNum;
                            EditorUtility.DisplayProgressBar(ReportGeneratorStyles.ProgressTitle.text, ReportGeneratorStyles.ProgressInfoCreating.text, progress);
                        }
                    }
                }
            }
        }

        public void Error(string message)
        {
            m_StringBuilder.AppendLine(message);
        }

        public void ErrorFormat(string format, params object[] args)
        {
            string message = string.Format(format, args);
            m_StringBuilder.AppendLine(message);
            ResultsLogger.LogSessionItem(message, LogVerbosityLevel.Error);
        }

        public void Info(string message)
        {
            m_StringBuilder.AppendLine(message);
        }

        public void InfoFormat(string format, params object[] args)
        {
            string message = string.Format(format, args);
            m_StringBuilder.AppendLine(message);

            if (string.Equals(format, "Writing report file \'{0}\'"))
            {
                ResultsLogger.LogSessionItem(message, LogVerbosityLevel.Verbose);
            }
            else
            {
                ResultsLogger.LogSessionItem(message, LogVerbosityLevel.Info);
            }
        }

        public void Warn(string message)
        {
            m_StringBuilder.AppendLine(message);
        }

        public void WarnFormat(string format, params object[] args)
        {
            string message = string.Format(format, args);
            m_StringBuilder.AppendLine(message);
            ResultsLogger.LogSessionItem(message, LogVerbosityLevel.Verbose);
        }

        public override string ToString()
        {
            return m_StringBuilder.ToString();
        }
    }
}