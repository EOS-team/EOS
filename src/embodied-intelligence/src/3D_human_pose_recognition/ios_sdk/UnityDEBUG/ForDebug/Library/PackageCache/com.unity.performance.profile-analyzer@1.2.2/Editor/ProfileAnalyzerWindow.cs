using System;
using System.Collections.Generic;
using System.Diagnostics;
using System.IO;
using System.Linq;
using System.Text.RegularExpressions;
using System.Threading;
using UnityEditor.IMGUI.Controls;
using UnityEditorInternal;
using UnityEngine;
using UnityEngine.Profiling;
using Debug = UnityEngine.Debug;
using ProfilerCommon = UnityEditorInternal.ProfilerDriver;
using ProfilerMarkerAbstracted = Unity.Profiling.ProfilerMarker;

namespace UnityEditor.Performance.ProfileAnalyzer
{
    enum ThreadRange
    {
        Median,
        UpperQuartile,
        Max
    };

    enum ActiveTab
    {
        Summary,
        Compare,
    };

    enum ActiveView
    {
        Single,
        Left,
        Right
    }

    enum ThreadActivity
    {
        None,
        Analyze,
        AnalyzeDone,
        Compare,
        CompareDone,
        Load,
        LoadDone
    };

    enum TopTenDisplay
    {
        Normalized,
        LongestTime,
    };

    enum NameFilterOperation
    {
        All,    // AND
        Any,    // OR
    };

    enum RemoveMarkerOperation
    {
        ShowAll,
        HideWaitForFPS,
        HideWaitForPresent,
        Custom,
    };

    /// <summary>
    /// Main profile Analyzer UI window
    /// </summary>
    public class ProfileAnalyzerWindow : EditorWindow
    {
        internal static class Styles
        {
            public static readonly GUIContent emptyString = new GUIContent("", "");
            public static readonly GUIContent dash = new GUIContent("-", "");
            public static readonly GUIContent thread = new GUIContent("Thread", "");
            public static readonly GUIContent noThread = new GUIContent("", "Thread not present on this data set");

            public static readonly GUIContent max = new GUIContent("Max", "The peak value in the data set");
            public static readonly GUIContent upperQuartile = new GUIContent("Upper Quartile", "The middle value between the median and the highest value of the data set. I.e. at 75% of the ordered data.");
            public static readonly GUIContent mean = new GUIContent("Mean", "The average value in the data set");
            public static readonly GUIContent median = new GUIContent("Median", "The central value in the data set");
            public static readonly GUIContent lowerQuartile = new GUIContent("Lower Quartile", "The middle number between the smallest number and the median of the data set. I.e. at 25% of the ordered data.");
            public static readonly GUIContent min = new GUIContent("Min", "The minimum value in the data set");
            public static readonly GUIContent individualMin = new GUIContent("Individual Min", "The minimum value in the data set for an individual marker instance (not the total in the frame)");
            public static readonly GUIContent individualMax = new GUIContent("Individual Max", "The maximum value in the data set for an individual marker instance (not the total in the frame)");

            public static readonly GUIContent export = new GUIContent("Export", "Export profiler data as CSV files");
            public static readonly GUIContent pullOpen = new GUIContent("Pull Data", "Pull data from Unity profiler.\nFirst you must open Unity profiler to pull data from it");
            public static readonly GUIContent pullRange = new GUIContent("Pull Data", "Pull data from Unity profiler.\nFirst you must use the Unity profiler to capture data from application");
            public static readonly GUIContent pullRecording = new GUIContent("Pull Data", "Pull data from Unity profiler.\nStop Unity profiler recording to enable pulling data");
            public static readonly GUIContent pull = new GUIContent("Pull Data", "Pull data from Unity profiler");
            public static readonly GUIContent nameFilter = new GUIContent("Name Filter : ", "Only show markers containing the strings\n\n(Effects the marker table below)");
            public static readonly GUIContent nameExclude = new GUIContent("Exclude Names : ", "Excludes markers containing the strings\n\n(Effects the marker table below)");
            public static readonly GUIContent threadFilter = new GUIContent("Thread : ", "Select threads to focus on\n\n(Effects the marker table below)");
            public static readonly GUIContent threadFilterSelect = new GUIContent("Select", "Select threads to focus on\n\n(Effects the marker table below)");
            public static readonly GUIContent unitFilter = new GUIContent("Units : ", "Units to show in UI");
            public static readonly GUIContent timingFilter = new GUIContent("Analysis Type : ", TimingOptions.Tooltip);
            public static readonly GUIContent markerColumns = new GUIContent("Marker Columns : ", "Set of Columns to show in the table");
            public static readonly GUIContent graphPairing = new GUIContent("Pair Graph Selection", "Selections on one graph will affect the other");
            public static readonly GUIContent removeMarker = new GUIContent("Remove : ", "Remove a specific marker from time analysis\n\n(Effects all views)");
            public static readonly GUIContent hideRemoveMarkers = new GUIContent("Hide Removed Markers", "Hide removed markers from the marker table");

            public static readonly GUIContent frameSummary = new GUIContent("Frame Summary", "");
            public static readonly GUIContent frameCount = new GUIContent("Frame Count", "Frame Count");
            public static readonly GUIContent frameStart = new GUIContent("Start", "Frame Start");
            public static readonly GUIContent frameEnd = new GUIContent("End", "Frame End");
            public static readonly GUIContent threadSummary = new GUIContent("Thread Summary", "");
            public static readonly GUIContent threadGraphScale = new GUIContent("Graph Scale : ", "");
            public static readonly GUIContent[] threadRanges =
            {
                new GUIContent("Median", "Median frame time"),
                new GUIContent("Upper quartile", "Upper quartile of frame time"),
                new GUIContent("Max", "Max frame time")
            };

            public static readonly GUIContent markerSummary = new GUIContent("Marker Summary", "");
            public static readonly GUIContent filters = new GUIContent("Filters", "");
            public static readonly GUIContent profileTable = new GUIContent("Marker Details for currently selected range", "");
            public static readonly GUIContent comparisonTable = new GUIContent("Marker Comparison for currently selected range", "");

            public static readonly GUIContent depthTitle = new GUIContent("Depth Slice : ", "Marker callstack depth to analyze");
            public static readonly GUIContent leftDepthTitle = new GUIContent("Left : ", "Marker callstack depth to analyze");
            public static readonly GUIContent rightDepthTitle = new GUIContent("Right : ", "Marker callstack depth to analyze");
            public static readonly string autoDepthTitleText = "Auto Depth (Diff: {0:+##;-##;None})";
            public static readonly GUIContent autoDepthTitle = new GUIContent("Auto Depth", "Match up the depth levels based on the most common difference between markers present in both data sets. If the selected depth is at a depth not present in the other data set, after applying this difference, it will use the deepest level.");
            public static readonly GUIContent parentMarker = new GUIContent("Parent Marker : ", "Marker to start analysis from.\nParent of the hierarchy to analyze.");
            public static readonly GUIContent selectParentMarker = new GUIContent("None", "Select using right click context menu on marker names in marker table");

            public static readonly GUIContent topMarkerRatio = new GUIContent("Ratio : ", "Normalize\tNormalized to time of the individual set\nLongest\tRatio based on longest time of the two");

            public static readonly GUIContent firstFrame = new GUIContent("First frame", "");

            public static readonly GUIContent[] topTenDisplayOptions =
            {
                new GUIContent("Normalized", "Ratio normalized to time of the individual data set"),
                new GUIContent("Longest", "Ratio based on longest time of the two data sets")
            };

            public static readonly GUIContent[] nameFilterOperation =
            {
                new GUIContent("All", "Marker name contains all strings"),
                new GUIContent("Any", "Marker name contains any of the strings")
            };

            public static readonly GUIContent[] removeMarkerOperation =
            {
                new GUIContent("None", "All markers shown. None removed."),
                new GUIContent("FPS Wait", "Remove the WaitForTargetFPS marker which represents targeted FPS (*) time\n\n(*) Via Application.targetFrameRate API usage (common on Mobile)"),
                new GUIContent("Present Wait", "Remove the Gfx.WaitForPresentOnGfxThread marker which represents time waiting for GPU to complete (common on consoles)"),
                new GUIContent("Custom", "Right click on a marker in the table and select 'Remove Marker' to remove a specific marker (and all children too)")
            };

            public static readonly GUIContent menuItemSelectFramesInAll = new GUIContent("Select Frames that contain this marker (within whole data set)", "");
            public static readonly GUIContent menuItemSelectFramesInCurrent = new GUIContent("Select Frames that contain this marker (within current selection)", "");
            public static readonly GUIContent menuItemSelectFramesAll = new GUIContent("Clear Selection", "");

            public static readonly GUIContent frameCosts = new GUIContent(" by frame costs", "Contains accumulated marker cost within the frame");
            public static readonly GUIContent dataMissing = new GUIContent("Pull or load a data set for analysis", "Pull data from Unity Profiler or load a previously saved analysis data set");
            public static readonly GUIContent comparisonDataMissing = new GUIContent("Pull or load a data set for comparison", "Pull data from Unity Profiler or load previously saved analysis data sets");

            public static readonly string topMarkersTooltip = "Top markers for the median frame.\nThe length of this frame is the median of those in the data set.\nIt is likely to be the most representative frame.";
            public static readonly string medianFrameTooltip = "The length of this frame is the median of those in the data set.\nIt is likely to be the most representative frame.";

            public static readonly string helpText =
@"This tool can analyze Unity Profiler data, to find representative frames and perform comparisons of data sets.

To gather data to analyze:
* Open the Unity Profiler. Either via the Unity menu under 'Windows', 'Analysis' or via the 'Open Profile Window' in the tool bar.
* Capture some profiling data in the Unity Profiler by selecting a target application and click the 'Record' button.
* Stop the capture by clicking again on the 'Record' button.

To analyze the data:
* Pull the Unity Profiler data into this tool by clicking the 'Pull Data' button in the single or compare views.
* The analysis will be automatically triggered (in the compare view two data sets are required before analysis is performed).
* Select a marker to see more detailed information about its time utilization over the frame time range.
* Save off a data file from here to keep for future use. (Recommend saving the profile .data file in the same folder).

To compare two data sets:
* Click the compare tab. The data in the single tab will be used by default. You can also load previously saved analysis data.
* Drag select a region in the frame time graph (above) to choose 1 or more frames for each of the two data sets.
* The comparison will be automatically triggered as the selection is made.";
        }

        const float k_ProgressBarHeight = 2f;

        ProgressBarDisplay m_ProgressBar;
        ProfileAnalyzer m_ProfileAnalyzer;

        ProfilerWindowInterface m_ProfilerWindowInterface;
        string m_LastProfilerSelectedMarker;

        string m_LastMarkerSuccesfullySyncedWithProfilerWindow = null;

        [NonSerialized] bool m_SelectionEventFromProfilerWindowInProgress = false;

        int m_TopNumber;
        string[] m_TopStrings;
        int[] m_TopValues;

        [SerializeField] DepthSliceUI m_DepthSliceUI;

        [SerializeField]
        TimingOptions.TimingOption m_TimingOption = TimingOptions.TimingOption.Time;

        [SerializeField]
        string m_ParentMarker = null;

        List<string> m_ThreadUINames = new List<string>();
        List<string> m_ThreadNames = new List<string>();
        Dictionary<string, string> m_ThreadNameToUIName;
        GUIContent[] m_removeMarkerDisplay = null;
        int[] m_removeMarkerValues = null;
        bool m_removeMarkerSomeMissing = false;

        struct MarkerFilter
        {
            public Dictionary<string, bool> MarkerCache;
            public bool NeedsRebuild;
            public List<string> IncludeFilter;
            public List<string> ExcludeFilter;
            public NameFilterOperation IncludeOperation;
            public NameFilterOperation ExcludeOperation;

            public void Clear()
            {
                MarkerCache.Clear();
                NeedsRebuild = true;
            }

            private void Rebuild(ProfileAnalyzerWindow window)
            {
                NeedsRebuild = false;
                IncludeFilter = window.GetNameFilters();
                ExcludeFilter = window.GetNameExcludes();
                IncludeOperation = window.m_NameFilterOperation;
                ExcludeOperation = window.m_NameExcludeOperation;
            }

            private bool ComputeFilter(string marker)
            {
                if (IncludeFilter.Count > 0)
                {
                    if (!NameInFilterList(marker, IncludeFilter, IncludeOperation))
                        return false;
                }
                if (ExcludeFilter.Count > 0)
                {
                    if (NameInFilterList(marker, ExcludeFilter, ExcludeOperation))
                        return false;
                }
                return true;
            }

            public bool DoesMarkerPassFilter(ProfileAnalyzerWindow window, string marker)
            {
                if (NeedsRebuild)
                    Rebuild(window);

                if (MarkerCache.TryGetValue(marker, out var value))
                    return value;
                bool passesFilter = ComputeFilter(marker);
                MarkerCache[marker] = passesFilter;
                return passesFilter;
            }
        }
        MarkerFilter m_MarkerFilter;

        [SerializeField]
        ThreadSelection m_ThreadSelection = new ThreadSelection();
        ThreadSelection m_ThreadSelectionNew;
        string m_ThreadSelectionSummary;

        [SerializeField]
        DisplayUnits m_DisplayUnits = new DisplayUnits(Units.Milliseconds);
        string[] m_UnitNames;

        [SerializeField]
        string m_NameFilter = "";
        [SerializeField]
        string m_NameExclude = "";

        [SerializeField]
        MarkerColumnFilter m_SingleModeFilter = new MarkerColumnFilter(MarkerColumnFilter.Mode.TimeAndCount);
        [SerializeField]
        MarkerColumnFilter m_CompareModeFilter = new MarkerColumnFilter(MarkerColumnFilter.Mode.TimeAndCount);

        [SerializeField]
        TopTenDisplay m_TopTenDisplay = TopTenDisplay.Normalized;
        [SerializeField]
        NameFilterOperation m_NameFilterOperation = NameFilterOperation.All;
        [SerializeField]
        NameFilterOperation m_NameExcludeOperation = NameFilterOperation.Any;
        [SerializeField]
        RemoveMarkerOperation m_removeMarkerOperation = RemoveMarkerOperation.ShowAll;
        [SerializeField]
        bool m_hideRemovedMarkers = true;

        int m_ProfilerFirstFrameIndex = 0;
        int m_ProfilerLastFrameIndex = 0;
        const int k_ProfileDataDefaultDisplayOffset = 1;

        ActiveTab m_NextActiveTab = ActiveTab.Summary;
        ActiveTab m_ActiveTab = ActiveTab.Summary;
        bool m_OtherTabDirty = false;
        bool m_OtherTableDirty = false;

        [SerializeField]
        string m_removeMarkerCustomRemoveMarker = null;

        [SerializeField]
        bool m_ShowFilters = true;
        [SerializeField]
        bool m_ShowTopNMarkers = true;
        [SerializeField]
        bool m_ShowFrameSummary = true;
        [SerializeField]
        bool m_ShowThreadSummary = false;
        [SerializeField]
        bool m_ShowMarkerSummary = true;
        [SerializeField]
        bool m_ShowMarkerTable = true;

        internal static class UIColor
        {
            static internal Color Color256(int r, int g, int b, int a)
            {
                return new Color((float)r / 255.0f, (float)g / 255.0f, (float)b / 255.0f, (float)a / 255.0f);
            }

            public static readonly Color white = new UnityEngine.Color(1.0f, 1.0f, 1.0f);
            public static readonly Color barBackground = new Color(0.5f, 0.5f, 0.5f);
            public static readonly Color barBackgroundSelected = new Color(0.6f, 0.6f, 0.6f);
            public static readonly Color boxAndWhiskerBoxColor = Color256(112, 112, 112, 255);
            public static readonly Color boxAndWhiskerLineColorLeft = Color256(206, 219, 238, 255);
            public static readonly Color boxAndWhiskerBoxColorLeft = Color256(59, 104, 144, 255);
            public static readonly Color boxAndWhiskerLineColorRight = Color256(247, 212, 201, 255);
            public static readonly Color boxAndWhiskerBoxColorRight = Color256(161, 83, 30, 255);
            public static readonly Color bar = new Color(0.95f, 0.95f, 0.95f);
            public static readonly Color barSelected = new Color(0.5f, 1.0f, 0.5f);
            public static readonly Color standardLine = new Color(1.0f, 1.0f, 1.0f);
            public static readonly Color gridLines = new Color(0.4f, 0.4f, 0.4f);

            public static readonly Color left = Color256(111, 163, 216, 255);
            public static readonly Color leftSelected = Color256(06, 219, 238, 255);
            public static readonly Color right = Color256(238, 134, 84, 255);
            public static readonly Color rightSelected = Color256(247, 212, 201, 255);
            public static readonly Color both = Color256(175, 150, 150, 255);
            public static readonly Color textTopMarkers = Color256(0, 0, 0, 255);
            public static readonly Color marker = new Color(0.0f, 0.5f, 0.5f);
            public static readonly Color markerSelected = new Color(0.0f, 0.6f, 0.6f);
            public static readonly Color thread = new Color(0.5f, 0.0f, 0.5f);
            public static readonly Color threadSelected = new Color(0.6f, 0.0f, 0.6f);
        }

        [SerializeField]
        ProfileDataView m_ProfileSingleView;
        [SerializeField]
        ProfileDataView m_ProfileLeftView;
        [SerializeField]
        ProfileDataView m_ProfileRightView;

        [SerializeField] ThreadMarkerInfo m_SelectedMarker = new ThreadMarkerInfo();

        [Serializable]
        struct ThreadMarkerInfo
        {
            [SerializeField]
            public int id;

            [SerializeField]
            public string threadName;
            [SerializeField]
            public string threadGroupName;
            [SerializeField]
            public string name;
        }

        FrameTimeGraphGlobalSettings m_FrameTimeGraphGlobalSettings;
        FrameTimeGraph m_FrameTimeGraph;
        FrameTimeGraph m_LeftFrameTimeGraph;
        FrameTimeGraph m_RightFrameTimeGraph;
        bool m_FrameTimeGraphsPaired = true;

        TopMarkers m_TopMarkers;
        TopMarkers m_TopMarkersLeft;
        TopMarkers m_TopMarkersRight;


        List<MarkerPairing> m_PairingsNew;
        int m_TotalCombinedMarkerCountNew;

        [SerializeField]
        List<MarkerPairing> m_Pairings = new List<MarkerPairing>();
        int m_TotalCombinedMarkerCount = 0;

        [SerializeField]
        int m_SelectedPairing = 0;

        [SerializeField]
        TreeViewState m_ProfileTreeViewState;
        [SerializeField]
        MultiColumnHeaderState m_ProfileMulticolumnHeaderState;
        ProfileTable m_ProfileTable;

        [SerializeField]
        TreeViewState m_ComparisonTreeViewState;
        [SerializeField]
        MultiColumnHeaderState m_ComparisonMulticolumnHeaderState;
        ComparisonTable m_ComparisonTable;

        internal static class LayoutSize
        {
            public static readonly int WidthColumn0 = 100;
            public static readonly int WidthColumn1 = 52;       // +2 to prevent some
            public static readonly int WidthColumn2 = 52;
            public static readonly int WidthColumn3 = 52;
            public static readonly int WidthRHS = 290;        // Column widths + label padding between (276) + scrollbar width
            public static readonly int FilterOptionsLeftLabelWidth = 100;
            public static readonly int FilterOptionsEnumWidth = 50;
            public static readonly int RemoveMarkerOptionsEnumWidth = 100;
            public static readonly int RemoveMarkerMissingOptionsEnumWidth = 200;
            public static readonly int FilterOptionsLockedEnumWidth = 120;
            public static readonly int FilterOptionsRightLabelWidth = 110;
            public static readonly int FilterOptionsRightEnumWidth = 150;
            public static readonly int HistogramWidth = 153;

            public static readonly int MinWindowWidth = 800 + WidthRHS;
            public static readonly int MinWindowHeight = 480;

            public static readonly int WindowWidth = MinWindowWidth;
            public static readonly int WindowHeight = 840;
            public static readonly int ScrollBarPadding = 6; // this is legacy and we might be able to kill it but it will slightly change the layout of the window.
        }

        Columns m_Columns = new Columns(LayoutSize.WidthColumn0, LayoutSize.WidthColumn1, LayoutSize.WidthColumn2, LayoutSize.WidthColumn3);

        [SerializeField]
        ThreadRange m_ThreadRange = ThreadRange.UpperQuartile;

        internal Draw2D m_2D;

        bool m_Async = true;
        Thread m_BackgroundThread;
        ThreadActivity m_ThreadActivity;
        ProfileData m_ProfilerData;
        string m_Path;
        int m_ThreadPhase;
        int m_ThreadPhases;
        int m_ThreadProgress;

        bool m_RequestRepaint;
        bool m_RequestAnalysis;
        bool m_RequestCompare;
        bool m_FullAnalysisRequired;
        bool m_FullCompareRequired;

        [SerializeField]
        int m_TopNBars = 10;

        bool m_EnableAnalysisProfiling = false;
        int m_AnalyzeInUpdatePhase = 0;

        string m_LastAnalysisTime = "";
        string m_LastCompareTime = "";
        float m_LastAnalysisTimeMilliseconds;
        float m_LastCompareTimeMilliseconds;
        bool m_NewDataLoaded = false;
        bool m_NewComparisonDataLoaded = false;

        Vector2 m_HelpScroll = new Vector2(0, 0);
        Vector2 m_ThreadScroll = new Vector2(0, 0);
        Vector2 m_MarkerSummaryScroll = new Vector2(0, 0);

        Rect m_ThreadsAreaRect = new Rect();
        Rect m_ComparisonThreadsAreaRect = new Rect();

        Vector2 m_LastScreenSize = new Vector2(0, 0);
        bool m_ScreenSizeChanged;
        double m_ScreenSizeChangedTimeStarted;
        double m_ScreenSizeChangedTimeFinished;
        ActiveTab m_ScreenSizeChangedTab;

        GUIStyle m_StyleMiddleRight;
        GUIStyle m_StyleUpperLeft;
        bool m_StylesSetup = false;

        static Regex quotedStringWithoutQuotes = new Regex("\"([^\"]*)\"");
        static Regex quotedString = new Regex("(\"[^\"]*\")");
        static Regex stringWithoutWhiteSpace = new Regex("([^ \t]+)");
        /*
                static Regex lastSpace = new Regex("(.+)[ ]([^ ]*)");
        */

        [MenuItem("Window/Analysis/Profile Analyzer")]
        static void Init()
        {
            var window = GetWindow<ProfileAnalyzerWindow>("Profile Analyzer");
            window.minSize = new Vector2(LayoutSize.MinWindowWidth, LayoutSize.MinWindowHeight);
            window.position.size.Set(LayoutSize.WindowWidth, LayoutSize.WindowHeight);
            window.Show();
            window.m_LastScreenSize = window.position.size;
        }

        /// <summary>
        /// Open profile analyzer window
        /// </summary>
        public static void OpenProfileAnalyzer()
        {
            Init();
        }

        void Awake()
        {
            m_ScreenSizeChanged = false;
            m_ScreenSizeChangedTimeStarted = 0.0;
            m_ScreenSizeChangedTimeFinished = 0.0;
            m_ScreenSizeChangedTab = ActiveTab.Summary;

            m_ProfileSingleView = new ProfileDataView();
            m_ProfileLeftView = new ProfileDataView();
            m_ProfileRightView = new ProfileDataView();

            m_RequestRepaint = false;
            m_RequestAnalysis = false;
            m_RequestCompare = false;

            m_FrameTimeGraphGlobalSettings = new FrameTimeGraphGlobalSettings();
        }

        static int s_TmpCount = 0;
        static string s_TmpDir = "";
        ActiveView m_ActiveLoadingView;
        static string s_ApplicationDataPath;

        internal static string TmpDir
        {
            get
            {
                if (string.IsNullOrEmpty(s_ApplicationDataPath))
                    s_ApplicationDataPath = Application.dataPath;
                if (string.IsNullOrEmpty(s_TmpDir))
                    s_TmpDir = string.Format("{0}{1}ProfileAnalyzer{1}", Directory.GetParent(s_ApplicationDataPath).FullName, Path.DirectorySeparatorChar);

                return s_TmpDir;
            }
        }


        internal static string TmpPath
        {
            get
            {
                if (!Directory.Exists(TmpDir))
                    Directory.CreateDirectory(TmpDir);

                while (File.Exists(string.Format("{0}tmp{1}.pdata", TmpDir, s_TmpCount)))
                {
                    s_TmpCount++;
                }
                return string.Format("{0}tmp{1}.pdata", TmpDir, s_TmpCount);
            }
        }

        void OnEnable()
        {
            //do this here to safeguard against Application.dataPAth being accessed off the main thread
            s_ApplicationDataPath = Application.dataPath;

            // Update styles so we get the theme changes
            m_StylesSetup = false;

            ProfileAnalyzerAnalytics.EnableAnalytics();

            m_ProgressBar = new ProgressBarDisplay();

            if (m_DepthSliceUI == null)
                m_DepthSliceUI = new DepthSliceUI(b => UpdateActiveTab(b));
            else
                m_DepthSliceUI.OnEnable(b => UpdateActiveTab(b));

            m_ProfilerWindowInterface = new ProfilerWindowInterface(m_ProgressBar);
            if (!m_ProfilerWindowInterface.IsReady())
            {
                m_ProfilerWindowInterface.GetProfilerWindowHandle();
            }

            if (IsSelectedMarkerNameValid())
            {
                var oldSelectedMarkerName = m_SelectedMarker;
                m_SelectedMarker.name = null;
                // wait a frame for the ProfilerWindow to get Enabled before re-setting the selection
                EditorApplication.delayCall += () => SelectMarkerByName(oldSelectedMarkerName.name, oldSelectedMarkerName.threadGroupName, oldSelectedMarkerName.threadName);
            }
            m_ProfilerWindowInterface.selectedMarkerChanged -= OnProfilerWindowCpuModuleSelectionChanged;
            m_ProfilerWindowInterface.selectedMarkerChanged += OnProfilerWindowCpuModuleSelectionChanged;

            m_ProfilerWindowInterface.selectedFrameChanged -= OnProfilerWindowSelectedFrameChanged;
            m_ProfilerWindowInterface.selectedFrameChanged += OnProfilerWindowSelectedFrameChanged;
            m_MarkerFilter = new MarkerFilter
            {
                MarkerCache = new Dictionary<string, bool>(),
                NeedsRebuild = true,
            };

            m_ProfileAnalyzer = new ProfileAnalyzer();

            if (m_ThreadSelection == null || m_ThreadSelection.empty)
            {
                ThreadIdentifier mainThreadSelection = new ThreadIdentifier("Main Thread", 1);
                m_ThreadSelection.Set(mainThreadSelection.threadNameWithIndex);
            }

            if (m_ThreadSelectionNew != null && m_ThreadSelectionNew.empty)
                m_ThreadSelectionNew = null;

            m_2D = new Draw2D("Unlit/ProfileAnalyzerShader");
            FrameTimeGraph.SetGlobalSettings(m_FrameTimeGraphGlobalSettings);
            m_FrameTimeGraph = new FrameTimeGraph(0, m_2D, m_DisplayUnits.Units, UIColor.barBackground, UIColor.barBackgroundSelected, UIColor.bar, UIColor.barSelected, UIColor.marker, UIColor.markerSelected, UIColor.thread, UIColor.threadSelected, UIColor.gridLines);
            m_FrameTimeGraph.SetRangeCallback(SetRange);
            m_FrameTimeGraph.SetActiveCallback(GraphActive);
            m_LeftFrameTimeGraph = new FrameTimeGraph(1, m_2D, m_DisplayUnits.Units, UIColor.barBackground, UIColor.barBackgroundSelected, UIColor.left, UIColor.leftSelected, UIColor.marker, UIColor.markerSelected, UIColor.thread, UIColor.threadSelected, UIColor.gridLines);
            m_LeftFrameTimeGraph.SetRangeCallback(SetLeftRange);
            m_LeftFrameTimeGraph.SetActiveCallback(GraphActive);
            m_RightFrameTimeGraph = new FrameTimeGraph(2, m_2D, m_DisplayUnits.Units, UIColor.barBackground, UIColor.barBackgroundSelected, UIColor.right, UIColor.rightSelected, UIColor.marker, UIColor.markerSelected, UIColor.thread, UIColor.threadSelected, UIColor.gridLines);
            m_RightFrameTimeGraph.SetRangeCallback(SetRightRange);
            m_RightFrameTimeGraph.SetActiveCallback(GraphActive);
            m_LeftFrameTimeGraph.PairWith(m_FrameTimeGraphsPaired ? m_RightFrameTimeGraph : null);

            m_TopMarkers = new TopMarkers(this, m_2D, UIColor.barBackground, UIColor.textTopMarkers);
            m_TopMarkersLeft = new TopMarkers(this, m_2D, UIColor.barBackground, UIColor.textTopMarkers);
            m_TopMarkersRight = new TopMarkers(this, m_2D, UIColor.barBackground, UIColor.textTopMarkers);

            m_ThreadActivity = ThreadActivity.None;
            m_ThreadProgress = 0;
            m_ThreadPhase = 0;

            List<int> values = new List<int>();
            List<String> strings = new List<string>();
            for (int i = 1; i <= 10; i++)
            {
                values.Add(i);
                strings.Add(i.ToString());
            }
            m_TopValues = values.ToArray();
            m_TopStrings = strings.ToArray();
            m_TopNumber = 3;

            List<string> unitNames = new List<string>(DisplayUnits.UnitNames);
            unitNames.RemoveAt(unitNames.Count - 1);
            m_UnitNames = unitNames.ToArray();

            // Regrenerate analysis if just re initialised with the existing profile data reloaded from serialisation (e.g. on enter play mode)
            // As we don't serialise the analysis itself.
            // UpdateActiveTab(true);

            UpdateThreadNames();

            if (m_ProfileSingleView.analysis != null)
            {
                CreateProfileTable();
                m_RequestRepaint = true;
            }

            if (m_ProfileLeftView.analysis != null && m_ProfileRightView.analysis != null)
            {
                CreateComparisonTable();
                m_RequestRepaint = true;
            }

            // Mouse movement calls OnGui
            wantsMouseMove = true;
        }

        void OnDisable()
        {
            if (ProfileAnalyzerExportWindow.IsOpen())
                ProfileAnalyzerExportWindow.CloseAll();
            m_ProfilerWindowInterface.selectedMarkerChanged -= OnProfilerWindowCpuModuleSelectionChanged;
            m_ProfilerWindowInterface.selectedFrameChanged -= OnProfilerWindowSelectedFrameChanged;
            m_ProfilerWindowInterface.OnDisable();
            m_ProfilerWindowInterface = null;
        }

        void OnDestroy()
        {
            if (m_BackgroundThread != null)
                m_BackgroundThread.Abort();
            if (m_ProfileSingleView != null && m_ProfileSingleView.data != null)
                m_ProfileSingleView.data.DeleteTmpFiles();
            if (m_ProfileLeftView != null && m_ProfileLeftView.data != null)
                m_ProfileLeftView.data.DeleteTmpFiles();
            if (m_ProfileRightView != null && m_ProfileRightView.data != null)
                m_ProfileRightView.data.DeleteTmpFiles();
            if (Directory.Exists(TmpDir) && Directory.GetFiles(TmpDir).Length == 0)
                Directory.Delete(TmpDir, true);
        }

        bool DisplayCount()
        {
            switch (m_SingleModeFilter.mode)
            {
                case MarkerColumnFilter.Mode.CountTotals:
                case MarkerColumnFilter.Mode.CountPerFrame:
                    return true;
                default:
                    return false;
            }
        }

        void OnGUI()
        {
            if (Event.current.type != EventType.MouseMove)
            {
                m_2D.OnGUI();

                Draw();
            }

            ProcessInput();
        }

        bool TmpInUse(ProfileDataView dv, string path)
        {
            if (dv != m_ProfileSingleView && m_ProfileSingleView.data != null && m_ProfileSingleView.data.FilePath == path)
                return true;

            if (dv != m_ProfileLeftView && m_ProfileLeftView.data != null && m_ProfileLeftView.data.FilePath == path)
                return true;

            if (dv != m_ProfileRightView && m_ProfileRightView.data != null && m_ProfileRightView.data.FilePath == path)
                return true;

            return false;
        }

        void SetView(ProfileDataView dst, ProfileData data, string path, FrameTimeGraph graph)
        {
            if (!data.IsSame(dst.data))
            {
                if (dst == m_ProfileSingleView)
                    m_NewDataLoaded = true;
                else
                    m_NewComparisonDataLoaded = true;
            }

            if (dst.data != null && (m_NewDataLoaded || m_NewComparisonDataLoaded) && !TmpInUse(dst, dst.data.FilePath))
                dst.data.DeleteTmpFiles();

            dst.data = data;
            dst.path = path;
            dst.SelectFullRange();

            graph.Reset();
            graph.SetData(GetFrameTimeData(dst.data));

            // One of the views changed so make sure the export window knows if its open
            ProfileAnalyzerExportWindow exportWindow = ProfileAnalyzerExportWindow.FindOpenWindow();
            if (exportWindow != null)
            {
                exportWindow.SetData(m_ProfileSingleView, m_ProfileLeftView, m_ProfileRightView);
            }
        }

        void SetView(ProfileDataView dst, ProfileDataView src, FrameTimeGraph graph)
        {
            SetView(dst, src.data, src.path, graph);
        }

        void UpdateThreadNames()
        {
            // Update threads list
            switch (m_ActiveTab)
            {
                case ActiveTab.Summary:
                    GetThreadNames(m_ProfileSingleView.data, out m_ThreadUINames, out m_ThreadNames, out m_ThreadNameToUIName);
                    break;
                case ActiveTab.Compare:
                    GetThreadNames(m_ProfileLeftView.data, m_ProfileRightView.data, out m_ThreadUINames, out m_ThreadNames, out m_ThreadNameToUIName);
                    break;
            }

            UpdateThreadGroupSelection(m_ThreadNames, m_ThreadSelection);
            m_ThreadSelectionSummary = CalculateSelectedThreadsSummary();
        }

        void ProcessTabSwitch()
        {
            if (m_NextActiveTab != m_ActiveTab)
            {
                m_ActiveTab = m_NextActiveTab;

                // Copy data if none present for this tab
                switch (m_ActiveTab)
                {
                    case ActiveTab.Summary:
                        if (!m_ProfileSingleView.IsDataValid())
                        {
                            if (m_ProfileLeftView.IsDataValid())
                            {
                                SetView(m_ProfileSingleView, m_ProfileLeftView, m_FrameTimeGraph);

                                m_RequestAnalysis = true;
                                m_FullAnalysisRequired = true;
                            }
                            else if (m_ProfileRightView.IsDataValid())
                            {
                                SetView(m_ProfileSingleView, m_ProfileRightView, m_FrameTimeGraph);

                                m_RequestAnalysis = true;
                                m_FullAnalysisRequired = true;
                            }
                        }
                        break;
                    case ActiveTab.Compare:
                        if ((!m_ProfileLeftView.IsDataValid() || !m_ProfileRightView.IsDataValid()) && m_ProfileSingleView.IsDataValid())
                        {
                            if (!m_ProfileLeftView.IsDataValid())
                            {
                                SetView(m_ProfileLeftView, m_ProfileSingleView, m_LeftFrameTimeGraph);
                            }

                            if (!m_ProfileRightView.IsDataValid())
                            {
                                SetView(m_ProfileRightView, m_ProfileSingleView, m_RightFrameTimeGraph);
                            }

                            // Remove pairing of both left/right point at the same data
                            if (m_ProfileLeftView.path == m_ProfileRightView.path)
                            {
                                SetFrameTimeGraphPairing(false);
                            }

                            m_RequestCompare = true;
                            m_FullCompareRequired = true;
                        }
                        break;
                }

                UpdateThreadNames();
                BuildRemoveMarkerList();

                if (!m_OtherTableDirty)
                    SelectMarker(m_SelectedMarker.name);

                if (m_OtherTabDirty)
                {
                    UpdateActiveTab(true, false);  // Make sure any depth/thread updates are applied when switching tabs, but don't dirty the other tab
                    m_OtherTabDirty = false;
                }

                if (m_OtherTableDirty)
                {
                    UpdateMarkerTable(false);  // Make sure any marker selection updates are applied when switching tabs, but don't dirty the other tab
                    m_OtherTableDirty = false;
                }

                if (!m_RequestAnalysis && !m_RequestCompare)
                    m_DepthSliceUI.UpdateDepthFilters(m_ActiveTab == ActiveTab.Summary, m_ProfileSingleView, m_ProfileLeftView, m_ProfileRightView);
            }
        }

        bool IsDocked()
        {
            return docked;
        }

        void CheckScreenSizeChanges()
        {
            // We get a 5 pixel change in y height during initialization.
            // We could wait before considering size changes but using a delta is also useful
            float sizeDeltaForChange = 10;

            Vector2 sizeDiff = position.size - m_LastScreenSize;
            if (Math.Abs(sizeDiff.x) > sizeDeltaForChange || Math.Abs(sizeDiff.y) > sizeDeltaForChange)
            {
                if (m_LastScreenSize.x != 0) // At initialization time the screen size has not yet been recorded. Don't consider this a screen size change
                {
                    m_LastScreenSize = position.size;
                    if (!m_ScreenSizeChanged)
                    {
                        // Record when we started the change
                        m_ScreenSizeChanged = true;
                        m_ScreenSizeChangedTimeStarted = EditorApplication.timeSinceStartup;
                    }
                    // Record the last time of a change
                    m_ScreenSizeChangedTimeFinished = EditorApplication.timeSinceStartup;

                    // Record which tab we were on when it was changed
                    m_ScreenSizeChangedTab = m_ActiveTab;
                }
            }

            if (m_ScreenSizeChanged)
            {
                double secondsSinceChanged = (EditorApplication.timeSinceStartup - m_ScreenSizeChangedTimeFinished);
                double secondsToDelay = 3f;
                if (secondsSinceChanged > secondsToDelay)
                {
                    // Send analytic
                    var uiResizeView = m_ScreenSizeChangedTab == ActiveTab.Summary ? ProfileAnalyzerAnalytics.UIResizeView.Single : ProfileAnalyzerAnalytics.UIResizeView.Comparison;
                    float durationInSeconds = (float)(m_ScreenSizeChangedTimeFinished - m_ScreenSizeChangedTimeStarted);
                    ProfileAnalyzerAnalytics.SendUIResizeEvent(uiResizeView, durationInSeconds, position.size.x, position.size.y, IsDocked());

                    m_ScreenSizeChanged = false;
                }
            }
        }

        internal void RequestRepaint()
        {
            m_RequestRepaint = true;
        }

        void ProcessInput()
        {
            FrameTimeGraph.State inputStatus = FrameTimeGraph.State.None;

            if (m_ActiveTab == ActiveTab.Summary)
            {
                inputStatus = m_FrameTimeGraph.ProcessInput();
            }
            else if (m_ActiveTab == ActiveTab.Compare)
            {
                if (m_ProfileLeftView.IsDataValid() && inputStatus == FrameTimeGraph.State.None)
                    inputStatus = m_LeftFrameTimeGraph.ProcessInput();

                if (m_ProfileRightView.IsDataValid() && inputStatus == FrameTimeGraph.State.None)
                    inputStatus = m_RightFrameTimeGraph.ProcessInput();
            }

            switch (inputStatus)
            {
                case FrameTimeGraph.State.Dragging:
                    m_RequestRepaint = true;
                    break;
                case FrameTimeGraph.State.DragComplete:
                    m_RequestCompare = true;
                    break;
            }

            if (Event.current.isKey && Event.current.type == EventType.KeyDown)
            {
                switch (Event.current.keyCode)
                {
                    case KeyCode.Alpha1:
                        if (m_ActiveTab == ActiveTab.Summary)
                        {
                            m_FrameTimeGraph.MakeGraphActive(true);
                            GUI.FocusControl("FrameTimeGraph");
                        }
                        else if (m_ActiveTab == ActiveTab.Compare)
                        {
                            m_LeftFrameTimeGraph.MakeGraphActive(true);
                            GUI.FocusControl("LeftFrameTimeGraph");
                        }

                        m_RequestRepaint = true;
                        break;
                    case KeyCode.Alpha2:
                        if (m_ActiveTab == ActiveTab.Compare)
                        {
                            m_RightFrameTimeGraph.MakeGraphActive(true);
                            GUI.FocusControl("RightFrameTimeGraph");
                        }
                        m_RequestRepaint = true;
                        break;
                }
            }
        }

        //Check if the ProfileDataView is in sync with the loaded frame data inside the profiler window
        //We are required to do this check in order to either enable or disable the ability to
        //jump into the matching frame data(loaded in the profiler window) for a specific profile analyzer capture
        void VerifyFrameDataInSyncWithProfilerWindow(ProfileDataView dataView)
        {
            var firstFrameIdx = m_ProfilerFirstFrameIndex - 1;
            var incompleteFrameCount = 0;
            if (dataView != null && dataView.data != null)
                incompleteFrameCount = (dataView.data.FirstFrameIncomplete ? 1 : 0) + (dataView.data.LastFrameIncomplete ? 1 : 0);

            var loadedFrameCount = m_ProfilerLastFrameIndex - m_ProfilerFirstFrameIndex + (firstFrameIdx != -1 ? 1 : 0)
                - incompleteFrameCount;

            if (loadedFrameCount == 0
                || !dataView.IsDataValid() //check if the data is valid and potentially reload the file, .data shouldn't be accessed before this point
                || dataView.data.GetFrameCount() != loadedFrameCount
                || m_ProfilerWindowInterface.GetThreadCountForFrame(firstFrameIdx) != dataView.data.GetFrame(0).threads.Count)
            {
                dataView.inSyncWithProfilerData = false;
            }
            else
            {
                var pDataFrame = dataView.data.GetFrame(0); //get the first frame we don't care about the offset as we only need to compare frames
                var loadedFrame = m_ProfilerWindowInterface.GetProfileFrameForThread(firstFrameIdx, 0);
                //compare frame start time and duration
                //todo improve this
                if (pDataFrame.msStartTime != loadedFrame.msStartTime
                    || pDataFrame.msFrame != loadedFrame.msFrame)
                {
                    dataView.inSyncWithProfilerData = false;
                }
                else
                {
                    dataView.inSyncWithProfilerData = true;
                }
            }
        }

        //Returns true if we were able to sync with the window, but not necessarily if the data is in sync
        bool SyncWithProfilerWindow()
        {
            if (m_ProfilerWindowInterface.IsReady())
            {
                // Check if a new profile has been recorded (or loaded) by checking the frame index range.
                int first;
                int last;
                m_ProfilerWindowInterface.GetFrameRangeFromProfiler(out first, out last);
                if (first != m_ProfilerFirstFrameIndex || last != m_ProfilerLastFrameIndex)
                {
                    // Store the updated range and alter the pull range
                    m_ProfilerFirstFrameIndex = first;
                    m_ProfilerLastFrameIndex = last;
                }

                VerifyFrameDataInSyncWithProfilerWindow(m_ProfileSingleView);
                VerifyFrameDataInSyncWithProfilerWindow(m_ProfileLeftView);
                VerifyFrameDataInSyncWithProfilerWindow(m_ProfileRightView);

                return true;
            }

            m_ProfilerWindowInterface.GetProfilerWindowHandle();
            return false;
        }

        void OnProfilerWindowCpuModuleSelectionChanged(string selectedMarker, string threadGroupName, string threadName)
        {
            // selectedMarker can be "" if in play mode and no active timeline shown (on versions pre 2021.1
            if (!string.IsNullOrEmpty(selectedMarker) && selectedMarker != m_LastProfilerSelectedMarker)
            {
                m_LastProfilerSelectedMarker = selectedMarker;
                m_SelectionEventFromProfilerWindowInProgress = true;
                SelectMarker(selectedMarker, threadGroupName, threadName);
                m_SelectionEventFromProfilerWindowInProgress = false;
                Repaint();
            }
        }

        void OnProfilerWindowSelectedFrameChanged(int newlySelectedFrame)
        {
            // selectedMarker can be "" if in play mode and no active timeline shown (on versions pre 2021.1
            if (!string.IsNullOrEmpty(m_LastProfilerSelectedMarker))
            {
                m_SelectionEventFromProfilerWindowInProgress = true;
                UpdateSelectedMarkerName(m_LastProfilerSelectedMarker);
                m_SelectionEventFromProfilerWindowInProgress = false;
                Repaint();
            }
        }

        void Update()
        {
            CheckScreenSizeChanges();

            // Check if profiler is open
            if (SyncWithProfilerWindow())
            {
                // Check if the selected marker in the profiler has changed
                m_ProfilerWindowInterface.PollProfilerWindowMarkerName();

                m_ProfilerWindowInterface.PollSelectedFrameChanges();
            }

            // Deferred to here so drawing isn't messed up by changing tab half way through a function rendering the old tab
            ProcessTabSwitch();

            // Force repaint for the progress bar
            if (IsAnalysisRunning())
            {
                int loadingProgress;
                int analysisProgress;
                if (IsLoading())
                {
                    loadingProgress = (int)(ProfileData.GetLoadingProgress() * 100);
                    analysisProgress = 0;
                }
                else
                {
                    loadingProgress = 100;
                    analysisProgress = m_ProfileAnalyzer.GetProgress();
                    if (m_ThreadPhases > 1)
                    {
                        // Use thread phases to evaluate the progress as analysis process might contain multiple ProfileAnalyzer passes.
                        analysisProgress = (100 * m_ThreadPhase) / m_ThreadPhases;
                    }
                }

                int progress = (loadingProgress + analysisProgress) / 2;
                if (m_ThreadProgress != progress)
                {
                    m_ThreadProgress = progress;
                    m_RequestRepaint = true;
                }
            }

            if (m_ThreadSelectionNew != null)
            {
                m_ThreadSelection = new ThreadSelection(m_ThreadSelectionNew);
                m_ThreadSelectionNew = null;
                m_ThreadSelectionSummary = CalculateSelectedThreadsSummary();
            }

            switch (m_ThreadActivity)
            {
                case ThreadActivity.AnalyzeDone:
                    // Create table when analysis complete
                    UpdateAnalysisFromAsyncProcessing(m_ProfileSingleView, m_FullAnalysisRequired);
                    m_FullAnalysisRequired = false;

                    UpdateThreadNames();
                    BuildRemoveMarkerList();

                    if (m_ProfileSingleView.analysis != null)
                    {
                        CreateProfileTable();
                        m_RequestRepaint = true;
                    }
                    m_ThreadActivity = ThreadActivity.None;

                    if (m_NewDataLoaded)
                    {
                        if (m_ProfileSingleView.IsDataValid())
                        {
                            // Don't bother sending an analytic if the data set is empty (should never occur anyway but consistent with comparison flow)
                            ProfileAnalyzerAnalytics.SendUIUsageModeEvent(ProfileAnalyzerAnalytics.UIUsageMode.Single, m_LastAnalysisTimeMilliseconds / 1000f);
                        }
                        m_NewDataLoaded = false;
                    }

                    SelectMarker(m_SelectedMarker.name);
                    break;

                case ThreadActivity.CompareDone:
                    UpdateAnalysisFromAsyncProcessing(m_ProfileLeftView, m_FullCompareRequired);
                    UpdateAnalysisFromAsyncProcessing(m_ProfileRightView, m_FullCompareRequired);
                    m_FullCompareRequired = false;
                    m_Pairings = m_PairingsNew;
                    m_TotalCombinedMarkerCount = m_TotalCombinedMarkerCountNew;

                    UpdateThreadNames();
                    BuildRemoveMarkerList();

                    if (m_ProfileLeftView.analysis != null && m_ProfileRightView.analysis != null)
                    {
                        CreateComparisonTable();
                        m_RequestRepaint = true;
                    }
                    m_ThreadActivity = ThreadActivity.None;

                    if (m_NewComparisonDataLoaded)
                    {
                        if (m_ProfileLeftView.IsDataValid() && m_ProfileRightView.IsDataValid())
                        {
                            // Don't bother sending an analytic when one (or more) of the data sets is blank (as no comparison is really made)
                            ProfileAnalyzerAnalytics.SendUIUsageModeEvent(ProfileAnalyzerAnalytics.UIUsageMode.Comparison, m_LastCompareTimeMilliseconds / 1000f);
                        }
                        m_NewComparisonDataLoaded = false;
                    }

                    SelectMarker(m_SelectedMarker.name);
                    break;

                case ThreadActivity.LoadDone:
                    SetView(GetActiveView, m_ProfilerData, m_Path, GetActiveFrameTimeGraph);
                    switch (m_ActiveTab)
                    {
                        case ActiveTab.Compare:
                            // Remove pairing if both left/right point at the same data
                            if (m_ProfileLeftView.path == m_ProfileRightView.path)
                            {
                                SetFrameTimeGraphPairing(false);
                            }

                            m_FullCompareRequired = true;
                            m_RequestCompare = true;
                            break;
                        case ActiveTab.Summary:
                            m_RequestAnalysis = true;
                            m_FullAnalysisRequired = true;
                            break;
                        default:
                            throw new ArgumentOutOfRangeException();
                    }
                    m_ThreadActivity = ThreadActivity.None;
                    break;
            }

            if (m_RequestAnalysis)
            {
                if (!IsAnalysisRunning())
                {
                    Analyze();
                    m_RequestAnalysis = false;
                }
            }
            if (m_RequestCompare)
            {
                if (!IsAnalysisRunning())
                {
                    Compare();
                    m_RequestCompare = false;
                }
            }

            if (m_RequestRepaint)
            {
                Repaint();
                m_RequestRepaint = false;
            }

            if (m_AnalyzeInUpdatePhase > 0)
            {
                switch (m_AnalyzeInUpdatePhase)
                {
                    case 1:
                        UnityEngine.Profiling.Profiler.enabled = true;
                        m_AnalyzeInUpdatePhase++;
                        return;
                    case 2:
                        AnalyzeSync();
                        UpdateAnalysisFromAsyncProcessing(m_ProfileSingleView, m_FullAnalysisRequired);
                        m_FullAnalysisRequired = false;
                        m_AnalyzeInUpdatePhase++;
                        return;
                    case 3:
                        m_AnalyzeInUpdatePhase++;
                        return;
                    case 4:
                        UnityEngine.Profiling.Profiler.enabled = false;
                        m_AnalyzeInUpdatePhase++;
                        return;
                    default:
                        m_AnalyzeInUpdatePhase = 0;
                        break;
                }
            }
        }

        ProfileDataView GetActiveView
        {
            get
            {
                switch (m_ActiveLoadingView)
                {
                    case ActiveView.Single:
                        return m_ProfileSingleView;
                    case ActiveView.Left:
                        return m_ProfileLeftView;
                    case ActiveView.Right:
                        return m_ProfileRightView;
                    default:
                        throw new ArgumentOutOfRangeException();
                }
            }
        }

        FrameTimeGraph GetActiveFrameTimeGraph
        {
            get
            {
                switch (m_ActiveLoadingView)
                {
                    case ActiveView.Single:
                        return m_FrameTimeGraph;
                    case ActiveView.Left:
                        return m_LeftFrameTimeGraph;
                    case ActiveView.Right:
                        return m_RightFrameTimeGraph;
                    default:
                        throw new ArgumentOutOfRangeException();
                }
            }
        }

        void UpdateAnalysisFromAsyncProcessing(ProfileDataView view, bool full)
        {
            view.analysis = view.analysisNew;
            if (full)
            {
                if (view.selectedIndices != null && view.IsDataValid() && view.selectedIndices.Count == view.data.GetFrameCount())
                    view.analysisFull = view.analysis;
                else
                    view.analysisFull = view.analysisFullNew;

                if (view.IsDataValid())
                    view.FindKeyMarkers();
            }
        }

        List<FrameTimeGraph.Data> GetFrameTimeData(ProfileData profileData)
        {
            List<FrameTimeGraph.Data> data = new List<FrameTimeGraph.Data>();
            int frames = profileData.GetFrameCount();

            bool removeFrameSyncTime = false;
            int removeMarkerIndex = -1;
            ThreadIdentifier mainThreadSelection = new ThreadIdentifier("Main Thread", 1);;

            string removeMarker = GetRemoveMarker();
            if (removeMarker != null)
            {
                // Find marker to remove 
                removeMarkerIndex = profileData.GetMarkerIndex(removeMarker);
                removeFrameSyncTime = (removeMarkerIndex != -1);
            }


            for (int frameOffset = 0; frameOffset < frames; frameOffset++)
            {
                ProfileFrame frame = profileData.GetFrame(frameOffset);
                float ms = frame.msFrame;

                if (removeFrameSyncTime)
                {
                    for (int threadIndex = 0; threadIndex < frame.threads.Count; threadIndex++)
                    {
                        ProfileThread thread = frame.threads[threadIndex];
                        // Marker only on main thread
                        if (profileData.GetThreadName(thread) == mainThreadSelection.threadNameWithIndex)
                        {
                            foreach (var marker in thread.markers)
                            {
                                if (marker.nameIndex != removeMarkerIndex)
                                    continue;

                                // May be multiple instances of this marker in the 'custom' case (so we can't just break out of marker loop here)
                                ms-= marker.msMarkerTotal;
                            }
                            break;
                        }
                    }
                }
                FrameTimeGraph.Data dataPoint = new FrameTimeGraph.Data(ms, frameOffset);
                data.Add(dataPoint);
            }

            return data;
        }

        void Load()
        {
            m_Path = EditorUtility.OpenFilePanel("Load profile analyzer data file", "", "pdata");
            if (m_Path.Length != 0)
            {
                m_ActiveLoadingView = ActiveView.Single;
                BeginAsyncAction(ThreadActivity.Load);
            }
            GUIUtility.ExitGUI();
        }

        void UpdateMatchingProfileData(ProfileData data, ref string path, ProfileAnalysis analysis, string newPath)
        {
            // Update left/right data if we are effectively overwriting it.
            if (m_ProfileLeftView.path == newPath)
            {
                SetView(m_ProfileLeftView, data, newPath, m_LeftFrameTimeGraph);

                m_RequestCompare = true;
                m_FullCompareRequired = true;
            }
            if (m_ProfileRightView.path == newPath)
            {
                SetView(m_ProfileRightView, data, newPath, m_RightFrameTimeGraph);

                m_RequestCompare = true;
                m_FullCompareRequired = true;
            }

            // Update single view if needed
            if (m_ProfileSingleView.path == newPath)
            {
                SetView(m_ProfileSingleView, data, newPath, m_FrameTimeGraph);

                m_ProfileSingleView.analysis = analysis;
            }

            path = newPath;
        }

        void Save(ProfileDataView dataView, bool updateDataViewWithSelectedPath = false)
        {
            string newPath = EditorUtility.SaveFilePanel("Save profile analyzer data file", "", "capture.pdata", "pdata");
            if (newPath.Length != 0)
            {
                if (updateDataViewWithSelectedPath)
                {
                    dataView.path = newPath;
                }

                if (ProfileData.Save(newPath, dataView.data))
                {
                    UpdateMatchingProfileData(dataView.data, ref dataView.path, dataView.analysis, newPath);
                }
            }
            GUIUtility.ExitGUI();
        }

        int GetTotalCombinedMarkerCount(ProfileData left, ProfileData right)
        {
            if (left == null)
                return 0;
            if (right == null)
                return 0;
            List<string> leftMarkers = left.GetMarkerNames();
            if (leftMarkers == null)
                return 0;
            List<string> rightMarkers = right.GetMarkerNames();
            if (rightMarkers == null)
                return 0;

            HashSet<string> markerPairs = new HashSet<string>();
            for (int index = 0; index < leftMarkers.Count; index++)
            {
                string markerName = leftMarkers[index];

                markerPairs.Add(markerName);
            }
            for (int index = 0; index < rightMarkers.Count; index++)
            {
                string markerName = rightMarkers[index];

                if (!markerPairs.Contains(markerName))
                {
                    markerPairs.Add(markerName);
                }
            }

            return markerPairs.Count;
        }

        List<MarkerPairing> GeneratePairings(ProfileAnalysis leftAnalysis, ProfileAnalysis rightAnalysis)
        {
            if (leftAnalysis == null)
                return null;
            if (rightAnalysis == null)
                return null;
            List<MarkerData> leftMarkers = leftAnalysis.GetMarkers();
            if (leftMarkers == null)
                return null;
            List<MarkerData> rightMarkers = rightAnalysis.GetMarkers();
            if (rightMarkers == null)
                return null;

            Dictionary<string, MarkerPairing> markerPairs = new Dictionary<string, MarkerPairing>();
            for (int index = 0; index < leftMarkers.Count; index++)
            {
                MarkerData marker = leftMarkers[index];

                MarkerPairing pair = new MarkerPairing
                {
                    name = marker.name,
                    leftIndex = index,
                    rightIndex = -1
                };
                markerPairs[marker.name] = pair;
            }
            for (int index = 0; index < rightMarkers.Count; index++)
            {
                MarkerData marker = rightMarkers[index];

                if (markerPairs.ContainsKey(marker.name))
                {
                    MarkerPairing pair = markerPairs[marker.name];
                    pair.rightIndex = index;
                    markerPairs[marker.name] = pair;
                }
                else
                {
                    MarkerPairing pair = new MarkerPairing
                    {
                        name = marker.name,
                        leftIndex = -1,
                        rightIndex = index
                    };
                    markerPairs[marker.name] = pair;
                }
            }

            List<MarkerPairing> pairings = new List<MarkerPairing>();
            foreach (MarkerPairing pair in markerPairs.Values)
                pairings.Add(pair);

            return pairings;
        }

        void SetThreadPhaseCount(ThreadActivity activity)
        {
            // Will be refined by the analysis functions
            if (activity == ThreadActivity.Compare)
            {
                m_ThreadPhases = 8;
            }
            else
            {
                m_ThreadPhases = 2;
            }
        }

        void BeginAsyncAction(ThreadActivity activity)
        {
            if (IsAnalysisRunning())
                return;

            m_ThreadActivity = activity;
            m_ThreadProgress = 0;
            m_ThreadPhase = 0;
            SetThreadPhaseCount(activity);

            m_BackgroundThread = new Thread(BackgroundThread);
            m_BackgroundThread.Start();
        }

        void CreateComparisonTable()
        {
            UpdateThreadNames();

            // Set default sorting state
            int sortedColumn = (int)ComparisonTable.MyColumns.AbsDiff;
            bool sortAscending = false;

            // Query last sorting state
            if (m_ComparisonMulticolumnHeaderState != null)
            {
                if (m_ComparisonMulticolumnHeaderState.sortedColumnIndex >= 0)
                {
                    sortedColumn = m_ComparisonMulticolumnHeaderState.sortedColumnIndex;
                    if (sortedColumn >= 0 && sortedColumn < m_ComparisonMulticolumnHeaderState.columns.Length)
                        sortAscending = m_ComparisonMulticolumnHeaderState.columns[sortedColumn].sortedAscending;
                }
            }

            if (m_ComparisonTreeViewState == null)
                m_ComparisonTreeViewState = new TreeViewState();

            m_ComparisonMulticolumnHeaderState = ComparisonTable.CreateDefaultMultiColumnHeaderState(m_CompareModeFilter);

            var multiColumnHeader = new MultiColumnHeader(m_ComparisonMulticolumnHeaderState);
            multiColumnHeader.SetSorting(sortedColumn, sortAscending);
            multiColumnHeader.ResizeToFit();
            m_ComparisonTable = new ComparisonTable(m_ComparisonTreeViewState, multiColumnHeader, m_ProfileLeftView, m_ProfileRightView, m_Pairings, m_hideRemovedMarkers, this, m_2D, UIColor.left, UIColor.right);

            if (!IsSelectedMarkerNameValid())
                SelectPairing(0);
            else
                SelectPairingByName(m_SelectedMarker.name);
        }

        void CalculatePairingbuckets(ProfileAnalysis left, ProfileAnalysis right, List<MarkerPairing> pairings)
        {
            var leftMarkers = left.GetMarkers();
            var rightMarkers = right.GetMarkers();
            // using a for loop instead of foreach is surprisingly faster on Mono
            for (int i = 0, n = pairings.Count; i < n; i++)
            {
                var pairing = pairings[i];
                float min = float.MaxValue;
                float max = 0.0f;
                MarkerData leftMarker = null;
                MarkerData rightMarker = null;
                if (pairing.leftIndex >= 0)
                {
                    leftMarker = leftMarkers[pairing.leftIndex];
                    max = Math.Max(max, leftMarker.msMax);
                    min = Math.Min(min, leftMarker.msMin);
                }
                if (pairing.rightIndex >= 0)
                {
                    rightMarker = rightMarkers[pairing.rightIndex];
                    max = Math.Max(max, rightMarker.msMax);
                    min = Math.Min(min, rightMarker.msMin);
                }

                int countMin = int.MaxValue;
                int countMax = 0;
                if (leftMarker != null)
                {
                    countMax = Math.Max(countMax, leftMarker.countMax);
                    countMin = Math.Min(countMin, leftMarker.countMin);
                }
                if (rightMarker != null)
                {
                    countMax = Math.Max(countMax, rightMarker.countMax);
                    countMin = Math.Min(countMin, rightMarker.countMin);
                }

                if (leftMarker != null)
                {
                    leftMarker.ComputeBuckets(min, max);
                    leftMarker.ComputeCountBuckets(countMin, countMax);
                }
                if (rightMarker != null)
                {
                    rightMarker.ComputeBuckets(min, max);
                    rightMarker.ComputeCountBuckets(countMin, countMax);
                }
            }
        }

        bool CompareSync()
        {
            if (!m_ProfileLeftView.IsDataValid())
                return false;
            if (!m_ProfileRightView.IsDataValid())
                return false;

            List<string> threadUINamesNew;
            List<string> threadNamesNew;
            Dictionary<string, string> threadNameToUINameNew;

            GetThreadNames(m_ProfileLeftView.data, m_ProfileRightView.data, out threadUINamesNew, out threadNamesNew, out threadNameToUINameNew);
            List<string> threadSelection = GetLimitedThreadSelection(threadNamesNew, m_ThreadSelection);

            System.Diagnostics.Stopwatch stopwatch = new System.Diagnostics.Stopwatch();
            stopwatch.Start();

            int updateDepthPhase = m_NewComparisonDataLoaded ? 2 : 0;
            int fullLeftPhase = (m_FullCompareRequired && m_ProfileLeftView.selectedIndices.Count != m_ProfileLeftView.data.GetFrameCount()) ? 1 : 0;
            int fullRightPhase = (m_FullCompareRequired && m_ProfileLeftView.selectedIndices.Count != m_ProfileLeftView.data.GetFrameCount()) ? 1 : 0;
            m_ThreadPhases = 2 /*scan left and right*/ + updateDepthPhase + 2 /*fullLeftPhase and fullRightPhase*/ + 2 /*analyze left and right*/;

            bool selfTimes = IsSelfTime();

            // First scan just the frames
            m_ThreadPhase = 0;
            var leftAnalysisNew = m_ProfileAnalyzer.Analyze(m_ProfileLeftView.data, m_ProfileLeftView.selectedIndices, null, m_DepthSliceUI.depthFilter1, selfTimes, m_ParentMarker, 0, GetRemoveMarker());
            m_ThreadPhase++;
            var rightAnalysisNew = m_ProfileAnalyzer.Analyze(m_ProfileRightView.data, m_ProfileRightView.selectedIndices, null, m_DepthSliceUI.depthFilter2, selfTimes, m_ParentMarker, 0, GetRemoveMarker());
            m_ThreadPhase++;

            if (leftAnalysisNew == null || rightAnalysisNew == null)
            {
                stopwatch.Stop();
                return false;
            }

            // Calculate the max frame time of the two scans
            float timeScaleMax = Math.Max(leftAnalysisNew.GetFrameSummary().msMax, rightAnalysisNew.GetFrameSummary().msMax);

            // Need to recalculate the depth difference when thread filters change
            // For now do it always if the depth is auto and not 'all'
            if (updateDepthPhase != 0)
            {
                var leftAnalysis = m_ProfileAnalyzer.Analyze(m_ProfileLeftView.data, m_ProfileLeftView.selectedIndices, threadSelection, ProfileAnalyzer.kDepthAll, selfTimes, m_ParentMarker, timeScaleMax, GetRemoveMarker());
                m_ThreadPhase++;
                var rightAnalysis = m_ProfileAnalyzer.Analyze(m_ProfileRightView.data, m_ProfileRightView.selectedIndices, threadSelection, ProfileAnalyzer.kDepthAll, selfTimes, m_ParentMarker, timeScaleMax, GetRemoveMarker());
                m_ThreadPhase++;

                var pairings = GeneratePairings(leftAnalysis, rightAnalysis);

                if (m_DepthSliceUI.UpdateDepthForCompareSync(leftAnalysis, rightAnalysis, pairings, m_ProfileLeftView, m_ProfileRightView))
                {
                    // New depth diff calculated to we need to do the full analysis
                    if (fullLeftPhase == 0)
                        fullLeftPhase = 1;
                    if (fullRightPhase == 0)
                        fullRightPhase = 1;
                }
            }

            // Now process the markers and setup buckets using the overall max frame time
            List<int> selection = new List<int>();
            if (fullLeftPhase != 0)
            {
                selection.Clear();
                for (int frameOffset = 0; frameOffset < m_ProfileLeftView.data.GetFrameCount(); frameOffset++)
                {
                    selection.Add(m_ProfileLeftView.data.OffsetToDisplayFrame(frameOffset));
                }

                // We don't pass timeScaleMax as that is only for the selected region.
                // Pass 0 to auto select full range
                m_ProfileLeftView.analysisFullNew = m_ProfileAnalyzer.Analyze(m_ProfileLeftView.data, selection, threadSelection, m_DepthSliceUI.depthFilter1, selfTimes, m_ParentMarker, 0, GetRemoveMarker());
                m_ThreadPhase++;
            }
            m_ThreadPhase++;

            if (fullRightPhase != 0)
            {
                selection.Clear();
                for (int frameOffset = 0; frameOffset < m_ProfileRightView.data.GetFrameCount(); frameOffset++)
                {
                    selection.Add(m_ProfileRightView.data.OffsetToDisplayFrame(frameOffset));
                }

                // We don't pass timeScaleMax as that is only for the selected region.
                // Pass 0 to auto select full range
                m_ProfileRightView.analysisFullNew = m_ProfileAnalyzer.Analyze(m_ProfileRightView.data, selection, threadSelection, m_DepthSliceUI.depthFilter2, selfTimes, m_ParentMarker, 0, GetRemoveMarker());
                m_ThreadPhase++;
            }
            m_ThreadPhase++;

            m_ProfileLeftView.analysisNew = m_ProfileAnalyzer.Analyze(m_ProfileLeftView.data, m_ProfileLeftView.selectedIndices, threadSelection, m_DepthSliceUI.depthFilter1, selfTimes, m_ParentMarker, timeScaleMax, GetRemoveMarker());
            m_ThreadPhase++;

            m_ProfileRightView.analysisNew = m_ProfileAnalyzer.Analyze(m_ProfileRightView.data, m_ProfileRightView.selectedIndices, threadSelection, m_DepthSliceUI.depthFilter2, selfTimes, m_ParentMarker, timeScaleMax, GetRemoveMarker());
            m_ThreadPhase++;

            m_TotalCombinedMarkerCountNew = GetTotalCombinedMarkerCount(m_ProfileLeftView.data, m_ProfileRightView.data);
            m_PairingsNew = GeneratePairings(m_ProfileLeftView.analysisNew, m_ProfileRightView.analysisNew);

            CalculatePairingbuckets(m_ProfileLeftView.analysisNew, m_ProfileRightView.analysisNew, m_PairingsNew);

            stopwatch.Stop();
            m_LastCompareTimeMilliseconds = stopwatch.ElapsedMilliseconds;

            TimeSpan ts = stopwatch.Elapsed;
            if (ts.Minutes > 0)
                m_LastCompareTime = string.Format("Last compare time {0} mins {1} secs {2} ms ", ts.Minutes, ts.Seconds, ts.Milliseconds);
            else if (ts.Seconds > 0)
                m_LastCompareTime = string.Format("Last compare time {0} secs {1} ms ", ts.Seconds, ts.Milliseconds);
            else
                m_LastCompareTime = string.Format("Last compare time {0} ms ", ts.Milliseconds);

            return true;
        }

        void Compare()
        {
            if (m_Async)
            {
                //m_comparisonTable = null;
                //m_ProfileLeftView.analysis = null;
                //m_ProfileRightView.analysis = null;
                BeginAsyncAction(ThreadActivity.Compare);
            }
            else
            {
                CompareSync();

                UpdateAnalysisFromAsyncProcessing(m_ProfileLeftView, m_FullCompareRequired);
                UpdateAnalysisFromAsyncProcessing(m_ProfileRightView, m_FullCompareRequired);
                m_FullCompareRequired = false;
            }
        }

        List<MarkerPairing> GetPairings()
        {
            return m_Pairings;
        }

        int GetUnsavedIndex(string path)
        {
            if (path == null)
                return 0;

            Regex unsavedRegExp = new Regex(@"^Unsaved[\s*]([\d]*)", RegexOptions.IgnoreCase);
            Match match = unsavedRegExp.Match(path);
            if (match.Length <= 0)
                return 0;

            return Int32.Parse(match.Groups[1].Value);
        }

        void PullFromProfiler(int firstFrame, int lastFrame, ProfileDataView view, FrameTimeGraph frameTimeGraph)
        {
            m_ProgressBar.InitProgressBar("Pulling Frames from Profiler", "Please wait...", lastFrame - firstFrame);

            var analytic = ProfileAnalyzerAnalytics.BeginAnalytic();
            ProfileData newProfileData = m_ProfilerWindowInterface.PullFromProfiler(firstFrame, lastFrame);
            ProfileAnalyzerAnalytics.SendUIButtonEvent(ProfileAnalyzerAnalytics.UIButton.Pull, analytic);

            frameTimeGraph.Reset();
            frameTimeGraph.SetData(GetFrameTimeData(newProfileData));

            // Check if this is new data (rather than repulling the same data)
            if (!newProfileData.IsSame(view.data))
            {
                if (view == m_ProfileSingleView)
                    m_NewDataLoaded = true;
                else
                    m_NewComparisonDataLoaded = true;
            }

            // Update the path to use the same saved file name if this is the same data as another view
            if (newProfileData.IsSame(m_ProfileLeftView.data))
            {
                view.path = m_ProfileLeftView.path;
            }
            else if (newProfileData.IsSame(m_ProfileRightView.data))
            {
                view.path = m_ProfileRightView.path;
            }
            else if (newProfileData.IsSame(m_ProfileSingleView.data))
            {
                view.path = m_ProfileSingleView.path;
            }
            else
            {
                int lastIndex = 0;
                lastIndex = Math.Max(lastIndex, GetUnsavedIndex(m_ProfileSingleView.path));
                lastIndex = Math.Max(lastIndex, GetUnsavedIndex(m_ProfileLeftView.path));
                lastIndex = Math.Max(lastIndex, GetUnsavedIndex(m_ProfileRightView.path));
                view.path = string.Format("Unsaved {0}", lastIndex + 1);
            }

            if (view.data != null && !TmpInUse(view, view.data.FilePath))
                view.data.DeleteTmpFiles();

            view.data = newProfileData;
            view.SelectFullRange();

            // Remove pairing if both left/right point at the same data
            if (m_ProfileLeftView.path == m_ProfileRightView.path)
            {
                SetFrameTimeGraphPairing(false);
            }

            m_ProgressBar.ClearProgressBar();
        }

        void BackgroundThread()
        {
            try
            {
                switch (m_ThreadActivity)
                {
                    case ThreadActivity.Analyze:
                        AnalyzeSync();
                        m_ThreadActivity = ThreadActivity.AnalyzeDone;
                        break;

                    case ThreadActivity.Compare:
                        CompareSync();
                        m_ThreadActivity = ThreadActivity.CompareDone;
                        break;

                    case ThreadActivity.AnalyzeDone:
                        break;

                    case ThreadActivity.CompareDone:
                        break;

                    case ThreadActivity.Load:
                        m_ThreadActivity = ProfileData.Load(m_Path, out m_ProfilerData) ? ThreadActivity.LoadDone : ThreadActivity.None;
                        break;

                    case ThreadActivity.LoadDone:
                        break;

                    default:
                        // m_threadActivity = ThreadActivity.None;
                        break;
                }
            }
            catch (ThreadAbortException)
            {
                var activity = (m_ThreadActivity == ThreadActivity.Load) ? "Load" : "Analysis";
                Debug.LogFormat("{0} failed due to a domain reload. Please try again.", activity);
            }
        }

        void SelectFirstMarkerInTable()
        {
            // SelectMarkerByIndex(0) would only select the first one found, not the first in the sorted list

            if (m_ProfileTable == null)
                return;

            var rows = m_ProfileTable.GetRows();
            if (rows == null || rows.Count < 1)
                return;

            SelectMarkerByName(rows[0].displayName);
        }

        bool IsSelectedMarkerNameValid()
        {
            if (string.IsNullOrEmpty(m_SelectedMarker.name))
                return false;

            if (m_hideRemovedMarkers && m_SelectedMarker.name == GetRemoveMarker())
                return false;

            return true;
        }

        void CreateProfileTable()
        {
            if (m_ProfileTreeViewState == null)
                m_ProfileTreeViewState = new TreeViewState();

            // Set default sorting state
            int sortedColumn = (int)ProfileTable.MyColumns.Median;
            bool sortAscending = false;

            // Query last sorting state
            if (m_ProfileMulticolumnHeaderState != null)
            {
                if (m_ProfileMulticolumnHeaderState.sortedColumnIndex >= 0)
                {
                    sortedColumn = m_ProfileMulticolumnHeaderState.sortedColumnIndex;
                    if (sortedColumn >= 0 && sortedColumn < m_ProfileMulticolumnHeaderState.columns.Length)
                        sortAscending = m_ProfileMulticolumnHeaderState.columns[sortedColumn].sortedAscending;
                }
            }

            m_ProfileMulticolumnHeaderState = ProfileTable.CreateDefaultMultiColumnHeaderState(m_SingleModeFilter);

            var multiColumnHeader = new MultiColumnHeader(m_ProfileMulticolumnHeaderState);
            multiColumnHeader.SetSorting(sortedColumn, sortAscending);
            multiColumnHeader.ResizeToFit();
            m_ProfileTable = new ProfileTable(m_ProfileTreeViewState, multiColumnHeader, m_ProfileSingleView, m_hideRemovedMarkers, this, m_2D, UIColor.bar);

            if (!IsSelectedMarkerNameValid())
                SelectFirstMarkerInTable();
            else
                SelectMarkerByName(m_SelectedMarker.name);

            UpdateThreadNames();
        }

        void AnalyzeSync()
        {
            if (!m_ProfileSingleView.IsDataValid())
                return;

            System.Diagnostics.Stopwatch stopwatch = new System.Diagnostics.Stopwatch();
            stopwatch.Start();

            List<string> threadUINamesNew;
            List<string> threadNamesNew;
            Dictionary<string, string> threadNameToUINameNew;

            GetThreadNames(m_ProfileSingleView.data, out threadUINamesNew, out threadNamesNew, out threadNameToUINameNew);
            List<string> threadSelection = GetLimitedThreadSelection(threadNamesNew, m_ThreadSelection);

            int fullPhase = (m_FullAnalysisRequired && (m_ProfileSingleView.selectedIndices.Count != m_ProfileSingleView.data.GetFrameCount())) ? 1 : 0;
            m_ThreadPhases = 1 + fullPhase;

            bool selfTimes = IsSelfTime();

            m_ThreadPhase = 0;
            if (fullPhase == 1)
            {
                List<int> selection = new List<int>();
                for (int frameOffset = 0; frameOffset < m_ProfileSingleView.data.GetFrameCount(); frameOffset++)
                {
                    selection.Add(m_ProfileSingleView.data.OffsetToDisplayFrame(frameOffset));
                }

                m_ProfileSingleView.analysisFullNew = m_ProfileAnalyzer.Analyze(m_ProfileSingleView.data, selection, threadSelection, m_DepthSliceUI.depthFilter, selfTimes, m_ParentMarker, 0, GetRemoveMarker());
                m_ThreadPhase++;
            }
            m_ProfileSingleView.analysisNew = m_ProfileAnalyzer.Analyze(m_ProfileSingleView.data, m_ProfileSingleView.selectedIndices, threadSelection, m_DepthSliceUI.depthFilter, selfTimes, m_ParentMarker, 0, GetRemoveMarker());
            m_ThreadPhase++;
            stopwatch.Stop();
            m_LastAnalysisTimeMilliseconds = stopwatch.ElapsedMilliseconds;

            TimeSpan ts = stopwatch.Elapsed;
            if (ts.Minutes > 0)
                m_LastAnalysisTime = string.Format("Last analysis time {0} mins {1} secs {2} ms ", ts.Minutes, ts.Seconds, ts.Milliseconds);
            else if (ts.Seconds > 0)
                m_LastAnalysisTime = string.Format("Last analysis time {0} secs {1} ms ", ts.Seconds, ts.Milliseconds);
            else
                m_LastAnalysisTime = string.Format("Last analysis time {0} ms ", ts.Milliseconds);
        }

        void Analyze()
        {
            if (m_EnableAnalysisProfiling)
            {
                m_AnalyzeInUpdatePhase = 1;
                return;
            }

            if (m_Async)
            {
                //m_profileTable = null;
                //m_ProfileSingleView.analysis = null;
                BeginAsyncAction(ThreadActivity.Analyze);
            }
            else
            {
                AnalyzeSync();
                UpdateAnalysisFromAsyncProcessing(m_ProfileSingleView, m_FullAnalysisRequired);
                m_FullAnalysisRequired = false;
            }
        }

        void GetThreadNames(ProfileData profleData, out List<string> threadUINames, out List<string> threadFilters, out Dictionary<string, string> threadNameToUIName)
        {
            GetThreadNames(profleData, null, out threadUINames, out threadFilters, out threadNameToUIName);
        }

        public string GetUIThreadName(string threadNameWithIndex)
        {
            string threadName = "";
            m_ThreadNameToUIName.TryGetValue(threadNameWithIndex, out threadName);
            return threadName;
        }

        string GetFriendlyThreadName(string threadNameWithIndex, bool single)
        {
            if (string.IsNullOrEmpty(threadNameWithIndex))
                return "";

            var info = threadNameWithIndex.Split(':');
            int threadGroupIndex = int.Parse(info[0]);
            var threadName = info[1].Trim();

            if (single) // Single instance of this thread name
            {
                return threadName;
            }
            else
            {
                // The original format was "Worker 0"
                // The internal formatting is 1:Worker (1+original value).
                // Hence the -1 here
                return string.Format("{0} {1}", threadName, threadGroupIndex - 1);
            }
        }

        internal int CompareUINames(string a, string b)
        {
            var aSpaceIndex = a.LastIndexOf(' ');
            var bSpaceIndex = b.LastIndexOf(' ');

            if (aSpaceIndex >= 0 && bSpaceIndex >= 0)
            {
                var aThreadName = a.Substring(0, aSpaceIndex);
                var bThreadName = b.Substring(0, bSpaceIndex);

                if (aThreadName == bThreadName)
                {
                    var aThreadIndex = a.Substring(aSpaceIndex + 1);
                    var bThreadIndex = b.Substring(bSpaceIndex + 1);

                    if (aThreadIndex == "All" && bThreadIndex != "All")
                        return -1;
                    if (aThreadIndex != "All" && bThreadIndex == "All")
                        return 1;

                    int aGroupIndex;
                    if (int.TryParse(aThreadIndex, out aGroupIndex))
                    {
                        int bGroupIndex;
                        if (int.TryParse(bThreadIndex, out bGroupIndex))
                        {
                            return aGroupIndex.CompareTo(bGroupIndex);
                        }
                    }
                }
            }

            return a.CompareTo(b);
        }

        void GetThreadNames(ProfileData leftData, ProfileData rightData, out List<string> threadUINames, out List<string> threadFilters, out Dictionary<string, string> threadNameToUIName)
        {
            List<string> threadNames = (leftData != null) ? new List<string>(leftData.GetThreadNames()) : new List<string>();
            if (rightData != null)
            {
                foreach (var threadNameWithIndex in rightData.GetThreadNames())
                {
                    if (!threadNames.Contains(threadNameWithIndex))
                    {
                        // TODO: Insert after last thread with same name (or at end)
                        threadNames.Add(threadNameWithIndex);
                    }
                }
            }

            Dictionary<string, string> threadNamesDict = new Dictionary<string, string>();
            for (int index = 0; index < threadNames.Count; index++)
            {
                var threadNameWithIndex = threadNames[index];
                var threadIdentifier = new ThreadIdentifier(threadNameWithIndex);

                if (threadIdentifier.index == 1)
                {
                    if (threadNames.Contains(string.Format("2:{0}", threadIdentifier.name)))
                    {
                        var threadGroupIdentifier = new ThreadIdentifier(threadIdentifier);
                        threadGroupIdentifier.SetAll();

                        // First thread name of a group with the same name
                        // Add an 'all' selection
                        threadNamesDict[string.Format("{0} : All", threadIdentifier.name)] = threadGroupIdentifier.threadNameWithIndex;
                        // And add the first item too
                        threadNamesDict[GetFriendlyThreadName(threadNameWithIndex, false)] = threadNameWithIndex;
                    }
                    else
                    {
                        // Single instance of this thread name
                        threadNamesDict[GetFriendlyThreadName(threadNameWithIndex, true)] = threadNameWithIndex;
                    }
                }
                else
                {
                    threadNamesDict[GetFriendlyThreadName(threadNameWithIndex, false)] = threadNameWithIndex;
                }
            }

            List<string> uiNames = new List<string>();
            foreach (string uiName in threadNamesDict.Keys)
                uiNames.Add(uiName);

            uiNames.Sort(CompareUINames);


            var allThreadIdentifier = new ThreadIdentifier();
            allThreadIdentifier.SetName("All");
            allThreadIdentifier.SetAll();

            threadUINames = new List<string>();
            threadFilters = new List<string>();
            threadNameToUIName = new Dictionary<string, string>();

            threadUINames.Add(allThreadIdentifier.name);
            threadFilters.Add(allThreadIdentifier.threadNameWithIndex);
            threadNameToUIName[allThreadIdentifier.name] = allThreadIdentifier.threadNameWithIndex;

            foreach (string uiName in uiNames)
            {
                // Strip off the group name
                // Note we don't do this in GetFriendlyThreadName else we would collapse the same named threads (in different groups) in the dict
                string groupName;
                string threadName = ProfileData.GetThreadNameWithoutGroup(uiName, out groupName);
                string threadFilter = threadNamesDict[uiName];
                threadUINames.Add(threadName);
                threadFilters.Add(threadFilter);

                threadNameToUIName[threadFilter] = threadName;
            }
        }

        void UpdateThreadGroupSelection(List<string> threadNames, ThreadSelection threadSelection)
        {
            // Make sure all members of active groups are present
            foreach (string threadGroupNameWithIndex in threadSelection.groups)
            {
                var threadGroupIdentifier = new ThreadIdentifier(threadGroupNameWithIndex);
                if (threadGroupIdentifier.name == "All" && threadGroupIdentifier.index == ThreadIdentifier.kAll)
                {
                    foreach (string threadNameWithIndex in threadNames)
                    {
                        var threadIdentifier = new ThreadIdentifier(threadNameWithIndex);
                        if (threadIdentifier.index != ThreadIdentifier.kAll)
                        {
                            if (!threadSelection.selection.Contains(threadNameWithIndex))
                                threadSelection.selection.Add(threadNameWithIndex);
                        }
                    }
                }
                else
                {
                    foreach (string threadNameWithIndex in threadNames)
                    {
                        var threadIdentifier = new ThreadIdentifier(threadNameWithIndex);
                        if (threadIdentifier.name == threadGroupIdentifier.name &&
                            threadIdentifier.index != ThreadIdentifier.kAll)
                        {
                            if (!threadSelection.selection.Contains(threadNameWithIndex))
                                threadSelection.selection.Add(threadNameWithIndex);
                        }
                    }
                }
            }
        }

        List<string> GetLimitedThreadSelection(List<string> threadNames, ThreadSelection threadSelection)
        {
            List<string> limitedThreadSelection = new List<string>();
            if (threadSelection.selection == null)
                return limitedThreadSelection;

            foreach (string threadNameWithIndex in threadSelection.selection)
            {
                if (threadNames.Contains(threadNameWithIndex))
                    limitedThreadSelection.Add(threadNameWithIndex);
            }

            // Make sure all members of active groups are present
            foreach (string threadGroupNameWithIndex in threadSelection.groups)
            {
                var threadGroupIdentifier = new ThreadIdentifier(threadGroupNameWithIndex);
                if (threadGroupIdentifier.name == "All" && threadGroupIdentifier.index == ThreadIdentifier.kAll)
                {
                    foreach (string threadNameWithIndex in threadNames)
                    {
                        var threadIdentifier = new ThreadIdentifier(threadNameWithIndex);
                        if (threadIdentifier.index != ThreadIdentifier.kAll)
                        {
                            if (!limitedThreadSelection.Contains(threadNameWithIndex))
                                limitedThreadSelection.Add(threadNameWithIndex);
                        }
                    }
                }
                else
                {
                    foreach (string threadNameWithIndex in threadNames)
                    {
                        var threadIdentifier = new ThreadIdentifier(threadNameWithIndex);
                        if (threadIdentifier.name == threadGroupIdentifier.name &&
                            threadIdentifier.index != ThreadIdentifier.kAll)
                        {
                            if (!limitedThreadSelection.Contains(threadNameWithIndex))
                                limitedThreadSelection.Add(threadNameWithIndex);
                        }
                    }
                }
            }

            return limitedThreadSelection;
        }

        int ClampToRange(int value, int min, int max)
        {
            if (value < min)
                value = min;
            if (value > max)
                value = max;

            return value;
        }

        void GraphActive(bool active)
        {
            RequestRepaint();
        }

        void SetRange(List<int> selectedOffsets, int clickCount, FrameTimeGraph.State inputStatus)
        {
            if (inputStatus == FrameTimeGraph.State.Dragging)
                return;

            if (clickCount == 2)
            {
                if (selectedOffsets.Count > 0 && m_ProfileSingleView.inSyncWithProfilerData)
                    JumpToFrame(m_ProfileSingleView.data.OffsetToDisplayFrame(selectedOffsets[0]), m_ProfileSingleView.data, false);
            }
            else
            {
                m_ProfileSingleView.selectedIndices.Clear();
                foreach (int offset in selectedOffsets)
                {
                    m_ProfileSingleView.selectedIndices.Add(m_ProfileSingleView.data.OffsetToDisplayFrame(offset));
                }
                // Keep indices sorted
                m_ProfileSingleView.selectedIndices.Sort();

                m_RequestAnalysis = true;
            }
        }

        internal void ClearSelection()
        {
            if (m_ActiveTab == ActiveTab.Summary)
            {
                m_ProfileSingleView.ClearSelection();

                m_RequestAnalysis = true;
            }
            if (m_ActiveTab == ActiveTab.Compare)
            {
                m_ProfileLeftView.ClearSelection();
                m_ProfileRightView.ClearSelection();

                m_RequestCompare = true;
            }
        }

        internal void SelectAllFrames()
        {
            if (m_ActiveTab == ActiveTab.Summary)
            {
                m_ProfileSingleView.SelectFullRange();

                m_RequestAnalysis = true;
            }
            if (m_ActiveTab == ActiveTab.Compare)
            {
                m_ProfileLeftView.SelectFullRange();
                m_ProfileRightView.SelectFullRange();

                m_RequestCompare = true;
            }
        }

        internal void SelectFramesContainingMarker(string markerName, bool inSelection)
        {
            if (m_ActiveTab == ActiveTab.Summary)
            {
                if (m_ProfileSingleView.SelectAllFramesContainingMarker(markerName, inSelection))
                {
                    m_RequestAnalysis = true;
                }
            }
            if (m_ActiveTab == ActiveTab.Compare)
            {
                if (m_ProfileLeftView.SelectAllFramesContainingMarker(markerName, inSelection))
                {
                    m_RequestCompare = true;
                }
                if (m_ProfileRightView.SelectAllFramesContainingMarker(markerName, inSelection))
                {
                    m_RequestCompare = true;
                }
            }
        }

        static List<string> GetNameFilters(string nameFilter)
        {
            List<string> nameFilters = new List<string>();
            if (string.IsNullOrEmpty(nameFilter))
                return nameFilters;

            // Get all quoted strings, without the quotes
            MatchCollection matches = quotedStringWithoutQuotes.Matches(nameFilter);
            foreach (Match match in matches)
            {
                var theData = match.Groups[1].Value;
                nameFilters.Add(theData);
            }

            // Get a new string with the quoted strings removed
            string remaining = quotedString.Replace(nameFilter, "");

            // Get all the remaining strings (that are space separated)
            matches = stringWithoutWhiteSpace.Matches(remaining);
            foreach (Match match in matches)
            {
                string theData = match.Groups[1].Value;
                nameFilters.Add(theData);
            }

            return nameFilters;
        }

        internal List<string> GetNameFilters()
        {
            return GetNameFilters(m_NameFilter);
        }

        internal List<string> GetNameExcludes()
        {
            return GetNameFilters(m_NameExclude);
        }

        internal bool DoesMarkerPassFilter(string name)
        {
            return m_MarkerFilter.DoesMarkerPassFilter(this, name);
        }

        internal bool NameInIncludeList(string name, List<string> nameFilters)
        {
            return NameInFilterList(name, nameFilters, m_NameFilterOperation);
        }

        internal bool NameInExcludeList(string name, List<string> nameExcludes)
        {
            return NameInFilterList(name, nameExcludes, m_NameExcludeOperation);
        }

        static bool NameInFilterList(string name, List<string> nameFilters, NameFilterOperation operation)
        {
            switch (operation)
            {
                default:
                //case NameFilterOperation.All:
                {
                    foreach (string subString in nameFilters)
                    {
                        // As soon as name doesn't match one in the list then return false
                        if (name.IndexOf(subString, StringComparison.OrdinalIgnoreCase) < 0)
                            return false;
                    }

                    // Name is matching all the filters in the list
                    return true;
                }
                case NameFilterOperation.Any:
                {
                    foreach (string subString in nameFilters)
                    {
                        // As soon as names matches one in the list then return true
                        if (name.IndexOf(subString, StringComparison.OrdinalIgnoreCase) >= 0)
                            return true;
                    }

                    return false;
                }
            }
        }

        static string FilterWithQuotes(string markerName)
        {
            return markerName.Contains(" ") ? string.Format("\"{0}\"", markerName) : markerName;
        }

        static void AddFilter(ref string filter, string quotedMarkerName)
        {
            if (string.IsNullOrEmpty(filter))
                filter = quotedMarkerName;
            else
                filter = string.Format("{0} {1}", filter, quotedMarkerName);
        }

        static bool AddFilter(List<string> nameFilters, ref string filter, string markerName)
        {
            bool justAdded = false;

            string quotedMarkerName = FilterWithQuotes(markerName);
            if (!nameFilters.Contains(quotedMarkerName))
            {
                AddFilter(ref filter, quotedMarkerName);

                justAdded = true;
            }

            return justAdded;
        }

        internal void AddToIncludeFilter(string markerName)
        {
            if (AddFilter(GetNameFilters(), ref m_NameFilter, markerName))
            {
                m_MarkerFilter.Clear();
                UpdateMarkerTable();
            }

            // Remove from exclude list if in the include list
            RemoveFromExcludeFilter(markerName);
        }

        internal void AddToExcludeFilter(string markerName)
        {
            if (AddFilter(GetNameExcludes(), ref m_NameExclude, markerName))
            {
                m_MarkerFilter.Clear();
                UpdateMarkerTable();
            }

            // Remove from include list if in the include list
            RemoveFromIncludeFilter(markerName);
        }

        internal void RemoveFromFilter(string markerName, List<string> nameFilters, ref string newFilters)
        {
            if (nameFilters.Count == 0)
                return;

            string nameFilterString = "";
            bool updated = false;
            foreach (string filter in nameFilters)
            {
                if (string.Compare(filter, markerName, StringComparison.CurrentCultureIgnoreCase) != 0)
                    AddFilter(ref nameFilterString, FilterWithQuotes(filter));
                else
                    updated = true;
            }

            if (updated)
            {
                newFilters = nameFilterString;

                m_MarkerFilter.Clear();
                UpdateMarkerTable();
            }
        }

        internal void RemoveFromIncludeFilter(string markerName)
        {
            RemoveFromFilter(markerName, GetNameFilters(), ref m_NameFilter);
        }

        internal void RemoveFromExcludeFilter(string markerName)
        {
            RemoveFromFilter(markerName, GetNameExcludes(), ref m_NameExclude);
        }

        internal void SetAsParentMarkerFilter(string markerName)
        {
            if (markerName != m_ParentMarker)
            {
                m_ParentMarker = markerName;
                UpdateActiveTab(true);
            }
        }

        float GetFilenameWidth(string path)
        {
            if (path == null)
                return 0f;

            string filename = System.IO.Path.GetFileName(path);
            GUIContent content = new GUIContent(filename, path);
            Vector2 size = GUI.skin.label.CalcSize(content);
            return size.x;
        }

        void ShowFilename(string path)
        {
            if (path != null)
            {
                string filename = System.IO.Path.GetFileNameWithoutExtension(path);
                GUIContent content = new GUIContent(filename, path);
                Vector2 size = GUI.skin.label.CalcSize(content);
                float width = Math.Min(size.x, 200f);
                EditorGUILayout.LabelField(content, GUILayout.MaxWidth(width));
            }
        }

        void DrawLoadSave()
        {
            EditorGUILayout.BeginHorizontal(GUILayout.MaxWidth(300), GUILayout.ExpandWidth(false));

            GUIStyle buttonStyle = GUI.skin.button;
            bool lastEnabled = GUI.enabled;
            bool isAnalysisRunning = IsAnalysisRunning();
            GUI.enabled = !isAnalysisRunning;
            if (GUILayout.Button("Load", buttonStyle, GUILayout.ExpandWidth(false), GUILayout.Width(50)))
                Load();
            GUI.enabled = !isAnalysisRunning && (m_ProfileSingleView.IsDataValid());
            if (GUILayout.Button("Save", buttonStyle, GUILayout.ExpandWidth(false), GUILayout.Width(50)))
                Save(m_ProfileSingleView);
            GUI.enabled = lastEnabled;

            ShowFilename(m_ProfileSingleView.path);
            EditorGUILayout.EndHorizontal();
        }

        bool IsSelectedMarkerValid()
        {
            bool valid = true;

            if (m_ActiveTab == ActiveTab.Summary)
            {
                if (IsAnalysisValid())
                {
                    valid = false;

                    List<MarkerData> markers = m_ProfileSingleView.analysis.GetMarkers();
                    if (markers != null)
                    {
                        int markerAt = m_SelectedMarker.id;
                        if (markerAt >= 0 && markerAt < markers.Count)
                        {
                            valid = true;
                        }
                    }
                }
            }
            else if (m_ActiveTab == ActiveTab.Compare)
            {
                if (IsAnalysisValid())
                {
                    valid = false;

                    List<MarkerData> leftMarkers = m_ProfileLeftView.analysis.GetMarkers();
                    List<MarkerData> rightMarkers = m_ProfileRightView.analysis.GetMarkers();
                    int pairingAt = m_SelectedPairing;
                    if (leftMarkers != null && rightMarkers != null && m_Pairings != null)
                    {
                        if (pairingAt >= 0 && pairingAt < m_Pairings.Count)
                        {
                            valid = true;
                        }
                    }
                }
            }

            return valid;
        }

        void ShowSelectedMarker()
        {
            bool valid = IsSelectedMarkerValid();

            if (valid)
            {
                DrawSelectedText(m_SelectedMarker.name);
            }
            else
            {
                var markerInThread = m_SelectedMarker.id == -1 && !string.IsNullOrEmpty(m_SelectedMarker.threadName);
                var threadText = markerInThread ?
                    string.Format(" (Selected in: {0}{1}{2})",
                    m_SelectedMarker.threadGroupName,
                    string.IsNullOrEmpty(m_SelectedMarker.threadGroupName) ? "" : ".",
                    m_SelectedMarker.threadName) :
                    null;
                string text = string.Format("{0}{1} not in selection", m_SelectedMarker.name, threadText);

                GUIContent content = new GUIContent(text, text);
                Vector2 size = GUI.skin.label.CalcSize(content);
                Rect rect = EditorGUILayout.GetControlRect(GUILayout.MaxWidth(size.x), GUILayout.Height(size.y));
                if (Event.current.type == EventType.Repaint)
                {
                    GUI.Label(rect, content);
                }
            }
        }

        internal bool AllSelected()
        {
            if (m_ActiveTab == ActiveTab.Summary)
            {
                if (m_ProfileSingleView.AllSelected())
                    return true;
            }
            if (m_ActiveTab == ActiveTab.Compare)
            {
                if (m_ProfileLeftView.AllSelected() && m_ProfileRightView.AllSelected())
                    return true;
            }

            return false;
        }

        internal bool HasSelection()
        {
            if (m_ActiveTab == ActiveTab.Summary)
            {
                if (m_ProfileSingleView.HasSelection())
                    return true;
            }
            if (m_ActiveTab == ActiveTab.Compare)
            {
                if (m_ProfileLeftView.HasSelection())
                    return true;
                if (m_ProfileRightView.HasSelection())
                    return true;
            }

            return false;
        }

        internal int GetRemappedUIFrameIndex(int frameIndex, ProfileDataView context)
        {
            if (context.inSyncWithProfilerData)
                return RemapFrameIndex(frameIndex, context.data.FrameIndexOffset);
            else
                return k_ProfileDataDefaultDisplayOffset + context.data.DisplayFrameToOffset(frameIndex);
        }

        internal bool CanExportComparisonTable()
        {
            return m_ProfileLeftView != null && m_ProfileLeftView.IsDataValid() && m_ProfileRightView != null && m_ProfileRightView.IsDataValid() &&
                m_ComparisonTable != null && m_ActiveTab == ActiveTab.Compare;
        }

        internal bool TryExportComparisonTable(StreamWriter writer)
        {
            if (m_ComparisonTable == null || m_ActiveTab != ActiveTab.Compare)
                return false;
            m_ComparisonTable.WriteTableContentsCSV(writer);
            return true;
        }

        int GetRemappedUIFirstFrameOffset(ProfileDataView context)
        {
            if (context.inSyncWithProfilerData)
                return RemapFrameIndex(context.data.OffsetToDisplayFrame(0), context.data.FrameIndexOffset);
            else
                return context.data.OffsetToDisplayFrame(0);
        }

        int GetRemappedUIFirstFrameDisplayOffset(ProfileDataView context)
        {
            if (context.inSyncWithProfilerData)
                return RemapFrameIndex(context.data.OffsetToDisplayFrame(0), context.data.FrameIndexOffset);
            else
                return k_ProfileDataDefaultDisplayOffset;
        }

        static readonly ProfilerMarkerAbstracted m_DrawFrameTimeGraphProfilerMarker = new ProfilerMarkerAbstracted("ProfileAnalyzer.DrawFrameTimeGraph");

        void DrawFrameTimeGraph(float height)
        {
            using (m_DrawFrameTimeGraphProfilerMarker.Auto())
            {
                GUI.SetNextControlName("FrameTimeGraph");
                Rect rect = EditorGUILayout.GetControlRect(GUILayout.Height(height));

                if (m_ProfileSingleView.IsDataValid())
                {
                    if (!m_FrameTimeGraph.HasData())
                        m_FrameTimeGraph.SetData(GetFrameTimeData(m_ProfileSingleView.data));
                    if (!m_ProfileSingleView.HasValidSelection())
                        m_ProfileSingleView.SelectFullRange();


                    List<int> selectedOffsets = new List<int>();
                    foreach (int index in m_ProfileSingleView.selectedIndices)
                    {
                        selectedOffsets.Add(m_ProfileSingleView.data.DisplayFrameToOffset(index));
                    }

                    float yRange = m_FrameTimeGraph.GetDataRange();
                    int offsetToDisplayMapping = GetRemappedUIFirstFrameDisplayOffset(m_ProfileSingleView);
                    int offsetToIndexMapping = GetRemappedUIFirstFrameOffset(m_ProfileSingleView);
                    bool enabled = !IsAnalysisRunning();
                    m_FrameTimeGraph.SetEnabled(enabled);

                    bool valid = IsSelectedMarkerValid();
                    string validMarkerName = valid ? m_SelectedMarker.name : "";

                    m_FrameTimeGraph.Draw(rect, m_ProfileSingleView.analysis, selectedOffsets, yRange, offsetToDisplayMapping, offsetToIndexMapping,
                        validMarkerName, 0, m_ProfileSingleView.analysisFull);

                    EditorGUILayout.BeginHorizontal();

                    GUILayout.FlexibleSpace();
                    ShowSelectedMarker();
                    EditorGUILayout.EndHorizontal();
                }
                else
                {
                    GUI.Label(rect, Styles.dataMissing, m_StyleUpperLeft);
                }
            }
        }

        void DrawParentFilter()
        {
            EditorGUILayout.BeginHorizontal();
            EditorGUILayout.LabelField(Styles.parentMarker, GUILayout.Width(100));

            if (!string.IsNullOrEmpty(m_ParentMarker))
            {
                bool lastEnabled = GUI.enabled;
                bool enabled = !IsAnalysisRunning();
                GUI.enabled = enabled;
                if (GUILayout.Button("Clear", EditorStyles.miniButton, GUILayout.MaxWidth(LayoutSize.FilterOptionsEnumWidth)))
                {
                    SetAsParentMarkerFilter("");
                }
                GUI.enabled = lastEnabled;

                DrawSelectedText(m_ParentMarker);
            }
            else
            {
                EditorGUILayout.LabelField(Styles.selectParentMarker);
            }

            EditorGUILayout.EndHorizontal();
        }

        internal void SetThreadSelection(ThreadSelection threadSelection)
        {
            m_ThreadSelectionNew = new ThreadSelection(threadSelection);

            UpdateActiveTab(true);
        }

        string CalculateSelectedThreadsSummary()
        {
            if (m_ThreadSelection.selection == null || m_ThreadSelection.selection.Count == 0)
                return "None";

            // Count all threads in a group
            var threadDict = new Dictionary<string, int>();
            var threadSelectionDict = new Dictionary<string, int>();
            foreach (var threadNameWithIndex in m_ThreadNames)
            {
                var threadIdentifier = new ThreadIdentifier(threadNameWithIndex);
                if (threadIdentifier.index == ThreadIdentifier.kAll)
                    continue;

                int count;
                if (threadDict.TryGetValue(threadIdentifier.name, out count))
                    threadDict[threadIdentifier.name] = count + 1;
                else
                    threadDict[threadIdentifier.name] = 1;

                threadSelectionDict[threadIdentifier.name] = 0;
            }

            // Count all the threads we have 'selected' in a group
            foreach (var threadNameWithIndex in m_ThreadSelection.selection)
            {
                var threadIdentifier = new ThreadIdentifier(threadNameWithIndex);

                if (threadDict.ContainsKey(threadIdentifier.name) &&
                    threadSelectionDict.ContainsKey(threadIdentifier.name) &&
                    threadIdentifier.index <= threadDict[threadIdentifier.name])
                {
                    // Selected thread valid and in the thread list
                    // and also within the range of valid threads for this data set
                    threadSelectionDict[threadIdentifier.name]++;
                }
            }

            // Count all thread groups where we have 'selected all the threads'
            int threadsSelected = 0;
            foreach (var threadName in threadDict.Keys)
            {
                if (threadSelectionDict[threadName] != threadDict[threadName])
                    continue;

                threadsSelected++;
            }

            // If we've just added all the thread names we have everything selected
            // Note we don't compare against the m_ThreadNames directly as this contains the 'all' versions
            if (threadsSelected == threadDict.Keys.Count)
                return "All";

            // Add all the individual threads were we haven't already added the group
            List<string> threads = new List<string>();
            foreach (var threadName in threadSelectionDict.Keys)
            {
                int selectionCount = threadSelectionDict[threadName];
                if (selectionCount <= 0)
                    continue;
                int threadCount = threadDict[threadName];
                if (threadCount == 1)
                    threads.Add(threadName);
                else if (selectionCount != threadCount)
                    threads.Add(string.Format("{0} ({1} of {2})", threadName, selectionCount, threadCount));
                else
                    threads.Add(string.Format("{0} (All)", threadName));
            }

            // Maintain alphabetical order
            threads.Sort(CompareUINames);

            if (threads.Count == 0)
                return "None";

            string threadsSelectedText = string.Join(", ", threads.ToArray());
            return threadsSelectedText;
        }

        string GetSelectedThreadsSummary()
        {
            return m_ThreadSelectionSummary;
        }

        void DrawThreadFilter(ProfileData profileData)
        {
            EditorGUILayout.BeginHorizontal();
            EditorGUILayout.LabelField(Styles.threadFilter, GUILayout.Width(LayoutSize.FilterOptionsLeftLabelWidth));
            if (profileData != null)
            {
                if (m_ThreadNames.Count > 0)
                {
                    bool lastEnabled = GUI.enabled;
                    bool enabled = !IsAnalysisRunning() && !ThreadSelectionWindow.IsOpen();
                    GUI.enabled = enabled;
                    if (GUILayout.Button(Styles.threadFilterSelect, GUILayout.Width(LayoutSize.FilterOptionsEnumWidth)))
                    {
                        Vector2 windowPosition = new Vector2(Event.current.mousePosition.x + LayoutSize.FilterOptionsEnumWidth, Event.current.mousePosition.y + GUI.skin.label.lineHeight);
                        Vector2 screenPosition = GUIUtility.GUIToScreenPoint(windowPosition);

                        ThreadSelectionWindow.Open(screenPosition.x, screenPosition.y, this, m_ThreadSelection, m_ThreadNames, m_ThreadUINames);
                        EditorGUIUtility.ExitGUI();
                    }

                    GUI.enabled = lastEnabled;
                    ShowSelectedThreads();
                    GUILayout.FlexibleSpace();
                }
            }

            EditorGUILayout.EndHorizontal();
        }

        void DrawSelectedText(string text)
        {
            if (text == null)
                return;

            GUIStyle treeViewSelectionStyle = "TV Selection";
            GUIStyle backgroundStyle = new GUIStyle(treeViewSelectionStyle);

            GUIStyle treeViewLineStyle = "TV Line";
            GUIStyle textStyle = new GUIStyle(treeViewLineStyle);

            GUIContent content = new GUIContent(text, text);
            Vector2 size = textStyle.CalcSize(content);
            Rect rect = EditorGUILayout.GetControlRect(GUILayout.MaxWidth(size.x), GUILayout.Height(size.y));
            if (Event.current.type == EventType.Repaint)
            {
                backgroundStyle.Draw(rect, false, false, true, true);
                GUI.Label(rect, content, textStyle);
            }
        }

        void ShowSelectedThreads()
        {
            string threadsSelected = GetSelectedThreadsSummary();

            DrawSelectedText(threadsSelected);
        }

        void DrawUnitFilter()
        {
            EditorGUILayout.BeginHorizontal(GUILayout.Width(LayoutSize.FilterOptionsRightLabelWidth + LayoutSize.FilterOptionsRightEnumWidth));
            EditorGUILayout.LabelField(Styles.unitFilter, m_StyleMiddleRight, GUILayout.Width(LayoutSize.FilterOptionsRightEnumWidth));

            bool lastEnabled = GUI.enabled;
            bool enabled = !IsAnalysisRunning();
            GUI.enabled = enabled;
            //Units units = (Units)EditorGUILayout.EnumPopup(m_DisplayUnits.Units, GUILayout.Width(LayoutSize.FilterOptionsRightEnumWidth));
            Units units = (Units)EditorGUILayout.Popup((int)m_DisplayUnits.Units, m_UnitNames, GUILayout.Width(LayoutSize.FilterOptionsRightEnumWidth));

            GUI.enabled = lastEnabled;
            if (units != m_DisplayUnits.Units)
            {
                SetUnits(units);
                m_FrameTimeGraph.SetUnits(m_DisplayUnits.Units);
                m_LeftFrameTimeGraph.SetUnits(m_DisplayUnits.Units);
                m_RightFrameTimeGraph.SetUnits(m_DisplayUnits.Units);
                UpdateMarkerTable();
            }
            EditorGUILayout.EndHorizontal();
        }

        bool IsSelfTime()
        {
            return (m_TimingOption == TimingOptions.TimingOption.Self) ? true : false;
        }

        void DrawTimingFilter()
        {
            EditorGUILayout.BeginHorizontal(GUILayout.Width(LayoutSize.FilterOptionsRightLabelWidth + LayoutSize.FilterOptionsRightEnumWidth));

            EditorGUILayout.LabelField(Styles.timingFilter, m_StyleMiddleRight, GUILayout.Width(LayoutSize.FilterOptionsRightLabelWidth));

            bool lastEnabled = GUI.enabled;
            bool enabled = !IsAnalysisRunning();
            GUI.enabled = enabled;
            var timingOption = (TimingOptions.TimingOption)EditorGUILayout.Popup((int)m_TimingOption, TimingOptions.TimingOptionNames, GUILayout.Width(LayoutSize.FilterOptionsRightEnumWidth));
            GUI.enabled = lastEnabled;
            if (timingOption != m_TimingOption)
            {
                m_TimingOption = timingOption;
                UpdateActiveTab(true, true);
            }
            EditorGUILayout.EndHorizontal();
        }

        internal string GetDisplayUnits()
        {
            return m_DisplayUnits.Postfix();
        }

        internal string ToDisplayUnits(float ms, bool showUnits = false, int limitToDigits = 5, bool showFullValueWhenBelowZero = false)
        {
            return m_DisplayUnits.ToString(ms, showUnits, limitToDigits, showFullValueWhenBelowZero);
        }

        internal string ToDisplayUnits(double ms, bool showUnits = false, int limitToDigits = 5, bool showFullValueWhenBelowZero = false)
        {
            return m_DisplayUnits.ToString((float)ms, showUnits, limitToDigits, showFullValueWhenBelowZero);
        }

        internal string ToTooltipDisplayUnits(float ms, bool showUnits = false, int frameIndex = -1)
        {
            return m_DisplayUnits.ToTooltipString(ms, showUnits, frameIndex);
        }

        internal GUIContent ToDisplayUnitsWithTooltips(float ms, bool showUnits = false, int frameIndex = -1)
        {
            return m_DisplayUnits.ToGUIContentWithTooltips(ms, showUnits, 5, frameIndex);
        }

        void SetUnits(Units units)
        {
            m_DisplayUnits = new DisplayUnits(units);
        }

        void UpdateActiveTab(bool fullAnalysisRequired = false, bool markOtherDirty = true)
        {
            switch (m_ActiveTab)
            {
                case ActiveTab.Summary:
                    m_RequestAnalysis = true;
                    m_FullAnalysisRequired = fullAnalysisRequired;
                    break;
                case ActiveTab.Compare:
                    m_RequestCompare = true;
                    m_FullCompareRequired = fullAnalysisRequired;
                    break;
            }

            if (markOtherDirty)
                m_OtherTabDirty = true;
        }

        void UpdateMarkerTable(bool markOtherDirty = true)
        {
            switch (m_ActiveTab)
            {
                case ActiveTab.Summary:
                    if (m_ProfileTable != null)
                        m_ProfileTable.Reload();
                    break;
                case ActiveTab.Compare:
                    if (m_ComparisonTable != null)
                        m_ComparisonTable.Reload();
                    break;
            }

            if (markOtherDirty)
                m_OtherTableDirty = true;
        }

        void DrawNameFilter()
        {
            EditorGUILayout.BeginHorizontal();
            EditorGUILayout.LabelField(Styles.nameFilter, GUILayout.Width(LayoutSize.FilterOptionsLeftLabelWidth));

            NameFilterOperation lastNameFilterOperation = m_NameFilterOperation;

            bool lastEnabled = GUI.enabled;
            bool enabled = !IsAnalysisRunning();
            GUI.enabled = enabled;
            m_NameFilterOperation = (NameFilterOperation)EditorGUILayout.Popup((int)m_NameFilterOperation, Styles.nameFilterOperation, GUILayout.MaxWidth(LayoutSize.FilterOptionsEnumWidth));
            GUI.enabled = lastEnabled;

            if (m_NameFilterOperation != lastNameFilterOperation)
            {
                m_MarkerFilter.Clear();
                UpdateMarkerTable();
            }
            string lastFilter = m_NameFilter;
            GUI.enabled = enabled;
            GUI.SetNextControlName("NameFilter");
            m_NameFilter = EditorGUILayout.TextField(m_NameFilter, GUILayout.MinWidth(200 - LayoutSize.FilterOptionsEnumWidth));
            GUI.enabled = lastEnabled;
            if (m_NameFilter != lastFilter)
            {
                m_MarkerFilter.Clear();
                UpdateMarkerTable();
            }

            EditorGUILayout.LabelField(Styles.nameExclude, GUILayout.Width(LayoutSize.FilterOptionsLeftLabelWidth));
            NameFilterOperation lastNameExcludeOperation = m_NameExcludeOperation;
            GUI.enabled = enabled;
            m_NameExcludeOperation = (NameFilterOperation)EditorGUILayout.Popup((int)m_NameExcludeOperation, Styles.nameFilterOperation, GUILayout.MaxWidth(LayoutSize.FilterOptionsEnumWidth));
            GUI.enabled = lastEnabled;
            if (m_NameExcludeOperation != lastNameExcludeOperation)
            {
                m_MarkerFilter.Clear();
                UpdateMarkerTable();
            }
            string lastExclude = m_NameExclude;
            GUI.enabled = enabled;
            GUI.SetNextControlName("ExcludeFilter");
            m_NameExclude = EditorGUILayout.TextField(m_NameExclude, GUILayout.MinWidth(200 - LayoutSize.FilterOptionsEnumWidth));
            GUI.enabled = lastEnabled;
            if (m_NameExclude != lastExclude)
            {
                m_MarkerFilter.Clear();
                UpdateMarkerTable();
            }
            EditorGUILayout.EndHorizontal();
        }

        internal void SetMode(MarkerColumnFilter.Mode newMode)
        {
            m_SingleModeFilter.mode = newMode;
            m_CompareModeFilter.mode = newMode;

            if (m_ProfileTable != null)
                m_ProfileTable.SetMode(m_SingleModeFilter);
            if (m_ComparisonTable != null)
                m_ComparisonTable.SetMode(m_CompareModeFilter);
        }

        internal void SetSingleModeColumns(int[] visibleColumns)
        {
            // If selecting the columns manually then override the currently stored selection with the current
            m_ProfileMulticolumnHeaderState.visibleColumns = visibleColumns;

            m_SingleModeFilter.mode = MarkerColumnFilter.Mode.Custom;
            m_SingleModeFilter.visibleColumns = visibleColumns;
        }

        internal void SetComparisonModeColumns(int[] visibleColumns)
        {
            // If selecting the columns manually then override the currently stored selection with the current
            m_ComparisonMulticolumnHeaderState.visibleColumns = visibleColumns;

            m_CompareModeFilter.mode = MarkerColumnFilter.Mode.Custom;
            m_CompareModeFilter.visibleColumns = visibleColumns;
        }

        void DrawMarkerColumnFilter()
        {
            EditorGUILayout.BeginHorizontal(GUILayout.Width(LayoutSize.FilterOptionsRightLabelWidth + LayoutSize.FilterOptionsRightEnumWidth));
            EditorGUILayout.LabelField(Styles.markerColumns, m_StyleMiddleRight, GUILayout.Width(LayoutSize.FilterOptionsRightLabelWidth));

            bool lastEnabled = GUI.enabled;
            bool enabled = !IsAnalysisRunning();
            GUI.enabled = enabled;

            var filterMode = m_ActiveTab == ActiveTab.Summary ? m_SingleModeFilter : m_CompareModeFilter;

            var oldMode = filterMode.mode;
            filterMode.mode = (MarkerColumnFilter.Mode)EditorGUILayout.IntPopup((int)filterMode.mode, MarkerColumnFilter.ModeNames, MarkerColumnFilter.ModeValues, GUILayout.Width(LayoutSize.FilterOptionsRightEnumWidth));
            if (filterMode.mode != oldMode)
            {
                if (m_ActiveTab == ActiveTab.Summary && m_ProfileTable != null)
                    m_ProfileTable.SetMode(filterMode);
                else if (m_ActiveTab == ActiveTab.Compare && m_ComparisonTable != null)
                    m_ComparisonTable.SetMode(filterMode);
            }

            GUI.enabled = lastEnabled;

            EditorGUILayout.EndHorizontal();
        }

        enum InDataSet
        {
            Left,
            Both,
            Right
        };

        int GetCombinedThreadCount(out int matchingCount, out int uniqueLeft, out int uniqueRight)
        {
            var threads = new Dictionary<string, InDataSet>();
            foreach (var threadName in m_ProfileLeftView.data.GetThreadNames())
            {
                threads[threadName] = InDataSet.Left;
            }
            foreach (var threadName in m_ProfileRightView.data.GetThreadNames())
            {
                if (threads.ContainsKey(threadName))
                    threads[threadName] = InDataSet.Both;
                else
                    threads[threadName] = InDataSet.Right;
            }

            matchingCount = 0;
            uniqueLeft = 0;
            uniqueRight = 0;
            int total = 0;
            foreach (var thread in threads)
            {
                switch (thread.Value)
                {
                    case InDataSet.Left:
                        uniqueLeft++;
                        break;
                    case InDataSet.Both:
                        matchingCount++;
                        break;
                    case InDataSet.Right:
                        uniqueRight++;
                        break;
                }
                total++;
            }

            return total;
        }

        void DrawMarkerCount()
        {
            if (!IsAnalysisValid())
                return;

            if (m_ActiveTab == ActiveTab.Summary)
            {
                int markersCount = m_ProfileSingleView.analysis.GetFrameSummary().totalMarkers;
                int filteredMarkersCount = (m_ProfileTable != null) ? m_ProfileTable.GetRows().Count : 0;

                var content = new GUIContent(
                    String.Format("{0} of {1} markers", filteredMarkersCount, markersCount),
                    "Number of markers in the filtered set, compared to the total in the data set");
                Vector2 size = GUI.skin.label.CalcSize(content);
                Rect rect = EditorGUILayout.GetControlRect(GUILayout.Width(size.x), GUILayout.Height(size.y));
                EditorGUI.LabelField(rect, content);
            }
            if (m_ActiveTab == ActiveTab.Compare)
            {
                int markersCount = m_TotalCombinedMarkerCount;
                int filteredMarkersCount = (m_ComparisonTable != null) ? m_ComparisonTable.GetRows().Count : 0;

                var content = new GUIContent(
                    String.Format("{0} of {1} markers", filteredMarkersCount, markersCount),
                    "Number of markers in the filtered set, compared to total unique markers in the combined data sets");
                Vector2 size = GUI.skin.label.CalcSize(content);
                Rect rect = EditorGUILayout.GetControlRect(GUILayout.Width(size.x), GUILayout.Height(size.y));
                EditorGUI.LabelField(rect, content);
            }
        }

        string GetThreadCountToolTipUnion(int allThreadsCount, int matchingCount)
        {
            return String.Format(
                "Total\n{0} Union : Combined over both data sets\n{1} Intersection : Matching in both data sets",
                allThreadsCount,
                matchingCount
            );
        }

        string GetThreadCountToolTipDifference(int allThreadsCount, int matchingCount, int uniqueLeft, int uniqueRight)
        {
            return String.Format(
                "Difference\n{0}\n{1} Unique to left\n{2} Unique to right",
                allThreadsCount - matchingCount,
                uniqueLeft,
                uniqueRight);
        }

        string GetThreadCountToolTip(int allThreadsCount, int matchingCount, int uniqueLeft, int uniqueRight)
        {
            return String.Format(
                "{0}\n\n{1}",
                GetThreadCountToolTipUnion(allThreadsCount, matchingCount),
                GetThreadCountToolTipDifference(allThreadsCount, matchingCount, uniqueLeft, uniqueRight)
            );
        }

        void DrawThreadCount()
        {
            if (!IsAnalysisValid())
                return;

            if (m_ActiveTab == ActiveTab.Summary)
            {
                int allThreadsCount = m_ProfileSingleView.data.GetThreadNames().Count;
                List<string> threadSelection = GetLimitedThreadSelection(m_ThreadNames, m_ThreadSelection);
                int selectedThreads = threadSelection.Count;

                var content = new GUIContent(
                    String.Format("{0} of {1} threads", selectedThreads, allThreadsCount),
                    "Number of threads in the filtered set, compared to the total in the data set");
                Vector2 size = GUI.skin.label.CalcSize(content);
                Rect rect = EditorGUILayout.GetControlRect(GUILayout.Width(size.x), GUILayout.Height(size.y));
                EditorGUI.LabelField(rect, content);
            }
            if (m_ActiveTab == ActiveTab.Compare)
            {
                int matchingCount, uniqueLeft, uniqueRight;
                int allThreadsCount = GetCombinedThreadCount(out matchingCount, out uniqueLeft, out uniqueRight);
                List<string> threadSelection = GetLimitedThreadSelection(m_ThreadNames, m_ThreadSelection);
                int selectedThreads = threadSelection.Count;

                string partialTooltip = GetThreadCountToolTip(allThreadsCount, matchingCount, uniqueLeft, uniqueRight);

                var content = new GUIContent(
                    String.Format("{0} of {1} threads", selectedThreads, allThreadsCount),
                    String.Format("Number of threads in the filtered set, compared to total unique threads in the combined data sets\n\n{0}", partialTooltip)
                );
                Vector2 size = GUI.skin.label.CalcSize(content);
                Rect rect = EditorGUILayout.GetControlRect(GUILayout.Width(size.x), GUILayout.Height(size.y));
                EditorGUI.LabelField(rect, content);
            }
        }

        static readonly ProfilerMarkerAbstracted m_DrawAnalysisOptionsProfilerMarker = new ProfilerMarkerAbstracted("ProfileAnalyzer.DrawAnalysisOptions");

        void UpdateRemoveMarkerDisplay()
        {
            UpdateActiveTab(true);
            // Force an update
            m_FrameTimeGraph.ClearData();
            m_LeftFrameTimeGraph.ClearData();
            m_RightFrameTimeGraph.ClearData();
        }

        void BuildRemoveMarkerList()
        {
            var entries = new List<GUIContent>();
            var values = new List<int>();
            m_removeMarkerSomeMissing = false;
            foreach (RemoveMarkerOperation filterOperation in Enum.GetValues(typeof(RemoveMarkerOperation)))
            {
                int value = (int)filterOperation;
                GUIContent entry = new GUIContent(Styles.removeMarkerOperation[value]);

                bool found = true;
                switch(filterOperation)
                {
                    case RemoveMarkerOperation.HideWaitForFPS:
                        {
                            switch (m_ActiveTab)
                            {
                                case ActiveTab.Summary:
                                    if (!m_ProfileSingleView.containsWaitForFPS)
                                        found = false;
                                    break;
                                case ActiveTab.Compare:
                                    if (!m_ProfileLeftView.containsWaitForFPS && !m_ProfileRightView.containsWaitForFPS)
                                        found = false;
                                    break;
                            }
                        }
                        break;
                    case RemoveMarkerOperation.HideWaitForPresent:
                        {
                            switch (m_ActiveTab)
                            {
                                case ActiveTab.Summary:
                                    if (!m_ProfileSingleView.containsWaitForPresent)
                                        found = false;
                                    break;
                                case ActiveTab.Compare:
                                    if (!m_ProfileLeftView.containsWaitForPresent && !m_ProfileRightView.containsWaitForPresent)
                                        found = false;
                                    break;
                            }
                        }
                        break;
                }

                if (!found)
                {
                    entry.text += " (not in capture)";
                    m_removeMarkerSomeMissing = true;
                }
                entries.Add(entry);
                values.Add(value);
            }

            m_removeMarkerDisplay = entries.ToArray();
            m_removeMarkerValues = values.ToArray();
        }


        void DrawRemoveMarker()
        {
            bool update = false;

            EditorGUILayout.BeginHorizontal();
            EditorGUILayout.LabelField(Styles.removeMarker, GUILayout.Width(LayoutSize.FilterOptionsLeftLabelWidth));

            bool lastEnabled = GUI.enabled;
            bool enabled = !IsAnalysisRunning();
            GUI.enabled = enabled;

            if (m_removeMarkerDisplay == null || m_removeMarkerValues == null)
                BuildRemoveMarkerList();

            // Hack to make popup wider if some markers missing, and have extra text to indicate that
            int width = (m_removeMarkerSomeMissing ? LayoutSize.RemoveMarkerMissingOptionsEnumWidth : LayoutSize.RemoveMarkerOptionsEnumWidth);
            RemoveMarkerOperation removeMarkerOperation = (RemoveMarkerOperation)EditorGUILayout.IntPopup((int)m_removeMarkerOperation, m_removeMarkerDisplay, m_removeMarkerValues, GUILayout.Width(width));
            if (removeMarkerOperation != m_removeMarkerOperation)
                update = true;

            if (m_removeMarkerOperation != RemoveMarkerOperation.ShowAll)
            {
                bool hide = EditorGUILayout.Toggle(Styles.hideRemoveMarkers, m_hideRemovedMarkers, GUILayout.ExpandWidth(false));
                if (hide != m_hideRemovedMarkers)
                {
                    m_hideRemovedMarkers = hide;
                    update = true;
                }
            }

            switch (m_removeMarkerOperation)
            {
                case RemoveMarkerOperation.ShowAll:
                    EditorGUILayout.LabelField("");
                    break;
/*
                // Could make custom marker editable, but we don't support multiple markers yet
                // So currently we limit this to being set by right click context menu
                case removeMarkerOperation.Custom:
                    string removeMarkerCustomRemoveMarker = EditorGUILayout.DelayedTextField(m_removeMarkerCustomRemoveMarker, GUILayout.MinWidth(200 - LayoutSize.removeMarkerOptionsEnumWidth));
                    if (removeMarkerCustomRemoveMarker != m_removeMarkerCustomRemoveMarker)
                    {
                        m_removeMarkerCustomRemoveMarker = removeMarkerCustomRemoveMarker;
                        update = true;
                    }
                    break;
*/
                default:
                    EditorGUILayout.LabelField(GetRemoveMarker());
                    break;
            }
            GUI.enabled = lastEnabled;
            EditorGUILayout.EndHorizontal();

            if (update)
            {
                m_removeMarkerOperation = removeMarkerOperation;
                UpdateRemoveMarkerDisplay();
            }
        }

        internal void SetAsRemoveMarker(string markerName)
        {
            if (markerName == "")
            {
                if (m_removeMarkerOperation != RemoveMarkerOperation.ShowAll)
                {
                    m_removeMarkerOperation = RemoveMarkerOperation.ShowAll;
                    UpdateRemoveMarkerDisplay();
                }
            }
            else
            {
                m_removeMarkerCustomRemoveMarker = markerName;
                m_removeMarkerOperation = RemoveMarkerOperation.Custom;
                UpdateRemoveMarkerDisplay();
            }
        }

        string GetRemoveMarker()
        {
            switch (m_removeMarkerOperation)
            {
                default:
                case RemoveMarkerOperation.ShowAll:
                    return null;
                case RemoveMarkerOperation.HideWaitForFPS:
                    // Gfx.WaitForPresentOnGfxThread is not always present on Android 
                    // E.g. when Application.targetFrameRate locks the frame rate
                    // So we support removing WaitForTargetFPS
                    // Often WaitForTargetFPS will be negligable on console and Gfx.WaitForPresentOnGfxThread is key
                    return "WaitForTargetFPS";
                case RemoveMarkerOperation.HideWaitForPresent:
                    // Gfx.WaitForPresentOnGfxThread is seen on consoles
                    return "Gfx.WaitForPresentOnGfxThread";
                case RemoveMarkerOperation.Custom:
                    if (m_removeMarkerCustomRemoveMarker == null || m_removeMarkerCustomRemoveMarker=="")
                        m_removeMarkerCustomRemoveMarker = "WaitForTargetFPS";
                    return m_removeMarkerCustomRemoveMarker;
            }
        }

        void DrawAnalysisOptions()
        {
            using (m_DrawAnalysisOptionsProfilerMarker.Auto())
            {
                EditorGUILayout.BeginVertical(GUI.skin.box);

                bool lastShowFilters = m_ShowFilters;
                m_ShowFilters = BoldFoldout(m_ShowFilters, Styles.filters);
                var analytic = ProfileAnalyzerAnalytics.BeginAnalytic();
                if (m_ShowFilters)
                {
                    DrawRemoveMarker();

                    DrawNameFilter();
                    EditorGUILayout.BeginHorizontal();
                    DrawThreadFilter(m_ProfileSingleView.data);
                    EditorGUILayout.EndHorizontal();

                    EditorGUILayout.BeginHorizontal();
                    m_DepthSliceUI.DrawDepthFilter(IsAnalysisRunning(), m_ActiveTab == ActiveTab.Summary,
                        m_ProfileSingleView, m_ProfileLeftView, m_ProfileRightView);
                    DrawTimingFilter();
                    EditorGUILayout.EndHorizontal();

                    EditorGUILayout.BeginHorizontal();
                    DrawParentFilter();
                    DrawUnitFilter();
                    EditorGUILayout.EndHorizontal();

                    EditorGUILayout.BeginHorizontal();
                    bool lastEnabled = GUI.enabled;
                    GUI.enabled = !IsAnalysisRunning();
                    if (GUILayout.Button(new GUIContent("Analyze", m_LastAnalysisTime), GUILayout.Width(100)))
                        m_RequestAnalysis = true;
                    GUI.enabled = lastEnabled;
                    DrawMarkerCount();
                    EditorGUILayout.LabelField(",", GUILayout.Width(10), GUILayout.ExpandWidth(false));
                    DrawThreadCount();
                    GUILayout.FlexibleSpace();
                    DrawMarkerColumnFilter();
                    EditorGUILayout.EndHorizontal();
                }

                if (m_ShowFilters != lastShowFilters)
                {
                    ProfileAnalyzerAnalytics.SendUIVisibilityEvent(ProfileAnalyzerAnalytics.UIVisibility.Filters,
                        analytic.GetDurationInSeconds(), m_ShowFilters);
                }

                EditorGUILayout.EndVertical();
            }
        }

        internal bool IsAnalysisRunning()
        {
            return m_ThreadActivity != ThreadActivity.None;
        }

        internal bool IsLoading()
        {
            return m_ThreadActivity == ThreadActivity.Load;
        }

        float GetProgress()
        {
            // We return the value from the update loop so the data doesn't change over the time onGui is called for layout and repaint
            // m_ThreadProgress ranges from 0 to 100.
            return m_ThreadProgress * 0.01f;
        }

        bool IsAnalysisValid(ProfileDataView view, bool checkFrameCount = false)
        {
            if (!view.IsDataValid())
                return false;

            if (view.analysis == null)
                return false;

            if (checkFrameCount)
            {
                if (view.analysis.GetFrameSummary().frames.Count <= 0)
                    return false;
            }

            return true;
        }

        bool IsAnalysisValid(bool checkFrameCount = false)
        {
            switch (m_ActiveTab)
            {
                case ActiveTab.Summary:
                    if (!IsAnalysisValid(m_ProfileSingleView, checkFrameCount))
                        return false;
                    break;

                case ActiveTab.Compare:
                    if (!IsAnalysisValid(m_ProfileLeftView, checkFrameCount))
                        return false;
                    if (!IsAnalysisValid(m_ProfileRightView, checkFrameCount))
                        return false;
                    break;
            }

            //if (IsAnalysisRunning())
            //    return false;

            return true;
        }

        void DrawProgress(Rect rect)
        {
            if (IsAnalysisRunning())
            {
                EditorGUILayout.BeginHorizontal(GUILayout.MaxWidth(140));
                float x = 0;
                float y = 0;
                float width = rect.width;
                float height = k_ProgressBarHeight;
                if (m_2D.DrawStart(width, height, Draw2D.Origin.TopLeft))
                {
                    float barLength = width * GetProgress();
                    m_2D.DrawFilledBox(x, y, barLength, height, UIColor.white);
                    m_2D.DrawEnd();
                }
                EditorGUILayout.EndHorizontal();
            }
            else
            {
                EditorGUILayout.BeginVertical();
                GUILayout.Space(k_ProgressBarHeight);
                EditorGUILayout.EndVertical();
            }
        }

        void DrawPullButton(Color color, ProfileDataView view, FrameTimeGraph frameTimeGraph)
        {
            bool lastEnabled = GUI.enabled;
            GUI.enabled = !IsAnalysisRunning();

            GUIContent content;
            if (!IsProfilerWindowOpen())
            {
                content = Styles.pullOpen;
                GUI.enabled = false;
            }
            else if (m_ProfilerFirstFrameIndex == 0 && m_ProfilerLastFrameIndex == 0)
            {
                content = Styles.pullRange;
                GUI.enabled = false;
            }
            /*
            // Commented out so we can capture even if recording
            else if (m_ProfilerWindowInterface.IsRecording())
            {
                content = Styles.pullRecording;
                GUI.enabled = false;
            }
            */
            else
            {
                content = Styles.pull;
            }

            Color oldColor = GUI.backgroundColor;
            GUI.backgroundColor = color;
            bool pull = GUILayout.Button(content, GUILayout.Width(100));
            GUI.backgroundColor = oldColor;
            if (pull)
            {
                PullFromProfiler(m_ProfilerFirstFrameIndex, m_ProfilerLastFrameIndex, view, frameTimeGraph);
                UpdateActiveTab(true, false);
            }
            GUI.enabled = lastEnabled;
        }

        static readonly ProfilerMarkerAbstracted m_DrawFilesLoadedProfilerMarker = new ProfilerMarkerAbstracted("ProfileAnalyzer.DrawFilesLoaded");

        void DrawFilesLoaded()
        {
            using (m_DrawFilesLoadedProfilerMarker.Auto())
            {
                var boxStyle = GUI.skin.box;
                var rect = EditorGUILayout.BeginVertical(boxStyle);

                if (m_ActiveTab == ActiveTab.Summary)
                {
                    EditorGUILayout.BeginHorizontal(GUILayout.Height(100 + GUI.skin.label.lineHeight +
                        (2 * (GUI.skin.label.margin.vertical +
                            GUI.skin.label.padding.vertical))));

                    float filenameWidth = GetFilenameWidth(m_ProfileSingleView.path);
                    filenameWidth = Math.Min(filenameWidth, 200);
                    EditorGUILayout.BeginVertical(GUILayout.MaxWidth(100 + filenameWidth),
                        GUILayout.ExpandWidth(false));
                    DrawPullButton(GUI.backgroundColor, m_ProfileSingleView, m_FrameTimeGraph);
                    DrawLoadSave();
                    EditorGUILayout.EndVertical();

                    EditorGUILayout.BeginVertical(GUILayout.ExpandWidth(true));
                    DrawFrameTimeGraph(100);
                    EditorGUILayout.EndVertical();

                    EditorGUILayout.EndHorizontal();
                }

                if (m_ActiveTab == ActiveTab.Compare)
                {
                    DrawComparisonLoadSave();
                }

                rect.width -= boxStyle.margin.right;
                DrawProgress(rect);

                EditorGUILayout.EndVertical();
            }
        }

        void ShowHelp()
        {
            EditorGUILayout.BeginVertical(GUI.skin.box);

            m_HelpScroll = EditorGUILayout.BeginScrollView(m_HelpScroll, GUILayout.ExpandHeight(true));
            GUILayout.TextArea(Styles.helpText);
            EditorGUILayout.EndScrollView();

            EditorGUILayout.EndVertical();
        }

        static readonly ProfilerMarkerAbstracted m_DrawAnalysisProfilerMarker = new ProfilerMarkerAbstracted("ProfileAnalyzer.DrawAnalysis");
        static readonly ProfilerMarkerAbstracted m_TopNMarkersProfilerMarker = new ProfilerMarkerAbstracted("ProfileAnalyzer.TopNMarkers");
        static readonly ProfilerMarkerAbstracted m_DrawMarkerTableProfilerMarker = new ProfilerMarkerAbstracted("ProfileAnalyzer.DrawMarkerTable");
        void DrawAnalysis()
        {
            using (m_DrawAnalysisProfilerMarker.Auto())
            {
                EditorGUILayout.BeginHorizontal();

                EditorGUILayout.BeginVertical();

                DrawFilesLoaded();

                if (m_ProfileSingleView.IsDataValid() && m_ProfileSingleView.data.GetFrameCount() > 0)
                {
                    DrawAnalysisOptions();

                    if (IsAnalysisValid())
                    {
                        EditorGUILayout.BeginVertical(GUI.skin.box);

                        string title = string.Format("Top {0} markers on median frame", m_TopNBars);
                        GUIContent markersTitle = new GUIContent(title, Styles.topMarkersTooltip);
                        bool lastShowTopMarkers = m_ShowTopNMarkers;
                        m_ShowTopNMarkers = BoldFoldout(m_ShowTopNMarkers, markersTitle);
                        var analytic = ProfileAnalyzerAnalytics.BeginAnalytic();
                        if (m_ShowTopNMarkers)
                        {
                            using (m_TopNMarkersProfilerMarker.Auto())
                            {
                                m_TopMarkers.SetData(m_ProfileSingleView, m_DepthSliceUI.depthFilter, GetNameFilters(),
                                    GetNameExcludes(), m_TimingOption, m_ThreadSelection.selection.Count, m_hideRemovedMarkers);

                                EditorGUILayout.BeginVertical(GUILayout.Height(20));

                                EditorGUILayout.BeginHorizontal();
                                FrameSummary frameSummary = m_ProfileSingleView.analysis.GetFrameSummary();
                                if (frameSummary.count > 0)
                                    DrawFrameIndexButton(frameSummary.medianFrameIndex, m_ProfileSingleView);
                                else
                                    GUILayout.Label("", GUILayout.MinWidth(50));

                                Rect rect = EditorGUILayout.GetControlRect(GUILayout.ExpandWidth(true),
                                    GUILayout.ExpandHeight(true));
                                float range = m_TopMarkers.GetTopMarkerTimeRange();
                                m_TopMarkers.Draw(rect, UIColor.bar, m_TopNBars, range, UIColor.barBackground,
                                    Color.black, Color.white, true, true);
                                EditorGUILayout.EndHorizontal();
                                EditorGUILayout.EndVertical();

                                EditorGUILayout.BeginVertical(GUILayout.Height(20));
                                GUILayout.Label(m_DepthSliceUI.GetUIInfo(false));
                                EditorGUILayout.EndVertical();
                            }
                        }

                        if (m_ShowTopNMarkers != lastShowTopMarkers)
                        {
                            ProfileAnalyzerAnalytics.SendUIVisibilityEvent(ProfileAnalyzerAnalytics.UIVisibility.TopTen,
                                analytic.GetDurationInSeconds(), m_ShowTopNMarkers);
                        }

                        EditorGUILayout.EndVertical();

                        if (m_ProfileTable != null)
                        {
                            m_ShowMarkerTable = BoldFoldout(m_ShowMarkerTable, Styles.profileTable);
                            if (m_ShowMarkerTable)
                            {
                                using (m_DrawMarkerTableProfilerMarker.Auto())
                                {
                                    Rect r = EditorGUILayout.GetControlRect(GUILayout.ExpandHeight(true));

                                    float scrollBarWidth = GUI.skin.verticalScrollbar.fixedWidth +
                                        GUI.skin.verticalScrollbar.border.horizontal +
                                        GUI.skin.verticalScrollbar.margin.horizontal +
                                        GUI.skin.verticalScrollbar.padding.horizontal;
                                    scrollBarWidth += LayoutSize.ScrollBarPadding;

                                    //offset vertically to get correct clipping behaviour
                                    Rect clipRect = new Rect(r.x, m_ProfileTable.state.scrollPos.y,
                                        r.width - scrollBarWidth,
                                        r.height -
                                        (m_ProfileTable.multiColumnHeader.height + GUI.skin.box.padding.top) -
                                        (m_ProfileTable.ShowingHorizontalScroll
                                            ? (scrollBarWidth - LayoutSize.ScrollBarPadding)
                                            : 0));
                                    m_2D.SetClipRect(clipRect);
                                    m_ProfileTable.OnGUI(r);
                                    m_2D.ClearClipRect();
                                }
                            }
                        }
                    }
                }
                else
                {
                    ShowHelp();
                }

                EditorGUILayout.EndVertical();

                EditorGUILayout.BeginVertical(GUILayout.Width(LayoutSize.WidthRHS));
                GUILayout.Space(4);
                DrawFrameSummary();
                DrawThreadSummary();
                DrawSelected();
                EditorGUILayout.EndVertical();

                EditorGUILayout.EndHorizontal();
            }
        }

        void SetRange(List<int> selectedOffsets, int clickCount, FrameTimeGraph.State inputStatus, ProfileDataView mainData, List<int> selectedIndices)
        {
            if (inputStatus == FrameTimeGraph.State.Dragging)
                return;

            var data = mainData.data;
            if (clickCount == 2)
            {
                if (mainData.inSyncWithProfilerData)
                {
                    int index = data.OffsetToDisplayFrame(selectedOffsets[0]);
                    JumpToFrame(index, mainData.data, false);
                }
            }
            else
            {
                selectedIndices.Clear();
                foreach (int offset in selectedOffsets)
                {
                    selectedIndices.Add(data.OffsetToDisplayFrame(offset));
                }
                // Keep indices sorted
                selectedIndices.Sort();

                m_RequestCompare = true;
            }
        }

        void SetLeftRange(List<int> selectedOffsets, int clickCount, FrameTimeGraph.State inputStatus)
        {
            SetRange(selectedOffsets, clickCount, inputStatus, m_ProfileLeftView, m_ProfileLeftView.selectedIndices);
        }

        void SetRightRange(List<int> selectedOffsets, int clickCount, FrameTimeGraph.State inputStatus)
        {
            SetRange(selectedOffsets, clickCount, inputStatus, m_ProfileRightView, m_ProfileRightView.selectedIndices);
        }

        void DrawComparisonLoadSaveButton(Color color, ProfileDataView view, FrameTimeGraph frameTimeGraph, ActiveView activeView)
        {
            bool lastEnabled = GUI.enabled;
            bool isAnalysisRunning = IsAnalysisRunning();

            EditorGUILayout.BeginHorizontal(GUILayout.MaxWidth(300), GUILayout.ExpandWidth(false));

            GUIStyle buttonStyle = GUI.skin.button;
            Color oldColor = GUI.backgroundColor;

            GUI.backgroundColor = color;
            GUI.enabled = !isAnalysisRunning;
            bool load = GUILayout.Button("Load", buttonStyle, GUILayout.ExpandWidth(false), GUILayout.Width(50));
            GUI.enabled = lastEnabled;
            GUI.backgroundColor = oldColor;
            if (load)
            {
                m_Path = EditorUtility.OpenFilePanel("Load profile analyzer data file", "", "pdata");
                if (m_Path.Length != 0)
                {
                    m_ActiveLoadingView = activeView;
                    BeginAsyncAction(ThreadActivity.Load);
                }
                GUIUtility.ExitGUI();
            }

            GUI.backgroundColor = color;
            GUI.enabled = !isAnalysisRunning && view.IsDataValid();
            bool save = GUILayout.Button("Save", buttonStyle, GUILayout.ExpandWidth(false), GUILayout.Width(50));
            GUI.enabled = lastEnabled;
            GUI.backgroundColor = oldColor;
            if (save)
            {
                Save(view, true);
            }

            ShowFilename(view.path);

            EditorGUILayout.EndHorizontal();
        }

        float GetComparisonYRange()
        {
            float yRangeLeft = m_ProfileLeftView.IsDataValid() ? m_LeftFrameTimeGraph.GetDataRange() : 0f;
            float yRangeRight = m_ProfileRightView.IsDataValid() ? m_RightFrameTimeGraph.GetDataRange() : 0f;
            float yRange = Math.Max(yRangeLeft, yRangeRight);

            return yRange;
        }

        void SetFrameTimeGraphPairing(bool paired)
        {
            if (paired != m_FrameTimeGraphsPaired)
            {
                m_FrameTimeGraphsPaired = paired;
                m_LeftFrameTimeGraph.PairWith(m_FrameTimeGraphsPaired ? m_RightFrameTimeGraph : null);
            }
        }

        void DrawComparisonLoadSave()
        {
            int leftFrames = m_ProfileLeftView.IsDataValid() ? m_ProfileLeftView.data.GetFrameCount() : 0;
            int rightFrames = m_ProfileRightView.IsDataValid() ? m_ProfileRightView.data.GetFrameCount() : 0;
            int maxFrames = Math.Max(leftFrames, rightFrames);

            float yRange = GetComparisonYRange();

            EditorGUILayout.BeginHorizontal(GUILayout.Height(100 + GUI.skin.label.lineHeight + (2 * (GUI.skin.label.margin.vertical + GUI.skin.label.padding.vertical))));

            float leftFilenameWidth = GetFilenameWidth(m_ProfileLeftView.path);
            float rightFilenameWidth = GetFilenameWidth(m_ProfileRightView.path);
            float filenameWidth = Math.Max(leftFilenameWidth, rightFilenameWidth);
            filenameWidth = Math.Min(filenameWidth, 200);

            EditorGUILayout.BeginVertical(GUILayout.MaxWidth(100 + filenameWidth), GUILayout.ExpandWidth(false));
            DrawPullButton(UIColor.left, m_ProfileLeftView, m_LeftFrameTimeGraph);
            DrawComparisonLoadSaveButton(UIColor.left, m_ProfileLeftView, m_LeftFrameTimeGraph, ActiveView.Left);
            DrawPullButton(UIColor.right, m_ProfileRightView, m_RightFrameTimeGraph);
            DrawComparisonLoadSaveButton(UIColor.right, m_ProfileRightView, m_RightFrameTimeGraph, ActiveView.Right);
            EditorGUILayout.EndVertical();


            bool lastEnabled = GUI.enabled;
            bool enabled = !IsAnalysisRunning();

            EditorGUILayout.BeginVertical(GUILayout.ExpandWidth(true));

            bool valid = IsSelectedMarkerValid();
            string validMarkerName = valid ? m_SelectedMarker.name : "";

            GUI.SetNextControlName("LeftFrameTimeGraph");
            Rect rect = EditorGUILayout.GetControlRect(GUILayout.Height(50));
            if (m_ProfileLeftView.IsDataValid())
            {
                if (!m_LeftFrameTimeGraph.HasData())
                    m_LeftFrameTimeGraph.SetData(GetFrameTimeData(m_ProfileLeftView.data));
                if (!m_ProfileLeftView.HasValidSelection())
                    m_ProfileLeftView.SelectFullRange();

                List<int> selectedOffsets = new List<int>();
                foreach (int index in m_ProfileLeftView.selectedIndices)
                {
                    selectedOffsets.Add(m_ProfileLeftView.data.DisplayFrameToOffset(index));
                }

                int offsetToDisplayMapping = GetRemappedUIFirstFrameDisplayOffset(m_ProfileLeftView);
                int offsetToIndexMapping = GetRemappedUIFirstFrameOffset(m_ProfileLeftView);

                m_LeftFrameTimeGraph.SetEnabled(enabled);
                m_LeftFrameTimeGraph.Draw(rect, m_ProfileLeftView.analysis, selectedOffsets, yRange, offsetToDisplayMapping, offsetToIndexMapping, validMarkerName, maxFrames, m_ProfileLeftView.analysisFull);
            }
            else
            {
                GUI.Label(rect, Styles.comparisonDataMissing, m_StyleUpperLeft);
            }

            GUI.SetNextControlName("RightFrameTimeGraph");
            rect = EditorGUILayout.GetControlRect(GUILayout.Height(50));
            if (m_ProfileRightView.IsDataValid())
            {
                if (!m_RightFrameTimeGraph.HasData())
                    m_RightFrameTimeGraph.SetData(GetFrameTimeData(m_ProfileRightView.data));
                if (!m_ProfileRightView.HasValidSelection())
                    m_ProfileRightView.SelectFullRange();

                List<int> selectedOffsets = new List<int>();
                foreach (int index in m_ProfileRightView.selectedIndices)
                {
                    selectedOffsets.Add(m_ProfileRightView.data.DisplayFrameToOffset(index));
                }

                int offsetToDisplayMapping = GetRemappedUIFirstFrameDisplayOffset(m_ProfileRightView);
                int offsetToIndexMapping = GetRemappedUIFirstFrameOffset(m_ProfileRightView);

                m_RightFrameTimeGraph.SetEnabled(enabled);
                m_RightFrameTimeGraph.Draw(rect, m_ProfileRightView.analysis, selectedOffsets, yRange, offsetToDisplayMapping, offsetToIndexMapping, validMarkerName, maxFrames, m_ProfileRightView.analysisFull);
            }
            else
            {
                GUI.Label(rect, Styles.comparisonDataMissing, m_StyleUpperLeft);
            }

            EditorGUILayout.BeginHorizontal();
            if (m_ProfileLeftView.IsDataValid() && m_ProfileRightView.IsDataValid() && m_ProfileLeftView.data.GetFrameCount() > 0 && m_ProfileRightView.data.GetFrameCount() > 0)
            {
                GUIStyle lockButtonStyle = "IN LockButton";
                GUIStyle style = new GUIStyle(lockButtonStyle);
                style.padding.left = 20;
                //bool paired = GUILayout.Toggle(m_frameTimeGraphsPaired, Styles.graphPairing, style);
                GUI.enabled = enabled;
                bool paired = EditorGUILayout.ToggleLeft(Styles.graphPairing, m_FrameTimeGraphsPaired, style, GUILayout.MaxWidth(200));
                GUI.enabled = lastEnabled;
                SetFrameTimeGraphPairing(paired);

                GUILayout.FlexibleSpace();
                ShowSelectedMarker();
            }
            EditorGUILayout.EndHorizontal();

            EditorGUILayout.EndVertical();
            EditorGUILayout.EndHorizontal();
        }

        void DrawComparisonHistogram(float height, float minValue, float maxValue, int bucketCount, int[] leftBuckets, int[] rightBuckets, int leftCount, int rightCount, bool leftValid, bool rightValid, DisplayUnits displayUnits)
        {
            Histogram histogram = new Histogram(m_2D, displayUnits.Units);
            float width = LayoutSize.HistogramWidth;
            float min = minValue;
            float max = maxValue;
            float spacing = 2;

            float range = max - min;
            // bucketCount = (range == 0f) ? 1 : bucketCount;

            float x = (spacing / 2);
            float y = 0;
            float w = ((width + spacing) / bucketCount) - spacing;
            float h = height;

            histogram.DrawStart(width);

            if (m_2D.DrawStart(width, height, Draw2D.Origin.BottomLeft))
            {
                float bucketWidth = (range / bucketCount);
                Rect rect = GUILayoutUtility.GetLastRect();

                histogram.DrawBackground(width, height, bucketCount, min, max, spacing);

                if (!IsAnalysisRunning())
                {
                    for (int bucketAt = 0; bucketAt < bucketCount; bucketAt++)
                    {
                        float leftBarCount = leftValid ? leftBuckets[bucketAt] : 0;
                        float rightBarCount = rightValid ? rightBuckets[bucketAt] : 0;
                        float leftBarHeight = leftValid ? ((h * leftBarCount) / leftCount) : 0;
                        float rightBarHeight = rightValid ? ((h * rightBarCount) / rightCount) : 0;

                        if (leftBarCount > 0)  // Make sure we always slow a small bar if non zero
                            leftBarHeight = Mathf.Max(1.0f, leftBarHeight);
                        if (rightBarCount > 0)  // Make sure we always slow a small bar if non zero
                            rightBarHeight = Mathf.Max(1.0f, rightBarHeight);

                        if ((int)rightBarHeight == (int)leftBarHeight)
                        {
                            m_2D.DrawFilledBox(x, y, w, leftBarHeight, UIColor.both);
                        }
                        else if (rightBarHeight > leftBarHeight)
                        {
                            m_2D.DrawFilledBox(x, y, w, rightBarHeight, UIColor.right);
                            m_2D.DrawFilledBox(x, y, w, leftBarHeight, UIColor.both);
                        }
                        else
                        {
                            m_2D.DrawFilledBox(x, y, w, leftBarHeight, UIColor.left);
                            m_2D.DrawFilledBox(x, y, w, rightBarHeight, UIColor.both);
                        }

                        float bucketStart = min + (bucketAt * bucketWidth);
                        float bucketEnd = bucketStart + bucketWidth;

                        string tooltip = string.Format(
                            "{0}-{1}\nLeft: {2} {3}\nRight: {4} {5}\n\nBar width: {6}",
                            displayUnits.ToTooltipString(bucketStart, false),
                            displayUnits.ToTooltipString(bucketEnd, true),
                            leftBarCount, leftBarCount == 1 ? "frame" : "frames",
                            rightBarCount, rightBarCount == 1 ? "frame" : "frames",
                            displayUnits.ToTooltipString(bucketWidth, true));
                        GUI.Label(new Rect(rect.x + x, rect.y + y, w, h),
                            new GUIContent("", tooltip)
                        );

                        x += w;
                        x += spacing;
                    }
                }

                m_2D.DrawEnd();
            }

            histogram.DrawEnd(width, min, max, spacing);
        }

        void DrawComparisonFrameSummary()
        {
            EditorGUILayout.BeginVertical(GUI.skin.box, GUILayout.Width(LayoutSize.WidthRHS));

            bool lastShowFrameSummary = m_ShowFrameSummary;
            m_ShowFrameSummary = BoldFoldout(m_ShowFrameSummary, Styles.frameSummary);
            var analytic = ProfileAnalyzerAnalytics.BeginAnalytic();
            if (m_ShowFrameSummary)
            {
                EditorGUILayout.BeginVertical(); // To match indenting on the marker summary where a scroll area is present

                if (IsAnalysisValid())
                {
                    var leftFrameSummary = m_ProfileLeftView.analysis.GetFrameSummary();
                    var rightFrameSummary = m_ProfileRightView.analysis.GetFrameSummary();

                    m_Columns.SetColumnSizes(LayoutSize.WidthColumn0, LayoutSize.WidthColumn1, LayoutSize.WidthColumn2, LayoutSize.WidthColumn3);
                    m_Columns.Draw4("", "Left", "Right", "Diff");

                    int diff = rightFrameSummary.count - leftFrameSummary.count;
                    m_Columns.Draw4(Styles.frameCount, GetFrameCountText(m_ProfileLeftView), GetFrameCountText(m_ProfileRightView), new GUIContent(diff.ToString(), ""));
                    m_Columns.Draw3(Styles.frameStart, GetFirstFrameText(m_ProfileLeftView), GetFirstFrameText(m_ProfileRightView));
                    m_Columns.Draw3(Styles.frameEnd, GetLastFrameText(m_ProfileLeftView), GetLastFrameText(m_ProfileRightView));

                    m_Columns.Draw(0, "");
                    string units = GetDisplayUnits();
                    m_Columns.Draw4("", units, units, units);

                    Draw4DiffMs(Styles.max, leftFrameSummary.msMax, leftFrameSummary.maxFrameIndex, rightFrameSummary.msMax, rightFrameSummary.maxFrameIndex);
                    Draw4DiffMs(Styles.upperQuartile, leftFrameSummary.msUpperQuartile, rightFrameSummary.msUpperQuartile);
                    Draw4DiffMs(Styles.median, leftFrameSummary.msMedian, leftFrameSummary.medianFrameIndex, rightFrameSummary.msMedian, rightFrameSummary.medianFrameIndex);
                    Draw4DiffMs(Styles.mean, leftFrameSummary.msMean, rightFrameSummary.msMean);
                    Draw4DiffMs(Styles.lowerQuartile, leftFrameSummary.msLowerQuartile, rightFrameSummary.msLowerQuartile);
                    Draw4DiffMs(Styles.min, leftFrameSummary.msMin, leftFrameSummary.minFrameIndex, rightFrameSummary.msMin, rightFrameSummary.minFrameIndex);

                    GUIStyle style = GUI.skin.label;
                    GUILayout.Space(style.lineHeight);

                    EditorGUILayout.BeginHorizontal();
                    int leftBucketCount = leftFrameSummary.buckets.Length;
                    int rightBucketCount = rightFrameSummary.buckets.Length;

                    float msFrameMax = Math.Max(leftFrameSummary.msMax, rightFrameSummary.msMax);
                    float yRange = msFrameMax;

                    if (leftBucketCount != rightBucketCount)
                    {
                        Debug.Log("Error left frame summary bucket count doesn't equal right summary");
                    }
                    else
                    {
                        DrawComparisonHistogram(40, 0, yRange, leftBucketCount, leftFrameSummary.buckets, rightFrameSummary.buckets, leftFrameSummary.count, rightFrameSummary.count, true, true, m_DisplayUnits);
                    }

                    BoxAndWhiskerPlot boxAndWhiskerPlot = new BoxAndWhiskerPlot(m_2D, m_DisplayUnits.Units);

                    float plotWidth = 40 + GUI.skin.box.padding.horizontal;
                    float plotHeight = 40;
                    plotWidth /= 2.0f;
                    boxAndWhiskerPlot.Draw(plotWidth, plotHeight, leftFrameSummary.msMin, leftFrameSummary.msLowerQuartile,
                        leftFrameSummary.msMedian, leftFrameSummary.msUpperQuartile, leftFrameSummary.msMax, 0, yRange,
                        UIColor.boxAndWhiskerLineColorLeft, UIColor.boxAndWhiskerBoxColorLeft);
                    boxAndWhiskerPlot.Draw(plotWidth, plotHeight, rightFrameSummary.msMin, rightFrameSummary.msLowerQuartile,
                        rightFrameSummary.msMedian, rightFrameSummary.msUpperQuartile, rightFrameSummary.msMax, 0, yRange,
                        UIColor.boxAndWhiskerLineColorRight, UIColor.boxAndWhiskerBoxColorRight);

                    boxAndWhiskerPlot.DrawText(m_Columns.GetColumnWidth(3), plotHeight, 0, yRange,
                        "Min frame time for selected frames in the 2 data sets",
                        "Max frame time for selected frames in the 2 data sets");

                    EditorGUILayout.EndHorizontal();
                }
                else
                {
                    EditorGUILayout.LabelField("No analysis data selected");
                }

                EditorGUILayout.EndVertical();
            }

            if (m_ShowFrameSummary != lastShowFrameSummary)
            {
                ProfileAnalyzerAnalytics.SendUIVisibilityEvent(ProfileAnalyzerAnalytics.UIVisibility.Frames, analytic.GetDurationInSeconds(), m_ShowFrameSummary);
            }

            EditorGUILayout.EndVertical();
        }

        void ShowThreadRange()
        {
            EditorGUILayout.BeginHorizontal();
            m_Columns.Draw(0, Styles.threadGraphScale);
            m_ThreadRange = (ThreadRange)EditorGUILayout.Popup((int)m_ThreadRange, Styles.threadRanges, GUILayout.Width(160));
            EditorGUILayout.EndHorizontal();
        }

        float GetThreadTimeRange(ProfileAnalysis profileAnalysis)
        {
            if (profileAnalysis == null)
                return 0.0f;

            var frameSummary = profileAnalysis.GetFrameSummary();
            float range = frameSummary.msMax;
            switch (m_ThreadRange)
            {
                case ThreadRange.Median:
                    range = frameSummary.msMedian;
                    break;
                case ThreadRange.UpperQuartile:
                    range = frameSummary.msUpperQuartile;
                    break;
                case ThreadRange.Max:
                    range = frameSummary.msMax;
                    break;
            }

            return range;
        }

        int GetThreadSelectionCount(out int leftSelectionCount, out int rightSelectionCount)
        {
            List<string> threadSelection = GetLimitedThreadSelection(m_ThreadNames, m_ThreadSelection);
            leftSelectionCount = 0;
            foreach (var threadName in m_ProfileLeftView.data.GetThreadNames())
            {
                if (threadSelection.Contains(threadName))
                {
                    leftSelectionCount++;
                }
            }
            rightSelectionCount = 0;
            foreach (var threadName in m_ProfileRightView.data.GetThreadNames())
            {
                if (threadSelection.Contains(threadName))
                {
                    rightSelectionCount++;
                }
            }
            return threadSelection.Count;
        }

        void DrawComparisonThreadSummary()
        {
            EditorGUILayout.BeginVertical(GUI.skin.box, GUILayout.Width(LayoutSize.WidthRHS));

            bool lastShowThreadSummary = m_ShowThreadSummary;
            m_ShowThreadSummary = BoldFoldout(m_ShowThreadSummary, Styles.threadSummary);
            var analytic = ProfileAnalyzerAnalytics.BeginAnalytic();
            if (m_ShowThreadSummary)
            {
                EditorGUILayout.BeginVertical(); // To match indenting on the marker summary where a scroll area is present

                if (IsAnalysisValid())
                {
                    m_Columns.SetColumnSizes(LayoutSize.WidthColumn0, LayoutSize.WidthColumn1, LayoutSize.WidthColumn2, LayoutSize.WidthColumn3);
                    EditorGUILayout.BeginHorizontal();
                    m_Columns.Draw4("", "Left", "Right", "Total");
                    EditorGUILayout.EndHorizontal();

                    int matchingCount, uniqueLeft, uniqueRight;
                    int allThreadsCount = GetCombinedThreadCount(out matchingCount, out uniqueLeft, out uniqueRight);

                    EditorGUILayout.BeginHorizontal();
                    m_Columns.Draw(0, "Total Count : ");
                    m_Columns.Draw(1, new GUIContent(m_ProfileLeftView.data.GetThreadCount().ToString(), "Total threads in left data set"));
                    m_Columns.Draw(2, new GUIContent(m_ProfileRightView.data.GetThreadCount().ToString(), "Total threads in right data set"));
                    string tooltip = GetThreadCountToolTipUnion(allThreadsCount, matchingCount);
                    m_Columns.Draw(3, new GUIContent(allThreadsCount.ToString(), tooltip));
                    EditorGUILayout.EndHorizontal();

                    EditorGUILayout.BeginHorizontal();
                    m_Columns.Draw(0, "Unique Count : ");
                    m_Columns.Draw(1, new GUIContent(uniqueLeft.ToString(), "Unique to left data set"));
                    m_Columns.Draw(2, new GUIContent(uniqueRight.ToString(), "Unique to right data set"));
                    tooltip = GetThreadCountToolTipDifference(allThreadsCount, matchingCount, uniqueLeft, uniqueRight);
                    m_Columns.Draw(3, new GUIContent((allThreadsCount - matchingCount).ToString(), tooltip));
                    EditorGUILayout.EndHorizontal();

                    int leftSelectionCount, rightSelectionCount;
                    int selectedThreads = GetThreadSelectionCount(out leftSelectionCount, out rightSelectionCount);

                    EditorGUILayout.BeginHorizontal();
                    m_Columns.Draw(0, "Selected : ");
                    m_Columns.Draw(1, new GUIContent(leftSelectionCount.ToString(), "Left selected"));
                    m_Columns.Draw(2, new GUIContent(rightSelectionCount.ToString(), "Right selected"));
                    m_Columns.Draw(3, new GUIContent(selectedThreads.ToString(), "Total selected"));
                    EditorGUILayout.EndHorizontal();

                    m_Columns.SetColumnSizes(LayoutSize.WidthColumn0, LayoutSize.WidthColumn1, LayoutSize.WidthColumn2 + LayoutSize.WidthColumn3, 0);
                    ShowThreadRange();

                    float width = 100;
                    float height = GUI.skin.label.lineHeight;

                    float xAxisMin = 0.0f;
                    float xAxisMax = GetThreadTimeRange(m_ProfileLeftView.analysis);

                    m_Columns.Draw3(Styles.emptyString, Styles.median, Styles.thread);

                    m_ThreadScroll = EditorGUILayout.BeginScrollView(m_ThreadScroll, GUIStyle.none, GUI.skin.verticalScrollbar);
                    Rect clipRect = new Rect(m_ThreadScroll.x, m_ThreadScroll.y, m_ComparisonThreadsAreaRect.width, m_ComparisonThreadsAreaRect.height);
                    m_2D.SetClipRect(clipRect);
                    for (int i = 0; i < m_ThreadUINames.Count; i++)
                    {
                        string threadNameWithIndex = m_ThreadNames[i];

                        bool include = ProfileAnalyzer.MatchThreadFilter(threadNameWithIndex, m_ThreadSelection.selection);
                        if (!include)
                            continue;

                        ThreadData threadLeft = m_ProfileLeftView.analysis.GetThreadByName(threadNameWithIndex);
                        ThreadData threadRight = m_ProfileRightView.analysis.GetThreadByName(threadNameWithIndex);

                        ThreadData thread = threadLeft != null ? threadLeft : threadRight;
                        if (thread == null)
                            continue;

                        bool singleThread = thread.threadsInGroup > 1 ? false : true;

                        BoxAndWhiskerPlot boxAndWhiskerPlot = new BoxAndWhiskerPlot(m_2D, m_DisplayUnits.Units);
                        EditorGUILayout.BeginHorizontal();
                        if (threadLeft != null)
                            boxAndWhiskerPlot.DrawHorizontal(width, height, threadLeft.msMin, threadLeft.msLowerQuartile, threadLeft.msMedian, threadLeft.msUpperQuartile, threadLeft.msMax, xAxisMin, xAxisMax, UIColor.boxAndWhiskerLineColorLeft, UIColor.boxAndWhiskerBoxColorLeft, GUI.skin.label);
                        else
                            EditorGUILayout.LabelField(Styles.noThread, GUILayout.Width(width));
                        m_Columns.Draw(1, (threadLeft != null) ? ToDisplayUnitsWithTooltips(threadLeft.msMedian) : Styles.noThread);
                        m_Columns.Draw(2, GetThreadNameWithGroupTooltip(thread.threadNameWithIndex, singleThread));
                        EditorGUILayout.EndHorizontal();

                        EditorGUILayout.BeginHorizontal();
                        if (threadRight != null)
                            boxAndWhiskerPlot.DrawHorizontal(width, height, threadRight.msMin, threadRight.msLowerQuartile, threadRight.msMedian, threadRight.msUpperQuartile, threadRight.msMax, xAxisMin, xAxisMax, UIColor.boxAndWhiskerLineColorRight, UIColor.boxAndWhiskerBoxColorRight, GUI.skin.label);
                        else
                            EditorGUILayout.LabelField(Styles.noThread, GUILayout.Width(width));
                        m_Columns.Draw(1, (threadRight != null) ? ToDisplayUnitsWithTooltips(threadRight.msMedian) : Styles.noThread);
                        m_Columns.Draw(2, "");
                        EditorGUILayout.EndHorizontal();
                    }
                    m_2D.ClearClipRect();
                    EditorGUILayout.EndScrollView();

                    if (Event.current.type == EventType.Repaint)
                    {
                        // This value is not valid at layout phase
                        m_ComparisonThreadsAreaRect = GUILayoutUtility.GetLastRect();
                    }
                }
                else
                {
                    EditorGUILayout.LabelField("No analysis data selected");
                }

                EditorGUILayout.EndVertical();
            }

            if (m_ShowThreadSummary != lastShowThreadSummary)
            {
                ProfileAnalyzerAnalytics.SendUIVisibilityEvent(ProfileAnalyzerAnalytics.UIVisibility.Threads, analytic.GetDurationInSeconds(), m_ShowThreadSummary);
            }

            EditorGUILayout.EndVertical();
        }

        static readonly ProfilerMarkerAbstracted m_DrawCompareOptionsProfilerMarker = new ProfilerMarkerAbstracted("ProfileAnalyzer.DrawCompareOptions");

        void DrawCompareOptions()
        {
            using (m_DrawCompareOptionsProfilerMarker.Auto())
            {
                EditorGUILayout.BeginVertical(GUI.skin.box);

                bool lastShowFilters = m_ShowFilters;
                m_ShowFilters = BoldFoldout(m_ShowFilters, Styles.filters);
                var analytic = ProfileAnalyzerAnalytics.BeginAnalytic();
                if (m_ShowFilters)
                {
                    DrawRemoveMarker();

                    DrawNameFilter();
                    EditorGUILayout.BeginHorizontal();
                    DrawThreadFilter(m_ProfileLeftView.data);
                    EditorGUILayout.EndHorizontal();

                    EditorGUILayout.BeginHorizontal();
                    m_DepthSliceUI.DrawDepthFilter(IsAnalysisRunning(), m_ActiveTab == ActiveTab.Summary,
                        m_ProfileSingleView, m_ProfileLeftView, m_ProfileRightView);
                    DrawTimingFilter();
                    EditorGUILayout.EndHorizontal();

                    EditorGUILayout.BeginHorizontal();
                    DrawParentFilter();
                    DrawUnitFilter();
                    EditorGUILayout.EndHorizontal();

                    EditorGUILayout.BeginHorizontal();
                    if (m_ProfileLeftView.IsDataValid() && m_ProfileRightView.IsDataValid())
                    {
                        bool lastEnabled = GUI.enabled;
                        GUI.enabled = !IsAnalysisRunning();
                        if (GUILayout.Button(new GUIContent("Compare", m_LastCompareTime), GUILayout.Width(100)))
                            m_RequestCompare = true;
                        GUI.enabled = lastEnabled;
                    }

                    DrawMarkerCount();
                    EditorGUILayout.LabelField(",", GUILayout.Width(10), GUILayout.ExpandWidth(false));
                    DrawThreadCount();
                    GUILayout.FlexibleSpace();
                    DrawMarkerColumnFilter();
                    EditorGUILayout.EndHorizontal();
                }

                if (m_ShowFilters != lastShowFilters)
                {
                    ProfileAnalyzerAnalytics.SendUIVisibilityEvent(ProfileAnalyzerAnalytics.UIVisibility.Filters,
                        analytic.GetDurationInSeconds(), m_ShowFilters);
                }

                EditorGUILayout.EndVertical();
            }
        }

        static readonly ProfilerMarkerAbstracted m_DrawComparisonProfilerMarker = new ProfilerMarkerAbstracted("ProfileAnalyzer.DrawComparison");
        static readonly ProfilerMarkerAbstracted m_DrawComparisonTableProfilerMarker = new ProfilerMarkerAbstracted("ProfileAnalyzer.DrawComparisonTable");

        void DrawComparison()
        {
            using (m_DrawComparisonProfilerMarker.Auto())
            {
                EditorGUILayout.BeginHorizontal();

                EditorGUILayout.BeginVertical();

                DrawFilesLoaded();

                if (m_ProfileLeftView.IsDataValid() && m_ProfileRightView.IsDataValid() &&
                    m_ProfileLeftView.data.GetFrameCount() > 0 && m_ProfileRightView.data.GetFrameCount() > 0)
                {
                    DrawCompareOptions();

                    if (m_ComparisonTable != null)
                    {
                        EditorGUILayout.BeginVertical(GUI.skin.box);

                        string title = string.Format("Top {0} markers on median frames", m_TopNBars);
                        GUIContent markersTitle = new GUIContent(title, Styles.topMarkersTooltip);

                        bool lastShowTopMarkers = m_ShowTopNMarkers;
                        m_ShowTopNMarkers = BoldFoldout(m_ShowTopNMarkers, markersTitle);
                        var analytic = ProfileAnalyzerAnalytics.BeginAnalytic();
                        if (m_ShowTopNMarkers)
                        {
                            using (m_TopNMarkersProfilerMarker.Auto())
                            {
                                EditorGUILayout.BeginVertical(GUILayout.Height(40));
                                Rect rect = EditorGUILayout.GetControlRect(GUILayout.ExpandWidth(true),
                                    GUILayout.ExpandHeight(true));
                                rect.height = rect.height / 2;

                                var nameFilters = GetNameFilters();
                                var nameExcludes = GetNameExcludes();

                                m_TopMarkersLeft.SetData(m_ProfileLeftView, m_DepthSliceUI.depthFilter1, nameFilters,
                                    nameExcludes, m_TimingOption, m_ThreadSelection.selection.Count, m_hideRemovedMarkers);
                                m_TopMarkersRight.SetData(m_ProfileRightView, m_DepthSliceUI.depthFilter2, nameFilters,
                                    nameExcludes, m_TimingOption, m_ThreadSelection.selection.Count, m_hideRemovedMarkers);


                                float leftRange = m_TopMarkersLeft.GetTopMarkerTimeRange();
                                float rightRange = m_TopMarkersRight.GetTopMarkerTimeRange();
                                if (m_TopTenDisplay == TopTenDisplay.LongestTime)
                                {
                                    float max = Math.Max(leftRange, rightRange);
                                    leftRange = max;
                                    rightRange = max;
                                }

                                int leftMedian = 0;
                                int rightMedian = 0;
                                if (m_ProfileLeftView.analysis != null)
                                {
                                    FrameSummary frameSummary = m_ProfileLeftView.analysis.GetFrameSummary();
                                    if (frameSummary.count > 0)
                                        leftMedian = frameSummary.medianFrameIndex;
                                }

                                if (m_ProfileRightView.analysis != null)
                                {
                                    FrameSummary frameSummary = m_ProfileRightView.analysis.GetFrameSummary();
                                    if (frameSummary.count > 0)
                                        rightMedian = frameSummary.medianFrameIndex;
                                }

                                int maxMedian = Math.Max(leftMedian, rightMedian);

                                Rect frameIndexRect = new Rect(rect);
                                Vector2 size =
                                    GUI.skin.button.CalcSize(new GUIContent(string.Format("{0}", maxMedian)));
                                frameIndexRect.width =
                                    Math.Max(size.x, 50); // DrawFrameIndexButton should always be at least 50 wide

                                if (leftMedian != 0f)
                                    DrawFrameIndexButton(frameIndexRect, leftMedian, m_ProfileLeftView);
                                else
                                    GUI.Label(frameIndexRect, "");

                                float padding = 2;
                                rect.x += frameIndexRect.width + padding;
                                rect.width -= frameIndexRect.width;
                                m_TopMarkersLeft.Draw(rect, UIColor.left, m_TopNBars, leftRange, UIColor.barBackground,
                                    Color.black, Color.white, true, true);
                                rect.y += rect.height;

                                frameIndexRect.y += rect.height;
                                if (rightMedian != 0f)
                                    DrawFrameIndexButton(frameIndexRect, rightMedian, m_ProfileRightView);
                                else
                                    GUI.Label(frameIndexRect, "");

                                m_TopMarkersRight.Draw(rect, UIColor.right, m_TopNBars, rightRange,
                                    UIColor.barBackground,
                                    Color.black, Color.white, true, true);
                                EditorGUILayout.EndVertical();

                                EditorGUILayout.BeginHorizontal();
                                GUILayout.Label(m_DepthSliceUI.GetUIInfo(true), GUILayout.ExpandWidth(true));
                                GUILayout.Label(Styles.topMarkerRatio, GUILayout.ExpandWidth(false));
                                m_TopTenDisplay = (TopTenDisplay)EditorGUILayout.Popup((int)m_TopTenDisplay, Styles.topTenDisplayOptions, GUILayout.MaxWidth(100));
                                EditorGUILayout.EndHorizontal();
                            }
                        }

                        if (m_ShowTopNMarkers != lastShowTopMarkers)
                        {
                            ProfileAnalyzerAnalytics.SendUIVisibilityEvent(
                                ProfileAnalyzerAnalytics.UIVisibility.Markers, analytic.GetDurationInSeconds(),
                                m_ShowTopNMarkers);
                        }

                        EditorGUILayout.EndVertical();

                        if (m_ComparisonTable != null)
                        {
                            m_ShowMarkerTable = BoldFoldout(m_ShowMarkerTable, Styles.comparisonTable);
                            if (m_ShowMarkerTable)
                            {
                                using (m_DrawComparisonTableProfilerMarker.Auto())
                                {
                                    Rect r = EditorGUILayout.GetControlRect(GUILayout.ExpandHeight(true));

                                    float scrollBarWidth = GUI.skin.verticalScrollbar.fixedWidth +
                                        GUI.skin.verticalScrollbar.border.horizontal +
                                        GUI.skin.verticalScrollbar.margin.horizontal +
                                        GUI.skin.verticalScrollbar.padding.horizontal;
                                    scrollBarWidth += LayoutSize.ScrollBarPadding;

                                    //offset vertically to get correct clipping behaviour
                                    Rect clipRect = new Rect(r.x, m_ComparisonTable.state.scrollPos.y,
                                        r.width - scrollBarWidth,
                                        r.height - (m_ComparisonTable.multiColumnHeader.height + GUI.skin.box.padding.top) -
                                        (m_ComparisonTable.ShowingHorizontalScroll
                                            ? (scrollBarWidth - LayoutSize.ScrollBarPadding)
                                            : 0));
                                    m_2D.SetClipRect(clipRect);
                                    m_ComparisonTable.OnGUI(r);
                                    m_2D.ClearClipRect();
                                }
                            }
                        }
                    }
                }
                else
                {
                    ShowHelp();
                }

                EditorGUILayout.EndVertical();

                EditorGUILayout.BeginVertical(GUILayout.Width(LayoutSize.WidthRHS));
                GUILayout.Space(4);
                DrawComparisonFrameSummary();
                DrawComparisonThreadSummary();
                DrawComparisonSelected();
                EditorGUILayout.EndVertical();

                EditorGUILayout.EndHorizontal();
            }
        }

        bool BoldFoldout(bool toggle, GUIContent content)
        {
            GUIStyle foldoutStyle = new GUIStyle(EditorStyles.foldout);
            foldoutStyle.fontStyle = FontStyle.Bold;
            return EditorGUILayout.Foldout(toggle, content, true, foldoutStyle);
        }

        void DrawComparisonSelectedStats(MarkerData leftMarker, MarkerData rightMarker)
        {
            GUIStyle style = GUI.skin.label;

            string units = GetDisplayUnits();
            m_Columns.Draw4("", units, units, units);
            Draw4DiffMs(Styles.max, MarkerData.GetMsMax(leftMarker), MarkerData.GetMaxFrameIndex(leftMarker), MarkerData.GetMsMax(rightMarker), MarkerData.GetMaxFrameIndex(rightMarker));
            Draw4DiffMs(Styles.upperQuartile, MarkerData.GetMsUpperQuartile(leftMarker), MarkerData.GetMsUpperQuartile(rightMarker));
            Draw4DiffMs(Styles.median, MarkerData.GetMsMedian(leftMarker), MarkerData.GetMedianFrameIndex(leftMarker), MarkerData.GetMsMedian(rightMarker), MarkerData.GetMedianFrameIndex(rightMarker));
            Draw4DiffMs(Styles.mean, MarkerData.GetMsMean(leftMarker), MarkerData.GetMsMean(rightMarker));
            Draw4DiffMs(Styles.lowerQuartile, MarkerData.GetMsLowerQuartile(leftMarker), MarkerData.GetMsLowerQuartile(rightMarker));
            Draw4DiffMs(Styles.min, MarkerData.GetMsMin(leftMarker), MarkerData.GetMinFrameIndex(leftMarker), MarkerData.GetMsMin(rightMarker), MarkerData.GetMinFrameIndex(rightMarker));

            GUILayout.Space(style.lineHeight);

            Draw4DiffMs(Styles.individualMax, MarkerData.GetMsMaxIndividual(leftMarker), MarkerData.GetMsMaxIndividual(rightMarker));
            Draw4DiffMs(Styles.individualMin, MarkerData.GetMsMinIndividual(leftMarker), MarkerData.GetMsMinIndividual(rightMarker));
        }

        void DrawComparisonSelected()
        {
            EditorGUILayout.BeginVertical(GUI.skin.box, GUILayout.Width(LayoutSize.WidthRHS));

            GUIStyle style = GUI.skin.label;

            bool lastMarkerSummary = m_ShowMarkerSummary;
            m_ShowMarkerSummary = BoldFoldout(m_ShowMarkerSummary, Styles.markerSummary);
            var analytic = ProfileAnalyzerAnalytics.BeginAnalytic();
            if (m_ShowMarkerSummary)
            {
                EditorGUILayout.BeginVertical(); // To match indenting on the marker summary where a scroll area is present

                if (IsAnalysisValid())
                {
                    List<MarkerData> leftMarkers = m_ProfileLeftView.analysis.GetMarkers();
                    List<MarkerData> rightMarkers = m_ProfileRightView.analysis.GetMarkers();
                    int pairingAt = m_SelectedPairing;
                    if (leftMarkers != null && rightMarkers != null && m_Pairings != null)
                    {
                        if (pairingAt >= 0 && pairingAt < m_Pairings.Count)
                        {
                            m_MarkerSummaryScroll = GUILayout.BeginScrollView(m_MarkerSummaryScroll, GUIStyle.none, GUI.skin.verticalScrollbar);
                            Rect clipRect = new Rect(m_MarkerSummaryScroll.x, m_MarkerSummaryScroll.y, LayoutSize.WidthRHS, 500);
                            m_2D.SetClipRect(clipRect);

                            EditorGUILayout.BeginVertical();

                            var pairing = m_Pairings[pairingAt];

                            var leftMarker = (pairing.leftIndex >= 0 && pairing.leftIndex < leftMarkers.Count) ? leftMarkers[pairing.leftIndex] : null;
                            var rightMarker = (pairing.rightIndex >= 0 && pairing.rightIndex < rightMarkers.Count) ? rightMarkers[pairing.rightIndex] : null;

                            EditorGUILayout.LabelField(pairing.name,
                                GUILayout.MaxWidth(LayoutSize.WidthRHS -
                                    (GUI.skin.box.padding.horizontal + GUI.skin.box.margin.horizontal)));
                            DrawComparisonFrameRatio(leftMarker, rightMarker);

                            m_Columns.SetColumnSizes(LayoutSize.WidthColumn0, LayoutSize.WidthColumn1, LayoutSize.WidthColumn2, LayoutSize.WidthColumn3);

                            EditorGUILayout.BeginHorizontal();
                            m_Columns.Draw(0, Styles.firstFrame);
                            if (leftMarker != null)
                                DrawFrameIndexButton(leftMarker.firstFrameIndex, m_ProfileLeftView);
                            else
                                m_Columns.Draw(1, Styles.emptyString);
                            if (rightMarker != null)
                                DrawFrameIndexButton(rightMarker.firstFrameIndex, m_ProfileRightView);
                            else
                                m_Columns.Draw(2, Styles.emptyString);
                            EditorGUILayout.EndHorizontal();

                            DrawTopComparison(leftMarker, rightMarker);

                            GUILayout.Space(style.lineHeight);

                            EditorGUILayout.BeginHorizontal();

                            int leftBucketCount = leftMarker != null ? leftMarker.buckets.Length : 0;
                            int rightBucketCount = rightMarker != null ? rightMarker.buckets.Length : 0;

                            float leftMin = MarkerData.GetMsMin(leftMarker);
                            float rightMin = MarkerData.GetMsMin(rightMarker);

                            float leftMax = MarkerData.GetMsMax(leftMarker);
                            float rightMax = MarkerData.GetMsMax(rightMarker);

                            int[] leftBuckets = leftMarker != null ? leftMarker.buckets : new int[0];
                            int[] rightBuckets = rightMarker != null ? rightMarker.buckets : new int[0];

                            Units units = m_DisplayUnits.Units;
                            string unitName = "marker time";
                            if (DisplayCount())
                            {
                                units = Units.Count;
                                unitName = "count";

                                leftBucketCount = leftMarker != null ? leftMarker.countBuckets.Length : 0;
                                rightBucketCount = rightMarker != null ? rightMarker.countBuckets.Length : 0;

                                leftMin = MarkerData.GetCountMin(leftMarker);
                                rightMin = MarkerData.GetCountMin(rightMarker);

                                leftMax = MarkerData.GetCountMax(leftMarker);
                                rightMax = MarkerData.GetCountMax(rightMarker);

                                leftBuckets = leftMarker != null ? leftMarker.countBuckets : new int[0];
                                rightBuckets = rightMarker != null ? rightMarker.countBuckets : new int[0];
                            }

                            DisplayUnits displayUnits = new DisplayUnits(units);

                            float minValue;
                            float maxValue;
                            if (leftMarker != null && rightMarker != null)
                            {
                                minValue = Math.Min(leftMin, rightMin);
                                maxValue = Math.Max(leftMax, rightMax);
                            }
                            else if (leftMarker != null)
                            {
                                minValue = leftMin;
                                maxValue = leftMax;
                            }
                            else // Either valid or 0
                            {
                                minValue = rightMin;
                                maxValue = rightMax;
                            }

                            if (leftBucketCount > 0 && rightBucketCount > 0 && leftBucketCount != rightBucketCount)
                            {
                                Debug.Log("Error - number of buckets doesn't match in the left and right marker analysis");
                            }
                            else
                            {
                                int bucketCount = Math.Max(leftBucketCount, rightBucketCount);
                                int leftFrameCount = MarkerData.GetPresentOnFrameCount(leftMarker);
                                int rightFrameCount = MarkerData.GetPresentOnFrameCount(rightMarker);

                                DrawComparisonHistogram(100, minValue, maxValue, bucketCount, leftBuckets, rightBuckets, leftFrameCount, rightFrameCount, leftMarker != null, rightMarker != null, displayUnits);
                            }

                            float plotWidth = 40 + GUI.skin.box.padding.horizontal;
                            float plotHeight = 100;
                            plotWidth /= 2.0f;
                            BoxAndWhiskerPlot boxAndWhiskerPlot = new BoxAndWhiskerPlot(m_2D, units);
                            DrawBoxAndWhiskerPlotForMarker(boxAndWhiskerPlot, plotWidth, plotHeight, m_ProfileLeftView.analysis, leftMarker, minValue, maxValue,
                                UIColor.boxAndWhiskerLineColorLeft, UIColor.boxAndWhiskerBoxColorLeft);
                            DrawBoxAndWhiskerPlotForMarker(boxAndWhiskerPlot, plotWidth, plotHeight, m_ProfileRightView.analysis, rightMarker, minValue, maxValue,
                                UIColor.boxAndWhiskerLineColorRight, UIColor.boxAndWhiskerBoxColorRight);

                            boxAndWhiskerPlot.DrawText(m_Columns.GetColumnWidth(3), plotHeight, minValue, maxValue,
                                string.Format("Min {0} for selected frames in the 2 data sets", unitName),
                                string.Format("Max {0} for selected frames in the 2 data sets", unitName));

                            EditorGUILayout.EndHorizontal();

                            GUILayout.Space(style.lineHeight);

                            DrawComparisonSelectedStats(leftMarker, rightMarker);

                            EditorGUILayout.EndVertical();

                            m_2D.ClearClipRect();
                            GUILayout.EndScrollView();
                        }
                        else
                        {
                            EditorGUILayout.LabelField("Marker not in selection");
                        }
                    }
                }
                else
                {
                    EditorGUILayout.LabelField("No marker data selected");
                }

                EditorGUILayout.EndVertical();
            }

            if (m_ShowMarkerSummary != lastMarkerSummary)
            {
                ProfileAnalyzerAnalytics.SendUIVisibilityEvent(ProfileAnalyzerAnalytics.UIVisibility.Markers, analytic.GetDurationInSeconds(), m_ShowMarkerSummary);
            }

            EditorGUILayout.EndVertical();
        }

        void SelectTab(ActiveTab newTab)
        {
            m_NextActiveTab = newTab;
        }

        static readonly ProfilerMarkerAbstracted m_DrawToolbarProfilerMarker = new ProfilerMarkerAbstracted("ProfileAnalyzer.DrawToolbar");

        void DrawToolbar()
        {
            using (m_DrawToolbarProfilerMarker.Auto())
            {
                EditorGUILayout.BeginHorizontal(EditorStyles.toolbar);

                EditorGUILayout.LabelField("Mode:", GUILayout.Width(40));
                ActiveTab newTab = (ActiveTab)GUILayout.Toolbar((int)m_ActiveTab, new string[] {"Single", "Compare"},
                    EditorStyles.toolbarButton, GUILayout.ExpandWidth(false));
                if (newTab != m_ActiveTab)
                {
                    SelectTab(newTab);
                }

                //GUILayout.FlexibleSpace();
                EditorGUILayout.Separator();
                bool lastEnabled = GUI.enabled;
                bool enabled = GUI.enabled;
                if (m_ProfileSingleView.IsDataValid() ||
                    (m_ProfileLeftView.IsDataValid() && m_ProfileRightView.IsDataValid()))
                    GUI.enabled = true;
                else
                    GUI.enabled = false;
                if (GUILayout.Button(Styles.export, EditorStyles.toolbarButton, GUILayout.Width(50)))
                {
                    Vector2 windowPosition = new Vector2(Event.current.mousePosition.x,
                        Event.current.mousePosition.y + GUI.skin.label.lineHeight);
                    Vector2 screenPosition = GUIUtility.GUIToScreenPoint(windowPosition);

                    ProfileAnalyzerExportWindow.Open(screenPosition.x, screenPosition.y, m_ProfileSingleView,
                        m_ProfileLeftView, m_ProfileRightView, this);

                    EditorGUIUtility.ExitGUI();
                }

                GUI.enabled = lastEnabled;

                bool profilerOpen = IsProfilerWindowOpen();
                if (!profilerOpen)
                {
                    if (GUILayout.Toggle(profilerOpen, "Open Profiler Window", EditorStyles.toolbarButton,
                        GUILayout.ExpandWidth(false)) == true)
                    {
                        var analytic = ProfileAnalyzerAnalytics.BeginAnalytic();
                        m_ProfilerWindowInterface.OpenProfilerOrUseExisting();
                        ProfileAnalyzerAnalytics.SendUIButtonEvent(ProfileAnalyzerAnalytics.UIButton.OpenProfiler,
                            analytic);
                        EditorGUIUtility.ExitGUI();
                    }
                }
                else
                {
                    if (GUILayout.Toggle(profilerOpen, "Close Profiler Window", EditorStyles.toolbarButton,
                        GUILayout.ExpandWidth(false)) == false)
                    {
                        var analytic = ProfileAnalyzerAnalytics.BeginAnalytic();
                        m_ProfilerWindowInterface.CloseProfiler();
                        ProfileAnalyzerAnalytics.SendUIButtonEvent(ProfileAnalyzerAnalytics.UIButton.CloseProfiler,
                            analytic);
                        EditorGUIUtility.ExitGUI();
                    }
                }

                EditorGUILayout.Separator();

                GUILayout.FlexibleSpace();

                EditorGUILayout.EndHorizontal();
            }
        }

        void SetupStyles()
        {
            if (!m_StylesSetup)
            {
                m_StyleMiddleRight = new GUIStyle(GUI.skin.label);
                m_StyleMiddleRight.alignment = TextAnchor.MiddleRight;

                m_StyleUpperLeft = new GUIStyle(GUI.skin.label);
                m_StyleUpperLeft.alignment = TextAnchor.UpperLeft;

                m_StylesSetup = true;
            }
        }

        static readonly ProfilerMarkerAbstracted m_DrawProfilerMarker = new ProfilerMarkerAbstracted("ProfileAnalyzer.Draw");

        void Draw()
        {
            // Make sure we start enabled (in case something overrode it last frame)
            GUI.enabled = true;

            using (m_DrawProfilerMarker.Auto())
            {
                SetupStyles();

                EditorGUILayout.BeginVertical();

                DrawToolbar();

                switch (m_ActiveTab)
                {
                    case ActiveTab.Summary:
                        DrawAnalysis();
                        break;
                    case ActiveTab.Compare:
                        DrawComparison();
                        break;
                }

                EditorGUILayout.EndVertical();
            }
        }

        int FindSelectionByName(List<MarkerData> markers, string name)
        {
            int index = 0;
            foreach (var marker in markers)
            {
                if (marker.name == name)
                    return index;
                index++;
            }
            return -1; // not found
        }

        /// <summary>
        /// Select marker to focus on
        /// </summary>
        /// <param name="name">Name of the marker</param>
        // Version 1.0 of the package exposed this API so we can't remove it until we increment the major package version.
        public void SelectMarker(string name)
        {
            SelectMarker(name, null, null);
        }

        void SelectMarker(string name, string threadGroupName = null, string threadName = null)
        {
            switch (m_ActiveTab)
            {
                case ActiveTab.Summary:
                    SelectMarkerByName(name, threadGroupName, threadName);
                    break;
                case ActiveTab.Compare:
                    SelectPairingByName(name, threadGroupName, threadName);
                    break;
            }
        }

        void UpdateSelectedMarkerName(string markerName)
        {
            m_SelectedMarker.name = markerName;

            // only update the Profiler Window if it wasn't updated successfully with this marker yet.
            if (m_LastMarkerSuccesfullySyncedWithProfilerWindow == markerName)
                return;
            var updatedSelectedSampleSuccesfully = false;
            if (m_ProfilerWindowInterface.IsReady() && !m_SelectionEventFromProfilerWindowInProgress && m_ThreadSelection.selection != null && m_ThreadSelection.selection.Count > 0)
            {
                updatedSelectedSampleSuccesfully = m_ProfilerWindowInterface.SetProfilerWindowMarkerName(markerName, m_ThreadSelection.selection);
            }
            if (updatedSelectedSampleSuccesfully)
                m_LastMarkerSuccesfullySyncedWithProfilerWindow = markerName;
        }

        internal void SelectMarkerByIndex(int index, string markerNameFallback = null, string threadGroupName = null, string threadName = null)
        {
            if (m_ProfileSingleView == null || m_ProfileSingleView.analysis == null)
                return;


            // Check if this marker is in the 'filtered' list
            var markers = m_ProfileSingleView.analysis.GetMarkers();
            if (markers.Count <= 0)
                return;

            bool valid = true;
            if (index >= 0 && index < markers.Count)
            {
                var marker = markers[index];
                valid = DoesMarkerPassFilter(marker.name);
            }

            m_SelectedMarker.id = index;

            if (m_ProfileTable != null)
            {
                List<int> selection = new List<int>();
                if (index >= 0 && valid)
                    selection.Add(index);
                m_ProfileTable.SetSelection(selection, TreeViewSelectionOptions.RevealAndFrame);
            }

            var markerName = GetMarkerName(index);

            if (index == -1 && !string.IsNullOrEmpty(markerNameFallback))
            {
                markerName = markerNameFallback;
                if (!string.IsNullOrEmpty(threadName))
                {
                    m_SelectedMarker.threadGroupName = threadGroupName;
                    m_SelectedMarker.threadName = threadName;
                }
                else
                {
                    m_SelectedMarker.threadGroupName = null;
                    m_SelectedMarker.threadName = null;
                }
            }
            else
            {
                m_SelectedMarker.threadGroupName = null;
                m_SelectedMarker.threadName = null;
            }

            if (markerName != null)
                UpdateSelectedMarkerName(markerName);
        }

        /// <summary>
        /// Get currently selected marker
        /// </summary>
        /// <returns>Name of currently selected marker, or null if none selected</returns>
        public string GetSelectedMarkerName()
        {
            switch (m_ActiveTab)
            {
                case ActiveTab.Summary:
                    return GetMarkerName(m_SelectedMarker.id);
                case ActiveTab.Compare:
                    return GetPairingName(m_SelectedPairing);
            }

            return null;
        }

        string GetMarkerName(int index)
        {
            if (m_ProfileSingleView.analysis == null)
                return null;

            var marker = m_ProfileSingleView.analysis.GetMarker(index);
            if (marker == null)
                return null;

            return marker.name;
        }

        void SelectMarkerByName(string markerName, string threadGroupName = null, string threadName = null)
        {
            int index = (m_ProfileSingleView.analysis != null) ? m_ProfileSingleView.analysis.GetMarkerIndexByName(markerName) : -1;

            SelectMarkerByIndex(index, markerName, threadGroupName, threadName);
        }

        internal void SelectPairing(int index, string threadGroupName = null, string threadName = null)
        {
            if (m_Pairings == null || m_Pairings.Count == 0)
                return;

            // Check if this marker is in the 'filtered' list
            bool valid = true;
            if (index >= 0 && index < m_Pairings.Count)
            {
                var pairing = m_Pairings[index];
                valid = DoesMarkerPassFilter(pairing.name);
            }

            m_SelectedPairing = index;

            if (m_ComparisonTable != null)
            {
                List<int> selection = new List<int>();
                if (index >= 0 && valid)
                    selection.Add(index);
                m_ComparisonTable.SetSelection(selection, TreeViewSelectionOptions.RevealAndFrame);
            }

            var markerName = GetPairingName(index);
            if (markerName != null)
                UpdateSelectedMarkerName(markerName);
        }

        string GetPairingName(int index)
        {
            if (m_Pairings == null)
                return null;

            if (index < 0 || index >= m_Pairings.Count)
                return null;

            return m_Pairings[index].name;
        }

        void SelectPairingByName(string pairingName, string threadGroupName = null, string threadName = null)
        {
            if (m_Pairings != null && pairingName != null)
            {
                for (int index = 0; index < m_Pairings.Count; index++)
                {
                    var pairing = m_Pairings[index];
                    if (pairing.name == pairingName)
                    {
                        SelectPairing(index, threadGroupName, threadName);
                        return;
                    }
                }
            }

            SelectPairing(-1, threadGroupName, threadName);
        }

        GUIContent GetFrameCountText(ProfileDataView context)
        {
            var frameSummary = context.analysis.GetFrameSummary();

            string text;
            string tooltip;
            if (frameSummary.first == frameSummary.last)
            {
                text = string.Format("{0}", frameSummary.count);
                tooltip = "";
            }
            else
            {
                int rangeSize = (1 + (frameSummary.last - frameSummary.first));
                if (frameSummary.count == rangeSize)
                {
                    text = string.Format("{0}", frameSummary.count);
                    tooltip = string.Format("{0} frames selected\n\n{1} first selected frame\n{2} last selected frame\n\nConsecutive Sequence"
                        , frameSummary.count
                        , GetRemappedUIFrameIndex(frameSummary.first, context)
                        , GetRemappedUIFrameIndex(frameSummary.last, context));
                }
                else
                {
                    text = string.Format("{0}*", frameSummary.count);
                    var ranges = RangesText(context);
                    tooltip = string.Format("{0} frames selected\n\nframe ranges: {1} \n\nNot a consecutive sequence", frameSummary.count, ranges);
                }
            }

            return new GUIContent(text, tooltip);
        }

        string RangesText(ProfileDataView context)
        {
            var sortedFrames = context.analysis.GetFrameSummary().frames.OrderBy(x => x.frameIndex).ToArray();

            var ranges = "";
            int lastAdded = GetRemappedUIFrameIndex(sortedFrames[0].frameIndex, context);
            ranges += lastAdded;

            for (int n = 1; n < sortedFrames.Length; ++n)
            {
                if (sortedFrames[n].frameIndex == (sortedFrames[n - 1].frameIndex + 1)) continue;

                int nIdx = GetRemappedUIFrameIndex(sortedFrames[n].frameIndex, context);
                int pNIdx = GetRemappedUIFrameIndex(sortedFrames[n - 1].frameIndex, context);

                if (lastAdded == pNIdx)
                {
                    ranges += ", " + nIdx;
                }
                else
                {
                    ranges += "-" + pNIdx + ", " + nIdx;
                }

                lastAdded = nIdx;
            }

            int remappedLastFrame = GetRemappedUIFrameIndex(sortedFrames.Last().frameIndex, context);
            if (lastAdded == remappedLastFrame)
                return ranges;

            ranges += "-" + remappedLastFrame;
            return ranges;
        }

        GUIContent GetFirstFrameText(ProfileDataView context)
        {
            var frameSummary = context.analysis.GetFrameSummary();

            string text;
            string tooltip;
            if (frameSummary.count == 0)
            {
                text = "";
                tooltip = "";
            }
            else if (frameSummary.first == frameSummary.last)
            {
                int remappedFrame = GetRemappedUIFrameIndex(frameSummary.first, context);
                text = string.Format("{0}", remappedFrame);
                tooltip = string.Format("Frame {0} selected", remappedFrame);
            }
            else
            {
                int rangeSize = (1 + (frameSummary.last - frameSummary.first));
                if (frameSummary.count == rangeSize)
                {
                    int remappedFirstFrame = GetRemappedUIFrameIndex(frameSummary.first, context);
                    text = string.Format("{0}", remappedFirstFrame);
                    tooltip = string.Format("{0} frames selected\n\n{1} first selected frame\n{2} last selected frame\n\nConsecutive Sequence"
                        , frameSummary.count
                        , remappedFirstFrame
                        , GetRemappedUIFrameIndex(frameSummary.last, context));
                }
                else
                {
                    text = string.Format("{0}*", GetRemappedUIFrameIndex(frameSummary.first, context));
                    var ranges = RangesText(context);
                    tooltip = string.Format("{0} frames selected\n\nframe ranges: {1} \n\nNot a consecutive sequence", frameSummary.count, ranges);
                }
            }

            return new GUIContent(text, tooltip);
        }

        GUIContent GetLastFrameText(ProfileDataView context)
        {
            var frameSummary =  context.analysis.GetFrameSummary();

            string text;
            string tooltip;
            if (frameSummary.count == 0)
            {
                text = "";
                tooltip = "";
            }
            else if (frameSummary.first == frameSummary.last)
            {
                text = "";
                tooltip = string.Format("Frame {0} selected", GetRemappedUIFrameIndex(frameSummary.first, context), context);
            }
            else
            {
                int rangeSize = (1 + (frameSummary.last - frameSummary.first));
                if (frameSummary.count == rangeSize)
                {
                    int remappedLastFrame = GetRemappedUIFrameIndex(frameSummary.last, context);
                    text = string.Format("{0}", remappedLastFrame);
                    tooltip = string.Format("{0} frames selected\n\n{1} first selected frame\n{2} last selected frame\n\nConsecutive Sequence",
                        frameSummary.count,
                        GetRemappedUIFrameIndex(frameSummary.first, context),
                        remappedLastFrame);
                }
                else
                {
                    text = string.Format("{0}*", GetRemappedUIFrameIndex(frameSummary.last, context));
                    var ranges = RangesText(context);
                    tooltip = string.Format("{0} frames selected\n\nframe ranges: {1} \n\nNot a consecutive sequence", frameSummary.count, ranges);;
                }
            }

            return new GUIContent(text, tooltip);
        }

        void DrawFrameSummary()
        {
            EditorGUILayout.BeginVertical(GUI.skin.box, GUILayout.Width(LayoutSize.WidthRHS));

            bool lastShowFrameSummary = m_ShowFrameSummary;
            m_ShowFrameSummary = BoldFoldout(m_ShowFrameSummary, Styles.frameSummary);
            var analytic = ProfileAnalyzerAnalytics.BeginAnalytic();
            if (m_ShowFrameSummary)
            {
                EditorGUILayout.BeginVertical(); // To match indenting on the marker summary where a scroll area is present

                if (IsAnalysisValid())
                {
                    var frameSummary = m_ProfileSingleView.analysis.GetFrameSummary();

                    m_Columns.SetColumnSizes(LayoutSize.WidthColumn0, LayoutSize.WidthColumn1, LayoutSize.WidthColumn2, LayoutSize.WidthColumn3);
                    m_Columns.Draw(0, "");

                    m_Columns.Draw2(Styles.frameCount, GetFrameCountText(m_ProfileSingleView));

                    EditorGUILayout.BeginHorizontal();
                    m_Columns.Draw(0, Styles.frameStart);
                    GUIContent firstFrameTextContent = GetFirstFrameText(m_ProfileSingleView);
                    m_Columns.Draw(1, firstFrameTextContent);
                    if (firstFrameTextContent.text != "")
                        DrawFrameIndexButton(frameSummary.first, m_ProfileSingleView);
                    EditorGUILayout.EndHorizontal();

                    EditorGUILayout.BeginHorizontal();
                    m_Columns.Draw(0, Styles.frameEnd);
                    GUIContent lastFrameTextContent = GetLastFrameText(m_ProfileSingleView);
                    m_Columns.Draw(1, lastFrameTextContent);
                    if (lastFrameTextContent.text != "")
                        DrawFrameIndexButton(frameSummary.last, m_ProfileSingleView);
                    EditorGUILayout.EndHorizontal();

                    m_Columns.SetColumnSizes(LayoutSize.WidthColumn0, LayoutSize.WidthColumn1, LayoutSize.WidthColumn2, LayoutSize.WidthColumn3);
                    m_Columns.Draw(0, "");
                    m_Columns.Draw3("", GetDisplayUnits(), "Frame");

                    Draw3LabelMsFrame(Styles.max, frameSummary.msMax, frameSummary.maxFrameIndex, m_ProfileSingleView);
                    Draw2LabelMs(Styles.upperQuartile, frameSummary.msUpperQuartile);
                    Draw3LabelMsFrame(Styles.median, frameSummary.msMedian, frameSummary.medianFrameIndex, m_ProfileSingleView);
                    Draw2LabelMs(Styles.mean, frameSummary.msMean);
                    Draw2LabelMs(Styles.lowerQuartile, frameSummary.msLowerQuartile);
                    Draw3LabelMsFrame(Styles.min, frameSummary.msMin, frameSummary.minFrameIndex, m_ProfileSingleView);

                    GUIStyle style = GUI.skin.label;
                    GUILayout.Space(style.lineHeight);

                    EditorGUILayout.BeginHorizontal();
                    Histogram histogram = new Histogram(m_2D, m_DisplayUnits.Units);
                    histogram.Draw(LayoutSize.HistogramWidth, 40, frameSummary.buckets, frameSummary.count, 0, frameSummary.msMax, UIColor.bar);

                    BoxAndWhiskerPlot boxAndWhiskerPlot = new BoxAndWhiskerPlot(m_2D, m_DisplayUnits.Units);

                    float plotWidth = 40 + GUI.skin.box.padding.horizontal;
                    float plotHeight = 40;
                    boxAndWhiskerPlot.Draw(plotWidth, plotHeight, frameSummary.msMin, frameSummary.msLowerQuartile, frameSummary.msMedian, frameSummary.msUpperQuartile, frameSummary.msMax, 0, frameSummary.msMax, UIColor.standardLine, UIColor.standardLine);

                    boxAndWhiskerPlot.DrawText(m_Columns.GetColumnWidth(3), plotHeight, frameSummary.msMin, frameSummary.msMax,
                        "Min frame time for selected frames",
                        "Max frame time for selected frames");

                    EditorGUILayout.EndHorizontal();
                }
                else
                {
                    EditorGUILayout.LabelField("No analysis data selected");
                }

                EditorGUILayout.EndVertical();
            }

            if (m_ShowFrameSummary != lastShowFrameSummary)
            {
                ProfileAnalyzerAnalytics.SendUIVisibilityEvent(ProfileAnalyzerAnalytics.UIVisibility.Frames, analytic.GetDurationInSeconds(), m_ShowFrameSummary);
            }

            EditorGUILayout.EndVertical();
        }

        GUIContent GetThreadNameWithGroupTooltip(string threadNameWithIndex, bool singleThread)
        {
            string friendlyThreadName = GetFriendlyThreadName(threadNameWithIndex, singleThread);
            string groupName;
            friendlyThreadName = ProfileData.GetThreadNameWithoutGroup(friendlyThreadName, out groupName);

            if (groupName == "")
                return new GUIContent(friendlyThreadName, string.Format("{0}", friendlyThreadName));
            else
                return new GUIContent(friendlyThreadName, string.Format("{0}\n{1}", friendlyThreadName, groupName));
        }

        void DrawThreadSummary()
        {
            EditorGUILayout.BeginVertical(GUI.skin.box, GUILayout.Width(LayoutSize.WidthRHS));

            bool lastShowThreadSummary = m_ShowThreadSummary;
            m_ShowThreadSummary = BoldFoldout(m_ShowThreadSummary, Styles.threadSummary);
            var analytic = ProfileAnalyzerAnalytics.BeginAnalytic();
            if (m_ShowThreadSummary)
            {
                EditorGUILayout.BeginVertical(); // To match indenting on the marker summary where a scroll area is present
                if (IsAnalysisValid())
                {
                    float xAxisMin = 0.0f;
                    float xAxisMax = GetThreadTimeRange(m_ProfileSingleView.analysis);

                    m_Columns.SetColumnSizes(LayoutSize.WidthColumn0, LayoutSize.WidthColumn1, LayoutSize.WidthColumn2, LayoutSize.WidthColumn3);
                    EditorGUILayout.BeginHorizontal();
                    m_Columns.Draw4("", "", "", "");
                    EditorGUILayout.EndHorizontal();

                    EditorGUILayout.BeginHorizontal();
                    m_Columns.Draw(0, "Total Count : ");
                    m_Columns.Draw(1, m_ProfileSingleView.data.GetThreadCount().ToString());
                    EditorGUILayout.EndHorizontal();

                    List<string> threadSelection = GetLimitedThreadSelection(m_ThreadNames, m_ThreadSelection);
                    int selectedThreads = threadSelection.Count;

                    EditorGUILayout.BeginHorizontal();
                    m_Columns.Draw(0, "Selected : ");
                    m_Columns.Draw(1, selectedThreads.ToString());
                    EditorGUILayout.EndHorizontal();
                    ShowThreadRange();

                    m_Columns.SetColumnSizes(LayoutSize.WidthColumn0, LayoutSize.WidthColumn1, LayoutSize.WidthColumn2 + LayoutSize.WidthColumn3, 0);
                    m_Columns.Draw3("", "Median", "Thread");

                    m_ThreadScroll = EditorGUILayout.BeginScrollView(m_ThreadScroll, GUIStyle.none, GUI.skin.verticalScrollbar);
                    Rect clipRect = new Rect(m_ThreadScroll.x, m_ThreadScroll.y, m_ThreadsAreaRect.width, m_ThreadsAreaRect.height);
                    m_2D.SetClipRect(clipRect);
                    for (int i = 0; i < m_ThreadUINames.Count; i++)
                    {
                        string threadNameWithIndex = m_ThreadNames[i];
                        if (!threadNameWithIndex.Contains(":"))
                            continue;    // Ignore 'All'

                        bool include = ProfileAnalyzer.MatchThreadFilter(threadNameWithIndex, m_ThreadSelection.selection);
                        if (!include)
                            continue;

                        ThreadData thread = m_ProfileSingleView.analysis.GetThreadByName(threadNameWithIndex);
                        if (thread == null)    // May be the 'all' field
                            continue;

                        bool singleThread = thread.threadsInGroup > 1 ? false : true;

                        BoxAndWhiskerPlot boxAndWhiskerPlot = new BoxAndWhiskerPlot(m_2D, m_DisplayUnits.Units);
                        EditorGUILayout.BeginHorizontal();
                        boxAndWhiskerPlot.DrawHorizontal(100, GUI.skin.label.lineHeight, thread.msMin, thread.msLowerQuartile, thread.msMedian, thread.msUpperQuartile, thread.msMax, xAxisMin, xAxisMax, UIColor.bar, UIColor.barBackground, GUI.skin.label);

                        m_Columns.Draw(1, ToDisplayUnitsWithTooltips(thread.msMedian));
                        m_Columns.Draw(2, GetThreadNameWithGroupTooltip(thread.threadNameWithIndex, singleThread));
                        EditorGUILayout.EndHorizontal();
                    }
                    m_2D.ClearClipRect();
                    EditorGUILayout.EndScrollView();

                    if (Event.current.type == EventType.Repaint)
                    {
                        // This value is not valid at layout phase
                        m_ThreadsAreaRect = GUILayoutUtility.GetLastRect();
                    }
                }
                else
                {
                    EditorGUILayout.LabelField("No analysis data selected");
                }
                EditorGUILayout.EndVertical();
            }

            if (m_ShowThreadSummary != lastShowThreadSummary)
            {
                ProfileAnalyzerAnalytics.SendUIVisibilityEvent(ProfileAnalyzerAnalytics.UIVisibility.Threads, analytic.GetDurationInSeconds(), m_ShowThreadSummary);
            }

            EditorGUILayout.EndVertical();
        }

        void DrawHistogramForMarker(Histogram histogram, MarkerData marker)
        {
            if (DisplayCount())
                histogram.Draw(LayoutSize.HistogramWidth, 100, marker.countBuckets, marker.presentOnFrameCount, marker.countMin, marker.countMax, UIColor.bar);
            else
                histogram.Draw(LayoutSize.HistogramWidth, 100, marker.buckets, marker.presentOnFrameCount, marker.msMin, marker.msMax, UIColor.bar);
        }

        internal bool IsProfilerWindowOpen()
        {
            return m_ProfilerWindowInterface.IsReady();
        }

        /// <summary>
        /// Used to remap frame indices when the loaded range in the profiler does not match the range present in the Profile Analyzer capture.
        /// This happens when we reload data into the Profiler Window as the index range becomes 1 -> n+1
        /// </summary>
        /// <param name="frameIndex">target frame index</param>
        /// <param name="frameIndexOffset">capture frameIndex offset</param>
        /// <returns></returns>
        internal int RemapFrameIndex(int frameIndex, int frameIndexOffset)
        {
            if (m_ProfilerFirstFrameIndex == 1 && frameIndex > frameIndexOffset)
                return frameIndex - frameIndexOffset;
            else
                return frameIndex;
        }

        internal void JumpToFrame(int frameIndex, ProfileData frameContext, bool reportErrors = true)
        {
            if (!m_ProfilerWindowInterface.IsReady())
                return;

            var analytic = ProfileAnalyzerAnalytics.BeginAnalytic();
            m_ProfilerWindowInterface.JumpToFrame(RemapFrameIndex(frameIndex, frameContext.FrameIndexOffset));
            if (IsSelectedMarkerNameValid())
                m_ProfilerWindowInterface.SetProfilerWindowMarkerName(m_SelectedMarker.name, m_ThreadSelection.selection);
            ProfileAnalyzerAnalytics.SendUIButtonEvent(ProfileAnalyzerAnalytics.UIButton.JumpToFrame, analytic);
        }

        internal float DrawFrameIndexButton(int frameIndex, ProfileDataView frameContext)
        {
            float defaultWidth = 50f;
            if (frameIndex < 0)
                return defaultWidth;

            bool enabled = GUI.enabled;
            if (!IsProfilerWindowOpen() || !frameContext.inSyncWithProfilerData)
                GUI.enabled = false;

            var remappedIndex = GetRemappedUIFrameIndex(frameIndex, frameContext);
            var content = new GUIContent(string.Format("{0}", remappedIndex), string.Format("Jump to frame {0} in the Unity Profiler", remappedIndex));
            Vector2 size = GUI.skin.button.CalcSize(content);
            //float height = size.y;
            float maxWidth = Math.Max(defaultWidth, size.x);
            if (GUILayout.Button(content, GUILayout.MinWidth(defaultWidth), GUILayout.MaxWidth(maxWidth)))
            {
                JumpToFrame(frameIndex, frameContext.data);
            }

            GUI.enabled = enabled;

            return maxWidth;
        }

        internal void DrawFrameIndexButton(Rect rect, int frameIndex, ProfileDataView frameContext)
        {
            if (frameIndex < 0)
                return;

            bool enabled = GUI.enabled;
            if (!IsProfilerWindowOpen() || !frameContext.inSyncWithProfilerData)
                GUI.enabled = false;

            // Clamp to max height to match other buttons
            // And centre vertically if needed
            var remappedIndex = GetRemappedUIFrameIndex(frameIndex, frameContext);
            var content = new GUIContent(string.Format("{0}", remappedIndex), string.Format("Jump to frame {0} in the Unity Profiler", remappedIndex));
            Vector2 size = GUI.skin.button.CalcSize(content);
            float height = size.y; // was 14
            rect.y += (rect.height - height) / 2;
            rect.height = Math.Min(rect.height, height);

            if (GUI.Button(rect, content))
            {
                JumpToFrame(frameIndex, frameContext.data);
            }

            GUI.enabled = enabled;
        }

        void Draw3LabelMsFrame(GUIContent col1, float ms, int frameIndex, ProfileDataView frameContext)
        {
            EditorGUILayout.BeginHorizontal();
            m_Columns.Draw(0, col1);
            m_Columns.Draw(1, ToDisplayUnitsWithTooltips(ms));
            DrawFrameIndexButton(frameIndex, frameContext);
            EditorGUILayout.EndHorizontal();
        }

        void Draw2LabelMs(GUIContent col1, float ms)
        {
            EditorGUILayout.BeginHorizontal();
            m_Columns.Draw(0, col1);
            m_Columns.Draw(1, ToDisplayUnitsWithTooltips(ms));
            EditorGUILayout.EndHorizontal();
        }

        void Draw4DiffMs(GUIContent col1, float msLeft, float msRight)
        {
            EditorGUILayout.BeginHorizontal();
            m_Columns.Draw(0, col1);
            m_Columns.Draw(1, ToDisplayUnitsWithTooltips(msLeft));
            m_Columns.Draw(2, ToDisplayUnitsWithTooltips(msRight));
            m_Columns.Draw(3, ToDisplayUnitsWithTooltips(msRight - msLeft));
            EditorGUILayout.EndHorizontal();
        }

        void Draw4DiffMs(GUIContent col1, float msLeft, int frameIndexLeft, float msRight, int frameIndexRight)
        {
            EditorGUILayout.BeginHorizontal();
            m_Columns.Draw(0, col1);
            m_Columns.Draw(1, ToDisplayUnitsWithTooltips(msLeft, false, frameIndexLeft));
            m_Columns.Draw(2, ToDisplayUnitsWithTooltips(msRight, false, frameIndexRight));
            m_Columns.Draw(3, ToDisplayUnitsWithTooltips(msRight - msLeft));
            EditorGUILayout.EndHorizontal();
        }

        void Draw4Ms(GUIContent col1, float value2, float value3, float value4)
        {
            EditorGUILayout.BeginHorizontal();
            m_Columns.Draw(0, col1);
            m_Columns.Draw(1, ToDisplayUnitsWithTooltips(value2));
            m_Columns.Draw(2, ToDisplayUnitsWithTooltips(value3));
            m_Columns.Draw(3, ToDisplayUnitsWithTooltips(value4));
            EditorGUILayout.EndHorizontal();
        }

        void DrawBoxAndWhiskerPlotForMarker(BoxAndWhiskerPlot boxAndWhiskerPlot, float width, float height, ProfileAnalysis analysis, MarkerData marker, float yAxisStart, float yAxisEnd, Color color, Color colorBackground)
        {
            if (marker == null)
            {
                boxAndWhiskerPlot.Draw(width, height, 0, 0, 0, 0, 0, yAxisStart, yAxisEnd, color, colorBackground);
                return;
            }

            if (DisplayCount())
                boxAndWhiskerPlot.Draw(width, height, marker.countMin, marker.countLowerQuartile, marker.countMedian, marker.countUpperQuartile, marker.countMax, yAxisStart, yAxisEnd, color, colorBackground);
            else
                boxAndWhiskerPlot.Draw(width, height, marker.msMin, marker.msLowerQuartile, marker.msMedian, marker.msUpperQuartile, marker.msMax, yAxisStart, yAxisEnd, color, colorBackground);
        }

        void DrawBoxAndWhiskerPlotHorizontalForMarker(BoxAndWhiskerPlot boxAndWhiskerPlot, float width, float height, ProfileAnalysis analysis, MarkerData marker, float yAxisStart, float yAxisEnd, Color color, Color colorBackground)
        {
            boxAndWhiskerPlot.DrawHorizontal(width, height, marker.msMin, marker.msLowerQuartile, marker.msMedian, marker.msUpperQuartile, marker.msMax, yAxisStart, yAxisEnd, color, colorBackground);
        }

        void DrawFrameRatio(MarkerData marker)
        {
            var frameSummary = m_ProfileSingleView.analysis.GetFrameSummary();

            GUIStyle style = GUI.skin.label;
            float w = LayoutSize.WidthColumn0;
            float h = style.lineHeight;
            float ySpacing = 2;
            float barHeight = h - ySpacing;

            EditorGUILayout.BeginVertical(GUILayout.Width(w + LayoutSize.WidthColumn1 + LayoutSize.WidthColumn2));

            float barMax = frameSummary.msMean;
            float barValue = marker.msMean;
            string text = "Mean frame contribution";
            Units units = m_DisplayUnits.Units;
            bool removed = marker.timeIgnored > 0;
            if (DisplayCount())
            {
                units = Units.Count;
                barMax = frameSummary.markerCountMaxMean;
                barValue = marker.countMean;
                text = "Mean count";
            }
            DisplayUnits displayUnits = new DisplayUnits(units);
            float barLength = Math.Min((w * barValue) / barMax, w);

            EditorGUILayout.LabelField(text);
            m_Columns.SetColumnSizes(LayoutSize.WidthColumn0, LayoutSize.WidthColumn1, LayoutSize.WidthColumn2, LayoutSize.WidthColumn3);

            m_Columns.Draw2("", "");
         
            EditorGUILayout.BeginHorizontal();

            if (removed)
            {
                EditorGUILayout.LabelField("(Marker Removed from analysis)");
            }
            else
            {
                // NOTE: This can effect the whole width of the region its inside
                // Not clear why
                if (m_2D.DrawStart(w, h, Draw2D.Origin.TopLeft, style))
                {
                    m_2D.DrawFilledBox(0, ySpacing, barLength, barHeight, UIColor.bar);
                    m_2D.DrawFilledBox(barLength, ySpacing, w - barLength, barHeight, UIColor.barBackground);
                    m_2D.DrawEnd();

                    Rect rect = GUILayoutUtility.GetLastRect();
                    string tooltip = string.Format("{0}", displayUnits.ToString(barValue, true, 5));
                    GUI.Label(rect, new GUIContent("", tooltip));
                }

                EditorGUILayout.LabelField(ShowPercent((100 * barValue) / barMax), GUILayout.MaxWidth(50));
            }

            EditorGUILayout.EndHorizontal();

            EditorGUILayout.EndVertical();
        }

        GUIContent ShowPercent(float percent)
        {
            string text;
            string tooltip;

            if (percent >= 999.95f)
            {
                text = string.Format("{0:f0}%", percent);
                tooltip = string.Format("{0:f2}%", percent);
            }
            else if (percent >= 99.995f)
            {
                text = string.Format("{0:f1}%", percent);
                tooltip = string.Format("{0:f2}%", percent);
            }
            else
            {
                text = string.Format("{0:f2}%", percent);
                tooltip = text;
            }


            return new GUIContent(text, tooltip);
        }

        void DrawComparisonFrameRatio(MarkerData leftMarker, MarkerData rightMarker)
        {
            var leftFrameSummary = m_ProfileLeftView.analysis.GetFrameSummary();
            var rightFrameSummary = m_ProfileRightView.analysis.GetFrameSummary();

            GUIStyle style = GUI.skin.label;
            float w = LayoutSize.WidthColumn0;
            float h = style.lineHeight;
            float ySpacing = 2;
            float barHeight = (h - ySpacing) / 2;

            EditorGUILayout.BeginVertical(GUILayout.Width(w + LayoutSize.WidthColumn1 + LayoutSize.WidthColumn2));

            float leftBarValue = MarkerData.GetMsMean(leftMarker);
            float rightBarValue = MarkerData.GetMsMean(rightMarker);
            float leftBarMax = leftFrameSummary.msMean;
            float rightBarMax = rightFrameSummary.msMean;

            string text = "Mean frame contribution";
            Units units = m_DisplayUnits.Units;
            bool removed = MarkerData.GetTimeIgnored(leftMarker) > 0 || MarkerData.GetTimeIgnored(rightMarker) > 0;
            if (DisplayCount())
            {
                units = Units.Count;
                leftBarValue = MarkerData.GetCountMean(leftMarker);
                rightBarValue = MarkerData.GetCountMean(rightMarker);
                leftBarMax = leftFrameSummary.markerCountMaxMean;
                rightBarMax = rightFrameSummary.markerCountMaxMean;
                text = "Mean count";
            }

            DisplayUnits displayUnits = new DisplayUnits(units);

            float leftBarLength = (leftBarMax > 0) ? (w * leftBarValue) / leftBarMax : 0f;
            leftBarLength = Math.Min(leftBarLength, w);
            float rightBarLength = (rightBarMax > 0) ? (w * rightBarValue) / rightBarMax : 0f;
            rightBarLength = Math.Min(rightBarLength, w);

            EditorGUILayout.LabelField(text);
            m_Columns.SetColumnSizes(LayoutSize.WidthColumn0, LayoutSize.WidthColumn1, LayoutSize.WidthColumn2, LayoutSize.WidthColumn3);
            if (removed)
                m_Columns.Draw4("", "", "", "");
            else
                m_Columns.Draw4("", "Left", "Right", "Diff");
            EditorGUILayout.BeginHorizontal();

            if (removed)
            {
                EditorGUILayout.LabelField("(Marker Removed from analysis)");
            }
            else
            {
                if (m_2D.DrawStart(w, h, Draw2D.Origin.TopLeft, style))
                {
                    m_2D.DrawFilledBox(0, ySpacing, w, h - ySpacing, UIColor.barBackground);

                    m_2D.DrawFilledBox(0, ySpacing, leftBarLength, barHeight, UIColor.left);
                    m_2D.DrawFilledBox(0, ySpacing + barHeight, rightBarLength, barHeight, UIColor.right);
                    m_2D.DrawEnd();

                    Rect rect = GUILayoutUtility.GetLastRect();
                    string tooltip = string.Format("Left: {0}\nRight: {1}", displayUnits.ToTooltipString(leftBarValue, true), displayUnits.ToTooltipString(rightBarValue, true));
                    GUI.Label(rect, new GUIContent("", tooltip));
                }
                float leftPercentage = leftBarMax > 0 ? (100 * leftBarValue) / leftBarMax : 0f;
                float rightPercentage = rightBarMax > 0 ? (100 * rightBarValue) / rightBarMax : 0f;

                EditorGUILayout.LabelField(ShowPercent(leftPercentage), GUILayout.Width(LayoutSize.WidthColumn1));
                EditorGUILayout.LabelField(ShowPercent(rightPercentage), GUILayout.Width(LayoutSize.WidthColumn2));
                if (leftMarker != null && rightMarker != null)
                    EditorGUILayout.LabelField(ShowPercent(rightPercentage - leftPercentage), GUILayout.Width(LayoutSize.WidthColumn3));
            }
            EditorGUILayout.EndHorizontal();

            EditorGUILayout.EndVertical();
        }

        void DrawTopComparison(MarkerData leftMarker, MarkerData rightMarker)
        {
            GUIStyle style = GUI.skin.label;
            float w = LayoutSize.WidthColumn0;
            float h = style.lineHeight;
            float ySpacing = 2;
            float barHeight = (h - ySpacing) / 2;

            EditorGUILayout.BeginVertical(GUILayout.Width(w + LayoutSize.WidthColumn1 + LayoutSize.WidthColumn2));
            bool showCount = DisplayCount();

            float leftMax = MarkerData.GetMsMax(leftMarker);
            float rightMax = MarkerData.GetMsMax(rightMarker);

            Units units = m_DisplayUnits.Units;
            if (showCount)
            {
                units = Units.Count;
                leftMax = MarkerData.GetCountMax(leftMarker);
                rightMax = MarkerData.GetCountMax(rightMarker);
            }
            DisplayUnits displayUnits = new DisplayUnits(units);

            TopMarkerList topMarkerList = new TopMarkerList(m_2D, units,
                LayoutSize.WidthColumn0, LayoutSize.WidthColumn1, LayoutSize.WidthColumn2, LayoutSize.WidthColumn3,
                UIColor.bar, UIColor.barBackground, DrawFrameIndexButton);
            m_TopNumber = topMarkerList.DrawTopNumber(m_TopNumber, m_TopStrings, m_TopValues);

            float barMax = Math.Max(leftMax, rightMax);

            List<FrameTime> leftFrames = leftMarker != null ? topMarkerList.GetTopN(leftMarker, m_TopNumber, showCount) : new List<FrameTime>();
            List<FrameTime> rightFrames = rightMarker != null ? topMarkerList.GetTopN(rightMarker, m_TopNumber, showCount) : new List<FrameTime>();

            FrameTime zeroFrameTime = new FrameTime(-1, 0.0f, 0);
            for (int i = 0; i < m_TopNumber; i++)
            {
                bool leftValid = i < leftFrames.Count;
                bool rightValid = i < rightFrames.Count;
                FrameTime leftFrameTime = leftValid ? leftFrames[i] : zeroFrameTime;
                FrameTime rightFrameTime = rightValid ? rightFrames[i] : zeroFrameTime;

                float leftBarValue = showCount ? leftFrameTime.count : leftFrameTime.ms;
                float rightBarValue = showCount ? rightFrameTime.count : rightFrameTime.ms;

                float leftBarLength = Math.Min((w * leftBarValue) / barMax, w);
                float rightBarLength = Math.Min((w * rightBarValue) / barMax, w);

                EditorGUILayout.BeginHorizontal();
                if (m_2D.DrawStart(w, h, Draw2D.Origin.TopLeft, style))
                {
                    if (leftValid || rightValid)
                    {
                        m_2D.DrawFilledBox(0, ySpacing, w, h - ySpacing, UIColor.barBackground);

                        m_2D.DrawFilledBox(0, ySpacing, leftBarLength, barHeight, UIColor.left);
                        m_2D.DrawFilledBox(0, ySpacing + barHeight, rightBarLength, barHeight, UIColor.right);
                    }
                    m_2D.DrawEnd();

                    Rect rect = GUILayoutUtility.GetLastRect();
                    string leftContent = leftValid ? displayUnits.ToTooltipString(leftBarValue, true, leftFrameTime.frameIndex) : "None";
                    string rightContent = rightValid ? displayUnits.ToTooltipString(rightBarValue, true, rightFrameTime.frameIndex) : "None";
                    GUI.Label(rect, new GUIContent("", string.Format("Left:\t{0}\nRight:\t{1}", leftContent, rightContent)));
                }

                EditorGUILayout.LabelField(leftValid ? displayUnits.ToGUIContentWithTooltips(leftBarValue, frameIndex: leftFrameTime.frameIndex) : Styles.emptyString, GUILayout.Width(LayoutSize.WidthColumn1));
                EditorGUILayout.LabelField(rightValid ? displayUnits.ToGUIContentWithTooltips(rightBarValue, frameIndex: rightFrameTime.frameIndex) : Styles.emptyString, GUILayout.Width(LayoutSize.WidthColumn2));
                if (leftValid || rightValid)
                    EditorGUILayout.LabelField(displayUnits.ToGUIContentWithTooltips(rightBarValue - leftBarValue), GUILayout.Width(LayoutSize.WidthColumn3));
                EditorGUILayout.EndHorizontal();
            }

            EditorGUILayout.EndVertical();
        }

        void DrawSelectedStats(MarkerData marker, ProfileDataView markerContext)
        {
            GUIStyle style = GUI.skin.label;

            m_Columns.Draw3("", GetDisplayUnits(), "Frame");
            Draw3LabelMsFrame(Styles.max, marker.msMax, marker.maxFrameIndex, markerContext);
            Draw2LabelMs(Styles.upperQuartile, marker.msUpperQuartile);
            Draw3LabelMsFrame(Styles.median, marker.msMedian, marker.medianFrameIndex, markerContext);
            Draw2LabelMs(Styles.mean, marker.msMean);
            Draw2LabelMs(Styles.lowerQuartile, marker.msLowerQuartile);
            Draw3LabelMsFrame(Styles.min, marker.msMin, marker.minFrameIndex, markerContext);

            GUILayout.Space(style.lineHeight);

            Draw3LabelMsFrame(Styles.individualMax, marker.msMaxIndividual,
                marker.maxIndividualFrameIndex, markerContext);
            Draw3LabelMsFrame(Styles.individualMin, marker.msMinIndividual,
                marker.minIndividualFrameIndex, markerContext);
        }

        void DrawSelected()
        {
            EditorGUILayout.BeginVertical(GUI.skin.box, GUILayout.Width(LayoutSize.WidthRHS));

            bool lastMarkerSummary = m_ShowMarkerSummary;
            m_ShowMarkerSummary = BoldFoldout(m_ShowMarkerSummary, Styles.markerSummary);
            var analytic = ProfileAnalyzerAnalytics.BeginAnalytic();
            if (m_ShowMarkerSummary)
            {
                if (IsAnalysisValid())
                {
                    List<MarkerData> markers = m_ProfileSingleView.analysis.GetMarkers();
                    if (markers != null)
                    {
                        int markerAt = m_SelectedMarker.id;
                        if (markerAt >= 0 && markerAt < markers.Count)
                        {
                            var marker = markers[markerAt];

                            m_MarkerSummaryScroll = GUILayout.BeginScrollView(m_MarkerSummaryScroll, GUIStyle.none, GUI.skin.verticalScrollbar);
                            Rect clipRect = new Rect(m_MarkerSummaryScroll.x, m_MarkerSummaryScroll.y, LayoutSize.WidthRHS, 500);
                            m_2D.SetClipRect(clipRect);

                            EditorGUILayout.BeginVertical();

                            EditorGUILayout.LabelField(marker.name,
                                GUILayout.MaxWidth(LayoutSize.WidthRHS -
                                    (GUI.skin.box.padding.horizontal + GUI.skin.box.margin.horizontal)));

                            DrawFrameRatio(marker);

                            m_Columns.SetColumnSizes(LayoutSize.WidthColumn0, LayoutSize.WidthColumn1, LayoutSize.WidthColumn2, LayoutSize.WidthColumn3);

                            EditorGUILayout.BeginHorizontal();
                            m_Columns.Draw(0, Styles.firstFrame);
                            m_Columns.Draw(1, Styles.emptyString);
                            DrawFrameIndexButton(marker.firstFrameIndex, m_ProfileSingleView);
                            EditorGUILayout.EndHorizontal();

                            GUIStyle style = GUI.skin.label;

                            float min = marker.msMin;
                            float max = marker.msMax;
                            string fieldString = "marker time";
                            Units units = m_DisplayUnits.Units;
                            if (DisplayCount())
                            {
                                min = marker.countMin;
                                max = marker.countMax;
                                fieldString = "count";
                                units = Units.Count;
                            }

                            TopMarkerList topMarkerList = new TopMarkerList(m_2D, units,
                                LayoutSize.WidthColumn0, LayoutSize.WidthColumn1, LayoutSize.WidthColumn2, LayoutSize.WidthColumn3,
                                UIColor.bar, UIColor.barBackground, DrawFrameIndexButton);
                            m_TopNumber = topMarkerList.Draw(marker, m_ProfileSingleView, m_TopNumber, m_TopStrings, m_TopValues);

                            GUILayout.Space(style.lineHeight);

                            float plotWidth = 40 + GUI.skin.box.padding.horizontal;
                            float plotHeight = 100;

                            EditorGUILayout.BeginHorizontal();

                            Histogram histogram = new Histogram(m_2D, units);
                            DrawHistogramForMarker(histogram, marker);

                            BoxAndWhiskerPlot boxAndWhiskerPlot = new BoxAndWhiskerPlot(m_2D, units);
                            DrawBoxAndWhiskerPlotForMarker(boxAndWhiskerPlot, plotWidth, plotHeight, m_ProfileSingleView.analysis, marker,
                                min, max, UIColor.standardLine, UIColor.boxAndWhiskerBoxColor);

                            boxAndWhiskerPlot.DrawText(m_Columns.GetColumnWidth(3), plotHeight, min, max,
                                string.Format("Min {0} for selected frames", fieldString),
                                string.Format("Max {0} for selected frames", fieldString));
                            EditorGUILayout.EndHorizontal();

                            GUILayout.Space(style.lineHeight);

                            DrawSelectedStats(marker, m_ProfileSingleView);

                            EditorGUILayout.EndVertical();

                            m_2D.ClearClipRect();
                            GUILayout.EndScrollView();
                        }
                        else
                        {
                            EditorGUILayout.LabelField("Marker not in selection");
                        }
                    }
                }
                else
                {
                    EditorGUILayout.LabelField("No marker data selected");
                }
            }

            if (m_ShowMarkerSummary != lastMarkerSummary)
            {
                ProfileAnalyzerAnalytics.SendUIVisibilityEvent(ProfileAnalyzerAnalytics.UIVisibility.Markers, analytic.GetDurationInSeconds(), m_ShowMarkerSummary);
            }

            EditorGUILayout.EndVertical();
        }

        internal static bool FileInTempDir(string filePath)
        {
            return Directory.Exists(TmpDir) && Directory.GetFiles(TmpDir).Contains(filePath);
        }
    }
}
