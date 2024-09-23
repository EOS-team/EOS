using System;
using System.Collections.Generic;
using UnityEngine;

namespace UnityEditor.Performance.ProfileAnalyzer
{
    [Serializable]
    internal class ProfileAnalysis
    {
        FrameSummary m_FrameSummary = new FrameSummary();
        List<MarkerData> m_Markers = new List<MarkerData>();
        List<ThreadData> m_Threads = new List<ThreadData>();

        public ProfileAnalysis()
        {
            m_FrameSummary.first = 0;
            m_FrameSummary.last = 0;
            m_FrameSummary.count = 0;
            m_FrameSummary.msTotal = 0.0;
            m_FrameSummary.msMin = float.MaxValue;
            m_FrameSummary.msMax = 0.0f;
            m_FrameSummary.minFrameIndex = 0;
            m_FrameSummary.maxFrameIndex = 0;
            m_FrameSummary.maxMarkerDepth = 0;
            m_FrameSummary.totalMarkers = 0;
            m_FrameSummary.markerCountMax = 0;
            m_FrameSummary.markerCountMaxMean = 0.0f;
            for (int b = 0; b < m_FrameSummary.buckets.Length; b++)
                m_FrameSummary.buckets[b] = 0;

            m_Markers.Clear();
            m_Threads.Clear();
        }

        public void SetRange(int firstFrameIndex, int lastFrameIndex)
        {
            m_FrameSummary.first = firstFrameIndex;
            m_FrameSummary.last = lastFrameIndex;

            // Ensure these are initialized to frame indices within the range
            m_FrameSummary.minFrameIndex = firstFrameIndex;
            // if this wasn't initialized, and all frames had 0 length, it wouldn't be set in the UpdateSummary step of the analysis and point out of range
            m_FrameSummary.maxFrameIndex = firstFrameIndex;
        }

        public void AddMarker(MarkerData marker)
        {
            m_Markers.Add(marker);
        }

        public void AddThread(ThreadData thread)
        {
            m_Threads.Add(thread);
        }

        public void UpdateSummary(int frameIndex, float msFrame)
        {
            m_FrameSummary.msTotal += msFrame;
            m_FrameSummary.count += 1;
            if (msFrame < m_FrameSummary.msMin)
            {
                m_FrameSummary.msMin = msFrame;
                m_FrameSummary.minFrameIndex = frameIndex;
            }
            if (msFrame > m_FrameSummary.msMax)
            {
                m_FrameSummary.msMax = msFrame;
                m_FrameSummary.maxFrameIndex = frameIndex;
            }

            m_FrameSummary.frames.Add(new FrameTime(frameIndex, msFrame, 1));
        }

        FrameTime GetPercentageOffset(List<FrameTime> frames, float percent, out int outputFrameIndex)
        {
            int index = (int)((frames.Count - 1) * percent / 100);
            outputFrameIndex = frames[index].frameIndex;

            // True median is half of the sum of the middle 2 frames for an even count. However this would be a value never recorded so we avoid that.
            return frames[index];
        }

        float GetThreadPercentageOffset(List<ThreadFrameTime> frames, float percent, out int outputFrameIndex)
        {
            int index = (int)((frames.Count - 1) * percent / 100);
            outputFrameIndex = frames[index].frameIndex;

            // True median is half of the sum of the middle 2 frames for an even count. However this would be a value never recorded so we avoid that.
            return frames[index].ms;
        }

        void CalculateStandardDeviations(MarkerData marker)
        {
            if (marker.frames.Count <= 1)
            {
                marker.msStandardDeviation = 0;
                marker.countStandardDeviation = 0;
                return;
            }

            int frameCount = marker.frames.Count;
            float msMean = marker.msMean;
            float countMean = marker.countMean;

            double msSum = 0.0;
            double countSum = 0.0;
            for (int i = 0; i < frameCount; ++i)
            {
                float delta = (marker.frames[i].ms - msMean);
                msSum += (delta * delta);

                delta = (marker.frames[i].count - countMean);
                countSum += (delta * delta);
            }

            double variance = msSum / (frameCount - 1);
            marker.msStandardDeviation = (float)Math.Sqrt(variance);
            variance = countSum / (frameCount - 1);
            marker.countStandardDeviation = (float)Math.Sqrt(variance);
        }

        public void SetupMarkers()
        {
            int countMax = 0;
            float countMaxMean = 0.0f;

            foreach (MarkerData marker in m_Markers)
            {
                marker.msAtMedian = 0.0f;
                marker.msMin = float.MaxValue;
                marker.msMax = float.MinValue;
                marker.minFrameIndex = 0;
                marker.maxFrameIndex = 0;
                marker.countMin = int.MaxValue;
                marker.countMax = int.MinValue;

                foreach (FrameTime frameTime in marker.frames)
                {
                    var ms = frameTime.ms;
                    int frameIndex = frameTime.frameIndex;

                    // Total time for marker over frame
                    if (ms < marker.msMin)
                    {
                        marker.msMin = ms;
                        marker.minFrameIndex = frameIndex;
                    }
                    if (ms > marker.msMax)
                    {
                        marker.msMax = ms;
                        marker.maxFrameIndex = frameIndex;
                    }

                    if (frameIndex == m_FrameSummary.medianFrameIndex)
                        marker.msAtMedian = ms;

                    var count = frameTime.count;

                    // count for marker over frame
                    if (count < marker.countMin)
                    {
                        marker.countMin = count;
                    }
                    if (count > marker.countMax)
                    {
                        marker.countMax = count;
                    }
                }

                int unusedIndex;

                marker.msMean = marker.presentOnFrameCount > 0 ? (float)(marker.msTotal / marker.presentOnFrameCount) : 0f;
                marker.frames.Sort(FrameTime.CompareCount);
                marker.countMedian = GetPercentageOffset(marker.frames, 50, out marker.medianFrameIndex).count;
                marker.countLowerQuartile = GetPercentageOffset(marker.frames, 25, out unusedIndex).count;
                marker.countUpperQuartile = GetPercentageOffset(marker.frames, 75, out unusedIndex).count;

                marker.countMean = marker.presentOnFrameCount > 0 ? (float)marker.count / marker.presentOnFrameCount : 0f;
                marker.frames.Sort(FrameTime.CompareMs);
                marker.msMedian = GetPercentageOffset(marker.frames, 50, out marker.medianFrameIndex).ms;
                marker.msLowerQuartile = GetPercentageOffset(marker.frames, 25, out unusedIndex).ms;
                marker.msUpperQuartile = GetPercentageOffset(marker.frames, 75, out unusedIndex).ms;

                CalculateStandardDeviations(marker);

                if (marker.countMax > countMax)
                    countMax = marker.countMax;
                if (marker.countMean > countMaxMean)
                    countMaxMean = marker.countMean;
            }

            m_FrameSummary.markerCountMax = countMax;
            m_FrameSummary.markerCountMaxMean = countMaxMean;
        }

        public void SetupMarkerBuckets()
        {
            // using a for loop instead of foreach is surprisingly faster on Mono
            for (int i = 0, n = m_Markers.Count; i < n; i++)
            {
                var marker = m_Markers[i];
                marker.ComputeBuckets(marker.msMin, marker.msMax);
                marker.ComputeCountBuckets(marker.countMin, marker.countMax);
            }
        }

        public void SetupFrameBuckets(float timeScaleMax)
        {
            float first = 0;
            float last = timeScaleMax;
            float range = last - first;

            int maxBucketIndex = m_FrameSummary.buckets.Length - 1;

            for (int bucketIndex = 0; bucketIndex < m_FrameSummary.buckets.Length; bucketIndex++)
            {
                m_FrameSummary.buckets[bucketIndex] = 0;
            }

            float scale = range > 0 ? m_FrameSummary.buckets.Length / range : 0;
            // using a for loop instead of foreach is surprisingly faster on Mono
            for (int i = 0, n = m_FrameSummary.frames.Count; i < n; i++)
            {
                var frameData = m_FrameSummary.frames[i];
                var msFrame = frameData.ms;
                //var frameIndex = frameData.frameIndex;

                int bucketIndex = (int)((msFrame - first) * scale);
                if (bucketIndex < 0 || bucketIndex > maxBucketIndex)
                {
                    // It can occur for the highest entry in the range (max-min/range) = 1
                    // if (ms > max)    // Check for the spilling case
                    // Debug.Log(string.Format("Frame {0}ms exceeds range {1}-{2} on frame {3}", msFrame, first, last, frameIndex));
                    if (bucketIndex > maxBucketIndex)
                        bucketIndex = maxBucketIndex;
                    else
                        bucketIndex = 0;
                }
                m_FrameSummary.buckets[bucketIndex] += 1;
            }

            if (range == 0)
            {
                // All buckets will be the same
                for (int bucketIndex = 1; bucketIndex < m_FrameSummary.buckets.Length; bucketIndex++)
                {
                    m_FrameSummary.buckets[bucketIndex] = m_FrameSummary.buckets[0];
                }
            }
        }

        void CalculateThreadMedians()
        {
            foreach (var thread in m_Threads)
            {
                if (thread.frames.Count > 0)
                {
                    thread.frames.Sort();
                    int unusedIndex;

                    thread.msMin = GetThreadPercentageOffset(thread.frames, 0, out thread.minFrameIndex);
                    thread.msLowerQuartile = GetThreadPercentageOffset(thread.frames, 25, out unusedIndex);
                    thread.msMedian = GetThreadPercentageOffset(thread.frames, 50, out thread.medianFrameIndex);
                    thread.msUpperQuartile = GetThreadPercentageOffset(thread.frames, 75, out unusedIndex);
                    thread.msMax = GetThreadPercentageOffset(thread.frames, 100, out thread.maxFrameIndex);

                    // Put back in order of frames
                    thread.frames.Sort((a, b) => a.frameIndex.CompareTo(b.frameIndex));
                }
                else
                {
                    thread.msMin = 0f;
                    thread.msLowerQuartile = 0f;
                    thread.msMedian = 0f;
                    thread.msUpperQuartile = 0f;
                    thread.msMax = 0f;
                }
            }
        }

        public void Finalise(float timeScaleMax, int maxMarkerDepth)
        {
            if (m_FrameSummary.frames.Count > 0)
            {
                m_FrameSummary.frames.Sort();
                m_FrameSummary.msMean = (float)(m_FrameSummary.msTotal / m_FrameSummary.count);
                m_FrameSummary.msMedian = GetPercentageOffset(m_FrameSummary.frames, 50, out m_FrameSummary.medianFrameIndex).ms;
                int unusedIndex;
                m_FrameSummary.msLowerQuartile = GetPercentageOffset(m_FrameSummary.frames, 25, out unusedIndex).ms;
                m_FrameSummary.msUpperQuartile = GetPercentageOffset(m_FrameSummary.frames, 75, out unusedIndex).ms;
            }
            else
            {
                m_FrameSummary.msMean = 0f;
                m_FrameSummary.msMedian = 0f;
                m_FrameSummary.msLowerQuartile = 0f;
                m_FrameSummary.msUpperQuartile = 0f;

                // This started as float.MaxValue and won't have been updated
                m_FrameSummary.msMin = 0f;
            }
            // No longer need the frame time list ?
            //m_frameSummary.msFrame.Clear();
            m_FrameSummary.maxMarkerDepth = maxMarkerDepth;

            if (timeScaleMax <= 0.0f)
            {
                // If max frame time range not specified then use the max frame value found.
                timeScaleMax = m_FrameSummary.msMax;
            }
            else if (timeScaleMax < m_FrameSummary.msMax)
            {
                Debug.Log(string.Format("Expanding timeScaleMax {0} to match max value found {1}", timeScaleMax, m_FrameSummary.msMax));

                // If max frame time range too small we must expand it.
                timeScaleMax = m_FrameSummary.msMax;
            }

            SetupMarkers();
            SetupMarkerBuckets();
            SetupFrameBuckets(timeScaleMax);

            // Sort in median order (highest first)
            m_Markers.Sort(SortByAtMedian);

            CalculateThreadMedians();
        }

        int SortByAtMedian(MarkerData a, MarkerData b)
        {
            if (a.msAtMedian == b.msAtMedian)
                return -a.medianFrameIndex.CompareTo(b.medianFrameIndex);

            return -a.msAtMedian.CompareTo(b.msAtMedian);
        }

        public List<MarkerData> GetMarkers()
        {
            return m_Markers;
        }

        public List<ThreadData> GetThreads()
        {
            return m_Threads;
        }

        public ThreadData GetThreadByName(string threadNameWithIndex)
        {
            foreach (var thread in m_Threads)
            {
                if (thread.threadNameWithIndex == threadNameWithIndex)
                    return thread;
            }

            return null;
        }

        public FrameSummary GetFrameSummary()
        {
            return m_FrameSummary;
        }

        public MarkerData GetMarker(int index)
        {
            if (index < 0 || index >= m_Markers.Count)
                return null;

            return m_Markers[index];
        }

        public int GetMarkerIndexByName(string markerName)
        {
            if (markerName == null)
                return -1;

            for (int index = 0; index < m_Markers.Count; index++)
            {
                var marker = m_Markers[index];
                if (marker.name == markerName)
                {
                    return index;
                }
            }

            return -1;
        }

        public MarkerData GetMarkerByName(string markerName)
        {
            if (markerName == null)
                return null;

            for (int index = 0; index < m_Markers.Count; index++)
            {
                var marker = m_Markers[index];
                if (marker.name == markerName)
                {
                    return marker;
                }
            }

            return null;
        }
    }
}
