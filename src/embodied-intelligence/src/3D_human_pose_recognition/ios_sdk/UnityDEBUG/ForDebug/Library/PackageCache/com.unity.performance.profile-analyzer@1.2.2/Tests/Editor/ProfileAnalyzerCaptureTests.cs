using NUnit.Framework;
using System.Collections;
using System.Collections.Generic;
using UnityEngine.TestTools;
using UnityEditor;
using UnityEditorInternal;
public class ProfileAnalyzerCaptureTests : ProfileAnalyzerBaseTest
{
    [UnityTest]
    public IEnumerator PlayMode_Capture_ContainsNoDuplicates()
    {
        StartProfiler();

        yield return null;
        yield return null;
        yield return null;
        yield return null;
        yield return null;

        StopProfiler();

        // Seem to need one more frame to get the data transfered over so the profile analyzer can pull it.
        yield return null;

        //analyze the data
        m_SetupData.profileData = m_SetupData.profilerWindowInterface.PullFromProfiler(m_SetupData.firstFrame, m_SetupData.lastFrame);
        var analysis = GetAnalysisFromFrameData(m_SetupData.profileData);

        var analysisMarkers = analysis.GetMarkers();
        var analysisMarkerDict = new Dictionary<string, int>();
        for (int i = 0; i < analysisMarkers.Count; ++i)
        {
            int count = 0;
            string curName = analysisMarkers[i].name;

            analysisMarkerDict.TryGetValue(curName, out count);

            analysisMarkerDict[curName] = count + 1;
        }

        Assert.IsTrue(0 != analysisMarkerDict.Count, "analysisMarkerSet count is zero!");

        foreach (var entry in analysisMarkerDict)
        {
            Assert.IsTrue(1 == entry.Value, "Duplicates found in analysis marker list: " + entry.Key);
        }
    }
}
