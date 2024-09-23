# Changelog
All notable changes to the Code Coverage package will be documented in this file.

The format is based on [Keep a Changelog](http://keepachangelog.com/en/1.0.0/)
and this project adheres to [Semantic Versioning](http://semver.org/spec/v2.0.0.html).

## [1.2.2] - 2022-11-18

### Fixes
- Temporary fix for the [Test Framework](https://docs.unity3d.com/Packages/com.unity.test-framework@latest/index.html) 1.3 issue where the *RunFinished* callback is not called when running from the command line and there is a domain reload (case [DSTR-692](https://issuetracker.unity3d.com/issues/registered-callbacks-dont-work-after-domain-reload)).

### Improvements
- Reduced the number of logs for the default *Verbosity:Info*.
- Added *Uncoverable lines* definition in [How to interpret the results](https://docs.unity3d.com/Packages/com.unity.testtools.codecoverage@1.2/manual/HowToInterpretResults.html#summary) page in the documentation.
- Updated documentation to match version 1.2.2.

## [1.2.1] - 2022-10-27

### Fixes
- Fixed compatibility with [Test Framework](https://docs.unity3d.com/Packages/com.unity.test-framework@latest/index.html) package version 1.3.

### Improvements
- A single file summary version of the report is now generated in Json format, in addition to the XML and Markdown formats.
- Added a warning when an invalid coverage option is passed in *-coverageOptions* in [batchmode](https://docs.unity3d.com/Packages/com.unity.testtools.codecoverage@1.2/manual/CoverageBatchmode.html).

## [1.2.0] - 2022-08-01

### Fixes
- Ensure assemblies are removed from the Included Assemblies field if they no longer exist (case [1318668](https://issuetracker.unity3d.com/issues/code-coverage-the-included-assemblies-field-shows-assemblies-that-no-longer-exist)).
- Ensure hidden sequence points are ignored (case [1372305](https://issuetracker.unity3d.com/issues/class-which-derives-from-methodbase-causes-incorrect-sequence-points-to-be-generated-by-coverage-api)).

### Changes
- Updated Report Generator to version 5.0.4.
- Updated the UI of the [Code Coverage window](https://docs.unity3d.com/Packages/com.unity.testtools.codecoverage@1.2/manual/CodeCoverageWindow.html) moving the action buttons into a toolbar at the top.
- Renamed *assemblyFilters* aliases in [batchmode](https://docs.unity3d.com/Packages/com.unity.testtools.codecoverage@1.2/manual/CoverageBatchmode.html); `<user>` was renamed to `<assets>` and `<project>` was renamed to `<all>`.
- Replaced `pathStrippingPatterns` with `pathReplacePatterns` in [batchmode](https://docs.unity3d.com/Packages/com.unity.testtools.codecoverage@1.2/manual/CoverageBatchmode.html). The `pathReplacePatterns` option allows stripping and replacing specific sections from the paths that are stored in the coverage results xml files.

See the [Upgrade guide](https://docs.unity3d.com/Packages/com.unity.testtools.codecoverage@1.2/manual/upgrade-guide.html) if upgrading to Code Coverage package version 1.2.

### Improvements
- The size of the coverage result files and the Code Coverage session duration have been optimized. At the start of the session a coverage xml result file is generated which includes all the lines but with zero coverage. The following coverage xml result files that are generated within a Code Coverage session include only the coverage data of the visited lines.
- Added Help IconButton in the toolbar in the [Code Coverage window](https://docs.unity3d.com/Packages/com.unity.testtools.codecoverage@1.2/manual/CodeCoverageWindow.html).
- Updated the mechanic for opening the containing folder, change the location or reset to the default location for *Results Location* and *Report History Location*.
- Refactored the [Code Coverage window](https://docs.unity3d.com/Packages/com.unity.testtools.codecoverage@1.2/manual/CodeCoverageWindow.html) UI to include a new *Report Options* section and removing the word 'Generate' from the options.
- Introduced new selection buttons under the *Included Assemblies* dropdown in the [Code Coverage window](https://docs.unity3d.com/Packages/com.unity.testtools.codecoverage@1.2/manual/CodeCoverageWindow.html); use the *All* button to select all the assemblies in the project. Use the *Assets* button to select only the assemblies under the `Assets` folder. Use the *Packages* button to select only the Packages' assemblies. If searching, the buttons will apply only to the assemblies visible in the list.
- Updated [What's new](https://docs.unity3d.com/Packages/com.unity.testtools.codecoverage@1.2/manual/whats-new.html) and [Upgrade guide](https://docs.unity3d.com/Packages/com.unity.testtools.codecoverage@1.2/manual/upgrade-guide.html) pages in the documentation.
- Added [Using relative paths in path filters](https://docs.unity3d.com/Packages/com.unity.testtools.codecoverage@1.2/manual/CoverageBatchmode.html#using-relative-paths-in-path-filters) section in documentation.
- Updated the editor and console logs; added information about the assembly and path filters, improved coverage session logs.
- Improved the progress bars for `Writing coverage results` and `Generating the report`.
- Added an icon for the [Code Coverage window](https://docs.unity3d.com/Packages/com.unity.testtools.codecoverage@1.2/manual/CodeCoverageWindow.html).
- Updated documentation to match version 1.2.0.

### Features
- Added `Pause Recording` and `Resume Recording` buttons in the toolbar in the [Code Coverage window](https://docs.unity3d.com/Packages/com.unity.testtools.codecoverage@1.2/manual/CodeCoverageWindow.html).
- Added `Log Verbosity Level` setting in the [Code Coverage window](https://docs.unity3d.com/Packages/com.unity.testtools.codecoverage@1.2/manual/CodeCoverageWindow.html) which allows setting the verbosity level for the editor and console logs.
- Added `Additional Reports` option in the [Code Coverage window](https://docs.unity3d.com/Packages/com.unity.testtools.codecoverage@1.2/manual/CodeCoverageWindow.html) which if checked [SonarQube](https://docs.sonarqube.org/latest/analysis/generic-test), [Cobertura](https://cobertura.github.io/cobertura) and [LCOV](https://github.com/linux-test-project/lcov) reports will be generated. Added `generateAdditionalReports` in *-coverageOptions* for [batchmode](https://docs.unity3d.com/Packages/com.unity.testtools.codecoverage@1.2/manual/CoverageBatchmode.html).
- Added `Test Runner References` report option in the [Code Coverage window](https://docs.unity3d.com/Packages/com.unity.testtools.codecoverage@1.2/manual/CodeCoverageWindow.html) which if checked includes test references to the generated coverage results and enables the [Coverage by test methods](https://docs.unity3d.com/Packages/com.unity.testtools.codecoverage@1.2/manual/HowToInterpretResults.html#coverage-by-test-methods) section in the HTML report, allowing you to see how each test contributes to the overall coverage. Added `generateTestReferences` in *-coverageOptions* for [batchmode](https://docs.unity3d.com/Packages/com.unity.testtools.codecoverage@1.2/manual/CoverageBatchmode.html).
- Added `Auto Open Report` option in the [Code Coverage window](https://docs.unity3d.com/Packages/com.unity.testtools.codecoverage@1.2/manual/CodeCoverageWindow.html) which if checked the coverage report will open automatically after it has been generated.
- Added `pathFiltersFromFile` in *-coverageOptions* for [batchmode](https://docs.unity3d.com/Packages/com.unity.testtools.codecoverage@1.2/manual/CoverageBatchmode.html) which allows specifying an external file which contains a list of path filters. When this file contains relative paths, the `sourcePaths` option can be used to specify the source directories.
- Added `dontClear` in *-coverageOptions* for [batchmode](https://docs.unity3d.com/Packages/com.unity.testtools.codecoverage@1.2/manual/CoverageBatchmode.html) which allows coverage results to be accumulated after every code coverage session. If not passed the results are cleared before a new session. For more information see [Generate combined report from EditMode and PlayMode tests](https://docs.unity3d.com/Packages/com.unity.testtools.codecoverage@1.2/manual/CoverageBatchmode.html#generate-combined-report-from-editmode-and-playmode-tests).
- When the `pathFilters` option or the `pathFiltersFromFile` option in *-coverageOptions* contains relative paths, the `sourcePaths` option can be used to specify the source directories.

## [1.1.1] - 2021-12-17

### Fixes
- Ensure assemblies are removed from the Included Assemblies field if they no longer exist (case [1318668](https://issuetracker.unity3d.com/issues/code-coverage-the-included-assemblies-field-shows-assemblies-that-no-longer-exist))

### Changes
- Updated Report Generator to version 4.8.13

### Improvements
- Added Help IconButton in the [Code Coverage window](https://docs.unity3d.com/Packages/com.unity.testtools.codecoverage@1.1/manual/CodeCoverageWindow.html) for Unity versions 2021.2.2f1 and above
- Added [What's new](https://docs.unity3d.com/Packages/com.unity.testtools.codecoverage@1.1/manual/whats-new.html) and [Upgrade guide](https://docs.unity3d.com/Packages/com.unity.testtools.codecoverage@1.1/manual/upgrade-guide.html) pages in the documentation 
- Updated documentation to match version 1.1.1


## [1.1.0] - 2021-06-09

### Fixes
- Ensure Results and History folders are created if they do not exist (case [1334551](https://issuetracker.unity3d.com/issues/code-coverage-results-slash-history-location-path-is-reset-to-default-if-set-path-no-longer-exists))
- Added support for [ExcludeFromCoverage/ExcludeFromCodeCoverage](https://docs.unity3d.com/Packages/com.unity.testtools.codecoverage@1.0/manual/UsingCodeCoverage.html#excluding-code-from-code-coverage) for lambda expressions and yield statements (case [1338636](https://issuetracker.unity3d.com/issues/code-coverage-excludefromcoverage-attribute-doesnt-exclude-lambda-expressions-and-yield-statements-from-coverage))
- Added support for [ExcludeFromCodeCoverage](https://docs.unity3d.com/Packages/com.unity.testtools.codecoverage@1.0/manual/UsingCodeCoverage.html#excluding-code-from-code-coverage) for getter/setter properties (case [1338665](https://issuetracker.unity3d.com/issues/code-coverage-get-and-set-accessors-are-still-marked-as-coverable-when-property-has-excludefromcodecoverage-attribute))
- *-coverageOptions* are only parsed when running from the command line ([feedback](https://forum.unity.com/threads/code-coverage-slowing-editor-on-enter-playmode-and-assembly-reload.1121566))

### Changes
- Updated Report Generator to version 4.8.9

### Improvements
- Implemented changes to support [Test Framework](https://docs.unity3d.com/Packages/com.unity.test-framework@latest/index.html) package version 1.2
- Logs while the Report is generated are output per message rather than at the end of the generation
- Do not log burst warning when `-burst-disable-compilation` is passed in the command line
- Added [Ignoring tests for Code Coverage](https://docs.unity3d.com/Packages/com.unity.testtools.codecoverage@1.1/manual/CoverageBatchmode.html#ignoring-tests-for-code-coverage) section in documentation
- Updated the [Generate combined report from separate projects](https://docs.unity3d.com/Packages/com.unity.testtools.codecoverage@1.1/manual/CoverageBatchmode.html#generate-combined-report-from-separate-projects) section in documentation
- Updated documentation to match version 1.1.0

### Features
- Added `Code Coverage session Events` [API](https://docs.unity3d.com/Packages/com.unity.testtools.codecoverage@1.1/api/UnityEditor.TestTools.CodeCoverage.Events.html) to subscribe to events invoked during a Code Coverage session
- Added `useProjectSettings` in *-coverageOptions* for [batchmode](https://docs.unity3d.com/Packages/com.unity.testtools.codecoverage@1.1/manual/CoverageBatchmode.html) which allows using the settings specified in `ProjectSettings/Settings.json`
- Added `pathStrippingPatterns` in *-coverageOptions* for [batchmode](https://docs.unity3d.com/Packages/com.unity.testtools.codecoverage@1.1/manual/CoverageBatchmode.html) which allows stripping specific sections from the paths that are stored in the coverage results xml files
- Added `sourcePaths` in *-coverageOptions* for [batchmode](https://docs.unity3d.com/Packages/com.unity.testtools.codecoverage@1.1/manual/CoverageBatchmode.html) which allows specifying the source directories which contain the corresponding source code

## [1.0.0] - 2021-03-09

### Fixes
- Fixed issues with Path Filtering (cases [1318896](https://issuetracker.unity3d.com/issues/code-coverage-typing-comma-into-the-included-or-excluded-paths-list-will-start-adding-row-for-each-letter-you-type-afterwards), [1318897](https://issuetracker.unity3d.com/issues/code-coverage-clearing-last-included-paths-row-immediatly-jumps-to-the-first-excluded-paths-row-and-starts-editing-it))

### Improvements
- Selection/focus is cleared when mouse is clicked outside of the individual settings' areas
- Added [Quickstart guide](https://docs.unity3d.com/Packages/com.unity.testtools.codecoverage@1.0/manual/Quickstart.html) in documentation
- Renamed the *Code Coverage Workshop* sample to *Code Coverage Tutorial*
- Updated documentation and workshop to match version 1.0.0

**Note:** In Unity 2019 and 2020 you can enable Code Coverage in [General Preferences](https://docs.unity3d.com/Manual/Preferences.html). This was removed in Unity 2021; the user interface for managing Code Coverage is now entirely inside the Code Coverage package.

## [1.0.0-pre.4] - 2021-02-26

### Fixes
- Fixed assembly version validation error due to internal libraries included in the ReportGeneratorMerged.dll (case [1312121](https://issuetracker.unity3d.com/issues/code-coverage-reportgeneratormerged-cant-be-loaded-due-to-assembly-version-validation-failure))

### Changes
- Added *Enable Code Coverage* checkbox under Settings in [Code Coverage window](https://docs.unity3d.com/Packages/com.unity.testtools.codecoverage@1.0/manual/CodeCoverageWindow.html).<br/>**Note:** In Unity 2019 and 2020 you can enable Code Coverage in [General Preferences](https://docs.unity3d.com/Manual/Preferences.html). This was removed in Unity 2021; the user interface for managing Code Coverage is now entirely inside the Code Coverage package.
- The settings and options passed in the command line override/disable the settings in the Code Coverage window and relevant warnings display to indicate this
- Updated Report Generator to version 4.8.5
- Updated documentation and workshop to match version 1.0.0-pre.4

### Improvements
- Added `verbosity` in *-coverageOptions* for [batchmode](https://docs.unity3d.com/Packages/com.unity.testtools.codecoverage@1.0/manual/CoverageBatchmode.html)
- Added *Generate combined report from separate projects* section in documentation, under [Using Code Coverage in batchmode](https://docs.unity3d.com/Packages/com.unity.testtools.codecoverage@1.0/manual/CoverageBatchmode.html#generate-combined-report-from-separate-projects)

## [1.0.0-pre.3] - 2021-01-21

### Fixes
- Updated Include Platforms to Editor only in the ReportGeneratorMerged.dll settings. Fixes an Android build error introduced in 1.0.0-pre.2 (case 1306557)

## [1.0.0-pre.2] - 2021-01-13

### Fixes
- Fixed multiple reports generated in batchmode when passing `generateHtmlReport` in *-coverageOptions* without passing `-runTests`

### Changes
- All project assemblies are included when there are included paths specified in *pathFilters* but no included assemblies in *assemblyFilters*, when running in batchmode
- Updated Report Generator to version 4.8.4
- Updated documentation to match version 1.0.0-pre.2

### Improvements
- Introduced new *assemblyFilters* aliases in [batchmode](https://docs.unity3d.com/Packages/com.unity.testtools.codecoverage@1.0/manual/CoverageBatchmode.html), used for referencing a group of assemblies to include or exclude. These are `<user>`, `<project>` and `<packages>`

## [1.0.0-pre.1] - 2020-11-12
- *1.0.0-pre.1* matches *0.4.0-preview*

## [0.4.0-preview] - 2020-11-11

### Changes
- Moved Code Coverage window under *Window* > *Analysis*
- *Included Assemblies* now use a single dropdown instead of an editable text field which acted as a dropdown
- Added CommandLineParser and removed dependency to internals in Test Framework
- Removed the old *EditorPref* workflow from CoveragePreferences
- Moved *Generate History* outside of *Generate HTML Report*. It is now disabled only if both *Generate HTML Report* and *Generate Badges* are not selected
- Updated Report Generator to version 4.7.1
- Updated documentation and workshop to match version 0.4.0-preview

### Improvements
- Implemented `{ProjectPath}` alias in `Settings.json`
- Added a console warning when *Burst Compilation* is enabled and an info HelpBox with a button to disable
- Added Analytics to help improve the user experience
- Disabled *Generate from Last* button when there are no assemblies selected
- Display an info HelpBox when there are no assemblies selected
- Paths are now stored with forward slashes on Windows
- Added warning about Code Coverage not being supported currently when running PlayMode tests in standalone player
- Refactored code; in Utils, Filtering, ResultWriter, Window and API classes
- Added *CoverageWindow* and *Filtering* folders

### Features
- Added *Included Paths* and *Excluded Paths* as ReorderableLists in the [Code Coverage window](https://docs.unity3d.com/Packages/com.unity.testtools.codecoverage@0.4/manual/CodeCoverageWindow.html)
- Added [support](https://docs.unity3d.com/Packages/com.unity.testtools.codecoverage@0.4/manual/UsingCodeCoverage.html#excluding-code-from-code-coverage) for `ExcludeFromCoverage` and `ExcludeFromCodeCoverage` attributes
- Added `CodeCoverage.VerbosityLevel` [API](https://docs.unity3d.com/Packages/com.unity.testtools.codecoverage@0.4/api/UnityEditor.TestTools.CodeCoverage.CodeCoverage.html) to set the verbosity level used in editor and console logs

## [0.3.1-preview] - 2020-08-03

### Fixes
- Fixed issue where CRAP calculation was incorrect when generic methods were parsed
- Corrected Six Labors License copyright in Third Party Notices

### Changes
- If `assemblyFilters` is not specified in *-coverageOptions* in batchmode, include only the assemblies found under the *Assets* folder
- Updated Report Generator to version 4.6.4

## [0.3.0-preview] - 2020-05-20

### Fixes
- Make sure *operator* and *anonymous function* names are generated correctly

### Changes
- Added *Generate Additional Metrics* setting in the [Code Coverage window](https://docs.unity3d.com/Packages/com.unity.testtools.codecoverage@0.3/manual/CodeCoverageWindow.html) and removed *Cyclomatic Complexity* (it is now included in Additional Metrics)
- Updated Report Generator to version 4.5.8
- Updated documentation to match version 0.3.0-preview

### Improvements
- Added [Code Coverage Workshop](https://docs.unity3d.com/Packages/com.unity.testtools.codecoverage@0.3/manual/CodeCoverageWorkshop.html) sample project
- Using the [Settings Manager](https://docs.unity3d.com/Manual/com.unity.settings-manager.html) package to handle the serialization of project settings
- Added an info HelpBox when *Code Optimization* is set to Release mode with a button to switch to Debug mode
- Execute *Stop Recording* on the update loop, instead of the OnGUI (removes an *EndLayoutGroup* error)
- Refactored code; in OpenCoverReporter class (to reduce Cyclomatic Complexity), in CodeCoverageWindow class and others

### Features
- Added *History Location* and *Generate History* settings in the [Code Coverage window](https://docs.unity3d.com/Packages/com.unity.testtools.codecoverage@0.3/manual/CodeCoverageWindow.html)
- Added `coverageHistoryPath` and `generateHtmlReportHistory` in *-coverageOptions* for [batchmode](https://docs.unity3d.com/Packages/com.unity.testtools.codecoverage@0.3/manual/UsingCodeCoverage.html#using-code-coverage-in-batchmode)
- Added `generateAdditionalMetrics` in *-coverageOptions* for [batchmode](https://docs.unity3d.com/Packages/com.unity.testtools.codecoverage@0.3/manual/UsingCodeCoverage.html#using-code-coverage-in-batchmode) and removed *enableCyclomaticComplexity* (it is now included in Additional Metrics)
- Added *Crap Score* in Additional Metrics. See [How to interpret the results](https://docs.unity3d.com/Packages/com.unity.testtools.codecoverage@0.3/manual/HowToInterpretResults.html).

## [0.2.3-preview] - 2020-02-18

### Fixes
- *Included Assemblies* dropdown is now resizing to the longest assembly name ([1215600](https://issuetracker.unity3d.com/issues/there-is-no-way-to-view-the-full-name-of-an-assembly-when-selecting-it-in-a-small-code-coverage-window))
- When closing (selecting outside of) the *Included Assemblies* dropdown, input is not accidentally propagated to the Code Coverage window

### Improvements
- If more than one instance of the *-coverageOptions* command-line argument is specified, they will now be merged into a single instance
- If more than one instance of the *-coverageResultsPath* command-line argument is specified, only the first instance will be accepted
- Added *Generate combined report from EditMode and PlayMode tests* section in documentation, under [Using Code Coverage in batchmode](https://docs.unity3d.com/Packages/com.unity.testtools.codecoverage@0.2/manual/UsingCodeCoverage.html#using-code-coverage-in-batchmode)

## [0.2.2-preview] - 2019-12-11

### Fixes
- Fixed unassigned *CodeCoverageWindow.m_IncludeWarnings* warning in 2019.3

### Changes
- The default *Included Assemblies* are now only the assemblies found under the project's *Assets* folder, instead of all project assemblies

### Improvements
- After the report is generated, the file viewer window highlights the `index.htm` file, if *Generate HTML Report* is selected

## [0.2.1-preview] - 2019-12-04

### Improvements
- Improved globbing for `pathFilters` and `assemblyFilters`
- Added new sections and examples in documentation
- Added confirmation dialogs when selecting *Clear Data* and *Clear History* buttons
- Added warning and button to switch to debug mode, when using Code Optimization in release mode in 2020.1 and above

### Features
- Added `pathFilters` in *-coverageOptions* for [batchmode](https://docs.unity3d.com/Packages/com.unity.testtools.codecoverage@0.2/manual/UsingCodeCoverage.html#using-code-coverage-in-batchmode)

## [0.2.0-preview] - 2019-11-13

### Fixes
- Make sure recording coverage results are saved in the *Recording* folder, and starting a new recording session does not affect existing non-recording data

### Changes
- Updated Report Generator to version 4.3.6
- Split documentation into separate pages

### Improvements
- Updated UX design of the Code Coverage window
- Make sure settings and Record button are disabled when coverage is running
- Make sure coverage window is disabled before unity is restarted when *Enabling Code Coverage* in Preferences
- Only parse xml files with the correct filename format when generating the report
- Implemented try/catch when deleting files/folders when selecting *Clear Data* or *Clear History*
- Handle nested classes, nested generic classes and anonymous functions

### Features
- Exposed `CodeCoverage.StartRecording()`, `CodeCoverage.StopRecording()`, `CodeCoverage.PauseRecording()` and `CodeCoverage.UnpauseRecording()` [API](https://docs.unity3d.com/Packages/com.unity.testtools.codecoverage@0.2/api/UnityEditor.TestTools.CodeCoverage.CodeCoverage.html)

## [0.1.0-preview.3] - 2019-09-27

### Improvements
- Passing `-coverageOptions generateHtmlReport` on the command line now creates a report if `-runTests` is not passed

## [0.1.0-preview.2] - 2019-09-23

### Changes
- Updated Report Generator to version 4.2.20

### Improvements
- Added support for correct naming of c# operators
- Added support for correct naming of constructors
- Added declaring type name as a prefix
- Added support for return types in method names

### Features
- Added [Coverage Recording](https://docs.unity3d.com/Packages/com.unity.testtools.codecoverage@0.2/manual/CoverageRecording.html) feature

## [0.1.0-preview.0] - 2019-03-18

### This is the first release of *Code Coverage Package*
