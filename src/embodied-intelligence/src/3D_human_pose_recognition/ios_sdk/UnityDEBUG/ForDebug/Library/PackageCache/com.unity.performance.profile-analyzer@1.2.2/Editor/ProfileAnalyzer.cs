using System.Collections.Generic;
using UnityEngine;
using UnityEditorInternal;
using System.Text.RegularExpressions;
using System;

namespace UnityEditor.Performance.ProfileAnalyzer
{
    internal class ProfileAnalyzer
    {
        public const int kDepthAll = -1;

        int m_Progress = 0;
        ProfilerFrameDataIterator m_frameData;
        List<string> m_threadNames = new List<string>();
        ProfileAnalysis m_analysis;

        public ProfileAnalyzer()
        {
        }

        public void QuickScan()
        {
            var frameData = new ProfilerFrameDataIterator();

            m_threadNames.Clear();
            int frameIndex = 0;
            int threadCount = frameData.GetThreadCount(0);
            frameData.SetRoot(frameIndex, 0);

            Dictionary<string, int> threadNameCount = new Dictionary<string, int>();
            for (int threadIndex = 0; threadIndex < threadCount; ++threadIndex)
            {
                frameData.SetRoot(frameIndex, threadIndex);

                var threadName = frameData.GetThreadName();
                var groupName = frameData.GetGroupName();
                threadName = ProfileData.GetThreadNameWithGroup(threadName, groupName);

                if (!threadNameCount.ContainsKey(threadName))
                    threadNameCount.Add(threadName, 1);
                else
                    threadNameCount[threadName] += 1;

                string threadNameWithIndex = ProfileData.ThreadNameWithIndex(threadNameCount[threadName], threadName);
                threadNameWithIndex = ProfileData.CorrectThreadName(threadNameWithIndex);

                m_threadNames.Add(threadNameWithIndex);
            }

            frameData.Dispose();
        }

        public List<string> GetThreadNames()
        {
            return m_threadNames;
        }

        void CalculateFrameTimeStats(ProfileData data, out float median, out float mean, out float standardDeviation)
        {
            List<float> frameTimes = new List<float>();
            for (int frameIndex = 0; frameIndex < data.GetFrameCount(); frameIndex++)
            {
                var frame = data.GetFrame(frameIndex);
                float msFrame = frame.msFrame;
                frameTimes.Add(msFrame);
            }
            frameTimes.Sort();
            median = frameTimes[frameTimes.Count / 2];


            double total = 0.0f;
            foreach (float msFrame in frameTimes)
            {
                total += msFrame;
            }
            mean = (float)(total / (double)frameTimes.Count);


            if (frameTimes.Count <= 1)
            {
                standardDeviation = 0f;
            }
            else
            {
                total = 0.0f;
                foreach (float msFrame in frameTimes)
                {
                    float d = msFrame - mean;
                    total += (d * d);
                }
                total /= (frameTimes.Count - 1);
                standardDeviation = (float)Math.Sqrt(total);
            }
        }

        int GetClampedOffsetToFrame(ProfileData profileData, int frameIndex)
        {
            int frameOffset = profileData.DisplayFrameToOffset(frameIndex);
            if (frameOffset < 0)
            {
                Debug.Log(string.Format("Frame index {0} offset {1} < 0, clamping", frameIndex, frameOffset));
                frameOffset = 0;
            }
            if (frameOffset >= profileData.GetFrameCount())
            {
                Debug.Log(string.Format("Frame index {0} offset {1} >= frame count {2}, clamping", frameIndex, frameOffset, profileData.GetFrameCount()));
                frameOffset = profileData.GetFrameCount() - 1;
            }

            return frameOffset;
        }

        public static bool MatchThreadFilter(string threadNameWithIndex, List<string> threadFilters)
        {
            if (threadFilters == null || threadFilters.Count == 0)
                return false;

            if (threadFilters.Contains(threadNameWithIndex))
                return true;

            return false;
        }

        public bool IsNullOrWhiteSpace(string s)
        {
            // return string.IsNullOrWhiteSpace(parentMarker);
            if (s == null || Regex.IsMatch(s, @"^[\s]*$"))
                return true;

            return false;
        }

        public void RemoveMarkerTimeFromParents(MarkerData[] markers, ProfileData profileData, ProfileThread threadData, int markerAt)
        {
            // Get the info for the marker we plan to remove (assume thats what we are at)
            ProfileMarker profileMarker = threadData.markers[markerAt];
            float markerTime = profileMarker.msMarkerTotal;

            // Traverse parents and remove time from them
            int currentDepth = profileMarker.depth;
            for (int parentMarkerAt = markerAt - 1; parentMarkerAt >= 0; parentMarkerAt--)
            {
                ProfileMarker parentMarkerData = threadData.markers[parentMarkerAt];
                if (parentMarkerData.depth == currentDepth - 1)
                {
                    currentDepth--;
                    if (parentMarkerData.nameIndex < markers.Length) // Had an issue where marker not yet processed(marker from another thread)
                    {
                        MarkerData parentMarker = markers[parentMarkerData.nameIndex];

                        // If a depth slice is applied we may not have a parent marker stored
                        if (parentMarker != null)
                        {
                            // Revise the duration of parent to remove time from there too
                            // Note if the marker to remove is nested (i.e. parent of the same name, this could reduce the msTotal, more than we add to the timeIgnored)
                            parentMarker.msTotal -= markerTime;

                            // Reduce from the max marker time too
                            // This could be incorrect when there are many instances that contribute the the total time
                            if (parentMarker.msMaxIndividual > markerTime)
                            {
                                parentMarker.msMaxIndividual -= markerTime;
                            }
                            if (parentMarker.msMinIndividual > markerTime)
                            {
                                parentMarker.msMinIndividual -= markerTime;
                            }

                            // Revise stored frame time
                            FrameTime frameTime = parentMarker.frames[parentMarker.frames.Count - 1];
                            frameTime = new FrameTime(frameTime.frameIndex, frameTime.ms - markerTime, frameTime.count);
                            parentMarker.frames[parentMarker.frames.Count - 1] = frameTime;

                            // Note that we have modified the time
                            parentMarker.timeRemoved += markerTime;

                            // Note markerTime can be 0 in some cases.
                            // Make sure timeRemoved is never left at 0.0
                            // This makes sure we can test for non zero to indicate the marker has been removed 
                            if (parentMarker.timeRemoved == 0.0)
                                parentMarker.timeRemoved = double.Epsilon;
                        }
                    }
                }
            }
        }

        public int RemoveMarker(ProfileThread threadData, int markerAt)
        {
            ProfileMarker profileMarker = threadData.markers[markerAt];
            int at = markerAt;

            // skip marker
            at++;

            // Skip children
            int currentDepth = profileMarker.depth;
            while (at < threadData.markers.Count)
            {
                profileMarker = threadData.markers[at];
                if (profileMarker.depth <= currentDepth)
                    break;

                at++;
            }

            // Mark the following number to be ignored
            int markerAndChildCount = at - markerAt;

            return markerAndChildCount;
        }

        public ProfileAnalysis Analyze(ProfileData profileData, List<int> selectionIndices, List<string> threadFilters, int depthFilter, bool selfTimes = false, string parentMarker = null, float timeScaleMax = 0, string removeMarker = null)
        {
            m_Progress = 0;
            if (profileData == null)
            {
                return null;
            }
            if (profileData.GetFrameCount() <= 0)
            {
                return null;
            }

            int frameCount = selectionIndices.Count;
            if (frameCount < 0)
            {
                return null;
            }

            if (profileData.HasFrames && !profileData.HasThreads)
            {
                if (!ProfileData.Load(profileData.FilePath, out profileData))
                {
                    return null;
                }
            }

            bool processMarkers = (threadFilters != null);

            ProfileAnalysis analysis = new ProfileAnalysis();
            if (selectionIndices.Count > 0)
                analysis.SetRange(selectionIndices[0], selectionIndices[selectionIndices.Count - 1]);
            else
                analysis.SetRange(0, 0);

            m_threadNames.Clear();

            int maxMarkerDepthFound = 0;
            var threads = new Dictionary<string, ThreadData>();
            var markers = new MarkerData[profileData.MarkerNameCount];
            var removedMarkers = new Dictionary<string, double>();

            var mainThreadIdentifier = new ThreadIdentifier("Main Thread", 1);

            int markerCount = 0;

            bool filteringByParentMarker = false;
            int parentMarkerIndex = -1;
            if (!IsNullOrWhiteSpace(parentMarker))
            {
                // Returns -1 if this marker doesn't exist in the data set
                parentMarkerIndex = profileData.GetMarkerIndex(parentMarker);
                filteringByParentMarker = true;
            }

            int at = 0;
            foreach (int frameIndex in selectionIndices)
            {
                int frameOffset = profileData.DisplayFrameToOffset(frameIndex);
                var frameData = profileData.GetFrame(frameOffset);
                if (frameData == null)
                    continue;
                var msFrame = frameData.msFrame;

                if (processMarkers)
                {
                    // get the file reader in case we need to rebuild the markers rather than opening
                    // the file for every marker
                    for (int threadIndex = 0; threadIndex < frameData.threads.Count; threadIndex++)
                    {
                        float msTimeOfMinDepthMarkers = 0.0f;
                        float msIdleTimeOfMinDepthMarkers = 0.0f;

                        var threadData = frameData.threads[threadIndex];
                        var threadNameWithIndex = profileData.GetThreadName(threadData);

                        ThreadData thread;
                        if (!threads.ContainsKey(threadNameWithIndex))
                        {
                            m_threadNames.Add(threadNameWithIndex);

                            thread = new ThreadData(threadNameWithIndex);

                            analysis.AddThread(thread);
                            threads[threadNameWithIndex] = thread;

                            // Update threadsInGroup for all thread records of the same group name
                            foreach (var threadAt in threads.Values)
                            {
                                if (threadAt == thread)
                                    continue;

                                if (thread.threadGroupName == threadAt.threadGroupName)
                                {
                                    threadAt.threadsInGroup += 1;
                                    thread.threadsInGroup += 1;
                                }
                            }
                        }
                        else
                        {
                            thread = threads[threadNameWithIndex];
                        }

                        bool include = MatchThreadFilter(threadNameWithIndex, threadFilters);

                        int parentMarkerDepth = -1;

                        if (threadData.markers.Count != threadData.markerCount)
                        {
                            if (!threadData.ReadMarkers(profileData.FilePath))
                            {
                                Debug.LogError("failed to read markers");
                            }
                        }

                        int markerAndChildCount = 0;
                        for (int markerAt = 0, n = threadData.markers.Count; markerAt < n; markerAt++)
                        {
                            var markerData = threadData.markers[markerAt];

                            if (markerAndChildCount > 0)
                                markerAndChildCount--;

                            string markerName = null;

                            float ms = markerData.msMarkerTotal - (selfTimes ? markerData.msChildren : 0);
                            var markerDepth = markerData.depth;
                            if (markerDepth > maxMarkerDepthFound)
                                maxMarkerDepthFound = markerDepth;

                            if (markerDepth == 1)
                            {
                                markerName = profileData.GetMarkerName(markerData);
                                if (markerName.Equals("Idle", StringComparison.Ordinal))
                                    msIdleTimeOfMinDepthMarkers += ms;
                                else
                                    msTimeOfMinDepthMarkers += ms;
                            }

                            if (removeMarker != null)
                            {
                                if (markerAndChildCount <= 0)   // If we are already removing markers - don't focus on other occurances in the children
                                {
                                    if (markerName == null)
                                        markerName = profileData.GetMarkerName(markerData);

                                    if (markerName == removeMarker)
                                    {
                                        float removeMarkerTime = markerData.msMarkerTotal;

                                        // Remove this markers time from frame time (if its on the main thread)
                                        if (thread.threadNameWithIndex == mainThreadIdentifier.threadNameWithIndex)
                                        {
                                            msFrame -= removeMarkerTime;
                                        }

                                        if (selfTimes == false) // (Self times would not need thread or parent adjustments)
                                        {
                                            // And from thread time
                                            if (markerName == "Idle")
                                                msIdleTimeOfMinDepthMarkers -= removeMarkerTime;
                                            else
                                                msTimeOfMinDepthMarkers -= removeMarkerTime;

                                            // And from parents
                                            RemoveMarkerTimeFromParents(markers, profileData, threadData, markerAt);
                                        }

                                        markerAndChildCount = RemoveMarker(threadData, markerAt);
                                    }
                                }
                            }

                            if (!include)
                                continue;

                            // If only looking for markers below the parent
                            if (filteringByParentMarker)
                            {
                                // If found the parent marker
                                if (markerData.nameIndex == parentMarkerIndex)
                                {
                                    // And we are not already below the parent higher in the depth tree
                                    if (parentMarkerDepth < 0)
                                    {
                                        // record the parent marker depth
                                        parentMarkerDepth = markerData.depth;
                                    }
                                }
                                else
                                {
                                    // If we are now above or beside the parent marker then we are done for this level
                                    if (markerData.depth <= parentMarkerDepth)
                                    {
                                        parentMarkerDepth = -1;
                                    }
                                }

                                if (parentMarkerDepth < 0)
                                    continue;
                            }

                            if (depthFilter != kDepthAll && markerDepth != depthFilter)
                                continue;

                            MarkerData marker = markers[markerData.nameIndex];
                            if (marker != null)
                            {
                                if (!marker.threads.Contains(threadNameWithIndex))
                                    marker.threads.Add(threadNameWithIndex);
                            }
                            else
                            {
                                if (markerName == null)
                                    markerName = profileData.GetMarkerName(markerData);
                                marker = new MarkerData(markerName);
                                marker.firstFrameIndex = frameIndex;
                                marker.minDepth = markerDepth;
                                marker.maxDepth = markerDepth;
                                marker.threads.Add(threadNameWithIndex);
                                analysis.AddMarker(marker);
                                markers[markerData.nameIndex] = marker;
                                markerCount += 1;
                            }
                            marker.count += 1;

                            if (markerAndChildCount > 0)
                            {
                                marker.timeIgnored += ms;

                                // Note ms can be 0 in some cases.
                                // Make sure timeIgnored is never left at 0.0
                                // This makes sure we can test for non zero to indicate the marker has been ignored 
                                if (marker.timeIgnored == 0.0)
                                    marker.timeIgnored = double.Epsilon;

                                // zero out removed marker time
                                // so we don't record in the individual marker times, marker frame times or min/max times
                                // ('min/max times' is calculated later from marker frame times)
                                ms = 0f;
                            }

                            marker.msTotal += ms;

                            // Individual marker time (not total over frame)
                            if (ms < marker.msMinIndividual)
                            {
                                marker.msMinIndividual = ms;
                                marker.minIndividualFrameIndex = frameIndex;
                            }
                            if (ms > marker.msMaxIndividual)
                            {
                                marker.msMaxIndividual = ms;
                                marker.maxIndividualFrameIndex = frameIndex;
                            }

                            // Record highest depth foun
                            if (markerDepth < marker.minDepth)
                                marker.minDepth = markerDepth;
                            if (markerDepth > marker.maxDepth)
                                marker.maxDepth = markerDepth;

                            FrameTime frameTime;
                            if (frameIndex != marker.lastFrame)
                            {
                                marker.presentOnFrameCount += 1;
                                frameTime = new FrameTime(frameIndex, ms, 1);
                                marker.frames.Add(frameTime);
                                marker.lastFrame = frameIndex;
                            }
                            else
                            {
                                frameTime = marker.frames[marker.frames.Count - 1];
                                frameTime = new FrameTime(frameTime.frameIndex, frameTime.ms + ms, frameTime.count + 1);
                                marker.frames[marker.frames.Count - 1] = frameTime;
                            }
                        }

                        if (include)
                            thread.frames.Add(new ThreadFrameTime(frameIndex, msTimeOfMinDepthMarkers, msIdleTimeOfMinDepthMarkers));
                    }
                }

                analysis.UpdateSummary(frameIndex, msFrame);

                at++;
                m_Progress = (100 * at) / frameCount;
            }

            analysis.GetFrameSummary().totalMarkers = profileData.MarkerNameCount;
            analysis.Finalise(timeScaleMax, maxMarkerDepthFound);

            /*
            foreach (int frameIndex in selectionIndices)
            {
                int frameOffset = profileData.DisplayFrameToOffset(frameIndex);

                var frameData = profileData.GetFrame(frameOffset);
                foreach (var threadData in frameData.threads)
                {
                    var threadNameWithIndex = profileData.GetThreadName(threadData);

                    if (filterThreads && threadFilter != threadNameWithIndex)
                        continue;

                    const bool enterChildren = true;
                    foreach (var markerData in threadData.markers)
                    {
                        var markerName = markerData.name;
                        var ms = markerData.msFrame;
                        var markerDepth = markerData.depth;
                        if (depthFilter != kDepthAll && markerDepth != depthFilter)
                            continue;

                        MarkerData marker = markers[markerName];
                        bucketIndex = (range > 0) ? (int)(((marker.buckets.Length-1) * (ms - first)) / range) : 0;
                        if (bucketIndex<0 || bucketIndex > (marker.buckets.Length - 1))
                        {
                            // This can happen if a single marker range is longer than the frame start end (which could occur if running on a separate thread)
                            // Debug.Log(string.Format("Marker {0} : {1}ms exceeds range {2}-{3} on frame {4}", marker.name, ms, first, last, frameIndex));
                            if (bucketIndex > (marker.buckets.Length - 1))
                                bucketIndex = (marker.buckets.Length - 1);
                            else
                                bucketIndex = 0;
                        }
                        marker.individualBuckets[bucketIndex] += 1;
                    }
                }
            }
*/
            m_Progress = 100;
            return analysis;
        }

        public int GetProgress()
        {
            return m_Progress;
        }
    }
}
