using System;
using System.Collections.Generic;

namespace UnityEditor.Performance.ProfileAnalyzer
{
    [Serializable]
    internal class MarkerData : IComparable<MarkerData>
    {
        public string name;
        public int nameLowerCaseHash;   // lower case name hash for faster comparisons

        public double msTotal;          // total time of this marker on a frame
        public int count;               // total number of marker calls in the timeline (multiple per frame)
        public int countMin;            // min count per frame
        public int countMax;            // max count per frame
        public float countMean;         // mean over all frames
        public int countMedian;         // median over all frames
        public int countLowerQuartile;  // over all frames
        public int countUpperQuartile;  // over all frames
        public float countStandardDeviation;
        public int lastFrame;
        public int presentOnFrameCount; // number of frames containing this marker
        public int firstFrameIndex;
        public float msMean;            // mean over all frames
        public float msMedian;          // median over all frames
        public float msLowerQuartile;   // over all frames
        public float msUpperQuartile;   // over all frames
        public float msMin;             // min total time per frame
        public float msMax;             // max total time per frame
        public float msStandardDeviation;
        public int minIndividualFrameIndex;
        public int maxIndividualFrameIndex;
        public float msMinIndividual;   // min individual function call
        public float msMaxIndividual;   // max individual function call
        public float msAtMedian;        // time at median frame
        public int medianFrameIndex;    // frame this markers median value is found on
        public int minFrameIndex;
        public int maxFrameIndex;
        public int minDepth;
        public int maxDepth;
        public List<string> threads;

        const int bucketCount = 20;
        public int[] buckets;           // Each bucket contains 'number of frames' for 'sum of markers in the frame' in that range
        public int[] countBuckets;      // Each bucket contains 'number of frames' for 'count in the frame' in that range
        public List<FrameTime> frames;

        public double timeRemoved;
        public double timeIgnored;

        public MarkerData(string markerName)
        {
            buckets = new int[bucketCount];
            countBuckets = new int[bucketCount];
            frames = new List<FrameTime>();
            threads = new List<string>();

            name = markerName;
            nameLowerCaseHash = markerName.ToLower().GetHashCode();
            msTotal = 0.0;
            count = 0;
            countMin = 0;
            countMax = 0;
            countMean = 0f;
            countMedian = 0;
            countLowerQuartile = 0;
            countUpperQuartile = 0;
            countStandardDeviation = 0f;
            lastFrame = -1;
            presentOnFrameCount = 0;
            firstFrameIndex = -1;
            msMean = 0f;
            msMedian = 0f;
            msLowerQuartile = 0f;
            msUpperQuartile = 0f;
            msMin = float.MaxValue;
            msMax = float.MinValue;
            msStandardDeviation = 0f;
            minIndividualFrameIndex = 0;
            maxIndividualFrameIndex = 0;
            msMinIndividual = float.MaxValue;
            msMaxIndividual = float.MinValue;
            msAtMedian = 0f;
            medianFrameIndex = 0;
            minFrameIndex = 0;
            maxFrameIndex = 0;
            minDepth = 0;
            maxDepth = 0;

            for (int b = 0; b < buckets.Length; b++)
            {
                buckets[b] = 0;
                countBuckets[b] = 0;
            }

            timeRemoved = 0.0;
            timeIgnored = 0.0;
        }

        /// <summary>Compare the time duration between the marker median times. Used for sorting in descending order</summary>
        /// <param name="other"> The other MarkerData to compare </param>
        /// <returns>-1 if this is larger, 0 if the same, 1 if this is smaller</returns>
        public int CompareTo(MarkerData other)
        {
            if (msMedian == other.msMedian)
            {
                if (medianFrameIndex == other.medianFrameIndex)
                {
                    // Tertiary sort by name order
                    return name.CompareTo(other.name);
                }

                // Secondary sort by frame index order
                return medianFrameIndex.CompareTo(other.medianFrameIndex);
            }

            return -msMedian.CompareTo(other.msMedian);
        }

        public float GetFrameMs(int frameIndex)
        {
            foreach (var frameData in frames)
            {
                if (frameData.frameIndex == frameIndex)
                    return frameData.ms;
            }

            return 0f;
        }

        public void ComputeBuckets(float min, float max)
        {
            float first = min;
            float last = max;
            float range = last - first;

            int maxBucketIndex = (buckets.Length - 1);

            for (int bucketIndex = 0; bucketIndex < buckets.Length; bucketIndex++)
            {
                buckets[bucketIndex] = 0;
            }

            float scale = range > 0 ? buckets.Length / range : 0;
            // using a for loop instead of foreach is surprisingly faster on Mono
            for (int i = 0, n = frames.Count; i < n; i++)
            {
                var frameTime = frames[i];
                var ms = frameTime.ms;
                //int frameIndex = frameTime.frameIndex;

                int bucketIndex = (int)((ms - first) * scale);
                if (bucketIndex < 0 || bucketIndex > maxBucketIndex)
                {
                    // This can happen if a single marker range is longer than the frame start end (which could occur if running on a separate thread)
                    // It can also occur for the highest entry in the range (max-min/range) = 1
                    // if (ms > max)    // Check for the spilling case
                    // Debug.Log(string.Format("Marker {0} : {1}ms exceeds range {2}-{3} on frame {4}", marker.name, ms, first, last, 1+frameIndex));

                    if (bucketIndex > maxBucketIndex)
                        bucketIndex = maxBucketIndex;
                    else
                        bucketIndex = 0;
                }
                buckets[bucketIndex] += 1;
            }

            if (range == 0)
            {
                // All buckets will be the same
                for (int bucketIndex = 1; bucketIndex < buckets.Length; bucketIndex++)
                {
                    buckets[bucketIndex] = buckets[0];
                }
            }
        }

        public void ComputeCountBuckets(int min, int max)
        {
            float first = min;
            float last = max;
            float range = last - first;

            int maxBucketIndex = (countBuckets.Length - 1);

            for (int bucketIndex = 0; bucketIndex < countBuckets.Length; bucketIndex++)
            {
                countBuckets[bucketIndex] = 0;
            }

            float scale = range > 0 ? countBuckets.Length / range : 0;
            // using a for loop instead of foreach is surprisingly faster on Mono
            for (int i = 0, n = frames.Count; i < n; i++)
            {
                var frameTime = frames[i];
                var count = frameTime.count;

                int bucketIndex = (int)((count - first) * scale);
                if (bucketIndex < 0 || bucketIndex > maxBucketIndex)
                {
                    if (bucketIndex > maxBucketIndex)
                        bucketIndex = maxBucketIndex;
                    else
                        bucketIndex = 0;
                }
                countBuckets[bucketIndex] += 1;
            }

            if (range == 0)
            {
                // All buckets will be the same
                for (int bucketIndex = 1; bucketIndex < countBuckets.Length; bucketIndex++)
                {
                    countBuckets[bucketIndex] = countBuckets[0];
                }
            }
        }

        public static string GetFirstThread(MarkerData marker)
        {
            return marker != null ? marker.threads[0] : "";
        }

        public static float GetMsMax(MarkerData marker)
        {
            return marker != null ? marker.msMax : 0.0f;
        }

        public static int GetMaxFrameIndex(MarkerData marker)
        {
            return marker != null ? marker.maxFrameIndex : 0;
        }

        public static float GetMsMin(MarkerData marker)
        {
            return marker != null ? marker.msMin : 0.0f;
        }

        public static int GetMinFrameIndex(MarkerData marker)
        {
            return marker != null ? marker.minFrameIndex : 0;
        }

        public static float GetMsMedian(MarkerData marker)
        {
            return marker != null ? marker.msMedian : 0.0f;
        }

        public static int GetMedianFrameIndex(MarkerData marker)
        {
            return marker != null ? marker.medianFrameIndex : 0;
        }

        public static float GetMsUpperQuartile(MarkerData marker)
        {
            return marker != null ? marker.msUpperQuartile : 0.0f;
        }

        public static float GetMsLowerQuartile(MarkerData marker)
        {
            return marker != null ? marker.msLowerQuartile : 0.0f;
        }

        public static float GetMsMean(MarkerData marker)
        {
            return marker != null ? marker.msMean : 0.0f;
        }

        public static float GetMsMinIndividual(MarkerData marker)
        {
            return marker != null ? marker.msMinIndividual : 0.0f;
        }

        public static float GetMsMaxIndividual(MarkerData marker)
        {
            return marker != null ? marker.msMaxIndividual : 0.0f;
        }

        public static int GetPresentOnFrameCount(MarkerData marker)
        {
            return marker != null ? marker.presentOnFrameCount : 0;
        }

        public static float GetMsAtMedian(MarkerData marker)
        {
            return marker != null ? marker.msAtMedian : 0.0f;
        }

        public static int GetCountMin(MarkerData marker)
        {
            return marker != null ? marker.countMin : 0;
        }

        public static int GetCountMax(MarkerData marker)
        {
            return marker != null ? marker.countMax : 0;
        }

        public static int GetCount(MarkerData marker)
        {
            return marker != null ? marker.count : 0;
        }

        public static float GetCountMean(MarkerData marker)
        {
            return marker != null ? marker.countMean : 0.0f;
        }

        public static double GetMsTotal(MarkerData marker)
        {
            return marker != null ? marker.msTotal : 0.0;
        }

        public static int GetMinDepth(MarkerData marker)
        {
            return marker != null ? marker.minDepth : 0;
        }

        public static int GetMaxDepth(MarkerData marker)
        {
            return marker != null ? marker.maxDepth : 0;
        }
        public static double GetTimeRemoved(MarkerData marker)
        {
            return marker != null ? marker.timeRemoved : 0.0;
        }
        public static double GetTimeIgnored(MarkerData marker)
        {
            return marker != null ? marker.timeIgnored : 0.0;
        }

        public bool IsFullyIgnored()
        {
            if (timeIgnored > 0.0)
            {
                if (msTotal == 0.0)
                    return true;
            }

            return false;
        }
    }
}
