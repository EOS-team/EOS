using System;
using System.Collections.Generic;
using System.Globalization;
using System.Linq;
using UnityEditor.IMGUI.Controls;
using UnityEngine;
using UnityEngine.Assertions;

namespace UnityEditor.Performance.ProfileAnalyzer
{
    class ProfileTreeViewItem : TreeViewItem
    {
        public MarkerData data { get; set; }
        public GUIContent[] cachedRowString;

        public ProfileTreeViewItem(int id, int depth, string displayName, MarkerData data) : base(id, depth, displayName)
        {
            this.data = data;
            cachedRowString = null;
        }
    }

    class ProfileTable : TreeView
    {
        Draw2D m_2D;
        ProfileDataView m_DataView;
        bool m_HideRemovedMarkers;
        ProfileAnalyzerWindow m_ProfileAnalyzerWindow;
        Color m_BarColor;
        float m_MaxMedian;
        int m_MaxCount;
        float m_MaxCountMean;
        double m_MaxTotal;

        const float kRowHeights = 20f;
        readonly List<TreeViewItem> m_Rows = new List<TreeViewItem>(100);

        // All columns
        public enum MyColumns
        {
            Name,
            State,
            Depth,
            Median,
            MedianBar,
            Mean,
            StandardDeviation,
            Min,
            Max,
            Range,
            Count,
            CountBar,
            CountMean,
            CountMeanBar,
            CountStandardDeviation,
            FirstFrame,
            AtMedian,
            Total,
            TotalBar,
            Threads,
        }

        static int m_MaxColumns;

        public enum SortOption
        {
            Name,
            State,
            Depth,
            Median,
            Mean,
            Min,
            Max,
            StandardDeviation,
            Range,
            Count,
            CountMean,
            CountStandardDeviation,
            FirstFrame,
            AtMedian,
            Total,
            Threads,
        }

        // Sort options per column
        SortOption[] m_SortOptions =
        {
            SortOption.Name,
            SortOption.State,
            SortOption.Depth,
            SortOption.Median,
            SortOption.Median,
            SortOption.Mean,
            SortOption.StandardDeviation,
            SortOption.Min,
            SortOption.Max,
            SortOption.Range,
            SortOption.Count,
            SortOption.Count,
            SortOption.CountMean,
            SortOption.CountMean,
            SortOption.CountStandardDeviation,
            SortOption.FirstFrame,
            SortOption.AtMedian,
            SortOption.Total,
            SortOption.Total,
            SortOption.Threads,
        };

        internal static class Styles
        {
            public static readonly GUIContent menuItemSelectFramesInAll = new GUIContent("Select Frames that contain this marker (within whole data set)", "");
            public static readonly GUIContent menuItemSelectFramesInCurrent = new GUIContent("Select Frames that contain this marker (within current selection)", "");
            //public static readonly GUIContent menuItemClearSelection = new GUIContent("Clear Selection");
            public static readonly GUIContent menuItemSelectFramesAll = new GUIContent("Select All");
            public static readonly GUIContent menuItemAddToIncludeFilter = new GUIContent("Add to Include Filter", "");
            public static readonly GUIContent menuItemAddToExcludeFilter = new GUIContent("Add to Exclude Filter", "");
            public static readonly GUIContent menuItemRemoveFromIncludeFilter = new GUIContent("Remove from Include Filter", "");
            public static readonly GUIContent menuItemRemoveFromExcludeFilter = new GUIContent("Remove from Exclude Filter", "");
            public static readonly GUIContent menuItemSetAsParentMarkerFilter = new GUIContent("Set as Parent Marker Filter", "");
            public static readonly GUIContent menuItemClearParentMarkerFilter = new GUIContent("Clear Parent Marker Filter", "");
            public static readonly GUIContent menuItemSetAsRemoveMarker = new GUIContent("Remove Marker", "");
            public static readonly GUIContent menuItemCopyToClipboard = new GUIContent("Copy to Clipboard", "");
        }

        public ProfileTable(TreeViewState state, MultiColumnHeader multicolumnHeader, ProfileDataView dataView, bool hideRemovedMarkers, ProfileAnalyzerWindow profileAnalyzerWindow, Draw2D draw2D, Color barColor) : base(state, multicolumnHeader)
        {
            m_2D = draw2D;
            m_DataView = dataView;
            m_HideRemovedMarkers = hideRemovedMarkers;
            m_ProfileAnalyzerWindow = profileAnalyzerWindow;
            m_BarColor = barColor;

            m_MaxColumns = Enum.GetValues(typeof(MyColumns)).Length;
            Assert.AreEqual(m_SortOptions.Length, m_MaxColumns, "Ensure number of sort options are in sync with number of MyColumns enum values");

            // Custom setup
            rowHeight = kRowHeights;
            showAlternatingRowBackgrounds = true;
            showBorder = true;
            customFoldoutYOffset = (kRowHeights - EditorGUIUtility.singleLineHeight) * 0.5f; // center foldout in the row since we also center content. See RowGUI
            // extraSpaceBeforeIconAndLabel = 0;
            multicolumnHeader.sortingChanged += OnSortingChanged;
            multicolumnHeader.visibleColumnsChanged += OnVisibleColumnsChanged;

            Reload();
        }

        protected override TreeViewItem BuildRoot()
        {
            int idForhiddenRoot = -1;
            int depthForHiddenRoot = -1;
            ProfileTreeViewItem root = new ProfileTreeViewItem(idForhiddenRoot, depthForHiddenRoot, "root", null);

            m_MaxMedian = 0.0f;
            m_MaxTotal = 0.0;
            m_MaxCount = 0;
            m_MaxCountMean = 0.0f;
            var markers = m_DataView.analysis.GetMarkers();
            for (int index = 0; index < markers.Count; ++index)
            {
                var marker = markers[index];
                if (!m_ProfileAnalyzerWindow.DoesMarkerPassFilter(marker.name))
                    continue;

                if (m_HideRemovedMarkers && marker.IsFullyIgnored())
                    continue;

               var item = new ProfileTreeViewItem(index, 0, marker.name, marker);
                root.AddChild(item);
                float ms = item.data.msMedian;
                if (ms > m_MaxMedian)
                    m_MaxMedian = ms;

                double msTotal = item.data.msTotal;
                if (msTotal > m_MaxTotal)
                    m_MaxTotal = msTotal;

                int count = item.data.count;
                if (count > m_MaxCount)
                    m_MaxCount = count;

                float countMean = item.data.countMean;
                if (countMean > m_MaxCountMean)
                    m_MaxCountMean = countMean;
            }

            return root;
        }

        protected override IList<TreeViewItem> BuildRows(TreeViewItem root)
        {
            m_Rows.Clear();

            if (rootItem != null && rootItem.children != null)
            {
                foreach (ProfileTreeViewItem node in rootItem.children)
                {
                    m_Rows.Add(node);
                }
            }

            SortIfNeeded(m_Rows);

            return m_Rows;
        }

        void OnSortingChanged(MultiColumnHeader _multiColumnHeader)
        {
            SortIfNeeded(GetRows());
        }

        protected virtual void OnVisibleColumnsChanged(MultiColumnHeader multiColumnHeader)
        {
            m_ProfileAnalyzerWindow.SetSingleModeColumns(multiColumnHeader.state.visibleColumns);
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

            // Sort the roots of the existing tree items
            SortByMultipleColumns();

            // Update the data with the sorted content
            rows.Clear();
            foreach (ProfileTreeViewItem node in rootItem.children)
            {
                rows.Add(node);
            }

            Repaint();
        }

        string GetThreadName(ProfileTreeViewItem item)
        {
            return m_ProfileAnalyzerWindow.GetUIThreadName(item.data.threads[0]);
        }

        string GetThreadNames(ProfileTreeViewItem item)
        {
            var uiNames = new List<string>();
            foreach (string threadNameWithIndex in item.data.threads)
            {
                string uiName = m_ProfileAnalyzerWindow.GetUIThreadName(threadNameWithIndex);

                uiNames.Add(uiName);
            }
            uiNames.Sort(m_ProfileAnalyzerWindow.CompareUINames);

            System.Text.StringBuilder sb = new System.Text.StringBuilder();
            bool first = true;
            foreach (var uiName in uiNames)
            {
                if (first)
                    first = false;
                else
                    sb.Append(", ");
                sb.Append(uiName);
            }

            return sb.ToString();
        }

        void SortByMultipleColumns()
        {
            var sortedColumns = multiColumnHeader.state.sortedColumns;

            if (sortedColumns.Length == 0)
            {
                return;
            }

            var myTypes = rootItem.children.Cast<ProfileTreeViewItem>();
            var orderedQuery = InitialOrder(myTypes, sortedColumns);
            for (int i = 1; i < sortedColumns.Length; i++)
            {
                SortOption sortOption = m_SortOptions[sortedColumns[i]];
                bool ascending = multiColumnHeader.IsSortedAscending(sortedColumns[i]);

                switch (sortOption)
                {
                    case SortOption.Name:
                        orderedQuery = orderedQuery.ThenBy(l => l.data.name, ascending);
                        break;
                    case SortOption.State:
                        orderedQuery = orderedQuery.ThenBy(l => State(l), ascending);
                        break;
                    case SortOption.Depth:
                        orderedQuery = orderedQuery.ThenBy(l => l.data.minDepth, ascending);
                        break;
                    case SortOption.Mean:
                        orderedQuery = orderedQuery.ThenBy(l => l.data.msMean, ascending);
                        break;
                    case SortOption.Median:
                        orderedQuery = orderedQuery.ThenBy(l => l.data.msMedian, ascending);
                        break;
                    case SortOption.StandardDeviation:
                        orderedQuery = orderedQuery.ThenBy(l => l.data.msStandardDeviation, ascending);
                        break;
                    case SortOption.Min:
                        orderedQuery = orderedQuery.ThenBy(l => l.data.msMin, ascending);
                        break;
                    case SortOption.Max:
                        orderedQuery = orderedQuery.ThenBy(l => l.data.msMax, ascending);
                        break;
                    case SortOption.Range:
                        orderedQuery = orderedQuery.ThenBy(l => (l.data.msMax - l.data.msMin), ascending);
                        break;
                    case SortOption.Count:
                        orderedQuery = orderedQuery.ThenBy(l => l.data.count, ascending);
                        break;
                    case SortOption.CountMean:
                        orderedQuery = orderedQuery.ThenBy(l => l.data.countMean, ascending);
                        break;
                    case SortOption.CountStandardDeviation:
                        orderedQuery = orderedQuery.ThenBy(l => l.data.countStandardDeviation, ascending);
                        break;
                    case SortOption.FirstFrame:
                        orderedQuery = orderedQuery.ThenBy(l => l.data.firstFrameIndex, ascending);
                        break;
                    case SortOption.AtMedian:
                        orderedQuery = orderedQuery.ThenBy(l => l.data.msAtMedian, ascending);
                        break;
                    case SortOption.Total:
                        orderedQuery = orderedQuery.ThenBy(l => l.data.msTotal, ascending);
                        break;
                    case SortOption.Threads:
                        orderedQuery = orderedQuery.ThenBy(l => l.cachedRowString != null ? l.cachedRowString[(int)MyColumns.Threads].text : GetThreadNames(l), ascending);
                        break;
                }
            }

            rootItem.children = orderedQuery.Cast<TreeViewItem>().ToList();
        }

        IOrderedEnumerable<ProfileTreeViewItem> InitialOrder(IEnumerable<ProfileTreeViewItem> myTypes, int[] history)
        {
            SortOption sortOption = m_SortOptions[history[0]];
            bool ascending = multiColumnHeader.IsSortedAscending(history[0]);
            switch (sortOption)
            {
                case SortOption.Name:
                    return myTypes.Order(l => l.data.name, ascending);
                case SortOption.State:
                    return myTypes.Order(l => State(l), ascending);
                case SortOption.Depth:
                    return myTypes.Order(l => l.data.minDepth, ascending);
                case SortOption.Mean:
                    return myTypes.Order(l => l.data.msMean, ascending);
                case SortOption.Median:
                    return myTypes.Order(l => l.data.msMedian, ascending);
                case SortOption.StandardDeviation:
                    return myTypes.Order(l => l.data.msStandardDeviation, ascending);
                case SortOption.Min:
                    return myTypes.Order(l => l.data.msMin, ascending);
                case SortOption.Max:
                    return myTypes.Order(l => l.data.msMax, ascending);
                case SortOption.Range:
                    return myTypes.Order(l => (l.data.msMax - l.data.msMin), ascending);
                case SortOption.Count:
                    return myTypes.Order(l => l.data.count, ascending);
                case SortOption.CountMean:
                    return myTypes.Order(l => l.data.countMean, ascending);
                case SortOption.CountStandardDeviation:
                    return myTypes.Order(l => l.data.countStandardDeviation, ascending);
                case SortOption.FirstFrame:
                    return myTypes.Order(l => l.data.firstFrameIndex, ascending);
                case SortOption.AtMedian:
                    return myTypes.Order(l => l.data.msAtMedian, ascending);
                case SortOption.Total:
                    return myTypes.Order(l => l.data.msTotal, ascending);
                case SortOption.Threads:
                    return myTypes.Order(l => l.cachedRowString != null ? l.cachedRowString[(int)MyColumns.Threads].text : GetThreadNames(l), ascending);
                default:
                    Assert.IsTrue(false, "Unhandled enum");
                    break;
            }

            // default
            return myTypes.Order(l => l.data.name, ascending);
        }

        int State(ProfileTreeViewItem item)
        {
            if (item.data.timeRemoved > 0.0)
            {
                return -3;
            }
            if (item.data.timeIgnored > 0.0)
            {
                if (item.data.IsFullyIgnored())
                    return -2;
                else
                    return -1;
            }

            return 0;
        }

        public bool ShowingHorizontalScroll
        {
            get
            {
                return showingHorizontalScrollBar;
            }
        }

        protected override void RowGUI(RowGUIArgs args)
        {
            var item = (ProfileTreeViewItem)args.item;

            var clipRect = m_2D.GetClipRect();
            clipRect.y = state.scrollPos.y;
            clipRect.x = state.scrollPos.x;
            m_2D.SetClipRect(clipRect);

            if (item.cachedRowString == null)
            {
                GenerateStrings(item);
            }

            for (int i = 0; i < args.GetNumVisibleColumns(); ++i)
            {
                CellGUI(args.GetCellRect(i), item, (MyColumns)args.GetColumn(i), ref args);
            }
            m_2D.ClearClipRect();
        }

        string ToDisplayUnits(float ms, bool showUnits = false, bool showFullValueWhenBelowZero = false)
        {
            return m_ProfileAnalyzerWindow.ToDisplayUnits(ms, showUnits, 0, showFullValueWhenBelowZero);
        }

        string ToDisplayUnits(double ms, bool showUnits = false, bool showFullValueWhenBelowZero = false)
        {
            return m_ProfileAnalyzerWindow.ToDisplayUnits(ms, showUnits, 0, showFullValueWhenBelowZero);
        }

        string ToTooltipDisplayUnits(float ms, bool showUnits = false, int onFrame = -1)
        {
            return m_ProfileAnalyzerWindow.ToTooltipDisplayUnits(ms, showUnits, onFrame);
        }

        string ToTooltipDisplayUnits(double ms, bool showUnits = false, int onFrame = -1)
        {
            return ToTooltipDisplayUnits((float)ms, showUnits, onFrame);
        }

        GUIContent ToDisplayUnitsWithTooltips(float ms, bool showUnits = false, int onFrame = -1)
        {
            return m_ProfileAnalyzerWindow.ToDisplayUnitsWithTooltips(ms, showUnits, onFrame);
        }

        GUIContent ToDisplayUnitsWithTooltips(double ms, bool showUnits = false, int onFrame = -1)
        {
            return ToDisplayUnitsWithTooltips((float)ms, showUnits, onFrame);
        }

        void CopyToClipboard(Event current, string text)
        {
            EditorGUIUtility.systemCopyBuffer = text;
        }

        GenericMenu GenerateActiveContextMenu(string markerName, Event evt, GUIContent content)
        {
            GenericMenu menu = new GenericMenu();

            menu.AddItem(Styles.menuItemSelectFramesInAll, false, () => m_ProfileAnalyzerWindow.SelectFramesContainingMarker(markerName, false));
            menu.AddItem(Styles.menuItemSelectFramesInCurrent, false, () => m_ProfileAnalyzerWindow.SelectFramesContainingMarker(markerName, true));

            if (m_ProfileAnalyzerWindow.AllSelected())
                menu.AddDisabledItem(Styles.menuItemSelectFramesAll);
            else
                menu.AddItem(Styles.menuItemSelectFramesAll, false, () => m_ProfileAnalyzerWindow.SelectAllFrames());

            menu.AddSeparator("");
            if (!m_ProfileAnalyzerWindow.GetNameFilters().Contains(markerName, StringComparer.OrdinalIgnoreCase))
                menu.AddItem(Styles.menuItemAddToIncludeFilter, false, () => m_ProfileAnalyzerWindow.AddToIncludeFilter(markerName));
            else
                menu.AddItem(Styles.menuItemRemoveFromIncludeFilter, false, () => m_ProfileAnalyzerWindow.RemoveFromIncludeFilter(markerName));
            if (!m_ProfileAnalyzerWindow.GetNameExcludes().Contains(markerName, StringComparer.OrdinalIgnoreCase))
                menu.AddItem(Styles.menuItemAddToExcludeFilter, false, () => m_ProfileAnalyzerWindow.AddToExcludeFilter(markerName));
            else
                menu.AddItem(Styles.menuItemRemoveFromExcludeFilter, false, () => m_ProfileAnalyzerWindow.RemoveFromExcludeFilter(markerName));
            menu.AddSeparator("");
            menu.AddItem(Styles.menuItemSetAsParentMarkerFilter, false, () => m_ProfileAnalyzerWindow.SetAsParentMarkerFilter(markerName));
            menu.AddItem(Styles.menuItemClearParentMarkerFilter, false, () => m_ProfileAnalyzerWindow.SetAsParentMarkerFilter(""));
            menu.AddSeparator("");
            menu.AddItem(Styles.menuItemSetAsRemoveMarker, false, () => m_ProfileAnalyzerWindow.SetAsRemoveMarker(markerName));
            menu.AddSeparator("");
            if (markerName != null && !string.IsNullOrEmpty(markerName))
                menu.AddItem(Styles.menuItemCopyToClipboard, false, () => CopyToClipboard(evt, markerName));

            return menu;
        }

        GenericMenu GenerateDisabledContextMenu(string markerName, GUIContent content)
        {
            GenericMenu menu = new GenericMenu();

            menu.AddDisabledItem(Styles.menuItemSelectFramesInAll);
            menu.AddDisabledItem(Styles.menuItemSelectFramesInCurrent);
            menu.AddDisabledItem(Styles.menuItemSelectFramesAll);

            menu.AddSeparator("");
            if (!m_ProfileAnalyzerWindow.GetNameFilters().Contains(markerName, StringComparer.OrdinalIgnoreCase))
                menu.AddDisabledItem(Styles.menuItemAddToIncludeFilter);
            else
                menu.AddDisabledItem(Styles.menuItemRemoveFromIncludeFilter);
            if (!m_ProfileAnalyzerWindow.GetNameExcludes().Contains(markerName, StringComparer.OrdinalIgnoreCase))
                menu.AddDisabledItem(Styles.menuItemAddToExcludeFilter);
            else
                menu.AddDisabledItem(Styles.menuItemRemoveFromExcludeFilter);
            menu.AddSeparator("");
            menu.AddDisabledItem(Styles.menuItemSetAsParentMarkerFilter);
            menu.AddDisabledItem(Styles.menuItemClearParentMarkerFilter);
            menu.AddSeparator("");
            menu.AddDisabledItem(Styles.menuItemSetAsRemoveMarker);
            menu.AddSeparator("");
            if (content != null && !string.IsNullOrEmpty(content.text))
                menu.AddDisabledItem(Styles.menuItemCopyToClipboard);

            return menu;
        }


        void ShowContextMenu(Rect cellRect, string markerName, GUIContent content)
        {
            Event current = Event.current;
            if (cellRect.Contains(current.mousePosition) && current.type == EventType.ContextClick)
            {
                GenericMenu menu;
                if (!m_ProfileAnalyzerWindow.IsAnalysisRunning())
                    menu = GenerateActiveContextMenu(markerName, current, content);
                else
                    menu = GenerateDisabledContextMenu(markerName, content);

                menu.ShowAsContext();

                current.Use();
            }
        }

        void ShowText(Rect rect, string text)
        {
            EditorGUI.LabelField(rect, text);
            //EditorGUI.TextArea(rect, text);
        }

        void ShowText(Rect rect, GUIContent content)
        {
            EditorGUI.LabelField(rect, content);
            //ShowText(rect, content.text);
        }

        void GenerateStrings(ProfileTreeViewItem item)
        {
            item.cachedRowString = new GUIContent[m_MaxColumns];

            int medianFrameIndex = m_ProfileAnalyzerWindow.GetRemappedUIFrameIndex(item.data.medianFrameIndex, m_DataView);
            int minFrameIndex = m_ProfileAnalyzerWindow.GetRemappedUIFrameIndex(item.data.minFrameIndex, m_DataView);
            int maxFrameIndex = m_ProfileAnalyzerWindow.GetRemappedUIFrameIndex(item.data.maxFrameIndex, m_DataView);
            int firstFrameIndex = m_ProfileAnalyzerWindow.GetRemappedUIFrameIndex(item.data.firstFrameIndex, m_DataView);
            int frameSummaryMedianFrameIndex = m_ProfileAnalyzerWindow.GetRemappedUIFrameIndex(m_DataView.analysis.GetFrameSummary().medianFrameIndex, m_DataView);

            if (item.data.timeRemoved > 0.0)
            {
                item.cachedRowString[(int)MyColumns.Name] = new GUIContent(item.data.name + " [Modified]", item.data.name + "\n\nTime reduced by removing child marker time");
                item.cachedRowString[(int)MyColumns.State] = new GUIContent("Modified", "Time reduced by removing child marker time");
            }
            else if (item.data.timeIgnored > 0.0)
            {
                if (item.data.IsFullyIgnored())
                {
                    item.cachedRowString[(int)MyColumns.Name] = new GUIContent(item.data.name + " [Removed]", item.data.name + "\n\nAll marker time removed");
                    item.cachedRowString[(int)MyColumns.State] = new GUIContent("Removed", "All marker time removed");
                }
                else
                {
                    item.cachedRowString[(int)MyColumns.Name] = new GUIContent(item.data.name + " [Partial Removal]", item.data.name + "\n\nSome marker time removed (some instances)");
                    item.cachedRowString[(int)MyColumns.State] = new GUIContent("Partial Removal", "Some marker time removed (some instances)");
                }
            }
            else
            {
                item.cachedRowString[(int)MyColumns.Name] = new GUIContent(item.data.name, item.data.name);
                item.cachedRowString[(int)MyColumns.State] = new GUIContent("", "");
            }
            item.cachedRowString[(int)MyColumns.Mean] = ToDisplayUnitsWithTooltips(item.data.msMean, false);
            item.cachedRowString[(int)MyColumns.Depth] = (item.data.minDepth == item.data.maxDepth) ? new GUIContent(string.Format("{0}", item.data.minDepth), "") : new GUIContent(string.Format("{0}-{1}", item.data.minDepth, item.data.maxDepth), "");
            item.cachedRowString[(int)MyColumns.Median] = ToDisplayUnitsWithTooltips(item.data.msMedian, false, medianFrameIndex);
            string tooltip = ToTooltipDisplayUnits(item.data.msMedian, true, medianFrameIndex);
            item.cachedRowString[(int)MyColumns.MedianBar] = new GUIContent("", tooltip);
            item.cachedRowString[(int)MyColumns.StandardDeviation] = ToDisplayUnitsWithTooltips(item.data.msStandardDeviation, false);
            item.cachedRowString[(int)MyColumns.Min] = ToDisplayUnitsWithTooltips(item.data.msMin, false, minFrameIndex);
            item.cachedRowString[(int)MyColumns.Max] = ToDisplayUnitsWithTooltips(item.data.msMax, false, maxFrameIndex);
            item.cachedRowString[(int)MyColumns.Range] = ToDisplayUnitsWithTooltips(item.data.msMax - item.data.msMin);
            item.cachedRowString[(int)MyColumns.Count] = new GUIContent(string.Format("{0}", item.data.count), "");
            item.cachedRowString[(int)MyColumns.CountBar] = new GUIContent("", string.Format("{0}", item.data.count));
            item.cachedRowString[(int)MyColumns.CountMean] = new GUIContent(string.Format(CultureInfo.InvariantCulture, "{0:f0}", item.data.countMean), "");
            item.cachedRowString[(int)MyColumns.CountMeanBar] = new GUIContent("", string.Format(CultureInfo.InvariantCulture, "{0:f0}", item.data.countMean));
            item.cachedRowString[(int)MyColumns.CountStandardDeviation] = new GUIContent(string.Format(CultureInfo.InvariantCulture, "{0:f0}", item.data.countStandardDeviation), string.Format(CultureInfo.InvariantCulture, "{0}", item.data.countStandardDeviation));
            item.cachedRowString[(int)MyColumns.FirstFrame] = new GUIContent(firstFrameIndex.ToString());
            item.cachedRowString[(int)MyColumns.AtMedian] = ToDisplayUnitsWithTooltips(item.data.msAtMedian, false, frameSummaryMedianFrameIndex);
            item.cachedRowString[(int)MyColumns.Total] = ToDisplayUnitsWithTooltips(item.data.msTotal);
            tooltip = ToTooltipDisplayUnits(item.data.msTotal, true, medianFrameIndex);
            item.cachedRowString[(int)MyColumns.TotalBar] = new GUIContent("", tooltip);

            string threadNames = GetThreadNames(item);
            item.cachedRowString[(int)MyColumns.Threads] = new GUIContent(threadNames, threadNames);
        }

        void ShowBar(Rect rect, float ms, float range, GUIContent content)
        {
            if (ms > 0.0f)
            {
                if (m_2D.DrawStart(rect))
                {
                    float w = Math.Max(1.0f, rect.width * ms / range);
                    m_2D.DrawFilledBox(0, 1, w, rect.height - 1, m_BarColor);
                    m_2D.DrawEnd();
                }
            }
            GUI.Label(rect, content);
        }

        void CellGUI(Rect cellRect, ProfileTreeViewItem item, MyColumns column, ref RowGUIArgs args)
        {
            // Center cell rect vertically (makes it easier to place controls, icons etc in the cells)
            CenterRectUsingSingleLineHeight(ref cellRect);

            GUIContent content = item.cachedRowString[(int)column];
            switch (column)
            {
                case MyColumns.Name:
                {
                    args.rowRect = cellRect;
                    //base.RowGUI(args);
                    //content = new GUIContent(item.data.name, item.data.name);
                    ShowText(cellRect, content);
                }
                break;

                case MyColumns.State:
                case MyColumns.Mean:
                case MyColumns.Depth:
                case MyColumns.Median:
                case MyColumns.StandardDeviation:
                case MyColumns.Min:
                case MyColumns.Max:
                case MyColumns.Range:
                case MyColumns.Count:
                case MyColumns.CountMean:
                case MyColumns.CountStandardDeviation:
                case MyColumns.AtMedian:
                case MyColumns.Total:
                case MyColumns.Threads:
                    ShowText(cellRect, content);
                    break;
                case MyColumns.MedianBar:
                    ShowBar(cellRect, item.data.msMedian, m_MaxMedian, content);
                    break;
                case MyColumns.TotalBar:
                    ShowBar(cellRect, (float)item.data.msTotal, (float)m_MaxTotal, content);
                    break;
                case MyColumns.CountBar:
                    ShowBar(cellRect, item.data.count, m_MaxCount, content);
                    break;
                case MyColumns.CountMeanBar:
                    ShowBar(cellRect, item.data.countMean, m_MaxCountMean, content);
                    break;
                case MyColumns.FirstFrame:
                    if (!m_ProfileAnalyzerWindow.IsProfilerWindowOpen() || !m_DataView.inSyncWithProfilerData)
                        GUI.enabled = false;
                    if (GUI.Button(cellRect, content))
                    {
                        m_ProfileAnalyzerWindow.SelectMarkerByIndex(item.id);
                        m_ProfileAnalyzerWindow.JumpToFrame(item.data.firstFrameIndex, m_DataView.data);
                    }

                    GUI.enabled = true;
                    break;
            }

            ShowContextMenu(cellRect, item.data.name, content);
        }

        // Misc
        //--------

        protected override bool CanMultiSelect(TreeViewItem item)
        {
            return false;
        }

        struct HeaderData
        {
            public readonly GUIContent content;
            public readonly float width;
            public readonly float minWidth;
            public readonly bool autoResize;
            public readonly bool allowToggleVisibility;
            public readonly bool ascending;

            public HeaderData(string name, string tooltip = "", float width = 50, float minWidth = 30, bool autoResize = true, bool allowToggleVisibility = true, bool ascending = false)
            {
                content = new GUIContent(name, tooltip);
                this.width = width;
                this.minWidth = minWidth;
                this.autoResize = autoResize;
                this.allowToggleVisibility = allowToggleVisibility;
                this.ascending = ascending;
            }
        }

        public static MultiColumnHeaderState CreateDefaultMultiColumnHeaderState(MarkerColumnFilter modeFilter)
        {
            var columnList = new List<MultiColumnHeaderState.Column>();
            HeaderData[] headerData = new HeaderData[]
            {
                new HeaderData("Marker Name", "Marker Name\n\nFrame marker time is total of all instances in frame", width: 300, minWidth: 100, autoResize: false, allowToggleVisibility: false, ascending: true),
                new HeaderData("State", "Status of marker entry (if modified or removed from frame time due to 'Remove' filter)"),
                new HeaderData("Depth", "Marker depth in marker hierarchy\n\nMay appear at multiple levels"),
                new HeaderData("Median", "Central marker time over all selected frames\n\nAlways present in data set\n1st of 2 central values for even frame count"),
                new HeaderData("Median Bar", "Central marker time over all selected frames", width: 50),
                new HeaderData("Mean", "Per frame marker time / number of non zero frames"),
                new HeaderData("SD", "Standard deviation in marker times"),
                new HeaderData("Min", "Minimum marker time"),
                new HeaderData("Max", "Maximum marker time"),
                new HeaderData("Range", "Difference between maximum and minimum"),
                new HeaderData("Count", "Marker count over all selected frames\n\nMultiple can occur per frame"),
                new HeaderData("Count Bar", "Marker count over all selected frames\n\nMultiple can occur per frame"),
                new HeaderData("Count Frame", "Average number of markers per frame\n\ntotal count / number of non zero frames", width: 70, minWidth: 50),
                new HeaderData("Count Frame Bar", "Average number of markers per frame\n\ntotal count / number of non zero frames", width: 70, minWidth: 50),
                new HeaderData("Count SD", "Standard deviation in marker per frame counts"),
                new HeaderData("1st", "First frame index that the marker appears on"),
                new HeaderData("At Median Frame", "Marker time on the median frame\n\nI.e. Marker total duration on the average frame", width: 90, minWidth: 50),
                new HeaderData("Total", "Marker total time over all selected frames"),
                new HeaderData("Total Bar", "Marker total time over all selected frames"),
                new HeaderData("Threads", "Threads the marker occurs on (with filtering applied)"),
            };
            foreach (var header in headerData)
            {
                columnList.Add(new MultiColumnHeaderState.Column
                {
                    headerContent = header.content,
                    headerTextAlignment = TextAlignment.Left,
                    sortedAscending = header.ascending,
                    sortingArrowAlignment = TextAlignment.Left,
                    width = header.width,
                    minWidth = header.minWidth,
                    autoResize = header.autoResize,
                    allowToggleVisibility = header.allowToggleVisibility
                });
            }
            ;
            var columns = columnList.ToArray();

            m_MaxColumns = Enum.GetValues(typeof(MyColumns)).Length;
            Assert.AreEqual(columns.Length, m_MaxColumns, "Number of columns should match number of enum values: You probably forgot to update one of them.");

            var state = new MultiColumnHeaderState(columns);
            SetMode(modeFilter, state);
            return state;
        }

        protected override void SelectionChanged(IList<int> selectedIds)
        {
            base.SelectionChanged(selectedIds);

            if (selectedIds.Count > 0)
            {
                m_ProfileAnalyzerWindow.SelectMarkerByIndex(selectedIds[0]);
                // A newly selected marker changes the marker summary's GUI content, conflicting with the previous layout pass. We need to exit GUI and re-layout.
                GUIUtility.ExitGUI();
            }
        }

        static int[] GetDefaultVisibleColumns(MarkerColumnFilter.Mode mode)
        {
            int[] visibleColumns;

            switch (mode)
            {
                default:
                case MarkerColumnFilter.Mode.Custom:
                case MarkerColumnFilter.Mode.TimeAndCount:
                    visibleColumns = new int[]
                    {
                        (int)MyColumns.Name,
                        (int)MyColumns.Depth,
                        (int)MyColumns.Median,
                        (int)MyColumns.MedianBar,
                        (int)MyColumns.Mean,
                        (int)MyColumns.Min,
                        (int)MyColumns.Max,
                        (int)MyColumns.Range,
                        (int)MyColumns.Count,
                        (int)MyColumns.CountMean,
                        (int)MyColumns.AtMedian,
                    };
                    break;
                case MarkerColumnFilter.Mode.Time:
                    visibleColumns = new int[]
                    {
                        (int)MyColumns.Name,
                        (int)MyColumns.Depth,
                        (int)MyColumns.Median,
                        (int)MyColumns.MedianBar,
                        (int)MyColumns.Min,
                        (int)MyColumns.Max,
                        (int)MyColumns.Range,
                        (int)MyColumns.AtMedian,
                    };
                    break;
                case MarkerColumnFilter.Mode.Totals:
                    visibleColumns = new int[]
                    {
                        (int)MyColumns.Name,
                        (int)MyColumns.Depth,
                        (int)MyColumns.Total,
                        (int)MyColumns.TotalBar,
                    };
                    break;
                case MarkerColumnFilter.Mode.TimeWithTotals:
                    visibleColumns = new int[]
                    {
                        (int)MyColumns.Name,
                        (int)MyColumns.Depth,
                        (int)MyColumns.Median,
                        (int)MyColumns.MedianBar,
                        (int)MyColumns.Min,
                        (int)MyColumns.Max,
                        (int)MyColumns.Range,
                        (int)MyColumns.AtMedian,
                        (int)MyColumns.Total,
                        (int)MyColumns.TotalBar,
                    };
                    break;
                case MarkerColumnFilter.Mode.CountTotals:
                    visibleColumns = new int[]
                    {
                        (int)MyColumns.Name,
                        (int)MyColumns.Depth,
                        (int)MyColumns.Count,
                        (int)MyColumns.CountBar,
                    };
                    break;
                case MarkerColumnFilter.Mode.CountPerFrame:
                    visibleColumns = new int[]
                    {
                        (int)MyColumns.Name,
                        (int)MyColumns.Depth,
                        (int)MyColumns.CountMean,
                        (int)MyColumns.CountMeanBar,
                    };
                    break;
                case MarkerColumnFilter.Mode.Depth:
                    visibleColumns = new int[]
                    {
                        (int)MyColumns.Name,
                        (int)MyColumns.Depth,
                    };
                    break;
                case MarkerColumnFilter.Mode.Threads:
                    visibleColumns = new int[]
                    {
                        (int)MyColumns.Name,
                        (int)MyColumns.Threads,
                    };
                    break;
            }

            return visibleColumns;
        }

        static void SetMode(MarkerColumnFilter modeFilter, MultiColumnHeaderState state)
        {
            switch (modeFilter.mode)
            {
                case MarkerColumnFilter.Mode.Custom:
                    if (modeFilter.visibleColumns == null)
                        state.visibleColumns = GetDefaultVisibleColumns(modeFilter.mode);
                    else
                        state.visibleColumns = modeFilter.visibleColumns;
                    break;
                default:
                    state.visibleColumns = GetDefaultVisibleColumns(modeFilter.mode);
                    break;
            }

            if (modeFilter.visibleColumns == null)
                modeFilter.visibleColumns = state.visibleColumns;
        }

        public void SetMode(MarkerColumnFilter modeFilter)
        {
            SetMode(modeFilter, multiColumnHeader.state);
            multiColumnHeader.ResizeToFit();
        }
    }

    static class MyExtensionMethods
    {
        public static IOrderedEnumerable<T> Order<T, TKey>(this IEnumerable<T> source, Func<T, TKey> selector, bool ascending)
        {
            if (ascending)
            {
                return source.OrderBy(selector);
            }
            else
            {
                return source.OrderByDescending(selector);
            }
        }

        public static IOrderedEnumerable<T> ThenBy<T, TKey>(this IOrderedEnumerable<T> source, Func<T, TKey> selector, bool ascending)
        {
            if (ascending)
            {
                return source.ThenBy(selector);
            }
            else
            {
                return source.ThenByDescending(selector);
            }
        }
    }
}
