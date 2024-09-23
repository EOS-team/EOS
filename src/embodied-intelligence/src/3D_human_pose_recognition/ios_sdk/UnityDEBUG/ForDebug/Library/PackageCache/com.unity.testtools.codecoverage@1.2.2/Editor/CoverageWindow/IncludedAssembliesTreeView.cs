using UnityEditor.IMGUI.Controls;
using UnityEditor.Compilation;
using System.Text.RegularExpressions;
using System.Text;
using System.Linq;
using System;
using UnityEngine;
using UnityEditor.TestTools.CodeCoverage.Analytics;

namespace UnityEditor.TestTools.CodeCoverage
{
    class IncludedAssembliesTreeView : TreeView
    {
        string m_AssembliesToInclude;
        readonly CodeCoverageWindow m_Parent;
        const float kCheckBoxWidth = 42f;

        public float Width { get; set; } = 100f;

        public IncludedAssembliesTreeView(CodeCoverageWindow parent, string assembliesToInclude)
            : base(new TreeViewState())
        {
            m_AssembliesToInclude = assembliesToInclude;
            m_Parent = parent;
            showAlternatingRowBackgrounds = true;
            showBorder = true;
            Reload();
        }

        protected override bool CanMultiSelect(TreeViewItem item)
        {
            return false;
        }

        protected override TreeViewItem BuildRoot()
        {
            string[] includeAssemblyFilters = m_AssembliesToInclude.Split(new[] { ',' }, StringSplitOptions.RemoveEmptyEntries);

            Regex[] includeAssemblies = includeAssemblyFilters
                .Select(f => AssemblyFiltering.CreateFilterRegex(f))
                .ToArray();

            TreeViewItem root = new TreeViewItem(-1, -1);

            bool developerMode = EditorPrefs.GetBool("DeveloperMode", false);

            if (developerMode)
            {
                System.Reflection.Assembly[] assemblies = AssemblyFiltering.GetAllProjectAssembliesInternal();               
                int assembliesLength = assemblies.Length;

                GUIContent textContent = new GUIContent();
                for (int i = 0; i < assembliesLength; ++i)
                {
                    System.Reflection.Assembly assembly = assemblies[i];
                    bool enabled = includeAssemblies.Any(f => f.IsMatch(assembly.GetName().Name.ToLowerInvariant()));
                    root.AddChild(new AssembliesTreeViewItem() { id = i + 1, displayName = assembly.GetName().Name, Enabled = enabled });

                    textContent.text = assembly.GetName().Name;
                    float itemWidth = TreeView.DefaultStyles.label.CalcSize(textContent).x + kCheckBoxWidth;
                    if (Width < itemWidth)
                        Width = itemWidth;
                    
                }
            }
            else
            {
                Assembly[] assemblies = AssemblyFiltering.GetAllProjectAssemblies();
                int assembliesLength = assemblies.Length;

                GUIContent textContent = new GUIContent();
                for (int i = 0; i < assembliesLength; ++i)
                {
                    Assembly assembly = assemblies[i];
                    bool enabled = includeAssemblies.Any(f => f.IsMatch(assembly.name.ToLowerInvariant()));
                    root.AddChild(new AssembliesTreeViewItem() { id = i + 1, displayName = assembly.name, Enabled = enabled });

                    textContent.text = assembly.name;
                    float itemWidth = TreeView.DefaultStyles.label.CalcSize(textContent).x + kCheckBoxWidth;
                    if (Width < itemWidth)
                        Width = itemWidth;
                }
            }

            return root;
        }

        protected override void RowGUI(RowGUIArgs args)
        {
            AssembliesTreeViewItem item = args.item as AssembliesTreeViewItem;
            EditorGUI.BeginChangeCheck();
            bool enabled = EditorGUI.ToggleLeft(args.rowRect, args.label, item.Enabled);
            if (EditorGUI.EndChangeCheck())
            {
                item.Enabled = enabled;
                ApplyChanges();
            }
        }

        public void SelectAll()
        {
            ToggleAll(true);
        }

        public void DeselectAll()
        {
            ToggleAll(false);
        }

        public void SelectAssets()
        {
            m_AssembliesToInclude = AssemblyFiltering.GetUserOnlyAssembliesString();
            SelectFromString(m_AssembliesToInclude);
        }

        public void SelectPackages()
        {
            m_AssembliesToInclude = AssemblyFiltering.GetPackagesOnlyAssembliesString();
            SelectFromString(m_AssembliesToInclude);
        }

        private void SelectFromString(string assembliesToInclude)
        {
            string[] includeAssemblyFilters = assembliesToInclude.Split(new[] { ',' }, StringSplitOptions.RemoveEmptyEntries);

            Regex[] includeAssemblies = includeAssemblyFilters
                .Select(f => AssemblyFiltering.CreateFilterRegex(f))
                .ToArray();

            foreach (var child in rootItem.children)
            {
                AssembliesTreeViewItem childItem = child as AssembliesTreeViewItem;

                bool enabled = includeAssemblies.Any(f => f.IsMatch(childItem.displayName.ToLowerInvariant()));
                if (searchString == null)
                    childItem.Enabled = enabled;
                else if (DoesItemMatchSearch(child, searchString))
                    childItem.Enabled = enabled;
            }

            ApplyChanges();
        }

        private void ToggleAll(bool enabled)
        {
            foreach (var child in rootItem.children)
            {
                AssembliesTreeViewItem childItem = child as AssembliesTreeViewItem;
                if (searchString == null)
                    childItem.Enabled = enabled;
                else if (DoesItemMatchSearch(child, searchString))
                    childItem.Enabled = enabled;
            }

            ApplyChanges();
        }

        void ApplyChanges()
        {
            CoverageAnalytics.instance.CurrentCoverageEvent.updateAssembliesDialog = true;

            StringBuilder sb = new StringBuilder();
            foreach(var child in rootItem.children)
            {
                AssembliesTreeViewItem childItem = child as AssembliesTreeViewItem;
                if (childItem.Enabled)
                {
                    if (sb.Length > 0)
                        sb.Append(",");

                    sb.Append(childItem.displayName);
                }
            }

            m_Parent.AssembliesToInclude = sb.ToString();
            m_Parent.Repaint();
        }
    }

    class AssembliesTreeViewItem : TreeViewItem
    {
        public bool Enabled { get; set; }
    }
}