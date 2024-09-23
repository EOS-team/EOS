using UnityEngine;
using UnityEditor.IMGUI.Controls;

namespace UnityEditor.TestTools.CodeCoverage
{
    class IncludedAssembliesPopupWindow : PopupWindowContent
    {
        readonly SearchField m_SearchField;
        readonly IncludedAssembliesTreeView m_TreeView;

        const float kWindowHeight = 221;

        public float Width { get; set; }

        static class Styles
        {
            public static readonly GUIContent SelectLabel = EditorGUIUtility.TrTextContent("Select:");
            public static readonly GUIContent SelectAllButtonLabel = EditorGUIUtility.TrTextContent("All", "Click this to select and include all the assemblies in the project. This includes both the assemblies under the 'Assets' folder and packages.\n\nIf searching, it will apply only to the assemblies visible in the list.");
            public static readonly GUIContent SelectAssetsButtonLabel = EditorGUIUtility.TrTextContent("Assets", "Click this to select and include only the assemblies under the 'Assets' folder.\n\nIf searching, it will apply only to the assemblies visible in the list.");
            public static readonly GUIContent SelectPackagesButtonLabel = EditorGUIUtility.TrTextContent("Packages", "Click this to select and include only the Packages' assemblies.\n\nIf searching, it will apply only to the assemblies visible in the list.");
            public static readonly GUIContent DeselectAllButtonLabel = EditorGUIUtility.TrTextContent("Deselect All", "Click this to deselect and exclude all the assemblies.\n\nIf searching, it will apply only to the assemblies visible in the list.");
        }

        public IncludedAssembliesPopupWindow(CodeCoverageWindow parent, string assembliesToInclude)
        {
            m_SearchField = new SearchField();
            m_TreeView = new IncludedAssembliesTreeView(parent, assembliesToInclude);
        }

        public override void OnGUI(Rect rect)
        {
            const int border = 4;
            const int topPadding = 12;
            const int searchHeight = 20;
            const int buttonHeight = 16;
            const int remainTop = topPadding + searchHeight + buttonHeight + border + border;

            float selectLabelWidth = EditorStyles.boldLabel.CalcSize(Styles.SelectLabel).x;
            float selectAllWidth = EditorStyles.miniButton.CalcSize(Styles.SelectAllButtonLabel).x;
            float selectAssetsWidth = EditorStyles.miniButton.CalcSize(Styles.SelectAssetsButtonLabel).x;
            float selectPackagesWidth = EditorStyles.miniButton.CalcSize(Styles.SelectPackagesButtonLabel).x;
            float deselectAllWidth = EditorStyles.miniButton.CalcSize(Styles.DeselectAllButtonLabel).x;

            Rect searchRect = new Rect(border, topPadding, rect.width - border * 2, searchHeight);
            Rect selectLabelRect = new Rect(border, topPadding + searchHeight + border, selectLabelWidth, searchHeight);
            Rect selectAllRect = new Rect(border + selectLabelWidth + border, topPadding + searchHeight + border, selectAllWidth, buttonHeight);
            Rect selectAssetsRect = new Rect(border + selectLabelWidth + border + selectAllWidth + border, topPadding + searchHeight + border, selectAssetsWidth, buttonHeight);
            Rect selectPackagesRect = new Rect(border + selectLabelWidth + border + selectAllWidth + border + selectAssetsWidth + border, topPadding + searchHeight + border, selectPackagesWidth, buttonHeight);
            Rect deselectAllRect = new Rect(rect.width - border - deselectAllWidth, topPadding + searchHeight + border, deselectAllWidth, buttonHeight);
            Rect remainingRect = new Rect(border, remainTop, rect.width - border * 2, rect.height - remainTop - border);

            m_TreeView.searchString = m_SearchField.OnGUI(searchRect, m_TreeView.searchString);

            GUI.Label(selectLabelRect, Styles.SelectLabel, EditorStyles.boldLabel);

            if (GUI.Button(selectAllRect, Styles.SelectAllButtonLabel, EditorStyles.miniButton))
            {
                m_TreeView.SelectAll();
            }

            if (GUI.Button(deselectAllRect, Styles.DeselectAllButtonLabel, EditorStyles.miniButton))
            {
                m_TreeView.DeselectAll();
            }

            if (GUI.Button(selectAssetsRect, Styles.SelectAssetsButtonLabel, EditorStyles.miniButton))
            {
                m_TreeView.SelectAssets();
            }

            if (GUI.Button(selectPackagesRect, Styles.SelectPackagesButtonLabel, EditorStyles.miniButton))
            {
                m_TreeView.SelectPackages();
            }

            m_TreeView.OnGUI(remainingRect);
        }

        public override Vector2 GetWindowSize()
        {
            return new Vector2(Mathf.Max(Width, m_TreeView.Width), kWindowHeight);
        }

        public override void OnOpen()
        {
            m_SearchField.SetFocus();
            base.OnOpen();
        }
    }
}