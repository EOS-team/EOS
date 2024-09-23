using UnityEngine;

namespace UnityEditor.Performance.ProfileAnalyzer
{
    internal class ProgressBarDisplay
    {
        int m_TotalFrames;
        int m_CurrentFrame;
        string m_Title;
        string m_Description;

        public void InitProgressBar(string title, string description, int frames)
        {
            m_CurrentFrame = 0;
            m_TotalFrames = frames;

            m_Title = title;
            m_Description = description;

            EditorUtility.DisplayProgressBar(m_Title, m_Description, m_CurrentFrame);
        }

        public void AdvanceProgressBar()
        {
            m_CurrentFrame++;
            int currentFrame = Mathf.Clamp(0, m_CurrentFrame, m_TotalFrames);
            float progress = m_TotalFrames > 0 ? (float)currentFrame / m_TotalFrames : 0f;
            EditorUtility.DisplayProgressBar(m_Title, m_Description, progress);
        }

        public void ClearProgressBar()
        {
            EditorUtility.ClearProgressBar();
        }
    }
}
