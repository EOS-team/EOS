using UnityEngine;
using UnityEditor.TestTools.CodeCoverage.OpenCover;
using UnityEditor.TestTools.CodeCoverage.Analytics;

namespace UnityEditor.TestTools.CodeCoverage
{
    internal class CoverageReporterManager
    {
        private readonly CoverageSettings m_CoverageSettings = null;
        private ICoverageReporter m_CoverageReporter = null;
        CoverageReportGenerator m_ReportGenerator = null;

        public CoverageReporterManager(CoverageSettings coverageSettings)
        {
            m_CoverageSettings = coverageSettings;
        }

        public ICoverageReporter CoverageReporter
        {
            get
            {
                if (m_CoverageReporter == null)
                {
                    CreateCoverageReporter();
                }
                return m_CoverageReporter;
            }
        }

        public void CreateCoverageReporter()
        {
            m_CoverageReporter = null;

            // Use OpenCover format as currently this is the only one supported
            CoverageFormat coverageFormat = CoverageFormat.OpenCover;

            switch (coverageFormat)
            {
                case CoverageFormat.OpenCover:
                    m_CoverageSettings.resultsFileExtension = "xml";
                    m_CoverageSettings.resultsFolderSuffix = "-opencov";
                    m_CoverageSettings.resultsFileName = CoverageRunData.instance.isRecording ? "RecordingCoverageResults" : "TestCoverageResults";

                    m_CoverageReporter = new OpenCoverReporter();
                    break;
            }

            if (m_CoverageReporter != null)
            {
                m_CoverageReporter.OnInitialise(m_CoverageSettings);
            }
        }

        public bool ShouldAutoGenerateReport()
        {
            bool shouldAutoGenerateReport = false;
            bool cmdLineGenerateHTMLReport = CommandLineManager.instance.generateHTMLReport;
            bool cmdLineGenerateBadge = CommandLineManager.instance.generateBadgeReport;
            bool cmdLineGenerateAdditionalReports = CommandLineManager.instance.generateAdditionalReports;
            bool generateHTMLReport = CoveragePreferences.instance.GetBool("GenerateHTMLReport", true);
            bool generateAdditionalReports = CoveragePreferences.instance.GetBool("GenerateAdditionalReports", false);
            bool generateBadge = CoveragePreferences.instance.GetBool("GenerateBadge", true);
            bool autoGenerateReport = CoveragePreferences.instance.GetBool("AutoGenerateReport", false);

            if (CommandLineManager.instance.runFromCommandLine)
            {
                if (CommandLineManager.instance.batchmode)
                {
                    if (CommandLineManager.instance.useProjectSettings)
                    {
                        shouldAutoGenerateReport =  cmdLineGenerateHTMLReport ||
                                                    cmdLineGenerateBadge ||
                                                    cmdLineGenerateAdditionalReports ||
                                                    (autoGenerateReport && (generateHTMLReport || generateBadge || generateAdditionalReports));
                    }
                    else
                    {
                        shouldAutoGenerateReport = cmdLineGenerateHTMLReport || cmdLineGenerateBadge || cmdLineGenerateAdditionalReports;
                    }
                }
                else
                {
                    shouldAutoGenerateReport =  cmdLineGenerateHTMLReport ||
                                                cmdLineGenerateBadge ||
                                                cmdLineGenerateAdditionalReports ||
                                                (autoGenerateReport && (generateHTMLReport || generateBadge || generateAdditionalReports));
                }
            } 
            else
            {
                autoGenerateReport = CoveragePreferences.instance.GetBool("AutoGenerateReport", true);
                shouldAutoGenerateReport = autoGenerateReport && (generateHTMLReport || generateBadge || generateAdditionalReports);
            }

            return shouldAutoGenerateReport;
        }

        public void GenerateReport()
        {
            if (ShouldAutoGenerateReport())
            {
                if (m_CoverageSettings != null)
                {
                    CoverageAnalytics.instance.CurrentCoverageEvent.actionID = ActionID.DataReport;
                    ReportGenerator.Generate(m_CoverageSettings);
                }
            }
            else
            {
                // Clear ProgressBar left from saving results to file,
                // otherwise continue on the same ProgressBar
                EditorUtility.ClearProgressBar();

                // Send Analytics event (Data Only)
                CoverageAnalytics.instance.SendCoverageEvent(true);
            }
        }

        public CoverageReportGenerator ReportGenerator
        {
            get 
            {
                if (m_ReportGenerator == null)
                    m_ReportGenerator = new CoverageReportGenerator();

                return m_ReportGenerator;
            }         
        }
    }
}
