using System;
using UnityEngine;

namespace UnityEditor.SettingsManagement
{
    [Flags]
    enum SettingVisibility
    {
        None = 0 << 0,

        /// <value>
        /// Matches any static field implementing IUserSetting and tagged with [UserSettingAttribute(visibleInSettingsProvider = true)].
        /// </value>
        /// <summary>
        /// These fields are automatically scraped by the SettingsProvider and displayed.
        /// </summary>
        Visible = 1 << 0,

        /// <value>
        /// Matches any static field implementing IUserSetting and tagged with [UserSettingAttribute(visibleInSettingsProvider = false)].
        /// </value>
        /// <summary>
        /// These fields will be reset by the "Reset All" menu in SettingsProvider, but are not shown in the interface.
        /// Typically these fields require some conditional formatting or data handling, and are shown in the
        /// SettingsProvider UI with a [UserSettingBlockAttribute].
        /// </summary>
        Hidden = 1 << 1,

        /// <value>
        /// A static or instance field tagged with [SettingsKeyAttribute].
        /// </value>
        /// <summary>
        /// Unlisted settings are not shown in the SettingsProvider, but are reset to default values by the "Reset All"
        /// context menu.
        /// </summary>
        Unlisted = 1 << 2,

        /// <value>
        /// A static field implementing IUserSetting that is not marked with any setting attribute.
        /// </value>
        /// <summary>
        /// Unregistered IUserSetting fields are not affected by the SettingsProvider.
        /// </summary>
        Unregistered = 1 << 3,

        All = Visible | Hidden | Unlisted | Unregistered
    }

    /// <summary>
    /// Types implementing IUserSetting are eligible for use with <see cref="UserSettingAttribute"/>, which enables
    /// fields to automatically populate the <see cref="UserSettingsProvider"/> interface.
    /// </summary>
    public interface IUserSetting
    {
        /// <value>
        /// The key for this value.
        /// </value>
        string key { get; }

        /// <value>
        /// The type of the stored value.
        /// </value>
        Type type { get; }

        /// <value>
        /// At which scope this setting is saved.
        /// </value>
        SettingsScope scope { get; }

        /// <summary>
        /// The name of the <see cref="ISettingsRepository"/> that this setting should be associated with. If null, the
        /// first repository matching the <see cref="scope"/> will be used.
        /// </summary>
        string settingsRepositoryName { get; }

        /// <value>
        /// The <see cref="Settings"/> instance that this setting should be saved and loaded from.
        /// </value>
        Settings settings { get; }

        /// <summary>
        /// Get the stored value.
        /// If you are implementing IUserSetting it is recommended that you cache this value.
        /// </summary>
        /// <returns>
        /// The stored value.
        /// </returns>
        object GetValue();

        /// <summary>
        /// Get the default value for this setting.
        /// </summary>
        /// <returns>
        /// The default value for this setting.
        /// </returns>
        object GetDefaultValue();

        /// <summary>
        /// Set the value for this setting.
        /// </summary>
        /// <param name="value">The new value.</param>
        /// <param name="saveProjectSettingsImmediately">
        /// True to immediately serialize the ISettingsRepository that is backing this value, or false to postpone.
        /// If not serializing immediately, be sure to call <see cref="Settings.Save"/>.
        /// </param>
        void SetValue(object value, bool saveProjectSettingsImmediately = false);

        /// <summary>
        /// When the inspected type is a reference value, it is possible to change properties without affecting the
        /// backing setting. ApplyModifiedProperties provides a method to force serialize these changes.
        /// </summary>
        void ApplyModifiedProperties();

        /// <summary>
        /// Set the current value back to the default.
        /// </summary>
        /// <param name="saveProjectSettingsImmediately">True to immediately re-serialize project settings.</param>
        void Reset(bool saveProjectSettingsImmediately = false);

        /// <summary>
        /// Delete the saved setting. Does not clear the current value.
        /// </summary>
        /// <see cref="Reset"/>
        /// <param name="saveProjectSettingsImmediately">True to immediately re-serialize project settings.</param>
        void Delete(bool saveProjectSettingsImmediately = false);
    }

    /// <summary>
    /// A generic implementation of IUserSetting to be used with a <see cref="Settings"/> instance. This default
    /// implementation assumes the <see cref="Settings"/> instance contains two <see cref="ISettingsRepository"/>, one
    /// for <see cref="SettingsScope.Project"/> and one for <see cref="SettingsScope.User"/>.
    /// </summary>
    /// <typeparam name="T"></typeparam>
    /// <inheritdoc />
    public class UserSetting<T> : IUserSetting
    {
        bool m_Initialized;
        string m_Key;
        string m_Repository;
        T m_Value;
        T m_DefaultValue;
        SettingsScope m_Scope;
        Settings m_Settings;

        UserSetting() {}

        /// <summary>
        /// Constructor for UserSetting{T} type.
        /// </summary>
        /// <param name="settings">The <see cref="Settings"/> instance that this setting should be saved and loaded from.</param>
        /// <param name="key">The key for this value.</param>
        /// <param name="value">The default value for this key.</param>
        /// <param name="scope">The scope at which to save this setting.</param>
        public UserSetting(Settings settings, string key, T value, SettingsScope scope = SettingsScope.Project)
        {
            m_Key = key;
            m_Repository = null;
            m_Value = value;
            m_Scope = scope;
            m_Initialized = false;
            m_Settings = settings;
        }

        /// <summary>
        /// Constructor for UserSetting{T} type.
        /// </summary>
        /// <param name="settings">The <see cref="Settings"/> instance that this setting should be saved and loaded from.</param>
        /// <param name="repository">The <see cref="ISettingsRepository"/> name that this setting should be saved and loaded from. Pass null to save to first available instance.</param>
        /// <param name="key">The key for this value.</param>
        /// <param name="value">The default value for this key.</param>
        /// <param name="scope">The scope at which to save this setting.</param>
        public UserSetting(Settings settings, string repository, string key, T value, SettingsScope scope = SettingsScope.Project)
        {
            m_Key = key;
            m_Repository = repository;
            m_Value = value;
            m_Scope = scope;
            m_Initialized = false;
            m_Settings = settings;
        }

        /// <value>
        /// The key for this value.
        /// </value>
        /// <inheritdoc />
        public string key
        {
            get { return m_Key; }
        }

        /// <value>
        /// The name of the repository that this setting is saved in.
        /// </value>
        /// <inheritdoc />
        public string settingsRepositoryName
        {
            get { return m_Repository; }
        }

        /// <value>
        /// The type that this setting represents ({T}).
        /// </value>
        /// <inheritdoc />
        public Type type
        {
            get { return typeof(T); }
        }

        /// <summary>
        /// Get a copy of the default value.
        /// </summary>
        /// <returns>
        /// The default value.
        /// </returns>
        /// <inheritdoc />
        public object GetDefaultValue()
        {
            return defaultValue;
        }

        /// <summary>
        /// Get the currently stored value.
        /// </summary>
        /// <returns>
        /// The value that is currently set.
        /// </returns>
        /// <inheritdoc />
        public object GetValue()
        {
            return value;
        }

        /// <summary>
        /// The scope affects which <see cref="ISettingsRepository"/> the <see cref="settings"/> instance will save
        /// it's data to.
        /// </summary>
        /// <value>
        /// The scope at which to save this key and value.
        /// </value>
        /// <inheritdoc />
        public SettingsScope scope
        {
            get { return m_Scope; }
        }

        /// <value>
        /// The <see cref="Settings"/> instance that this setting will be read from and saved to.
        /// </value>
        /// <inheritdoc />
        public Settings settings
        {
            get { return m_Settings; }
        }

        /// <summary>
        /// Set the value for this setting.
        /// </summary>
        /// <param name="value">The new value.</param>
        /// <param name="saveProjectSettingsImmediately">
        /// True to immediately serialize the ISettingsRepository that is backing this value, or false to postpone.
        /// If not serializing immediately, be sure to call <see cref="Settings.Save"/>.
        /// </param>
        /// <inheritdoc />
        public void SetValue(object value, bool saveProjectSettingsImmediately = false)
        {
            // we do want to allow null values
            if (value != null && !(value is T))
                throw new ArgumentException("Value must be of type " + typeof(T) + "\n" + key + " expecting value of type " + type + ", received " + value.GetType());
            SetValue((T)value, saveProjectSettingsImmediately);
        }

        /// <summary>
        /// Set the value for this setting.
        /// </summary>
        /// <param name="value">The new value.</param>
        /// <param name="saveProjectSettingsImmediately">
        /// True to immediately serialize the ISettingsRepository that is backing this value, or false to postpone.
        /// If not serializing immediately, be sure to call <see cref="Settings.Save"/>.
        /// </param>
        public void SetValue(T value, bool saveProjectSettingsImmediately = false)
        {
            Init();
            m_Value = value;
            settings.Set<T>(key, m_Value, m_Scope);

            if (saveProjectSettingsImmediately)
                settings.Save();
        }

        /// <summary>
        /// Delete the saved setting. Does not clear the current value.
        /// </summary>
        /// <see cref="M:UnityEditor.SettingsManagement.UserSetting`1.Reset(System.Boolean)" />
        /// <param name="saveProjectSettingsImmediately">True to immediately re-serialize project settings.</param>
        /// <inheritdoc cref="IUserSetting.Delete"/>
        public void Delete(bool saveProjectSettingsImmediately = false)
        {
            settings.DeleteKey<T>(key, scope);
            // Don't Init() because that will set the key again. We just want to reset the m_Value with default and
            // pretend that this field hasn't been initialised yet.
            m_Value = ValueWrapper<T>.DeepCopy(m_DefaultValue);
            m_Initialized = false;
        }

        /// <summary>
        /// When the inspected type is a reference value, it is possible to change properties without affecting the
        /// backing setting. ApplyModifiedProperties provides a method to force serialize these changes.
        /// </summary>
        /// <inheritdoc cref="IUserSetting.ApplyModifiedProperties"/>
        public void ApplyModifiedProperties()
        {
            settings.Set<T>(key, m_Value, m_Scope);
            settings.Save();
        }

        /// <summary>
        /// Set the current value back to the default.
        /// </summary>
        /// <param name="saveProjectSettingsImmediately">True to immediately re-serialize project settings.</param>
        /// <inheritdoc cref="IUserSetting.Reset"/>
        public void Reset(bool saveProjectSettingsImmediately = false)
        {
            SetValue(defaultValue, saveProjectSettingsImmediately);
        }

        void Init()
        {
            if (!m_Initialized)
            {
                if (m_Scope == SettingsScope.Project && settings == null)
                    throw new Exception("UserSetting \"" + m_Key + "\" is attempting to access SettingsScope.Project setting with no Settings instance!");

                m_Initialized = true;

                // DeepCopy uses EditorJsonUtility which is not permitted during construction
                m_DefaultValue = ValueWrapper<T>.DeepCopy(m_Value);

                if (settings.ContainsKey<T>(m_Key, m_Scope))
                    m_Value = settings.Get<T>(m_Key, m_Scope);
                else
                    settings.Set<T>(m_Key, m_Value, m_Scope);
            }
        }

        /// <value>
        /// The default value for this setting.
        /// </value>
        public T defaultValue
        {
            get
            {
                Init();
                return ValueWrapper<T>.DeepCopy(m_DefaultValue);
            }
        }

        /// <value>
        /// The currently stored value.
        /// </value>
        public T value
        {
            get
            {
                Init();
                return m_Value;
            }

            set { SetValue(value); }
        }

        /// <summary>
        /// Implicit cast to backing type.
        /// </summary>
        /// <param name="pref">The UserSetting{T} to cast to {T}.</param>
        /// <returns>
        /// The currently stored <see cref="value"/>.
        /// </returns>
        public static implicit operator T(UserSetting<T> pref)
        {
            return pref.value;
        }

        /// <summary>
        /// Get a summary of this setting.
        /// </summary>
        /// <returns>A string summary of this setting.</returns>
        public override string ToString()
        {
            return string.Format("{0} setting. Key: {1}  Value: {2}", scope, key, value);
        }
    }
}
