using System;

namespace UnityEditor.Performance.ProfileAnalyzer
{
    [Serializable]
    internal class MarkerPairing
    {
        public string name;
        public int leftIndex;
        public int rightIndex;
    }
}
