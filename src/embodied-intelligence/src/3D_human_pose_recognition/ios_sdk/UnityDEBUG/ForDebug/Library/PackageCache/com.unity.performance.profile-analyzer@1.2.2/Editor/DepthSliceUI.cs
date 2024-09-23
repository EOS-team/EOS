using System;
using System.Collections.Generic;
using UnityEngine;
using UnityEngine.Video;

namespace UnityEditor.Performance.ProfileAnalyzer
{
    [Serializable]
    public class DepthSliceUI
    {
        [SerializeField] int m_DepthFilter = ProfileAnalyzer.kDepthAll;
        public int depthFilter {get { return m_DepthFilter; }}

        [SerializeField] int m_DepthFilter1 = ProfileAnalyzer.kDepthAll;
        public int depthFilter1 {get { return m_DepthFilter1; }}

        [SerializeField] int m_DepthFilter2 = ProfileAnalyzer.kDepthAll;
        public int depthFilter2 {get { return m_DepthFilter2; }}

        [SerializeField] bool m_DepthFilterAuto = true;

        [SerializeField] int m_MostCommonDepthDiff = 0;

        int mostCommonDepthDiff
        {
            set
            {
                if (m_MostCommonDepthDiff != value)
                {
                    m_MostCommonDepthDiff = value;
                    UpdateAutoDepthTitleText();
                }
            }
            get
            {
                return m_MostCommonDepthDiff;
            }
        }

        void UpdateAutoDepthTitleText()
        {
            ProfileAnalyzerWindow.Styles.autoDepthTitle.text =
                string.Format(ProfileAnalyzerWindow.Styles.autoDepthTitleText, mostCommonDepthDiff);
        }

        Action<bool> m_UpdateActiveTabCallback = null;
        public DepthSliceUI(Action<bool> updateActiveTabCallback)
        {
            m_UpdateActiveTabCallback = updateActiveTabCallback;
            UpdateAutoDepthTitleText();
        }

        public void OnEnable(Action<bool> updateActiveTabCallback)
        {
            m_UpdateActiveTabCallback = updateActiveTabCallback;
            UpdateAutoDepthTitleText();
        }

        enum ViewType
        {
            Single,
            Left,
            Right,
            Locked,
        }
        void DrawDepthFilterDropdown(GUIContent title, bool enabled, ProfileDataView view, Action<int, int, int> callback,
            ViewType viewType, ProfileDataView profileSingleView, ProfileDataView profileLeftView, ProfileDataView profileRightView)
        {
            if(title !=null)
                EditorGUILayout.LabelField(title, GUILayout.Width(ProfileAnalyzerWindow.LayoutSize.FilterOptionsEnumWidth));

            int depthFilter = ProfileAnalyzer.kDepthAll;
            int depthFilterOther = ProfileAnalyzer.kDepthAll;
            var maxDepth = view.GetMaxDepth();
            var maxDepthLeft = ProfileAnalyzer.kDepthAll;
            var maxDepthRight = ProfileAnalyzer.kDepthAll;

            var oldDepthFilter = ProfileAnalyzer.kDepthAll;
            var oldDepthFilterOtherLocked = ProfileAnalyzer.kDepthAll;
            var depthDiff = mostCommonDepthDiff;
            GUIContent content;
            switch (viewType)
            {
                case ViewType.Single:
                    oldDepthFilter = m_DepthFilter;
                    depthFilter = m_DepthFilter =
                        m_DepthFilter == ProfileAnalyzer.kDepthAll ?
                        ProfileAnalyzer.kDepthAll :
                        profileSingleView.ClampToValidDepthValue(m_DepthFilter);
                    content = new GUIContent(DepthFilterToString(depthFilter));
                    depthFilterOther = depthFilter;
                    depthDiff = 0;
                    break;
                case ViewType.Left:
                    oldDepthFilter = m_DepthFilter1;
                    depthFilter = m_DepthFilter1 =
                        m_DepthFilter1 == ProfileAnalyzer.kDepthAll ?
                        ProfileAnalyzer.kDepthAll :
                        profileLeftView.ClampToValidDepthValue(m_DepthFilter1);
                    content = new GUIContent(DepthFilterToString(depthFilter));
                    depthFilterOther = depthFilter;
                    break;
                case ViewType.Right:
                    oldDepthFilter = m_DepthFilter2;
                    depthFilter = m_DepthFilter2 = m_DepthFilter2 == ProfileAnalyzer.kDepthAll
                        ? ProfileAnalyzer.kDepthAll
                        : profileRightView.ClampToValidDepthValue(m_DepthFilter2);
                    content = new GUIContent(DepthFilterToString(depthFilter));
                    depthFilterOther = depthFilter;
                    break;
                case ViewType.Locked:
                    oldDepthFilter = m_DepthFilter1;
                    oldDepthFilterOtherLocked = m_DepthFilter2;
                    maxDepth = maxDepthLeft = profileLeftView.GetMaxDepth();
                    maxDepthRight = profileRightView.GetMaxDepth();

                    ClampDepthFilterForAutoRespectingDiff(ref m_DepthFilter1, ref m_DepthFilter2, profileLeftView, profileRightView);

                    depthFilter = m_DepthFilter1;
                    depthFilterOther = m_DepthFilter2;
                    content = new GUIContent(DepthFilterToString(m_DepthFilter1, m_DepthFilter2, mostCommonDepthDiff < 0));
                    break;
                default:
                    throw new NotImplementedException();
            }

            var lastEnabled = GUI.enabled;
            GUI.enabled = enabled;
            var rect = GUILayoutUtility.GetRect(content, EditorStyles.popup, GUILayout.MinWidth(ProfileAnalyzerWindow.LayoutSize.FilterOptionsEnumWidth));
            if (GUI.Button(rect, content, EditorStyles.popup))
            {
                var dropdown = new DepthSliceDropdown(maxDepth, depthFilter, depthFilterOther, (slice, left, right) =>
                {
                    if (slice != depthFilter || (viewType == ViewType.Locked && (left != m_DepthFilter1 || right != m_DepthFilter2)))
                    {
                        callback(slice, left, right);
                        UpdateDepthFilters(viewType == ViewType.Single, profileSingleView, profileLeftView, profileRightView);
                        m_UpdateActiveTabCallback(true);
                    }
                }, depthDiff, maxDepthRight);
                dropdown.Show(rect);
                EditorGUIUtility.ExitGUI();
            }
            else
            {
                // The depths can change because the data changed, not just because the user selected a different option in the dropdown
                // in that case, the depth filters need to perform a refresh
                if(oldDepthFilter != depthFilter || viewType == ViewType.Locked && oldDepthFilterOtherLocked != depthFilterOther)
                {
                    UpdateDepthFilters(viewType == ViewType.Single, profileSingleView, profileLeftView, profileRightView);
                    m_UpdateActiveTabCallback(true);
                }
            }
            GUI.enabled = lastEnabled;
        }

        int CalcSliceMenuEntryIndex(int filterDepthLeft, int filterDepthRight, int leftMax, int rightMax)
        {
            return mostCommonDepthDiff > 0 ?
                filterDepthRight + Math.Max(0, filterDepthLeft - rightMax + (rightMax > 0 ? mostCommonDepthDiff : filterDepthLeft > 0 ? 1 : 0)) :
                filterDepthLeft + Math.Max(0, filterDepthRight - leftMax - (leftMax > 0 ? mostCommonDepthDiff : filterDepthRight > 0 ? -1 :0));
        }

        void CalcAutoSlicesFromMenuEntryIndex(int depthSlcieMenuEntryIndex, ref int filterDepthLeft, ref int filterDepthRight, int leftMax, int rightMax)
        {
            if (mostCommonDepthDiff > 0)
            {
                filterDepthRight = Mathf.Clamp(depthSlcieMenuEntryIndex, 1, rightMax);
                filterDepthLeft = Mathf.Clamp(depthSlcieMenuEntryIndex - (rightMax > 0 ? mostCommonDepthDiff : 0), 1, leftMax);
            }
            else
            {
                filterDepthLeft = Mathf.Clamp(depthSlcieMenuEntryIndex, 1, leftMax);
                filterDepthRight = Mathf.Clamp(depthSlcieMenuEntryIndex + (leftMax > 0 ? mostCommonDepthDiff : 0), 1, rightMax);
            }
            // if a side has no depth, only allow All
            if (leftMax <= 0)
                filterDepthLeft = -1;
            if (rightMax <= 0)
                filterDepthRight = -1;
        }

        void ClampDepthFilterForAutoRespectingDiff(ref int filterDepthLeft, ref int filterDepthRight, ProfileDataView profileLeftView, ProfileDataView profileRightView)
        {
            if (filterDepthLeft == ProfileAnalyzer.kDepthAll && filterDepthRight == ProfileAnalyzer.kDepthAll)
            {
                // nothing to do here, keep showing all
                return;
            }

            var leftMax = profileLeftView.GetMaxDepth();
            var rightMax = profileRightView.GetMaxDepth();

            var sliceMenuEntryIndex = CalcSliceMenuEntryIndex(filterDepthLeft, filterDepthRight, leftMax, rightMax);
            CalcAutoSlicesFromMenuEntryIndex(sliceMenuEntryIndex, ref filterDepthLeft, ref filterDepthRight, leftMax, rightMax);
        }

        internal void DrawDepthFilter(bool isAnalysisRunning, bool singleView,
            ProfileDataView profileSingleView, ProfileDataView profileLeftView, ProfileDataView profileRightView)
        {
            bool lastEnabled = GUI.enabled;
            bool enabled = !isAnalysisRunning;

            EditorGUILayout.BeginHorizontal();
            if (singleView)
            {
                EditorGUILayout.LabelField(ProfileAnalyzerWindow.Styles.depthTitle, GUILayout.Width(ProfileAnalyzerWindow.LayoutSize.FilterOptionsLeftLabelWidth));
                DrawDepthFilterDropdown(null, enabled,
                    profileSingleView, (primary, left, right) => m_DepthFilter = primary,
                    ViewType.Single, profileSingleView, profileLeftView, profileRightView);
            }
            else
            {
                EditorGUILayout.LabelField(ProfileAnalyzerWindow.Styles.depthTitle, GUILayout.Width(ProfileAnalyzerWindow.LayoutSize.FilterOptionsLeftLabelWidth));

                if (m_DepthFilterAuto)
                {
                    DrawDepthFilterDropdown(null, enabled, profileLeftView, (primary, left, right) =>
                        {
                            m_DepthFilter1 = left;
                            m_DepthFilter2 = right;
                            ClampDepthFilterForAutoRespectingDiff(ref m_DepthFilter1, ref m_DepthFilter2, profileLeftView, profileRightView);
                        },
                        ViewType.Locked, profileSingleView, profileLeftView, profileRightView);
                }
                else
                {

                    DrawDepthFilterDropdown(ProfileAnalyzerWindow.Styles.leftDepthTitle, enabled, profileLeftView,
                        (primary, left, right) => m_DepthFilter1 = primary,
                        ViewType.Left, profileSingleView, profileLeftView, profileRightView);

                    DrawDepthFilterDropdown(ProfileAnalyzerWindow.Styles.rightDepthTitle, enabled && !m_DepthFilterAuto, profileRightView,
                        (primary, left, right) => m_DepthFilter2 = primary,
                        ViewType.Right, profileSingleView, profileLeftView, profileRightView);
                }
                bool lastDepthFilterLock = m_DepthFilterAuto;
                GUI.enabled = enabled;
                m_DepthFilterAuto = EditorGUILayout.ToggleLeft(ProfileAnalyzerWindow.Styles.autoDepthTitle, m_DepthFilterAuto);
                GUI.enabled = lastEnabled;
                if (m_DepthFilterAuto != lastDepthFilterLock)
                {
                    if (UpdateDepthFilters(singleView, profileSingleView, profileLeftView, profileRightView))
                        m_UpdateActiveTabCallback(true);
                }
            }
            GUILayout.FlexibleSpace();
            EditorGUILayout.EndHorizontal();
        }

        internal bool UpdateDepthFilters(bool singleView, ProfileDataView profileSingleView, ProfileDataView profileLeftView, ProfileDataView profileRightView)
        {
            bool changed = false;

            if (!singleView)
            {
                // First respect the auto flag
                if (UpdateAutoDepthFilter(profileLeftView, profileRightView))
                    changed = true;

                // Make sure Single matches the updated comparison view
                if (profileLeftView.path == profileSingleView.path)
                {
                    // Use same filter on single view if its the same file
                    if (m_DepthFilter != m_DepthFilter1)
                    {
                        m_DepthFilter = m_DepthFilter1;
                        changed = true;
                    }
                }
                if (profileRightView.path == profileSingleView.path)
                {
                    // Use same filter on single view if its the same file
                    if (m_DepthFilter != m_DepthFilter2)
                    {
                        m_DepthFilter = m_DepthFilter2;
                        changed = true;
                    }
                }
            }
            else
            {
                // Make sure comparisons match updated single view
                if (profileLeftView.path == profileSingleView.path)
                {
                    // Use same filter on comparison left view if its the same file
                    if (m_DepthFilter1 != m_DepthFilter)
                    {
                        m_DepthFilter1 = m_DepthFilter;
                        changed = true;
                    }
                    if (m_DepthFilterAuto)
                    {
                        var newDepthFilter2 = m_DepthFilter;
                        ClampDepthFilterForAutoRespectingDiff(ref m_DepthFilter1, ref newDepthFilter2,  profileLeftView, profileRightView);

                        if (m_DepthFilter2 != newDepthFilter2)
                        {
                            m_DepthFilter2 = newDepthFilter2;
                            changed = true;
                        }

                        if (UpdateAutoDepthFilter(profileLeftView, profileRightView))
                            changed = true;
                    }

                    if (UpdateAutoDepthFilter(profileLeftView, profileRightView))
                        changed = true;
                }

                if (profileRightView.path == profileSingleView.path)
                {
                    // Use same filter on comparison right view if its the same file
                    if (m_DepthFilter2 != m_DepthFilter)
                    {
                        m_DepthFilter2 = m_DepthFilter;
                        changed = true;
                    }
                    if (m_DepthFilterAuto)
                    {
                        var newDepthFilter1 = m_DepthFilter;
                        ClampDepthFilterForAutoRespectingDiff(ref newDepthFilter1, ref m_DepthFilter2, profileLeftView, profileRightView);
                        if (m_DepthFilter1 != newDepthFilter1)
                        {
                            m_DepthFilter1 = newDepthFilter1;
                            changed = true;
                        }

                        if (UpdateAutoDepthFilter(profileLeftView, profileRightView))
                            changed = true;
                    }
                }
            }

            return changed;
        }

        int CalculateDepthDifference(ProfileAnalysis leftAnalysis, ProfileAnalysis rightAnalysis, List<MarkerPairing> pairings)
        {
            if (pairings.Count <= 0)
            {
                mostCommonDepthDiff = 0;
                return 0;
            }

            var leftMarkers = leftAnalysis.GetMarkers();
            var rightMarkers = rightAnalysis.GetMarkers();

            int totalCount = 0;
            Dictionary<int, int> depthDifferences = new Dictionary<int, int>();
            foreach (var pairing in pairings)
            {
                if (pairing.leftIndex >= 0 && pairing.rightIndex >= 0)
                {
                    MarkerData leftMarker = leftMarkers[pairing.leftIndex];
                    MarkerData rightMarker = rightMarkers[pairing.rightIndex];
                    int markerDepthDiff = rightMarker.minDepth - leftMarker.minDepth;

                    int value = 0;
                    depthDifferences.TryGetValue(markerDepthDiff, out value);
                    depthDifferences[markerDepthDiff] = value + 1;
                    totalCount += 1;
                }
            }

            var newDepthDiff = 0;

            // Find most common depth difference
            int maxCount = 0;
            foreach (var diff in depthDifferences.Keys)
            {
                if (depthDifferences[diff] > maxCount)
                {
                    maxCount = depthDifferences[diff];
                    newDepthDiff = diff;
                }
            }

            return mostCommonDepthDiff = newDepthDiff;
        }

        bool UpdateAutoDepthFilter(ProfileDataView profileLeftView, ProfileDataView profileRightView)
        {
            if (m_DepthFilterAuto)
            {
                var newDepthFilter1 = m_DepthFilter1;
                var newDepthFilter2 = m_DepthFilter2;
                ClampDepthFilterForAutoRespectingDiff(ref newDepthFilter1, ref newDepthFilter2, profileLeftView, profileRightView);
                if (m_DepthFilter1 != newDepthFilter1)
                {
                    m_DepthFilter1 = newDepthFilter1;
                    return true;
                }

                if (m_DepthFilter2 != newDepthFilter2)
                {
                    m_DepthFilter2 = newDepthFilter2;
                    return true;
                }
            }

            return false;
        }

        internal bool UpdateDepthForCompareSync(ProfileAnalysis leftAnalysis, ProfileAnalysis rightAnalysis, List<MarkerPairing> pairings, ProfileDataView profileLeftView, ProfileDataView profileRightView)
        {
            int originalDepthDiff = mostCommonDepthDiff;
            int newDepthDiff = CalculateDepthDifference(leftAnalysis, rightAnalysis, pairings);
            if (newDepthDiff != originalDepthDiff)
            {
                UpdateAutoDepthFilter(profileLeftView, profileRightView);
                return true;
            }
            return false;
        }

        internal GUIContent GetUIInfo(bool compare)
        {
            GUIContent info;
            if (compare && m_DepthFilter1 == ProfileAnalyzer.kDepthAll && m_DepthFilter2 == ProfileAnalyzer.kDepthAll ||
                !compare && depthFilter == ProfileAnalyzer.kDepthAll)
            {
                info = new GUIContent("(All depths)", string.Format("{0}\n\nSet depth 1 to get an overview of the frame", ProfileAnalyzerWindow.Styles.medianFrameTooltip));
            }
            else
            {
                if (compare && depthFilter1 != depthFilter2)
                {
                    if (m_DepthFilter1 == ProfileAnalyzer.kDepthAll)
                        info = new GUIContent(string.Format("(Filtered to 'all' depths in the first data set, and depth '{0}' in the second)", m_DepthFilter2), ProfileAnalyzerWindow.Styles.medianFrameTooltip);
                    else if (m_DepthFilter2 == ProfileAnalyzer.kDepthAll)
                        info = new GUIContent(string.Format("(Filtered to depth '{0}' in the first data set, and 'all' depths in the second)", m_DepthFilter1), ProfileAnalyzerWindow.Styles.medianFrameTooltip);
                    else
                        info = new GUIContent(string.Format("(Filtered to depth '{0}' in the first data set, and depth '{1}' in the second)", m_DepthFilter1, depthFilter2), ProfileAnalyzerWindow.Styles.medianFrameTooltip);
                }
                else
                    info = new GUIContent(string.Format("(Filtered to depth '{0}' only)", compare ? m_DepthFilter1 : depthFilter), ProfileAnalyzerWindow.Styles.medianFrameTooltip);
            }
            return info;
        }


        public static string DepthFilterToString(int depthFilter)
        {
            return depthFilter == ProfileAnalyzer.kDepthAll ? "All" : depthFilter.ToString();
        }

        public static string DepthFilterToString(int depthSliceLeft, int depthSliceRight, bool leftIsMain)
        {
            if(depthSliceLeft != depthSliceRight)
            {
                if (leftIsMain)
                    return string.Format("{0} ({1}{2})", DepthFilterToString(depthSliceLeft), ProfileAnalyzerWindow.Styles.rightDepthTitle.text, DepthFilterToString(depthSliceRight));
                else
                    return string.Format("{0} ({1}{2})", DepthFilterToString(depthSliceRight), ProfileAnalyzerWindow.Styles.leftDepthTitle.text, DepthFilterToString(depthSliceLeft));
            }
            return DepthFilterToString(depthSliceLeft);
        }
    }
}
