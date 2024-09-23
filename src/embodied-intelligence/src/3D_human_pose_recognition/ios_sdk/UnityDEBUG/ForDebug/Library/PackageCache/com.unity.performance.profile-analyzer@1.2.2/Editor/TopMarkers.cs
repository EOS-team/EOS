using System;
using System.Text;
using System.Collections.Generic;
using UnityEngine;
using UnityEngine.Profiling;

namespace UnityEditor.Performance.ProfileAnalyzer
{
    internal class TopMarkers
    {
        internal class RangeSettings
        {
            public readonly ProfileDataView dataView;

            public readonly int depthFilter;
            public readonly List<string> nameFilters;
            public readonly List<string> nameExcludes;
            public readonly TimingOptions.TimingOption timingOption;
            public readonly int threadSelectionCount;
            public readonly bool hideRemovedMarkers;

            public RangeSettings(ProfileDataView dataView, int depthFilter, List<string> nameFilters, List<string> nameExcludes, TimingOptions.TimingOption timingOption, int threadSelectionCount, bool hideRemovedMarkers)
            {
                // Make a copy rather than keeping a reference
                this.dataView = dataView==null ? new ProfileDataView() : new ProfileDataView(dataView);
                this.depthFilter = depthFilter;
                this.nameFilters = nameFilters;
                this.nameExcludes = nameExcludes;
                this.timingOption = timingOption;
                this.threadSelectionCount = threadSelectionCount;
                this.hideRemovedMarkers = hideRemovedMarkers;
            }

            public override int GetHashCode()
            {
                int hash = 13;
                hash = (hash * 7) + dataView.GetHashCode();
                hash = (hash * 7) + depthFilter.GetHashCode();
                hash = (hash * 7) + nameFilters.GetHashCode();
                hash = (hash * 7) + nameExcludes.GetHashCode();
                hash = (hash * 7) + timingOption.GetHashCode();
                hash = (hash * 7) + threadSelectionCount.GetHashCode();
                hash = (hash * 7) + hideRemovedMarkers.GetHashCode();

                return hash;
            }

            public override bool Equals(object b)
            {
                if (System.Object.ReferenceEquals(null, b))
                {
                    return false;
                }

                if (System.Object.ReferenceEquals(this, b))
                {
                    return true;
                }

                if (b.GetType() != this.GetType())
                {
                    return false;
                }

                return IsEqual((RangeSettings)b);
            }

            bool IsEqual(RangeSettings b)
            {
                if (timingOption != b.timingOption)
                    return false;

                if (b.dataView == null && dataView != null)
                    return false;

                // Check contents of data view (the reference will definitly not match as we made a copy)
                if (b.dataView != null)
                {
                    // Only need to check data, analysis and selectedIndices
                    if (dataView.data != b.dataView.data)
                        return false;
                    if (dataView.analysis != b.dataView.analysis)
                        return false;

                    if (dataView.selectedIndices.Count != b.dataView.selectedIndices.Count)
                        return false;

                    // Want to check if contents match, not just if reference is the same
                    for (int i = 0; i < dataView.selectedIndices.Count; i++)
                    {
                        if (dataView.selectedIndices[i] != b.dataView.selectedIndices[i])
                            return false;
                    }
                }

                if (depthFilter != b.depthFilter)
                    return false;
                if (threadSelectionCount != b.threadSelectionCount)
                    return false;

                if (hideRemovedMarkers != b.hideRemovedMarkers)
                    return false;

                if (nameFilters.Count != b.nameFilters.Count)
                    return false;
                if (nameExcludes.Count != b.nameExcludes.Count)
                    return false;

                // Want to check if contents match, not just if reference is the same
                for (int i = 0; i < nameFilters.Count; i++)
                {
                    if (nameFilters[i] != b.nameFilters[i])
                        return false;
                }
                for (int i = 0; i < nameExcludes.Count; i++)
                {
                    if (nameExcludes[i] != b.nameExcludes[i])
                        return false;
                }

                return true;
            }

            public static bool operator==(RangeSettings a, RangeSettings b)
            {
                // If both are null, or both are same instance, return true.
                if (System.Object.ReferenceEquals(a, b))
                {
                    return true;
                }

                // If one is null, but not both, return false.
                if (((object)a == null) || ((object)b == null))
                {
                    return false;
                }

                return a.IsEqual(b);
            }

            public static bool operator!=(RangeSettings a, RangeSettings b)
            {
                return !(a == b);
            }
        }

        internal class Settings
        {   
            public readonly int barCount;
            public readonly float timeRange;

            public readonly bool includeOthers;
            public readonly bool includeUnaccounted;

            public RangeSettings rangeSettings { get ; private set ; }

            public Settings(RangeSettings rangeSettings, int barCount, float timeRange, bool includeOthers, bool includeUnaccounted)
            {
                this.rangeSettings = rangeSettings;
                this.barCount = barCount;
                this.timeRange = timeRange;
                this.includeOthers = includeOthers;
                this.includeUnaccounted = includeUnaccounted;
            }

            public override int GetHashCode()
            {
                int hash = 13;
                hash = (hash * 7) + rangeSettings.GetHashCode();
                hash = (hash * 7) + barCount.GetHashCode();
                hash = (hash * 7) + timeRange.GetHashCode();
                hash = (hash * 7) + includeOthers.GetHashCode();
                hash = (hash * 7) + includeUnaccounted.GetHashCode();

                return hash;
            }

            public override bool Equals(object b)
            {
                if (System.Object.ReferenceEquals(null, b))
                {
                    return false;
                }

                if (System.Object.ReferenceEquals(this, b))
                {
                    return true;
                }

                if (b.GetType() != this.GetType())
                {
                    return false;
                }

                return IsEqual((Settings)b);
            }

            bool IsEqual(Settings b)
            {
                if (rangeSettings != b.rangeSettings)
                    return false;
                if (barCount != b.barCount)
                    return false;
                if (timeRange != b.timeRange)
                    return false;
                if (includeOthers != b.includeOthers)
                    return false;
                if (includeUnaccounted != b.includeUnaccounted)
                    return false;

                return true;
            }

            public static bool operator==(Settings a, Settings b)
            {
                // If both are null, or both are same instance, return true.
                if (System.Object.ReferenceEquals(a, b))
                {
                    return true;
                }

                // If one is null, but not both, return false.
                if (((object)a == null) || ((object)b == null))
                {
                    return false;
                }

                return a.IsEqual(b);
            }

            public static bool operator!=(Settings a, Settings b)
            {
                return !(a == b);
            }
        }

        internal enum SummaryType
        {
            Marker,
            Other,
            Unaccounted
        }

        internal struct MarkerSummaryEntry
        {
            public readonly string name;
            public readonly float msAtMedian;   // At the median frame (Miliseconds)
            public readonly float msMedian;     // median value for marker over all frames (Miliseconds) on frame medianFrameIndex
            public readonly float x;
            public readonly float w;
            public readonly int medianFrameIndex;
            public readonly SummaryType summaryType;

            public MarkerSummaryEntry(string name, float msAtMedian, float msMedian, float x, float w, int medianFrameIndex, SummaryType summaryType)
            {
                this.name = name;
                this.msAtMedian = msAtMedian;
                this.msMedian = msMedian;
                this.x = x;
                this.w = w;
                this.medianFrameIndex = medianFrameIndex;
                this.summaryType = summaryType;
            }
        }

        internal class MarkerSummary
        {
            public List<MarkerSummaryEntry> entry;

            public float totalTime;

            public MarkerSummary()
            {
                entry = new List<MarkerSummaryEntry>();
                totalTime = 0f;
            }
        }

        Settings m_CurrentSettings;             // Current settings, including latest RangeSettings
        RangeSettings m_RequestedRangeSettings; // Next requested range setting set by SetData

        float m_TimeRange;
        bool m_TimeRangeDirty;                  // Set when renquested range settings change
        MarkerSummary m_MarkerSummary;

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

        private static class Content
        {
            public static readonly string frameTime = L10n.Tr("{0} Frame time on median frame");
            public static readonly string multipleThreads = L10n.Tr("Multiple threads selected\nSelect a single thread for an overview");
            public static readonly string totalTimeAllDepths = L10n.Tr("{0} Total time of all markers at all depths");
            public static readonly string totalTimeAtSpecificDepth = L10n.Tr("{0} Total time of all markers at Depth {1}");
            public static readonly string selectSelf = L10n.Tr("For an overview select Analysis Type Self");
            public static readonly string selectTotal = L10n.Tr("To include child times select Analysis Type Total");
            public static readonly string selfTimeAllDepths = L10n.Tr("{0} Self time of markers at all depths");
            public static readonly string selfTimeAtSpecificDepth = L10n.Tr("{0} Self time of markers at Depth {1}");

            public static readonly string tooltip = L10n.Tr("{0}\n{1:f2}% ({2} on median frame {3})\n\nMedian marker time (in currently selected frames)\n{4} on frame {5}");
        }

        ProfileAnalyzerWindow m_ProfileAnalyzerWindow;
        Draw2D m_2D;
        Color m_BackgroundColor;
        Color m_TextColor;

        public TopMarkers(ProfileAnalyzerWindow profileAnalyzerWindow, Draw2D draw2D, Color backgroundColor, Color textColor)
        {
            m_ProfileAnalyzerWindow = profileAnalyzerWindow;
            m_2D = draw2D;
            m_BackgroundColor = backgroundColor;
            m_TextColor = textColor;

            m_CurrentSettings = new Settings(new RangeSettings(null, 0, null, null, TimingOptions.TimingOption.Time, 0, false), 0, 0, false, false);
            m_TimeRangeDirty = true;
        }

        string ToDisplayUnits(float ms, bool showUnits = false, int limitToDigits = 5)
        {
            return m_ProfileAnalyzerWindow.ToDisplayUnits(ms, showUnits, limitToDigits);
        }

        public void SetData(ProfileDataView dataView, int depthFilter, List<string> nameFilters, List<string> nameExcludes, TimingOptions.TimingOption timingOption, int threadSelectionCount, bool hideRemovedMarkers)
        {
            m_RequestedRangeSettings = new RangeSettings(dataView, depthFilter, nameFilters, nameExcludes, timingOption, threadSelectionCount, hideRemovedMarkers);
            if (m_CurrentSettings.rangeSettings != m_RequestedRangeSettings)
                m_TimeRangeDirty = true;
        }

        float CalculateTopMarkerTimeRange(RangeSettings rangeSettings)
        {
            if (rangeSettings == null)
                return 0.0f;
            if (rangeSettings.dataView == null)
                return 0.0f;

            ProfileAnalysis analysis = rangeSettings.dataView.analysis;
            if (analysis == null)
                return 0.0f;

            var frameSummary = analysis.GetFrameSummary();
            if (frameSummary == null)
                return 0.0f;

            int depthFilter = rangeSettings.depthFilter;
            List<string> nameFilters = rangeSettings.nameFilters;
            List<string> nameExcludes = rangeSettings.nameExcludes;
            bool hideRemovedMarkers = rangeSettings.hideRemovedMarkers;

            var markers = analysis.GetMarkers();

            float range = 0;
            foreach (var marker in markers)
            {
                if (depthFilter != ProfileAnalyzer.kDepthAll && marker.minDepth != depthFilter)
                {
                    continue;
                }

                if (nameFilters.Count > 0)
                {
                    if (!m_ProfileAnalyzerWindow.NameInIncludeList(marker.name, nameFilters))
                        continue;
                }
                if (nameExcludes.Count > 0)
                {
                    if (m_ProfileAnalyzerWindow.NameInExcludeList(marker.name, nameExcludes))
                        continue;
                }

                if (hideRemovedMarkers && marker.IsFullyIgnored())
                {
                    continue;
                }

                range += marker.msAtMedian;
            }

            // Minimum is the frame time range
            // As we can have unaccounted markers
            if (range < frameSummary.msMedian)
                range = frameSummary.msMedian;

            return range;
        }

        public float GetTopMarkerTimeRange()
        {
            if (m_TimeRangeDirty)
            {
                Profiler.BeginSample("CalculateTopMarkerTimeRange");

                // Use latest requested rather than current (as current may not yet be updated)
                m_TimeRange = CalculateTopMarkerTimeRange(m_RequestedRangeSettings);
                m_TimeRangeDirty = false;

                Profiler.EndSample();
            }

            return m_TimeRange;
        }

        public MarkerSummary CalculateTopMarkers()
        {
            if (m_CurrentSettings.rangeSettings.dataView == null)
                return null;

            ProfileAnalysis analysis = m_CurrentSettings.rangeSettings.dataView.analysis;
            if (analysis == null)
                return null;

            FrameSummary frameSummary = analysis.GetFrameSummary();
            if (frameSummary == null)
                return new MarkerSummary();

            var markers = analysis.GetMarkers();
            if (markers == null)
                return new MarkerSummary();

            float timeRange = m_CurrentSettings.timeRange;
            int depthFilter = m_CurrentSettings.rangeSettings.depthFilter;
            List<string> nameFilters = m_CurrentSettings.rangeSettings.nameFilters;
            List<string> nameExcludes = m_CurrentSettings.rangeSettings.nameExcludes;
            bool hideRemovedMarkers = m_CurrentSettings.rangeSettings.hideRemovedMarkers;

            // Show marker graph
            float x = 0;
            float width = 1.0f;

            int max = m_CurrentSettings.barCount;
            int at = 0;

            float other = 0.0f;

            if (timeRange <= 0.0f)
                timeRange = frameSummary.msMedian;

            float msToWidth = width / timeRange;

            float totalMarkerTime = 0;

            MarkerSummary markerSummary = new MarkerSummary();

            foreach (var marker in markers)
            {
                float msAtMedian = MarkerData.GetMsAtMedian(marker);

                // We do this at the top so that totalMarkerTime is not increased
                // This excludes the hidden markers time
                if (hideRemovedMarkers && marker.IsFullyIgnored())
                {
                    continue;
                }

                if (depthFilter != ProfileAnalyzer.kDepthAll && marker.minDepth != depthFilter)
                {
                    continue;
                }

                if (nameFilters.Count > 0)
                {
                    if (!m_ProfileAnalyzerWindow.NameInIncludeList(marker.name, nameFilters))
                        continue;
                }
                if (nameExcludes.Count > 0)
                {
                    if (m_ProfileAnalyzerWindow.NameInExcludeList(marker.name, nameExcludes))
                        continue;
                }

                totalMarkerTime += msAtMedian;

                if (at < max)
                {
                    float w = CaculateWidth(x, msAtMedian, msToWidth, width);
                    float msMedian = MarkerData.GetMsMedian(marker);
                    int medianFrameIndex = m_ProfileAnalyzerWindow.GetRemappedUIFrameIndex(marker.medianFrameIndex, m_CurrentSettings.rangeSettings.dataView);
                    markerSummary.entry.Add(new MarkerSummaryEntry(marker.name, msAtMedian, msMedian, x, w, medianFrameIndex, SummaryType.Marker));

                    x += w;
                }
                else
                {
                    other += msAtMedian;
                    if (!m_CurrentSettings.includeOthers)
                        break;
                }

                at++;
            }

            if (m_CurrentSettings.includeOthers && other > 0.0f)
            {
                float w = CaculateWidth(x, other, msToWidth, width);
                markerSummary.entry.Add(new MarkerSummaryEntry("Other", other, 0f, x, w, -1, SummaryType.Other));
                x += w;
            }
            if (m_CurrentSettings.includeUnaccounted && totalMarkerTime < frameSummary.msMedian)
            {
                float unaccounted = frameSummary.msMedian - totalMarkerTime;
                float w = CaculateWidth(x, unaccounted, msToWidth, width);
                markerSummary.entry.Add(new MarkerSummaryEntry("Unaccounted", unaccounted, 0f, x, w, -1, SummaryType.Unaccounted));
                x += w;
            }

            markerSummary.totalTime = totalMarkerTime;

            return markerSummary;
        }

        GenericMenu GenerateActiveContextMenu(string markerName, Event evt)
        {
            GenericMenu menu = new GenericMenu();
            menu.AddItem(Styles.menuItemSelectFramesInAll, false, () => m_ProfileAnalyzerWindow.SelectFramesContainingMarker(markerName, false));
            menu.AddItem(Styles.menuItemSelectFramesInCurrent, false, () => m_ProfileAnalyzerWindow.SelectFramesContainingMarker(markerName, true));

            if (m_ProfileAnalyzerWindow.AllSelected())
                menu.AddDisabledItem(Styles.menuItemSelectFramesAll);
            else
                menu.AddItem(Styles.menuItemSelectFramesAll, false, () => m_ProfileAnalyzerWindow.SelectAllFrames());

            menu.AddSeparator("");
            if (!m_CurrentSettings.rangeSettings.nameFilters.Contains(markerName))
                menu.AddItem(Styles.menuItemAddToIncludeFilter, false, () => m_ProfileAnalyzerWindow.AddToIncludeFilter(markerName));
            else
                menu.AddItem(Styles.menuItemRemoveFromIncludeFilter, false, () => m_ProfileAnalyzerWindow.RemoveFromIncludeFilter(markerName));
            if (!m_CurrentSettings.rangeSettings.nameExcludes.Contains(markerName))
                menu.AddItem(Styles.menuItemAddToExcludeFilter, false, () => m_ProfileAnalyzerWindow.AddToExcludeFilter(markerName));
            else
                menu.AddItem(Styles.menuItemRemoveFromExcludeFilter, false, () => m_ProfileAnalyzerWindow.RemoveFromExcludeFilter(markerName));
            menu.AddSeparator("");
            menu.AddItem(Styles.menuItemSetAsParentMarkerFilter, false, () => m_ProfileAnalyzerWindow.SetAsParentMarkerFilter(markerName));
            menu.AddItem(Styles.menuItemClearParentMarkerFilter, false, () => m_ProfileAnalyzerWindow.SetAsParentMarkerFilter(""));
            menu.AddSeparator("");
            menu.AddItem(Styles.menuItemSetAsRemoveMarker, false, () => m_ProfileAnalyzerWindow.SetAsRemoveMarker(markerName));
            menu.AddSeparator("");
            menu.AddItem(Styles.menuItemCopyToClipboard, false, () => CopyToClipboard(evt, markerName));

            return menu;
        }

        GenericMenu GenerateDisabledContextMenu(string markerName)
        {
            GenericMenu menu = new GenericMenu();
            menu.AddDisabledItem(Styles.menuItemSelectFramesInAll);
            menu.AddDisabledItem(Styles.menuItemSelectFramesInCurrent);
            menu.AddDisabledItem(Styles.menuItemSelectFramesAll);

            menu.AddSeparator("");
            if (!m_CurrentSettings.rangeSettings.nameFilters.Contains(markerName))
                menu.AddDisabledItem(Styles.menuItemAddToIncludeFilter);
            else
                menu.AddDisabledItem(Styles.menuItemRemoveFromIncludeFilter);
            if (!m_CurrentSettings.rangeSettings.nameExcludes.Contains(markerName))
                menu.AddDisabledItem(Styles.menuItemAddToExcludeFilter);
            else
                menu.AddDisabledItem(Styles.menuItemRemoveFromExcludeFilter);

            menu.AddSeparator("");
            menu.AddDisabledItem(Styles.menuItemSetAsParentMarkerFilter);
            menu.AddDisabledItem(Styles.menuItemClearParentMarkerFilter);
            menu.AddSeparator("");
            menu.AddDisabledItem(Styles.menuItemSetAsRemoveMarker);
            menu.AddSeparator("");
            menu.AddDisabledItem(Styles.menuItemCopyToClipboard);

            return menu;
        }
        
        public GUIContent ConstructTimeRangeText()
        {
            StringBuilder sb = new StringBuilder();

            ProfileAnalysis analysis = m_CurrentSettings.rangeSettings.dataView.analysis;
            int depthFilter = m_CurrentSettings.rangeSettings.depthFilter;
            
            FrameSummary frameSummary = analysis.GetFrameSummary();

            string frameTimeString = ToDisplayUnits(frameSummary.msMedian, true, 0);
            string accountedTimeString = ToDisplayUnits(m_MarkerSummary.totalTime, true, 0);
            sb.AppendFormat(Content.frameTime, frameTimeString);

            // Note m_CurrentSettings.rangeSettings.dataView.analysis.GetThreads contains all thread names, not just the filtered threads
            bool singleThread = m_CurrentSettings.rangeSettings.threadSelectionCount == 1;

            if (depthFilter == ProfileAnalyzer.kDepthAll)
            {
                if (m_CurrentSettings.rangeSettings.timingOption == TimingOptions.TimingOption.Time)
                {
                    sb.Append("\n");
                    sb.AppendFormat(Content.totalTimeAllDepths, accountedTimeString);
                    if (singleThread)
                    {
                        sb.Append("\n\n");
                        sb.Append(Content.selectSelf);
                    }
                }
                else
                {
                    sb.Append("\n");
                    sb.AppendFormat(Content.selfTimeAllDepths,accountedTimeString);
                }
            }
            else 
            {
                if (m_CurrentSettings.rangeSettings.timingOption == TimingOptions.TimingOption.Self)
                {
                    sb.Append("\n");
                    sb.AppendFormat(Content.selfTimeAtSpecificDepth,accountedTimeString, depthFilter);
                    if (singleThread)
                    {
                        sb.Append("\n\n");
                        sb.Append(Content.selectTotal);
                    }
                }
                else
                {
                    sb.Append("\n");
                    sb.AppendFormat(Content.totalTimeAtSpecificDepth, accountedTimeString, depthFilter);
                }
            }

            if (!singleThread)
            {
                sb.Append("\n\n");
                sb.Append(Content.multipleThreads);
            }

            string timeRangeString = ToDisplayUnits(m_CurrentSettings.timeRange, true);
            return new GUIContent(timeRangeString, sb.ToString());
        } 

        public void Draw(Rect rect, Color barColor, int barCount, float timeRange, Color selectedBackground, Color selectedBorder, Color selectedText, bool includeOthers, bool includeUnaccounted)
        {
            Settings newSettings = new Settings(m_RequestedRangeSettings, barCount, timeRange, includeOthers, includeUnaccounted);
            if (m_CurrentSettings != newSettings)
            {
                Profiler.BeginSample("CalculateTopMarkers");

                m_CurrentSettings = newSettings;
                m_MarkerSummary = CalculateTopMarkers();

                Profiler.EndSample();
            }

            if (m_CurrentSettings.rangeSettings == null)
                return;
            if (m_CurrentSettings.rangeSettings.dataView == null)
                return;
            if (m_CurrentSettings.rangeSettings.dataView.analysis == null)
                return;

            if (m_MarkerSummary == null || m_MarkerSummary.entry == null)
                return;

            ProfileAnalysis analysis = m_CurrentSettings.rangeSettings.dataView.analysis;
            
            FrameSummary frameSummary = analysis.GetFrameSummary();
            if (frameSummary == null)
                return;
            if (frameSummary.count <= 0)
                return;

            var markers = analysis.GetMarkers();
            if (markers == null)
                return;

            Profiler.BeginSample("DrawHeader");

            int rangeLabelWidth = 60;

            // After the marker graph we want an indication of the time range
            if (frameSummary.count > 0)
            {
                Rect rangeLabelRect = new Rect(rect.x + rect.width - rangeLabelWidth, rect.y, rangeLabelWidth, rect.height);
                GUIContent timeRangeText = ConstructTimeRangeText();
                GUI.Label(rangeLabelRect, timeRangeText);
            }

            // Reduce the size of the marker graph for the button/label we just added
            rect.width -= rangeLabelWidth;

            // Show marker graph
            float y = 0;
            float width = rect.width;
            float height = rect.height;

            string selectedPairingMarkerName = m_ProfileAnalyzerWindow.GetSelectedMarkerName();

            if (timeRange <= 0.0f)
                timeRange = frameSummary.msMedian;

            Profiler.EndSample();

            if (m_2D.DrawStart(rect, Draw2D.Origin.BottomLeft))
            {
                Profiler.BeginSample("DrawBars");

                m_2D.DrawFilledBox(0, y, width, height, m_BackgroundColor);

                foreach (MarkerSummaryEntry entry in m_MarkerSummary.entry)
                {
                    String name = entry.name;

                    float x = entry.x * width;
                    float w = entry.w * width;
                    if (entry.summaryType == SummaryType.Marker)
                    {
                        if (name == selectedPairingMarkerName)
                        {
                            DrawBar(x, y, w, height, selectedBackground, selectedBorder, true);
                        }
                        else
                        {
                            DrawBar(x, y, w, height, barColor, selectedBorder, false);
                        }
                    }
                    else
                    {
                        // Others / Unaccounted
                        Color color = entry.summaryType == SummaryType.Unaccounted ? new Color(barColor.r * 0.5f, barColor.g * 0.5f, barColor.b * 0.5f, barColor.a) : barColor;

                        DrawBar(x, y, w, height, color, selectedBorder, false);
                    }
                }

                Profiler.EndSample();

                m_2D.DrawEnd();
            }

            GUIStyle centreAlignStyle = new GUIStyle(GUI.skin.label);
            centreAlignStyle.alignment = TextAnchor.MiddleCenter;
            centreAlignStyle.normal.textColor = m_TextColor;
            GUIStyle leftAlignStyle = new GUIStyle(GUI.skin.label);
            leftAlignStyle.alignment = TextAnchor.MiddleLeft;
            leftAlignStyle.normal.textColor = m_TextColor;
            Color contentColor = GUI.contentColor;

            int frameSummaryMedianFrameIndex = m_ProfileAnalyzerWindow.GetRemappedUIFrameIndex(frameSummary.medianFrameIndex, m_CurrentSettings.rangeSettings.dataView);

            Profiler.BeginSample("DrawText");
            foreach (MarkerSummaryEntry entry in m_MarkerSummary.entry)
            {
                if (entry.summaryType == SummaryType.Marker)
                {
                    DrawEntryBarText(rect, width, timeRange, entry, leftAlignStyle, frameSummaryMedianFrameIndex, selectedText, centreAlignStyle, selectedPairingMarkerName);
                }
                else
                {
                    // Note this only displays the tooltip and not the text itself as we assume unaccounted is small
                    DrawSimpleEntryBarText(rect, width, timeRange, entry, leftAlignStyle, frameSummaryMedianFrameIndex);
                }
            }

            Profiler.EndSample();
        }

        static float CaculateWidth(float x, float msTime, float msToWidth, float width)
        {
            float w = msTime * msToWidth;
            if (x + w > width)
                w = width - x;

            return w;
        }

        float DrawBar(float x, float y, float w, float height, Color barColor, Color selectedBorder, bool withBorder)
        {
            if (withBorder)
                m_2D.DrawFilledBox(x + 1, y + 1, w, height - 2, selectedBorder);

            m_2D.DrawFilledBox(x + 2, y + 2, w - 2, height - 4, barColor);

            return w;
        }

        void DrawEntryBarText(Rect rect, float totalWidth, float timeRange, MarkerSummaryEntry entry, GUIStyle leftAlignStyle, int frameSummaryMedianFrameIndex, Color selectedText, GUIStyle centreAlignStyle, string selectedPairingMarkerName)
        {
            float x = entry.x * totalWidth;
            float w = entry.w * totalWidth;
            String name = entry.name;
            float msAtMedian = entry.msAtMedian;

            Rect labelRect = new Rect(rect.x + x, rect.y, w, rect.height);
            GUIStyle style = centreAlignStyle;
            String displayName = "";
            if (w >= 20)
            {
                displayName = name;
                Vector2 size = centreAlignStyle.CalcSize(new GUIContent(name));
                if (size.x > w)
                {
                    var words = name.Split('.');
                    displayName = words[words.Length - 1];
                    style = leftAlignStyle;
                }
            }

            float percentAtMedian = msAtMedian * 100 / timeRange;
            string tooltip = string.Format(
                Content.tooltip,
                name,
                percentAtMedian, ToDisplayUnits(msAtMedian, true, 0), frameSummaryMedianFrameIndex,
                ToDisplayUnits(entry.msMedian, true, 0), entry.medianFrameIndex);
            if (name == selectedPairingMarkerName)
                style.normal.textColor = selectedText;
            else
                style.normal.textColor = m_TextColor;
            GUI.Label(labelRect, new GUIContent(displayName, tooltip), style);

            Event current = Event.current;
            if (labelRect.Contains(current.mousePosition))
            {
                if (current.type == EventType.ContextClick)
                {
                    GenericMenu menu;
                    if (!m_ProfileAnalyzerWindow.IsAnalysisRunning())
                        menu = GenerateActiveContextMenu(name, current);
                    else
                        menu = GenerateDisabledContextMenu(name);

                    menu.ShowAsContext();

                    current.Use();
                }
                if (current.type == EventType.MouseDown)
                {
                    m_ProfileAnalyzerWindow.SelectMarker(name);
                    m_ProfileAnalyzerWindow.RequestRepaint();
                }
            }
        }

        void DrawSimpleEntryBarText(Rect rect, float totalWidth, float timeRange, MarkerSummaryEntry entry, GUIStyle leftAlignStyle, int frameSummaryMedianFrameIndex)
        {
            float x = entry.x * totalWidth;
            float w = entry.w * totalWidth;
            String name = entry.name;
            float msAtMedian = entry.msAtMedian;

            float width = rect.width;
            Rect labelRect = new Rect(rect.x + x, rect.y, w, rect.height);
            float percent = msAtMedian / timeRange * 100;
            GUIStyle style = leftAlignStyle;
            string tooltip = string.Format("{0}\n{1:f2}% ({2} on median frame {3})",
                name,
                percent,
                ToDisplayUnits(msAtMedian, true, 0),
                frameSummaryMedianFrameIndex);
            GUI.Label(labelRect, new GUIContent("", tooltip), style);

            Event current = Event.current;
            if (labelRect.Contains(current.mousePosition))
            {
                if (current.type == EventType.ContextClick)
                {
                    GenericMenu menu = new GenericMenu();

                    if (!m_ProfileAnalyzerWindow.IsAnalysisRunning())
                        menu.AddItem(Styles.menuItemSelectFramesAll, false, m_ProfileAnalyzerWindow.SelectAllFrames);
                    else
                        menu.AddDisabledItem(Styles.menuItemSelectFramesAll);

                    menu.ShowAsContext();

                    current.Use();
                }
                if (current.type == EventType.MouseDown)
                {
                    m_ProfileAnalyzerWindow.SelectMarker(null);
                    m_ProfileAnalyzerWindow.RequestRepaint();
                }
            }
        }

        void CopyToClipboard(Event current, string text)
        {
            EditorGUIUtility.systemCopyBuffer = text;
        }
    }
}
