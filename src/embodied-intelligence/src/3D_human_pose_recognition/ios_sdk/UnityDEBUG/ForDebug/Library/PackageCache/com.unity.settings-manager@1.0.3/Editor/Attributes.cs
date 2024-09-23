using System;
using UnityEngine;

namespace UnityEditor.SettingsManagement
{
    /// <summary>
    /// Register a static field of type IUserSetting with the UserSettingsProvider window.
    /// </summary>
    [AttributeUsage(AttributeTargets.Field)]
    public sealed class UserSettingAttribute : Attribute
    {
        string m_Category;
        GUIContent m_Title;
        bool m_VisibleInSettingsProvider;

        /// <summary>
        /// Settings that are automatically scraped from assemblies are displayed in groups, organized by category.
        /// </summary>
        /// <value>
        /// The title of the group of settings that this setting will be shown under.
        /// </value>
        public string category
        {
            get { return m_Category; }
        }

        /// <value>
        /// The label to show for this setting.
        /// </value>
        public GUIContent title
        {
            get { return m_Title; }
        }

        /// <value>
        /// True if this field should be shown in the UserSettingsProvider interface, false if not.
        /// </value>
        public bool visibleInSettingsProvider
        {
            get { return m_VisibleInSettingsProvider; }
        }

        /// <summary>
        /// Register a static field as a setting. Field must be of a type implementing IUserSetting.
        /// </summary>
        public UserSettingAttribute()
        {
            m_VisibleInSettingsProvider = false;
        }

        /// <summary>
        /// Register a static field as a setting and create an entry in the UI. Field must be of a type implementing IUserSetting.
        /// </summary>
        public UserSettingAttribute(string category, string title, string tooltip = null)
        {
            m_Category = category;
            m_Title = new GUIContent(title, tooltip);
            m_VisibleInSettingsProvider = true;
        }
    }

    /// <summary>
    /// Register a field with Settings, but do not automatically create a property field in the SettingsProvider.
    /// Unlike UserSettingAttribute, this attribute is valid for instance properties as well as static. These values
    /// will not be shown in the SettingsProvider, but will have their stored values cleared when "Reset All" is invoked.
    /// </summary>
    [AttributeUsage(AttributeTargets.Field)]
    public sealed class SettingsKeyAttribute : Attribute
    {
        string m_Key;
        SettingsScope m_Scope;

        /// <value>
        /// The key for this value.
        /// </value>
        public string key
        {
            get { return m_Key; }
        }

        /// <value>
        /// Where this setting is serialized.
        /// </value>
        public SettingsScope scope
        {
            get { return m_Scope; }
        }

        /// <summary>
        /// Register a field as a setting. This allows the UserSettingsProvider to reset it's value and display it's
        /// value in debugging modes.
        /// </summary>
        /// <param name="key">The setting key.</param>
        /// <param name="scope">The scope in which this setting is serialized.</param>
        public SettingsKeyAttribute(string key, SettingsScope scope = SettingsScope.Project)
        {
            m_Key = key;
            m_Scope = scope;
        }
    }

    /// <summary>
    /// UserSettingBlock allows you add a section of settings to a category.
    /// </summary>
    [AttributeUsage(AttributeTargets.Method)]
    public sealed class UserSettingBlockAttribute : Attribute
    {
        string m_Category;

        /// <summary>
        /// Settings that are automatically scraped from assemblies are displayed in groups, organized by category.
        /// </summary>
        /// <value>
        /// The title of the group of settings that this setting will be shown under.
        /// </value>
        public string category
        {
            get { return m_Category; }
        }

        /// <summary>
        /// Register a static method for a callback in the UserSettingsProvider editor under a category.
        /// <code>
        /// [UserSettingBlock("General")]
        /// static void GeneralSettings(string[] searchContext) {}
        /// </code>
        /// </summary>
        /// <param name="category">The title of the group of settings that this setting will be shown under.</param>
        public UserSettingBlockAttribute(string category)
        {
            m_Category = category;
        }
    }
}
