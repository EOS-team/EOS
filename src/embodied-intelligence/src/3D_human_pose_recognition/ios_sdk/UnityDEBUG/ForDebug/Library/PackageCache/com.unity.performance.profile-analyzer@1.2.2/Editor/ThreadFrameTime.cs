using System;

namespace UnityEditor.Performance.ProfileAnalyzer
{
    [Serializable]
    internal struct ThreadFrameTime : IComparable<ThreadFrameTime>
    {
        public int frameIndex;
        public float ms;
        public float msIdle;

        public ThreadFrameTime(int index, float msTime, float msTimeIdle)
        {
            frameIndex = index;
            ms = msTime;
            msIdle = msTimeIdle;
        }

        public int CompareTo(ThreadFrameTime other)
        {
            return ms.CompareTo(other.ms);
        }
    }
}
