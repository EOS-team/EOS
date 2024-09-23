using UnityEngine;

namespace UnityEditor.TestTools.CodeCoverage
{
    [InitializeOnLoad]
    internal class CompatibilityInitializer
    {
        static CompatibilityInitializer()
        {
            Debug.LogError("[Code Coverage] The Code Coverage package is not compatible with versions of Unity earlier than 2019.2.");
        }
    }
}
