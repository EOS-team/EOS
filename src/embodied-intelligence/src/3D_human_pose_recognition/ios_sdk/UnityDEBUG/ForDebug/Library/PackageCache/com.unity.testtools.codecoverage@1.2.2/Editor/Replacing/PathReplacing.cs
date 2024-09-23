using System;
using System.Collections.Generic;
using System.Text.RegularExpressions;
using UnityEditor.TestTools.CodeCoverage.Utils;
using UnityEngine;

namespace UnityEditor.TestTools.CodeCoverage
{
    internal class PathReplacing
    {
        private readonly Dictionary<Regex, string> m_PathReplacePatterns;
        private bool m_HasPathReplacePatterns;

        public PathReplacing()
        {
            m_PathReplacePatterns = new Dictionary<Regex, string>();
        }

        public void Parse(string replacePatterns)
        {
            string[] replacePatternsArray = replacePatterns.Split(',');

            // There should be at least one pair 
            m_HasPathReplacePatterns = replacePatternsArray.Length > 1;

            if (m_HasPathReplacePatterns)
            {
                // If an odd number of elements is passed trim the last element
                int evenArrayLength = replacePatternsArray.Length % 2 == 0 ? replacePatternsArray.Length : replacePatternsArray.Length - 1;

                for (int i = 0; i < evenArrayLength; i = i + 2)
                {
                    m_PathReplacePatterns.Add(CreateFilterRegex(replacePatternsArray[i]), replacePatternsArray[i + 1]);
                }
            }
        }

        public string ReplacePath(string path)
        {
            if (m_HasPathReplacePatterns)
            {
                string newPath = CoverageUtils.NormaliseFolderSeparators(path);

                foreach (Regex replacePattern in m_PathReplacePatterns.Keys)
                {
                    Match match = replacePattern.Match(newPath);
                    if (match.Success)
                        path = path.Replace(match.Value, m_PathReplacePatterns[replacePattern]);
                }
            }

            return path;
        }

        Regex CreateFilterRegex(string filter)
        {
            filter = CoverageUtils.NormaliseFolderSeparators(filter);
          
            return new Regex(CoverageUtils.GlobToRegex(filter, false), RegexOptions.Compiled | RegexOptions.IgnoreCase);
        }
    }
}
