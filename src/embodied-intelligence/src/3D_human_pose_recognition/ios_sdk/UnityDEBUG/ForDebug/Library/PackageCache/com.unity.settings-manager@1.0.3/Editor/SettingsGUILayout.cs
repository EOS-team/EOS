using System;
using System.Collections.Generic;
using System.Linq;
using UnityEngine;

namespace UnityEditor.SettingsManagement
{
    static class SettingsGUIStyles
    {
        const string k_SettingsGearIcon = "Packages/" + UserSettings.packageName + "/Content/Options.png";

        static bool s_Initialized;
        public static GUIStyle s_SettingsGizmo;
        public static GUIStyle s_SettingsArea;
        public static GUIStyle s_IndentedSettingBlock;

        static void Init()
        {
            if (s_Initialized)
                return;

            s_Initialized = true;

            s_SettingsGizmo = new GUIStyle()
            {
                normal = new GUIStyleState()
                {
                    background = AssetDatabase.LoadAssetAtPath<Texture2D>(k_SettingsGearIcon)
                },
                fixedWidth = 14,
                fixedHeight = 14,
                padding = new RectOffset(0, 0, 0, 0),
                margin = new RectOffset(4, 4, 4, 4),
                imagePosition = ImagePosition.ImageOnly
            };

            s_SettingsArea = new GUIStyle()
            {
                margin = new RectOffset(6, 6, 0, 0)
            };

            s_IndentedSettingBlock = new GUIStyle()
            {
                padding = new RectOffset(16, 0, 0, 0)
            };
        }

        public static GUIStyle settingsGizmo
        {
            get
            {
                Init();
                return s_SettingsGizmo;
            }
        }

        public static GUIStyle settingsArea
        {
            get
            {
                Init();
                return s_SettingsArea;
            }
        }

        public static GUIStyle indentedSettingBlock
        {
            get
            {
                Init();
                return s_IndentedSettingBlock;
            }
        }
    }

    /// <summary>
    /// Extension methods for GUILayout that also implement settings-specific functionality.
    /// </summary>
    public static class SettingsGUILayout
    {
        /// <inheritdoc />
        /// <summary>
        /// Create an indented GUI section.
        /// </summary>
        public class IndentedGroup : IDisposable
        {
            bool m_IsDisposed;

            /// <summary>
            /// Create an indented GUI section.
            /// </summary>
            public IndentedGroup()
            {
                EditorGUIUtility.labelWidth -= SettingsGUIStyles.indentedSettingBlock.padding.left - 4;
                GUILayout.BeginVertical(SettingsGUIStyles.indentedSettingBlock);
            }

            /// <summary>
            /// Create an indented GUI section with a header.
            /// </summary>
            public IndentedGroup(string label)
            {
                GUILayout.Label(label);
                EditorGUIUtility.labelWidth -= SettingsGUIStyles.indentedSettingBlock.padding.left - 4;
                GUILayout.BeginVertical(SettingsGUIStyles.indentedSettingBlock);
            }

            /// <inheritdoc />
            /// <summary>
            /// Revert the GUI indent back to it's original value.
            /// </summary>
            public void Dispose()
            {
                if (m_IsDisposed)
                    return;
                m_IsDisposed = true;
                GUILayout.EndVertical();
                EditorGUIUtility.labelWidth += SettingsGUIStyles.indentedSettingBlock.padding.left - 4;
            }
        }

        internal static HashSet<string> s_Keywords = null;

        internal static bool MatchSearchGroups(string searchContext, string label)
        {
            if (s_Keywords != null)
            {
                foreach (var keyword in label.Split(' '))
                    s_Keywords.Add(keyword);
            }

            if (searchContext == null)
                return true;

            var ctx = searchContext.Trim();

            if (string.IsNullOrEmpty(ctx))
                return true;

            var split = searchContext.Split(' ');

            return split.Any(x => !string.IsNullOrEmpty(x) && label.IndexOf(x, StringComparison.InvariantCultureIgnoreCase) > -1);
        }

        internal static bool DebugModeFilter(IUserSetting pref)
        {
            if (!EditorPrefs.GetBool("DeveloperMode", false))
                return true;

            if (pref.scope == SettingsScope.Project && UserSettingsProvider.showProjectSettings)
                return true;

            if (pref.scope == SettingsScope.User && UserSettingsProvider.showUserSettings)
                return true;

            return false;
        }

        /// <summary>
        /// A slider that implements search filtering.
        /// </summary>
        /// <param name="label">Label in front of the value field.</param>
        /// <param name="value">The value to edit.</param>
        /// <param name="min">The value at the left end of the slider.</param>
        /// <param name="max">The value at the right end of the slider.</param>
        /// <param name="searchContext">A string representing the current search query. Empty or null strings are to be treated as matching any value.</param>
        /// <returns>The value that has been set by the user.</returns>
        public static float SearchableSlider(GUIContent label, float value, float min, float max, string searchContext)
        {
            if (!MatchSearchGroups(searchContext, label.text))
                return value;
            return EditorGUILayout.Slider(label, value, min, max);
        }

        /// <summary>
        /// A slider that implements search filtering.
        /// </summary>
        /// <param name="label">Label in front of the value field.</param>
        /// <param name="value">The value to edit.</param>
        /// <param name="min">The value at the left end of the slider.</param>
        /// <param name="max">The value at the right end of the slider.</param>
        /// <param name="searchContext">A string representing the current search query. Empty or null strings are to be treated as matching any value.</param>
        /// <returns>The value that has been set by the user.</returns>
        public static float SearchableSlider(string label, float value, float min, float max, string searchContext)
        {
            if (!MatchSearchGroups(searchContext, label))
                return value;
            return EditorGUILayout.Slider(label, value, min, max);
        }

        /// <summary>
        /// A float field that implements search filtering.
        /// </summary>
        /// <param name="label">Label in front of the value field.</param>
        /// <param name="value">The value to edit.</param>
        /// <param name="searchContext">A string representing the current search query. Empty or null strings are to be treated as matching any value.</param>
        /// <returns>The value that has been set by the user.</returns>
        public static float SearchableFloatField(GUIContent label, float value, string searchContext)
        {
            if (!MatchSearchGroups(searchContext, label.text))
                return value;
            return EditorGUILayout.FloatField(label, value);
        }

        /// <summary>
        /// A float field that implements search filtering.
        /// </summary>
        /// <param name="label">Label in front of the value field.</param>
        /// <param name="value">The value to edit.</param>
        /// <param name="searchContext">A string representing the current search query. Empty or null strings are to be treated as matching any value.</param>
        /// <returns>The value that has been set by the user.</returns>
        public static float SearchableFloatField(string label, float value, string searchContext)
        {
            if (!MatchSearchGroups(searchContext, label))
                return value;
            return EditorGUILayout.FloatField(label, value);
        }

        /// <summary>
        /// An int field that implements search filtering.
        /// </summary>
        /// <param name="label">Label in front of the value field.</param>
        /// <param name="value">The value to edit.</param>
        /// <param name="searchContext">A string representing the current search query. Empty or null strings are to be treated as matching any value.</param>
        /// <returns>The value that has been set by the user.</returns>
        public static int SearchableIntField(GUIContent label, int value, string searchContext)
        {
            if (!MatchSearchGroups(searchContext, label.text))
                return value;
            return EditorGUILayout.IntField(label, value);
        }

        /// <summary>
        /// An int field that implements search filtering.
        /// </summary>
        /// <param name="label">Label in front of the value field.</param>
        /// <param name="value">The value to edit.</param>
        /// <param name="searchContext">A string representing the current search query. Empty or null strings are to be treated as matching any value.</param>
        /// <returns>The value that has been set by the user.</returns>
        public static int SearchableIntField(string label, int value, string searchContext)
        {
            if (!MatchSearchGroups(searchContext, label))
                return value;
            return EditorGUILayout.IntField(label, value);
        }

        /// <summary>
        /// An toggle field that implements search filtering.
        /// </summary>
        /// <param name="label">Label in front of the value field.</param>
        /// <param name="value">The value to edit.</param>
        /// <param name="searchContext">A string representing the current search query. Empty or null strings are to be treated as matching any value.</param>
        /// <returns>The value that has been set by the user.</returns>
        public static bool SearchableToggle(GUIContent label, bool value, string searchContext)
        {
            if (!MatchSearchGroups(searchContext, label.text))
                return value;
            return EditorGUILayout.Toggle(label, value);
        }

        /// <summary>
        /// An toggle field that implements search filtering.
        /// </summary>
        /// <param name="label">Label in front of the value field.</param>
        /// <param name="value">The value to edit.</param>
        /// <param name="searchContext">A string representing the current search query. Empty or null strings are to be treated as matching any value.</param>
        /// <returns>The value that has been set by the user.</returns>
        public static bool SearchableToggle(string label, bool value, string searchContext)
        {
            if (!MatchSearchGroups(searchContext, label))
                return value;
            return EditorGUILayout.Toggle(label, value);
        }

        /// <summary>
        /// An text field that implements search filtering.
        /// </summary>
        /// <param name="label">Label in front of the value field.</param>
        /// <param name="value">The value to edit.</param>
        /// <param name="searchContext">A string representing the current search query. Empty or null strings are to be treated as matching any value.</param>
        /// <returns>The value that has been set by the user.</returns>
        public static string SearchableTextField(GUIContent label, string value, string searchContext)
        {
            if (!MatchSearchGroups(searchContext, label.text))
                return value;
            return EditorGUILayout.TextField(label, value);
        }

        /// <summary>
        /// An text field that implements search filtering.
        /// </summary>
        /// <param name="label">Label in front of the value field.</param>
        /// <param name="value">The value to edit.</param>
        /// <param name="searchContext">A string representing the current search query. Empty or null strings are to be treated as matching any value.</param>
        /// <returns>The value that has been set by the user.</returns>
        public static string SearchableTextField(string label, string value, string searchContext)
        {
            if (!MatchSearchGroups(searchContext, label))
                return value;
            return EditorGUILayout.TextField(label, value);
        }

        /// <summary>
        /// An color field that implements search filtering.
        /// </summary>
        /// <param name="label">Label in front of the value field.</param>
        /// <param name="value">The value to edit.</param>
        /// <param name="searchContext">A string representing the current search query. Empty or null strings are to be treated as matching any value.</param>
        /// <returns>The value that has been set by the user.</returns>
        public static Color SearchableColorField(GUIContent label, Color value, string searchContext)
        {
            if (!MatchSearchGroups(searchContext, label.text))
                return value;
            return EditorGUILayout.ColorField(label, value);
        }

        /// <summary>
        /// An color field that implements search filtering.
        /// </summary>
        /// <param name="label">Label in front of the value field.</param>
        /// <param name="value">The value to edit.</param>
        /// <param name="searchContext">A string representing the current search query. Empty or null strings are to be treated as matching any value.</param>
        /// <returns>The value that has been set by the user.</returns>
        public static Color SearchableColorField(string label, Color value, string searchContext)
        {
            if (!MatchSearchGroups(searchContext, label))
                return value;
            return EditorGUILayout.ColorField(label, value);
        }

        /// <summary>
        /// A slider that implements search filtering and context menu reset.
        /// </summary>
        /// <param name="label">Label in front of the value field.</param>
        /// <param name="value">The value to edit.</param>
        /// <param name="min">The value at the left end of the slider.</param>
        /// <param name="max">The value at the right end of the slider.</param>
        /// <param name="searchContext">A string representing the current search query. Empty or null strings are to be treated as matching any value.</param>
        /// <returns>The value that has been set by the user.</returns>
        public static float SettingsSlider(GUIContent label, UserSetting<float> value, float min, float max, string searchContext)
        {
            if (!DebugModeFilter(value) || !MatchSearchGroups(searchContext, label.text))
                return value;
            var res = EditorGUILayout.Slider(label, value, min, max);
            DoResetContextMenuForLastRect(value);
            return res;
        }

        /// <summary>
        /// A slider that implements search filtering and context menu reset.
        /// </summary>
        /// <param name="label">Label in front of the value field.</param>
        /// <param name="value">The value to edit.</param>
        /// <param name="min">The value at the left end of the slider.</param>
        /// <param name="max">The value at the right end of the slider.</param>
        /// <param name="searchContext">A string representing the current search query. Empty or null strings are to be treated as matching any value.</param>
        /// <returns>The value that has been set by the user.</returns>
        public static float SettingsSlider(string label, UserSetting<float> value, float min, float max, string searchContext)
        {
            if (!DebugModeFilter(value) || !MatchSearchGroups(searchContext, label))
                return value;
            var res = EditorGUILayout.Slider(label, value, min, max);
            DoResetContextMenuForLastRect(value);
            return res;
        }

        /// <summary>
        /// A slider that implements search filtering and context menu reset.
        /// </summary>
        /// <param name="label">Label in front of the value field.</param>
        /// <param name="value">The value to edit.</param>
        /// <param name="min">The value at the left end of the slider.</param>
        /// <param name="max">The value at the right end of the slider.</param>
        /// <param name="searchContext">A string representing the current search query. Empty or null strings are to be treated as matching any value.</param>
        /// <returns>The value that has been set by the user.</returns>
        public static int SettingsSlider(GUIContent label, UserSetting<int> value, int min, int max, string searchContext)
        {
            if (!DebugModeFilter(value) || !MatchSearchGroups(searchContext, label.text))
                return value;
            var res = EditorGUILayout.IntSlider(label, value, min, max);
            DoResetContextMenuForLastRect(value);
            return res;
        }

        /// <summary>
        /// A slider that implements search filtering and context menu reset.
        /// </summary>
        /// <param name="label">Label in front of the value field.</param>
        /// <param name="value">The value to edit.</param>
        /// <param name="min">The value at the left end of the slider.</param>
        /// <param name="max">The value at the right end of the slider.</param>
        /// <param name="searchContext">A string representing the current search query. Empty or null strings are to be treated as matching any value.</param>
        /// <returns>The value that has been set by the user.</returns>
        public static int SettingsSlider(string label, UserSetting<int> value, int min, int max, string searchContext)
        {
            if (!DebugModeFilter(value) || !MatchSearchGroups(searchContext, label))
                return value;
            var res = EditorGUILayout.IntSlider(label, value, min, max);
            DoResetContextMenuForLastRect(value);
            return res;
        }

        /// <summary>
        /// A float field that implements search filtering and context menu reset.
        /// </summary>
        /// <param name="label">Label in front of the value field.</param>
        /// <param name="value">The value to edit.</param>
        /// <param name="searchContext">A string representing the current search query. Empty or null strings are to be treated as matching any value.</param>
        /// <returns>The value that has been set by the user.</returns>
        public static float SettingsFloatField(GUIContent label, UserSetting<float> value, string searchContext)
        {
            if (!DebugModeFilter(value) || !MatchSearchGroups(searchContext, label.text))
                return value;
            var res = EditorGUILayout.FloatField(label, value);
            DoResetContextMenuForLastRect(value);
            return res;
        }

        /// <summary>
        /// A float field that implements search filtering and context menu reset.
        /// </summary>
        /// <param name="label">Label in front of the value field.</param>
        /// <param name="value">The value to edit.</param>
        /// <param name="searchContext">A string representing the current search query. Empty or null strings are to be treated as matching any value.</param>
        /// <returns>The value that has been set by the user.</returns>
        public static float SettingsFloatField(string label, UserSetting<float> value, string searchContext)
        {
            if (!DebugModeFilter(value) || !MatchSearchGroups(searchContext, label))
                return value;
            var res = EditorGUILayout.FloatField(label, value);
            DoResetContextMenuForLastRect(value);
            return res;
        }

        /// <summary>
        /// An integer field that implements search filtering and context menu reset.
        /// </summary>
        /// <param name="label">Label in front of the value field.</param>
        /// <param name="value">The value to edit.</param>
        /// <param name="searchContext">A string representing the current search query. Empty or null strings are to be treated as matching any value.</param>
        /// <returns>The value that has been set by the user.</returns>
        public static int SettingsIntField(GUIContent label, UserSetting<int> value, string searchContext)
        {
            if (!DebugModeFilter(value) || !MatchSearchGroups(searchContext, label.text))
                return value;
            var res = EditorGUILayout.IntField(label, value);
            DoResetContextMenuForLastRect(value);
            return res;
        }

        /// <summary>
        /// An integer field that implements search filtering and context menu reset.
        /// </summary>
        /// <param name="label">Label in front of the value field.</param>
        /// <param name="value">The value to edit.</param>
        /// <param name="searchContext">A string representing the current search query. Empty or null strings are to be treated as matching any value.</param>
        /// <returns>The value that has been set by the user.</returns>
        public static int SettingsIntField(string label, UserSetting<int> value, string searchContext)
        {
            if (!DebugModeFilter(value) || !MatchSearchGroups(searchContext, label))
                return value;
            var res = EditorGUILayout.IntField(label, value);
            DoResetContextMenuForLastRect(value);
            return res;
        }

        /// <summary>
        /// A boolean toggle field that implements search filtering and context menu reset.
        /// </summary>
        /// <param name="label">Label in front of the value field.</param>
        /// <param name="value">The value to edit.</param>
        /// <param name="searchContext">A string representing the current search query. Empty or null strings are to be treated as matching any value.</param>
        /// <returns>The value that has been set by the user.</returns>
        public static bool SettingsToggle(GUIContent label, UserSetting<bool> value, string searchContext)
        {
            if (!DebugModeFilter(value) || !MatchSearchGroups(searchContext, label.text))
                return value;
            var res = EditorGUILayout.Toggle(label, value);
            DoResetContextMenuForLastRect(value);
            return res;
        }

        /// <summary>
        /// A boolean toggle field that implements search filtering and context menu reset.
        /// </summary>
        /// <param name="label">Label in front of the value field.</param>
        /// <param name="value">The value to edit.</param>
        /// <param name="searchContext">A string representing the current search query. Empty or null strings are to be treated as matching any value.</param>
        /// <returns>The value that has been set by the user.</returns>
        public static bool SettingsToggle(string label, UserSetting<bool> value, string searchContext)
        {
            if (!DebugModeFilter(value) || !MatchSearchGroups(searchContext, label))
                return value;
            var res = EditorGUILayout.Toggle(label, value);
            DoResetContextMenuForLastRect(value);
            return res;
        }

        /// <summary>
        /// A text field that implements search filtering and context menu reset.
        /// </summary>
        /// <param name="label">Label in front of the value field.</param>
        /// <param name="value">The value to edit.</param>
        /// <param name="searchContext">A string representing the current search query. Empty or null strings are to be treated as matching any value.</param>
        /// <returns>The value that has been set by the user.</returns>
        public static string SettingsTextField(GUIContent label, UserSetting<string> value, string searchContext)
        {
            if (!DebugModeFilter(value) || !MatchSearchGroups(searchContext, label.text))
                return value;
            var res = EditorGUILayout.TextField(label, value);
            DoResetContextMenuForLastRect(value);
            return res;
        }

        /// <summary>
        /// A text field that implements search filtering and context menu reset.
        /// </summary>
        /// <param name="label">Label in front of the value field.</param>
        /// <param name="value">The value to edit.</param>
        /// <param name="searchContext">A string representing the current search query. Empty or null strings are to be treated as matching any value.</param>
        /// <returns>The value that has been set by the user.</returns>
        public static string SettingsTextField(string label, UserSetting<string> value, string searchContext)
        {
            if (!DebugModeFilter(value) || !MatchSearchGroups(searchContext, label))
                return value;
            var res = EditorGUILayout.TextField(label, value);
            DoResetContextMenuForLastRect(value);
            return res;
        }

        /// <summary>
        /// A color field that implements search filtering and context menu reset.
        /// </summary>
        /// <param name="label">Label in front of the value field.</param>
        /// <param name="value">The value to edit.</param>
        /// <param name="searchContext">A string representing the current search query. Empty or null strings are to be treated as matching any value.</param>
        /// <returns>The value that has been set by the user.</returns>
        public static Color SettingsColorField(GUIContent label, UserSetting<Color> value, string searchContext)
        {
            if (!DebugModeFilter(value) || !MatchSearchGroups(searchContext, label.text))
                return value;
            var res = EditorGUILayout.ColorField(label, value);
            DoResetContextMenuForLastRect(value);
            return res;
        }

        /// <summary>
        /// A color field that implements search filtering and context menu reset.
        /// </summary>
        /// <param name="label">Label in front of the value field.</param>
        /// <param name="value">The value to edit.</param>
        /// <param name="searchContext">A string representing the current search query. Empty or null strings are to be treated as matching any value.</param>
        /// <returns>The value that has been set by the user.</returns>
        public static Color SettingsColorField(string label, UserSetting<Color> value, string searchContext)
        {
            if (!DebugModeFilter(value) || !MatchSearchGroups(searchContext, label))
                return value;
            var res = EditorGUILayout.ColorField(label, value);
            DoResetContextMenuForLastRect(value);
            return res;
        }

        /// <summary>
        /// Using the last automatically layoutted rect, implement a context click menu for a user setting.
        /// </summary>
        /// <param name="setting">The target setting for the reset context menu.</param>
        public static void DoResetContextMenuForLastRect(IUserSetting setting)
        {
            DoResetContextMenu(GUILayoutUtility.GetLastRect(), setting);
        }

        static void DoResetContextMenu(Rect rect, IUserSetting pref)
        {
            var evt = Event.current;

            if (evt.type == EventType.ContextClick && rect.Contains(evt.mousePosition))
            {
                var menu = new GenericMenu();
                menu.AddItem(new GUIContent("Reset [" + pref.scope + "] " + pref.key), false, () =>
                    {
                        pref.Reset(true);
                    });
                menu.ShowAsContext();
            }
        }
    }
}
