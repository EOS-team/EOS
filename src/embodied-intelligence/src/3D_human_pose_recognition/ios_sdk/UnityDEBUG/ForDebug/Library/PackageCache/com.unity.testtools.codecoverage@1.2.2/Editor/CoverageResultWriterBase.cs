using System.IO;
using NUnit.Framework;
using UnityEditor.TestTools.CodeCoverage.Utils;
using UnityEngine;

namespace UnityEditor.TestTools.CodeCoverage
{
    internal abstract class CoverageResultWriterBase<T> where T : class
    {
        protected CoverageSettings m_CoverageSettings;

        public T CoverageSession { get; set; }

        protected CoverageResultWriterBase(CoverageSettings coverageSettings)
        {
            m_CoverageSettings = coverageSettings;
        }

        public virtual void WriteCoverageSession(CoverageReportType reportType)
        {
#if UNITY_2020_1_OR_NEWER
            if (Compilation.CompilationPipeline.codeOptimization == Compilation.CodeOptimization.Release)
            {
                ResultsLogger.Log(ResultID.Warning_DebugCodeOptimization);

                if (!CommandLineManager.instance.batchmode)
                {
                    if (EditorUtility.DisplayDialog(
                        L10n.Tr("Code Coverage"),
                        L10n.Tr($"Code Coverage requires Code Optimization to be set to debug mode in order to obtain accurate coverage information. Do you want to switch to debug mode?\n\nNote that you would need to rerun {(CoverageRunData.instance.isRecording ? "the Coverage Recording session." : "the tests.")}"),
                        L10n.Tr("Switch to debug mode"),
                        L10n.Tr("Cancel")))
                    {
                        Compilation.CompilationPipeline.codeOptimization = Compilation.CodeOptimization.Debug;
                        EditorPrefs.SetBool("ScriptDebugInfoEnabled", true);
                    }
                }
            }
#endif
#if BURST_INSTALLED
            if (EditorPrefs.GetBool("BurstCompilation", false) && !CommandLineManager.instance.burstDisabled)
            {
                ResultsLogger.Log(ResultID.Warning_BurstCompilationEnabled);
            }
#endif
        }

        public void SetupCoveragePaths()
        {
            string folderName = CoverageUtils.GetProjectFolderName();
            string resultsRootDirectoryName = string.Concat(folderName, m_CoverageSettings.resultsFolderSuffix);

            bool isRecording = CoverageRunData.instance.isRecording;

            // We want to save in the 'Recording' subdirectory of the results folder when recording
#if TEST_FRAMEWORK_2_0_OR_NEWER
            string testSuite = isRecording ? "Recording" : "Automated";
#else
            string testSuite = isRecording ? "Recording" : TestContext.Parameters.Get("platform");
#endif
            string directoryName = CoverageUtils.JoinPaths(resultsRootDirectoryName, testSuite != null ? testSuite : "");

            m_CoverageSettings.rootFolderPath = CoverageUtils.GetRootFolderPath(m_CoverageSettings);
            m_CoverageSettings.historyFolderPath = CoverageUtils.GetHistoryFolderPath(m_CoverageSettings);

            string filePath = CoverageUtils.JoinPaths(directoryName, m_CoverageSettings.resultsFileName);
            filePath = CoverageUtils.JoinPaths(m_CoverageSettings.rootFolderPath, filePath);
            filePath = CoverageUtils.NormaliseFolderSeparators(filePath);
            CoverageUtils.EnsureFolderExists(Path.GetDirectoryName(filePath));

            m_CoverageSettings.resultsRootFolderPath = CoverageUtils.NormaliseFolderSeparators(CoverageUtils.JoinPaths(m_CoverageSettings.rootFolderPath, resultsRootDirectoryName));
            m_CoverageSettings.resultsFolderPath = CoverageUtils.NormaliseFolderSeparators(CoverageUtils.JoinPaths(m_CoverageSettings.rootFolderPath, directoryName));
            m_CoverageSettings.resultsFilePath = filePath;
        }

        public void ClearCoverageFolderIfExists()
        {
            CoverageUtils.ClearFolderIfExists(m_CoverageSettings.resultsFolderPath, "*.xml");
        }

        public string GetRootFullEmptyPath()
        {
            return CoverageUtils.JoinPaths(m_CoverageSettings.rootFolderPath, m_CoverageSettings.resultsFileName + "_fullEmpty" + "." + m_CoverageSettings.resultsFileExtension);
        }

        protected string GetNextFullFilePath()
        {
            int nextFileIndex = m_CoverageSettings.hasPersistentRunData ? CoverageUtils.GetNumberOfFilesInFolder(m_CoverageSettings.resultsFolderPath, "*.xml", SearchOption.TopDirectoryOnly) : 0;
            string fullFilePath = m_CoverageSettings.resultsFilePath + "_" + nextFileIndex.ToString("D4") + "." + m_CoverageSettings.resultsFileExtension;
            return fullFilePath;
        }
    }
}