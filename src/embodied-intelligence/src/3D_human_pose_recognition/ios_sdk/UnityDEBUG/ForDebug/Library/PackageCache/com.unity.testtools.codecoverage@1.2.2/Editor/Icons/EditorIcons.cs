using UnityEngine;

namespace UnityEditor.TestTools.CodeCoverage
{
    internal static class EditorIcons
    {
        public static Texture2D FolderOpened { get; private set; }
        public static Texture2D CoverageWindow { get; private set; }

        static EditorIcons()
        {
            FolderOpened = GetTexture("FolderOpened.png");
            CoverageWindow = GetTexture("CodeCoverage.png");
        }

        static Texture2D GetTexture(string path) => EditorGUIUtility.FindTexture("Packages/com.unity.testtools.codecoverage/Editor/Icons/" + path);
    }
}