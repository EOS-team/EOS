using System;
using System.Linq;
using System.Text.RegularExpressions;
using UnityEditor.TestTools.CodeCoverage.Analytics;
using UnityEditor.TestTools.CodeCoverage.Utils;

namespace UnityEditor.TestTools.CodeCoverage
{
    internal class PathFiltering
    {
        public string includedPaths
        {
            get;
            private set;
        }

        public string excludedPaths
        {
            get;
            private set;
        }

        private Regex[] m_IncludePaths;
        private Regex[] m_ExcludePaths;

        private bool m_HasIncludePaths;
        private bool m_HasExcludePaths;

        public PathFiltering()
        {
            m_IncludePaths = new Regex[] { };
            m_ExcludePaths = new Regex[] { };
        }

        public void Parse(string includePaths, string excludePaths)
        {
            includedPaths = includePaths;
            excludedPaths = excludePaths;

            string[] includePathFilters = includePaths.Split(new[] { ',' }, StringSplitOptions.RemoveEmptyEntries);
            string[] excludePathFilters = excludePaths.Split(new[] { ',' }, StringSplitOptions.RemoveEmptyEntries);

            m_IncludePaths = includePathFilters
                .Where(f => f != "-")
                .Select(f => CreateFilterRegex(f))
                .ToArray();

            m_ExcludePaths = excludePathFilters
                .Where(f => f != "-")
                .Select(f => CreateFilterRegex(f))
                .ToArray();

            CoverageAnalytics.instance.CurrentCoverageEvent.numOfIncludedPaths = m_IncludePaths.Length;
            CoverageAnalytics.instance.CurrentCoverageEvent.numOfExcludedPaths = m_ExcludePaths.Length;

            m_HasIncludePaths = m_IncludePaths.Length > 0;
            m_HasExcludePaths = m_ExcludePaths.Length > 0;
        }

        public bool IsPathIncluded(string name)
        {
            if (!m_HasIncludePaths && !m_HasExcludePaths)
                return true;

            name = name.ToLowerInvariant();
            name = CoverageUtils.NormaliseFolderSeparators(name, true);

            if (m_ExcludePaths.Any(f => f.IsMatch(name)))
            {
                return false;
            }
            else
            {
                return !m_HasIncludePaths || m_IncludePaths.Any(f => f.IsMatch(name));
            }
        }

        Regex CreateFilterRegex(string filter)
        {
            filter = filter.ToLowerInvariant();
            filter = CoverageUtils.NormaliseFolderSeparators(filter, true);
          
            return new Regex(CoverageUtils.GlobToRegex(filter), RegexOptions.Compiled);
        }
    }
}
