using System;
using System.IO;
using UnityEngine;
using UnityEngine.TestTools;
using UnityEditor.TestTools.CodeCoverage.Utils;
using UnityEditor.TestTools.TestRunner;
using UnityEditor.TestTools.CodeCoverage.Analytics;
using UnityEditorInternal;
using System.Collections.Generic;
using System.Linq;

namespace UnityEditor.TestTools.CodeCoverage
{
    [ExcludeFromCoverage]
    internal class CodeCoverageWindow : EditorWindow
    {
        private bool m_EnableCodeCoverage;
#if !UNITY_2019_3_OR_NEWER
        private bool m_HasLatestScriptingRuntime;
#endif
        private string m_CodeCoveragePath;
        private string m_CodeCoverageHistoryPath;
        private FolderDropDownMenu m_ResultsFolderDropDownMenu;
        private FolderDropDownMenu m_HistoryFolderDropDownMenu;
        private CoverageFormat m_CodeCoverageFormat;
        private bool m_IncludeHistoryInReport;
        private string m_AssembliesToInclude;
        private int m_AssembliesToIncludeLength;
        private string m_PathsToInclude;
        private string m_PathsToExclude;
        private PathToAddDropDownMenu m_AddPathToIncludeDropDownMenu;
        private PathToAddDropDownMenu m_AddPathToExcludeDropDownMenu;

        private CoverageReportGenerator m_ReportGenerator;
        private bool m_GenerateHTMLReport;
        private bool m_GenerateAdditionalReports;
        private bool m_GenerateBadge;
        private bool m_GenerateAdditionalMetrics;
        private bool m_GenerateTestReferences;
        private bool m_AutoGenerateReport;
        private bool m_OpenReportWhenGenerated;

        private CoverageSettings m_CoverageSettings;

        private static readonly Vector2 s_WindowMinSizeNormal = new Vector2(445, 65);

        private bool m_LoseFocus = false; 

        private bool m_GenerateReport = false;
        private bool m_StopRecording = false;

        private ReorderableList m_PathsToIncludeReorderableList;
        private ReorderableList m_PathsToExcludeReorderableList;
        private List<string> m_PathsToIncludeList;
        private List<string> m_PathsToExcludeList;

        private Vector2 m_WindowScrollPosition = new Vector2(0, 0);

        private readonly string kLatestScriptingRuntimeMessage = L10n.Tr("Code Coverage requires the latest Scripting Runtime Version (.NET 4.x). You can set this in the Player Settings.");
        private readonly string kCodeCoverageDisabledNoRestartMessage = L10n.Tr("Code Coverage should be enabled in order to generate Coverage data and reports.\nNote that Code Coverage can affect the Editor performance.");
        private readonly string kEnablingCodeCoverageMessage = L10n.Tr("Enabling Code Coverage will not take effect until Unity is restarted.");
        private readonly string kDisablingCodeCoverageMessage = L10n.Tr("Disabling Code Coverage will not take effect until Unity is restarted.");
        private readonly string kCodeOptimizationMessage = L10n.Tr("Code Coverage requires Code Optimization to be set to debug mode in order to obtain accurate coverage information.");
        private readonly string kBurstCompilationOnMessage = L10n.Tr("Code Coverage requires Burst Compilation to be disabled in order to obtain accurate coverage information.");
        private readonly string kSelectCoverageDirectoryMessage = L10n.Tr("Select the Coverage results directory");
        private readonly string kSelectCoverageHistoryDirectoryMessage = L10n.Tr("Select the Coverage Report history directory");
        private readonly string kClearDataMessage = L10n.Tr("Are you sure you would like to clear the Coverage results from previous test runs or from previous Coverage Recording sessions? Note that you cannot undo this action.");
        private readonly string kClearHistoryMessage = L10n.Tr("Are you sure you would like to clear the coverage report history? Note that you cannot undo this action.");
        private readonly string kNoAssembliesSelectedMessage = L10n.Tr("Make sure you have included at least one assembly.");
        private readonly string kSettingOverriddenMessage = L10n.Tr("{0} is overridden by the {1} command line argument.");
        private readonly string kSettingsOverriddenMessage = L10n.Tr("{0} are overridden by the {1} command line argument.");

        private readonly string[] kVerbosityLevelLabels = new string[]
        {
            "Verbose",
            "Info",
            "Warning",
            "Error",
            "Off"
        };

        private void Update()
        {
            if (m_GenerateReport)
            {
                // Start the timer for analytics on 'Generate from Last' (report only)
                CoverageAnalytics.instance.StartTimer();
                CoverageAnalytics.instance.CurrentCoverageEvent.actionID = ActionID.ReportOnly;
                CoverageAnalytics.instance.CurrentCoverageEvent.generateFromLast = true;

                m_ReportGenerator.Generate(m_CoverageSettings);
                m_GenerateReport = false;
            }

            if (m_StopRecording)
            {
                CodeCoverage.StopRecordingInternal();
                m_StopRecording = false;
            }
        }

        public void LoseFocus()
        {
            m_LoseFocus = true;
        }

        public string AssembliesToInclude
        {
            set
            {
                m_AssembliesToInclude = value.TrimStart(',').TrimEnd(',');
                m_AssembliesToIncludeLength = m_AssembliesToInclude.Length;
                CoveragePreferences.instance.SetString("IncludeAssemblies", m_AssembliesToInclude);
            }
        }

        public string PathsToInclude
        {
            set
            {
                m_PathsToInclude = value.TrimStart(',').TrimEnd(',');
                m_PathsToInclude = CoverageUtils.NormaliseFolderSeparators(m_PathsToInclude, false);
                CoveragePreferences.instance.SetStringForPaths("PathsToInclude", m_PathsToInclude);

                m_PathsToIncludeList = m_PathsToInclude.Split(new[] { ',' }, StringSplitOptions.RemoveEmptyEntries).ToList();
                m_PathsToIncludeReorderableList.list = m_PathsToIncludeList;

                CoverageAnalytics.instance.CurrentCoverageEvent.updateIncludedPaths = true;
            }
        }

        public string PathsToExclude
        {
            set
            {
                m_PathsToExclude = value.TrimStart(',').TrimEnd(',');
                m_PathsToExclude = CoverageUtils.NormaliseFolderSeparators(m_PathsToExclude, false);
                CoveragePreferences.instance.SetStringForPaths("PathsToExclude", m_PathsToExclude);

                m_PathsToExcludeList = new List<string>(m_PathsToExclude.Split(new[] { ',' }, StringSplitOptions.RemoveEmptyEntries));
                m_PathsToExcludeReorderableList.list = m_PathsToExcludeList;

                CoverageAnalytics.instance.CurrentCoverageEvent.updateExcludedPaths = true;
            }
        }

        static class Styles
        {
            static bool s_Initialized;

            public static readonly GUIContent SwitchToDebugCodeOptimizationButton = EditorGUIUtility.TrTextContent("Switch to debug mode");
            public static readonly GUIContent SwitchBurstCompilationOffButton = EditorGUIUtility.TrTextContent("Disable Burst Compilation");
            public static readonly GUIContent CodeCoverageResultsLocationLabel = EditorGUIUtility.TrTextContent("Results Location", "Specify the folder where the coverage results and report are saved to. The default location is the Project's folder.\n\nClick the dropdown to open the containing folder, change the location or reset to the default location.");
            public static readonly GUIContent CodeCoverageHistoryLocationLabel = EditorGUIUtility.TrTextContent("Report History Location", "Specify the folder where the coverage report history is saved to. The default location is the Project's folder.\n\nClick the dropdown to open the containing folder, change the location or reset to the default location.");
            public static readonly GUIContent ResultsFolderButton = EditorGUIUtility.TrIconContent(EditorIcons.FolderOpened, "Specify the folder where the coverage results and report are saved to.\n\nClick this to open the containing folder, change the location or reset to the default location.");
            public static readonly GUIContent HistoryFolderButton = EditorGUIUtility.TrIconContent(EditorIcons.FolderOpened, "Specify the folder where the coverage report history is saved to.\n\nClick this to open the containing folder, change the location or reset to the default location.");
            public static readonly GUIContent CoverageSettingsLabel = EditorGUIUtility.TrTextContent("Settings");
            public static readonly GUIContent CoverageReportOptionsLabel = EditorGUIUtility.TrTextContent("Report Options");
            public static readonly GUIContent EnableCodeCoverageLabel = EditorGUIUtility.TrTextContent("Enable Code Coverage", "Check this to enable Code Coverage. This is required in order to generate Coverage data and reports. Note that Code Coverage can affect the Editor performance.");
            public static readonly GUIContent CodeCoverageFormat = EditorGUIUtility.TrTextContent("Coverage Format", "The Code Coverage format used when saving the results.");
            public static readonly GUIContent GenerateAdditionalMetricsLabel = EditorGUIUtility.TrTextContent("Additional Metrics", "Check this to generate and include additional metrics in the HTML report. These currently include Cyclomatic Complexity and Crap Score calculations for each method.");
            public static readonly GUIContent CoverageHistoryLabel = EditorGUIUtility.TrTextContent("Report History", "Check this to generate and include the coverage history in the HTML report.");
            public static readonly GUIContent AssembliesToIncludeLabel = EditorGUIUtility.TrTextContent("Included Assemblies", "Specify the assemblies that will be included in the coverage results.\n\nClick the dropdown to view and select or deselect the assemblies.");
            public static readonly GUIContent AssembliesToIncludeDropdownLabel = EditorGUIUtility.TrTextContent("<this will contain a list of the assemblies>", "<this will contain a list of the assemblies>");
            public static readonly GUIContent AssembliesToIncludeEmptyDropdownLabel = EditorGUIUtility.TrTextContent(" Select", "Click this to view and select or deselect the assemblies.");
            public static readonly GUIContent PathsToIncludeLabel = EditorGUIUtility.TrTextContent("Included Paths", "Click Add (+) to specify individual folders and files to include in coverage results. You can use globbing to filter the paths. If the list is empty, Unity includes all files in the Included Assemblies.\n\nTo remove an individual list entry, select the entry and then click Remove (-).");
            public static readonly GUIContent PathsToExcludeLabel = EditorGUIUtility.TrTextContent("Excluded Paths", "Click Add (+) to specify individual folders and files to exclude from coverage results. You can use globbing to filter the paths.\n\nTo remove an individual list entry, select the entry and then click Remove (-).");
            public static readonly GUIContent VerbosityLabel = EditorGUIUtility.TrTextContent("Log Verbosity Level", "Click the dropdown to set the verbosity level for the editor and console logs.\n\nVerbose: All logs\nInfo: Logs, Warnings and Errors\nWarning: Warnings and Errors\nError: Only Errors\nOff: No logs");
            public static readonly GUIContent GenerateHTMLReportLabel = EditorGUIUtility.TrTextContent("HTML Report", "Check this to generate an HTML report.");
            public static readonly GUIContent GenerateAdditionalReportsLabel = EditorGUIUtility.TrTextContent("Additional Reports", "Check this to generate SonarQube, Cobertura and LCOV reports.");
            public static readonly GUIContent GenerateBadgeReportLabel = EditorGUIUtility.TrTextContent("Summary Badges", "Check this to generate coverage summary badges in SVG and PNG format.");
            public static readonly GUIContent GenerateTestRunnerReferencesLabel = EditorGUIUtility.TrTextContent("Test Runner References", "Check this to include test references to the generated coverage results and enable the 'Coverage by test methods' section in the HTML report, allowing you to see how each test contributes to the overall coverage.");
            public static readonly GUIContent AutoGenerateReportLabel = EditorGUIUtility.TrTextContent("Auto Generate Report", "Check this to generate the report automatically after the Test Runner has finished running the tests or the Coverage Recording session has completed.");
            public static readonly GUIContent OpenReportWhenGeneratedLabel = EditorGUIUtility.TrTextContent("Auto Open Report", "Check this to open the coverage report automatically after it has been generated.");
            public static readonly GUIContent GenerateReportButtonLabel = EditorGUIUtility.TrTextContent("Generate Report", "Generates a coverage report from the last set of tests that were run in the Test Runner or from the last Coverage Recording session.");
            public static readonly GUIContent ClearCoverageButtonLabel = EditorGUIUtility.TrTextContent("Clear Results", "Clears the Coverage results from previous test runs or from previous Coverage Recording sessions, for the current project.");
            public static readonly GUIContent ClearHistoryButtonLabel = EditorGUIUtility.TrTextContent("Clear History", "Clears the coverage report history.");
            public static readonly GUIContent StartRecordingButton = EditorGUIUtility.TrIconContent("Record Off", "Record coverage data.");
            public static readonly GUIContent StopRecordingButton = EditorGUIUtility.TrIconContent("Record On", "Stop recording coverage data.");
            public static readonly GUIContent PauseRecordingButton = EditorGUIUtility.TrIconContent("PauseButton", "Pause recording coverage data.");
            public static readonly GUIContent UnpauseRecordingButton = EditorGUIUtility.TrIconContent("PlayButton", "Resume recording coverage data.");
            public static readonly GUIContent HelpIcon = EditorGUIUtility.TrIconContent("_Help", "Open Reference for Code Coverage.");

            public static readonly GUIStyle settings = new GUIStyle();

            public static void Init()
            {
                if (s_Initialized)
                    return;

                s_Initialized = true;

                settings.margin = new RectOffset(8, 4, 10, 4);
            }
        }

        [MenuItem("Window/Analysis/Code Coverage")]
        public static void ShowWindow()
        {
#if TEST_FRAMEWORK_1_1_18_OR_NEWER
            TestRunnerWindow.ShowWindow();
#else
            TestRunnerWindow.ShowPlaymodeTestsRunnerWindowCodeBased();
#endif
            CodeCoverageWindow window = GetWindow<CodeCoverageWindow>(typeof(TestRunnerWindow));
            window.Show();
        }

        private void OnFocus()
        {
            Repaint();
        }

        private void ResetWindow()
        {
            CodeCoverageWindow window = GetWindow<CodeCoverageWindow>(typeof(TestRunnerWindow));
            window.minSize = s_WindowMinSizeNormal;
            window.titleContent = EditorGUIUtility.TrTextContentWithIcon(L10n.Tr("Code Coverage"), EditorIcons.CoverageWindow);
        }

        private void InitCodeCoverageWindow()
        {
            m_CoverageSettings = new CoverageSettings()
            {
                resultsPathFromCommandLine = CommandLineManager.instance.coverageResultsPath,
                historyPathFromCommandLine = CommandLineManager.instance.coverageHistoryPath
            };

            m_CodeCoveragePath = CoveragePreferences.instance.GetStringForPaths("Path", string.Empty);
            m_CodeCoverageHistoryPath = CoveragePreferences.instance.GetStringForPaths("HistoryPath", string.Empty);
            m_CodeCoverageFormat = (CoverageFormat)CoveragePreferences.instance.GetInt("Format", 0);
            m_GenerateAdditionalMetrics = CoveragePreferences.instance.GetBool("GenerateAdditionalMetrics", false);
            m_GenerateTestReferences = CoveragePreferences.instance.GetBool("GenerateTestReferences", false);
            m_IncludeHistoryInReport = CoveragePreferences.instance.GetBool("IncludeHistoryInReport", true);
            m_AssembliesToInclude = GetIncludedAssemblies();
            m_AssembliesToIncludeLength = m_AssembliesToInclude.Length;
            m_PathsToInclude = CoveragePreferences.instance.GetStringForPaths("PathsToInclude", string.Empty);
            m_PathsToExclude = CoveragePreferences.instance.GetStringForPaths("PathsToExclude", string.Empty);
            CodeCoverage.VerbosityLevel = (LogVerbosityLevel)CoveragePreferences.instance.GetInt("VerbosityLevel", 1);
            m_ReportGenerator = new CoverageReportGenerator();
            m_GenerateHTMLReport = CoveragePreferences.instance.GetBool("GenerateHTMLReport", true);
            m_GenerateAdditionalReports = CoveragePreferences.instance.GetBool("GenerateAdditionalReports", false);
            m_GenerateBadge = CoveragePreferences.instance.GetBool("GenerateBadge", true);
            m_AutoGenerateReport = CoveragePreferences.instance.GetBool("AutoGenerateReport", true);
            m_OpenReportWhenGenerated = CoveragePreferences.instance.GetBool("OpenReportWhenGenerated", true);

            m_ResultsFolderDropDownMenu = new FolderDropDownMenu(this, FolderType.Results);
            m_HistoryFolderDropDownMenu = new FolderDropDownMenu(this, FolderType.History);
            m_AddPathToIncludeDropDownMenu = new PathToAddDropDownMenu(this, PathFilterType.Include);
            m_AddPathToExcludeDropDownMenu = new PathToAddDropDownMenu(this, PathFilterType.Exclude);
            m_PathsToIncludeList = new List<string>(m_PathsToInclude.Split(new[] { ',' }, StringSplitOptions.RemoveEmptyEntries));
            m_PathsToIncludeReorderableList = new ReorderableList(m_PathsToIncludeList, typeof(String), true, false, true, true);
            m_PathsToIncludeReorderableList.headerHeight = 0;
            m_PathsToExcludeList = new List<string>(m_PathsToExclude.Split(new[] { ',' }, StringSplitOptions.RemoveEmptyEntries));
            m_PathsToExcludeReorderableList = new ReorderableList(m_PathsToExcludeList, typeof(String), true, false, true, true);
            m_PathsToExcludeReorderableList.headerHeight = 0;

            UpdateCoverageSettings();

            m_GenerateReport = false;
            m_StopRecording = false;

            ResetWindow();
        }

        void UpdateCoverageSettings()
        {
            if (m_CoverageSettings != null)
            {
                m_CoverageSettings.rootFolderPath = CoverageUtils.GetRootFolderPath(m_CoverageSettings);
                m_CoverageSettings.historyFolderPath = CoverageUtils.GetHistoryFolderPath(m_CoverageSettings);

                if (m_CodeCoverageFormat == CoverageFormat.OpenCover)
                {
                    m_CoverageSettings.resultsFolderSuffix = "-opencov";
                    string folderName = CoverageUtils.GetProjectFolderName();
                    string resultsRootDirectoryName = string.Concat(folderName, m_CoverageSettings.resultsFolderSuffix);
                    m_CoverageSettings.resultsRootFolderPath = CoverageUtils.NormaliseFolderSeparators(CoverageUtils.JoinPaths(m_CoverageSettings.rootFolderPath, resultsRootDirectoryName));
                }
            }
        }

        private string GetIncludedAssemblies()
        {
            string assembliesFromPreferences = CoveragePreferences.instance.GetString("IncludeAssemblies", AssemblyFiltering.GetUserOnlyAssembliesString());
            string filteredAssemblies = AssemblyFiltering.RemoveAssembliesThatNoLongerExist(assembliesFromPreferences);
            CoveragePreferences.instance.SetString("IncludeAssemblies", filteredAssemblies);

            return filteredAssemblies;
        }

        private void OnEnable()
        {
            InitCodeCoverageWindow();
        }

        public void OnGUI()
        {
            Styles.Init();

            EditorGUIUtility.labelWidth = 190f;

            GUILayout.BeginVertical();

            float toolbarHeight = 0;
#if !UNITY_2019_3_OR_NEWER
            using (new EditorGUI.DisabledScope(!m_HasLatestScriptingRuntime))
#endif
            {
                toolbarHeight = DrawToolbar();
            }

            // Window scrollbar
            float maxHeight = Mathf.Max(position.height, 0) - toolbarHeight;
            m_WindowScrollPosition = EditorGUILayout.BeginScrollView(m_WindowScrollPosition, GUILayout.Height(maxHeight));

            GUILayout.BeginVertical(Styles.settings);

#if !UNITY_2019_3_OR_NEWER
            CheckScriptingRuntimeVersion();
#endif
            CheckCoverageEnabled();
#if UNITY_2020_1_OR_NEWER
            CheckCodeOptimization();
#endif
#if BURST_INSTALLED
            CheckBurstCompilation();
#endif

#if !UNITY_2019_3_OR_NEWER
            using (new EditorGUI.DisabledScope(!m_HasLatestScriptingRuntime))
#endif
            {
                // Draw Settings
                EditorGUILayout.LabelField(Styles.CoverageSettingsLabel, EditorStyles.boldLabel);
                using (new EditorGUI.DisabledScope(CoverageRunData.instance.isRunning))
                {
                    DrawCoverageSettings();
                }

                // Draw Coverage Report Options
                GUILayout.Space(10);
                EditorGUILayout.LabelField(Styles.CoverageReportOptionsLabel, EditorStyles.boldLabel);
                using (new EditorGUI.DisabledScope(CoverageRunData.instance.isRunning))
                {
                    DrawCoverageReportOptions();
                }
            }

            GUILayout.EndVertical();

            EditorGUILayout.EndScrollView();
            GUILayout.EndVertical();

            HandleFocusRepaint();
        }

        float DrawToolbar()
        {
            GUILayout.BeginHorizontal(EditorStyles.toolbar);

            GUILayout.Space(10);

            // Coverage Recording button
            bool isRunning = CoverageRunData.instance.isRunning;
            bool isRecording = CoverageRunData.instance.isRecording;
            bool isRecordingPaused = CoverageRunData.instance.isRecordingPaused;

            using (new EditorGUI.DisabledScope((isRunning && !isRecording) || m_StopRecording || m_AssembliesToIncludeLength == 0 || !Coverage.enabled || m_EnableCodeCoverage != Coverage.enabled || EditorApplication.isCompiling))
            {
                if (EditorGUILayout.DropdownButton(isRecording ? Styles.StopRecordingButton : Styles.StartRecordingButton, FocusType.Keyboard, EditorStyles.toolbarButton))
                {
                    if (isRecording)
                    {
                        m_StopRecording = true;
                    }
                    else
                    {
                        CodeCoverage.StartRecordingInternal();
                    }
                }
            }

            using (new EditorGUI.DisabledScope(!isRecording || m_StopRecording || m_AssembliesToIncludeLength == 0 || !Coverage.enabled || m_EnableCodeCoverage != Coverage.enabled || EditorApplication.isCompiling))
            {
                if (EditorGUILayout.DropdownButton(isRecordingPaused ? Styles.UnpauseRecordingButton : Styles.PauseRecordingButton, FocusType.Keyboard, EditorStyles.toolbarButton))
                {
                    if (isRecordingPaused)
                    {
                        CodeCoverage.UnpauseRecordingInternal();
                    }
                    else
                    {
                        CodeCoverage.PauseRecordingInternal();
                    }
                }
            }

            using (new EditorGUI.DisabledScope((!m_GenerateHTMLReport && !m_GenerateBadge && !m_GenerateAdditionalReports) || !DoesResultsRootFolderExist() || CoverageRunData.instance.isRunning || m_GenerateReport || m_AssembliesToIncludeLength == 0 || !Coverage.enabled || m_EnableCodeCoverage != Coverage.enabled || EditorApplication.isCompiling))
            {
                if (EditorGUILayout.DropdownButton(Styles.GenerateReportButtonLabel, FocusType.Keyboard, EditorStyles.toolbarButton))
                {
                    m_GenerateReport = true;
                }
            }

            GUILayout.FlexibleSpace();

            using (new EditorGUI.DisabledScope(!DoesResultsRootFolderExist() || CoverageRunData.instance.isRunning))
            {
                if (EditorGUILayout.DropdownButton(Styles.ClearCoverageButtonLabel, FocusType.Keyboard, EditorStyles.toolbarButton))
                {
                    ClearResultsRootFolderIfExists();
                }
            }

            using (new EditorGUI.DisabledScope(!DoesReportHistoryExist() || CoverageRunData.instance.isRunning))
            {
                if (EditorGUILayout.DropdownButton(Styles.ClearHistoryButtonLabel, FocusType.Keyboard, EditorStyles.toolbarButton))
                {
                    ClearReportHistoryFolderIfExists();
                }
            }

            DrawHelpIcon();

            GUILayout.EndHorizontal();

            return EditorStyles.toolbar.fixedHeight;
        }

        void DrawHelpIcon()
        {
            if (GUILayout.Button(Styles.HelpIcon, EditorStyles.toolbarButton))
            {
                PackageManager.PackageInfo packageInfo = PackageManager.PackageInfo.FindForAssetPath("Packages/com.unity.testtools.codecoverage");
                if (packageInfo != null)
                {
                    string shortVersion = packageInfo.version.Substring(0, packageInfo.version.IndexOf('.', packageInfo.version.IndexOf('.') + 1));
                    string documentationUrl = $"https://docs.unity3d.com/Packages/com.unity.testtools.codecoverage@{shortVersion}";
                    Application.OpenURL(documentationUrl);
                }
            }
        }

        void HandleFocusRepaint()
        {
            Rect r = EditorGUILayout.GetControlRect();
            // Lose focus if mouse is down outside of UI elements
            if (Event.current.type == EventType.MouseDown && !r.Contains(Event.current.mousePosition))
            {
                m_LoseFocus = true;
            }

            if (m_LoseFocus)
            {
                GUI.FocusControl("");
                m_PathsToIncludeReorderableList.ReleaseKeyboardFocus();
                m_PathsToExcludeReorderableList.ReleaseKeyboardFocus();
                m_LoseFocus = false;
                Repaint();
            }
        }

#if !UNITY_2019_3_OR_NEWER
        void CheckScriptingRuntimeVersion()
        {
            m_HasLatestScriptingRuntime = PlayerSettings.scriptingRuntimeVersion == ScriptingRuntimeVersion.Latest;

            if (!m_HasLatestScriptingRuntime)
            {
                EditorGUILayout.HelpBox(kLatestScriptingRuntimeMessage, MessageType.Warning);
                GUILayout.Space(5);
            }
        }
#endif

        void CheckCoverageEnabled()
        {
#if UNITY_2020_2_OR_NEWER
            m_EnableCodeCoverage = Coverage.enabled;
#else
            m_EnableCodeCoverage = EditorPrefs.GetBool("CodeCoverageEnabled", false);
#endif
        }

#if UNITY_2020_1_OR_NEWER
        void CheckCodeOptimization()
        {
            if (Compilation.CompilationPipeline.codeOptimization == Compilation.CodeOptimization.Release)
            {
                EditorGUILayout.HelpBox(kCodeOptimizationMessage, MessageType.Warning);

                using (new EditorGUI.DisabledScope(EditorApplication.isCompiling))
                {
                    if (GUILayout.Button(Styles.SwitchToDebugCodeOptimizationButton))
                    {
                        CoverageAnalytics.instance.CurrentCoverageEvent.switchToDebugMode = true;
                        Compilation.CompilationPipeline.codeOptimization = Compilation.CodeOptimization.Debug;
                        EditorPrefs.SetBool("ScriptDebugInfoEnabled", true);
                    }
                }

                GUILayout.Space(5);
            }
        }
#endif

#if BURST_INSTALLED
        void CheckBurstCompilation()
        {
            if (EditorPrefs.GetBool("BurstCompilation", false))
            {
                EditorGUILayout.HelpBox(kBurstCompilationOnMessage, MessageType.Warning);

                if (GUILayout.Button(Styles.SwitchBurstCompilationOffButton))
                {
                    CoverageAnalytics.instance.CurrentCoverageEvent.switchBurstOff = true;
                    EditorApplication.ExecuteMenuItem("Jobs/Burst/Enable Compilation");
                }

                GUILayout.Space(5);
            }
        }
#endif

        void CheckIfIncludedAssembliesIsEmpty()
        {
            if (m_AssembliesToIncludeLength == 0)
            {
                EditorGUILayout.HelpBox(kNoAssembliesSelectedMessage, MessageType.Warning);
            }
        }

        void DrawCodeCoverageLocation()
        {
            GUILayout.BeginHorizontal();

            bool settingPassedInCmdLine = CommandLineManager.instance.runFromCommandLine && CommandLineManager.instance.coverageResultsPath.Length > 0;

            using (new EditorGUI.DisabledScope(CoverageRunData.instance.isRunning || settingPassedInCmdLine))
            {
                Rect textFieldPosition = EditorGUILayout.GetControlRect();
                textFieldPosition = EditorGUI.PrefixLabel(textFieldPosition, Styles.CodeCoverageResultsLocationLabel);
                EditorGUI.SelectableLabel(textFieldPosition, settingPassedInCmdLine ? CommandLineManager.instance.coverageResultsPath : m_CodeCoveragePath, EditorStyles.textField);

                bool autoDetect = !CoverageUtils.EnsureFolderExists(m_CodeCoveragePath);

                if (autoDetect)
                {
                    SetDefaultCoverageLocation();
                }

                Rect btnPosition = GUILayoutUtility.GetRect(Styles.ResultsFolderButton, EditorStyles.miniPullDown, GUILayout.MaxWidth(38));
                if (EditorGUI.DropdownButton(btnPosition, Styles.ResultsFolderButton, FocusType.Keyboard, EditorStyles.miniPullDown))
                {
                    m_ResultsFolderDropDownMenu.Show(btnPosition, m_CodeCoveragePath, kSelectCoverageDirectoryMessage);
                }
            }

            GUILayout.EndHorizontal();

            if (settingPassedInCmdLine)
            {
                EditorGUILayout.HelpBox(string.Format(kSettingOverriddenMessage, "Results Location", "-coverageResultsPath"), MessageType.Warning);
            }
        }

        void DrawCodeCoverageHistoryLocation()
        {
            GUILayout.BeginHorizontal();

            bool settingPassedInCmdLine = CommandLineManager.instance.runFromCommandLine && CommandLineManager.instance.coverageHistoryPath.Length > 0;

            using (new EditorGUI.DisabledScope(CoverageRunData.instance.isRunning || settingPassedInCmdLine))
            {
                Rect textFieldPosition = EditorGUILayout.GetControlRect();
                textFieldPosition = EditorGUI.PrefixLabel(textFieldPosition, Styles.CodeCoverageHistoryLocationLabel);
                EditorGUI.SelectableLabel(textFieldPosition, settingPassedInCmdLine ? CommandLineManager.instance.coverageHistoryPath : m_CodeCoverageHistoryPath, EditorStyles.textField);

                bool autoDetect = !CoverageUtils.EnsureFolderExists(m_CodeCoverageHistoryPath);

                if (autoDetect)
                {
                    SetDefaultCoverageHistoryLocation();
                }

                Rect btnPosition = GUILayoutUtility.GetRect(Styles.HistoryFolderButton, EditorStyles.miniPullDown, GUILayout.MaxWidth(38));
                if (EditorGUI.DropdownButton(btnPosition, Styles.HistoryFolderButton, FocusType.Keyboard, EditorStyles.miniPullDown))
                {
                    m_HistoryFolderDropDownMenu.Show(btnPosition, m_CodeCoverageHistoryPath, kSelectCoverageHistoryDirectoryMessage);
                }
            }

            GUILayout.EndHorizontal();

            if (settingPassedInCmdLine)
            {
                EditorGUILayout.HelpBox(string.Format(kSettingOverriddenMessage, "History Location", "-coverageHistoryPath"), MessageType.Warning);
            }
        }

        void DrawIncludedAssemblies()
        {
            GUILayout.BeginHorizontal();

            bool settingPassedInCmdLine = CommandLineManager.instance.runFromCommandLine && CommandLineManager.instance.assemblyFiltersSpecified;

            using (new EditorGUI.DisabledScope(CoverageRunData.instance.isRunning || settingPassedInCmdLine))
            {
                EditorGUILayout.PrefixLabel(Styles.AssembliesToIncludeLabel);

                Rect buttonRect = EditorGUILayout.GetControlRect(GUILayout.MinWidth(10));

                Styles.AssembliesToIncludeDropdownLabel.text = string.Concat(" ", m_AssembliesToInclude);
                Styles.AssembliesToIncludeDropdownLabel.tooltip = m_AssembliesToInclude.Replace(",", "\n");

                if (EditorGUI.DropdownButton(buttonRect, m_AssembliesToInclude.Length > 0 ? Styles.AssembliesToIncludeDropdownLabel : Styles.AssembliesToIncludeEmptyDropdownLabel, FocusType.Keyboard, EditorStyles.miniPullDown))
                {
                    CoverageAnalytics.instance.CurrentCoverageEvent.enterAssembliesDialog = true;
                    GUI.FocusControl("");
                    PopupWindow.Show(buttonRect, new IncludedAssembliesPopupWindow(this, m_AssembliesToInclude) { Width = buttonRect.width });
                }
            }

            GUILayout.EndHorizontal();

            if (settingPassedInCmdLine)
            {
                EditorGUILayout.HelpBox(string.Format(kSettingOverriddenMessage, "Included Assemblies", "-coverageOptions: assemblyFilters"), MessageType.Warning);
            }
        }

        void DrawPathFiltering()
        {
            bool settingPassedInCmdLine = CommandLineManager.instance.runFromCommandLine && CommandLineManager.instance.pathFiltersSpecified;

            using (new EditorGUI.DisabledScope(CoverageRunData.instance.isRunning || settingPassedInCmdLine))
            {
                DrawIncludedPaths();
                DrawExcludedPaths();
            }

            if (settingPassedInCmdLine)
            {
                EditorGUILayout.HelpBox(string.Format(kSettingsOverriddenMessage, "Included/Excluded Paths", "-coverageOptions: pathFilters/pathFiltersFromFile"), MessageType.Warning);
            }
        }

        void DrawIncludedPaths()
        {
            GUILayout.BeginHorizontal();

            EditorGUILayout.PrefixLabel(Styles.PathsToIncludeLabel);

            GUILayout.BeginVertical();

            GUILayout.Space(4);
            EditorGUI.BeginChangeCheck();

            m_PathsToIncludeReorderableList.drawElementCallback = DrawPathsToIncludeListItem;
            m_PathsToIncludeReorderableList.onChangedCallback = (rl) => { PathsToInclude = string.Join(",", rl.list as List<string>); };
            m_PathsToIncludeReorderableList.onAddDropdownCallback = (rect, rl) => { m_AddPathToIncludeDropDownMenu.Show(rect, m_PathsToInclude); };
            m_PathsToIncludeReorderableList.DoLayoutList();

            if (EditorGUI.EndChangeCheck())
            {
                OnPathsListChange(m_PathsToIncludeReorderableList, PathFilterType.Include);
            }

            GUILayout.EndVertical();

            GUILayout.Space(2);
            GUILayout.EndHorizontal();
        }

        void DrawExcludedPaths()
        {
            GUILayout.BeginHorizontal();

            EditorGUILayout.PrefixLabel(Styles.PathsToExcludeLabel);

            GUILayout.BeginVertical();

            GUILayout.Space(4);
            EditorGUI.BeginChangeCheck();

            m_PathsToExcludeReorderableList.drawElementCallback = DrawPathsToExcludeListItem;
            m_PathsToExcludeReorderableList.onChangedCallback = (rl) => { PathsToExclude = string.Join(",", rl.list as List<string>); };
            m_PathsToExcludeReorderableList.onAddDropdownCallback = (rect, rl) => { m_AddPathToExcludeDropDownMenu.Show(rect, m_PathsToExclude); };
            m_PathsToExcludeReorderableList.DoLayoutList();

            if (EditorGUI.EndChangeCheck())
            {
                OnPathsListChange(m_PathsToExcludeReorderableList, PathFilterType.Exclude);
            }

            GUILayout.EndVertical();

            GUILayout.Space(2);
            GUILayout.EndHorizontal();
        }

        void DrawPathsToIncludeListItem(Rect rect, int index, bool isActive, bool isFocused)
        {
            if (index >= 0 && index < m_PathsToIncludeList.Count)
            {
                string pathToInclude = m_PathsToIncludeList[index].Replace(",", "");
                m_PathsToIncludeReorderableList.list[index] = EditorGUI.TextField(rect, pathToInclude);
            }
        }

        void DrawPathsToExcludeListItem(Rect rect, int index, bool isActive, bool isFocused)
        {
            if (index >= 0 && index < m_PathsToExcludeList.Count)
            {
                string pathToExclude = m_PathsToExcludeList[index];
                m_PathsToExcludeReorderableList.list[index] = EditorGUI.TextField(rect, pathToExclude);
            }
        }

        void OnPathsListChange(ReorderableList rl, PathFilterType pathFilterType)
        {
            var pathsList = rl.list as List<string>;
            int listSize = pathsList.Count;

            for (int i = 0; i < listSize; ++i)
            {
                var itemStr = pathsList[i];
                itemStr = itemStr.Replace(",", "");

                if (string.IsNullOrWhiteSpace(itemStr))
                {
                    itemStr = "-";
                }

                pathsList[i] = itemStr;
            }

            if (pathFilterType == PathFilterType.Include)
                PathsToInclude = string.Join(",", pathsList);
            else if (pathFilterType == PathFilterType.Exclude)
                PathsToExclude = string.Join(",", pathsList);
        }

        void CheckIfCodeCoverageIsDisabled()
        {
            if (!m_EnableCodeCoverage)
            {
                EditorGUILayout.HelpBox(kCodeCoverageDisabledNoRestartMessage, MessageType.Warning);
            }
#if !UNITY_2020_2_OR_NEWER
            if (m_EnableCodeCoverage != Coverage.enabled)
            {
                if (m_EnableCodeCoverage)
                    EditorGUILayout.HelpBox(kEnablingCodeCoverageMessage, MessageType.Warning);
                else
                    EditorGUILayout.HelpBox(kDisablingCodeCoverageMessage, MessageType.Warning);
            }
#endif
        }

        void DrawVerbosityLevel()
        {
            GUILayout.Space(2);

            bool settingPassedInCmdLine = CommandLineManager.instance.runFromCommandLine && CommandLineManager.instance.verbosityLevelSpecified;

            using (new EditorGUI.DisabledScope(CoverageRunData.instance.isRunning || settingPassedInCmdLine))
            {
                EditorGUI.BeginChangeCheck();
                int verbosityLevel = EditorGUILayout.Popup(Styles.VerbosityLabel, (int)CodeCoverage.VerbosityLevel, kVerbosityLevelLabels);
                if (EditorGUI.EndChangeCheck())
                {
                    CoveragePreferences.instance.SetInt("VerbosityLevel", verbosityLevel);
                    CodeCoverage.VerbosityLevel = (LogVerbosityLevel)verbosityLevel;
                }
            }

            if (settingPassedInCmdLine)
            {
                EditorGUILayout.HelpBox(string.Format(kSettingOverriddenMessage, "Logs Verbosity Level", "-coverageOptions: verbosity"), MessageType.Warning);
            }

            
        }

        void DrawEnableCoverageCheckbox()
        {
            bool settingPassedInCmdLine = CommandLineManager.instance.runFromCommandLine;

            using (new EditorGUI.DisabledScope(EditorApplication.isCompiling || settingPassedInCmdLine))
            {
                EditorGUI.BeginChangeCheck();
                m_EnableCodeCoverage = EditorGUILayout.Toggle(Styles.EnableCodeCoverageLabel, m_EnableCodeCoverage, GUILayout.ExpandWidth(false));
                if (EditorGUI.EndChangeCheck())
                {
                    CoveragePreferences.instance.SetBool("EnableCodeCoverage", m_EnableCodeCoverage);
#if UNITY_2020_2_OR_NEWER
                    Coverage.enabled = m_EnableCodeCoverage;
#else
                    EditorPrefs.SetBool("CodeCoverageEnabled", m_EnableCodeCoverage);
                    EditorPrefs.SetBool("CodeCoverageEnabledMessageShown", false);
                    SettingsService.NotifySettingsProviderChanged();
#endif
                }
            }
            if (settingPassedInCmdLine)
            {
                EditorGUILayout.HelpBox(string.Format(kSettingOverriddenMessage, "Enable Code Coverage", "-enableCodeCoverage"), MessageType.Warning);
            }
            else
            {
                CheckIfCodeCoverageIsDisabled();
            }
        }

        void DrawGenerateHTMLReportCheckbox()
        {
            bool settingPassedInCmdLine = CommandLineManager.instance.runFromCommandLine && CommandLineManager.instance.generateHTMLReport;

            using (new EditorGUI.DisabledScope(CoverageRunData.instance.isRunning || settingPassedInCmdLine))
            {
                EditorGUI.BeginChangeCheck();
                m_GenerateHTMLReport = EditorGUILayout.Toggle(Styles.GenerateHTMLReportLabel, m_GenerateHTMLReport, GUILayout.ExpandWidth(false));
                if (EditorGUI.EndChangeCheck())
                {
                    CoveragePreferences.instance.SetBool("GenerateHTMLReport", m_GenerateHTMLReport);
                }
            }

            if (settingPassedInCmdLine)
            {
                EditorGUILayout.HelpBox(string.Format(kSettingOverriddenMessage, "HTML Report", "-coverageOptions: generateHtmlReport"), MessageType.Warning);
            }
        }

        void DrawGenerateAdditionalReportsCheckbox()
        {
            bool settingPassedInCmdLine = CommandLineManager.instance.runFromCommandLine && CommandLineManager.instance.generateAdditionalReports;

            using (new EditorGUI.DisabledScope(CoverageRunData.instance.isRunning || settingPassedInCmdLine))
            {
                EditorGUI.BeginChangeCheck();
                m_GenerateAdditionalReports = EditorGUILayout.Toggle(Styles.GenerateAdditionalReportsLabel, m_GenerateAdditionalReports, GUILayout.ExpandWidth(false));
                if (EditorGUI.EndChangeCheck())
                {
                    CoveragePreferences.instance.SetBool("GenerateAdditionalReports", m_GenerateAdditionalReports);
                }
            }

            if (settingPassedInCmdLine)
            {
                EditorGUILayout.HelpBox(string.Format(kSettingOverriddenMessage, "Additional Reports", "-coverageOptions: generateAdditionalReports"), MessageType.Warning);
            }
        }

        void DrawGenerateBadgeReportCheckbox()
        {
            bool settingPassedInCmdLine = CommandLineManager.instance.runFromCommandLine && CommandLineManager.instance.generateBadgeReport;

            using (new EditorGUI.DisabledScope(CoverageRunData.instance.isRunning || settingPassedInCmdLine))
            {
                EditorGUI.BeginChangeCheck();
                m_GenerateBadge = EditorGUILayout.Toggle(Styles.GenerateBadgeReportLabel, m_GenerateBadge, GUILayout.ExpandWidth(false));
                if (EditorGUI.EndChangeCheck())
                {
                    CoveragePreferences.instance.SetBool("GenerateBadge", m_GenerateBadge);
                }
            }

            if (settingPassedInCmdLine)
            {
                EditorGUILayout.HelpBox(string.Format(kSettingOverriddenMessage, "Summary Badges", "-coverageOptions: generateBadgeReport"), MessageType.Warning);
            }
        }

        void DrawGenerateHistoryCheckbox()
        {
            bool settingPassedInCmdLine = CommandLineManager.instance.runFromCommandLine && CommandLineManager.instance.generateHTMLReportHistory;

            using (new EditorGUI.DisabledScope(CoverageRunData.instance.isRunning || settingPassedInCmdLine))
            {
                EditorGUI.BeginChangeCheck();
                using (new EditorGUI.DisabledScope(!m_GenerateHTMLReport && !m_GenerateAdditionalReports))
                {
                    m_IncludeHistoryInReport = EditorGUILayout.Toggle(Styles.CoverageHistoryLabel, m_IncludeHistoryInReport, GUILayout.ExpandWidth(false));
                    if (EditorGUI.EndChangeCheck())
                    {
                        CoveragePreferences.instance.SetBool("IncludeHistoryInReport", m_IncludeHistoryInReport);
                    }
                }
            }

            if (settingPassedInCmdLine)
            {
                EditorGUILayout.HelpBox(string.Format(kSettingOverriddenMessage, "Report History", "-coverageOptions: generateHtmlReportHistory"), MessageType.Warning);
            }
        }

        void DrawGenerateAdditionalMetricsCheckbox()
        {
            bool settingPassedInCmdLine = CommandLineManager.instance.runFromCommandLine && CommandLineManager.instance.generateAdditionalMetrics;

            using (new EditorGUI.DisabledScope(CoverageRunData.instance.isRunning || settingPassedInCmdLine))
            {
                EditorGUI.BeginChangeCheck();
                m_GenerateAdditionalMetrics = EditorGUILayout.Toggle(Styles.GenerateAdditionalMetricsLabel, m_GenerateAdditionalMetrics, GUILayout.ExpandWidth(false));
                if (EditorGUI.EndChangeCheck())
                {
                    CoveragePreferences.instance.SetBool("GenerateAdditionalMetrics", m_GenerateAdditionalMetrics);
                }
            }

            if (settingPassedInCmdLine)
            {
                EditorGUILayout.HelpBox(string.Format(kSettingOverriddenMessage, "Additional Metrics", "-coverageOptions: generateAdditionalMetrics"), MessageType.Warning);
            }
        }

        void DrawGenerateTestRunnerReferencesCheckbox()
        {
            bool settingPassedInCmdLine = CommandLineManager.instance.runFromCommandLine && CommandLineManager.instance.generateTestReferences;

            using (new EditorGUI.DisabledScope(CoverageRunData.instance.isRunning || settingPassedInCmdLine))
            {
                EditorGUI.BeginChangeCheck();
                m_GenerateTestReferences = EditorGUILayout.Toggle(Styles.GenerateTestRunnerReferencesLabel, m_GenerateTestReferences, GUILayout.ExpandWidth(false));
                if (EditorGUI.EndChangeCheck())
                {
                    CoveragePreferences.instance.SetBool("GenerateTestReferences", m_GenerateTestReferences);
                }
            }

            if (settingPassedInCmdLine)
            {
                EditorGUILayout.HelpBox(string.Format(kSettingOverriddenMessage, "Test Runner References", "-coverageOptions: generateTestReferences"), MessageType.Warning);
            }
        }

        void DrawAutoGenerateReportCheckbox()
        {
            bool settingPassedInCmdLine = CommandLineManager.instance.runFromCommandLine && (CommandLineManager.instance.generateHTMLReport || CommandLineManager.instance.generateBadgeReport);

            using (new EditorGUI.DisabledScope(CoverageRunData.instance.isRunning || settingPassedInCmdLine))
            {
                EditorGUI.BeginChangeCheck();
                m_AutoGenerateReport = EditorGUILayout.Toggle(Styles.AutoGenerateReportLabel, m_AutoGenerateReport, GUILayout.ExpandWidth(false));
                if (EditorGUI.EndChangeCheck())
                {
                    CoveragePreferences.instance.SetBool("AutoGenerateReport", m_AutoGenerateReport);
                }
            }

            if (settingPassedInCmdLine)
            {
                EditorGUILayout.HelpBox(string.Format(kSettingOverriddenMessage, "Auto Generate Report", CommandLineManager.instance.generateHTMLReport ? "-coverageOptions: generateHtmlReport" : "-coverageOptions: generateBadgeReport"), MessageType.Warning);
            }
        }

        void DrawOpenReportWhenGeneratedCheckbox()
        {
            using (new EditorGUI.DisabledScope(CoverageRunData.instance.isRunning))
            {
                EditorGUI.BeginChangeCheck();
                m_OpenReportWhenGenerated = EditorGUILayout.Toggle(Styles.OpenReportWhenGeneratedLabel, m_OpenReportWhenGenerated, GUILayout.ExpandWidth(false));
                if (EditorGUI.EndChangeCheck())
                {
                    CoveragePreferences.instance.SetBool("OpenReportWhenGenerated", m_OpenReportWhenGenerated);
                }
            }
        }

        void DrawCoverageSettings()
        {
            DrawEnableCoverageCheckbox();
            DrawCodeCoverageLocation();
            DrawCodeCoverageHistoryLocation();
            DrawIncludedAssemblies();
            CheckIfIncludedAssembliesIsEmpty();
            DrawPathFiltering();
            DrawVerbosityLevel();
        }

        void DrawCoverageReportOptions()
        {
            DrawGenerateHTMLReportCheckbox();
            DrawGenerateAdditionalReportsCheckbox();
            DrawGenerateHistoryCheckbox();
            DrawGenerateBadgeReportCheckbox();
            DrawGenerateAdditionalMetricsCheckbox();
            DrawGenerateTestRunnerReferencesCheckbox();
            DrawAutoGenerateReportCheckbox();
            DrawOpenReportWhenGeneratedCheckbox(); 
        }

        private bool DoesResultsRootFolderExist()
        {
            if (m_CoverageSettings == null)
                return false;

            string resultsRootFolderPath = m_CoverageSettings.resultsRootFolderPath;
            return CoverageUtils.GetNumberOfFilesInFolder(resultsRootFolderPath, "*.xml", SearchOption.AllDirectories) > 0;
        }

        private void ClearResultsRootFolderIfExists()
        {
            CoverageAnalytics.instance.CurrentCoverageEvent.clearData = true;

            if (!EditorUtility.DisplayDialog(L10n.Tr("Clear Results"), kClearDataMessage, L10n.Tr("Clear"), L10n.Tr("Cancel")))
                return;

            if (m_CoverageSettings == null)
                return;

            string resultsRootFolderPath = m_CoverageSettings.resultsRootFolderPath;

            CoverageUtils.ClearFolderIfExists(resultsRootFolderPath, "*.xml");
        }

        public string GetResultsRootFolder()
        {
            if (m_CoverageSettings == null)
                return m_CodeCoveragePath;

            string resultsRootFolderPath = m_CoverageSettings.resultsRootFolderPath;
            CoverageUtils.EnsureFolderExists(resultsRootFolderPath);
            return resultsRootFolderPath;
        }

        private bool DoesReportHistoryExist()
        {
            if (m_CoverageSettings == null)
                return false;

            string historyFolderPath = m_CoverageSettings.historyFolderPath;

            return CoverageUtils.GetNumberOfFilesInFolder(historyFolderPath, "????-??-??_??-??-??_CoverageHistory.xml", SearchOption.TopDirectoryOnly) > 0;
        }

        private void ClearReportHistoryFolderIfExists()
        {
            CoverageAnalytics.instance.CurrentCoverageEvent.clearHistory = true;

            if (!EditorUtility.DisplayDialog(L10n.Tr("Clear Report History"), kClearHistoryMessage, L10n.Tr("Clear"), L10n.Tr("Cancel")))
                return;

            if (m_CoverageSettings == null)
                return;

            string historyFolderPath = m_CoverageSettings.historyFolderPath;

            CoverageUtils.ClearFolderIfExists(historyFolderPath, "????-??-??_??-??-??_CoverageHistory.xml");
        }

        public string GetReportHistoryFolder()
        {
            if (m_CoverageSettings == null)
                return m_CodeCoverageHistoryPath;

            string historyFolderPath = m_CoverageSettings.historyFolderPath;
            CoverageUtils.EnsureFolderExists(historyFolderPath);
            return historyFolderPath;
        }

        public void SetCoverageLocation(string folderPath)
        {
            if (CoverageUtils.IsValidFolder(folderPath))
            {
                m_CodeCoveragePath = folderPath;
                CoveragePreferences.instance.SetStringForPaths("Path", m_CodeCoveragePath);
                UpdateCoverageSettings();
            }
        }

        public void SetDefaultCoverageLocation()
        {
            SetCoverageLocation(CoverageUtils.GetProjectPath());
        }

        public void SetCoverageHistoryLocation(string folderPath)
        {
            if (CoverageUtils.IsValidFolder(folderPath))
            {
                m_CodeCoverageHistoryPath = folderPath;
                CoveragePreferences.instance.SetStringForPaths("HistoryPath", m_CodeCoverageHistoryPath);
                UpdateCoverageSettings();
            }
        }

        public void SetDefaultCoverageHistoryLocation()
        {
            SetCoverageHistoryLocation(CoverageUtils.GetProjectPath());
        }
    }
}
