using UnityEngine;
using System;
using System.Collections.Generic;
using UnityEngine.Profiling;

namespace UnityEditor.Performance.ProfileAnalyzer
{
    [Serializable]
    internal class FrameTimeGraphGlobalSettings
    {
        public bool showThreads = false;
        public bool showSelectedMarker = true;
        public bool showFrameLines = true;
        public bool showFrameLineText = true;
        public bool showOrderedByFrameDuration = false;
    }

    internal class FrameTimeGraph
    {
        static FrameTimeGraphGlobalSettings m_GlobalSettings = new FrameTimeGraphGlobalSettings();

        static public void SetGlobalSettings(FrameTimeGraphGlobalSettings globalSettings)
        {
            m_GlobalSettings = globalSettings;
        }

        public struct Data
        {
            public readonly float ms;
            public readonly int frameOffset;

            public Data(float _ms, int _index)
            {
                ms = _ms;
                frameOffset = _index;
            }
        };

        public delegate void SetRange(List<int> selected, int clickCount, FrameTimeGraph.State inputStatus);
        public delegate void SetActive(bool active);

        public enum State
        {
            None,
            Dragging,
            DragComplete
        };

        enum DragDirection
        {
            Start,
            Forward,
            Backward,
            None
        };

        enum AxisMode
        {
            One60HzFrame,
            Two60HzFrames,
            Four60HzFrames,
            Max,
            Custom
        };

        Draw2D m_2D;
        int m_DragBeginFirstOffset;
        int m_DragBeginLastOffset;

        bool m_Dragging;
        int m_DragFirstOffset;
        int m_DragLastOffset;
        bool m_Moving;
        int m_MoveHandleOffset;
        bool m_SingleControlAction;

        int m_ClickCount;
        double m_LastClickTime;
        bool m_MouseReleased;

        bool m_Zoomed;
        int m_ZoomStartOffset;
        int m_ZoomEndOffset;

        Color m_ColorBarBackground;
        Color m_ColorBarBackgroundSelected;
        Color m_ColorBar;
        Color m_ColorBarOutOfRange;
        Color m_ColorBarSelected;
        Color m_ColorBarThreads;
        Color m_ColorBarThreadsOutOfRange;
        Color m_ColorBarThreadsSelected;
        Color m_ColorBarMarker;
        Color m_ColorBarMarkerOutOfRange;
        Color m_ColorBarMarkerSelected;
        Color m_ColorGridLine;

        FrameTimeGraph m_PairedWithFrameTimeGraph;

        internal static class Styles
        {
            public static readonly GUIContent menuItemClearSelection = new GUIContent("Clear Selection");
            public static readonly GUIContent menuItemSelectAll = new GUIContent("Select All");
            public static readonly GUIContent menuItemInvertSelection = new GUIContent("Invert Selection");
            public static readonly GUIContent menuItemZoomSelection = new GUIContent("Zoom Selection");
            public static readonly GUIContent menuItemZoomAll = new GUIContent("Zoom All");
            public static readonly GUIContent menuItemSelectMin = new GUIContent("Select Shortest Frame");
            public static readonly GUIContent menuItemSelectMax = new GUIContent("Select Longest Frame");
            public static readonly GUIContent menuItemSelectMedian = new GUIContent("Select Median Frame");

            public static readonly GUIContent menuItemSelectPrevious = new GUIContent("Move selection left _LEFT");
            public static readonly GUIContent menuItemSelectNext = new GUIContent("Move selection right _RIGHT");

            public static readonly GUIContent menuItemSelectGrow = new GUIContent("Grow selection  _=");
            public static readonly GUIContent menuItemSelectShrink = new GUIContent("Shrink selection  _-");
            public static readonly GUIContent menuItemSelectGrowLeft = new GUIContent("Grow selection left  _<");
            public static readonly GUIContent menuItemSelectGrowRight = new GUIContent("Grow selection right  _>");
            public static readonly GUIContent menuItemSelectShrinkLeft = new GUIContent("Shrink selection left  _&<");
            public static readonly GUIContent menuItemSelectShrinkRight = new GUIContent("Shrink selection right  _&>");

            public static readonly GUIContent menuItemSelectGrowFast = new GUIContent("Grow selection (fast)  _#=");
            public static readonly GUIContent menuItemSelectShrinkFast = new GUIContent("Shrink selection (fast)  _#-");
            public static readonly GUIContent menuItemSelectGrowLeftFast = new GUIContent("Grow selection left (fast)  _#<");
            public static readonly GUIContent menuItemSelectGrowRightFast = new GUIContent("Grow selection right (fast) _#>");
            public static readonly GUIContent menuItemSelectShrinkLeftFast = new GUIContent("Shrink selection left (fast) _#&<");
            public static readonly GUIContent menuItemSelectShrinkRightFast = new GUIContent("Shrink selection right (fast) _#&>");

            public static readonly GUIContent menuItemShowSelectedMarker = new GUIContent("Show Selected Marker");
            public static readonly GUIContent menuItemShowThreads = new GUIContent("Show Filtered Threads");
            //    public static readonly GUIContent menuItemDetailedMode = new GUIContent("Detailed mode");
            public static readonly GUIContent menuItemShowFrameLines = new GUIContent("Show Frame Lines");
            public static readonly GUIContent menuItemShowFrameLineText = new GUIContent("Show Frame Line Text");
            public static readonly GUIContent menuItemShowOrderedByFrameDuration = new GUIContent("Order by Frame Duration");
        }

        const int kXAxisWidth = 80;
        const int kYAxisDetailThreshold = 40;
        const int kOverrunHeight = 3;

        static AxisMode s_YAxisMode;
        static float m_YAxisMs;

        bool m_IsOrderedByFrameDuration;

        List<Data> m_Values = new List<Data> {};
        List<int> m_LastSelectedFrameOffsets = new List<int> {};
        int[] m_FrameOffsetToDataOffsetMapping = new int[] {};
        SetRange m_SetRange;
        SetActive m_SetActive;

        List<int> m_CurrentSelection = new List<int>();
        int m_CurrentSelectionFirstDataOffset;
        int m_CurrentSelectionLastDataOffset;

        int m_GraphId;
        int m_ControlID;
        static int s_LastSelectedGraphId = -1;
        static int s_CurrentSelectedGraphId = -1;

        bool m_Enabled;

        struct BarData
        {
            public float x;
            public float y;
            public float w;
            public float h;

            public int startDataOffset;
            public int endDataOffset;
            public float yMin;
            public float yMax;

            public BarData(float _x, float _y, float _w, float _h, int _startDataOffset, int _endDataOffset, float _yMin, float _yMax)
            {
                x = _x;
                y = _y;
                w = _w;
                h = _h;
                startDataOffset = _startDataOffset;
                endDataOffset = _endDataOffset;
                yMin = _yMin;
                yMax = _yMax;
            }
        }

        List<BarData> m_Bars = new List<BarData>();

        DisplayUnits m_Units;
        Rect m_LastRect;
        int m_MaxFrames;

        string DisplayUnits()
        {
            return m_Units.Postfix();
        }

        string ToDisplayUnits(float ms, bool showUnits = false, int limitToNDigits = 5)
        {
            return m_Units.ToString(ms, showUnits, limitToNDigits);
        }

        public void SetUnits(Units units)
        {
            m_Units = new DisplayUnits(units);
        }

        public void Reset()
        {
            m_Zoomed = false;
            m_Dragging = false;
            ClearDragSelection();

            m_Moving = false;
            m_ClickCount = 0;
            m_MouseReleased = false;
        }

        void Init()
        {
            Reset();

            m_PairedWithFrameTimeGraph = null;
            m_YAxisMs = 100f;
            s_YAxisMode = AxisMode.Max;
            m_IsOrderedByFrameDuration = false;

            m_Enabled = true;

            m_LastRect = new Rect(0, 0, 0, 0);
            m_MaxFrames = -1;
        }

        public void MakeGraphActive(bool activate)
        {
            if (activate)
            {
                if (s_CurrentSelectedGraphId != m_GraphId)
                {
                    s_LastSelectedGraphId = m_GraphId;
                    s_CurrentSelectedGraphId = m_GraphId;

                    // Make sure we are not still selecting another graph
                    GUIUtility.hotControl = 0;

                    m_SetActive(true);
                }

                if (GUI.GetNameOfFocusedControl() != "FrameTimeGraph")
                {
                    // Take focus away from any other control
                    // Doesn't really matter what the name is here
                    GUI.FocusControl("FrameTimeGraph");
                }
            }
            else
            {
                if (s_CurrentSelectedGraphId == m_GraphId)
                {
                    s_CurrentSelectedGraphId = -1;

                    // Remember this was the active control
                    // Before this point one of the inner labels would have been active
                    // GUIUtility.hotControl = m_ControlID;

                    m_SetActive(false);
                }
            }
        }

        public bool IsGraphActive()
        {
            if (s_CurrentSelectedGraphId == m_GraphId)
                return true;

            if (s_LastSelectedGraphId == m_GraphId && GUIUtility.hotControl == m_ControlID)
                return true;

            return false;
        }

        public FrameTimeGraph(int graphID, Draw2D draw2D, Units units, Color background, Color backgroundSelected, Color barColor, Color barSelected, Color barMarker, Color barMarkerSelected, Color barThreads, Color barThreadsSelected, Color colorGridlines)
        {
            m_GraphId = graphID;
            m_ControlID = 0;

            m_2D = draw2D;
            SetUnits(units);
            Init();

            float ratio = 0.75f;
            m_ColorBarBackground = background;
            m_ColorBarBackgroundSelected = backgroundSelected;

            m_ColorBar = barColor;
            m_ColorBarOutOfRange = new Color(barColor.r * ratio, barColor.g * ratio, barColor.b * ratio);
            m_ColorBarSelected = barSelected;

            m_ColorBarMarker = barMarker;
            m_ColorBarMarkerOutOfRange = new Color(barMarker.r * ratio, barMarker.g * ratio, barMarker.b * ratio);
            m_ColorBarMarkerSelected = barMarkerSelected;

            m_ColorBarThreads = barThreads;
            m_ColorBarThreadsOutOfRange = new Color(barThreads.r * ratio, barThreads.g * ratio, barThreads.b * ratio);
            m_ColorBarThreadsSelected = barThreadsSelected;

            m_ColorGridLine = colorGridlines;
        }

        int ClampToRange(int value, int min, int max)
        {
            if (value < min)
                value = min;
            if (value > max)
                value = max;

            return value;
        }

        int GetDataOffsetForXUnclamped(int xPosition, int width, int totalDataSize)
        {
            int visibleDataSize;
            if (m_Zoomed)
                visibleDataSize = (m_ZoomEndOffset - m_ZoomStartOffset) + 1;
            else
                visibleDataSize = totalDataSize;

            int dataOffset = (int)(xPosition * visibleDataSize / width);

            if (m_Zoomed)
                dataOffset += m_ZoomStartOffset;

            return dataOffset;
        }

        int GetDataOffsetForX(int xPosition, int width, int totalDataSize)
        {
            //xPosition = ClampToRange(xPosition, 0, width-1);
            int dataOffset = GetDataOffsetForXUnclamped(xPosition, width, totalDataSize);
            return ClampToRange(dataOffset, 0, totalDataSize - 1);
        }

        int GetXForDataOffset(int dataOffset, int width, int totalDataSize)
        {
            //frameOffset = ClampToRange(frameOffset, 0, frames-1);

            int visibleDataSize;
            if (m_Zoomed)
            {
                dataOffset = ClampToRange(dataOffset, m_ZoomStartOffset, m_ZoomEndOffset + 1);
                dataOffset -= m_ZoomStartOffset;
                visibleDataSize = (m_ZoomEndOffset - m_ZoomStartOffset) + 1;
            }
            else
                visibleDataSize = totalDataSize;

            int x = (int)(dataOffset * width / visibleDataSize);

            x = ClampToRange(x, 0, width - 1);
            return x;
        }

        void SetDragMovement(int startOffset, int endOffset, int currentSelectionFirstDataOffset, int currentSelectionLastDataOffset)
        {
            // Maintain length but clamp to range
            int frames = m_Values.Count;

            int currentSelectionRange = currentSelectionLastDataOffset - currentSelectionFirstDataOffset;
            endOffset = startOffset + currentSelectionRange;

            startOffset = ClampToRange(startOffset, 0, frames - (currentSelectionRange + 1));
            endOffset = ClampToRange(endOffset, 0, frames - 1);

            SetDragSelection(startOffset, endOffset);

            if (m_PairedWithFrameTimeGraph != null && !m_SingleControlAction)
            {
                m_PairedWithFrameTimeGraph.SetDragSelection(m_DragFirstOffset, m_DragLastOffset);
            }
        }

        void SetDragSelection(int startOffset, int endOffset, DragDirection dragDirection)
        {
            // No need to clamp these as input is clamped.
            switch (dragDirection)
            {
                case DragDirection.Forward:
                    SetDragSelection(m_DragBeginFirstOffset, endOffset);
                    break;

                case DragDirection.Backward:
                    SetDragSelection(startOffset, m_DragBeginLastOffset);
                    break;

                case DragDirection.Start:
                    SetDragSelection(startOffset, endOffset);

                    // Record first selected bar range
                    m_DragBeginFirstOffset = m_DragFirstOffset;
                    m_DragBeginLastOffset = m_DragLastOffset;
                    break;
            }

            if (m_PairedWithFrameTimeGraph != null && !m_SingleControlAction)
            {
                m_PairedWithFrameTimeGraph.SetDragSelection(m_DragFirstOffset, m_DragLastOffset);
            }
        }

        public void SetDragSelection(int startOffset, int endOffset)
        {
            m_DragFirstOffset = startOffset;
            m_DragLastOffset = endOffset;
        }

        public void ClearDragSelection()
        {
            m_DragFirstOffset = -1;
            m_DragLastOffset = -1;
        }

        public bool HasDragRegion()
        {
            return (m_DragFirstOffset != -1);
        }

        public void GetSelectedRange(List<int> frameOffsets, out int firstDataOffset, out int lastDataOffset, out int firstFrameOffset, out int lastFrameOffset)
        {
            int frames = m_Values != null ? m_Values.Count : 0;

            firstDataOffset = 0;
            lastDataOffset = frames - 1;
            firstFrameOffset = 0;
            lastFrameOffset = frames - 1;

            if (m_FrameOffsetToDataOffsetMapping.Length > 0)
            {
                // By default data is ordered by index so first/last will be the selected visible range

                if (frameOffsets.Count >= 1)
                {
                    firstFrameOffset = frameOffsets[0];
                    lastFrameOffset = firstFrameOffset;

                    firstDataOffset = GetDataOffset(firstFrameOffset);
                    lastDataOffset = firstDataOffset;
                }
                if (frameOffsets.Count >= 2)
                {
                    lastFrameOffset = frameOffsets[frameOffsets.Count - 1];

                    lastDataOffset = GetDataOffset(lastFrameOffset);
                }

                if (m_GlobalSettings.showOrderedByFrameDuration)
                {
                    // Need to find the selected items with lowest and highest ms values
                    if (frameOffsets.Count > 0)
                    {
                        int dataOffset = GetDataOffset(firstFrameOffset);

                        firstDataOffset = dataOffset;
                        lastDataOffset = dataOffset;
                        float firstDataMS = m_Values[dataOffset].ms;
                        float lastDataMS = m_Values[dataOffset].ms;

                        foreach (int frameOffset in frameOffsets)
                        {
                            dataOffset = GetDataOffset(frameOffset);

                            float ms = m_Values[dataOffset].ms;
                            if (ms <= firstDataMS && dataOffset < firstDataOffset)
                            {
                                firstDataMS = ms;
                                firstDataOffset = dataOffset;
                            }
                            if (ms >= lastDataMS && dataOffset > lastDataOffset)
                            {
                                lastDataMS = ms;
                                lastDataOffset = dataOffset;
                            }
                        }
                    }
                }
            }
        }

        public bool IsMultiSelectControlHeld()
        {
#if UNITY_EDITOR_OSX
            return Event.current.command;
#else
            return Event.current.control;
#endif
        }

        public State ProcessInput()
        {
            if (!IsEnabled())
                return State.None;

            if (m_Values == null)
                return State.None;

            if (m_LastRect.width == 0 || m_MaxFrames < 0)
                return State.None;

            Rect rect = m_LastRect;
            int maxFrames = m_MaxFrames;

            int dataLength = m_Values.Count;
            if (dataLength <= 0)
                return State.None;

            if (m_IsOrderedByFrameDuration != m_GlobalSettings.showOrderedByFrameDuration)
            {
                // Reorder if necessary
                SetData(m_Values);
            }

            int currentSelectionFirstDataOffset;
            int currentSelectionLastDataOffset;
            int currentSelectionFirstFrameOffset;
            int currentSelectionLastFrameOffset;

            GetSelectedRange(m_LastSelectedFrameOffsets, out currentSelectionFirstDataOffset, out currentSelectionLastDataOffset, out currentSelectionFirstFrameOffset, out currentSelectionLastFrameOffset);

            m_CurrentSelection.Clear();
            m_CurrentSelection.AddRange(m_LastSelectedFrameOffsets);
            m_CurrentSelectionFirstDataOffset = currentSelectionFirstDataOffset;
            m_CurrentSelectionLastDataOffset = currentSelectionLastDataOffset;

            if (Event.current.isKey && Event.current.type == EventType.KeyDown && !m_Dragging && !m_MouseReleased)
            {
                if (IsGraphActive())
                {
                    int step = Event.current.shift ? 10 : 1;
                    var eventUsed = false;
                    switch (Event.current.keyCode)
                    {
                        case KeyCode.LeftArrow:
                            SelectPrevious(step);
                            eventUsed = true;
                            break;
                        case KeyCode.RightArrow:
                            SelectNext(step);
                            eventUsed = true;
                            break;
                        case KeyCode.Less:
                        case KeyCode.Comma:
                            if (Event.current.alt)
                                SelectShrinkLeft(step);
                            else
                                SelectGrowLeft(step);
                            eventUsed = true;
                            break;
                        case KeyCode.Greater:
                        case KeyCode.Period:
                            if (Event.current.alt)
                                SelectShrinkRight(step);
                            else
                                SelectGrowRight(step);
                            eventUsed = true;
                            break;
                        case KeyCode.Plus:
                        case KeyCode.Equals:
                        case KeyCode.KeypadPlus:
                            if (Event.current.alt)
                                SelectShrink(step);
                            else
                                SelectGrow(step);
                            eventUsed = true;
                            break;
                        case KeyCode.Underscore:
                        case KeyCode.Minus:
                        case KeyCode.KeypadMinus:
                            if (Event.current.alt)
                                SelectGrow(step);
                            else
                                SelectShrink(step);
                            eventUsed = true;
                            break;
                    }

                    if (eventUsed)
                        Event.current.Use();

                }
            }

            float doubleClickTimeout = 0.25f;
            if (m_MouseReleased)
            {
                if ((EditorApplication.timeSinceStartup - m_LastClickTime) > doubleClickTimeout)
                {
                    // By this point we will know if its a single or double click
                    bool append = IsMultiSelectControlHeld();
                    CallSetRange(m_DragFirstOffset, m_DragLastOffset, m_ClickCount, m_SingleControlAction, FrameTimeGraph.State.DragComplete, append);

                    ClearDragSelection();
                    if (m_PairedWithFrameTimeGraph != null && !m_SingleControlAction)
                        m_PairedWithFrameTimeGraph.ClearDragSelection();

                    m_MouseReleased = false;
                }
            }

            int width = (int)rect.width;
            int height = (int)rect.height;
            float xStart = rect.xMin;
            if (height > kYAxisDetailThreshold)
            {
                float h = GUI.skin.label.lineHeight;
                xStart += kXAxisWidth;
                width -= kXAxisWidth;
            }
            if (maxFrames > 0)
            {
                if (!m_Zoomed)
                    width = width * dataLength / maxFrames;
            }

            // Process input
            Event e = Event.current;
            if (e.isMouse)
            {
                if (m_Dragging)
                {
                    if (e.type == EventType.MouseUp)
                    {
                        m_Dragging = false;
                        m_Moving = false;

                        // Delay the action as we are checking for double click
                        m_MouseReleased = true;
                        return State.Dragging;
                    }
                }

                int x = (int)(e.mousePosition.x - xStart);

                int dataOffset =  GetDataOffsetForXUnclamped(x, width, dataLength);
                if (m_Moving)
                    dataOffset -= m_MoveHandleOffset;
                dataOffset = ClampToRange(dataOffset, 0, dataLength - 1);
                int frameOffsetBeforeNext = Math.Max(dataOffset,  GetDataOffsetForX(x + 1, width, dataLength) - 1);

                if (m_Dragging)
                {
                    if (e.button == 0)
                    {
                        // Still dragging (doesn't have to be within the y bounds)
                        if (m_Moving)
                        {
                            // Forward drag from start point
                            SetDragMovement(dataOffset, frameOffsetBeforeNext, currentSelectionFirstDataOffset, currentSelectionLastDataOffset);
                        }
                        else
                        {
                            DragDirection dragDirection = (dataOffset < m_DragBeginFirstOffset) ? DragDirection.Backward : DragDirection.Forward;
                            SetDragSelection(dataOffset, frameOffsetBeforeNext, dragDirection);
                        }

                        CallSetRange(m_DragFirstOffset, m_DragLastOffset, m_ClickCount, m_SingleControlAction, FrameTimeGraph.State.Dragging);
                        return State.Dragging;
                    }
                }
                else
                {
                    if (e.mousePosition.x >= rect.x && e.mousePosition.x <= rect.xMax &&
                        e.mousePosition.y >= rect.y && e.mousePosition.y <= rect.yMax)
                    {
                        if (e.mousePosition.x >= xStart && e.mousePosition.x <= (xStart + width) &&
                            e.mousePosition.y >= rect.y && e.mousePosition.y < rect.yMax)
                        {
                            MakeGraphActive(true);

                            if (e.type == EventType.MouseDown && e.button == 0)
                            {
                                // Drag start (must be within the bounds of the control)
                                // Might be single or double click
                                m_LastClickTime = EditorApplication.timeSinceStartup;
                                m_ClickCount = e.clickCount;

                                m_Dragging = true;
                                m_Moving = false;

                                if (currentSelectionFirstDataOffset != 0 || currentSelectionLastDataOffset != dataLength - 1)
                                {
                                    // Selection is valid
                                    if (e.shift && dataOffset >= currentSelectionFirstDataOffset && frameOffsetBeforeNext <= currentSelectionLastDataOffset)
                                    {
                                        // Moving if shift held and we are inside the current selection range
                                        m_Moving = true;
                                    }
                                }

                                if (m_PairedWithFrameTimeGraph != null)
                                    m_SingleControlAction = e.alt;  // Record if we are acting only on this control rather than the paired one too
                                else
                                    m_SingleControlAction = true;

                                if (m_Moving)
                                {
                                    m_MoveHandleOffset = dataOffset - currentSelectionFirstDataOffset;

                                    SetDragMovement(currentSelectionFirstDataOffset, currentSelectionLastDataOffset, currentSelectionFirstDataOffset, currentSelectionLastDataOffset);
                                }
                                else
                                {
                                    //SetDragSelection(dataOffset, frameOffsetBeforeNext, DragDirection.Start);

                                    // Select just 1 frame
                                    SetDragSelection(dataOffset, dataOffset, DragDirection.Start);
                                }
                                CallSetRange(m_DragFirstOffset, m_DragLastOffset, m_ClickCount, m_SingleControlAction, FrameTimeGraph.State.Dragging);
                                return State.Dragging;
                            }
                        }
                    }
                    else
                    {
                        // Left this graph area
                        MakeGraphActive(false);
                    }
                }
            }

            if (m_MouseReleased)
            {
                // Not finished drag fully yet
                CallSetRange(m_DragFirstOffset, m_DragLastOffset, m_ClickCount, m_SingleControlAction, FrameTimeGraph.State.Dragging);
                return State.Dragging;
            }

            return State.None;
        }

        public float GetDataRange()
        {
            if (m_Values == null)
                return 0f;

            int frames = m_Values.Count;

            float min = 0f;
            float max = 0f;
            for (int frameOffset = 0; frameOffset < frames; frameOffset++)
            {
                float ms = m_Values[frameOffset].ms;
                if (ms > max)
                    max = ms;
            }
            float hRange = max - min;

            return hRange;
        }

        public void PairWith(FrameTimeGraph otherFrameTimeGraph)
        {
            if (m_PairedWithFrameTimeGraph != null)
            {
                // Clear existing pairing
                m_PairedWithFrameTimeGraph.m_PairedWithFrameTimeGraph = null;
            }

            m_PairedWithFrameTimeGraph = otherFrameTimeGraph;
            if (otherFrameTimeGraph != null)
                otherFrameTimeGraph.m_PairedWithFrameTimeGraph = this;
        }

        public FrameTimeGraph GetPairedWith()
        {
            return m_PairedWithFrameTimeGraph;
        }

        public float GetYAxisRange(float yMax)
        {
            switch (s_YAxisMode)
            {
                case AxisMode.One60HzFrame:
                    return 1000f / 60f;
                case AxisMode.Two60HzFrames:
                    return 2000f / 60f;
                case AxisMode.Four60HzFrames:
                    return 4000f / 60f;
                case AxisMode.Max:
                    return yMax;
                case AxisMode.Custom:
                    return m_YAxisMs;
            }

            return yMax;
        }

        public void SetData(List<Data> values)
        {
            if (values == null)
                return;

            m_Values = values;

            if (m_GlobalSettings.showOrderedByFrameDuration)
                m_Values.Sort((a, b) => { return a.ms.CompareTo(b.ms); });
            else
                m_Values.Sort((a, b) => { return a.frameOffset.CompareTo(b.frameOffset); });

            m_FrameOffsetToDataOffsetMapping = new int[m_Values.Count];
            for (int dataOffset = 0; dataOffset < m_Values.Count; dataOffset++)
                m_FrameOffsetToDataOffsetMapping[m_Values[dataOffset].frameOffset] = dataOffset;

            m_CurrentSelection.Clear();
            for (int frameIndex = 0; frameIndex < m_Values.Count; frameIndex++)
            {
                m_CurrentSelection.Add(frameIndex);
            }
            m_CurrentSelectionFirstDataOffset = 0;
            m_CurrentSelectionLastDataOffset = m_Values.Count - 1;

            m_IsOrderedByFrameDuration = m_GlobalSettings.showOrderedByFrameDuration;
        }

        int GetDataOffset(int frameOffset)
        {
            if (frameOffset < 0 || frameOffset >= m_FrameOffsetToDataOffsetMapping.Length)
            {
                Debug.Log(string.Format("{0} out of range of frame offset to data offset mapping {1}", frameOffset, m_FrameOffsetToDataOffsetMapping.Length));
                return 0;
            }

            return m_FrameOffsetToDataOffsetMapping[frameOffset];
        }

        public void ClearData()
        {
            m_Values = null;
        }

        public bool HasData()
        {
            if (m_Values == null)
                return false;
            if (m_Values.Count == 0)
                return false;

            return true;
        }

        public void SetActiveCallback(SetActive setActive)
        {
            m_SetActive = setActive;
        }

        public void SetRangeCallback(SetRange setRange)
        {
            m_SetRange = setRange;
        }

        void CallSetRange(int startDataOffset, int endDataOffset, int clickCount, bool singleControlAction, FrameTimeGraph.State inputStatus, bool append = false, bool effectPaired = true)
        {
            if (m_SetRange == null)
                return;

            if (startDataOffset < 0 && endDataOffset < 0)
            {
                // Clear
                m_SetRange(new List<int>(), clickCount, inputStatus);
                return;
            }

            startDataOffset = Math.Max(0, startDataOffset);
            endDataOffset = Math.Min(endDataOffset, m_Values.Count - 1);

            List<int> selected = new List<int>();
            if (append && m_LastSelectedFrameOffsets.Count != m_Values.Count)
            {
                foreach (int frameOffset in m_LastSelectedFrameOffsets)
                {
                    int dataOffset = m_FrameOffsetToDataOffsetMapping[frameOffset];
                    if (dataOffset >= 0 && dataOffset < m_Values.Count)
                    {
                        selected.Add(frameOffset);
                    }
                }
            }
            for (int dataOffset = startDataOffset; dataOffset <= endDataOffset; dataOffset++)
            {
                if (dataOffset >= 0 && dataOffset < m_Values.Count)
                {
                    int frameOffset = m_Values[dataOffset].frameOffset;
                    if (append == false || !selected.Contains(frameOffset))
                    {
                        selected.Add(frameOffset);
                    }
                }
            }
            // Sort selection in frame index order so start is lowest and end is highest
            selected.Sort();

            if (selected.Count == 0)
                return;

            m_SetRange(selected, clickCount, inputStatus);

            if (m_PairedWithFrameTimeGraph != null && m_PairedWithFrameTimeGraph.m_Values.Count > 1 && effectPaired && !singleControlAction)
            {
                // Update selection on the other frame time graph
                int mainMaxFrame = m_Values.Count - 1;
                int otherMaxFrame = m_PairedWithFrameTimeGraph.m_Values.Count - 1;

                int startOffset = startDataOffset;
                int endOffset = endDataOffset;

                if (startOffset > otherMaxFrame)
                {
                    if (append)
                    {
                        // Nothing more to do
                        return;
                    }

                    // Select all, if the main selection is outsize the range of the other
                    startOffset = 0;
                    endOffset = otherMaxFrame;
                }
                else
                {
                    if (startOffset == 0 && endOffset == mainMaxFrame)
                    {
                        // If clearing main selection then clear the other section fully too
                        endOffset = otherMaxFrame;
                    }

                    startOffset = ClampToRange(startOffset, 0, otherMaxFrame);
                    endOffset = ClampToRange(endOffset, 0, otherMaxFrame);
                }

                m_PairedWithFrameTimeGraph.CallSetRange(startOffset, endOffset, clickCount, singleControlAction, inputStatus, append, false);
            }
        }

        bool HasNoSelection()
        {
            if (m_Values == null)
                return false;

            int frames = m_Values.Count;

            return ((m_CurrentSelectionFirstDataOffset < 0 || m_CurrentSelectionLastDataOffset >= frames) &&
                (m_CurrentSelectionLastDataOffset < 0 || m_CurrentSelectionLastDataOffset >= frames));
        }

        bool HasSelectedAll()
        {
            if (m_Values == null)
                return false;

            int frames = m_Values.Count;

            return (m_CurrentSelectionFirstDataOffset == 0 && m_CurrentSelectionLastDataOffset == (frames - 1));
        }

        bool HasSubsetSelected()
        {
            if (m_Values == null)
                return false;

            return !(HasSelectedAll() || HasNoSelection());
        }

        void RegenerateBars(float x, float y, float width, float height, float yRange)
        {
            int frames = m_Values.Count;
            if (frames <= 0)
                return;

            m_Bars.Clear();

            int nextDataOffset =  GetDataOffsetForX(0, (int)width, frames);
            for (int barX = 0; barX < width; barX++)
            {
                int startDataOffset = nextDataOffset;
                nextDataOffset =  GetDataOffsetForX(barX + 1, (int)width, frames);
                int endDataOffset = Math.Max(startDataOffset, nextDataOffset - 1);

                float min = m_Values[startDataOffset].ms;
                float max = min;
                for (int dataOffset = startDataOffset + 1; dataOffset <= endDataOffset; dataOffset++)
                {
                    float ms = m_Values[dataOffset].ms;
                    if (ms < min)
                        min = ms;
                    if (ms > max)
                        max = ms;
                }
                float maxClamped = Math.Min(max, yRange);
                float h = height * maxClamped / yRange;

                m_Bars.Add(new BarData(x + barX, y, 1, h, startDataOffset, endDataOffset, min, max));
            }
        }

        public void SetEnabled(bool enabled)
        {
            m_Enabled = enabled;
        }

        public bool IsEnabled()
        {
            return m_Enabled;
        }

        float GetTotalSelectionTime(List<int> selectedFrameOffsets)
        {
            float totalMs = 0;
            for (int i = 0; i < selectedFrameOffsets.Count; i++)
            {
                int frameOffset = selectedFrameOffsets[i];
                int dataOffset = m_FrameOffsetToDataOffsetMapping[frameOffset];
                totalMs += m_Values[dataOffset].ms;
            }

            return totalMs;
        }

        float GetTotalSelectionTime(int firstOffset, int lastOffset)
        {
            float totalMs = 0;
            for (int frameOffset = firstOffset; frameOffset <= lastOffset; frameOffset++)
            {
                if (frameOffset < m_FrameOffsetToDataOffsetMapping.Length)
                {
                    int dataOffset = m_FrameOffsetToDataOffsetMapping[frameOffset];
                    totalMs += m_Values[dataOffset].ms;
                }
            }

            return totalMs;
        }

        void ShowFrameLines(float x, float y, float yRange, float width, float height)
        {
            float msSegment = 1000f / 60f;
            int lines = (int)(yRange / msSegment);
            int step = 1;
            for (int line = 1; line <= lines; line += step, step *= 2)
            {
                float ms = line * msSegment;
                float h = height * ms / yRange;
                m_2D.DrawLine(x, y + h, x + width - 1, y + h, m_ColorGridLine);
            }
        }

        bool InSelectedRegion(int startDataOffset, int endDataOffset, int selectedFirstOffset, int selectedLastOffset, Dictionary<int, int> frameOffsetToSelectionIndex, bool subsetSelected)
        {
            bool inSelectionRegion = false;

            bool showCurrentSelection = false;
            if (HasDragRegion())
            {
                if (endDataOffset >= selectedFirstOffset && startDataOffset <= selectedLastOffset)
                {
                    inSelectionRegion = true;
                }
                if (IsMultiSelectControlHeld() && m_LastSelectedFrameOffsets.Count != m_Values.Count)
                {
                    // Show current selection too
                    showCurrentSelection = true;
                }
            }
            else
            {
                showCurrentSelection = true;
            }

            if (showCurrentSelection)
            {
                //if (subsetSelected)
                {
                    for (int dataOffset = startDataOffset; dataOffset <= endDataOffset; dataOffset++)
                    {
                        int frameOffset = m_Values[dataOffset].frameOffset;
                        if (frameOffsetToSelectionIndex.ContainsKey(frameOffset))
                        {
                            inSelectionRegion = true;
                            break;
                        }
                    }
                }
            }

            return inSelectionRegion;
        }

        public void Draw(Rect rect, ProfileAnalysis analysis, List<int> selectedFrameOffsets, float yMax, int offsetToDisplayMapping, int offsetToIndexMapping, string selectedMarkerName, int maxFrames = 0, ProfileAnalysis fullAnalysis = null)
        {
            Profiler.BeginSample("FrameTimeGraph.Draw");

            // Must be outside repaint to make sure next controls work correctly (specifically marker name filter)
            int controlID = GUIUtility.GetControlID(FocusType.Keyboard);

            if (Event.current.type == EventType.Repaint)
            {
                m_LastRect = rect;

                // Control id would change during non repaint phase if tooltips displayed if we update this outside the repaint
                m_ControlID = controlID;
            }

            m_MaxFrames = maxFrames;

            if (m_Values == null)
                return;

            m_LastSelectedFrameOffsets = selectedFrameOffsets;

            int totalDataSize = m_Values.Count;
            if (totalDataSize <= 0)
                return;

            if (m_IsOrderedByFrameDuration != m_GlobalSettings.showOrderedByFrameDuration)
            {
                // Reorder if necessary
                SetData(m_Values);
            }

            // Get start and end selection span
            int currentSelectionFirstDataOffset;
            int currentSelectionLastDataOffset;
            int currentSelectionFirstFrameOffset;
            int currentSelectionLastFrameOffset;

            GetSelectedRange(selectedFrameOffsets, out currentSelectionFirstDataOffset, out currentSelectionLastDataOffset, out currentSelectionFirstFrameOffset, out currentSelectionLastFrameOffset);


            // Create mapping from offset to selection for faster selection detection
            Dictionary<int, int> frameOffsetToSelectionIndex = new Dictionary<int, int>();
            for (int i = 0; i < selectedFrameOffsets.Count; i++)
            {
                int frameOffset = selectedFrameOffsets[i];
                frameOffsetToSelectionIndex[frameOffset] = i;
            }

            Event current = Event.current;

            int selectedFirstOffset;
            int selectedLastOffset;
            int selectedCount;
            bool subsetSelected = false;
            if (HasDragRegion())
            {
                selectedFirstOffset = m_DragFirstOffset;
                selectedLastOffset = m_DragLastOffset;

                if (selectedFirstOffset > m_Values.Count - 1)
                {
                    // Selection off the end
                    selectedFirstOffset = m_Values.Count;
                    selectedLastOffset = m_Values.Count;
                    selectedCount = 0;
                }
                else
                {
                    selectedFirstOffset = ClampToRange(selectedFirstOffset, 0, m_Values.Count - 1);
                    selectedLastOffset = ClampToRange(selectedLastOffset, 0, m_Values.Count - 1);

                    selectedCount = 1 + (selectedLastOffset - selectedFirstOffset);
                    subsetSelected = true;
                }
            }
            else
            {
                selectedFirstOffset = currentSelectionFirstDataOffset;
                selectedLastOffset = currentSelectionLastDataOffset;
                selectedCount = selectedFrameOffsets.Count;
                subsetSelected = (selectedCount > 0 && selectedCount != totalDataSize);
            }

            // Draw frames and selection
            float width = rect.width;
            float height = rect.height;

            bool showAxis = false;
            float xStart = 0f;
            float yStart = 0f;
            if (height > kYAxisDetailThreshold)
            {
                showAxis = true;

                float h = GUI.skin.label.lineHeight;
                xStart += kXAxisWidth;
                width -= kXAxisWidth;

                yStart += h;
                height -= h;
            }
            if (maxFrames > 0)
            {
                if (!m_Zoomed)
                    width = width * totalDataSize / maxFrames;
            }

            // Start / End
            int startOffset = m_Zoomed ? m_ZoomStartOffset : 0;
            int endOffset = m_Zoomed ? m_ZoomEndOffset : totalDataSize - 1;

            // Get try index values
            int startIndex = offsetToDisplayMapping + startOffset;
            int endIndex = offsetToDisplayMapping + endOffset;
            int selectedFirstIndex = offsetToDisplayMapping + selectedFirstOffset;
            int selectedLastIndex = offsetToDisplayMapping + selectedLastOffset;

            string detailsString = "";

            if (!showAxis)
            {
                string frameRangeString;
                if (startIndex == endIndex)
                    frameRangeString = string.Format("Total Range {0}", startIndex);
                else
                    frameRangeString = string.Format("Total Range {0} - {1} [{2}]", startIndex, endIndex, 1 + (endIndex - startIndex));

                // Selection range
                string selectedTooltip = "";
                if (subsetSelected)
                {
                    if (selectedFirstIndex == selectedLastIndex)
                        selectedTooltip = string.Format("\nSelected {0}\n", selectedFirstIndex);
                    else
                        selectedTooltip = string.Format("\nSelected {0} - {1} [{2}]", selectedFirstIndex, selectedLastIndex, selectedCount);
                }

                detailsString = string.Format("\n\n{0}{1}", frameRangeString, selectedTooltip);
            }

            float yRange = GetYAxisRange(yMax);

            bool lastEnabled = GUI.enabled;
            bool enabled = IsEnabled();
            GUI.enabled = enabled;

            if (m_2D.DrawStart(rect, Draw2D.Origin.BottomLeft))
            {
                float totalMs;
                if (HasDragRegion())
                {
                    totalMs = GetTotalSelectionTime(selectedFirstOffset, selectedLastOffset);
                }
                else
                {
                    totalMs = GetTotalSelectionTime(selectedFrameOffsets);
                }

                string timeForSelectedFrames = ToDisplayUnits(totalMs, true, 0);
                string timeForSelectedFramesClamped = ToDisplayUnits(totalMs, true, 1);
                string selectionAreaString = string.Format("\n\nTotal time for {0} selected frames\n{1} ({2})", selectedCount, timeForSelectedFrames, timeForSelectedFramesClamped);

                Color selectedControl = GUI.skin.settings.selectionColor;

                if (IsGraphActive())
                {
                    m_2D.DrawBox(xStart, yStart, width, height, selectedControl);
                }

                //xStart -= 1f;
                yStart += 1f;
                width -= 1f;
                height -= 1f;

                m_2D.DrawFilledBox(xStart, yStart, width, height, m_ColorBarBackground);

                RegenerateBars(xStart, yStart, width, height, yRange);

                foreach (BarData bar in m_Bars)
                {
                    bool inSelectionRegion = InSelectedRegion(bar.startDataOffset, bar.endDataOffset, selectedFirstOffset, selectedLastOffset, frameOffsetToSelectionIndex, subsetSelected);
                    if (inSelectionRegion)
                    {
                        m_2D.DrawFilledBox(bar.x, bar.y, bar.w, height, m_ColorBarBackgroundSelected);
                    }
                }

                if (m_GlobalSettings.showFrameLines)
                {
                    ShowFrameLines(xStart, yStart, yRange, width, height);
                }

                ProfileAnalysis analysisData = analysis;
                bool full = false;
                if (fullAnalysis != null)
                {
                    analysisData = fullAnalysis;
                    full = true;
                }

                MarkerData selectedMarker = (m_GlobalSettings.showSelectedMarker && analysisData != null) ? analysisData.GetMarkerByName(selectedMarkerName) : null;
                foreach (BarData bar in m_Bars)
                {
                    bool inSelectionRegion = InSelectedRegion(bar.startDataOffset, bar.endDataOffset, selectedFirstOffset, selectedLastOffset, frameOffsetToSelectionIndex, subsetSelected);
                    if (inSelectionRegion)
                    {
                        m_2D.DrawFilledBox(bar.x, bar.y, bar.w, bar.h, m_ColorBarSelected);
                    }
                    else
                    {
                        m_2D.DrawFilledBox(bar.x, bar.y, bar.w, bar.h, m_ColorBar);
                    }

                    // Show where its been clamped
                    if (bar.yMax > yRange)
                    {
                        m_2D.DrawFilledBox(bar.x, bar.y + height, 1, kOverrunHeight, m_ColorBarOutOfRange);
                    }


                    if (analysisData != null && (full || !m_Dragging))
                    {
                        // Analysis is just on the subset
                        if (m_GlobalSettings.showThreads)
                        {
                            Profiler.BeginSample("FrameTimeGraph.ShowThreads");
                            ShowThreads(height, yRange, bar, full,
                                analysisData.GetThreads(), subsetSelected, selectedFirstOffset, selectedLastOffset,
                                offsetToIndexMapping, frameOffsetToSelectionIndex);
                            Profiler.EndSample();
                        }

                        if (m_GlobalSettings.showSelectedMarker)
                        {
                            // Analysis is just on the subset (unless we have full analysis data)
                            ShowSelectedMarker(height, yRange, bar, full, selectedMarker, subsetSelected, selectedFirstOffset, selectedLastOffset,
                                offsetToIndexMapping, frameOffsetToSelectionIndex);
                        }
                    }
                }

                m_2D.DrawEnd();

                if (m_GlobalSettings.showFrameLines && m_GlobalSettings.showFrameLineText)
                {
                    ShowFrameLinesText(rect, xStart, yStart, yRange, width, height, subsetSelected, selectedFirstOffset, selectedLastOffset);
                }

                foreach (BarData bar in m_Bars)
                {
                    bool inSelectionRegion = InSelectedRegion(bar.startDataOffset, bar.endDataOffset, selectedFirstOffset, selectedLastOffset, frameOffsetToSelectionIndex, subsetSelected);

                    // Draw tooltip for bar (or 1 pixel segment of bar)
                    {
                        int barStartIndex = offsetToDisplayMapping + m_Values[bar.startDataOffset].frameOffset;
                        int barEndIndex = offsetToDisplayMapping + m_Values[bar.endDataOffset].frameOffset;
                        string tooltip;
                        if (barStartIndex == barEndIndex)
                            tooltip = string.Format("Frame {0}\n{1}{2}", barStartIndex, ToDisplayUnits(bar.yMax, true), detailsString);
                        else
                            tooltip = string.Format("Frame {0}-{1}\n{2} max\n{3} min{4}", barStartIndex, barEndIndex, ToDisplayUnits(bar.yMax, true), ToDisplayUnits(bar.yMin, true), detailsString);

                        if (inSelectionRegion)
                            tooltip += selectionAreaString;
                        GUI.Label(new Rect(rect.x + bar.x, rect.y + 5, bar.w, height), new GUIContent("", tooltip));
                    }
                }
            }

            GUI.enabled = lastEnabled;

            if (showAxis)
            {
                int zoomedSelectedFirstOffset = selectedFirstOffset;
                int zoomedSelectedLastOffset = selectedLastOffset;
                int zoomedSelectedCount = selectedCount;
                if (m_Zoomed)
                {
                    if (selectedFirstOffset > endOffset || selectedLastOffset < startOffset)
                    {
                        zoomedSelectedCount = 0;
                    }
                    else
                    {
                        // Clamp selection range to zoom range
                        zoomedSelectedFirstOffset = ClampToRange(selectedFirstOffset, startOffset, endOffset);
                        zoomedSelectedLastOffset = ClampToRange(selectedLastOffset, startOffset, endOffset);
                        if (HasDragRegion())
                        {
                            zoomedSelectedCount = 1 + (zoomedSelectedLastOffset - zoomedSelectedFirstOffset);
                        }
                        else
                        {
                            zoomedSelectedCount = 0;
                            foreach (var offset in selectedFrameOffsets)
                            {
                                if (offset >= startOffset && offset <= endOffset)
                                {
                                    zoomedSelectedCount++;
                                }
                            }
                        }
                    }
                }

                ShowAxis(rect, xStart, width, startOffset, endOffset, zoomedSelectedFirstOffset, zoomedSelectedLastOffset, zoomedSelectedCount, selectedCount, yMax, totalDataSize, offsetToDisplayMapping);
            }

            GUI.enabled = enabled;
            if (rect.Contains(current.mousePosition) && current.type == EventType.ContextClick)
            {
                var analytic = ProfileAnalyzerAnalytics.BeginAnalytic();

                ShowContextMenu(subsetSelected, selectedCount);

                current.Use();

                ProfileAnalyzerAnalytics.SendUIVisibilityEvent(ProfileAnalyzerAnalytics.UIVisibility.FrameTimeContextMenu, analytic.GetDurationInSeconds(), true);
            }
            GUI.enabled = lastEnabled;

            Profiler.EndSample();
        }

        void ShowThreads(float height, float yRange, BarData bar, bool full,
            List<ThreadData> threads, bool subsetSelected, int selectedFirstOffset, int selectedLastOffset,
            int offsetToIndexMapping,  Dictionary<int, int> frameOffsetToSelectionIndex)
        {
            float max = float.MinValue;
            bool selected = false;
            for (int dataOffset = bar.startDataOffset; dataOffset <= bar.endDataOffset; dataOffset++)
            {
                int frameOffset = m_Values[dataOffset].frameOffset;
                if (!full && !frameOffsetToSelectionIndex.ContainsKey(frameOffset))
                    continue;

                float threadMs = 0f;
                foreach (var thread in threads)
                {
                    int frameIndex = offsetToIndexMapping + frameOffset;
                    var frame = thread.GetFrame(frameIndex);
                    if (frame == null)
                        continue;

                    float ms = frame.Value.ms;
                    if (ms > threadMs)
                        threadMs = ms;
                }

                if (threadMs > max)
                    max = threadMs;

                if (m_Dragging)
                {
                    if (frameOffset >= selectedFirstOffset && frameOffset <= selectedLastOffset)
                        selected = true;
                }
                else if (subsetSelected)
                {
                    if (frameOffsetToSelectionIndex.ContainsKey(frameOffset))
                        selected = true;
                }
            }

            if (full || selected)
            {
                // Clamp to frame time (these values can be time summed over multiple threads)
                if (max > bar.yMax)
                    max = bar.yMax;

                float maxClamped = Math.Min(max, yRange);
                float h = height * maxClamped / yRange;

                m_2D.DrawFilledBox(bar.x, bar.y, bar.w, h, selected ? m_ColorBarThreadsSelected : m_ColorBarThreads);

                // Show where its been clamped
                if (max > yRange)
                {
                    m_2D.DrawFilledBox(bar.x, bar.y + height, bar.w, kOverrunHeight, m_ColorBarThreadsOutOfRange);
                }
            }
        }

        void ShowSelectedMarker(float height, float yRange, BarData bar, bool full,
            MarkerData selectedMarker, bool subsetSelected, int selectedFirstOffset, int selectedLastOffset,
            int offsetToIndexMapping,  Dictionary<int, int> frameOffsetToSelectionIndex)
        {
            float max = 0f;
            bool selected = false;
            if (selectedMarker != null)
            {
                for (int dataOffset = bar.startDataOffset; dataOffset <= bar.endDataOffset; dataOffset++)
                {
                    int frameOffset = m_Values[dataOffset].frameOffset;
                    if (!full && !frameOffsetToSelectionIndex.ContainsKey(frameOffset))
                        continue;

                    float ms = selectedMarker.GetFrameMs(offsetToIndexMapping + frameOffset);

                    if (ms > max)
                        max = ms;

                    if (m_Dragging)
                    {
                        if (frameOffset >= selectedFirstOffset && frameOffset <= selectedLastOffset)
                            selected = true;
                    }
                    else if (subsetSelected)
                    {
                        if (frameOffsetToSelectionIndex.ContainsKey(frameOffset))
                            selected = true;
                    }
                }
            }

            if (full || selected)
            {
                // Clamp to frame time (these values can be tiem summed over multiple threads)
                if (max > bar.yMax)
                    max = bar.yMax;

                float maxClamped = Math.Min(max, yRange);
                float h = height * maxClamped / yRange;

                m_2D.DrawFilledBox(bar.x, bar.y, bar.w, h, selected ? m_ColorBarMarkerSelected : m_ColorBarMarker);

                if (max > 0f)
                {
                    // we start the bar lower so that very small markers still show up.
                    m_2D.DrawFilledBox(bar.x, bar.y - kOverrunHeight, bar.w, kOverrunHeight, m_ColorBarMarkerOutOfRange);
                }

                // Show where its been clamped
                if (max > yRange)
                {
                    m_2D.DrawFilledBox(bar.x, bar.y + height, bar.w, kOverrunHeight, m_ColorBarMarkerOutOfRange);
                }
            }
        }

        void ShowFrameLinesText(Rect rect, float xStart, float yStart, float yRange, float width, float height, bool subsetSelected, int selectedFirstOffset, int selectedLastOffset)
        {
            int totalDataSize = m_Values.Count;
            float y = yStart;

            float msSegment = 1000f / 60f;

            int lines = (int)(yRange / msSegment);
            int step = 1;
            for (int line = 1; line <= lines; line += step, step *= 2)
            {
                float ms = line * msSegment;
                float h = height * ms / yRange;
                int edgePad = 3;
                if (h >= (height / 4) && h < (height - GUI.skin.label.lineHeight))
                {
                    GUIContent content = new GUIContent(ToDisplayUnits((float)Math.Floor(ms), true, 0));
                    Vector2 size = EditorStyles.miniTextField.CalcSize(content);

                    bool left = true;
                    if (subsetSelected)
                    {
                        float x = GetXForDataOffset(selectedFirstOffset, (int)width, totalDataSize);
                        float x2 = GetXForDataOffset(selectedLastOffset + 1, (int)width, totalDataSize);

                        // text would overlap selection so move it if that prevents overlap
                        if (left)
                        {
                            if (x < (size.x + edgePad) && x2 < (width - (size.x + edgePad)))
                                left = false;
                        }
                        else
                        {
                            if (x > (size.x + edgePad) && x2 > (width - (size.x + edgePad)))
                                left = true;
                        }
                    }

                    Rect r;

                    if (left)
                        r = new Rect(rect.x + (xStart + edgePad), (rect.y - y) + (height - h), size.x, EditorStyles.miniTextField.lineHeight);
                    else
                        r = new Rect(rect.x + (xStart + width) - (size.x + edgePad), (rect.y - y) + (height - h), size.x, EditorStyles.miniTextField.lineHeight);
                    GUI.Label(r, content, EditorStyles.miniTextField);
                }
            }
        }

        void ShowSelectionMenuItem(bool subsetSelected, GenericMenu menu, GUIContent style, bool state, GenericMenu.MenuFunction func)
        {
            if (subsetSelected)
                menu.AddItem(style, state, func);
            else
                menu.AddDisabledItem(style);
        }

        void ShowContextMenu(bool subsetSelected, int selectionCount)
        {
            GenericMenu menu = new GenericMenu();
            bool showselectionOptions = subsetSelected || ((m_PairedWithFrameTimeGraph != null) && m_PairedWithFrameTimeGraph.HasSubsetSelected());
            ShowSelectionMenuItem(showselectionOptions || selectionCount == 0, menu, Styles.menuItemSelectAll, false, () => SelectAll());
            ShowSelectionMenuItem(showselectionOptions || selectionCount == m_Values.Count, menu, Styles.menuItemClearSelection, false, () => ClearSelection());
            menu.AddItem(Styles.menuItemInvertSelection, false, () => InvertSelection());
            menu.AddItem(Styles.menuItemSelectMin, false, () => SelectMin());
            menu.AddItem(Styles.menuItemSelectMax, false, () => SelectMax());
            menu.AddItem(Styles.menuItemSelectMedian, false, () => SelectMedian());
            menu.AddSeparator("");
            ShowSelectionMenuItem(showselectionOptions, menu, Styles.menuItemSelectPrevious, false, () => SelectPrevious(1));
            ShowSelectionMenuItem(showselectionOptions, menu, Styles.menuItemSelectNext, false, () => SelectNext(1));
            menu.AddSeparator("");
            ShowSelectionMenuItem(showselectionOptions, menu, Styles.menuItemSelectGrow, false, () => SelectGrow(1));
            ShowSelectionMenuItem(showselectionOptions, menu, Styles.menuItemSelectShrink, false, () => SelectShrink(1));
            ShowSelectionMenuItem(showselectionOptions, menu, Styles.menuItemSelectGrowLeft, false, () => SelectGrowLeft(1));
            ShowSelectionMenuItem(showselectionOptions, menu, Styles.menuItemSelectGrowRight, false, () => SelectGrowRight(1));
            ShowSelectionMenuItem(showselectionOptions, menu, Styles.menuItemSelectShrinkLeft, false, () => SelectShrinkLeft(1));
            ShowSelectionMenuItem(showselectionOptions, menu, Styles.menuItemSelectShrinkRight, false, () => SelectShrinkRight(1));
            menu.AddSeparator("");
            ShowSelectionMenuItem(showselectionOptions, menu, Styles.menuItemSelectGrowFast, false, () => SelectGrow(10));
            ShowSelectionMenuItem(showselectionOptions, menu, Styles.menuItemSelectShrinkFast, false, () => SelectShrink(10));
            ShowSelectionMenuItem(showselectionOptions, menu, Styles.menuItemSelectGrowLeftFast, false, () => SelectGrowLeft(10));
            ShowSelectionMenuItem(showselectionOptions, menu, Styles.menuItemSelectGrowRightFast, false, () => SelectGrowRight(10));
            ShowSelectionMenuItem(showselectionOptions, menu, Styles.menuItemSelectShrinkLeftFast, false, () => SelectShrinkLeft(10));
            ShowSelectionMenuItem(showselectionOptions, menu, Styles.menuItemSelectShrinkRightFast, false, () => SelectShrinkRight(10));
            menu.AddSeparator("");
            ShowSelectionMenuItem(showselectionOptions, menu, Styles.menuItemZoomSelection, false, () => ZoomSelection());
            ShowSelectionMenuItem(m_Zoomed, menu, Styles.menuItemZoomAll, false, () => ZoomAll());
            menu.AddSeparator("");
            menu.AddItem(Styles.menuItemShowSelectedMarker, m_GlobalSettings.showSelectedMarker, () => ToggleShowSelectedMarker());
            menu.AddItem(Styles.menuItemShowThreads, m_GlobalSettings.showThreads, () => ToggleShowThreads());
            menu.AddItem(Styles.menuItemShowFrameLines, m_GlobalSettings.showFrameLines, () => ToggleShowFrameLines());
            //menu.AddItem(Styles.menuItemShowFrameLineText, m_GlobalSettings.showFrameLineText, () => ToggleShowFrameLinesText());
            menu.AddSeparator("");
            menu.AddItem(Styles.menuItemShowOrderedByFrameDuration, m_GlobalSettings.showOrderedByFrameDuration, () => ToggleShowOrderedByFrameDuration());

            menu.ShowAsContext();
        }

        string GetYMaxText(float value)
        {
            return ToDisplayUnits(value, true, 0);
        }

        void DrawYAxisRangeSelector(Rect rect, float yMax)
        {
            string yMaxText = GetYMaxText(yMax);

            List<GUIContent> yAxisOptions = new List<GUIContent>();
            var graphScaleUnits = ToDisplayUnits(1000f / 60f, true, 0);
            yAxisOptions.Add(new GUIContent(graphScaleUnits, string.Format("Graph Scale : {0} is equivalent to 60Hz or 60FPS.", graphScaleUnits)));
            graphScaleUnits = ToDisplayUnits(1000f / 30f, true, 0);
            yAxisOptions.Add(new GUIContent(graphScaleUnits, string.Format("Graph Scale : {0} is equivalent to 30Hz or 30FPS.", graphScaleUnits)));
            graphScaleUnits = ToDisplayUnits(1000f / 15f, true, 0);
            yAxisOptions.Add(new GUIContent(graphScaleUnits, string.Format("Graph Scale : {0} is equivalent to 15Hz or 15FPS.", graphScaleUnits)));
            yAxisOptions.Add(new GUIContent(yMaxText, "Graph Scale : Max frame time from data"));

            float width = 0;
            foreach (var content in yAxisOptions)
            {
                Vector2 size = EditorStyles.popup.CalcSize(content);
                if (size.x > width)
                    width = size.x;
            }

            // Use smaller width if text is shorter
            int margin = 2;
            width = Math.Min(width + margin, rect.width);
            // Shift right to right align
            rect.x += (rect.width - width);
            rect.x -= margin;
            rect.width = width;
            s_YAxisMode = (AxisMode)EditorGUI.Popup(rect, (int)s_YAxisMode, yAxisOptions.ToArray());
        }

        void ShowAxis(Rect rect, float xStart, float width, int startOffset, int endOffset, int selectedFirstOffset, int selectedLastOffset, int selectedCount, int totalSelectedCount, float yMax, int totalDataSize, int offsetToDisplayMapping)
        {
            GUIStyle leftAlignStyle = new GUIStyle(GUI.skin.label);
            leftAlignStyle.padding = new RectOffset(leftAlignStyle.padding.left, leftAlignStyle.padding.right, 0, 0);
            leftAlignStyle.alignment = TextAnchor.MiddleLeft;
            GUIStyle rightAlignStyle = new GUIStyle(GUI.skin.label);
            rightAlignStyle.padding = new RectOffset(rightAlignStyle.padding.left, rightAlignStyle.padding.right, 0, 0);
            rightAlignStyle.alignment = TextAnchor.MiddleRight;

            // y axis
            float h = GUI.skin.label.lineHeight;
            float y = rect.y + ((rect.height - 1) - h);


            DrawYAxisRangeSelector(new Rect(rect.x, rect.y, kXAxisWidth, h), yMax);

            string yMinText = ToDisplayUnits(0, true);
            GUI.Label(new Rect(rect.x, y - h, kXAxisWidth, h), yMinText, rightAlignStyle);


            // x axis
            rect.x += xStart;

            leftAlignStyle.padding = new RectOffset(0, 0, leftAlignStyle.padding.top, leftAlignStyle.padding.bottom);
            rightAlignStyle.padding = new RectOffset(0, 0, rightAlignStyle.padding.top, rightAlignStyle.padding.bottom);

            int startIndex = offsetToDisplayMapping + startOffset;
            string startIndexText = string.Format("{0}", startIndex);
            GUIContent startIndexContent = new GUIContent(startIndexText);
            Vector2 startIndexSize = GUI.skin.label.CalcSize(startIndexContent);
            bool drawStart = !m_GlobalSettings.showOrderedByFrameDuration;

            int endIndex = offsetToDisplayMapping + endOffset;
            string endIndexText = string.Format("{0}", endIndex);
            GUIContent endIndexContent = new GUIContent(endIndexText);
            Vector2 endIndexSize = GUI.skin.label.CalcSize(endIndexContent);
            bool drawEnd = !m_GlobalSettings.showOrderedByFrameDuration;


            // Show selection frame values (if space for them)
            if (totalSelectedCount > 0)
            {
                if (selectedCount == 0)
                {
                    // If we have no selection then adjust 'selection start/end to span whole view so the count display is centred)
                    selectedFirstOffset = startOffset;
                    selectedLastOffset = endOffset;
                }

                int selectedFirstX = GetXForDataOffset(selectedFirstOffset, (int)width, totalDataSize);
                int selectedLastX = GetXForDataOffset(selectedLastOffset + 1, (int)width, totalDataSize);   // last + 1 so right hand side of the bbar
                int selectedRangeWidth = 1 + (selectedLastX - selectedFirstX);

                int selectedFirstIndex = offsetToDisplayMapping + selectedFirstOffset;
                int selectedLastIndex = offsetToDisplayMapping + selectedLastOffset;

                string selectionCountText;
                if (totalSelectedCount != selectedCount)
                    selectionCountText = string.Format("[{0} of {1}]", selectedCount, totalSelectedCount);
                else
                    selectionCountText = string.Format("[{0}]", selectedCount);

                string selectionRangeText;
                if (selectedCount > 1)
                {
                    if (m_GlobalSettings.showOrderedByFrameDuration)
                        selectionRangeText = selectionCountText;
                    else
                        selectionRangeText = string.Format("{0} {1} {2}", selectedFirstIndex, selectionCountText, selectedLastIndex);
                }
                else
                    selectionRangeText = string.Format("{0} {1}", selectedFirstIndex, selectionCountText);

                string tooltip = string.Format("{0} frames in selection", selectedCount);
                if (totalSelectedCount != selectedCount)
                {
                    tooltip = string.Format("{0} frames in zoomed selection\n{1} frames in overall selection", selectedCount, totalSelectedCount);
                }

                GUIContent selectionRangeTextContent = new GUIContent(selectionRangeText, tooltip);
                Vector2 selectionRangeTextSize = GUI.skin.label.CalcSize(selectionRangeTextContent);
                if ((selectedRangeWidth > selectionRangeTextSize.x && selectedCount > 1) || selectedCount == 0)
                {
                    // Selection width is larger than the text so we can split the text
                    string selectedFirstIndexText = string.Format("{0}", selectedFirstIndex);
                    GUIContent selectedFirstIndexContent = new GUIContent(selectedFirstIndexText);
                    Vector2 selectedFirstIndexSize = GUI.skin.label.CalcSize(selectedFirstIndexContent);
                    if (m_GlobalSettings.showOrderedByFrameDuration)
                        selectedFirstIndexSize.x = 0;

                    string selectedLastIndexText = string.Format("{0}", selectedLastIndex);
                    GUIContent selectedLastIndexContent = new GUIContent(selectedLastIndexText);
                    Vector2 selectedLastIndexSize = GUI.skin.label.CalcSize(selectedLastIndexContent);
                    if (m_GlobalSettings.showOrderedByFrameDuration)
                        selectedLastIndexSize.x = 0;

                    GUIContent selectedCountContent = new GUIContent(selectionCountText, tooltip);
                    Vector2 selectedCountSize = GUI.skin.label.CalcSize(selectedCountContent);

                    Rect rFirst = new Rect(rect.x + selectedFirstX, y, selectedFirstIndexSize.x, selectedFirstIndexSize.y);
                    GUI.Label(rFirst, selectedFirstIndexContent);

                    Rect rLast = new Rect(rect.x + selectedLastX - selectedLastIndexSize.x, y, selectedLastIndexSize.x, selectedLastIndexSize.y);
                    GUI.Label(rLast, selectedLastIndexContent);

                    float mid = selectedFirstX + ((selectedLastX - selectedFirstX) / 2);
                    Rect rCount = new Rect(rect.x + mid - (selectedCountSize.x / 2), y, selectedCountSize.x, selectedCountSize.y);
                    GUI.Label(rCount, selectedCountContent);

                    if (selectedFirstX < startIndexSize.x)
                    {
                        // would overlap with start text
                        drawStart = false;
                    }
                    if (selectedLastX > ((width - 1) - endIndexSize.x))
                    {
                        // would overlap with end text
                        drawEnd = false;
                    }
                }
                else
                {
                    int mid = (selectedFirstX + (selectedRangeWidth / 2));
                    int selectionTextX = mid - (int)(selectionRangeTextSize.x / 2);
                    selectionTextX = ClampToRange(selectionTextX, 0, (int)((width - 1) - selectionRangeTextSize.x));

                    Rect rangeRect = new Rect(rect.x + selectionTextX, y, selectionRangeTextSize.x, selectionRangeTextSize.y);
                    GUI.Label(rangeRect, selectionRangeTextContent);

                    if (selectionTextX < startIndexSize.x)
                    {
                        // would overlap with start text
                        drawStart = false;
                    }
                    if ((selectionTextX + selectionRangeTextSize.x) > ((width - 1) - endIndexSize.x))
                    {
                        // would overlap with end text
                        drawEnd = false;
                    }
                }
            }


            // Show start and end values
            if (drawStart)
            {
                Rect leftRect = new Rect(rect.x, y, startIndexSize.x, startIndexSize.y);
                GUI.Label(leftRect, startIndexContent, leftAlignStyle);
            }

            if (drawEnd)
            {
                Rect rightRect = new Rect(rect.x + ((width - 1) - endIndexSize.x), y, endIndexSize.x, endIndexSize.y);
                GUI.Label(rightRect, endIndexContent, rightAlignStyle);
            }
        }

        void ClearSelection(bool effectPaired = true)
        {
            int dataLength = m_Values.Count;

            bool singleControlAction = true;    // As we need the frame range to be unique to each data set
            CallSetRange(-1, -1, 0, singleControlAction, FrameTimeGraph.State.DragComplete);

            // Disable zoom
            m_Zoomed = false;

            if (m_PairedWithFrameTimeGraph != null && effectPaired)
                m_PairedWithFrameTimeGraph.ClearSelection(false);
        }

        void SelectAll(bool effectPaired = true)
        {
            int dataLength = m_Values.Count;

            bool singleControlAction = true;    // As we need the frame range to be unique to each data set
            CallSetRange(0, dataLength - 1, 0, singleControlAction, FrameTimeGraph.State.DragComplete);

            // Disable zoom
            m_Zoomed = false;

            if (m_PairedWithFrameTimeGraph != null && effectPaired)
                m_PairedWithFrameTimeGraph.SelectAll(false);
        }

        void InvertSelection(bool effectPaired = true)
        {
            int dataLength = m_Values.Count;

            Dictionary<int, int> frameOffsetToSelectionIndex = new Dictionary<int, int>();
            for (int i = 0; i < m_CurrentSelection.Count; i++)
            {
                int frameOffset = m_CurrentSelection[i];
                frameOffsetToSelectionIndex[frameOffset] = i;
            }

            m_CurrentSelection.Clear();
            for (int frameIndex = 0; frameIndex < dataLength; frameIndex++)
            {
                if (!frameOffsetToSelectionIndex.ContainsKey(frameIndex))
                    m_CurrentSelection.Add(frameIndex);
            }

            m_SetRange(m_CurrentSelection, 1, FrameTimeGraph.State.DragComplete);

            // Disable zoom
            m_Zoomed = false;

            if (m_PairedWithFrameTimeGraph != null && effectPaired)
                m_PairedWithFrameTimeGraph.InvertSelection(false);
        }

        void ZoomSelection(bool effectPaired = true)
        {
            m_Zoomed = true;
            m_ZoomStartOffset = m_CurrentSelectionFirstDataOffset;
            m_ZoomEndOffset = m_CurrentSelectionLastDataOffset;

            if (m_PairedWithFrameTimeGraph != null && effectPaired)
                m_PairedWithFrameTimeGraph.ZoomSelection(false);
        }

        void ZoomAll(bool effectPaired = true)
        {
            m_Zoomed = false;
            int frames = m_Values.Count;

            m_ZoomStartOffset = 0;
            m_ZoomEndOffset = frames - 1;


            if (m_PairedWithFrameTimeGraph != null && effectPaired)
                m_PairedWithFrameTimeGraph.ZoomAll(false);
        }

        void SelectMin(bool effectPaired = true)
        {
            int dataLength = m_Values.Count;
            if (dataLength <= 0)
                return;


            int minDataOffset = 0;
            float msMin = m_Values[0].ms;
            for (int dataOffset = 0; dataOffset < dataLength; dataOffset++)
            {
                float ms = m_Values[dataOffset].ms;
                if (ms < msMin)
                {
                    msMin = ms;
                    minDataOffset = dataOffset;
                }
            }

            bool singleControlAction = true;
            CallSetRange(minDataOffset, minDataOffset, 1, singleControlAction, State.DragComplete); // act like single click, so we select frame
            //CallSetRange(minDataOffset, minDataOffset, 2, singleControlAction, State.DragComplete); // act like double click, so we jump to the frame
            m_Zoomed = false;


            if (m_PairedWithFrameTimeGraph != null && effectPaired)
                m_PairedWithFrameTimeGraph.SelectMin(false);
        }

        void SelectMax(bool effectPaired = true)
        {
            int dataLength = m_Values.Count;
            if (dataLength <= 0)
                return;


            int maxDataOffset = 0;
            float msMax = m_Values[0].ms;
            for (int dataOffset = 0; dataOffset < dataLength; dataOffset++)
            {
                float ms = m_Values[dataOffset].ms;
                if (ms > msMax)
                {
                    msMax = ms;
                    maxDataOffset = dataOffset;
                }
            }

            bool singleControlAction = true;
            CallSetRange(maxDataOffset, maxDataOffset, 1, singleControlAction, State.DragComplete); // act like single click, so we select frame
            //CallSetRange(maxDataOffset, maxDataOffset, 2, singleControlAction, State.DragComplete); // act like double click, so we jump to the frame
            m_Zoomed = false;


            if (m_PairedWithFrameTimeGraph != null && effectPaired)
                m_PairedWithFrameTimeGraph.SelectMax(false);
        }

        float GetPercentageOffset(List<Data> data, float percent, out int outputFrameOffset)
        {
            int index = (int)((data.Count - 1) * percent / 100);
            outputFrameOffset = data[index].frameOffset;

            // True median is half of the sum of the middle 2 frames for an even count. However this would be a value never recorded so we avoid that.
            return data[index].ms;
        }

        void SelectMedian(bool effectPaired = true)
        {
            int dataLength = m_Values.Count;
            if (dataLength <= 0)
                return;

            List<Data> sortedValues = new List<Data>();
            foreach (var value in m_Values)
            {
                Data data = new Data(value.ms, value.frameOffset);
                sortedValues.Add(data);
            }
            // If ms value is the same then order by frame offset
            sortedValues.Sort((a, b) => { int compare = a.ms.CompareTo(b.ms); return compare == 0 ? a.frameOffset.CompareTo(b.frameOffset) : compare; });
            int medianFrameOffset = 0;
            GetPercentageOffset(sortedValues, 50, out medianFrameOffset);
            int medianDataOffset = m_FrameOffsetToDataOffsetMapping[medianFrameOffset];


            bool singleControlAction = true;
            CallSetRange(medianDataOffset, medianDataOffset, 1, singleControlAction, State.DragComplete); // act like single click, so we select frame
            //CallSetRange(medianDataOffset, medianDataOffset, 2, singleControlAction, State.DragComplete); // act like double click, so we jump to the frame
            m_Zoomed = false;


            if (m_PairedWithFrameTimeGraph != null && effectPaired)
                m_PairedWithFrameTimeGraph.SelectMedian(false);
        }

        public void SelectPrevious(int step, bool effectPaired = true)
        {
            int clicks = 1;
            bool singleClickAction = true;

            MoveSelectedRange(-step, clicks, singleClickAction, State.DragComplete);

            if (m_PairedWithFrameTimeGraph != null && effectPaired)
                m_PairedWithFrameTimeGraph.SelectPrevious(step, false);
        }

        public void SelectNext(int step, bool effectPaired = true)
        {
            int clicks = 1;
            bool singleClickAction = true;

            MoveSelectedRange(step, clicks, singleClickAction, State.DragComplete);

            if (m_PairedWithFrameTimeGraph != null && effectPaired)
                m_PairedWithFrameTimeGraph.SelectNext(step, false);
        }

        public void SelectGrow(int step, bool effectPaired = true)
        {
            int clicks = 1;
            bool singleClickAction = true;

            ResizeSelectedRange(-step, step, clicks, singleClickAction, State.DragComplete);

            if (m_PairedWithFrameTimeGraph != null && effectPaired)
                m_PairedWithFrameTimeGraph.SelectGrow(step, false);
        }

        public void SelectGrowLeft(int step, bool effectPaired = true)
        {
            int clicks = 1;
            bool singleClickAction = true;

            ResizeSelectedRange(-step, 0, clicks, singleClickAction, State.DragComplete);

            if (m_PairedWithFrameTimeGraph != null && effectPaired)
                m_PairedWithFrameTimeGraph.SelectGrowLeft(step, false);
        }

        public void SelectGrowRight(int step, bool effectPaired = true)
        {
            int clicks = 1;
            bool singleClickAction = true;

            ResizeSelectedRange(0, step, clicks, singleClickAction, State.DragComplete);

            if (m_PairedWithFrameTimeGraph != null && effectPaired)
                m_PairedWithFrameTimeGraph.SelectGrowRight(step, false);
        }

        public void SelectShrink(int step, bool effectPaired = true)
        {
            int clicks = 1;
            bool singleClickAction = true;

            ResizeSelectedRange(step, -step, clicks, singleClickAction, State.DragComplete);

            if (m_PairedWithFrameTimeGraph != null && effectPaired)
                m_PairedWithFrameTimeGraph.SelectShrink(step, false);
        }

        public void SelectShrinkLeft(int step, bool effectPaired = true)
        {
            int clicks = 1;
            bool singleClickAction = true;

            ResizeSelectedRange(step, 0, clicks, singleClickAction, State.DragComplete);

            if (m_PairedWithFrameTimeGraph != null && effectPaired)
                m_PairedWithFrameTimeGraph.SelectShrinkLeft(step, false);
        }

        public void SelectShrinkRight(int step, bool effectPaired = true)
        {
            int clicks = 1;
            bool singleClickAction = true;

            ResizeSelectedRange(0, -step, clicks, singleClickAction, State.DragComplete);

            if (m_PairedWithFrameTimeGraph != null && effectPaired)
                m_PairedWithFrameTimeGraph.SelectShrinkRight(step, false);
        }

        public void ToggleShowThreads()
        {
            m_GlobalSettings.showThreads ^= true;
        }

        public void ToggleShowSelectedMarker()
        {
            m_GlobalSettings.showSelectedMarker ^= true;
        }

        public void ToggleShowFrameLines()
        {
            m_GlobalSettings.showFrameLines ^= true;
        }

        public void ToggleShowFrameLinesText()
        {
            m_GlobalSettings.showFrameLineText ^= true;
        }

        public void ToggleShowOrderedByFrameDuration()
        {
            m_GlobalSettings.showOrderedByFrameDuration ^= true;
            SetData(m_Values);  // Update order

            if (m_PairedWithFrameTimeGraph != null)
            {
                m_PairedWithFrameTimeGraph.SetData(m_PairedWithFrameTimeGraph.m_Values);  // Update order
            }
        }

        internal struct SelectedRangeState
        {
            public int currentSelectionFirstDataOffset;
            public int currentSelectionLastDataOffset;
            public List<int> lastSelectedFrameOffsets;
        }

        void MoveSelectedRange(int offset, int clickCount, bool singleControlAction, State inputStatus)
        {
            MoveSelectedRange(offset, clickCount, singleControlAction, inputStatus, new SelectedRangeState()
            {
                currentSelectionFirstDataOffset = m_CurrentSelectionFirstDataOffset,
                currentSelectionLastDataOffset = m_CurrentSelectionLastDataOffset,
                lastSelectedFrameOffsets = m_LastSelectedFrameOffsets,
            });
        }

        internal void MoveSelectedRange(int offset, int clickCount, bool singleControlAction, State inputStatus, SelectedRangeState selectedRange)
        {
            var currentSelectionFirstDataOffset = selectedRange.currentSelectionFirstDataOffset;
            var currentSelectionLastDataOffset = selectedRange.currentSelectionLastDataOffset;
            var lastSelectedFrameOffsets = selectedRange.lastSelectedFrameOffsets;

            if (offset < 0)
            {
                // Clamp offset to graph lower bound.
                if (currentSelectionFirstDataOffset + offset < 0)
                {
                    offset = -currentSelectionFirstDataOffset;
                }
            }
            else
            {
                // Clamp offset to graph upper bound.
                if (currentSelectionLastDataOffset + offset >= m_Values.Count)
                {
                    offset = (m_Values.Count - 1) - currentSelectionLastDataOffset;
                }
            }

            // Offset selection.
            List<int> selected = new List<int>();
            foreach (int selectedFrameOffset in lastSelectedFrameOffsets)
            {
                var selectedDataOffset = m_FrameOffsetToDataOffsetMapping[selectedFrameOffset];
                var newDataOffset = selectedDataOffset + offset;
                if (newDataOffset >= 0 && newDataOffset < m_Values.Count)
                {
                    var newFrameOffset = m_Values[newDataOffset].frameOffset;
                    if (!selected.Contains(newFrameOffset))
                    {
                        selected.Add(newFrameOffset);
                    }
                }
            }

            // Sort selection in frame index order.
            selected.Sort();

            m_SetRange(selected, clickCount, inputStatus);
        }

        void ResizeSelectedRange(int leftOffset, int rightOffset, int clickCount, bool singleControlAction, State inputState)
        {
            ResizeSelectedRange(leftOffset, rightOffset, clickCount, singleControlAction, inputState, new SelectedRangeState()
            {
                currentSelectionFirstDataOffset = m_CurrentSelectionFirstDataOffset,
                currentSelectionLastDataOffset = m_CurrentSelectionLastDataOffset,
                lastSelectedFrameOffsets = m_LastSelectedFrameOffsets,
            });
        }

        internal void ResizeSelectedRange(int leftOffset, int rightOffset, int clickCount, bool singleControlAction, State inputState, SelectedRangeState selectedRange)
        {
            const int k_InvalidDataOffset = -1;
            var currentSelectionFirstDataOffset = selectedRange.currentSelectionFirstDataOffset;
            var currentSelectionLastDataOffset = selectedRange.currentSelectionLastDataOffset;
            var lastSelectedFrameOffsets = selectedRange.lastSelectedFrameOffsets;

            // Clamp left offset to lower graph bound.
            bool isGrowingLeft = leftOffset < 0;
            if (isGrowingLeft && currentSelectionFirstDataOffset + leftOffset < 0)
            {
                leftOffset = -currentSelectionFirstDataOffset;
            }

            // Clamp right offset to upper graph bound.
            bool isGrowingRight = rightOffset > 0;
            if (isGrowingRight && currentSelectionLastDataOffset + rightOffset > (m_Values.Count - 1))
            {
                rightOffset = (m_Values.Count - 1) - currentSelectionLastDataOffset;
            }

            int selectionStartDataOffset = k_InvalidDataOffset;
            List<int> selected = new List<int>(lastSelectedFrameOffsets);
            for (int dataOffset = 0; dataOffset < m_Values.Count; ++dataOffset)
            {
                var frameOffset = m_Values[dataOffset].frameOffset;
                if (selectionStartDataOffset == k_InvalidDataOffset)
                {
                    // Find selection start.
                    if (lastSelectedFrameOffsets.Contains(frameOffset))
                    {
                        selectionStartDataOffset = dataOffset;
                    }
                }
                else
                {
                    // Find selection end.
                    bool isSelected = lastSelectedFrameOffsets.Contains(frameOffset);
                    if (!isSelected || dataOffset == (m_Values.Count - 1))
                    {
                        int selectionEndDataOffset;

                        // If we reached the last index and it is selected, this index is the selection end. Otherwise, the previous index is the last selected.
                        bool isLastIndex = dataOffset == (m_Values.Count - 1);
                        if (isLastIndex && isSelected)
                        {
                            selectionEndDataOffset = dataOffset;
                        }
                        else
                        {
                            selectionEndDataOffset = dataOffset - 1;
                        }

                        int newSelectionStartDataOffset = Mathf.Clamp(selectionStartDataOffset + leftOffset, 0, m_Values.Count - 1);
                        int newSelectionEndDataOffset = Mathf.Clamp(selectionEndDataOffset + rightOffset, 0, m_Values.Count - 1);

                        // Enforce a minimum selection width.
                        if (newSelectionEndDataOffset < newSelectionStartDataOffset)
                        {
                            var maximumOffset = (selectionEndDataOffset - selectionStartDataOffset) * 0.5f;
                            var adjustedLeftOffset = Mathf.CeilToInt(maximumOffset);
                            newSelectionStartDataOffset = Mathf.Clamp(selectionStartDataOffset + adjustedLeftOffset, 0, m_Values.Count - 1);
                            var adjustedRightOffset = -Mathf.FloorToInt(maximumOffset);
                            newSelectionEndDataOffset = Mathf.Clamp(selectionEndDataOffset + adjustedRightOffset, 0, m_Values.Count - 1);
                        }

                        if (selectionStartDataOffset != newSelectionStartDataOffset)
                        {
                            // Resize from left edge.
                            int startDataOffset = Mathf.Min(selectionStartDataOffset, newSelectionStartDataOffset);
                            int endDataOffset = Mathf.Max(selectionStartDataOffset, newSelectionStartDataOffset);
                            MoveSelectionEdge(startDataOffset, endDataOffset, isGrowingLeft, ref selected);
                        }

                        if (selectionEndDataOffset != newSelectionEndDataOffset)
                        {
                            // Resize from right edge (iterate backwards).
                            int startDataOffset = Mathf.Max(selectionEndDataOffset, newSelectionEndDataOffset);
                            int endDataOffset = Mathf.Min(selectionEndDataOffset, newSelectionEndDataOffset);
                            MoveSelectionEdge(startDataOffset, endDataOffset, isGrowingRight, ref selected);
                        }

                        // Reset to find next selection.
                        selectionStartDataOffset = k_InvalidDataOffset;
                    }
                }
            }

            // Sort selection in frame index order.
            selected.Sort();

            m_SetRange(selected, clickCount, inputState);
        }

        void MoveSelectionEdge(int startDataOffset, int endDataOffset, bool isGrowingFromEdge, ref List<int> selection)
        {
            var direction = (startDataOffset >= endDataOffset) ? -1 : 1;
            for (int dataOffset = startDataOffset; dataOffset != endDataOffset; dataOffset += direction)
            {
                var frameOffset = m_Values[dataOffset].frameOffset;
                var indexInSelection = selection.IndexOf(frameOffset);
                if (isGrowingFromEdge)
                {
                    if (indexInSelection == -1)
                    {
                        selection.Add(frameOffset);
                    }
                }
                else
                {
                    if (indexInSelection != -1)
                    {
                        selection.RemoveAt(indexInSelection);
                    }
                }
            }
        }
    }
}
