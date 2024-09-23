using NUnit.Framework;
using UnityEditor.Performance.ProfileAnalyzer;
using System.Collections.Generic;

public class MillisecondsUnitsFixture : UnitsTestFixture
{
    [SetUp]
    public void SetupTest()
    {
        displayUnits = new DisplayUnits(Units.Milliseconds);
    }
}

public class MillisecondsUnitsTest : MillisecondsUnitsFixture
{
    static TestData[] DecimalLimitValues = new TestData[]
    {
        new TestData(0.00000001f,   "0.00"),
        new TestData(0.0009f,       "0.00"),
        new TestData(0.004f,        "0.00"),
        new TestData(0.005f,        "0.01"),
        new TestData(0.1012f,       "0.10"),
        new TestData(1.0012f,       "1.00"),
        new TestData(10.0012f,      "10.00"),
        new TestData(100.0012f,     "100.00"),
        new TestData(1000.0012f,    "1000.00"),
        new TestData(10000.0012f,   "10000.00"),
        new TestData(100000.0012f,  "100000.00"),
    };

    [Test]
    public void DecimalLimit([ValueSource("DecimalLimitValues")] TestData testData)
    {
        string output = displayUnits.ToString(testData.value, showUnits: false, limitToNDigits: 0, showFullValueWhenBelowZero: false);
        Assert.AreEqual(testData.expectedOutput, output);
    }

    static TestData[] ShowFullValueWhenBelowZeroValues = new TestData[]
    {
        new TestData(0.00000001f,   "1E-08"),
        new TestData(0.0009f,       "0.0009"),
        new TestData(0.004f,        "0.004"),
        new TestData(0.005f,        "0.005"),
        new TestData(0.01f,         "0.01"),
        new TestData(0.015f,        "0.02"),
        new TestData(0.1012f,       "0.10"),
        new TestData(1.0012f,       "1.00"),
        new TestData(10.0012f,      "10.00"),
        new TestData(100.0012f,     "100.00"),
        new TestData(1000.0012f,    "1000.00"),
        new TestData(10000.0012f,   "10000.00"),
        new TestData(100000.0012f,  "100000.00"),
    };

    [Test]
    public void ShowFullValueWhenBelowZero([ValueSource("ShowFullValueWhenBelowZeroValues")] TestData testData)
    {
        string output = displayUnits.ToString(testData.value, showUnits: false, limitToNDigits: 0, showFullValueWhenBelowZero: true);
        Assert.AreEqual(testData.expectedOutput, output);
    }

    static TestData[] ShowUnitsValues = new TestData[]
    {
        new TestData(0.00000001f,   "0.00ms"),
        new TestData(0.0009f,       "0.00ms"),
        new TestData(0.004f,        "0.00ms"),
        new TestData(0.005f,        "0.01ms"),
        new TestData(0.1012f,       "0.10ms"),
        new TestData(1.0012f,       "1.00ms"),
        new TestData(10.0012f,      "10.00ms"),
        new TestData(100.0012f,     "100.00ms"),
        new TestData(1000.0012f,    "1000.00ms"),
        new TestData(10000.0012f,   "10000.00ms"),
        new TestData(100000.0012f,  "100000.00ms"),
    };

    [Test]
    public void ShowUnits([ValueSource("ShowUnitsValues")] TestData testData)
    {
        string output = displayUnits.ToString(testData.value, showUnits: true, limitToNDigits: 0, showFullValueWhenBelowZero: false);
        Assert.AreEqual(testData.expectedOutput, output);
    }

    static TestData[] LimitedTo5DigitsValues = new TestData[]
    {
        new TestData(0.00000001f,   "0.00"),
        new TestData(0.0009f,       "0.00"),
        new TestData(0.004f,        "0.00"),
        new TestData(0.005f,        "0.01"),
        new TestData(0.1012f,       "0.10"),
        new TestData(1.0012f,       "1.00"),
        new TestData(10.0012f,      "10.00"),
        new TestData(100.0012f,     "100.00"),
        new TestData(1000.0012f,    "1000.0"),
        new TestData(10000.0012f,   "10000"),
        new TestData(100000.0012f,  "100s"),
    };

    [Test]
    public void LimitedTo5Digits([ValueSource("LimitedTo5DigitsValues")] TestData testData)
    {
        string output = displayUnits.ToString(testData.value, showUnits: false, limitToNDigits: 5, showFullValueWhenBelowZero: false);
        Assert.AreEqual(testData.expectedOutput, output);
    }

    static TestData[] WithUnitsLimitedTo5DigitsValues = new TestData[]
    {
        new TestData(0.00000001f,   "0.00ms"),
        new TestData(0.0009f,       "0.00ms"),
        new TestData(0.004f,        "0.00ms"),
        new TestData(0.005f,        "0.01ms"),
        new TestData(0.1012f,       "0.10ms"),
        new TestData(1.0012f,       "1.00ms"),
        new TestData(10.0012f,      "10.0ms"),
        new TestData(100.0012f,     "100ms"),
        new TestData(1000.0012f,    "1.00s"),
        new TestData(10000.0012f,   "10.0s"),
        new TestData(100000.0012f,  "100s"),
    };

    [Test]
    public void WithUnitsLimitedTo5Digits([ValueSource("WithUnitsLimitedTo5DigitsValues")] TestData testData)
    {
        string output = displayUnits.ToString(testData.value, showUnits: true, limitToNDigits: 5, showFullValueWhenBelowZero: false);
        Assert.AreEqual(testData.expectedOutput, output);
    }

    static TestData[] WithUnitsLimitedTo5DigitsValuesAndShowFullValueWhenBelowZeroValues = new TestData[]
    {
        new TestData(0.00000001f,   "0.01ns"),
        new TestData(0.0009f,       "0.90us"),
        new TestData(0.004f,        "4.00us"),
        new TestData(0.005f,        "5.00us"),
        new TestData(0.1012f,       "0.1012ms"),
        new TestData(1.0012f,       "1.0012ms"),
        new TestData(10.0012f,      "10.001ms"),
        new TestData(100.0012f,     "100.00ms"),
        new TestData(1000.0012f,    "1000.0ms"),
        new TestData(10000.0012f,   "10000ms"),
        new TestData(100000.0012f,  "100.00s"),
    };

    [Test]
    public void WithUnitsLimitedTo5DigitsAndShowFullValueWhenBelowZero([ValueSource("WithUnitsLimitedTo5DigitsValuesAndShowFullValueWhenBelowZeroValues")] TestData testData)
    {
        string output = displayUnits.ToString(testData.value, showUnits: true, limitToNDigits: 5, showFullValueWhenBelowZero: true);
        Assert.AreEqual(testData.expectedOutput, output);
    }
}
