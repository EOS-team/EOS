using System;

namespace UnityEditor.TestTools.CodeCoverage.Analytics
{
    [Serializable]
    internal class CoverageAnalyticsEvent
    {
        // The action performed on the event (batchmode compatible)
        public ActionID actionID;
        // Array of resultsIds (batchmode compatible)
        public int[] resultIds;
        // The coverage mode that was performed on successful action (batchmode compatible)
        public CoverageModeID coverageModeId;
        // Duration (in ms) for which the Coverage session lasted (batchmode compatible)
        public long duration;
        // Was the coverage session result a success (batchmode compatible)
        public bool success;
        // Did the user run the editor from the command line
        public bool runFromCommandLine;
        // Did the user run the editor in batch mode
        public bool batchmode;
        // Did the user pass the useProjectSettings option in batch mode (batchmode only)
        public bool useProjectSettings;
        // Did the user have Generate HTML Report selected  (batchmode compatible)
        public bool generateHTMLReport;
        // Did the user have Generate History selected (batchmode compatible)
        public bool generateHistory;
        // Did the user have Generate Badges selected (batchmode compatible)
        public bool createBadges;
        // Did the user have Generate Additional Metrics selected (batchmode compatible)
        public bool generateMetrics;
        // Did the user have Generate Test Runner References selected (batchmode compatible)
        public bool generateTestReferences;
        // Did the user have Generate Additional reports selected (batchmode compatible)
        public bool generateAdditionalReports;
        // Did the user have passed the dontClear coverage option (batchmode compatible)
        public bool dontClear;
        // Did the user have Auto Generate Report selected (batchmode compatible)
        public bool autogenerate;
        // Did the user have Auto Open Report selected
        public bool autoOpenReport;
        // Did the user select the Clear Data button
        public bool clearData;
        // Did the user select the Clear History button
        public bool clearHistory;
        // Did the user select the Generate From Last button
        public bool generateFromLast;
        // Did the user switch to Debug mode (Code Optimization) in the Coverage window
        public bool switchToDebugMode;
        // Is the editor in Code Optimization: Debug mode (batchmode compatible)
        public bool inDebugMode;
        // Did the user disable Burst Compilation in the Coverage window
        public bool switchBurstOff;
        // Did the user select a new Results location or uses the default one (batchmode compatible)
        public bool useDefaultResultsLoc;
        // Did the user select a new History location or uses the default one (batchmode compatible)
        public bool useDefaultHistoryLoc;
        // Did the user specify different assemblies from the default ones (batchmode compatible)
        public bool useDefaultAssemblyFilters;
        // Did the user specify different paths filtering from the default one (batchmode compatible)
        public bool useDefaultPathFilters;
        // Did the user enter the Selected Assemblies dialog/dropdown
        public bool enterAssembliesDialog;
        // Did the user update any assemblies via the Selected Assemblies dialog/dropdown
        public bool updateAssembliesDialog;
        // Did the user update Included Paths
        public bool updateIncludedPaths;
        // Did the user select Add Folder for Included Paths
        public bool selectAddFolder_IncludedPaths;
        // Did the user select Add File for Included Paths
        public bool selectAddFile_IncludedPaths;
        // How many paths are included (batchmode compatible)
        public int numOfIncludedPaths;
        // Did the user update Excluded Paths
        public bool updateExcludedPaths;
        // Did the user select Add Folder for Excluded Paths
        public bool selectAddFolder_ExcludedPaths;
        // Did the user select Add File for Excluded Paths
        public bool selectAddFile_ExcludedPaths;
        // How many paths are excluded (batchmode compatible)
        public int numOfExcludedPaths;
        // Did the user use the Coverage API to StartRecording (batchmode compatible)
        public bool useAPI_StartRec;
        // Did the user use the Coverage API to StopRecording (batchmode compatible)
        public bool useAPI_StopRec;
        // Did the user use the Coverage API to PauseRecording (batchmode compatible)
        public bool useAPI_PauseRec;
        // Did the user use the Coverage API to UnpauseRecording (batchmode compatible)
        public bool useAPI_UnpauseRec;
        // Array of individual included assembly names (batchmode compatible)
        public string[] includedAssemblies;
        // Array of individual excluded assembly names (batchmode only)
        public string[] excludedAssemblies;
        // Did the user use the onCoverageSessionStarted event (batchmode compatible)
        public bool useEvent_onCoverageSessionStarted;
        // Did the user use the onCoverageSessionFinished event (batchmode compatible)
        public bool useEvent_onCoverageSessionFinished;
        // Did the user use the onCoverageSessionPaused event (batchmode compatible)
        public bool useEvent_onCoverageSessionPaused;
        // Did the user use the onCoverageSessionUnpaused event (batchmode compatible)
        public bool useEvent_onCoverageSessionUnpaused;
        // Did the user specify path replace patterns (command line only)
        public bool usePathReplacePatterns;
        // Did the user specify source paths (command line only)
        public bool useSourcePaths;
        // Did the user specify path filters from file option (command line only)
        public bool usePathFiltersFromFile;
    }
}
