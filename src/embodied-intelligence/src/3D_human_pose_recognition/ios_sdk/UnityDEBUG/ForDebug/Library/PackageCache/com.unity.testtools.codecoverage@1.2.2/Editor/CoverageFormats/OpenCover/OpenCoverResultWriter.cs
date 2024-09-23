using System.Xml.Serialization;
using System.IO;
using OpenCover.Framework.Model;
using UnityEditor.TestTools.CodeCoverage.Utils;

namespace UnityEditor.TestTools.CodeCoverage.OpenCover
{
    internal class OpenCoverResultWriter : CoverageResultWriterBase<CoverageSession>
    {
        public OpenCoverResultWriter(CoverageSettings coverageSettings) : base(coverageSettings)
        {
        }

        public override void WriteCoverageSession(CoverageReportType reportType)
        {
            bool atRoot = CommandLineManager.instance.generateRootEmptyReport && reportType == CoverageReportType.FullEmpty;

            XmlSerializer serializer = new XmlSerializer(typeof(CoverageSession));
            string fileFullPath = atRoot ? GetRootFullEmptyPath() : GetNextFullFilePath();
            if (!System.IO.File.Exists(fileFullPath))
            {
                using (TextWriter writer = new StreamWriter(fileFullPath))
                {
                    serializer.Serialize(writer, CoverageSession);
                    if (!CommandLineManager.instance.batchmode)
                        EditorUtility.DisplayProgressBar(OpenCoverReporterStyles.ProgressTitle.text, OpenCoverReporterStyles.ProgressWritingFile.text, 1f);
                }

                ResultsLogger.Log(reportType == CoverageReportType.CoveredMethodsOnly ? ResultID.Log_VisitedResultsSaved : ResultID.Log_ResultsSaved, fileFullPath);
                CoverageEventData.instance.AddSessionResultPath(fileFullPath);

                base.WriteCoverageSession(reportType);
            }
        }
    }
}
