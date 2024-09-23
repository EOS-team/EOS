using UnityEngine;
using UnityEditor.TestTools.CodeCoverage.Utils;

namespace UnityEditor.TestTools.CodeCoverage
{
    internal class FolderDropDownMenu
    {
        GenericMenu m_Menu;
        string m_Path;
        string m_Message;
        readonly CodeCoverageWindow m_Parent;
        readonly FolderType m_FolderType;

        static class Styles
        {
            public static readonly GUIContent ShowInExplorerLabel = EditorGUIUtility.TrTextContent("Open Containing Folder");
            public static readonly GUIContent ChangeLocationLabel = EditorGUIUtility.TrTextContent("Change Location");
            public static readonly GUIContent ResetToDefaultLabel = EditorGUIUtility.TrTextContent("Reset to Default Location");
        }

        public FolderDropDownMenu(CodeCoverageWindow parent, FolderType type)
        {
            m_Parent = parent;
            m_FolderType = type;
        }

        private void PopulateMenu()
        {
            m_Menu = new GenericMenu();

            m_Menu.AddItem(Styles.ShowInExplorerLabel, false, () => ShowInExplorer());
            m_Menu.AddItem(Styles.ChangeLocationLabel, false, () => ChangeLocation());

            if (m_Path.Equals(CoverageUtils.GetProjectPath()))
                m_Menu.AddDisabledItem(Styles.ResetToDefaultLabel);
            else
                m_Menu.AddItem(Styles.ResetToDefaultLabel, false, () => ResetToDefault());
        }

        public void Show(Rect position, string folderPath, string message)
        {
            m_Path = folderPath;
            m_Message = message;

            PopulateMenu();

            m_Menu.DropDown(position);
        }

        private void ShowInExplorer()
        {
            string path = m_FolderType == FolderType.Results ? 
                m_Parent.GetResultsRootFolder() : 
                m_Parent.GetReportHistoryFolder();

            EditorUtility.RevealInFinder(path);
        }

        private void ChangeLocation()
        {
            string candidate = CoverageUtils.BrowseForDir(m_Path, m_Message);
            if (m_FolderType == FolderType.Results)
                m_Parent.SetCoverageLocation(candidate);
            else
                m_Parent.SetCoverageHistoryLocation(candidate);
            m_Parent.LoseFocus();
        }

        private void ResetToDefault()
        {
            if (m_FolderType == FolderType.Results)
                m_Parent.SetDefaultCoverageLocation();
            else
                m_Parent.SetDefaultCoverageHistoryLocation();
            m_Parent.LoseFocus();
        }
    }
}
