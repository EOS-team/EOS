using UnityEngine;

namespace UnityEditor.TestTools.CodeCoverage
{
    internal static class OpenCoverReporterStyles
    {
        public static readonly GUIContent ProgressTitle = EditorGUIUtility.TrTextContent("Code Coverage");
        public static readonly GUIContent ProgressGatheringResults = EditorGUIUtility.TrTextContent("Gathering Coverage results..");
        public static readonly GUIContent ProgressWritingFile = EditorGUIUtility.TrTextContent("Writing Coverage results to file..");
    }
}