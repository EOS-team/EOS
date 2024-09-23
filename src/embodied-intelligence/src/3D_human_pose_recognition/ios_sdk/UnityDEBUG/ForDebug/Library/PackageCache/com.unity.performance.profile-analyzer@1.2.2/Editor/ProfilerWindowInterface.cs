using UnityEditorInternal;
using System.Reflection;
using System;
using System.Collections;
using UnityEngine;
using System.Collections.Generic;
using System.IO;
using UnityEngine.Profiling;
using UnityEditor.Profiling;
#if UNITY_2021_2_OR_NEWER
using Unity.Profiling.Editor;
// stub so that ProfilerWindow can be moved to this namespace in trunk without a need to change PA
namespace Unity.Profiling.Editor {}
#endif

namespace UnityEditor.Performance.ProfileAnalyzer
{
    internal class ProfilerWindowInterface
    {
        bool m_ProfilerWindowInitialized = false;
        const float k_NsToMs = 1000000;
        ProgressBarDisplay m_progressBar;

        [NonSerialized] bool m_SendingSelectionEventToProfilerWindowInProgress = false;
        [NonSerialized] int m_LastSelectedFrameInProfilerWindow = 0;

#if UNITY_2021_1_OR_NEWER
        [NonSerialized] ProfilerWindow m_ProfilerWindow;
        [NonSerialized] IProfilerFrameTimeViewSampleSelectionController m_CpuProfilerModule;
#else
        Type m_ProfilerWindowType;
        EditorWindow m_ProfilerWindow;
        FieldInfo m_CurrentFrameFieldInfo;
        FieldInfo m_TimeLineGUIFieldInfo;
        FieldInfo m_SelectedEntryFieldInfo;
        FieldInfo m_SelectedNameFieldInfo;
        FieldInfo m_SelectedTimeFieldInfo;
        FieldInfo m_SelectedDurationFieldInfo;
        FieldInfo m_SelectedInstanceIdFieldInfo;
        FieldInfo m_SelectedFrameIdFieldInfo;
        FieldInfo m_SelectedThreadIndexFieldInfo;
        FieldInfo m_SelectedNativeIndexFieldInfo;
        FieldInfo m_SelectedInstanceCountFieldInfo;
        FieldInfo m_SelectedInstanceCountForThreadFieldInfo;
        FieldInfo m_SelectedInstanceCountForFrameFieldInfo;
        FieldInfo m_SelectedMetaDataFieldInfo;
        FieldInfo m_SelectedThreadCountFieldInfo;
        FieldInfo m_SelectedCallstackInfoFieldInfo;

        MethodInfo m_GetProfilerModuleInfo;
        Type m_CPUProfilerModuleType;
#endif

        public ProfilerWindowInterface(ProgressBarDisplay progressBar)
        {
            m_progressBar = progressBar;

#if !UNITY_2021_1_OR_NEWER
            Assembly assem = typeof(Editor).Assembly;
            m_ProfilerWindowType = assem.GetType("UnityEditor.ProfilerWindow");
            m_CurrentFrameFieldInfo = m_ProfilerWindowType.GetField("m_CurrentFrame", BindingFlags.NonPublic | BindingFlags.Instance);

            m_TimeLineGUIFieldInfo = m_ProfilerWindowType.GetField("m_CPUTimelineGUI", BindingFlags.NonPublic | BindingFlags.Instance);
            if (m_TimeLineGUIFieldInfo == null)
            {
                // m_CPUTimelineGUI isn't present in 2019.3.0a8 onward
                m_GetProfilerModuleInfo = m_ProfilerWindowType.GetMethod("GetProfilerModule", BindingFlags.NonPublic | BindingFlags.Instance);
                if (m_GetProfilerModuleInfo == null)
                {
                    Debug.Log("Unable to initialise link to Profiler Timeline, no GetProfilerModule found");
                }

                m_CPUProfilerModuleType = assem.GetType("UnityEditorInternal.Profiling.CPUProfilerModule");
                m_TimeLineGUIFieldInfo = m_CPUProfilerModuleType.GetField("m_TimelineGUI", BindingFlags.NonPublic | BindingFlags.Instance);
                if (m_TimeLineGUIFieldInfo == null)
                {
                    Debug.Log("Unable to initialise link to Profiler Timeline");
                }
            }

            if (m_TimeLineGUIFieldInfo != null)
                m_SelectedEntryFieldInfo = m_TimeLineGUIFieldInfo.FieldType.GetField("m_SelectedEntry", BindingFlags.Public | BindingFlags.NonPublic | BindingFlags.Instance);
            if (m_SelectedEntryFieldInfo != null)
            {
                m_SelectedNameFieldInfo = m_SelectedEntryFieldInfo.FieldType.GetField("name", BindingFlags.Public | BindingFlags.NonPublic | BindingFlags.Instance);
                m_SelectedTimeFieldInfo = m_SelectedEntryFieldInfo.FieldType.GetField("time", BindingFlags.Public | BindingFlags.NonPublic | BindingFlags.Instance);
                m_SelectedDurationFieldInfo = m_SelectedEntryFieldInfo.FieldType.GetField("duration", BindingFlags.Public | BindingFlags.NonPublic | BindingFlags.Instance);
                m_SelectedInstanceIdFieldInfo = m_SelectedEntryFieldInfo.FieldType.GetField("instanceId", BindingFlags.Public | BindingFlags.NonPublic | BindingFlags.Instance);
                m_SelectedFrameIdFieldInfo = m_SelectedEntryFieldInfo.FieldType.GetField("frameId", BindingFlags.Public | BindingFlags.NonPublic | BindingFlags.Instance);
                // confusingly this is called threadId but is the thread _index_
                m_SelectedThreadIndexFieldInfo = m_SelectedEntryFieldInfo.FieldType.GetField("threadId", BindingFlags.Public | BindingFlags.NonPublic | BindingFlags.Instance);
                m_SelectedNativeIndexFieldInfo = m_SelectedEntryFieldInfo.FieldType.GetField("nativeIndex", BindingFlags.Public | BindingFlags.NonPublic | BindingFlags.Instance);
                m_SelectedInstanceCountFieldInfo = m_SelectedEntryFieldInfo.FieldType.GetField("instanceCount", BindingFlags.Public | BindingFlags.NonPublic | BindingFlags.Instance);
                m_SelectedInstanceCountForThreadFieldInfo = m_SelectedEntryFieldInfo.FieldType.GetField("instanceCountForThread", BindingFlags.Public | BindingFlags.NonPublic | BindingFlags.Instance);
                m_SelectedInstanceCountForFrameFieldInfo = m_SelectedEntryFieldInfo.FieldType.GetField("instanceCountForFrame", BindingFlags.Public | BindingFlags.NonPublic | BindingFlags.Instance);
                m_SelectedThreadCountFieldInfo = m_SelectedEntryFieldInfo.FieldType.GetField("threadCount", BindingFlags.Public | BindingFlags.NonPublic | BindingFlags.Instance);
                m_SelectedMetaDataFieldInfo = m_SelectedEntryFieldInfo.FieldType.GetField("metaData", BindingFlags.Public | BindingFlags.NonPublic | BindingFlags.Instance);
                m_SelectedCallstackInfoFieldInfo = m_SelectedEntryFieldInfo.FieldType.GetField("callstackInfo", BindingFlags.Public | BindingFlags.NonPublic | BindingFlags.Instance);
            }
#endif
        }

        public bool IsReady()
        {
            return m_ProfilerWindow != null && m_ProfilerWindowInitialized;
        }

        public void GetProfilerWindowHandle()
        {
            Profiler.BeginSample("GetProfilerWindowHandle");
#if UNITY_2021_1_OR_NEWER
            if (m_CpuProfilerModule != null)
            {
                m_CpuProfilerModule.selectionChanged -= OnSelectionChangedInCpuProfilerModule;
                m_CpuProfilerModule = null;
            }

            var windows = Resources.FindObjectsOfTypeAll<ProfilerWindow>();
            if (windows != null && windows.Length > 0)
                m_ProfilerWindow = windows[0];
            if (m_ProfilerWindow != null)
            {
#if UNITY_2021_2_OR_NEWER
                var cpuModuleIdentifier = ProfilerWindow.cpuModuleIdentifier;
#else
                var cpuModuleIdentifier = ProfilerWindow.cpuModuleName;
#endif
                m_CpuProfilerModule =
                    m_ProfilerWindow.GetFrameTimeViewSampleSelectionController(cpuModuleIdentifier);
                m_CpuProfilerModule.selectionChanged -= OnSelectionChangedInCpuProfilerModule;
                m_CpuProfilerModule.selectionChanged += OnSelectionChangedInCpuProfilerModule;

                m_ProfilerWindow.Repaint();
                m_ProfilerWindowInitialized = false;
                // wait a frame for the Profiler to get Repainted
                EditorApplication.delayCall += () => m_ProfilerWindowInitialized = true;
            }
#else
            UnityEngine.Object[] windows = Resources.FindObjectsOfTypeAll(m_ProfilerWindowType);
            if (windows != null && windows.Length > 0)
                m_ProfilerWindow = (EditorWindow)windows[0];
            m_ProfilerWindowInitialized = true;
#endif
            Profiler.EndSample();
        }

        public void OpenProfilerOrUseExisting()
        {
            // Note we use existing if possible to fix a bug after domain reload
            // Where calling EditorWindow.GetWindow directly causes a second window to open
            if (m_ProfilerWindow == null)
            {
#if UNITY_2021_1_OR_NEWER
                m_ProfilerWindow = EditorWindow.GetWindow<ProfilerWindow>();
#if UNITY_2021_2_OR_NEWER
                var cpuModuleIdentifier = ProfilerWindow.cpuModuleIdentifier;
#else
                var cpuModuleIdentifier = ProfilerWindow.cpuModuleName;
#endif
                m_CpuProfilerModule = m_ProfilerWindow.GetFrameTimeViewSampleSelectionController(cpuModuleIdentifier);
                m_CpuProfilerModule.selectionChanged -= OnSelectionChangedInCpuProfilerModule;
                m_CpuProfilerModule.selectionChanged += OnSelectionChangedInCpuProfilerModule;
#else
                // Create new
                m_ProfilerWindow = EditorWindow.GetWindow(m_ProfilerWindowType);
#endif
            }
        }

        public bool GetFrameRangeFromProfiler(out int first, out int last)
        {
            if (m_ProfilerWindow != null)
            {
                first = 1 + ProfilerDriver.firstFrameIndex;
                last = 1 + ProfilerDriver.lastFrameIndex;
                return true;
            }

            first = 1;
            last = 1;
            return false;
        }

        public void CloseProfiler()
        {
            if (m_ProfilerWindow != null)
                m_ProfilerWindow.Close();
        }

#if !UNITY_2021_1_OR_NEWER
        object GetTimeLineGUI()
        {
            object timeLineGUI = null;

            if (m_CPUProfilerModuleType != null)
            {
                object[] parametersArray = new object[] { ProfilerArea.CPU };
                var getCPUProfilerModuleInfo = m_GetProfilerModuleInfo.MakeGenericMethod(m_CPUProfilerModuleType);
                var cpuModule = getCPUProfilerModuleInfo.Invoke(m_ProfilerWindow, parametersArray);

                timeLineGUI = m_TimeLineGUIFieldInfo.GetValue(cpuModule);
            }
            else if (m_TimeLineGUIFieldInfo != null)
            {
                timeLineGUI = m_TimeLineGUIFieldInfo.GetValue(m_ProfilerWindow);
            }

            return timeLineGUI;
        }

#endif


#if UNITY_2021_1_OR_NEWER
        private void OnSelectionChangedInCpuProfilerModule(IProfilerFrameTimeViewSampleSelectionController controller, ProfilerTimeSampleSelection selection)
        {
            if (controller == m_CpuProfilerModule && !m_SendingSelectionEventToProfilerWindowInProgress)
            {
                if (selection != null && selection.markerNamePath != null && selection.markerNamePath.Count > 0)
                {
                    selectedMarkerChanged(selection.markerNamePath[selection.markerNamePath.Count - 1], selection.threadGroupName, selection.threadName);
                }
            }
        }

#endif

        public event Action<string, string, string> selectedMarkerChanged = delegate {};

        public void PollProfilerWindowMarkerName()
        {
#if !UNITY_2021_1_OR_NEWER
            if (m_ProfilerWindow != null)
            {
                var timeLineGUI = GetTimeLineGUI();
                if (timeLineGUI != null && m_SelectedEntryFieldInfo != null)
                {
                    var selectedEntry = m_SelectedEntryFieldInfo.GetValue(timeLineGUI);
                    if (selectedEntry != null && m_SelectedNameFieldInfo != null)
                    {
                        string threadGroupName = null;
                        string threadName = null;
                        if (m_SelectedFrameIdFieldInfo != null && m_SelectedThreadIndexFieldInfo != null)
                        {
                            using (RawFrameDataView frameData = ProfilerDriver.GetRawFrameDataView((int)m_SelectedFrameIdFieldInfo.GetValue(selectedEntry), (int)m_SelectedThreadIndexFieldInfo.GetValue(selectedEntry)))
                            {
                                if (frameData != null && frameData.valid)
                                {
                                    threadGroupName = frameData.threadGroupName;
                                    threadName = frameData.threadName;
                                }
                            }
                        }
                        selectedMarkerChanged(m_SelectedNameFieldInfo.GetValue(selectedEntry).ToString(), threadGroupName, threadName);
                    }
                }
            }
#endif
        }

        public ProfileData PullFromProfiler(int firstFrameDisplayIndex, int lastFrameDisplayIndex)
        {
            Profiler.BeginSample("ProfilerWindowInterface.PullFromProfiler");

            bool recording = IsRecording();
            if (recording)
                StopRecording();

            int firstFrameIndex = Mathf.Max(firstFrameDisplayIndex - 1, 0);
            int lastFrameIndex = lastFrameDisplayIndex - 1;
            ProfileData profileData = GetData(firstFrameIndex, lastFrameIndex);

            if (recording)
                StartRecording();

            Profiler.EndSample();
            return profileData;
        }

        public int GetThreadCountForFrame(int frameIndex)
        {
            if (!IsReady())
                return 0;

            ProfilerFrameDataIterator frameData = new ProfilerFrameDataIterator();
            frameData.SetRoot(frameIndex, 0);
            return frameData.GetThreadCount(frameIndex);
        }

        public ProfileFrame GetProfileFrameForThread(int frameIndex, int threadIndex)
        {
            if (!IsReady())
                return null;

            var frame = new ProfileFrame();
            using (RawFrameDataView frameData = ProfilerDriver.GetRawFrameDataView(frameIndex, threadIndex))
            {
                frame.msStartTime = frameData.frameStartTimeMs;
                frame.msFrame = frameData.frameTimeMs;
            }
            return frame;
        }

        ProfileData GetDataRaw(ProfileData data, int firstFrameIndex, int lastFrameIndex)
        {
            bool firstError = true;

            data.SetFrameIndexOffset(firstFrameIndex);

            var depthStack = new Stack<int>();

            var threadNameCount = new Dictionary<string, int>();
            var markerIdToNameIndex = new Dictionary<int, int>();

            for (int frameIndex = firstFrameIndex; frameIndex <= lastFrameIndex; ++frameIndex)
            {
                m_progressBar.AdvanceProgressBar();

                int threadIndex = 0;

                threadNameCount.Clear();
                ProfileFrame frame = null;
                while (true)
                {
                    using (RawFrameDataView frameData = ProfilerDriver.GetRawFrameDataView(frameIndex, threadIndex))
                    {
                        if (threadIndex == 0)
                        {
                            if ((frameIndex == firstFrameIndex || frameIndex == lastFrameIndex)
                                && firstFrameIndex != lastFrameIndex && (!frameData.valid || frameData.frameTimeNs == 0))
                            {
                                // skip incomplete frames when they are at the beginning or end of the capture
                                if (++frameIndex <= lastFrameIndex)
                                {
                                    data.FirstFrameIncomplete = true;
                                    data.SetFrameIndexOffset(frameIndex);
                                    continue;
                                }
                                else
                                {
                                    // break out entirely if this is the last frame
                                    data.LastFrameIncomplete = true;
                                    break;
                                }
                            }
                            frame = new ProfileFrame();
                            if (frameData.valid)
                            {
                                frame.msStartTime = frameData.frameStartTimeMs;
                                frame.msFrame = frameData.frameTimeMs;
                            }
                            data.Add(frame);
                        }

                        if (!frameData.valid)
                            break;

                        string threadNameWithIndex = null;
                        string threadName = frameData.threadName;
                        if (threadName.Trim() == "")
                        {
                            Debug.Log(string.Format("Warning: Unnamed thread found on frame {0}. Corrupted data suspected, ignoring frame", frameIndex));
                            threadIndex++;
                            continue;
                        }
                        var groupName = frameData.threadGroupName;
                        threadName = ProfileData.GetThreadNameWithGroup(threadName, groupName);

                        int nameCount = 0;
                        threadNameCount.TryGetValue(threadName, out nameCount);
                        threadNameCount[threadName] = nameCount + 1;

                        threadNameWithIndex = ProfileData.ThreadNameWithIndex(threadNameCount[threadName], threadName);

                        var thread = new ProfileThread();
                        data.AddThreadName(threadNameWithIndex, thread);

                        frame.Add(thread);

                        // The markers are in depth first order
                        depthStack.Clear();
                        // first sample is the thread name
                        for (int i = 1; i < frameData.sampleCount; i++)
                        {
                            float durationMS = frameData.GetSampleTimeMs(i);
                            int markerId = frameData.GetSampleMarkerId(i);
                            if (durationMS < 0)
                            {
                                if (firstError)
                                {
                                    int displayIndex = data.OffsetToDisplayFrame(frameIndex);

                                    string name = frameData.GetSampleName(i);
                                    Debug.LogFormat("Ignoring Invalid marker time found for {0} on frame {1} on thread {2} ({3} < 0)",
                                        name, displayIndex, threadName, durationMS);

                                    firstError = false;
                                }
                            }
                            else
                            {
                                int depth = 1 + depthStack.Count;
                                var markerData = ProfileMarker.Create(durationMS, depth);

                                // Use name index directly if we have already stored this named marker before
                                int nameIndex;
                                if (markerIdToNameIndex.TryGetValue(markerId, out nameIndex))
                                {
                                    markerData.nameIndex = nameIndex;
                                }
                                else
                                {
                                    string name = frameData.GetSampleName(i);
                                    data.AddMarkerName(name, markerData);
                                    markerIdToNameIndex[markerId] = markerData.nameIndex;
                                }

                                thread.AddMarker(markerData);
                            }

                            int childrenCount = frameData.GetSampleChildrenCount(i);
                            if (childrenCount > 0)
                            {
                                depthStack.Push(childrenCount);
                            }
                            else
                            {
                                while (depthStack.Count > 0)
                                {
                                    int remainingChildren = depthStack.Pop();
                                    if (remainingChildren > 1)
                                    {
                                        depthStack.Push(remainingChildren - 1);
                                        break;
                                    }
                                }
                            }
                        }
                    }
                    threadIndex++;
                }
            }

            data.Finalise();

            return data;
        }

        ProfileData GetDataOriginal(ProfileData data, int firstFrameIndex, int lastFrameIndex)
        {
            ProfilerFrameDataIterator frameData = new ProfilerFrameDataIterator();
            bool firstError = true;

            data.SetFrameIndexOffset(firstFrameIndex);

            Dictionary<string, int> threadNameCount = new Dictionary<string, int>();
            for (int frameIndex = firstFrameIndex; frameIndex <= lastFrameIndex; ++frameIndex)
            {
                m_progressBar.AdvanceProgressBar();

                int threadCount = frameData.GetThreadCount(frameIndex);
                frameData.SetRoot(frameIndex, 0);

                var msFrame = frameData.frameTimeMS;

                if ((frameIndex == firstFrameIndex || frameIndex == lastFrameIndex)
                    && firstFrameIndex != lastFrameIndex && msFrame == 0)
                {
                    var nextFrame = frameIndex + 1;
                    // skip incomplete frames when they are at the beginning or end of the capture
                    if (nextFrame <= lastFrameIndex)
                    {
                        data.FirstFrameIncomplete = true;
                        data.SetFrameIndexOffset(nextFrame);
                        continue;
                    }
                    else
                    {
                        // break out entirely if this is the last frame
                        data.LastFrameIncomplete = true;
                        break;
                    }
                }

                /*
                if (frameIndex == lastFrameIndex)
                {
                    // Check if last frame appears to be invalid data
                    float median;
                    float mean;
                    float standardDeviation;
                    CalculateFrameTimeStats(data, out median, out mean, out standardDeviation);
                    float execessiveDeviation = (3f * standardDeviation);
                    if (msFrame > (median + execessiveDeviation))
                    {
                        Debug.LogFormat("Dropping last frame as it is significantly larger than the median of the rest of the data set {0} > {1} (median {2} + 3 * standard deviation {3})", msFrame, median + execessiveDeviation, median, standardDeviation);
                        break;
                    }
                    if (msFrame < (median - execessiveDeviation))
                    {
                        Debug.LogFormat("Dropping last frame as it is significantly smaller than the median of the rest of the data set {0} < {1} (median {2} - 3 * standard deviation {3})", msFrame, median - execessiveDeviation, median, standardDeviation);
                        break;
                    }
                }
                */

                ProfileFrame frame = new ProfileFrame();
                frame.msStartTime = 1000.0 * frameData.GetFrameStartS(frameIndex);
                frame.msFrame = msFrame;
                data.Add(frame);

                threadNameCount.Clear();
                for (int threadIndex = 0; threadIndex < threadCount; ++threadIndex)
                {
                    frameData.SetRoot(frameIndex, threadIndex);

                    var threadName = frameData.GetThreadName();
                    if (threadName.Trim() == "")
                    {
                        Debug.Log(string.Format("Warning: Unnamed thread found on frame {0}. Corrupted data suspected, ignoring frame", frameIndex));
                        continue;
                    }

                    var groupName = frameData.GetGroupName();
                    threadName = ProfileData.GetThreadNameWithGroup(threadName, groupName);

                    ProfileThread thread = new ProfileThread();
                    frame.Add(thread);

                    int nameCount = 0;
                    threadNameCount.TryGetValue(threadName, out nameCount);
                    threadNameCount[threadName] = nameCount + 1;

                    data.AddThreadName(ProfileData.ThreadNameWithIndex(threadNameCount[threadName], threadName), thread);

                    const bool enterChildren = true;
                    // The markers are in depth first order and the depth is known
                    // So we can infer a parent child relationship
                    while (frameData.Next(enterChildren))
                    {
                        if (frameData.durationMS < 0)
                        {
                            if (firstError)
                            {
                                int displayIndex = data.OffsetToDisplayFrame(frameIndex);

                                Debug.LogFormat("Ignoring Invalid marker time found for {0} on frame {1} on thread {2} ({3} < 0) : Instance id : {4}",
                                    frameData.name, displayIndex, threadName, frameData.durationMS, frameData.instanceId);

                                firstError = false;
                            }
                            continue;
                        }
                        var markerData = ProfileMarker.Create(frameData);

                        data.AddMarkerName(frameData.name, markerData);
                        thread.AddMarker(markerData);
                    }
                }
            }

            data.Finalise();

            frameData.Dispose();
            return data;
        }

        ProfileData GetData(int firstFrameIndex, int lastFrameIndex)
        {
            ProfileData data = new ProfileData(ProfileAnalyzerWindow.TmpPath);
            GetDataRaw(data, firstFrameIndex, lastFrameIndex);
            data.Write();
            return data;
        }

        public float GetFrameTimeRaw(int frameIndex)
        {
            using (RawFrameDataView frameData = ProfilerDriver.GetRawFrameDataView(frameIndex, 0))
            {
                if (!frameData.valid)
                    return 0f;

                return frameData.frameTimeMs;
            }
        }

        public float GetFrameTime(int frameIndex)
        {
            return GetFrameTimeRaw(frameIndex);
        }

        struct ThreadIndexIterator
        {
            public ProfilerFrameDataIterator frameData;
            public int threadIndex;
        }

        IEnumerator<ThreadIndexIterator> GetNextThreadIndexFittingThreadFilters(int frameIndex, List<string> threadFilters)
        {
            ProfilerFrameDataIterator frameData = new ProfilerFrameDataIterator();

            int threadCount = frameData.GetThreadCount(frameIndex);
            Dictionary<string, int> threadNameCount = new Dictionary<string, int>();
            for (int threadIndex = 0; threadIndex < threadCount; ++threadIndex)
            {
                frameData.SetRoot(frameIndex, threadIndex);

                var threadName = frameData.GetThreadName();
                // Name here could be "Worker Thread 1"

                var groupName = frameData.GetGroupName();
                threadName = ProfileData.GetThreadNameWithGroup(threadName, groupName);

                int nameCount = 0;
                threadNameCount.TryGetValue(threadName, out nameCount);
                threadNameCount[threadName] = nameCount + 1;

                var threadNameWithIndex = ProfileData.ThreadNameWithIndex(threadNameCount[threadName], threadName);

                // To compare on the filter we need to remove the postfix on the thread name
                // "3:Worker Thread 0" -> "1:Worker Thread"
                // The index of the thread (0) is used +1 as a prefix
                // The preceding number (3) is the count of number of threads with this name
                // Unfortunately multiple threads can have the same name
                threadNameWithIndex = ProfileData.CorrectThreadName(threadNameWithIndex);

                if (threadFilters.Contains(threadNameWithIndex))
                {
                    yield return new ThreadIndexIterator {frameData = frameData, threadIndex = threadIndex};
                }
            }
            frameData.Dispose();
        }

        bool GetMarkerInfo(string markerName, int frameIndex, List<string> threadFilters, out int outThreadIndex, out int outNativeIndex, out float time, out float duration, out int instanceId)
        {
            outThreadIndex = 0;
            outNativeIndex = 0;
            time = 0.0f;
            duration = 0.0f;
            instanceId = 0;
            bool found = false;

            var iterator = GetNextThreadIndexFittingThreadFilters(frameIndex, threadFilters);
            while (iterator.MoveNext())
            {
                const bool enterChildren = true;
                while (iterator.Current.frameData.Next(enterChildren))
                {
                    if (iterator.Current.frameData.name == markerName)
                    {
                        time = iterator.Current.frameData.startTimeMS;
                        duration = iterator.Current.frameData.durationMS;
                        instanceId = iterator.Current.frameData.instanceId;
                        outNativeIndex = iterator.Current.frameData.sampleId;
                        outThreadIndex = iterator.Current.threadIndex;
                        found = true;
                        break;
                    }
                }

                if (found)
                    break;
            }

            return found;
        }

        public bool SetProfilerWindowMarkerName(string markerName, List<string> threadFilters)
        {
            m_SendingSelectionEventToProfilerWindowInProgress = true;
            if (m_ProfilerWindow == null)
                return false;
#if UNITY_2021_1_OR_NEWER
#if UNITY_2021_2_OR_NEWER
            var cpuModuleIdentifier = ProfilerWindow.cpuModuleIdentifier;
            var selectedModuleIdentifier = m_ProfilerWindow.selectedModuleIdentifier;
#else
            var cpuModuleIdentifier = ProfilerWindow.cpuModuleName;
            var selectedModuleIdentifier = m_ProfilerWindow.selectedModuleName;
#endif
            m_CpuProfilerModule = m_ProfilerWindow.GetFrameTimeViewSampleSelectionController(cpuModuleIdentifier);
            if (m_CpuProfilerModule != null && selectedModuleIdentifier == cpuModuleIdentifier
                && m_ProfilerWindow.firstAvailableFrameIndex >= 0)
            {
                // Read profiler data direct from profile to find time/duration
                int currentFrameIndex = (int)m_ProfilerWindow.selectedFrameIndex;

                var iterator = GetNextThreadIndexFittingThreadFilters(currentFrameIndex, threadFilters);
                while (iterator.MoveNext())
                {
                    using (var rawFrameDataView = ProfilerDriver.GetRawFrameDataView(currentFrameIndex, iterator.Current.threadIndex))
                    {
                        if (m_CpuProfilerModule.SetSelection(currentFrameIndex,
                            rawFrameDataView.threadGroupName, rawFrameDataView.threadName, markerName,
                            threadId: rawFrameDataView.threadId))
                        {
                            m_ProfilerWindow.Repaint();
                            m_SendingSelectionEventToProfilerWindowInProgress = false;
                            return true; // setting the selection was successful, nothing more to do here.
                        }
                    }
                }
                // selection couldn't be found, so clear the current one to avoid confusion
                m_CpuProfilerModule.ClearSelection();
                m_ProfilerWindow.Repaint();
            }
#else
            var timeLineGUI = GetTimeLineGUI();
            if (timeLineGUI == null)
            {
                m_SendingSelectionEventToProfilerWindowInProgress = false;
                return false;
            }

            if (m_SelectedEntryFieldInfo != null)
            {
                var selectedEntry = m_SelectedEntryFieldInfo.GetValue(timeLineGUI);
                if (selectedEntry != null)
                {
                    // Read profiler data direct from profile to find time/duration
                    int currentFrameIndex = (int)m_CurrentFrameFieldInfo.GetValue(m_ProfilerWindow);
                    float time;
                    float duration;
                    int instanceId;
                    int nativeIndex;
                    int threadIndex;
                    if (GetMarkerInfo(markerName, currentFrameIndex, threadFilters, out threadIndex, out nativeIndex, out time, out duration, out instanceId))
                    {
                        /*
                        Debug.Log(string.Format("Setting profiler to {0} on {1} at frame {2} at {3}ms for {4}ms ({5})",
                                                markerName, currentFrameIndex, threadFilter, time, duration, instanceId));
                         */

                        if (m_SelectedNameFieldInfo != null)
                            m_SelectedNameFieldInfo.SetValue(selectedEntry, markerName);
                        if (m_SelectedTimeFieldInfo != null)
                            m_SelectedTimeFieldInfo.SetValue(selectedEntry, time);
                        if (m_SelectedDurationFieldInfo != null)
                            m_SelectedDurationFieldInfo.SetValue(selectedEntry, duration);
                        if (m_SelectedInstanceIdFieldInfo != null)
                            m_SelectedInstanceIdFieldInfo.SetValue(selectedEntry, instanceId);
                        if (m_SelectedFrameIdFieldInfo != null)
                            m_SelectedFrameIdFieldInfo.SetValue(selectedEntry, currentFrameIndex);
                        if (m_SelectedNativeIndexFieldInfo != null)
                            m_SelectedNativeIndexFieldInfo.SetValue(selectedEntry, nativeIndex);
                        if (m_SelectedThreadIndexFieldInfo != null)
                            m_SelectedThreadIndexFieldInfo.SetValue(selectedEntry, threadIndex);

                        // TODO : Update to fill in the total and number of instances.
                        // For now we force Instance count to 1 to avoid the incorrect info showing.
                        if (m_SelectedInstanceCountFieldInfo != null)
                            m_SelectedInstanceCountFieldInfo.SetValue(selectedEntry, 1);
                        if (m_SelectedInstanceCountForThreadFieldInfo != null)
                            m_SelectedInstanceCountForThreadFieldInfo.SetValue(selectedEntry, 1);
                        if (m_SelectedInstanceCountForFrameFieldInfo != null)
                            m_SelectedInstanceCountForFrameFieldInfo.SetValue(selectedEntry, 1);
                        if (m_SelectedThreadCountFieldInfo != null)
                            m_SelectedThreadCountFieldInfo.SetValue(selectedEntry, 1);
                        if (m_SelectedMetaDataFieldInfo != null)
                            m_SelectedMetaDataFieldInfo.SetValue(selectedEntry, "");
                        if (m_SelectedCallstackInfoFieldInfo != null)
                            m_SelectedCallstackInfoFieldInfo.SetValue(selectedEntry, "");

                        m_ProfilerWindow.Repaint();
                        m_SendingSelectionEventToProfilerWindowInProgress = false;
                        return true;
                    }
                }
            }
#endif
            m_SendingSelectionEventToProfilerWindowInProgress = false;
            return false;
        }

        public bool JumpToFrame(int index)
        {
            if (m_ProfilerWindow == null)
                return false;

            if (index - 1 < ProfilerDriver.firstFrameIndex)
                return false;
            if (index - 1 > ProfilerDriver.lastFrameIndex)
                return false;

#if UNITY_2021_1_OR_NEWER
            m_ProfilerWindow.selectedFrameIndex = index - 1;
#else
            m_CurrentFrameFieldInfo.SetValue(m_ProfilerWindow, index - 1);
#endif
            m_ProfilerWindow.Repaint();
            return true;
        }

        public int selectedFrame
        {
            get
            {
                if (m_ProfilerWindow == null)
                    return 0;
#if UNITY_2021_1_OR_NEWER
                return (int)m_ProfilerWindow.selectedFrameIndex + 1;
#else
                return (int)m_CurrentFrameFieldInfo.GetValue(m_ProfilerWindow) + 1;
#endif
            }
        }

        public event Action<int> selectedFrameChanged = delegate {};

        public void PollSelectedFrameChanges()
        {
            var currentlySelectedFrame = selectedFrame;
            if (m_LastSelectedFrameInProfilerWindow != currentlySelectedFrame && !m_SendingSelectionEventToProfilerWindowInProgress)
            {
                m_LastSelectedFrameInProfilerWindow = currentlySelectedFrame;
                selectedFrameChanged(currentlySelectedFrame);
            }
        }

        public bool IsRecording()
        {
            return ProfilerDriver.enabled;
        }

        public void StopRecording()
        {
            // Stop recording first
            ProfilerDriver.enabled = false;
        }

        public void StartRecording()
        {
            // Stop recording first
            ProfilerDriver.enabled = true;
        }

        public void OnDisable()
        {
            if (m_ProfilerWindow != null)
            {
                m_ProfilerWindow = null;
            }

#if UNITY_2021_1_OR_NEWER
            if (m_CpuProfilerModule != null)
            {
                m_CpuProfilerModule.selectionChanged -= OnSelectionChangedInCpuProfilerModule;
                m_CpuProfilerModule = null;
            }
#endif
        }
    }
}
