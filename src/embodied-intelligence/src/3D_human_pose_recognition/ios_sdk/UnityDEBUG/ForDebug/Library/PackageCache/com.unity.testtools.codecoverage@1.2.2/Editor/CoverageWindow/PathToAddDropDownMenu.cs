using UnityEditor.TestTools.CodeCoverage.Analytics;
using UnityEngine;

namespace UnityEditor.TestTools.CodeCoverage
{
    internal class PathToAddDropDownMenu
    {
        GenericMenu m_Menu;
        string m_PathsToFilter;
        readonly PathToAddHandler m_AddPathToIncludeHandler;
        readonly PathToAddHandler m_AddPathToExcludeHandler;
        readonly PathFilterType m_PathFilterType;

        static class Styles
        {
            public static readonly GUIContent AddFolderLabel = EditorGUIUtility.TrTextContent("Add Folder");
            public static readonly GUIContent AddFileLabel = EditorGUIUtility.TrTextContent("Add File");
        }

        public PathToAddDropDownMenu(CodeCoverageWindow parent, PathFilterType type)
        {
            m_PathFilterType = type;

            m_AddPathToIncludeHandler = new PathToAddHandler(parent, PathFilterType.Include);
            m_AddPathToExcludeHandler = new PathToAddHandler(parent, PathFilterType.Exclude);
        }

        private void PopulateMenu()
        {
            m_Menu = new GenericMenu();

            m_Menu.AddItem(Styles.AddFolderLabel, false, () => AddFolder());
            m_Menu.AddItem(Styles.AddFileLabel, false, () => AddFile());
        }

        public void Show(Rect position, string pathsToFilter)
        {
            m_PathsToFilter = pathsToFilter;
            
            PopulateMenu();

            m_Menu.DropDown(position);
        }

        private void AddFolder()
        {
            if (m_PathFilterType == PathFilterType.Include)
            {
                CoverageAnalytics.instance.CurrentCoverageEvent.selectAddFolder_IncludedPaths = true;
                m_AddPathToIncludeHandler.BrowseForDir(m_PathsToFilter);
            }
            else
            {
                CoverageAnalytics.instance.CurrentCoverageEvent.selectAddFolder_ExcludedPaths = true;
                m_AddPathToExcludeHandler.BrowseForDir(m_PathsToFilter);
            }
        }

        private void AddFile()
        {
            if (m_PathFilterType == PathFilterType.Include)
            {
                CoverageAnalytics.instance.CurrentCoverageEvent.selectAddFile_IncludedPaths = true;
                m_AddPathToIncludeHandler.BrowseForFile(m_PathsToFilter);
            }
            else
            {
                CoverageAnalytics.instance.CurrentCoverageEvent.selectAddFile_ExcludedPaths = true;
                m_AddPathToExcludeHandler.BrowseForFile(m_PathsToFilter);
            }
        }
    }
}
