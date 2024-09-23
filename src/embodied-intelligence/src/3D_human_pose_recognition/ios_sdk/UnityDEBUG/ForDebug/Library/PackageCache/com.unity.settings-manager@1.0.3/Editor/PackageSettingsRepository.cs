using System;
using System.IO;
using UnityEngine;

namespace UnityEditor.SettingsManagement
{
    /// <inheritdoc />
    /// <summary>
    /// A settings repository that stores data local to a Unity project.
    /// </summary>
    [Serializable]
    public sealed class PackageSettingsRepository : ISettingsRepository
    {
        const string k_PackageSettingsDirectory = "ProjectSettings/Packages";
        const bool k_PrettyPrintJson = true;

        bool m_Initialized;

        [SerializeField]
        string m_Name;

        [SerializeField]
        string m_Path;

        [SerializeField]
        SettingsDictionary m_Dictionary = new SettingsDictionary();

        string m_cachedJson;

        /// <summary>
        /// Constructor sets the serialized data path.
        /// </summary>
        /// <param name="package">
        /// The package name.
        /// </param>
        /// <param name="name">
        /// A name for this settings file. Settings are saved in `ProjectSettings/Packages/{package}/{name}.json`.
        /// </param>
        public PackageSettingsRepository(string package, string name)
        {
            m_Name = name;
            m_Path = GetSettingsPath(package, name);
            m_Initialized = false;

            AssemblyReloadEvents.beforeAssemblyReload += Save;
            EditorApplication.quitting += Save;
        }

        void Init()
        {
            if (m_Initialized)
                return;

            m_Initialized = true;

            if (File.Exists(path))
            {
                m_Dictionary = null;
                m_cachedJson = File.ReadAllText(path);
                EditorJsonUtility.FromJsonOverwrite(m_cachedJson, this);
                if (m_Dictionary == null)
                    m_Dictionary = new SettingsDictionary();
            }
        }

        /// <value>
        /// This repository implementation is relevant to the Project scope.
        /// </value>
        /// <inheritdoc cref="ISettingsRepository.scope"/>
        public SettingsScope scope
        {
            get { return SettingsScope.Project; }
        }

        /// <value>
        /// The full path to the settings file.
        /// This corresponds to `Unity Project/Project Settings/Packages/com.unity.package/name`.
        /// </value>
        /// <inheritdoc cref="ISettingsRepository.path"/>
        public string path
        {
            get { return m_Path; }
        }

        /// <summary>
        /// The name of this settings file.
        /// </summary>
        public string name
        {
            get { return m_Name; }
        }

        // Cannot call FindFromAssembly from a constructor or field initializer
//        static string CreateSettingsPath(Assembly assembly, string name)
//        {
//            var info = PackageManager.PackageInfo.FindForAssembly(assembly);
//            return string.Format("{0}/{1}/{2}.json", k_PackageSettingsDirectory, info.name, name);
//        }

        /// <summary>
        /// Get a path for a settings file relative to the calling assembly package directory.
        /// </summary>
        /// <param name="packageName">The name of the package requesting this setting.</param>
        /// <param name="name">An optional name for the settings file. Default is "Settings."</param>
        /// <returns>A package-scoped path to the settings file within Project Settings.</returns>
        public static string GetSettingsPath(string packageName, string name = "Settings")
        {
            return string.Format("{0}/{1}/{2}.json", k_PackageSettingsDirectory, packageName, name);
        }

        /// <summary>
        /// Save all settings to their serialized state.
        /// </summary>
        /// <inheritdoc cref="ISettingsRepository.Save"/>

        public void Save()
        {
            Init();

            if (!File.Exists(path))
            {
                var directory = Path.GetDirectoryName(path);
                Directory.CreateDirectory(directory);
            }

            string newSettingsJson = EditorJsonUtility.ToJson(this, k_PrettyPrintJson);
            bool areJsonsEqual = newSettingsJson == m_cachedJson;

#if UNITY_2019_3_OR_NEWER
            if (!AssetDatabase.IsOpenForEdit(path) && areJsonsEqual == false)
            {
                if (!AssetDatabase.MakeEditable(path))
                {
                    Debug.LogWarning($"Could not save package settings to {path}");
                    return;
                }
            }
#endif

            try
            {
                if (!areJsonsEqual)
                {
                    File.WriteAllText(path, newSettingsJson);
                    m_cachedJson = newSettingsJson;
                }
            }
            catch (UnauthorizedAccessException)
            {
                Debug.LogWarning($"Could not save package settings to {path}");
            }
        }

        /// <summary>
        /// Set a value for key of type T.
        /// </summary>
        /// <param name="key">The settings key.</param>
        /// <param name="value">The value to set. Must be serializable.</param>
        /// <typeparam name="T">Type of value.</typeparam>
        /// <inheritdoc cref="ISettingsRepository.Set{T}"/>
        public void Set<T>(string key, T value)
        {
            Init();
            m_Dictionary.Set<T>(key, value);
        }

        /// <summary>
        /// Get a value with key of type T, or return the fallback value if no matching key is found.
        /// </summary>
        /// <param name="key">The settings key.</param>
        /// <param name="fallback">If no key with a value of type T is found, this value is returned.</param>
        /// <typeparam name="T">Type of value to search for.</typeparam>
        /// <inheritdoc cref="ISettingsRepository.Get{T}"/>
        public T Get<T>(string key, T fallback = default(T))
        {
            Init();
            return m_Dictionary.Get<T>(key, fallback);
        }

        /// <summary>
        /// Does the repository contain a setting with key and type.
        /// </summary>
        /// <param name="key">The settings key.</param>
        /// <typeparam name="T">The type of value to search for.</typeparam>
        /// <returns>True if a setting matching both key and type is found, false if no entry is found.</returns>
        /// <inheritdoc cref="ISettingsRepository.ContainsKey{T}"/>
        public bool ContainsKey<T>(string key)
        {
            Init();
            return m_Dictionary.ContainsKey<T>(key);
        }

        /// <summary>
        /// Remove a key value pair from the settings repository.
        /// </summary>
        /// <param name="key"></param>
        /// <typeparam name="T"></typeparam>
        /// <inheritdoc cref="ISettingsRepository.Remove{T}"/>
        public void Remove<T>(string key)
        {
            Init();
            m_Dictionary.Remove<T>(key);
        }
    }
}
