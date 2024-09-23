using UnityEngine;

namespace UnityEditor.Performance.ProfileAnalyzer
{
    internal class Histogram
    {
        Draw2D m_2D;
        Color m_ColorBarBackground;
        DisplayUnits m_Units;

        public void SetUnits(Units units)
        {
            m_Units = new DisplayUnits(units);
        }

        public Histogram(Draw2D draw2D, Units units)
        {
            m_2D = draw2D;
            SetUnits(units);
            m_ColorBarBackground = new Color(0.5f, 0.5f, 0.5f);
        }

        public Histogram(Draw2D draw2D, Units units, Color barBackground)
        {
            m_2D = draw2D;
            SetUnits(units);
            m_ColorBarBackground = barBackground;
        }

        public void DrawStart(float width)
        {
            EditorGUILayout.BeginHorizontal(GUILayout.Width(width + 10));

            EditorGUILayout.BeginVertical();
        }

        public void DrawEnd(float width, float min, float max, float spacing)
        {
            EditorGUILayout.BeginHorizontal();
            float halfWidth = width / 2;
            GUIStyle leftAlignStyle = new GUIStyle(GUI.skin.label);
            leftAlignStyle.contentOffset = new Vector2(-5, 0);
            leftAlignStyle.alignment = TextAnchor.MiddleLeft;
            GUIStyle rightAlignStyle = new GUIStyle(GUI.skin.label);
            rightAlignStyle.contentOffset = new Vector2(-5, 0);
            rightAlignStyle.alignment = TextAnchor.MiddleRight;
            EditorGUILayout.LabelField(m_Units.ToString(min, false, 5, true), leftAlignStyle, GUILayout.Width(halfWidth));
            EditorGUILayout.LabelField(m_Units.ToString(max, false, 5, true), rightAlignStyle, GUILayout.Width(halfWidth));
            EditorGUILayout.EndHorizontal();

            EditorGUILayout.EndVertical();

            EditorGUILayout.EndHorizontal();
        }

        public void DrawBackground(float width, float height, int bucketCount, float min, float max, float spacing)
        {
            //bucketCount = (range == 0f) ? 1 : bucketCount;

            float x = (spacing / 2);
            float y = 0;
            float w = ((width + spacing) / bucketCount) - spacing;
            float h = height;

            for (int i = 0; i < bucketCount; i++)
            {
                m_2D.DrawFilledBox(x, y, w, h, m_ColorBarBackground);
                x += w;
                x += spacing;
            }
        }

        public void DrawData(float width, float height, int[] buckets, int totalFrameCount, float min, float max, Color barColor, float spacing)
        {
            float range = max - min;

            //int bucketCount = (range == 0f) ? 1 : buckets.Length;
            int bucketCount = buckets.Length;

            float x = (spacing / 2);
            float y = 0;
            float w = ((width + spacing) / bucketCount) - spacing;
            float h = height;

            float bucketWidth = (range / bucketCount);
            Rect rect = GUILayoutUtility.GetLastRect();
            for (int bucketAt = 0; bucketAt < bucketCount; bucketAt++)
            {
                var count = buckets[bucketAt];

                float barHeight = (h * count) / totalFrameCount;
                if (count > 0)  // Make sure we always slow a small bar if non zero
                    barHeight = Mathf.Max(1.0f, barHeight);
                m_2D.DrawFilledBox(x, y, w, barHeight, barColor);

                float bucketStart = min + (bucketAt * bucketWidth);
                float bucketEnd = bucketStart + bucketWidth;

                var tooltip = string.Format("{0}-{1}\n{2} {3}\n\nBar width: {4}",
                    m_Units.ToTooltipString(bucketStart, false),
                    m_Units.ToTooltipString(bucketEnd, true),
                    count,
                    count == 1 ? "frame" : "frames",
                    m_Units.ToTooltipString(bucketWidth, true)
                );

                var content = new GUIContent("", tooltip);
                GUI.Label(new Rect(rect.x + x, rect.y + y, w, h), content);

                x += w;
                x += spacing;
            }
        }

        public void Draw(float width, float height, int[] buckets, int totalFrameCount, float min, float max, Color barColor)
        {
            DrawStart(width);

            float spacing = 2;

            if (m_2D.DrawStart(width, height, Draw2D.Origin.BottomLeft))
            {
                DrawBackground(width, height, buckets.Length, min, max, spacing);
                DrawData(width, height, buckets, totalFrameCount, min, max, barColor, spacing);
                m_2D.DrawEnd();
            }

            DrawEnd(width, min, max, spacing);
        }
    }
}
