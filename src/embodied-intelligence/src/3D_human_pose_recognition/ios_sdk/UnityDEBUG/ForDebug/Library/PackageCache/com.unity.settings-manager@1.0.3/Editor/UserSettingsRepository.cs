namespace UnityEditor.SettingsManagement
{
    /// <summary>
    /// A settings repository backed by the UnityEditor.EditorPrefs class.
    /// </summary>
    public class UserSettingsRepository : ISettingsRepository
    {
        static string GetEditorPrefKey<T>(string key)
        {
            return GetEditorPrefKey(typeof(T).FullName, key);
        }

        static string GetEditorPrefKey(string fullName, string key)
        {
            return fullName + "::" + key;
        }

        static void SetEditorPref<T>(string key, T value)
        {
            var k = GetEditorPrefKey<T>(key);

            if (typeof(T) == typeof(string))
                EditorPrefs.SetString(k, (string)(object)value);
            else if (typeof(T) == typeof(bool))
                EditorPrefs.SetBool(k, (bool)(object)value);
            else if (typeof(T) == typeof(float))
                EditorPrefs.SetFloat(k, (float)(object)value);
            else if (typeof(T) == typeof(int))
                EditorPrefs.SetInt(k, (int)(object)value);
            else
                EditorPrefs.SetString(k, ValueWrapper<T>.Serialize(value));
        }

        static T GetEditorPref<T>(string key, T fallback = default(T))
        {
            var k = GetEditorPrefKey<T>(key);

            if (!EditorPrefs.HasKey(k))
                return fallback;

            var o = (object)fallback;

            if (typeof(T) == typeof(string))
                o = EditorPrefs.GetString(k, (string)o);
            else if (typeof(T) == typeof(bool))
                o = EditorPrefs.GetBool(k, (bool)o);
            else if (typeof(T) == typeof(float))
                o = EditorPrefs.GetFloat(k, (float)o);
            else if (typeof(T) == typeof(int))
                o = EditorPrefs.GetInt(k, (int)o);
            else
                return ValueWrapper<T>.Deserialize(EditorPrefs.GetString(k));

            return (T)o;
        }

        /// <value>
        /// What SettingsScope this repository applies to.
        /// </value>
        /// <inheritdoc cref="ISettingsRepository.scope"/>
        public SettingsScope scope
        {
            get { return SettingsScope.User; }
        }

        /// <summary>
        /// An identifying name for this repository. User settings are named "EditorPrefs."
        /// </summary>
        public string name
        {
            get { return "EditorPrefs"; }
        }

        /// <value>
        /// File path to the serialized settings data.
        /// </value>
        /// <remarks>
        /// This property returns an empty string.
        /// </remarks>
        /// <inheritdoc cref="ISettingsRepository.path"/>
        public string path
        {
            get { return string.Empty; }
        }

        /// <summary>
        /// Save all settings to their serialized state.
        /// </summary>
        /// <inheritdoc cref="ISettingsRepository.Save"/>
        public void Save()
        {
        }

        /// <inheritdoc />
        /// <summary>
        /// Set a value for key of type T.
        /// </summary>
        /// <param name="key">The settings key.</param>
        /// <param name="value">The value to set. Must be serializable.</param>
        /// <typeparam name="T">Type of value.</typeparam>
        public void Set<T>(string key, T value)
        {
            SetEditorPref<T>(key, value);
        }

        /// <inheritdoc />
        /// <summary>
        /// Get a value with key of type T, or return the fallback value if no matching key is found.
        /// </summary>
        /// <param name="key">The settings key.</param>
        /// <param name="fallback">If no key with a value of type T is found, this value is returned.</param>
        /// <typeparam name="T">Type of value to search for.</typeparam>
        public T Get<T>(string key, T fallback = default(T))
        {
            return GetEditorPref<T>(key, fallback);
        }

        /// <inheritdoc />
        /// <summary>
        /// Does the repository contain a setting with key and type.
        /// </summary>
        /// <param name="key">The settings key.</param>
        /// <typeparam name="T">The type of value to search for.</typeparam>
        /// <returns>True if a setting matching both key and type is found, false if no entry is found.</returns>
        public bool ContainsKey<T>(string key)
        {
            return EditorPrefs.HasKey(GetEditorPrefKey<T>(key));
        }

        /// <inheritdoc />
        /// <summary>
        /// Remove a key value pair from the settings repository.
        /// </summary>
        /// <param name="key"></param>
        /// <typeparam name="T"></typeparam>
        public void Remove<T>(string key)
        {
            EditorPrefs.DeleteKey(GetEditorPrefKey<T>(key));
        }
    }
}
