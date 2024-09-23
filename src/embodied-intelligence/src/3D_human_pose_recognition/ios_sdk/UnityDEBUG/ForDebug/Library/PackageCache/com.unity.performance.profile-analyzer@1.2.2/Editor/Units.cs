using System;
using System.Globalization;
using UnityEngine.Assertions;
using UnityEngine;

namespace UnityEditor.Performance.ProfileAnalyzer
{
    /// <summary>Unit type identifier</summary>
    internal enum Units
    {
        /// <summary>Time in milliseconds</summary>
        Milliseconds,
        /// <summary>Time in microseconds</summary>
        Microseconds,
        /// <summary>Count of number of instances</summary>
        Count,
    };

    internal class DisplayUnits
    {
        public static readonly string[] UnitNames =
        {
            "Milliseconds",
            "Microseconds",
            "Count",
        };

        public static readonly int[] UnitValues = (int[])Enum.GetValues(typeof(Units));

        public readonly Units Units;

        const bool kShowFullValueWhenBelowZero = true;
        const int kTooltipDigitsNumber = 7;

        public DisplayUnits(Units units)
        {
            Assert.AreEqual(UnitNames.Length, UnitValues.Length, "Number of UnitNames should match number of enum values UnitValues: You probably forgot to update one of them.");

            Units = units;
        }

        public string Postfix()
        {
            switch (Units)
            {
                default:
                case Units.Milliseconds:
                    return "ms";
                case Units.Microseconds:
                    return "us";
                case Units.Count:
                    return "";
            }
        }

        int ClampToRange(int value, int min, int max)
        {
            if (value < min)
                value = min;
            if (value > max)
                value = max;

            return value;
        }

        string RemoveTrailingZeros(string numberStr, int minNumberStringLength)
        {
            // Find out string length without trailing zeroes.
            var strLenWithoutTrailingZeros = numberStr.Length;
            while (strLenWithoutTrailingZeros > minNumberStringLength && numberStr[strLenWithoutTrailingZeros - 1] == '0')
                strLenWithoutTrailingZeros--;

            // Remove hanging '.' in case all zeroes can be omitted.
            if (strLenWithoutTrailingZeros > 0 && numberStr[strLenWithoutTrailingZeros - 1] == '.')
                strLenWithoutTrailingZeros--;


            return strLenWithoutTrailingZeros == numberStr.Length ? numberStr : numberStr.Substring(0, strLenWithoutTrailingZeros);
        }

        public string ToString(float ms, bool showUnits, int limitToNDigits, bool showFullValueWhenBelowZero = false)
        {
            float value = ms;
            int unitPower = -3;

            int minNumberStringLength = -1;
            int maxDecimalPlaces = 0;
            float minValueShownWhenUsingLimitedDecimalPlaces = 1f;
            switch (Units)
            {
                default:
                case Units.Milliseconds:
                    maxDecimalPlaces = 2;
                    minValueShownWhenUsingLimitedDecimalPlaces = 0.01f;
                    break;
                case Units.Microseconds:
                    value *= 1000f;
                    unitPower -= 3;
                    if (value < 100)
                    {
                        maxDecimalPlaces = 1;
                        minValueShownWhenUsingLimitedDecimalPlaces = 0.1f;
                    }
                    else
                    {
                        maxDecimalPlaces = 0;
                        minValueShownWhenUsingLimitedDecimalPlaces = 1f;
                    }
                    break;
                case Units.Count:
                    showUnits = false;
                    break;
            }

            int sgn = Math.Sign(value);
            if (value < 0)
                value = -value;
            int numberOfDecimalPlaces = maxDecimalPlaces;
            int unitsTextLength = showUnits ? 2 : 0;
            int signTextLength = sgn == -1 ? 1 : 0;
            if (limitToNDigits > 0 && value > float.Epsilon)
            {
                int numberOfSignificantFigures = limitToNDigits;
                if (!showFullValueWhenBelowZero)
                    numberOfSignificantFigures -= unitsTextLength + signTextLength;

                int valueExp = (int)Math.Log10(value);
                // Less than 1 values needs exponent correction as (int) rounds to the upper negative.
                if (value < 1)
                    valueExp -= 1;

                int originalUnitPower = unitPower;
                float limitRange = (float)Math.Pow(10, numberOfSignificantFigures);
                if (limitRange > 0)
                {
                    if (value >= limitRange)
                    {
                        while (value >= 1000f && unitPower < 9)
                        {
                            value /= 1000f;
                            unitPower += 3;
                            valueExp -= 3;
                        }
                    }
                    else if (showFullValueWhenBelowZero) // Only upscale and change unit type if we want to see exact number.
                    {
                        while (value < 0.01f && unitPower > -9)
                        {
                            value *= 1000f;
                            unitPower -= 3;
                            valueExp += 3;
                        }
                    }
                }

                if (unitPower != originalUnitPower)
                {
                    showUnits = true;
                    unitsTextLength = 2;
                    numberOfSignificantFigures = limitToNDigits;
                    if (!showFullValueWhenBelowZero)
                        numberOfSignificantFigures -= unitsTextLength + signTextLength;
                }

                // Use all allowed digits to display significant digits if we have any beyond maxDecimalPlaces
                int numberOfDigitsBeforeDecimalPoint = 1 + Math.Max(0, valueExp);
                if (showFullValueWhenBelowZero)
                {
                    numberOfDecimalPlaces = numberOfSignificantFigures - numberOfDigitsBeforeDecimalPoint;
                    minNumberStringLength = numberOfDigitsBeforeDecimalPoint + signTextLength + maxDecimalPlaces + 1;
                }
                else
                    numberOfDecimalPlaces = ClampToRange(numberOfSignificantFigures - numberOfDigitsBeforeDecimalPoint, 0, maxDecimalPlaces);
            }

            value *= sgn;

            string numberStr;
            if (value < minValueShownWhenUsingLimitedDecimalPlaces && showFullValueWhenBelowZero)
            {
                numberStr = string.Format(CultureInfo.InvariantCulture, "{0}", value);
            }
            else
            {
                string formatString = string.Concat("{0:f", numberOfDecimalPlaces, "}");
                numberStr = string.Format(CultureInfo.InvariantCulture, formatString, value);
            }

            // Remove trailing 0 if any from string
            if (minNumberStringLength > 0 && numberStr.Length > 0)
                numberStr = RemoveTrailingZeros(numberStr, minNumberStringLength);

            if (!showUnits)
                return numberStr;

            string siUnitString = GetSIUnitString(unitPower) + "s";
            return string.Concat(numberStr, siUnitString);
        }

        public static string GetSIUnitString(int unitPower)
        {
            // https://en.wikipedia.org/wiki/Metric_prefix
            switch (unitPower)
            {
                case -9:
                    return "n";
                case -6:
                    return "u";
                case -3:
                    return "m";
                case 0:
                    return "";
                case 3:
                    return "k";
                case 6:
                    return "M";
                case 9:
                    return "G";
            }

            return "?";
        }

        public string ToTooltipString(double ms, bool showUnits, int frameIndex = -1)
        {
            if (frameIndex >= 0)
                return string.Format("{0} on frame {1}", ToString((float)ms, showUnits, kTooltipDigitsNumber, kShowFullValueWhenBelowZero), frameIndex);

            return ToString((float)ms, showUnits, kTooltipDigitsNumber, kShowFullValueWhenBelowZero);
        }

        public GUIContent ToGUIContentWithTooltips(float ms, bool showUnits = false, int limitToNDigits = 5, int frameIndex = -1)
        {
            return new GUIContent(ToString(ms, showUnits, limitToNDigits), ToTooltipString(ms, true, frameIndex));
        }
    }
}
