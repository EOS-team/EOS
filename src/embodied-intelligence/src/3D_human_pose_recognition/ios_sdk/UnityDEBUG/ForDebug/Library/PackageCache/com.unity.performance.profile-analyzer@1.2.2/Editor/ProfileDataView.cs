using System;
using System.Collections.Generic;

namespace UnityEditor.Performance.ProfileAnalyzer
{
    [Serializable]
    internal class ProfileDataView
    {
        public string path;
        public ProfileData data;
        public ProfileAnalysis analysisFullNew;
        public ProfileAnalysis analysisFull;
        public ProfileAnalysis analysisNew;
        public ProfileAnalysis analysis;
        public List<int> selectedIndices = new List<int> { 0, 0 };
        [NonSerialized]
        public bool inSyncWithProfilerData;
        public bool containsWaitForFPS { get; private set; }
        public bool containsWaitForPresent { get; private set; }

        public ProfileDataView()
        {
        }

        public ProfileDataView(ProfileDataView dataView)
        {
            path = dataView.path;
            data = dataView.data;
            analysisFullNew = dataView.analysisFullNew;
            analysisFull = dataView.analysisFull;
            analysisNew = dataView.analysisNew;
            analysis = dataView.analysis;
            selectedIndices = new List<int>(dataView.selectedIndices);
            inSyncWithProfilerData = dataView.inSyncWithProfilerData;
            containsWaitForFPS = dataView.containsWaitForFPS;
            containsWaitForPresent = dataView.containsWaitForPresent;
        }

        public void FindKeyMarkers()
        {
            containsWaitForFPS = data.GetMarkerIndex("WaitForTargetFPS") != -1;
            containsWaitForPresent = data.GetMarkerIndex("Gfx.WaitForPresentOnGfxThread") != -1;
        }

        public bool IsDataValid()
        {
            if (data == null)
                return false;

            if (data.GetFrameCount() == 0)
                return false;

            if (data.NeedsMarkerRebuild)
            {
                if (!ProfileData.Load(data.FilePath, out data))
                {
                    return false;
                }
            }

            return true;
        }

        public bool HasValidSelection()
        {
            if (selectedIndices.Count == 2 && selectedIndices[0] == 0 && selectedIndices[1] == 0)
                return false;

            return true;
        }

        public bool HasSelection()
        {
            if (selectedIndices.Count == 0)
                return false;
            if (selectedIndices.Count == data.GetFrameCount())
                return false;

            return HasValidSelection();
        }

        public bool AllSelected()
        {
            if (selectedIndices.Count != data.GetFrameCount())
                return false;

            return true;
        }

        public int GetMaxDepth()
        {
            return (analysis == null) ? 1 : analysis.GetFrameSummary().maxMarkerDepth;
        }

        int Clamp(int value, int min, int max)
        {
            if (value < min)
                value = min;
            else if (value > max)
                value = max;

            return value;
        }

        public int ClampToValidDepthValue(int depthFilter)
        {
            // ProfileAnalyzer.kDepthAll is special case that we don't test for here

            // If we have no depth values then return -1 for all (as clamp expects min<max)
            int maxDepth = GetMaxDepth();
            if (maxDepth < 1)
                return ProfileAnalyzer.kDepthAll;

            return Clamp(depthFilter, 1, maxDepth);
        }

        bool SelectAllFramesContainingMarker(string markerName, ProfileAnalysis inAnalysis)
        {
            if (inAnalysis == null)
                return false;

            selectedIndices.Clear();

            MarkerData markerData = inAnalysis.GetMarkerByName(markerName);
            if (markerData == null)
                return true;

            foreach (var frameTime in markerData.frames)
            {
                selectedIndices.Add(frameTime.frameIndex);
            }

            // Order from lowest to highest so the start/end frame display makes sense
            selectedIndices.Sort();

            return true;
        }

        public bool SelectAllFramesContainingMarker(string markerName, bool inSelection)
        {
            return SelectAllFramesContainingMarker(markerName, inSelection ? analysis : analysisFull);
        }

        int ClampToRange(int value, int min, int max)
        {
            if (value < min)
                value = min;
            if (value > max)
                value = max;

            return value;
        }

        public void ClearSelection()
        {
            selectedIndices.Clear();
        }

        public void SelectFullRange()
        {
            selectedIndices.Clear();

            if (data == null)
                return;

            for (int offset = 0; offset < data.GetFrameCount(); offset++)
            {
                selectedIndices.Add(data.OffsetToDisplayFrame(offset));
            }
        }
    }
}
