namespace UnityEditor.SettingsManagement
{
    /// <summary>
    /// A settings repository is responsible for implementing the saving and loading of values.
    /// </summary>
    public interface ISettingsRepository
    {
        /// <value>
        /// What SettingsScope this repository applies to.
        /// </value>
        SettingsScope scope { get; }

        /// <summary>
        /// A name to identify this repository.
        /// </summary>
        string name { get; }

        /// <value>
        /// File path to the serialized settings data.
        /// </value>
        string path { get; }

        /// <summary>
        /// Save all settings to their serialized state.
        /// </summary>
        void Save();

        /// <summary>
        /// Set a value for key of type T.
        /// </summary>
        /// <param name="key">The settings key.</param>
        /// <param name="value">The value to set. Must be serializable.</param>
        /// <typeparam name="T">Type of value.</typeparam>
        void Set<T>(string key, T value);

        /// <summary>
        /// Get a value with key of type T, or return the fallback value if no matching key is found.
        /// </summary>
        /// <param name="key">The settings key.</param>
        /// <param name="fallback">If no key with a value of type T is found, this value is returned.</param>
        /// <typeparam name="T">Type of value to search for.</typeparam>
        T Get<T>(string key, T fallback = default(T));

        /// <summary>
        /// Does the repository contain a setting with key and type.
        /// </summary>
        /// <param name="key">The settings key.</param>
        /// <typeparam name="T">The type of value to search for.</typeparam>
        /// <returns>True if a setting matching both key and type is found, false if no entry is found.</returns>
        bool ContainsKey<T>(string key);

        /// <summary>
        /// Remove a key value pair from the settings repository.
        /// </summary>
        /// <param name="key"></param>
        /// <typeparam name="T"></typeparam>
        void Remove<T>(string key);
    }
}
