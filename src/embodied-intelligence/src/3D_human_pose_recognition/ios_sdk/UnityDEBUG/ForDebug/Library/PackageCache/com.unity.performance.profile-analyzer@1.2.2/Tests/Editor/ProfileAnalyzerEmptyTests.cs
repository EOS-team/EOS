using NUnit.Framework;
using System.Collections.Generic;
using UnityEngine;

public class ProfileAnalyzerEmptyTests : ProfileAnalyzerBaseTest
{
    List<int> SelectRange(int startIndex, int endIndex)
    {
        List<int> list = new List<int>();
        for (int c = startIndex; c <= endIndex; c++)
        {
            list.Add(c);
        }

        return list;
    }

    [Test]
    public void ProfileAnalyzer_EmptyData_IsEmpty()
    {
        int k_FirstFrameInProfiler = 1;
        int k_LastFrameInProfiler = 300;
        int k_FrameCountInProfiler = k_LastFrameInProfiler - k_FirstFrameInProfiler + 1;
        // the first and last frame are incomplete (empty) and therefore removed from the frame count of the loaded data in Profile Analyzer
        int k_FrameCountInProfileAnalyzer = k_FrameCountInProfiler - 2;
        // The first frame is invalid,
        int k_FirstValidFrameInProfiler = k_FirstFrameInProfiler + 1;
        // the first frame being missing is adjusted for via the frame offset, so only the last frame needs to be trimmed in
        int k_LastFrameInProfileAnalyzer = k_LastFrameInProfiler - 1;

        var analyzer = m_SetupData.analyzer;
        var profileData = m_SetupData.profilerWindowInterface.PullFromProfiler(k_FirstFrameInProfiler, k_LastFrameInProfiler);
        var depthFilter = m_SetupData.depthFilter;
        var threadFilters = m_SetupData.threadFilters;

        int firstFrameIndex = profileData.OffsetToDisplayFrame(0);
        int lastFrameIndex  = profileData.OffsetToDisplayFrame(profileData.GetFrameCount() - 1);

        Assert.AreEqual(k_FirstValidFrameInProfiler, firstFrameIndex, "First Frame index not " + k_FirstValidFrameInProfiler);
        Assert.AreEqual(k_LastFrameInProfileAnalyzer, lastFrameIndex, "Last Frame index is not " + k_LastFrameInProfileAnalyzer);

        var analysis = analyzer.Analyze(profileData, SelectRange(firstFrameIndex, lastFrameIndex), threadFilters, depthFilter);
        var frameSummary = analysis.GetFrameSummary();

        Assert.AreEqual(0, analysis.GetThreads().Count);
        Assert.AreEqual(0, frameSummary.msTotal);
        Assert.AreEqual(k_FirstValidFrameInProfiler, frameSummary.first);
        Assert.AreEqual(k_LastFrameInProfileAnalyzer, frameSummary.last);
        Assert.AreEqual(k_FrameCountInProfileAnalyzer, frameSummary.count);
        Assert.AreEqual(0, frameSummary.msMean);
        Assert.AreEqual(0, frameSummary.msMedian);
        Assert.AreEqual(0, frameSummary.msLowerQuartile);
        Assert.AreEqual(0, frameSummary.msUpperQuartile);
        Assert.AreEqual(0, frameSummary.msMin);
        Assert.AreEqual(0, frameSummary.msMax);
        Assert.AreEqual(Mathf.RoundToInt((float)(k_LastFrameInProfileAnalyzer + 0.1f) / 2.0f), frameSummary.medianFrameIndex);
        Assert.AreEqual(k_FirstValidFrameInProfiler, frameSummary.minFrameIndex);
        Assert.AreEqual(k_FirstValidFrameInProfiler, frameSummary.maxFrameIndex);
        Assert.AreEqual(0, frameSummary.maxMarkerDepth);
        Assert.AreEqual(0, frameSummary.totalMarkers);
    }
}
