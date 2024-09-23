using System;
using System.Collections.Generic;
using System.Globalization;
using System.Linq;
using UnityEditor;
using UnityEditor.IMGUI.Controls;
using UnityEngine;
using UnityEngine.Assertions;

namespace UnityEditor.Performance.ProfileAnalyzer
{
    class ComparisonTreeViewItem : TreeViewItem
    {
        public MarkerPairing data { get; set; }
        public GUIContent[] cachedRowString;

        public ComparisonTreeViewItem(int id, int depth, string displayName, MarkerPairing data) : base(id, depth, displayName)
        {
            this.data = data;
            cachedRowString = null;
        }
    }

    class ComparisonTable : TreeView
    {
        Draw2D m_2D;
        ProfileDataView m_LeftDataView;
        ProfileDataView m_RightDataView;
        Color m_LeftColor;
        Color m_RightColor;
        List<MarkerPairing> m_Pairings;
        bool m_HideRemovedMarkers;
        ProfileAnalyzerWindow m_ProfileAnalyzerWindow;
        float m_DiffRange;
        float m_CountDiffRange;
        float m_CountMeanDiffRange;
        double m_TotalDiffRange;

        const float kRowHeights = 20f;
        readonly List<TreeViewItem> m_Rows = new List<TreeViewItem>(100);

        // All columns
        public enum MyColumns
        {
            Name,
            State,
            LeftMedian,
            LeftBar,
            RightBar,
            RightMedian,
            Diff,
            DiffPercent,
            AbsDiff,
            LeftCount,
            LeftCountBar,
            RightCountBar,
            RightCount,
            CountDiff,
            CountDiffPercent,
            AbsCountDiff,
            LeftCountMean,
            LeftCountMeanBar,
            RightCountMeanBar,
            RightCountMean,
            CountMeanDiff,
            CountMeanDiffPercent,
            AbsCountMeanDiff,
            LeftTotal,
            LeftTotalBar,
            RightTotalBar,
            RightTotal,
            TotalDiff,
            TotalDiffPercent,
            AbsTotalDiff,
            LeftDepth,
            RightDepth,
            DepthDiff,
            LeftThreads,
            RightThreads,
        }

        static int m_MaxColumns;

        public enum SortOption
        {
            Name,
            State,
            LeftMedian,
            RightMedian,
            Diff,
            ReverseDiff,
            DiffPercent,
            AbsDiff,
            LeftCount,
            RightCount,
            CountDiff,
            ReverseCountDiff,
            CountDiffPercent,
            AbsCountDiff,
            LeftCountMean,
            RightCountMean,
            CountMeanDiff,
            ReverseCountMeanDiff,
            CountMeanDiffPercent,
            AbsCountMeanDiff,
            LeftTotal,
            RightTotal,
            TotalDiff,
            ReverseTotalDiff,
            TotalDiffPercent,
            AbsTotalDiff,
            LeftDepth,
            RightDepth,
            DepthDiff,
            LeftThreads,
            RightThreads,
        }

        // Sort options per column
        SortOption[] m_SortOptions =
        {
            SortOption.Name,
            SortOption.State,
            SortOption.LeftMedian,
            SortOption.ReverseDiff,
            SortOption.Diff,
            SortOption.RightMedian,
            SortOption.Diff,
            SortOption.DiffPercent,
            SortOption.AbsDiff,
            SortOption.LeftCount,
            SortOption.ReverseCountDiff,
            SortOption.CountDiff,
            SortOption.RightCount,
            SortOption.CountDiff,
            SortOption.CountDiffPercent,
            SortOption.AbsCountDiff,
            SortOption.LeftCountMean,
            SortOption.ReverseCountMeanDiff,
            SortOption.CountMeanDiff,
            SortOption.RightCountMean,
            SortOption.CountMeanDiff,
            SortOption.CountMeanDiffPercent,
            SortOption.AbsCountMeanDiff,
            SortOption.LeftTotal,
            SortOption.ReverseTotalDiff,
            SortOption.TotalDiff,
            SortOption.RightTotal,
            SortOption.TotalDiff,
            SortOption.TotalDiffPercent,
            SortOption.AbsTotalDiff,
            SortOption.LeftDepth,
            SortOption.RightDepth,
            SortOption.DepthDiff,
            SortOption.LeftThreads,
            SortOption.RightThreads,
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

            public static readonly GUIContent invalidEntry = new GUIContent("-");
        }

        public ComparisonTable(TreeViewState state, MultiColumnHeader multicolumnHeader, ProfileDataView left, ProfileDataView right, List<MarkerPairing> pairings, bool hideRemovedMarkers, ProfileAnalyzerWindow profileAnalyzerWindow, Draw2D draw2D, Color leftColor, Color rightColor) : base(state, multicolumnHeader)
        {
            m_2D = draw2D;
            m_LeftDataView = left;
            m_RightDataView = right;
            m_LeftColor = leftColor;
            m_RightColor = rightColor;
            m_Pairings = pairings;
            m_HideRemovedMarkers = hideRemovedMarkers;
            m_ProfileAnalyzerWindow = profileAnalyzerWindow;

            m_MaxColumns = Enum.GetValues(typeof(MyColumns)).Length;
            Assert.AreEqual(m_SortOptions.Length, m_MaxColumns, "Ensure number of sort options are in sync with number of MyColumns enum values");

            // Custom setup
            rowHeight = kRowHeights;
            showAlternatingRowBackgrounds = true;
            showBorder = true;
            customFoldoutYOffset = (kRowHeights - EditorGUIUtility.singleLineHeight) * 0.5f; // center foldout in the row since we also center content. See RowGUI
            // extraSpaceBeforeIconAndLabel = 0;
            multicolumnHeader.sortingChanged += OnSortingChanged;
            multiColumnHeader.visibleColumnsChanged += OnVisibleColumnsChanged;

            Reload();
        }

        protected override TreeViewItem BuildRoot()
        {
            int idForhiddenRoot = -1;
            int depthForHiddenRoot = -1;
            ProfileTreeViewItem root = new ProfileTreeViewItem(idForhiddenRoot, depthForHiddenRoot, "root", null);

            float minDiff = float.MaxValue;
            float maxDiff = 0.0f;
            double totalMinDiff = float.MaxValue;
            double totalMaxDiff = 0.0f;
            float countMinDiff = float.MaxValue;
            float countMaxDiff = 0.0f;
            float countMeanMinDiff = float.MaxValue;
            float countMeanMaxDiff = 0.0f;
            for (int index = 0; index < m_Pairings.Count; ++index)
            {
                var pairing = m_Pairings[index];

                if (!m_ProfileAnalyzerWindow.DoesMarkerPassFilter(pairing.name))
                    continue;

                double timeIgnored;
                double msTotal;
                if (m_HideRemovedMarkers && IsFullyIgnored(pairing, out timeIgnored, out msTotal))
                    continue;

                var item = new ComparisonTreeViewItem(index, 0, pairing.name, pairing);
                root.AddChild(item);

                float diff = Diff(item);
                if (diff < minDiff)
                    minDiff = diff;
                if (diff > maxDiff && diff < float.MaxValue)
                    maxDiff = diff;

                double totalDiff = TotalDiff(item);
                if (totalDiff < totalMinDiff)
                    totalMinDiff = totalDiff;
                if (totalDiff > totalMaxDiff && totalDiff < float.MaxValue)
                    totalMaxDiff = totalDiff;

                float countDiff = CountDiff(item);
                if (countDiff < countMinDiff)
                    countMinDiff = countDiff;
                if (countDiff > countMaxDiff && countDiff < float.MaxValue)
                    countMaxDiff = countDiff;

                float countMeanDiff = CountMeanDiff(item);
                if (countMeanDiff < countMeanMinDiff)
                    countMeanMinDiff = countMeanDiff;
                if (countMeanDiff > countMeanMaxDiff && countMeanDiff < float.MaxValue)
                    countMeanMaxDiff = countMeanDiff;
            }

            m_DiffRange = Math.Max(Math.Abs(minDiff), Math.Abs(maxDiff));
            m_TotalDiffRange = Math.Max(Math.Abs(totalMinDiff), Math.Abs(totalMaxDiff));
            m_CountDiffRange = Math.Max(Math.Abs(countMinDiff), Math.Abs(countMaxDiff));
            m_CountMeanDiffRange = Math.Max(Math.Abs(countMeanMinDiff), Math.Abs(countMeanMaxDiff));

            return root;
        }

        protected override IList<TreeViewItem> BuildRows(TreeViewItem root)
        {
            m_Rows.Clear();

            if (rootItem != null && rootItem.children != null)
            {
                foreach (ComparisonTreeViewItem node in rootItem.children)
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
            m_ProfileAnalyzerWindow.SetComparisonModeColumns(multiColumnHeader.state.visibleColumns);
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
            foreach (var node in rootItem.children)
            {
                rows.Add(node);
            }

            Repaint();
        }

        void SortByMultipleColumns()
        {
            var sortedColumns = multiColumnHeader.state.sortedColumns;

            if (sortedColumns.Length == 0)
            {
                return;
            }

            var myTypes = rootItem.children.Cast<ComparisonTreeViewItem>();
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
                    case SortOption.LeftMedian:
                        orderedQuery = orderedQuery.ThenBy(l => LeftMedianSorting(l), ascending);
                        break;
                    case SortOption.RightMedian:
                        orderedQuery = orderedQuery.ThenBy(l => RightMedianSorting(l), ascending);
                        break;
                    case SortOption.Diff:
                        orderedQuery = orderedQuery.ThenBy(l => Diff(l), ascending);
                        break;
                    case SortOption.ReverseDiff:
                        orderedQuery = orderedQuery.ThenBy(l => - Diff(l), ascending);
                        break;
                    case SortOption.DiffPercent:
                        orderedQuery = orderedQuery.ThenBy(l => DiffPercent(l), ascending).ThenBy(l => Diff(l), ascending);
                        break;
                    case SortOption.AbsDiff:
                        orderedQuery = orderedQuery.ThenBy(l => AbsDiff(l), ascending);
                        break;
                    case SortOption.LeftCount:
                        orderedQuery = orderedQuery.ThenBy(l => LeftCount(l), ascending);
                        break;
                    case SortOption.RightCount:
                        orderedQuery = orderedQuery.ThenBy(l => RightCount(l), ascending);
                        break;
                    case SortOption.CountDiff:
                        orderedQuery = orderedQuery.ThenBy(l => CountDiff(l), ascending);
                        break;
                    case SortOption.ReverseCountDiff:
                        orderedQuery = orderedQuery.ThenBy(l => - CountDiff(l), ascending);
                        break;
                    case SortOption.CountDiffPercent:
                        orderedQuery = orderedQuery.ThenBy(l => CountDiffPercent(l), ascending).ThenBy(l => CountDiff(l), ascending);
                        break;
                    case SortOption.AbsCountDiff:
                        orderedQuery = orderedQuery.ThenBy(l => AbsCountDiff(l), ascending);
                        break;
                    case SortOption.LeftCountMean:
                        orderedQuery = orderedQuery.ThenBy(l => LeftCountMean(l), ascending);
                        break;
                    case SortOption.RightCountMean:
                        orderedQuery = orderedQuery.ThenBy(l => RightCountMean(l), ascending);
                        break;
                    case SortOption.CountMeanDiff:
                        orderedQuery = orderedQuery.ThenBy(l => CountMeanDiff(l), ascending);
                        break;
                    case SortOption.ReverseCountMeanDiff:
                        orderedQuery = orderedQuery.ThenBy(l => - CountMeanDiff(l), ascending);
                        break;
                    case SortOption.CountMeanDiffPercent:
                        orderedQuery = orderedQuery.ThenBy(l => CountMeanDiffPercent(l), ascending).ThenBy(l => CountMeanDiff(l), ascending);
                        break;
                    case SortOption.AbsCountMeanDiff:
                        orderedQuery = orderedQuery.ThenBy(l => AbsCountMeanDiff(l), ascending);
                        break;
                    case SortOption.LeftTotal:
                        orderedQuery = orderedQuery.ThenBy(l => LeftTotal(l), ascending);
                        break;
                    case SortOption.RightTotal:
                        orderedQuery = orderedQuery.ThenBy(l => RightTotal(l), ascending);
                        break;
                    case SortOption.TotalDiff:
                        orderedQuery = orderedQuery.ThenBy(l => TotalDiff(l), ascending);
                        break;
                    case SortOption.ReverseTotalDiff:
                        orderedQuery = orderedQuery.ThenBy(l => - TotalDiff(l), ascending);
                        break;
                    case SortOption.TotalDiffPercent:
                        orderedQuery = orderedQuery.ThenBy(l => TotalDiffPercent(l), ascending).ThenBy(l => TotalDiff(l), ascending);
                        break;
                    case SortOption.AbsTotalDiff:
                        orderedQuery = orderedQuery.ThenBy(l => AbsTotalDiff(l), ascending);
                        break;
                    case SortOption.LeftDepth:
                        orderedQuery = orderedQuery.ThenBy(l => LeftMinDepth(l), ascending);
                        break;
                    case SortOption.RightDepth:
                        orderedQuery = orderedQuery.ThenBy(l => RightMinDepth(l), ascending);
                        break;
                    case SortOption.DepthDiff:
                        orderedQuery = orderedQuery.ThenBy(l => DepthDiff(l), ascending);
                        break;
                    case SortOption.LeftThreads:
                        orderedQuery = orderedQuery.ThenBy(l => l.cachedRowString != null ? l.cachedRowString[(int)MyColumns.LeftThreads].text : LeftThreads(l), ascending);
                        break;
                    case SortOption.RightThreads:
                        orderedQuery = orderedQuery.ThenBy(l => l.cachedRowString != null ? l.cachedRowString[(int)MyColumns.RightThreads].text : RightThreads(l), ascending);
                        break;
                }
            }

            rootItem.children = orderedQuery.Cast<TreeViewItem>().ToList();
        }

        MarkerData GetLeftMarker(MarkerPairing pairing)
        {
            if (pairing.leftIndex < 0)
                return null;

            List<MarkerData> markers = m_LeftDataView.analysis.GetMarkers();
            if (pairing.leftIndex >= markers.Count)
                return null;

            return markers[pairing.leftIndex];
        }
        MarkerData GetRightMarker(MarkerPairing pairing)
        {
            if (pairing.rightIndex < 0)
                return null;

            List<MarkerData> markers = m_RightDataView.analysis.GetMarkers();
            if (pairing.rightIndex >= markers.Count)
                return null;

            return markers[pairing.rightIndex];
        }

        MarkerData GetLeftMarker(ComparisonTreeViewItem item)
        {
            return GetLeftMarker(item.data);
        }

        MarkerData GetRightMarker(ComparisonTreeViewItem item)
        {
            return GetRightMarker(item.data);
        }

        string LeftFirstThread(ComparisonTreeViewItem item)
        {
            return m_ProfileAnalyzerWindow.GetUIThreadName(MarkerData.GetFirstThread(GetLeftMarker(item)));
        }

        string RightFirstThread(ComparisonTreeViewItem item)
        {
            return m_ProfileAnalyzerWindow.GetUIThreadName(MarkerData.GetFirstThread(GetRightMarker(item)));
        }

        string GetThreadNames(MarkerData marker)
        {
            if (marker == null)
                return "";

            var uiNames = new List<string>();
            foreach (string threadNameWithIndex in marker.threads)
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

        string LeftThreads(ComparisonTreeViewItem item)
        {
            return GetThreadNames(GetLeftMarker(item));
        }

        string RightThreads(ComparisonTreeViewItem item)
        {
            return GetThreadNames(GetRightMarker(item));
        }

        float LeftMedianSorting(ComparisonTreeViewItem item)
        {
            var marker = GetLeftMarker(item);
            if (marker == null)
                return -1f;
            return MarkerData.GetMsMedian(marker);
        }

        float LeftMedian(ComparisonTreeViewItem item)
        {
            return MarkerData.GetMsMedian(GetLeftMarker(item));
        }

        int LeftMedianFrameIndex(ComparisonTreeViewItem item)
        {
            return MarkerData.GetMedianFrameIndex(GetLeftMarker(item));
        }

        float RightMedianSorting(ComparisonTreeViewItem item)
        {
            var marker = GetRightMarker(item);
            if (marker == null)
                return -1f;
            return MarkerData.GetMsMedian(marker);
        }

        float RightMedian(ComparisonTreeViewItem item)
        {
            return MarkerData.GetMsMedian(GetRightMarker(item));
        }

        int RightMedianFrameIndex(ComparisonTreeViewItem item)
        {
            return MarkerData.GetMedianFrameIndex(GetRightMarker(item));
        }

        float Diff(ComparisonTreeViewItem item)
        {
            return RightMedian(item) - LeftMedian(item);
        }

        float DiffPercent(float left, float right)
        {
            if (left == 0f)
                return float.MaxValue;
            if (right == 0f)
                return float.MinValue;

            float diff = right - left;
            return (100f * diff) / left;
        }

        float DiffPercent(double left, double right)
        {
            if (left == 0.0)
                return float.MaxValue;
            if (right == 0.0)
                return float.MinValue;

            double diff = right - left;
            return (float)((100.0 * diff) / left);
        }

        float DiffPercent(ComparisonTreeViewItem item)
        {
            float right = RightMedian(item);
            float left = LeftMedian(item);
            return DiffPercent(left, right);
        }

        float AbsDiff(ComparisonTreeViewItem item)
        {
            return Math.Abs(Diff(item));
        }

        float LeftCount(ComparisonTreeViewItem item)
        {
            return MarkerData.GetCount(GetLeftMarker(item));
        }

        float RightCount(ComparisonTreeViewItem item)
        {
            return MarkerData.GetCount(GetRightMarker(item));
        }

        float CountDiff(ComparisonTreeViewItem item)
        {
            return RightCount(item) - LeftCount(item);
        }

        float CountDiffPercent(ComparisonTreeViewItem item)
        {
            float right = RightCount(item);
            float left = LeftCount(item);
            return DiffPercent(left, right);
        }

        float AbsCountDiff(ComparisonTreeViewItem item)
        {
            return Math.Abs(CountDiff(item));
        }

        float LeftCountMean(ComparisonTreeViewItem item)
        {
            return MarkerData.GetCountMean(GetLeftMarker(item));
        }

        float RightCountMean(ComparisonTreeViewItem item)
        {
            return MarkerData.GetCountMean(GetRightMarker(item));
        }

        float CountMeanDiff(ComparisonTreeViewItem item)
        {
            return RightCountMean(item) - LeftCountMean(item);
        }

        float CountMeanDiffPercent(ComparisonTreeViewItem item)
        {
            float right = RightCountMean(item);
            float left = LeftCountMean(item);
            return DiffPercent(left, right);
        }

        float AbsCountMeanDiff(ComparisonTreeViewItem item)
        {
            return Math.Abs(CountMeanDiff(item));
        }

        double LeftTotal(ComparisonTreeViewItem item)
        {
            return MarkerData.GetMsTotal(GetLeftMarker(item));
        }

        double RightTotal(ComparisonTreeViewItem item)
        {
            return MarkerData.GetMsTotal(GetRightMarker(item));
        }

        double TotalDiff(ComparisonTreeViewItem item)
        {
            return RightTotal(item) - LeftTotal(item);
        }

        float TotalDiffPercent(ComparisonTreeViewItem item)
        {
            double right = RightTotal(item);
            double left = LeftTotal(item);
            return DiffPercent(left, right);
        }

        double AbsTotalDiff(ComparisonTreeViewItem item)
        {
            return Math.Abs(TotalDiff(item));
        }

        int LeftMinDepth(ComparisonTreeViewItem item)
        {
            return MarkerData.GetMinDepth(GetLeftMarker(item));
        }

        int RightMinDepth(ComparisonTreeViewItem item)
        {
            return MarkerData.GetMinDepth(GetRightMarker(item));
        }

        int LeftMaxDepth(ComparisonTreeViewItem item)
        {
            return MarkerData.GetMaxDepth(GetLeftMarker(item));
        }

        int RightMaxDepth(ComparisonTreeViewItem item)
        {
            return MarkerData.GetMaxDepth(GetRightMarker(item));
        }

        int DepthDiff(ComparisonTreeViewItem item)
        {
            if (item.data.leftIndex < 0)
                return int.MaxValue;
            if (item.data.rightIndex < 0)
                return int.MaxValue - 1;

            return RightMinDepth(item) - LeftMinDepth(item);
        }

        double TimeRemoved(ComparisonTreeViewItem item)
        {
            double removed;
            removed = MarkerData.GetTimeRemoved(GetLeftMarker(item));
            if (removed > 0.0)
                return removed;
            removed = MarkerData.GetTimeRemoved(GetRightMarker(item));
            if (removed > 0.0)
                return removed;

            return 0.0;
        }

        double TimeIgnored(MarkerPairing pairing)
        {
            double ignored;
            ignored = MarkerData.GetTimeIgnored(GetLeftMarker(pairing));
            if (ignored > 0.0)
                return ignored;
            ignored = MarkerData.GetTimeIgnored(GetRightMarker(pairing));
            if (ignored > 0.0)
                return ignored;

            return 0.0;
        }

        double TimeIgnored(ComparisonTreeViewItem item)
        {
            double ignored;
            ignored = MarkerData.GetTimeIgnored(GetLeftMarker(item));
            if (ignored > 0.0)
                return ignored;
            ignored = MarkerData.GetTimeIgnored(GetRightMarker(item));
            if (ignored > 0.0)
                return ignored;

            return 0.0;
        }

        bool IsFullyIgnored(MarkerPairing pairing, out double timeIgnored, out double msTotal)
        {
            MarkerData left = GetLeftMarker(pairing);
            MarkerData right = GetRightMarker(pairing);

            double leftTimeIgnored = MarkerData.GetTimeIgnored(left);
            double rightTimeIgnored = MarkerData.GetTimeIgnored(right);
            double leftMsTotal = MarkerData.GetMsTotal(left);
            double rightMsTotal = MarkerData.GetMsTotal(right);

            bool result = false;
            if (leftTimeIgnored > 0.0 || rightTimeIgnored > 0.0)
            {
                if (leftMsTotal == 0.0 && rightMsTotal == 0.0)
                    result = true;
            }

            if (leftTimeIgnored > 0.0)
            {
                timeIgnored = leftTimeIgnored;
                msTotal = leftMsTotal;
            }
            else
            {
                timeIgnored = rightTimeIgnored;
                msTotal = rightMsTotal;
            }

            return result;
        }

        bool IsFullyIgnored(ComparisonTreeViewItem item, out double timeIgnored, out double msTotal)
        {
            return IsFullyIgnored(item.data, out timeIgnored, out msTotal);
        }

        int State(ComparisonTreeViewItem item)
        {
            if (TimeRemoved(item) > 0.0)
            {
                return -3;
            }
            if (TimeIgnored(item) > 0.0)
            {
                double timeIgnored;
                double msTotal;
                bool removed = IsFullyIgnored(item, out timeIgnored, out msTotal);
                if (removed)
                    return -2;
                else
                    return -1;
            }

            return 0;
        }

        IOrderedEnumerable<ComparisonTreeViewItem> InitialOrder(IEnumerable<ComparisonTreeViewItem> myTypes, int[] history)
        {
            SortOption sortOption = m_SortOptions[history[0]];
            bool ascending = multiColumnHeader.IsSortedAscending(history[0]);
            switch (sortOption)
            {
                case SortOption.Name:
                    return myTypes.Order(l => l.data.name, ascending);
                case SortOption.State:
                    return myTypes.Order(l => State(l), ascending);
                case SortOption.LeftMedian:
                    return myTypes.Order(l => LeftMedianSorting(l), ascending);
                case SortOption.RightMedian:
                    return myTypes.Order(l => RightMedianSorting(l), ascending);
                case SortOption.Diff:
                    return myTypes.Order(l => Diff(l), ascending);
                case SortOption.ReverseDiff:
                    return myTypes.Order(l => - Diff(l), ascending);
                case SortOption.DiffPercent:
                    return myTypes.Order(l => DiffPercent(l), ascending).ThenBy(l => Diff(l), ascending);
                case SortOption.AbsDiff:
                    return myTypes.Order(l => AbsDiff(l), ascending);
                case SortOption.LeftCount:
                    return myTypes.Order(l => LeftCount(l), ascending);
                case SortOption.RightCount:
                    return myTypes.Order(l => RightCount(l), ascending);
                case SortOption.CountDiff:
                    return myTypes.Order(l => CountDiff(l), ascending);
                case SortOption.ReverseCountDiff:
                    return myTypes.Order(l => - CountDiff(l), ascending);
                case SortOption.CountDiffPercent:
                    return myTypes.Order(l => CountDiffPercent(l), ascending).ThenBy(l => CountDiff(l), ascending);
                case SortOption.AbsCountDiff:
                    return myTypes.Order(l => AbsCountDiff(l), ascending);
                case SortOption.LeftCountMean:
                    return myTypes.Order(l => LeftCountMean(l), ascending);
                case SortOption.RightCountMean:
                    return myTypes.Order(l => RightCountMean(l), ascending);
                case SortOption.CountMeanDiff:
                    return myTypes.Order(l => CountMeanDiff(l), ascending);
                case SortOption.ReverseCountMeanDiff:
                    return myTypes.Order(l => - CountMeanDiff(l), ascending);
                case SortOption.CountMeanDiffPercent:
                    return myTypes.Order(l => CountMeanDiffPercent(l), ascending).ThenBy(l => CountMeanDiff(l), ascending);
                case SortOption.AbsCountMeanDiff:
                    return myTypes.Order(l => AbsCountMeanDiff(l), ascending);
                case SortOption.LeftTotal:
                    return myTypes.Order(l => LeftTotal(l), ascending);
                case SortOption.RightTotal:
                    return myTypes.Order(l => RightTotal(l), ascending);
                case SortOption.TotalDiff:
                    return myTypes.Order(l => TotalDiff(l), ascending);
                case SortOption.ReverseTotalDiff:
                    return myTypes.Order(l => - TotalDiff(l), ascending);
                case SortOption.TotalDiffPercent:
                    return myTypes.Order(l => TotalDiffPercent(l), ascending).ThenBy(l => TotalDiff(l), ascending);
                case SortOption.AbsTotalDiff:
                    return myTypes.Order(l => AbsTotalDiff(l), ascending);
                case SortOption.LeftDepth:
                    return myTypes.Order(l => LeftMinDepth(l), ascending);
                case SortOption.RightDepth:
                    return myTypes.Order(l => RightMinDepth(l), ascending);
                case SortOption.DepthDiff:
                    return myTypes.Order(l => DepthDiff(l), ascending);
                case SortOption.LeftThreads:
                    return myTypes.Order(l => l.cachedRowString != null ? l.cachedRowString[(int)MyColumns.LeftThreads].text : LeftThreads(l), ascending);
                case SortOption.RightThreads:
                    return myTypes.Order(l => l.cachedRowString != null ? l.cachedRowString[(int)MyColumns.RightThreads].text : RightThreads(l), ascending);
                default:
                    Assert.IsTrue(false, "Unhandled enum");
                    break;
            }

            // default
            return myTypes.Order(l => l.data.name, ascending);
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
            var item = (ComparisonTreeViewItem)args.item;

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

        static bool IsBar(MyColumns col)
        {
            switch (col)
            {
                case MyColumns.LeftBar:
                case MyColumns.RightBar:
                case MyColumns.LeftCountBar:
                case MyColumns.RightCountBar:
                case MyColumns.LeftTotalBar:
                case MyColumns.RightTotalBar:
                case MyColumns.LeftCountMeanBar:
                case MyColumns.RightCountMeanBar:
                    return true;
            }

            return false;
        }

        internal void WriteTableContentsCSV(System.IO.StreamWriter writer)
        {
            var visibleColumns = multiColumnHeader.state.visibleColumns.Where(c => !IsBar((MyColumns)c)).ToArray();
            for (int i = 0, n = visibleColumns.Length; i < n; i++)
            {
                if (i != 0)
                    writer.Write(';');
                var colIdx = visibleColumns[i];
                writer.Write(s_HeaderData[colIdx].content.text);
            }
            writer.WriteLine();

            foreach (var child in rootItem.children)
            {
                var item = (ComparisonTreeViewItem)child;
                if (item.cachedRowString == null)
                    GenerateStrings(item);
                for (int i = 0, n = visibleColumns.Length; i < n; i++)
                {
                    if (i != 0)
                        writer.Write(';');
                    var colIdx = visibleColumns[i];
                    writer.Write(item.cachedRowString[colIdx].text);
                }

                writer.WriteLine();
            }
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

        string PercentString(float percent)
        {
            if (percent == float.MinValue)
                return "-";
            if (percent == float.MaxValue)
                return "-";

            return string.Format(CultureInfo.InvariantCulture, "{0:+0.##;-0.##;0}%", percent);
        }

        string DiffPercentString(ComparisonTreeViewItem item)
        {
            return PercentString(DiffPercent(item));
        }

        string CountDiffPercentString(ComparisonTreeViewItem item)
        {
            return PercentString(CountDiffPercent(item));
        }

        string CountMeanDiffPercentString(ComparisonTreeViewItem item)
        {
            return PercentString(CountMeanDiffPercent(item));
        }

        string TotalDiffPercentString(ComparisonTreeViewItem item)
        {
            return PercentString(TotalDiffPercent(item));
        }

        void GenerateStrings(ComparisonTreeViewItem item)
        {
            item.cachedRowString = new GUIContent[m_MaxColumns];

            int leftMedianFrameIndex = m_ProfileAnalyzerWindow.GetRemappedUIFrameIndex(LeftMedianFrameIndex(item), m_LeftDataView);
            int rightMedianFrameIndex = m_ProfileAnalyzerWindow.GetRemappedUIFrameIndex(RightMedianFrameIndex(item), m_RightDataView);

            if (TimeRemoved(item) > 0.0)
            {
                item.cachedRowString[(int)MyColumns.Name] = new GUIContent(item.data.name + " [Modified]", item.data.name + "\n\nTime reduced by removing child marker time");
                item.cachedRowString[(int)MyColumns.State] = new GUIContent("Modified", "Time reduced by removing child marker time");
            }
            else if (TimeIgnored(item) > 0.0)
            {
                double timeIgnored;
                double msTotal;
                bool removed = IsFullyIgnored(item, out timeIgnored, out msTotal);

                if (removed)
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
            item.cachedRowString[(int)MyColumns.LeftMedian] = item.data.leftIndex < 0 ? Styles.invalidEntry : ToDisplayUnitsWithTooltips(LeftMedian(item), false, leftMedianFrameIndex);
            item.cachedRowString[(int)MyColumns.RightMedian] = item.data.rightIndex < 0 ? Styles.invalidEntry : ToDisplayUnitsWithTooltips(RightMedian(item), false, rightMedianFrameIndex);
            string tooltip = ToTooltipDisplayUnits(Diff(item), true);
            item.cachedRowString[(int)MyColumns.LeftBar] = Diff(item) < 0 ? new GUIContent("", tooltip) : new GUIContent("", "");
            item.cachedRowString[(int)MyColumns.RightBar] = Diff(item) > 0 ? new GUIContent("", tooltip) : new GUIContent("", "");
            item.cachedRowString[(int)MyColumns.Diff] = ToDisplayUnitsWithTooltips(Diff(item));
            item.cachedRowString[(int)MyColumns.DiffPercent] = new GUIContent(DiffPercentString(item), "");
            item.cachedRowString[(int)MyColumns.AbsDiff] = ToDisplayUnitsWithTooltips(AbsDiff(item));

            item.cachedRowString[(int)MyColumns.LeftCount] = item.data.leftIndex < 0 ? Styles.invalidEntry : new GUIContent(string.Format("{0}", LeftCount(item)));
            item.cachedRowString[(int)MyColumns.RightCount] = item.data.rightIndex < 0 ? Styles.invalidEntry : new GUIContent(string.Format("{0}", RightCount(item)));
            tooltip = string.Format("{0}", CountDiff(item));
            item.cachedRowString[(int)MyColumns.LeftCountBar] = CountDiff(item) < 0 ? new GUIContent("", tooltip) : new GUIContent("", "");
            item.cachedRowString[(int)MyColumns.RightCountBar] = CountDiff(item) > 0 ? new GUIContent("", tooltip) : new GUIContent("", "");
            item.cachedRowString[(int)MyColumns.CountDiff] = (item.data.leftIndex < 0 && item.data.rightIndex < 0) ? Styles.invalidEntry : new GUIContent(string.Format("{0}", CountDiff(item)), CountDiffPercentString(item));
            item.cachedRowString[(int)MyColumns.CountDiffPercent] = (item.data.leftIndex < 0 && item.data.rightIndex < 0) ? Styles.invalidEntry : new GUIContent(CountDiffPercentString(item), "");
            item.cachedRowString[(int)MyColumns.AbsCountDiff] = (item.data.leftIndex < 0 && item.data.rightIndex < 0) ? Styles.invalidEntry : new GUIContent(string.Format("{0}", AbsCountDiff(item)));

            item.cachedRowString[(int)MyColumns.LeftCountMean] = item.data.leftIndex < 0 ? Styles.invalidEntry : new GUIContent(string.Format(CultureInfo.InvariantCulture, "{0:f0}", LeftCountMean(item)));
            item.cachedRowString[(int)MyColumns.RightCountMean] = item.data.rightIndex < 0 ? Styles.invalidEntry : new GUIContent(string.Format(CultureInfo.InvariantCulture, "{0:f0}", RightCountMean(item)));
            tooltip = string.Format("{0}", CountMeanDiff(item));
            item.cachedRowString[(int)MyColumns.LeftCountMeanBar] = CountMeanDiff(item) < 0 ? new GUIContent("", tooltip) : new GUIContent("", "");
            item.cachedRowString[(int)MyColumns.RightCountMeanBar] = CountMeanDiff(item) > 0 ? new GUIContent("", tooltip) : new GUIContent("", "");
            item.cachedRowString[(int)MyColumns.CountMeanDiff] = (item.data.leftIndex < 0 && item.data.rightIndex < 0) ? Styles.invalidEntry : new GUIContent(string.Format("{0:f0}", CountMeanDiff(item)), CountMeanDiffPercentString(item));
            item.cachedRowString[(int)MyColumns.CountMeanDiffPercent] = (item.data.leftIndex < 0 && item.data.rightIndex < 0) ? Styles.invalidEntry : new GUIContent(CountMeanDiffPercentString(item), "");
            item.cachedRowString[(int)MyColumns.AbsCountMeanDiff] = (item.data.leftIndex < 0 && item.data.rightIndex < 0) ? Styles.invalidEntry : new GUIContent(string.Format("{0:f0}", AbsCountMeanDiff(item)));

            item.cachedRowString[(int)MyColumns.LeftTotal] = item.data.leftIndex < 0 ? Styles.invalidEntry : ToDisplayUnitsWithTooltips(LeftTotal(item));
            item.cachedRowString[(int)MyColumns.RightTotal] = item.data.rightIndex < 0 ? Styles.invalidEntry : ToDisplayUnitsWithTooltips(RightTotal(item));
            tooltip = ToTooltipDisplayUnits(TotalDiff(item), true);
            item.cachedRowString[(int)MyColumns.LeftTotalBar] = TotalDiff(item) < 0 ? new GUIContent("", tooltip) : new GUIContent("", "");
            item.cachedRowString[(int)MyColumns.RightTotalBar] = TotalDiff(item) > 0 ? new GUIContent("", tooltip) : new GUIContent("", "");
            item.cachedRowString[(int)MyColumns.TotalDiff] = (item.data.leftIndex < 0 && item.data.rightIndex < 0) ? Styles.invalidEntry : ToDisplayUnitsWithTooltips(TotalDiff(item));
            item.cachedRowString[(int)MyColumns.TotalDiffPercent] = (item.data.leftIndex < 0 && item.data.rightIndex < 0) ? Styles.invalidEntry : new GUIContent(TotalDiffPercentString(item), "");
            item.cachedRowString[(int)MyColumns.AbsTotalDiff] = (item.data.leftIndex < 0 && item.data.rightIndex < 0) ? Styles.invalidEntry : ToDisplayUnitsWithTooltips(AbsTotalDiff(item));

            item.cachedRowString[(int)MyColumns.LeftDepth] = item.data.leftIndex < 0 ? Styles.invalidEntry : (LeftMinDepth(item) == LeftMaxDepth(item)) ? new GUIContent(string.Format("{0}", LeftMinDepth(item)), "") : new GUIContent(string.Format("{0}-{1}", LeftMinDepth(item), LeftMaxDepth(item)), "");
            item.cachedRowString[(int)MyColumns.RightDepth] = item.data.rightIndex < 0 ? Styles.invalidEntry : (RightMinDepth(item) == RightMaxDepth(item)) ? new GUIContent(string.Format("{0}", RightMinDepth(item)), "") : new GUIContent(string.Format("{0}-{1}", RightMinDepth(item), RightMaxDepth(item)), "");
            item.cachedRowString[(int)MyColumns.DepthDiff] = (item.data.leftIndex < 0 || item.data.rightIndex < 0) ? Styles.invalidEntry : new GUIContent(string.Format("{0}", DepthDiff(item)));

            item.cachedRowString[(int)MyColumns.LeftThreads] = item.data.leftIndex < 0 ? Styles.invalidEntry : new GUIContent(string.Format("{0}", LeftThreads(item)), LeftThreads(item));
            item.cachedRowString[(int)MyColumns.RightThreads] = item.data.rightIndex < 0 ? Styles.invalidEntry : new GUIContent(string.Format("{0}", RightThreads(item)), RightThreads(item));
        }

        void ShowBar(Rect rect, float ms, float range, GUIContent content, Color color, bool rightAlign)
        {
            if (ms > 0.0f)
            {
                if (m_2D.DrawStart(rect))
                {
                    float w = Math.Max(1.0f, rect.width * ms / range);
                    float x = rightAlign ? rect.width - w : 0.0f;
                    m_2D.DrawFilledBox(x, 1, w, rect.height - 1, color);
                    m_2D.DrawEnd();
                }
            }
            GUI.Label(rect, content);
        }

        void CellGUI(Rect cellRect, ComparisonTreeViewItem item, MyColumns column, ref RowGUIArgs args)
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
                    ShowText(cellRect, content);
                }
                break;
                case MyColumns.State:
                case MyColumns.LeftMedian:
                case MyColumns.Diff:
                case MyColumns.RightMedian:
                case MyColumns.DiffPercent:
                case MyColumns.AbsDiff:
                case MyColumns.LeftCount:
                case MyColumns.RightCount:
                case MyColumns.CountDiff:
                case MyColumns.CountDiffPercent:
                case MyColumns.AbsCountDiff:
                case MyColumns.LeftCountMean:
                case MyColumns.RightCountMean:
                case MyColumns.CountMeanDiff:
                case MyColumns.CountMeanDiffPercent:
                case MyColumns.AbsCountMeanDiff:
                case MyColumns.LeftTotal:
                case MyColumns.RightTotal:
                case MyColumns.TotalDiff:
                case MyColumns.TotalDiffPercent:
                case MyColumns.AbsTotalDiff:
                case MyColumns.LeftDepth:
                case MyColumns.RightDepth:
                case MyColumns.LeftThreads:
                case MyColumns.RightThreads:
                case MyColumns.DepthDiff:
                    ShowText(cellRect, content);
                    break;
                case MyColumns.LeftBar:
                    ShowBar(cellRect, -Diff(item), m_DiffRange, content, m_LeftColor, true);
                    break;
                case MyColumns.RightBar:
                    ShowBar(cellRect, Diff(item), m_DiffRange, content, m_RightColor, false);
                    break;
                case MyColumns.LeftCountBar:
                    ShowBar(cellRect, -CountDiff(item), m_CountDiffRange, content, m_LeftColor, true);
                    break;
                case MyColumns.RightCountBar:
                    ShowBar(cellRect, CountDiff(item), m_CountDiffRange, content, m_RightColor, false);
                    break;
                case MyColumns.LeftCountMeanBar:
                    ShowBar(cellRect, -CountMeanDiff(item), m_CountMeanDiffRange, content, m_LeftColor, true);
                    break;
                case MyColumns.RightCountMeanBar:
                    ShowBar(cellRect, CountMeanDiff(item), m_CountMeanDiffRange, content, m_RightColor, false);
                    break;
                case MyColumns.LeftTotalBar:
                    ShowBar(cellRect, (float)-TotalDiff(item), (float)m_TotalDiffRange, content, m_LeftColor, true);
                    break;
                case MyColumns.RightTotalBar:
                    ShowBar(cellRect, (float)TotalDiff(item), (float)m_TotalDiffRange, content, m_RightColor, false);
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

        static HeaderData[] s_HeaderData = new HeaderData[]
            {
                new HeaderData("Marker Name", "Marker Name\n\nFrame marker time is total of all instances in frame", width: 300, minWidth: 100, autoResize: false, allowToggleVisibility: false, ascending: true),
                new HeaderData("State", "Status of marker entry (if modified or removed from frame time due to 'Remove' filter)"),

                new HeaderData("Left Median", "Left median time\n\nCentral marker time over all selected frames"),
                new HeaderData("<", "Difference if left data set is a larger value", width: 50),
                new HeaderData(">", "Difference if right data set is a larger value", width: 50),
                new HeaderData("Right Median", "Right median time\n\nCentral marker time over all selected frames"),
                new HeaderData("Diff", "Difference between left and right times"),
                new HeaderData("Diff Percent", "Difference between left and right times as percentage of left marker time"),
                new HeaderData("Abs Diff", "Absolute difference between left and right times"),

                new HeaderData("Count Left", "Left marker count over all selected frames\n\nMultiple can occur per frame"),
                new HeaderData("< Count", "Count Difference if left data set count is a larger value", width: 50),
                new HeaderData("> Count", "Count Difference if right data set count is a larger value", width: 50),
                new HeaderData("Count Right", "Right marker count over all selected frames\n\nMultiple can occur per frame"),
                new HeaderData("Count Delta", "Difference in marker count"),
                new HeaderData("Count Delta Percent", "Difference in marker count as percentage of left marker count"),
                new HeaderData("Abs Count", "Absolute difference in marker count"),

                new HeaderData("Count Left Frame", "Average number of markers per frame in left data set\n\ntotal count / number of non zero frames"),
                new HeaderData("< Frame Count", "Per frame Count Difference if left data set count is a larger value", width: 50),
                new HeaderData("> Frame Count", "Per frame Count Difference if right data set count is a larger value", width: 50),
                new HeaderData("Count Right Frame", "Average number of markers per frame in right data set\n\ntotal count / number of non zero frames"),
                new HeaderData("Count Delta Frame", "Difference in per frame marker count"),
                new HeaderData("Count Delta Percent Frame", "Difference in per frame marker count as percentage of left count"),
                new HeaderData("Abs Frame Count", "Absolute difference in per frame marker count"),

                new HeaderData("Total Left", "Left marker total time over all selected frames"),
                new HeaderData("< Total", "Total Difference if left data set total is a larger value", width: 50),
                new HeaderData("> Total", "Total Difference if right data set total is a larger value", width: 50),
                new HeaderData("Total Right", "Right marker total time over all selected frames"),
                new HeaderData("Total Delta", "Difference in total time over all selected frames"),
                new HeaderData("Total Delta Percent", "Difference in total time over all selected frames as percentage of left marker total time"),
                new HeaderData("Abs Total", "Absolute difference in total time over all selected frames"),

                new HeaderData("Depth Left", "Marker depth in marker hierarchy\n\nMay appear at multiple levels"),
                new HeaderData("Depth Right", "Marker depth in marker hierarchy\n\nMay appear at multiple levels"),
                new HeaderData("Depth Diff", "Absolute difference in min marker depth total over all selected frames"),

                new HeaderData("Threads Left", "Threads the marker occurs on in left data set (with filtering applied)"),
                new HeaderData("Threads Right", "Threads the marker occurs on in right data set (with filtering applied)"),
            };
        public static MultiColumnHeaderState CreateDefaultMultiColumnHeaderState(MarkerColumnFilter modeFilter)
        {
            var columnList = new List<MultiColumnHeaderState.Column>();
            foreach (var header in s_HeaderData)
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
                m_ProfileAnalyzerWindow.SelectPairing(selectedIds[0]);
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
                        (int)MyColumns.LeftMedian,
                        (int)MyColumns.LeftBar,
                        (int)MyColumns.RightBar,
                        (int)MyColumns.RightMedian,
                        (int)MyColumns.Diff,
                        (int)MyColumns.AbsDiff,
                        (int)MyColumns.LeftCount,
                        (int)MyColumns.RightCount,
                        (int)MyColumns.CountDiff,
                    };
                    break;
                case MarkerColumnFilter.Mode.Time:
                    visibleColumns = new int[]
                    {
                        (int)MyColumns.Name,
                        (int)MyColumns.LeftMedian,
                        (int)MyColumns.LeftBar,
                        (int)MyColumns.RightBar,
                        (int)MyColumns.RightMedian,
                        (int)MyColumns.Diff,
                        (int)MyColumns.AbsDiff,
                    };
                    break;
                case MarkerColumnFilter.Mode.Totals:
                    visibleColumns = new int[]
                    {
                        (int)MyColumns.Name,
                        (int)MyColumns.LeftTotal,
                        (int)MyColumns.LeftTotalBar,
                        (int)MyColumns.RightTotalBar,
                        (int)MyColumns.RightTotal,
                        (int)MyColumns.TotalDiff,
                        (int)MyColumns.AbsTotalDiff,
                    };
                    break;
                case MarkerColumnFilter.Mode.TimeWithTotals:
                    visibleColumns = new int[]
                    {
                        (int)MyColumns.Name,
                        (int)MyColumns.LeftMedian,
                        (int)MyColumns.LeftBar,
                        (int)MyColumns.RightBar,
                        (int)MyColumns.RightMedian,
                        (int)MyColumns.AbsDiff,
                        (int)MyColumns.LeftTotal,
                        (int)MyColumns.LeftTotalBar,
                        (int)MyColumns.RightTotalBar,
                        (int)MyColumns.RightTotal,
                        (int)MyColumns.AbsTotalDiff,
                    };
                    break;
                case MarkerColumnFilter.Mode.CountTotals:
                    visibleColumns = new int[]
                    {
                        (int)MyColumns.Name,
                        (int)MyColumns.LeftCount,
                        (int)MyColumns.LeftCountBar,
                        (int)MyColumns.RightCountBar,
                        (int)MyColumns.RightCount,
                        (int)MyColumns.CountDiff,
                        (int)MyColumns.AbsCountDiff,
                    };
                    break;
                case MarkerColumnFilter.Mode.CountPerFrame:
                    visibleColumns = new int[]
                    {
                        (int)MyColumns.Name,
                        (int)MyColumns.LeftCountMean,
                        (int)MyColumns.LeftCountMeanBar,
                        (int)MyColumns.RightCountMeanBar,
                        (int)MyColumns.RightCountMean,
                        (int)MyColumns.CountMeanDiff,
                        (int)MyColumns.AbsCountMeanDiff,
                    };
                    break;
                case MarkerColumnFilter.Mode.Depth:
                    visibleColumns = new int[]
                    {
                        (int)MyColumns.Name,
                        (int)MyColumns.LeftDepth,
                        (int)MyColumns.RightDepth,
                        (int)MyColumns.DepthDiff,
                    };
                    break;
                case MarkerColumnFilter.Mode.Threads:
                    visibleColumns = new int[]
                    {
                        (int)MyColumns.Name,
                        (int)MyColumns.LeftThreads,
                        (int)MyColumns.RightThreads,
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
}
