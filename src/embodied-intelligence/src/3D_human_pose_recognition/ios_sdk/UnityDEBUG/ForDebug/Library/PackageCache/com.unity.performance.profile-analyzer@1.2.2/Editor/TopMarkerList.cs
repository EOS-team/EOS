using System;
using System.Collections.Generic;
using UnityEngine;

namespace UnityEditor.Performance.ProfileAnalyzer
{
    internal class TopMarkerList
    {
        internal static class Styles
        {
            public static readonly GUIContent frameCosts = new GUIContent(" by frame costs", "Contains accumulated marker cost within the frame");
            public static readonly GUIContent frameCounts = new GUIContent(" by frame counts", "Contains marker count within the frame");
        }

        public delegate float DrawFrameIndexButton(int frameIndex, ProfileDataView frameContext);

        DrawFrameIndexButton m_DrawFrameIndexButton;
        Draw2D m_2D;
        DisplayUnits m_Units;
        int m_WidthColumn0;
        int m_WidthColumn1;
        int m_WidthColumn2;
        int m_WidthColumn3;
        Color colorBar;
        Color colorBarBackground;

        public TopMarkerList(Draw2D draw2D, Units units,
                             int widthColumn0, int widthColumn1, int widthColumn2, int widthColumn3,
                             Color colorBar, Color colorBarBackground, DrawFrameIndexButton drawFrameIndexButton)
        {
            m_2D = draw2D;
            SetUnits(units);
            m_WidthColumn0 = widthColumn0;
            m_WidthColumn1 = widthColumn1;
            m_WidthColumn2 = widthColumn2;
            m_WidthColumn3 = widthColumn3;
            this.colorBar = colorBar;
            this.colorBarBackground = colorBarBackground;
            m_DrawFrameIndexButton = drawFrameIndexButton;
        }

        void SetUnits(Units units)
        {
            m_Units = new DisplayUnits(units);
        }

        public int DrawTopNumber(int topNumber, string[] topStrings, int[] topValues)
        {
            EditorGUILayout.BeginHorizontal();
            EditorGUILayout.LabelField("Top ", GUILayout.Width(30));
            topNumber = EditorGUILayout.IntPopup(topNumber, topStrings, topValues, GUILayout.Width(40));
            EditorGUILayout.LabelField(m_Units.Units == Units.Count ? Styles.frameCounts : Styles.frameCosts, GUILayout.Width(100));
            EditorGUILayout.EndHorizontal();

            return topNumber;
        }

        public List<FrameTime> GetTopNByCount(MarkerData marker, int n)
        {
            if (marker.frames.Count <= 0)
                return new List<FrameTime>();

            List<FrameTime> sortedFrames = new List<FrameTime>(marker.frames);
            sortedFrames.Sort(FrameTime.CompareCountDescending);

            var frameTimes = new List<FrameTime>();
            for (int i = 0; i < Math.Min(n, sortedFrames.Count); i++)
                frameTimes.Add(sortedFrames[i]);

            return frameTimes;
        }

        public List<FrameTime> GetTopNByTime(MarkerData marker, int n)
        {
            if (marker.frames.Count <= 0)
                return new List<FrameTime>();

            List<FrameTime> sortedFrames = new List<FrameTime>(marker.frames);
            sortedFrames.Sort(FrameTime.CompareMsDescending);

            var frameTimes = new List<FrameTime>();
            for (int i = 0; i < Math.Min(n, sortedFrames.Count); i++)
                frameTimes.Add(sortedFrames[i]);

            return frameTimes;
        }

        public List<FrameTime> GetTopN(MarkerData marker, int n, bool showCount)
        {
            return showCount ? GetTopNByCount(marker, n) : GetTopNByTime(marker, n);
        }

        public int Draw(MarkerData marker, ProfileDataView markerContext, int topNumber, string[] topStrings, int[] topValues)
        {
            GUIStyle style = GUI.skin.label;
            float w = m_WidthColumn0;
            float h = style.lineHeight;
            float ySpacing = 2;
            float barHeight = h - ySpacing;

            EditorGUILayout.BeginVertical(GUILayout.Width(w + m_WidthColumn1 + m_WidthColumn2 + m_WidthColumn3));

            topNumber = DrawTopNumber(topNumber, topStrings, topValues);

            /*
            EditorGUILayout.BeginHorizontal();
            EditorGUILayout.LabelField("", GUILayout.Width(w));
            EditorGUILayout.LabelField("Value", GUILayout.Width(LayoutSize.WidthColumn1));
            EditorGUILayout.LabelField("Frame", GUILayout.Width(LayoutSize.WidthColumn2));
            EditorGUILayout.EndHorizontal();
            */

            // var frameSummary = m_ProfileSingleView.analysis.GetFrameSummary();
            float barMax = marker.msMax; // frameSummary.msMax
            if (m_Units.Units == Units.Count)
            {
                barMax = marker.countMax;
            }

            // Marker frames are ordered by frame time
            // If we are sorting by count then we need to apply a sort
            bool showCount = m_Units.Units == Units.Count;
            List<FrameTime> frames = GetTopN(marker, topNumber, showCount);

            foreach (FrameTime frameTime in frames)
            {
                float barValue = (m_Units.Units == Units.Count) ? frameTime.count : frameTime.ms;
                float barLength = Math.Min((w * barValue) / barMax, w);

                EditorGUILayout.BeginHorizontal();
                if (m_2D.DrawStart(w, h, Draw2D.Origin.TopLeft, style))
                {
                    m_2D.DrawFilledBox(0, ySpacing, barLength, barHeight, colorBar);
                    m_2D.DrawFilledBox(barLength, ySpacing, w - barLength, barHeight, colorBarBackground);
                    m_2D.DrawEnd();

                    Rect rect = GUILayoutUtility.GetLastRect();
                    GUI.Label(rect, new GUIContent("", m_Units.ToTooltipString(barValue, true)));
                }
                EditorGUILayout.LabelField(m_Units.ToGUIContentWithTooltips(barValue, true), GUILayout.Width(m_WidthColumn2));
                if (m_DrawFrameIndexButton != null)
                    m_DrawFrameIndexButton(frameTime.frameIndex, markerContext);
                GUILayout.FlexibleSpace();
                EditorGUILayout.EndHorizontal();
            }

            // Show blank space for missing frames
            var content = new GUIContent("", "");
            Vector2 size = GUI.skin.button.CalcSize(content);
            h = Math.Max(barHeight, size.y);
            for (int i = frames.Count; i < topNumber; i++)
            {
                EditorGUILayout.BeginHorizontal();
                GUILayout.Label("", GUILayout.Height(h));
                EditorGUILayout.EndHorizontal();
            }

            EditorGUILayout.EndVertical();

            return topNumber;
        }
    }
}
