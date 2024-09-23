using NUnit.Framework;
using UnityEditor.Performance.ProfileAnalyzer;
using UnityEngine;

public class ProfileAnalysisAPITests : ProfileAnalyzerBaseTest
{
    [Test]
    public void ProfileAnalysis_SetRange_ModifiesFirstLastIndices([Values(0, 2)] int first, [Values(150, 300)] int last)
    {
        var analysis = new ProfileAnalysis();
        analysis.SetRange(first, last);

        Assert.IsTrue(first == analysis.GetFrameSummary().first);
        Assert.IsTrue(first == analysis.GetFrameSummary().minFrameIndex);
        Assert.IsTrue(first == analysis.GetFrameSummary().maxFrameIndex);
        Assert.IsTrue(last == analysis.GetFrameSummary().last);
    }

    [Test]
    public void ProfileAnalysis_UpdateSummary_AddsFramesToSummary()
    {
        var analysis = new ProfileAnalysis();
        var frameSummary = analysis.GetFrameSummary();

        Assert.IsTrue(0 == frameSummary.frames.Count);

        analysis.UpdateSummary(0, 0.1f);

        Assert.IsTrue(1 == frameSummary.frames.Count);
    }

    [Test]
    public void ProfileAnalysis_SetupMarkers_GeneratesExpectedValues()
    {
        var analysis = new ProfileAnalysis();

        var marker = new MarkerData("Test Marker");
        marker.presentOnFrameCount = 1;

        for (int i = 0; i < 10; ++i)
        {
            var frameTime = new FrameTime(i, 0.1f * i, 1);
            marker.frames.Add(frameTime);
        }

        analysis.AddMarker(marker);
        analysis.SetupMarkers();

        Assert.IsTrue(marker.count == 0);
        Assert.IsTrue(marker.firstFrameIndex == -1);
        Assert.IsTrue(marker.frames.Count == 10);
        Assert.IsTrue(marker.lastFrame == -1);
        Assert.IsTrue(marker.maxDepth == 0);
        Assert.IsTrue(marker.maxFrameIndex == 9);
        Assert.IsTrue(marker.maxIndividualFrameIndex == 0);
        Assert.IsTrue(marker.medianFrameIndex == 4);
        Assert.IsTrue(marker.minDepth == 0);
        Assert.IsTrue(marker.minFrameIndex == 0);
        Assert.IsTrue(marker.minIndividualFrameIndex == 0);
        Assert.IsTrue(marker.msAtMedian == 0);
        Assert.IsTrue(marker.msMean == 0);
        Assert.IsTrue(marker.msMinIndividual == float.MaxValue);
        Assert.IsTrue(marker.msMaxIndividual == float.MinValue);
        Assert.IsTrue(marker.msMin == 0);
        Assert.IsTrue(marker.msTotal == 0);
        Assert.IsTrue(marker.name == "Test Marker");
        Assert.IsTrue(marker.presentOnFrameCount == 1);

        //Handle floats "approximately"
        Assert.IsTrue(Mathf.Approximately(marker.msLowerQuartile, 0.2f));
        Assert.IsTrue(Mathf.Approximately(marker.msMax, 0.9f));
        Assert.IsTrue(Mathf.Approximately(marker.msUpperQuartile, 0.6f));
        Assert.IsTrue(Mathf.Approximately(marker.msMedian, 0.4f));
    }

    [Test]
    public void ProfileAnalysis_SetupMarkerBuckets_GeneratesExpectedValues()
    {
        var analysis = new ProfileAnalysis();

        var marker = new MarkerData("Test Marker");
        marker.presentOnFrameCount = 1;

        int range = 10;
        float offset = 0.000001f; // Without the offset rounding errors can shift a value into the preceeding bucket
        for (int i = 0; i <= range; ++i)
        {
            float value = ((float)i / (float)range);
            if (i != 0 && i != range)
                value += offset;
            var frameTime = new FrameTime(i, value, 1);
            marker.frames.Add(frameTime);
        }

        analysis.AddMarker(marker);
        analysis.SetupMarkers();
        analysis.SetupMarkerBuckets();

        Assert.IsTrue(marker.buckets[0] == 1);
        Assert.IsTrue(marker.buckets[1] == 0);
        Assert.IsTrue(marker.buckets[2] == 1);
        Assert.IsTrue(marker.buckets[3] == 0);
        Assert.IsTrue(marker.buckets[4] == 1);
        Assert.IsTrue(marker.buckets[5] == 0);
        Assert.IsTrue(marker.buckets[6] == 1);
        Assert.IsTrue(marker.buckets[7] == 0);
        Assert.IsTrue(marker.buckets[8] == 1);
        Assert.IsTrue(marker.buckets[9] == 0);
        Assert.IsTrue(marker.buckets[10] == 1);
        Assert.IsTrue(marker.buckets[11] == 0);
        Assert.IsTrue(marker.buckets[12] == 1);
        Assert.IsTrue(marker.buckets[13] == 0);
        Assert.IsTrue(marker.buckets[14] == 1);
        Assert.IsTrue(marker.buckets[15] == 0);
        Assert.IsTrue(marker.buckets[16] == 1);
        Assert.IsTrue(marker.buckets[17] == 0);
        Assert.IsTrue(marker.buckets[18] == 1);
        Assert.IsTrue(marker.buckets[19] == 1); // max value would fall into the next bucket but is clamped to here.
    }

    [Test]
    public void ProfileAnalysis_SetupFrameBuckets_GeneratesExpectedValues()
    {
        var analysis = new ProfileAnalysis();

        var marker = new MarkerData("Test Marker");
        marker.presentOnFrameCount = 1;

        for (int i = 0; i < 20; ++i)
        {
            analysis.UpdateSummary(i, 1.0f * i);
        }

        analysis.SetupFrameBuckets(20);

        var summary = analysis.GetFrameSummary();

        for (int i = 0; i < summary.buckets.Length; ++i)
        {
            Assert.IsTrue(1 == summary.buckets[i]);
        }
    }
}
