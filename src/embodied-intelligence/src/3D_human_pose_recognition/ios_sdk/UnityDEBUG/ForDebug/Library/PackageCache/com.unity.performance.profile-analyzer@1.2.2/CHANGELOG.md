# Changelog
All notable changes to this package will be documented in this file.

The format is based on [Keep a Changelog](http://keepachangelog.com/en/1.0.0/)
and this project adheres to [Semantic Versioning](http://semver.org/spec/v2.0.0.html).

## [1.2.2] - 2024-01-30

### Fixed

* Fixed exception thrown when removing marker.

## [1.2.1] - 2024-01-02

### Changed

* Updated What's New page

## [1.2.0] - 2024-01-02

### Added 

* Added support for removing vsync time, so we can see actual CPU duration over multiple frames. A drop down has been added to 'remove' a marker from the analysis and has entries for "FPS Wait", "Present Wait" and "Custom" where you can select any marker from the table using the right click context menu to "Remove Marker".
* Added optional standard deviation (SD) column into marker table (on single view) for sorting data based on variation of marker timings in over the frames.
* Added export of the comparison table as CSV.

### Changed

* Improved profile analysis performance.
* Updated minimal suppourted version to Unity 2020.3.

### Fixed

* Fixed bug when depth filter and marker vsync removal both applied.
* Fixed commas in CSV export file.

## [1.1.1] - 2021-10-04

### Fixed
* Fixed pulling frame data from the Profiler to exclude first and last frames if their main thread profiler data is incomplete, so that they don't skewer the analysis results [(case 1359686)](https://issuetracker.unity3d.com/product/unity/issues/guid/1359686/).
* Fixed IndexOutOfRangeException thrown when using Profiler or selecting Profiler frames in Profile Analyzer. [(1366931)](https://issuetracker.unity3d.com/product/unity/issues/guid/1366931/)

## [1.1.0] - 2021-07-23

### Fixed

* Fixed x axis display on frame time graph when capture doesn't match Unity Profiler contents.
* Fixed selected marker name to be updated even when Profiler fails to sync selection.

## [1.1.0-pre.2] - 2021-04-16

### Changed

* Ensured forward compilation compatibility of ProfilerWindow API usage.

### Fixed

* Fixed Frame View Dropdown not showing time in Microseconds when Microseconds are set as display unit type.
* Fixed ArgurmentOutOfRange exception on 'Clear Selection' usage in Frame Control.
* Improved loading and analysis progressbar progression to be monotonously incremental and reflect actual stage correctly.
* Fixed the marked frame time overlay so that it works on loaded data (case 1327888).
* Fixed keyboard frame controls so that they do not play an error sound (case 1327931).
* Fixed RangeSettings.IsEqual so that it doesn't throw an exception when there is a different dataView.selectedIndices.

## [1.1.0-pre.1] - 2021-02-22

### Changed

* Used the new `EditorWindow.docked` API on 2020.1 and later to replace reflection usage.  
* Made the **Thread Selection** window a utility window.
* Added a progress bar, which shows data loading progress.
* **All** no longer collapses in the thread view because it was superfluous and held no discernible purpose.
* Added information to the frame summary tooltip to show the selected ranges.
* Selecting the **Frame**, **Thread**, and **Marker Summary** title labels now also toggles their foldout.
* Timings in tooltips now display up to seven digits with a unit type change if below 0.01. This ensures the value is readable and correct.
* Frame Time Graph selection modifiers (Next, Previous, Grow, Shrink etc.) now work when multiple regions are selected.
* Added new (hidden) columns to the comparison tab for **Percentage Diff** for each of the diff types
* The **Depth Slice** drop-down is searchable and scrollable for Unity versions 2019.1 or newer.
* Improved tooltips on the top 10 marker duration time.

### Fixed

* The Thread Summary and Marker Summary views now correctly clip their contents when scrolling on 2018.4 and below.
* Marker detail bars are no longer drawn on top of the list's headers when scrolling.
* Fixed an issue with saving when there is no data loaded, which previously caused a null reference exception.
* **Add to** and **Remove from** name filter is no longer case sensitive.
* Optimized frame selection with **Show Filtered Threads** option enabled and **All Threads** turned on.
* Fixed an issue where the **Upper Quartile of frame time** option did not fit in the Graph scale dropdown in **Thread Summary**.
* Improved the error message when the Profile Analyzer fails to load a .data file.
* Fixed an issue where changing the depth slice and right-clicking on a marker caused an error in 2018.4 and earlier.
* Fixed the automatic increase of an unsaved file counter in Single view.
* Improved the Marker Column mode so that it is separate for Single and Compare view.
* The tooltip of the frame graph's scale control is now clearer and matches the documentation.
* Fixed an issue where a frame of '0' was incorrectly shown in the **Marker Summary** when the marker had a duration of 0ms.
* Fixed an issue with **Selection Synchronization** between Profiler Window and Profile Analyzer in Unity 2021.1 or newer.
* Added a log message when the loading or analysis fails due to domain reload.
* Fixed an issue with the table context menu, where it was not disabled during data processing.
* Fixed an issue where exporting a marker table that contained markers with commas or quotation marks to the .csv format would break the format.

## [1.0.3] - 2020-07-31

### Fixes

* Fixed warnings in code so package will not break Unity projects that have warnings as errors enabled.

## [1.0.2] - 2020-06-30

### Fixes

* Fixed issue where second profiler instance could appear after entering play mode when profile analyzer is open
* Improved performance of pulling data from Unity profiler
* Pull Data button now enabled when profiler is recording. This will stop the recording for the duration of the action of pulling of data.
* Fixed Median sorting in Comparison mode when value missing from one data set, Missing values always sorted before 0 values.

## [1.0.1] - 2020-06-16

### Fixes

* Fixed "median marker time (in currently selected frames)" in tooltip for 'Top 10 markers'. 
* Fixed profile analyzer 5.6 support
* Fixed minor visual artefact when no marker selection 
* Fixed issue where frame time graph tooltips were not always appearing when hovering
* Marker Summary - Count Values are now correctly sorted in descending order
* Hiding selected marker in comparison view if lacking 2 valid data sets
* Corrected bucketing of histogram data for counts. Display was fractionally incorrect
* Fixed sorting of frames (by time/count) to be more stable by providing a secondary sort by frame index
* Marker table export now orders by descending median time (to match the default UI sort option).

## [1.0.0] - 2020-06-02

### Changes
* Export window updates. Includes tooltips, better on screen positioning. 
  * If opened and then new data set loaded. New data set now correctly exported.
* Frame Time graph now shows border when selected
  * Keys 1 and 2 select the first or second frame time graph to take keyboard focus
* Frame Time graph now shows highlighted when all frames selected, un highlighted when no frames selected.

### Fixes
* Improved histogram display when 0 range.
* Widened frame index buttons when frame value large
* 'total number of markers' value in Compare tab now corrected to use unfiltered count
* Improved error messages when jumpping to frame when profiler data doesn't match profile analyzer data

## [0.7.0-preview.4] - 2020-05-29

### Changes
* Thread selection window 
  * Now only applys thread selection changes when clicking 'Apply'. Closing the window no longer applies the changes automatically.
  * Now contains buttons to Reset to previous thread selection, Select just "Main Thread" or "Common" set selection (Main, Render and Jobs).
  * Apply button is now greyed out while analysis is running.
  * Group column enabled and used to split off group from thread name, default to by group first
  * Re sorted when selection changed (so states sort correctly when sorting by state)
  * Sort order preserved when buttons pressed and re-opening
  * Fixed sort order for threads (10 after 9 rather than between 1 and 2)
* Marker sorting
  * Marker table column filtering now preserved when clicking Analyse or Compare or tab switching.
  * Compare table bars to now sort by bar size (=delta value) rather than the left/right value.
  * Marker table bars now start sorted by descending size which is more common usage.
* Added depths columns into the comparison table and a 'Depth' option in the Marker Columns dropdown
* Added thread count into thread summary area
* Added threads column into marker tables (threads the marker occurs on)
* Thread summary now contains thread count, unique count per data set and selection count
* Frame time area now allows adding to the selection by holding CTRL (or COMMAND on Mac)
* Frame range value now includes tooltip to give more detail about the selection
* Improved text string for depth filter status on the top marker display

### Fixes
* Detected and ignored invalid frame markers (duration < 0)
* Fixed/Removed warnings on loading data in compare view when selection active.
* Disabled "Analyze" button while analysis is already running
* Fixed Marker Summary 'Top' dropdown selection text width
* Fixed thread selection window sort order
* Fixed tooltip on 'count' bars to be scalar value (not a time unit)
* Clamped selected region shown when zoomed in on frame time graph
* Frame Control value no longer overlaps with drop-down list when Units are set to Microseconds
* Fixed thread count text when analysis completes after swapping single/compare tabs during analysis
* Fixed Frame View's tooltip "total time" when rapidly changing frames 
* Fixed overlapping text when selecting single frame when zoomed in
* Histogram frame count now lists "1 frame" when single frame rather than "1 frames"
* Mode: text in top bar now same size as rest of text.
* Clear Selection in marker right click context menu now only shown if a selection has been made
* Cut/Paste now supported in the include/exclude marker name filter
* Frame Time graph frame selection grow/shrink now keeps current selection in paired graph
* Fixed frame start/end display for "Select Frames that contain this marker (within whole data set)"
* Corrected bucketing of histogram data. Display was fractionally incorrect
* Histogram now shows non zero height bar if an item in the bucket
* Fixed right click "Remove from Included Filters" when using (quoted) markers with spaces in names
* Fixed data auto-load from Single to Compare tab when capturing after entering playmode
* Fixed auto right calculation to use most common difference
* Fixed auto right display to show + or - and not both at the same time
* Fixed infinite analysis loop when auto right calculation was clamping to max depth in the other data set
* Auto right now clamps to min/max depth rather than reverting to 'all'
* Fixed thread count bug when using comparison mode and loading data on top of existing data
* Selected marker refocused in marker list when table is regenerated (e.g. when selecting all frames containing the marker).
* Column by which the Marker Details is sorted by is now maintained  when entering Play Mode
* Updated frame time graph context menu to make hotkeys more consistent with out Unity UI layouts
* Percent symbol no longer cut off in Mean frame contribution when the value is 100% 
* Frame time graph now scales up when zooming when comparing data with different frame amounts
* Changing 'Auto Right' tick box no longer causes re-analysis if depth settings unchanged
* Most tooltips now show values without rounding, so its more obvious when low values give non zero deltas. 
* Fixed individual max tooltip
* Disabled 'Pull data' button when Unity profiler is still capturing data (as the pull would not complete).
* Fixed right frame index in exported comparison CSV file.
* Frame selection no longer changed when moving the cursor in the Filter text box with arrow keys
* Help text now continues to be shown after entering play mode, while no data set loaded/pulled
* Fixed frame index button heights when alternative (Verdana) font used.
* Fixed cache styles to be updated when changing theme (to fix text colour in filters area)
* Single click (without drag) now always single frame rather than the group of frames in the pixel wide area.
* Split frame range into start and end rows to display more information when values large 
* Marker Summary comparison times are now limited to 5 digits to prevent text clipping off

### Enhancements
* Optimised filter processing for more responsive scrolling when a large marker filter list is supplied.
* Optimised thread name summary display

## [0.6.0-preview.1] - 2020-02-04
* Fixed a crash with the thread selection API in Unity 2020.1 
* Fixed marker and thread scroll bars
* Added extra documentation for public API elements

## [0.5.0-preview.2] - 2019-09-19
* Minor documentation update to fix the changelog formatting

## [0.5.0-preview.1] - 2019-09-18

### Features 
* Added self time option to display 'exclusive' time of markers excluding time in children.
* Added ability to filter to a parent marker to reduce the marker list to a part of the tree.
* Added option to filter the column list to set groups (including custom).
* Added column for total marker time over the frame
* Added copy to clipboard on table entries
* Added export option for marker table 

### Enhancements
* Improved Top N Markers graph to make it clearer this is for the median frames of each data set.
* Added thread count display (next to marker count).
* Added frame index to more tooltips to improve clarity of the data values (marker table, frame and marker summary).
* Added additional visual bars for total and count diffs. Added abs count column.
* Improved performance of adding to include/exclude filter via the right click menu by only refreshing the table (and no longer rerunning full analysis)
* Improved performance for scrolling by caching strings for profile table and comparison table.
* Added unaccounted time into the Top N Markers graph when total is less than the median frame time
* Added grow/shrink selection hot keys and menu options
* Added tooltip info for frame time duration for selection range

### Fixes
* Fixed issue with combined marker count when data sets have some unique markers.
* Fixed bars less than 1 pixel wide to clamp to min width of 1.
* Fixed help text for new editor skin in 2019.3
* Fixed bug with calculation of the auto right depth offset (see with 2017.4/2018.4 comparisons)
* Improved the frame offset times in the frame time and comparison frame time exports
* Fixed bug with missing first frame of data / frame offset incorrect when reloading .pdata

## [0.4.0-preview.5] - 2019-04-02

* Updated package.json file to indicate this package is valid for all unity versions

## [0.4.0-preview.4] - 2019-04-02

* Fixed issue in 2017.4 with unsupported analytics API and a GUI style.

## [0.4.0-preview.3] - 2019-04-01

* First public release of Profile Analyzer. 

## [0.1.0-preview] - 2018-12-07

* This is the first beta release of Profile Analyzer

The profile analyzer tool augments the standard Unity Profiler. It provides multi frame analysis of the profiling data.
