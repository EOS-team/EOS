using UnityEngine;
using System.IO;
using System;
using System.Collections.Generic;
using UnityEditor.IMGUI.Controls;
using UnityEngine.Assertions;
using System.Linq;

namespace UnityEditor.Performance.ProfileAnalyzer
{
    class ThreadTreeViewItem : TreeViewItem
    {
        public readonly ThreadIdentifier threadIdentifier;

        public ThreadTreeViewItem(int id, int depth, string displayName, ThreadIdentifier threadIdentifier) : base(id, depth, displayName)
        {
            this.threadIdentifier = threadIdentifier;
        }
    }

    class ThreadTable : TreeView
    {
        const float kRowHeights = 20f;
        readonly List<TreeViewItem> m_Rows = new List<TreeViewItem>(100);

        List<string> m_ThreadNames;
        List<string> m_ThreadUINames;
        ThreadIdentifier m_AllThreadIdentifier;
        ThreadSelection m_ThreadSelection;

        GUIStyle activeLineStyle;

        public bool StateChanged { get; private set; }

        // All columns
        public enum MyColumns
        {
            GroupName,
            ThreadName,
            State,
        }

        public enum SortOption
        {
            GroupName,
            ThreadName,
            State,
        }

        // Sort options per column
        SortOption[] m_SortOptions =
        {
            SortOption.GroupName,
            SortOption.ThreadName,
            SortOption.State,
        };

        public enum ThreadSelected
        {
            Selected,
            Partial,
            NotSelected
        };

        public ThreadTable(TreeViewState state, MultiColumnHeader multicolumnHeader, List<string> threadNames, List<string> threadUINames, ThreadSelection threadSelection) : base(state, multicolumnHeader)
        {
            StateChanged = false;

            m_AllThreadIdentifier = new ThreadIdentifier();
            m_AllThreadIdentifier.SetName("All");
            m_AllThreadIdentifier.SetAll();

            Assert.AreEqual(m_SortOptions.Length, Enum.GetValues(typeof(MyColumns)).Length, "Ensure number of sort options are in sync with number of MyColumns enum values");

            // Custom setup
            rowHeight = kRowHeights;
            showAlternatingRowBackgrounds = true;
            columnIndexForTreeFoldouts = (int)(MyColumns.GroupName);
            showBorder = true;
            customFoldoutYOffset = (kRowHeights - EditorGUIUtility.singleLineHeight) * 0.5f; // center foldout in the row since we also center content. See RowGUI
            // extraSpaceBeforeIconAndLabel = 0;
            multicolumnHeader.sortingChanged += OnSortingChanged;
            multicolumnHeader.visibleColumnsChanged += OnVisibleColumnsChanged;

            m_ThreadNames = threadNames;
            m_ThreadUINames = threadUINames;
            m_ThreadSelection = new ThreadSelection(threadSelection);
            this.foldoutOverride += DoFoldout;
            Reload();
        }

        bool DoFoldout(Rect position, bool expandedstate, GUIStyle style)
        {
            return !(position.y < rowHeight) && GUI.Toggle(position, expandedstate, GUIContent.none, style);
        }

        public void ClearThreadSelection()
        {
            m_ThreadSelection.selection.Clear();
            m_ThreadSelection.groups.Clear();

            StateChanged = true;
            Reload();
        }

        public void SelectMain()
        {
            m_ThreadSelection.selection.Clear();
            m_ThreadSelection.groups.Clear();

            foreach (var threadName in m_ThreadNames)
            {
                if (threadName.StartsWith("All:"))
                    continue;

                if (threadName.EndsWith(":Main Thread"))    // Usually just starts with 1:
                    m_ThreadSelection.selection.Add(threadName);
            }

            StateChanged = true;
            Reload();
        }

        public void SelectCommon()
        {
            m_ThreadSelection.selection.Clear();
            m_ThreadSelection.groups.Clear();

            foreach (var threadName in m_ThreadNames)
            {
                if (threadName.StartsWith("All:"))
                    continue;

                if (threadName.EndsWith(":Render Thread"))  // Usually just starts with 1:
                    m_ThreadSelection.selection.Add(threadName);
                if (threadName.EndsWith(":Main Thread"))    // Usually just starts with 1:
                    m_ThreadSelection.selection.Add(threadName);
                if (threadName.EndsWith(":Job.Worker"))     // Mulitple jobs, number depends on processor setup
                    m_ThreadSelection.selection.Add(threadName);
            }

            StateChanged = true;
            Reload();
        }

        public ThreadSelection GetThreadSelection()
        {
            return m_ThreadSelection;
        }

        protected int GetChildCount(ThreadIdentifier selectedThreadIdentifier, out int selected)
        {
            int count = 0;
            int selectedCount = 0;

            if (selectedThreadIdentifier.index == ThreadIdentifier.kAll)
            {
                if (selectedThreadIdentifier.name == "All")
                {
                    for (int index = 0; index < m_ThreadNames.Count; ++index)
                    {
                        var threadNameWithIndex = m_ThreadNames[index];
                        var threadIdentifier = new ThreadIdentifier(threadNameWithIndex);

                        if (threadIdentifier.index != ThreadIdentifier.kAll)
                        {
                            count++;
                            if (m_ThreadSelection.selection.Contains(threadNameWithIndex))
                                selectedCount++;
                        }
                    }
                }
                else
                {
                    for (int index = 0; index < m_ThreadNames.Count; ++index)
                    {
                        var threadNameWithIndex = m_ThreadNames[index];
                        var threadIdentifier = new ThreadIdentifier(threadNameWithIndex);

                        if (selectedThreadIdentifier.name == threadIdentifier.name &&
                            threadIdentifier.index != ThreadIdentifier.kAll)
                        {
                            count++;
                            if (m_ThreadSelection.selection.Contains(threadNameWithIndex))
                                selectedCount++;
                        }
                    }
                }
            }

            selected = selectedCount;
            return count;
        }

        protected override TreeViewItem BuildRoot()
        {
            int idForHiddenRoot = -1;
            int depthForHiddenRoot = -1;
            ProfileTreeViewItem root = new ProfileTreeViewItem(idForHiddenRoot, depthForHiddenRoot, "root", null);

            int depth = 0;

            var top = new ThreadTreeViewItem(-1, depth, m_AllThreadIdentifier.name, m_AllThreadIdentifier);
            root.AddChild(top);

            var expandList = new List<int>() {-1};
            string lastThreadName = "";
            TreeViewItem node = root;

            for (int index = 0; index < m_ThreadNames.Count; ++index)
            {
                var threadNameWithIndex = m_ThreadNames[index];
                if (threadNameWithIndex == m_AllThreadIdentifier.threadNameWithIndex)
                    continue;

                var threadIdentifier = new ThreadIdentifier(threadNameWithIndex);
                var item = new ThreadTreeViewItem(index, depth, m_ThreadUINames[index], threadIdentifier);

                if (threadIdentifier.name != lastThreadName)
                {
                    // New threads at root
                    node = top;
                    depth = 0;
                }

                node.AddChild(item);


                if (threadIdentifier.name != lastThreadName)
                {
                    // Extra instances hang of the parent
                    lastThreadName = threadIdentifier.name;
                    node = item;
                    depth = 1;
                }
            }

            SetExpanded(expandList);

            SetupDepthsFromParentsAndChildren(root);

            return root;
        }

        void BuildRowRecursive(IList<TreeViewItem> rows, TreeViewItem item)
        {
            //if (item.children == null)
            //    return;

            if (!IsExpanded(item.id))
                return;

            foreach (ThreadTreeViewItem subNode in item.children)
            {
                rows.Add(subNode);

                if (subNode.children != null)
                    BuildRowRecursive(rows, subNode);
            }
        }

        void BuildAllRows(IList<TreeViewItem> rows, TreeViewItem rootItem)
        {
            rows.Clear();
            if (rootItem == null)
                return;

            if (rootItem.children == null)
                return;

            foreach (ThreadTreeViewItem node in rootItem.children)
            {
                rows.Add(node);

                if (node.children != null)
                    BuildRowRecursive(rows, node);
            }
        }

        protected override IList<TreeViewItem> BuildRows(TreeViewItem root)
        {
            BuildAllRows(m_Rows, root);

            SortIfNeeded(m_Rows);

            return m_Rows;
        }

        void OnSortingChanged(MultiColumnHeader _multiColumnHeader)
        {
            SortIfNeeded(GetRows());
        }

        void OnVisibleColumnsChanged(MultiColumnHeader _multiColumnHeader)
        {
            multiColumnHeader.ResizeToFit();
        }

        void SortIfNeeded(IList<TreeViewItem> rows)
        {
            if (rows.Count <= 1)
            {
                return;
            }

            if (multiColumnHeader.sortedColumnIndex == -1)
            {
                return; // No column to sort for (just use the order the data are in)
            }

            SortByMultipleColumns();

            BuildAllRows(rows, rootItem);

            Repaint();
        }

        string GetItemGroupName(ThreadTreeViewItem item)
        {
            string groupName;
            string threadName = item.threadIdentifier.name;
            threadName = ProfileData.GetThreadNameWithoutGroup(item.threadIdentifier.name, out groupName);

            return groupName;
        }

        List<TreeViewItem> SortChildrenByMultipleColumns(List<TreeViewItem> children)
        {
            int[] sortedColumns = multiColumnHeader.state.sortedColumns;

            if (sortedColumns.Length == 0)
            {
                return children;
            }

            var myTypes = children.Cast<ThreadTreeViewItem>();
            var orderedQuery = InitialOrder(myTypes, sortedColumns);
            for (int i = 0; i < sortedColumns.Length; i++)
            {
                SortOption sortOption = m_SortOptions[sortedColumns[i]];
                bool ascending = multiColumnHeader.IsSortedAscending(sortedColumns[i]);

                switch (sortOption)
                {
                    case SortOption.GroupName:
                        orderedQuery = orderedQuery.ThenBy(l => GetItemGroupName(l), ascending);
                        break;
                    case SortOption.ThreadName:
                        orderedQuery = orderedQuery.ThenBy(l => GetItemDisplayText(l), ascending);
                        break;
                    case SortOption.State:
                        orderedQuery = orderedQuery.ThenBy(l => GetStateSort(l), ascending);
                        break;
                }
            }

            return orderedQuery.Cast<TreeViewItem>().ToList();
        }

        void SortByMultipleColumns()
        {
            rootItem.children = SortChildrenByMultipleColumns(rootItem.children);

            // Sort all the next level children too (As 'All' is the only item at the top)
            for (int i = 0; i < rootItem.children.Count; i++)
            {
                var child = rootItem.children[0];
                child.children = SortChildrenByMultipleColumns(child.children);
            }
        }

        IOrderedEnumerable<ThreadTreeViewItem> InitialOrder(IEnumerable<ThreadTreeViewItem> myTypes, int[] history)
        {
            SortOption sortOption = m_SortOptions[history[0]];
            bool ascending = multiColumnHeader.IsSortedAscending(history[0]);
            switch (sortOption)
            {
                case SortOption.GroupName:
                    return myTypes.Order(l => GetItemGroupName(l), ascending);
                case SortOption.ThreadName:
                    return myTypes.Order(l => GetItemDisplayText(l), ascending);
                case SortOption.State:
                    return myTypes.Order(l => GetStateSort(l), ascending);
                default:
                    Assert.IsTrue(false, "Unhandled enum");
                    break;
            }

            // default
            return myTypes.Order(l => GetItemDisplayText(l), ascending);
        }

        protected override void RowGUI(RowGUIArgs args)
        {
            ThreadTreeViewItem item = (ThreadTreeViewItem)args.item;

            for (int i = 0; i < args.GetNumVisibleColumns(); ++i)
            {
                CellGUI(args.GetCellRect(i), item, (MyColumns)args.GetColumn(i), ref args);
            }
        }

        string GetStateSort(ThreadTreeViewItem item)
        {
            ThreadSelected threadSelected = GetThreadSelectedState(item.threadIdentifier);

            string sortString = ((int)threadSelected).ToString() + GetItemDisplayText(item);

            return sortString;
        }

        ThreadSelected GetThreadSelectedState(ThreadIdentifier selectedThreadIdentifier)
        {
            if (ProfileAnalyzer.MatchThreadFilter(selectedThreadIdentifier.threadNameWithIndex, m_ThreadSelection.selection))
                return ThreadSelected.Selected;

            // If querying the 'All' filter then check if all selected
            if (selectedThreadIdentifier.threadNameWithIndex == m_AllThreadIdentifier.threadNameWithIndex)
            {
                // Check all threads without All in the name are selected
                int count = 0;
                int selectedCount = 0;
                foreach (var threadNameWithIndex in m_ThreadNames)
                {
                    var threadIdentifier = new ThreadIdentifier(threadNameWithIndex);
                    if (threadIdentifier.index == ThreadIdentifier.kAll || threadIdentifier.index == ThreadIdentifier.kSingle)
                        continue;

                    if (m_ThreadSelection.selection.Contains(threadNameWithIndex))
                        selectedCount++;

                    count++;
                }

                if (selectedCount == count)
                    return ThreadSelected.Selected;

                if (selectedCount > 0)
                    return ThreadSelected.Partial;

                return ThreadSelected.NotSelected;
            }

            // Need to check 'All' and thread group All.
            if (selectedThreadIdentifier.index == ThreadIdentifier.kAll)
            {
                // Count all threads that match this thread group
                int count = 0;
                foreach (var threadNameWithIndex in m_ThreadNames)
                {
                    var threadIdentifier = new ThreadIdentifier(threadNameWithIndex);
                    if (threadIdentifier.index == ThreadIdentifier.kAll || threadIdentifier.index == ThreadIdentifier.kSingle)
                        continue;

                    if (selectedThreadIdentifier.name != threadIdentifier.name)
                        continue;

                    count++;
                }

                // Count all the threads we have selected that match this thread group
                int selectedCount = 0;
                foreach (var threadNameWithIndex in m_ThreadSelection.selection)
                {
                    var threadIdentifier = new ThreadIdentifier(threadNameWithIndex);
                    if (selectedThreadIdentifier.name != threadIdentifier.name)
                        continue;
                    if (threadIdentifier.index > count)
                        continue;

                    selectedCount++;
                }

                if (selectedCount == count)
                    return ThreadSelected.Selected;

                if (selectedCount > 0)
                    return ThreadSelected.Partial;
            }

            return ThreadSelected.NotSelected;
        }

        string GetItemDisplayText(ThreadTreeViewItem item)
        {
            int selectedChildren;
            int childCount = GetChildCount(item.threadIdentifier, out selectedChildren);

            string fullThreadName = item.threadIdentifier.name;
            string groupName;
            string threadName = ProfileData.GetThreadNameWithoutGroup(fullThreadName, out groupName);

            string displayThreadName = multiColumnHeader.IsColumnVisible((int)MyColumns.GroupName) ? threadName : fullThreadName;

            string text;
            if (childCount <= 1)
            {
                text = item.displayName;
            }
            else if (selectedChildren != childCount)
            {
                text = string.Format("{0} ({1} of {2})", displayThreadName, selectedChildren, childCount);
            }
            else
            {
                text = string.Format("{0} (All)", displayThreadName);
            }

            return text;
        }

        void GetThreadTreeViewItemInfo(ThreadTreeViewItem item, out string text, out string tooltip)
        {
            text = GetItemDisplayText(item);
            int selectedChildren;
            int childCount = GetChildCount(item.threadIdentifier, out selectedChildren);

            string groupName = GetItemGroupName(item);

            if (childCount <= 1)
            {
                tooltip = (groupName == "") ? text : string.Format("{0}\n{1}", text, groupName);
            }
            else if (selectedChildren != childCount)
            {
                tooltip = (groupName == "") ? text : string.Format("{0}\n{1}", text, groupName);
            }
            else
            {
                tooltip = (groupName == "") ? text : string.Format("{0}\n{1}", text, groupName);
            }
        }

        Rect DrawIndent(Rect rect, ThreadTreeViewItem item, ref RowGUIArgs args)
        {
            // The rect is assumed indented and sized after the content when pinging
            float indent = GetContentIndent(item) + extraSpaceBeforeIconAndLabel;
            rect.xMin += indent;

            int iconRectWidth = 16;
            int kSpaceBetweenIconAndText = 2;

            // Draw icon
            Rect iconRect = rect;
            iconRect.width = iconRectWidth;
            // iconRect.x += 7f;

            Texture icon = args.item.icon;
            if (icon != null)
                GUI.DrawTexture(iconRect, icon, ScaleMode.ScaleToFit);

            rect.xMin += icon == null ? 0 : iconRectWidth + kSpaceBetweenIconAndText;

            return rect;
        }

        void CellGUI(Rect cellRect, ThreadTreeViewItem item, MyColumns column, ref RowGUIArgs args)
        {
            // Center cell rect vertically (makes it easier to place controls, icons etc in the cells)
            CenterRectUsingSingleLineHeight(ref cellRect);

            switch (column)
            {
                case MyColumns.ThreadName:
                {
                    args.rowRect = cellRect;
                    // base.RowGUI(args);    // Required to show tree indenting

                    // Draw manually to keep indenting while add a tooltip
                    Rect rect = cellRect;
                    if (Event.current.rawType == EventType.Repaint)
                    {
                        string text;
                        string tooltip;
                        GetThreadTreeViewItemInfo(item, out text, out tooltip);
                        var content = new GUIContent(text, tooltip);

                        if (activeLineStyle == null)
                        {
                            // activeLineStyle = DefaultStyles.boldLabel;
                            activeLineStyle = new GUIStyle(DefaultStyles.label);
                            activeLineStyle.normal.textColor = DefaultStyles.boldLabel.onActive.textColor;
                        }

                        // rect = DrawIndent(rect, item, ref args);

                        //bool mouseOver = rect.Contains(Event.current.mousePosition);
                        //DefaultStyles.label.Draw(rect, content, mouseOver, false, args.selected, args.focused);

                        // Must use this call to draw tooltip
                        EditorGUI.LabelField(rect, content, args.selected ? activeLineStyle : DefaultStyles.label);
                    }
                }
                break;
                case MyColumns.GroupName:
                {
                    Rect rect = cellRect;
                    if (Event.current.rawType == EventType.Repaint)
                    {
                        rect = DrawIndent(rect, item, ref args);

                        string groupName = GetItemGroupName(item);
                        var content = new GUIContent(groupName, groupName);
                        EditorGUI.LabelField(rect, content);
                    }
                }
                break;
                case MyColumns.State:
                    bool oldState = GetThreadSelectedState(item.threadIdentifier) == ThreadSelected.Selected;
                    bool newState = EditorGUI.Toggle(cellRect, oldState);
                    if (newState != oldState)
                    {
                        if (item.threadIdentifier.threadNameWithIndex == m_AllThreadIdentifier.threadNameWithIndex)
                        {
                            // Record active groups
                            m_ThreadSelection.groups.Clear();
                            if (newState)
                            {
                                if (!m_ThreadSelection.groups.Contains(item.threadIdentifier.threadNameWithIndex))
                                    m_ThreadSelection.groups.Add(item.threadIdentifier.threadNameWithIndex);
                            }

                            // Update selection
                            m_ThreadSelection.selection.Clear();
                            if (newState)
                            {
                                foreach (string threadNameWithIndex in m_ThreadNames)
                                {
                                    if (threadNameWithIndex != m_AllThreadIdentifier.threadNameWithIndex)
                                    {
                                        var threadIdentifier = new ThreadIdentifier(threadNameWithIndex);
                                        if (threadIdentifier.index != ThreadIdentifier.kAll)
                                        {
                                            m_ThreadSelection.selection.Add(threadNameWithIndex);
                                        }
                                    }
                                }
                            }
                        }
                        else if (item.threadIdentifier.index == ThreadIdentifier.kAll)
                        {
                            // Record active groups
                            if (newState)
                            {
                                if (!m_ThreadSelection.groups.Contains(item.threadIdentifier.threadNameWithIndex))
                                    m_ThreadSelection.groups.Add(item.threadIdentifier.threadNameWithIndex);
                            }
                            else
                            {
                                m_ThreadSelection.groups.Remove(item.threadIdentifier.threadNameWithIndex);
                                // When turning off a sub group, turn of the 'all' group too
                                m_ThreadSelection.groups.Remove(m_AllThreadIdentifier.threadNameWithIndex);
                            }

                            // Update selection
                            if (newState)
                            {
                                foreach (string threadNameWithIndex in m_ThreadNames)
                                {
                                    var threadIdentifier = new ThreadIdentifier(threadNameWithIndex);
                                    if (threadIdentifier.name == item.threadIdentifier.name &&
                                        threadIdentifier.index != ThreadIdentifier.kAll)
                                    {
                                        if (!m_ThreadSelection.selection.Contains(threadNameWithIndex))
                                            m_ThreadSelection.selection.Add(threadNameWithIndex);
                                    }
                                }
                            }
                            else
                            {
                                var removeSelection = new List<string>();
                                foreach (string threadNameWithIndex in m_ThreadSelection.selection)
                                {
                                    var threadIdentifier = new ThreadIdentifier(threadNameWithIndex);
                                    if (threadIdentifier.name == item.threadIdentifier.name &&
                                        threadIdentifier.index != ThreadIdentifier.kAll)
                                    {
                                        removeSelection.Add(threadNameWithIndex);
                                    }
                                }
                                foreach (string threadNameWithIndex in removeSelection)
                                {
                                    m_ThreadSelection.selection.Remove(threadNameWithIndex);
                                }
                            }
                        }
                        else
                        {
                            if (newState)
                            {
                                m_ThreadSelection.selection.Add(item.threadIdentifier.threadNameWithIndex);
                            }
                            else
                            {
                                m_ThreadSelection.selection.Remove(item.threadIdentifier.threadNameWithIndex);

                                // Turn off any group its in too
                                var groupIdentifier = new ThreadIdentifier(item.threadIdentifier);
                                groupIdentifier.SetAll();
                                m_ThreadSelection.groups.Remove(groupIdentifier.threadNameWithIndex);

                                // Turn of the 'all' group too
                                m_ThreadSelection.groups.Remove(m_AllThreadIdentifier.threadNameWithIndex);
                            }
                        }

                        StateChanged = true;

                        // Re-sort
                        SortIfNeeded(GetRows());
                    }
                    break;
            }
        }

        // Misc
        //--------

        protected override bool CanMultiSelect(TreeViewItem item)
        {
            return false;
        }

        struct HeaderData
        {
            public GUIContent content;
            public float width;
            public float minWidth;
            public bool autoResize;
            public bool allowToggleVisibility;

            public HeaderData(string name, string tooltip = "", float _width = 50, float _minWidth = 30, bool _autoResize = true, bool _allowToggleVisibility = true)
            {
                content = new GUIContent(name, tooltip);
                width = _width;
                minWidth = _minWidth;
                autoResize = _autoResize;
                allowToggleVisibility = _allowToggleVisibility;
            }
        }

        public static MultiColumnHeaderState CreateDefaultMultiColumnHeaderState(float treeViewWidth)
        {
            var columnList = new List<MultiColumnHeaderState.Column>();
            HeaderData[] headerData = new HeaderData[]
            {
                new HeaderData("Group", "Thread Group", 200, 100, true, false),
                new HeaderData("Thread", "Thread Name", 350, 100, true, false),
                new HeaderData("Show", "Check to show this thread in the analysis views", 40, 100, false, false),
            };
            foreach (var header in headerData)
            {
                columnList.Add(new MultiColumnHeaderState.Column
                {
                    headerContent = header.content,
                    headerTextAlignment = TextAlignment.Left,
                    sortedAscending = true,
                    sortingArrowAlignment = TextAlignment.Left,
                    width = header.width,
                    minWidth = header.minWidth,
                    autoResize = header.autoResize,
                    allowToggleVisibility = header.allowToggleVisibility
                });
            }
            ;
            var columns = columnList.ToArray();

            Assert.AreEqual(columns.Length, Enum.GetValues(typeof(MyColumns)).Length, "Number of columns should match number of enum values: You probably forgot to update one of them.");

            var state = new MultiColumnHeaderState(columns);
            state.visibleColumns = new int[]
            {
                (int)MyColumns.GroupName,
                (int)MyColumns.ThreadName,
                (int)MyColumns.State,
            };
            return state;
        }

        protected override void SelectionChanged(IList<int> selectedIds)
        {
            base.SelectionChanged(selectedIds);

            if (selectedIds.Count > 0)
            {
            }
        }
    }

    internal class ThreadSelectionWindow : EditorWindow
    {
        private static HashSet<ThreadSelectionWindow> s_Instances = new HashSet<ThreadSelectionWindow>();
        ProfileAnalyzerWindow m_ProfileAnalyzerWindow;
        TreeViewState m_ThreadTreeViewState;
        //Static to store state between open/close
        static MultiColumnHeaderState m_ThreadMulticolumnHeaderState;
        ThreadTable m_ThreadTable;

        List<string> m_ThreadNames;
        List<string> m_ThreadUINames;
        ThreadSelection m_OriginalThreadSelection;
        bool m_EnableApplyButton = false;
        bool m_EnableResetButton = false;
        bool m_RequestClose;

        internal static class Styles
        {
            public static readonly GUIContent reset = new GUIContent("Reset", "Reset selection to previous set");
            public static readonly GUIContent clear = new GUIContent("Clear", "Clear selection below");
            public static readonly GUIContent main = new GUIContent("Main Only", "Select Main Thread only");
            public static readonly GUIContent common = new GUIContent("Common Set", "Select Common threads : Main, Render and Jobs");
            public static readonly GUIContent apply = new GUIContent("Apply", "");
        }

        static public bool IsOpen()
        {
            return s_Instances.Count > 0;
        }

        static public ThreadSelectionWindow Open(float screenX, float screenY, ProfileAnalyzerWindow profileAnalyzerWindow, ThreadSelection threadSelection, List<string> threadNames, List<string> threadUINames)
        {
            ThreadSelectionWindow window = GetWindow<ThreadSelectionWindow>(true, "Threads");
            window.minSize = new Vector2(380, 200);
            window.position = new Rect(screenX, screenY, 500, 500);
            window.SetData(profileAnalyzerWindow, threadSelection, threadNames, threadUINames);
            window.Show();

            return window;
        }

        void OnEnable()
        {
            m_RequestClose = false;
            s_Instances.Add(this);
        }

        private void OnDisable()
        {
            s_Instances.Remove(this);
        }

        void CreateTable(ProfileAnalyzerWindow profileAnalyzerWindow, List<string> threadNames, List<string> threadUINames, ThreadSelection threadSelection)
        {
            if (m_ThreadTreeViewState == null)
                m_ThreadTreeViewState = new TreeViewState();

            int sortedColumn;
            bool sortAscending;
            if (m_ThreadMulticolumnHeaderState == null)
            {
                m_ThreadMulticolumnHeaderState = ThreadTable.CreateDefaultMultiColumnHeaderState(700);
                sortedColumn = (int)ThreadTable.MyColumns.GroupName;
                sortAscending = true;
            }
            else
            {
                // Remember last sort key
                sortedColumn = m_ThreadMulticolumnHeaderState.sortedColumnIndex;
                sortAscending = m_ThreadMulticolumnHeaderState.columns[sortedColumn].sortedAscending;
            }

            var multiColumnHeader = new MultiColumnHeader(m_ThreadMulticolumnHeaderState);
            multiColumnHeader.SetSorting(sortedColumn, sortAscending);
            multiColumnHeader.ResizeToFit();
            m_ThreadTable = new ThreadTable(m_ThreadTreeViewState, multiColumnHeader, threadNames, threadUINames, threadSelection);
        }

        void SetData(ProfileAnalyzerWindow profileAnalyzerWindow, ThreadSelection threadSelection, List<string> threadNames, List<string> threadUINames)
        {
            m_ProfileAnalyzerWindow = profileAnalyzerWindow;
            m_OriginalThreadSelection = threadSelection;
            m_ThreadNames = threadNames;
            m_ThreadUINames = threadUINames;
            CreateTable(profileAnalyzerWindow, threadNames, threadUINames, threadSelection);
        }

        void OnDestroy()
        {
            // By design we now no longer apply the thread settings when closing the dialog.
            // Apply must be clicked to set them.
            // m_ProfileAnalyzerWindow.SetThreadSelection(m_ThreadTable.GetThreadSelection());
        }

        void OnGUI()
        {
            EditorGUILayout.BeginVertical(GUILayout.ExpandWidth(true));
            GUIStyle style = new GUIStyle(GUI.skin.label);
            style.alignment = TextAnchor.MiddleLeft;
            GUILayout.Label("Select Thread : ", style);

            EditorGUILayout.BeginHorizontal();
            bool lastEnabled = GUI.enabled;

            GUI.enabled = m_EnableResetButton;
            if (GUILayout.Button(Styles.reset, GUILayout.Width(50)))
            {
                // Reset the thread window contents only
                CreateTable(m_ProfileAnalyzerWindow, m_ThreadNames, m_ThreadUINames, m_OriginalThreadSelection);
                m_EnableApplyButton = true;
                m_EnableResetButton = false;
            }
            GUI.enabled = lastEnabled;

            if (GUILayout.Button(Styles.clear, GUILayout.Width(50)))
            {
                m_ThreadTable.ClearThreadSelection();
            }

            if (GUILayout.Button(Styles.main, GUILayout.Width(100)))
            {
                m_ThreadTable.SelectMain();
            }

            if (GUILayout.Button(Styles.common, GUILayout.Width(100)))
            {
                m_ThreadTable.SelectCommon();
            }

            GUI.enabled = m_EnableApplyButton && !m_ProfileAnalyzerWindow.IsAnalysisRunning();

            EditorGUILayout.Space();

            if (GUILayout.Button(Styles.apply, GUILayout.Width(50)))
            {
                m_ProfileAnalyzerWindow.SetThreadSelection(m_ThreadTable.GetThreadSelection());
                m_EnableApplyButton = false;
                m_EnableResetButton = true;
            }
            GUI.enabled = lastEnabled;

            EditorGUILayout.EndHorizontal();

            if (m_ThreadTable != null)
            {
                Rect r = EditorGUILayout.GetControlRect(GUILayout.ExpandHeight(true));
                m_ThreadTable.OnGUI(r);
            }

            EditorGUILayout.EndVertical();
        }

        void Update()
        {
            if (m_ThreadTable != null && m_ThreadTable.StateChanged)
            {
                m_EnableApplyButton = true;
                m_EnableResetButton = true;
            }

            if (m_RequestClose)
                Close();
        }

        void OnLostFocus()
        {
            m_RequestClose = true;
        }
    }
}
