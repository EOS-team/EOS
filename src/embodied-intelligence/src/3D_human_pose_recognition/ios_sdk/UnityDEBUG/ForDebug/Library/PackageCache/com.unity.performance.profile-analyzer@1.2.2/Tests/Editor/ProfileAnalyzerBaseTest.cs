using NUnit.Framework;
using UnityEditor.Performance.ProfileAnalyzer;
using UnityEditor;
using UnityEditorInternal;
using System.Collections.Generic;

public class ProfileAnalyzerBaseTest
{
    protected struct FrameSetupData
    {
        internal ProgressBarDisplay progressBar;
        internal ProfileAnalyzer analyzer;
        internal ProfilerWindowInterface profilerWindowInterface;
        internal ProfileData profileData;
        internal int depthFilter;
        internal List<string> threadFilters;
        internal int firstFrame;
        internal int lastFrame;
        internal FrameSetupData(int minFrame, int maxFrame, int filterDepth, List<string> filterThreads)
        {
            progressBar = new ProgressBarDisplay();
            firstFrame = minFrame;
            lastFrame = maxFrame;
            analyzer = new ProfileAnalyzer();
            profilerWindowInterface = new ProfilerWindowInterface(progressBar);
            profileData = profilerWindowInterface.PullFromProfiler(minFrame, maxFrame);
            depthFilter = filterDepth;
            threadFilters = filterThreads;
        }
    };

    protected FrameSetupData m_SetupData;

    [SetUp]
    public void SetupTest()
    {
        ProfilerDriver.ClearAllFrames();
        m_SetupData = new FrameSetupData(1, 300, -1, new List<string> { "1:Main Thread" });
    }

    [TearDown]
    public void TearDownTest()
    {
    }

    List<int> SelectRange(int startIndex, int endIndex)
    {
        List<int> list = new List<int>();
        for (int c = startIndex; c <= endIndex; c++)
        {
            list.Add(c);
        }

        return list;
    }

    internal ProfileAnalysis GetAnalysisFromFrameData(ProfileData profileData)
    {
        return m_SetupData.analyzer.Analyze(profileData,
            SelectRange(m_SetupData.firstFrame, m_SetupData.lastFrame),
            m_SetupData.threadFilters,
            m_SetupData.depthFilter);
    }

    protected void StartProfiler()
    {
#if UNITY_2017_1_OR_NEWER
        ProfilerDriver.enabled = true;
#endif
        ProfilerDriver.profileEditor = true;
    }

    protected void StopProfiler()
    {
        EditorApplication.isPlaying = false;
#if UNITY_2017_1_OR_NEWER
        ProfilerDriver.enabled = false;
#endif
        ProfilerDriver.profileEditor = false;
    }
}
