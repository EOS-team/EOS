#if UNITY_2018_3_OR_NEWER
#define SETTINGS_PROVIDER_ENABLED
#endif

using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using System.Reflection;
using UnityEngine;
#if SETTINGS_PROVIDER_ENABLED
#if UNITY_2019_1_OR_NEWER
using UnityEngine.UIElements;
#else
using UnityEngine.Experimental.UIElements;
#endif
#endif

namespace UnityEditor.SettingsManagement
{
    /// <summary>
    /// A <see cref="UnityEditor.SettingsProvider"/> implementation that creates an interface from settings reflected
    /// from a collection of assemblies.
    /// </summary>
#if SETTINGS_PROVIDER_ENABLED
    public sealed class UserSettingsProvider : SettingsProvider
#else
    public sealed class UserSettingsProvider
#endif
    {
        public const string developerModeCategory = "Developer Mode";

        const string k_SettingsName = "UserSettingsProviderSettings";

#if SETTINGS_PROVIDER_ENABLED
        const int k_LabelWidth = 240;

        static int labelWidth
        {
            get
            {
                if (s_DefaultLabelWidth != null)
                    return (int)((float)s_DefaultLabelWidth.GetValue(null, null));

                return k_LabelWidth;
            }
        }

        static int defaultLayoutMaxWidth
        {
            get
            {
                if (s_DefaultLayoutMaxWidth != null)
                    return (int)((float)s_DefaultLayoutMaxWidth.GetValue(null, null));

                return 0;
            }
        }
#else
        const int k_LabelWidth = 180;

        int labelWidth
        {
            get { return k_LabelWidth; }
        }

        int defaultLayoutMaxWidth
        {
            get { return 0; }
        }
#endif

        List<string> m_Categories;
        Dictionary<string, List<PrefEntry>> m_Settings;
        Dictionary<string, List<MethodInfo>> m_SettingBlocks;
#if !SETTINGS_PROVIDER_ENABLED
        HashSet<string> keywords = new HashSet<string>();
#endif
        static readonly string[] s_SearchContext = new string[1];
        EventType m_SettingsBlockKeywordsInitialized;
        Assembly[] m_Assemblies;
        static Settings s_Settings;
        Settings m_SettingsInstance;

#if SETTINGS_PROVIDER_ENABLED
        static PropertyInfo s_DefaultLabelWidth;
        static PropertyInfo s_DefaultLayoutMaxWidth;
#endif

        static Settings userSettingsProviderSettings
        {
            get
            {
                if (s_Settings == null)
                    s_Settings = new Settings(new [] { new UserSettingsRepository() });

                return s_Settings;
            }
        }

        internal static UserSetting<bool> showHiddenSettings = new UserSetting<bool>(userSettingsProviderSettings, "settings.showHidden", false, SettingsScope.User);
        internal static UserSetting<bool> showUnregisteredSettings = new UserSetting<bool>(userSettingsProviderSettings, "settings.showUnregistered", false, SettingsScope.User);
        internal static UserSetting<bool> listByKey = new UserSetting<bool>(userSettingsProviderSettings, "settings.listByKey", false, SettingsScope.User);
        internal static UserSetting<bool> showUserSettings = new UserSetting<bool>(userSettingsProviderSettings, "settings.showUserSettings", true, SettingsScope.User);
        internal static UserSetting<bool> showProjectSettings = new UserSetting<bool>(userSettingsProviderSettings, "settings.showProjectSettings", true, SettingsScope.User);

#if SETTINGS_PROVIDER_ENABLED
        /// <summary>
        /// Create a new UserSettingsProvider.
        /// </summary>
        /// <param name="path">The settings menu path.</param>
        /// <param name="settings">The Settings instance that this provider is inspecting.</param>
        /// <param name="assemblies">A collection of assemblies to scan for <see cref="UserSettingAttribute"/> and <see cref="UserSettingBlockAttribute"/> attributes.</param>
        /// <param name="scopes">Which scopes this provider is valid for.</param>
        /// <exception cref="ArgumentNullException">Thrown if settings or assemblies is null.</exception>
        public UserSettingsProvider(string path, Settings settings, Assembly[] assemblies, SettingsScope scopes = SettingsScope.User)
            : base(path, scopes)
#else
        /// <summary>
        /// Create a new UserSettingsProvider.
        /// </summary>
        /// <param name="settings">The Settings instance that this provider is inspecting.</param>
        /// <param name="assemblies">A collection of assemblies to scan for <see cref="UserSettingAttribute"/> and <see cref="UserSettingBlockAttribute"/> attributes.</param>
        public UserSettingsProvider(Settings settings, Assembly[] assemblies)
#endif
        {
            if (settings == null)
                throw new ArgumentNullException("settings");

            if (assemblies == null)
                throw new ArgumentNullException("assemblies");

            m_SettingsInstance = settings;
            m_Assemblies = assemblies;

#if !SETTINGS_PROVIDER_ENABLED
            SearchForUserSettingAttributes();
#endif
        }

#if SETTINGS_PROVIDER_ENABLED
        /// <summary>
        /// Invoked by the SettingsProvider when activated in the Editor.
        /// </summary>
        /// <param name="searchContext"></param>
        /// <param name="rootElement"></param>
        public override void OnActivate(string searchContext, VisualElement rootElement)
        {
            SearchForUserSettingAttributes();

            var window = GetType().GetProperty("settingsWindow", BindingFlags.Instance | BindingFlags.NonPublic);

            if (window != null)
            {
                s_DefaultLabelWidth = window.PropertyType.GetProperty("s_DefaultLabelWidth", BindingFlags.Public | BindingFlags.Static);
                s_DefaultLayoutMaxWidth = window.PropertyType.GetProperty("s_DefaultLayoutMaxWidth", BindingFlags.Public | BindingFlags.Static);
            }
        }
#endif

        struct PrefEntry
        {
            GUIContent m_Content;
            IUserSetting m_Pref;

            public GUIContent content
            {
                get { return m_Content; }
            }

            public IUserSetting pref
            {
                get { return m_Pref; }
            }

            public PrefEntry(GUIContent content, IUserSetting pref)
            {
                m_Content = content;
                m_Pref = pref;
            }
        }

        void SearchForUserSettingAttributes()
        {
            var isDeveloperMode = EditorPrefs.GetBool("DeveloperMode", false);
            var keywordsHash = new HashSet<string>();

            if (m_Settings != null)
                m_Settings.Clear();
            else
                m_Settings = new Dictionary<string, List<PrefEntry>>();

            if (m_SettingBlocks != null)
                m_SettingBlocks.Clear();
            else
                m_SettingBlocks = new Dictionary<string, List<MethodInfo>>();

            var types = m_Assemblies.SelectMany(x => x.GetTypes());

            // collect instance fields/methods too, but only so we can throw a warning that they're invalid.
            var fields = types.SelectMany(x =>
                    x.GetFields(BindingFlags.Public | BindingFlags.NonPublic | BindingFlags.Instance | BindingFlags.Static)
                    .Where(prop => Attribute.IsDefined(prop, typeof(UserSettingAttribute))));

            var methods = types.SelectMany(x => x.GetMethods(BindingFlags.Instance | BindingFlags.Static | BindingFlags.Public | BindingFlags.NonPublic)
                    .Where(y => Attribute.IsDefined(y, typeof(UserSettingBlockAttribute))));

            foreach (var field in fields)
            {
                if (!field.IsStatic)
                {
                    Debug.LogWarning("Cannot create setting entries for instance fields. Skipping \"" + field.Name + "\".");
                    continue;
                }

                var attrib = (UserSettingAttribute)Attribute.GetCustomAttribute(field, typeof(UserSettingAttribute));

                if (!attrib.visibleInSettingsProvider)
                    continue;

                var pref = (IUserSetting)field.GetValue(null);

                if (pref == null)
                {
                    Debug.LogWarning("[UserSettingAttribute] is only valid for types implementing the IUserSetting interface. Skipping \"" + field.Name + "\"");
                    continue;
                }

                var category = string.IsNullOrEmpty(attrib.category) ? "Uncategorized" : attrib.category;
                var content = listByKey ? new GUIContent(pref.key) : attrib.title;

                if (developerModeCategory.Equals(category) && !isDeveloperMode)
                    continue;

                List<PrefEntry> settings;

                if (m_Settings.TryGetValue(category, out settings))
                    settings.Add(new PrefEntry(content, pref));
                else
                    m_Settings.Add(category, new List<PrefEntry>() { new PrefEntry(content, pref) });
            }

            foreach (var method in methods)
            {
                var attrib = (UserSettingBlockAttribute)Attribute.GetCustomAttribute(method, typeof(UserSettingBlockAttribute));
                var category = string.IsNullOrEmpty(attrib.category) ? "Uncategorized" : attrib.category;

                if (developerModeCategory.Equals(category) && !isDeveloperMode)
                    continue;

                List<MethodInfo> blocks;

                var parameters = method.GetParameters();

                if (!method.IsStatic || parameters.Length < 1 || parameters[0].ParameterType != typeof(string))
                {
                    Debug.LogWarning("[UserSettingBlockAttribute] is only valid for static functions with a single string parameter. Ex, `static void MySettings(string searchContext)`. Skipping \"" + method.Name + "\"");
                    continue;
                }

                if (m_SettingBlocks.TryGetValue(category, out blocks))
                    blocks.Add(method);
                else
                    m_SettingBlocks.Add(category, new List<MethodInfo>() { method });
            }

            if (showHiddenSettings)
            {
                var unlisted = new List<PrefEntry>();
                m_Settings.Add("Unlisted", unlisted);
                foreach (var pref in UserSettings.FindUserSettings(m_Assemblies, SettingVisibility.Unlisted | SettingVisibility.Hidden))
                    unlisted.Add(new PrefEntry(new GUIContent(pref.key), pref));
            }

            if (showUnregisteredSettings)
            {
                var unregistered = new List<PrefEntry>();
                m_Settings.Add("Unregistered", unregistered);
                foreach (var pref in UserSettings.FindUserSettings(m_Assemblies, SettingVisibility.Unregistered))
                    unregistered.Add(new PrefEntry(new GUIContent(pref.key), pref));
            }

            foreach (var cat in m_Settings)
            {
                foreach (var entry in cat.Value)
                {
                    var content = entry.content;

                    if (content != null && !string.IsNullOrEmpty(content.text))
                    {
                        foreach (var word in content.text.Split(' '))
                            keywordsHash.Add(word);
                    }
                }
            }

            keywords = keywordsHash;
            m_Categories = m_Settings.Keys.Union(m_SettingBlocks.Keys).ToList();
            m_Categories.Sort();
        }

#if SETTINGS_PROVIDER_ENABLED
        /// <summary>
        /// Invoked by the SettingsProvider container when drawing the UI header.
        /// </summary>
        public override void OnTitleBarGUI()
        {
            if (GUILayout.Button(GUIContent.none, SettingsGUIStyles.settingsGizmo))
                DoContextMenu();
        }
#endif

        void InitSettingsBlockKeywords()
        {
            // Have to let the blocks run twice - one for Layout, one for Repaint.
            if (m_SettingsBlockKeywordsInitialized == EventType.Repaint)
                return;

            m_SettingsBlockKeywordsInitialized = Event.current.type;

            // Allows SettingsGUILayout.SettingsField to populate keywords
            SettingsGUILayout.s_Keywords = new HashSet<string>(keywords);

            // Set a dummy value so that GUI blocks with conditional foldouts will behave as though searching.
            s_SearchContext[0] = "Search";

            foreach (var category in m_SettingBlocks)
            {
                foreach (var block in category.Value)
                    block.Invoke(null, s_SearchContext);
            }

            keywords = SettingsGUILayout.s_Keywords;
            SettingsGUILayout.s_Keywords = null;
            s_SearchContext[0] = "";
        }

        void DoContextMenu()
        {
            var menu = new GenericMenu();

            menu.AddItem(new GUIContent("Reset All"), false, () =>
                {
                    if (!UnityEditor.EditorUtility.DisplayDialog("Reset All Settings", "Reset all settings? This is not undo-able.", "Reset", "Cancel"))
                        return;

                    // Do not reset SettingVisibility.Unregistered
                    foreach (var pref in UserSettings.FindUserSettings(m_Assemblies, SettingVisibility.Visible | SettingVisibility.Hidden | SettingVisibility.Unlisted))
                        pref.Reset();

                    m_SettingsInstance.Save();
                });

            if (EditorPrefs.GetBool("DeveloperMode", false))
            {
                menu.AddSeparator("");

                menu.AddItem(new GUIContent("Developer/List Settings By Key"), listByKey, () =>
                    {
                        listByKey.SetValue(!listByKey, true);
                        SearchForUserSettingAttributes();
                    });

                menu.AddSeparator("Developer/");

                menu.AddItem(new GUIContent("Developer/Show User Settings"), showUserSettings, () =>
                    {
                        showUserSettings.SetValue(!showUserSettings, true);
                        SearchForUserSettingAttributes();
                    });

                menu.AddItem(new GUIContent("Developer/Show Project Settings"), showProjectSettings, () =>
                    {
                        showProjectSettings.SetValue(!showProjectSettings, true);
                        SearchForUserSettingAttributes();
                    });

                menu.AddSeparator("Developer/");

                menu.AddItem(new GUIContent("Developer/Show Unlisted Settings"), showHiddenSettings, () =>
                    {
                        showHiddenSettings.SetValue(!showHiddenSettings, true);
                        SearchForUserSettingAttributes();
                    });

                menu.AddItem(new GUIContent("Developer/Show Unregistered Settings"), showUnregisteredSettings, () =>
                    {
                        showUnregisteredSettings.SetValue(!showUnregisteredSettings, true);
                        SearchForUserSettingAttributes();
                    });

                menu.AddSeparator("Developer/");

                menu.AddItem(new GUIContent("Developer/Open Project Settings File"), false, () =>
                    {
                        var project = m_SettingsInstance.GetRepository(SettingsScope.Project);

                        if (project != null)
                        {
                            var path = Path.GetFullPath(project.path);
                            System.Diagnostics.Process.Start(path);
                        }
                    });

                menu.AddItem(new GUIContent("Developer/Print All Settings"), false, () =>
                    {
                        Debug.Log(UserSettings.GetSettingsString(m_Assemblies));
                    });

#if UNITY_2019_1_OR_NEWER
                menu.AddSeparator("Developer/");
#if UNITY_2019_3_OR_NEWER
                menu.AddItem(new GUIContent("Developer/Recompile Scripts"), false, EditorUtility.RequestScriptReload);
#else
                menu.AddItem(new GUIContent("Developer/Recompile Scripts"), false, UnityEditorInternal.InternalEditorUtility.RequestScriptReload);
#endif
#endif
            }

            menu.ShowAsContext();
        }

#if SETTINGS_PROVIDER_ENABLED
        /// <summary>
        /// Invoked by the Settings editor.
        /// </summary>
        /// <param name="searchContext">
        /// A string containing the contents of the search bar.
        /// </param>
        public override void OnGUI(string searchContext)
#else
        /// <summary>
        /// Invoked by the Settings editor.
        /// </summary>
        /// <param name="searchContext">
        /// A string containing the contents of the search bar.
        /// </param>
        public void OnGUI(string searchContext)
#endif
        {
#if !SETTINGS_PROVIDER_ENABLED
            var evt = Event.current;
            if (evt.type == EventType.ContextClick)
                DoContextMenu();
#endif
            InitSettingsBlockKeywords();

            EditorGUIUtility.labelWidth = labelWidth;

            EditorGUI.BeginChangeCheck();

            var maxWidth = defaultLayoutMaxWidth;

            if (maxWidth != 0)
                GUILayout.BeginVertical(SettingsGUIStyles.settingsArea, GUILayout.MaxWidth(maxWidth));
            else
                GUILayout.BeginVertical(SettingsGUIStyles.settingsArea);

            var hasSearchContext = !string.IsNullOrEmpty(searchContext);
            s_SearchContext[0] = searchContext;
            if (hasSearchContext)
            {
                // todo - Improve search comparison
                var searchKeywords = searchContext.Split(' ');

                foreach (var settingField in m_Settings)
                {
                    foreach (var setting in settingField.Value)
                    {
                        if (searchKeywords.Any(x => !string.IsNullOrEmpty(x) && setting.content.text.IndexOf(x, StringComparison.InvariantCultureIgnoreCase) > -1))
                            DoPreferenceField(setting.content, setting.pref);
                    }
                }

                foreach (var settingsBlock in m_SettingBlocks)
                {
                    foreach (var block in settingsBlock.Value)
                    {
                        block.Invoke(null, s_SearchContext);
                    }
                }
            }
            else
            {
                foreach (var key in m_Categories)
                {
                    GUILayout.Label(key, EditorStyles.boldLabel);

                    List<PrefEntry> settings;

                    if (m_Settings.TryGetValue(key, out settings))
                        foreach (var setting in settings)
                            DoPreferenceField(setting.content, setting.pref);

                    List<MethodInfo> blocks;

                    if (m_SettingBlocks.TryGetValue(key, out blocks))
                        foreach (var block in blocks)
                            block.Invoke(null, s_SearchContext);

                    GUILayout.Space(8);
                }
            }

            EditorGUIUtility.labelWidth = 0;

            GUILayout.EndVertical();

            if (EditorGUI.EndChangeCheck())
            {
                m_SettingsInstance.Save();
            }
        }

        void DoPreferenceField(GUIContent title, IUserSetting pref)
        {
            if (EditorPrefs.GetBool("DeveloperMode", false))
            {
                if (pref.scope == SettingsScope.Project && !showProjectSettings)
                    return;
                if (pref.scope == SettingsScope.User && !showUserSettings)
                    return;
            }

            if (pref is UserSetting<float>)
            {
                var cast = (UserSetting<float>)pref;
                cast.value = EditorGUILayout.FloatField(title, cast.value);
            }
            else if (pref is UserSetting<int>)
            {
                var cast = (UserSetting<int>)pref;
                cast.value = EditorGUILayout.IntField(title, cast.value);
            }
            else if (pref is UserSetting<bool>)
            {
                var cast = (UserSetting<bool>)pref;
                cast.value = EditorGUILayout.Toggle(title, cast.value);
            }
            else if (pref is UserSetting<string>)
            {
                var cast = (UserSetting<string>)pref;
                cast.value = EditorGUILayout.TextField(title, cast.value);
            }
            else if (pref is UserSetting<Color>)
            {
                var cast = (UserSetting<Color>)pref;
                cast.value = EditorGUILayout.ColorField(title, cast.value);
            }
#if UNITY_2018_3_OR_NEWER
            else if (pref is UserSetting<Gradient>)
            {
                var cast = (UserSetting<Gradient>)pref;
                cast.value = EditorGUILayout.GradientField(title, cast.value);
            }
#endif
            else if (pref is UserSetting<Vector2>)
            {
                var cast = (UserSetting<Vector2>)pref;
                cast.value = EditorGUILayout.Vector2Field(title, cast.value);
            }
            else if (pref is UserSetting<Vector3>)
            {
                var cast = (UserSetting<Vector3>)pref;
                cast.value = EditorGUILayout.Vector3Field(title, cast.value);
            }
            else if (pref is UserSetting<Vector4>)
            {
                var cast = (UserSetting<Vector4>)pref;
                cast.value = EditorGUILayout.Vector4Field(title, cast.value);
            }
            else if (typeof(Enum).IsAssignableFrom(pref.type))
            {
                Enum val = (Enum)pref.GetValue();
                EditorGUI.BeginChangeCheck();
                if (Attribute.IsDefined(pref.type, typeof(FlagsAttribute)))
                    val = EditorGUILayout.EnumFlagsField(title, val);
                else
                    val = EditorGUILayout.EnumPopup(title, val);
                if (EditorGUI.EndChangeCheck())
                    pref.SetValue(val);
            }
            else if (typeof(UnityEngine.Object).IsAssignableFrom(pref.type))
            {
                var obj = (UnityEngine.Object)pref.GetValue();
                EditorGUI.BeginChangeCheck();
                obj = EditorGUILayout.ObjectField(title, obj, pref.type, false);
                if (EditorGUI.EndChangeCheck())
                    pref.SetValue(obj);
            }
            else
            {
                GUILayout.BeginHorizontal();
                GUILayout.Label(title, GUILayout.Width(EditorGUIUtility.labelWidth - EditorStyles.label.margin.right * 2));
                var obj = pref.GetValue();
                GUILayout.Label(obj == null ? "null" : pref.GetValue().ToString());
                GUILayout.EndHorizontal();
            }

            SettingsGUILayout.DoResetContextMenuForLastRect(pref);
        }
    }
}
