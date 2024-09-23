using NUnit.Framework;
using UnityEditor.Performance.ProfileAnalyzer;
using System.Collections.Generic;

public class UnitsTestFixture
{
    internal DisplayUnits displayUnits;

    public struct TestData
    {
        public readonly float value;
        public readonly string expectedOutput;

        public TestData(float value, string expectedOutput)
        {
            this.value = value;
            this.expectedOutput = expectedOutput;
        }

        public override string ToString()
        {
            return string.Format("{0} becomes {1}", value, expectedOutput);
        }
    }
}
