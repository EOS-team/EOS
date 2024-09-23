using UnityEngine;

namespace UnityEditor.TestTools.CodeCoverage
{
    internal static class ReportGeneratorStyles
    {
        public static readonly GUIContent ProgressTitle = EditorGUIUtility.TrTextContent("Code Coverage");
        public static readonly GUIContent ProgressInfoCreating = EditorGUIUtility.TrTextContent("Generating the report..");
    }
}