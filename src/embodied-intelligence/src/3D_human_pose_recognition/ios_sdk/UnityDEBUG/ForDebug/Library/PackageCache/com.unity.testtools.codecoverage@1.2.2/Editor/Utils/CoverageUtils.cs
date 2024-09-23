using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using System.Text;
using UnityEditor.Networking.PlayerConnection;
using UnityEngine;
using UnityEngine.TestTools;

namespace UnityEditor.TestTools.CodeCoverage.Utils
{
    internal static class CoverageUtils
    {
        public static bool IsConnectedToPlayer
        {
            get
            {
                return EditorConnection.instance.ConnectedPlayers.Count > 0;
            }
        }

        public static string NormaliseFolderSeparators(string folderPath, bool stripTrailingSlash = false)
        {
            if (folderPath != null)
            {
                folderPath = folderPath.Replace('\\', '/');
                if (stripTrailingSlash)
                {
                    folderPath = folderPath.TrimEnd('/');
                }
            }

            return folderPath;
        }

        public static bool EnsureFolderExists(string folderPath)
        {
            if (string.IsNullOrEmpty(folderPath))
                return false;

            if (!Directory.Exists(folderPath))
            {
                try
                {
                    Directory.CreateDirectory(folderPath);
                }
                catch (Exception)
                {
                    return false;
                }
            }

            return true;
        }

        public static string GetProjectFolderName()
        {
            string[] projectPathArray = GetProjectPath().Split('/');

            Debug.Assert(projectPathArray.Length > 0);

            string folderName = projectPathArray[projectPathArray.Length - 1];

            char[] invalidChars = Path.GetInvalidPathChars();
            StringBuilder folderNameStringBuilder = new StringBuilder();
            foreach (char c in folderName)
            {
                if (invalidChars.Contains(c))
                {
                    folderNameStringBuilder.Append('_');
                }
                else
                {
                    folderNameStringBuilder.Append(c);
                }
            }

            return folderNameStringBuilder.ToString();
        }

        public static string StripAssetsFolderIfExists(string folderPath)
        {
            if (folderPath != null)
            {
                string toTrim = "Assets";
                folderPath = folderPath.TrimEnd(toTrim.ToCharArray());
            }

            return folderPath;
        }

        public static string GetProjectPath()
        {
            return NormaliseFolderSeparators(StripAssetsFolderIfExists(Application.dataPath), true);
        }

        public static string GetRootFolderPath(CoverageSettings coverageSettings)
        {
            string rootFolderPath = string.Empty;
            string coverageFolderPath = string.Empty;

            if (CommandLineManager.instance.batchmode && !CommandLineManager.instance.useProjectSettings)
            {
                if (coverageSettings.resultsPathFromCommandLine.Length > 0)
                {
                    coverageFolderPath = coverageSettings.resultsPathFromCommandLine;
                    EnsureFolderExists(coverageFolderPath);
                }
            }
            else
            {
                if (CommandLineManager.instance.runFromCommandLine && coverageSettings.resultsPathFromCommandLine.Length > 0)
                {
                    coverageFolderPath = coverageSettings.resultsPathFromCommandLine;
                    EnsureFolderExists(coverageFolderPath);
                }
                else
                {
                    coverageFolderPath = CoveragePreferences.instance.GetStringForPaths("Path", string.Empty);
                }
            }

            string projectPath = GetProjectPath();

            if (EnsureFolderExists(coverageFolderPath))
            {
                coverageFolderPath = NormaliseFolderSeparators(coverageFolderPath, true);

                // Add 'CodeCoverage' directory if coverageFolderPath is projectPath
                if (string.Equals(coverageFolderPath, projectPath, StringComparison.InvariantCultureIgnoreCase))
                    rootFolderPath = JoinPaths(coverageFolderPath, coverageSettings.rootFolderName);
                // else user coverageFolderPath as the root folder
                else
                    rootFolderPath = coverageFolderPath;
            }
            else
            {
                // Add 'CodeCoverage' directory to projectPath if coverageFolderPath is not valid
                rootFolderPath = JoinPaths(projectPath, coverageSettings.rootFolderName);
            }
            return rootFolderPath;
        }

        public static string GetHistoryFolderPath(CoverageSettings coverageSettings)
        {
            string historyFolderPath = string.Empty;
            string rootFolderPath = coverageSettings.rootFolderPath;

            if (CommandLineManager.instance.batchmode && !CommandLineManager.instance.useProjectSettings)
            {
                if (coverageSettings.historyPathFromCommandLine.Length > 0)
                {
                    historyFolderPath = coverageSettings.historyPathFromCommandLine;
                    EnsureFolderExists(historyFolderPath);
                }
            }
            else
            {
                if (CommandLineManager.instance.runFromCommandLine && coverageSettings.historyPathFromCommandLine.Length > 0)
                {
                    historyFolderPath = coverageSettings.historyPathFromCommandLine;
                    EnsureFolderExists(historyFolderPath);
                }
                else
                {
                    historyFolderPath = CoveragePreferences.instance.GetStringForPaths("HistoryPath", string.Empty);
                }
            }

            bool addHistorySubDir = false;
            string projectPath = GetProjectPath();

            if (EnsureFolderExists(historyFolderPath))
            {
                historyFolderPath = NormaliseFolderSeparators(historyFolderPath, true);
                
                // If historyFolderPath == rootFolderPath, add 'Report-history' sub directory in rootFolderPath 
                if (string.Equals(historyFolderPath, rootFolderPath, StringComparison.InvariantCultureIgnoreCase))
                {
                    addHistorySubDir = true;
                }
                // If historyFolderPath == projectPath, add 'CodeCoverage' directory to projectPath
                // and add 'Report-history' sub directory in rootFolderPath 
                else if (string.Equals(historyFolderPath, projectPath, StringComparison.InvariantCultureIgnoreCase))
                {
                    rootFolderPath = JoinPaths(projectPath, coverageSettings.rootFolderName);
                    addHistorySubDir = true;
                }
                // otherwise keep the original historyFolderPath
            }
            else
            {
                // If historyFolderPath is not valid, add 'CodeCoverage' directory to projectPath
                // and add 'Report-history' sub directory in rootFolderPath
                rootFolderPath = JoinPaths(projectPath, coverageSettings.rootFolderName);
                addHistorySubDir = true;
            }

            if (addHistorySubDir)
            {
                historyFolderPath = JoinPaths(rootFolderPath, CoverageSettings.ReportHistoryFolderName);
            }

            return historyFolderPath;
        }

        public static string JoinPaths(string pathLeft, string pathRight)
        {
            string[] pathsToJoin = new string[] { pathLeft, pathRight };
            return string.Join("/", pathsToJoin);
        }

        public static int GetNumberOfFilesInFolder(string folderPath, string filePattern, SearchOption searchOption)
        {
            if (folderPath != null && Directory.Exists(folderPath))
            {
                string[] files = Directory.GetFiles(folderPath, filePattern, searchOption);
                return files.Length;
            }

            return 0;
        }

        public static void ClearFolderIfExists(string folderPath, string filePattern)
        {
            if (folderPath != null && Directory.Exists(folderPath))
            {
                DirectoryInfo dirInfo = new DirectoryInfo(folderPath);

                foreach (FileInfo file in dirInfo.GetFiles(filePattern))
                {
                    try
                    {
                        file.Delete();
                    }
                    catch (Exception)
                    {
                        ResultsLogger.Log(ResultID.Warning_FailedToDeleteFile, file.FullName);
                    }
                }

                foreach (DirectoryInfo dir in dirInfo.GetDirectories())
                {
                    try
                    {
                        dir.Delete(true);
                    }
                    catch (Exception)
                    {
                        ResultsLogger.Log(ResultID.Warning_FailedToDeleteDir, dir.FullName);
                    }
                }
            }
        }

        public static bool IsValidFolder(string folderPath)
        {
            return !string.IsNullOrEmpty(folderPath) && Directory.Exists(folderPath);
        }

        public static bool IsValidFile(string filePath)
        {
            return !string.IsNullOrEmpty(filePath) && File.Exists(filePath);
        }

        private static HashSet<char> regexSpecialChars = new HashSet<char>(new[] { '[', '\\', '^', '$', '.', '|', '?', '*', '+', '(', ')' });

        public static string GlobToRegex(string glob, bool startEndConstrains = true)
        {
            var regex = new StringBuilder();
            var characterClass = false;
            char prevChar = Char.MinValue;

            if (startEndConstrains)
                regex.Append("^");
            foreach (var c in glob)
            {
                if (characterClass)
                {
                    if (c == ']')
                    {
                        characterClass = false;
                    }
                    regex.Append(c);
                    continue;
                }
                switch (c)
                {
                    case '*':
                        if (prevChar == '*')
                            regex.Append(".*"); //if it's double * pattern then don't stop at folder separator
                        else
                            regex.Append("[^\\n\\r/]*"); //else match everything except folder separator (and new line)
                        break;
                    case '?':
                        regex.Append("[^\\n\\r/]");
                        break;
                    case '[':
                        characterClass = true;
                        regex.Append(c);
                        break;
                    default:
                        if (regexSpecialChars.Contains(c))
                        {
                            regex.Append('\\');
                        }
                        regex.Append(c);
                        break;
                }

                prevChar = c;
            }
            if (startEndConstrains)
                regex.Append("$");
            return regex.ToString();
        }

        public static string[] GetFilteringLogParams(AssemblyFiltering assemblyFiltering, PathFiltering pathFiltering, string[] otherParams = null)
        {
            string[] logParams = { assemblyFiltering != null && assemblyFiltering.includedAssemblies.Length > 0 ? assemblyFiltering.includedAssemblies : "<Not specified>",
                    assemblyFiltering != null && assemblyFiltering.excludedAssembliesNoDefault.Length > 0 ? assemblyFiltering.excludedAssembliesNoDefault : "<Not specified>",
                    pathFiltering != null && pathFiltering.includedPaths.Length > 0 ? pathFiltering.includedPaths : "<Not specified>",
                    pathFiltering != null && pathFiltering.excludedPaths.Length > 0 ? pathFiltering.excludedPaths : "<Not specified>" };

            if (otherParams != null && otherParams.Length > 0)
                logParams = otherParams.Concat(logParams).ToArray();

            return logParams;
        }

        [ExcludeFromCoverage]
        public static string BrowseForDir(string directory, string title)
        {
            if (string.IsNullOrEmpty(directory))
            {
                string variable = "ProgramFiles";
#if UNITY_EDITOR_OSX
                variable = "HOME";
#endif
                string candidateDirectory = Environment.GetEnvironmentVariable(variable);
                if (IsValidFolder(candidateDirectory))
                    directory = candidateDirectory;
            }

            directory = EditorUtility.OpenFolderPanel(title, directory, string.Empty);

            EditorWindow.FocusWindowIfItsOpen(typeof(CodeCoverageWindow));

            if (!IsValidFolder(directory))
                return string.Empty;

            return directory;
        }

        [ExcludeFromCoverage]
        public static string BrowseForFile(string directory, string title)
        {
            if (string.IsNullOrEmpty(directory))
            {
                string variable = "ProgramFiles";
#if UNITY_EDITOR_OSX
                variable = "HOME";
#endif
                string candidateDirectory = Environment.GetEnvironmentVariable(variable);
                if (IsValidFolder(candidateDirectory))
                    directory = candidateDirectory;
            }

            string file = EditorUtility.OpenFilePanel(title, directory, "cs");

            EditorWindow.FocusWindowIfItsOpen(typeof(CodeCoverageWindow));

            if (!IsValidFile(file))
                return string.Empty;

            return file;
        }
    }
}
