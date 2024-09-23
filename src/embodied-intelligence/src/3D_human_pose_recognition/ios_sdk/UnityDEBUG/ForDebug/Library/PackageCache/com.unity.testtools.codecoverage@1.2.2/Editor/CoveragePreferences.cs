using UnityEditor.SettingsManagement;
using UnityEditor.TestTools.CodeCoverage.Utils;

namespace UnityEditor.TestTools.CodeCoverage
{
    internal class CoveragePreferences : CoveragePreferencesImplementation
    {
        private static CoveragePreferences s_Instance = null;
        private const string k_PackageName = "com.unity.testtools.codecoverage";

        public static CoveragePreferences instance
        {
            get
            {
                if (s_Instance == null)
                    s_Instance = new CoveragePreferences();

                return s_Instance;
            }
        }

        protected CoveragePreferences() : base(k_PackageName)
        {
        }
    }

    internal class CoveragePreferencesImplementation
    {
        private const string k_ProjectPathAlias = "{ProjectPath}";

        protected Settings m_Settings;

        public CoveragePreferencesImplementation(string packageName)
        {
            m_Settings = new Settings(packageName);
        }

        public bool GetBool(string key, bool defaultValue, SettingsScope scope = SettingsScope.Project)
        {
            if (m_Settings.ContainsKey<bool>(key, scope))
            {
                return m_Settings.Get<bool>(key, scope, defaultValue);
            }

            return defaultValue;
        }

        public int GetInt(string key, int defaultValue, SettingsScope scope = SettingsScope.Project)
        {
            if (m_Settings.ContainsKey<int>(key, scope))
            {
                return m_Settings.Get<int>(key, scope, defaultValue);
            }

            return defaultValue;
        }

        public string GetStringForPaths(string key, string defaultValue, SettingsScope scope = SettingsScope.Project)
        {
            string value = GetString(key, defaultValue, scope);
            value = value.Replace(k_ProjectPathAlias, CoverageUtils.GetProjectPath());
            return value;
        }

        public string GetString(string key, string defaultValue, SettingsScope scope = SettingsScope.Project)
        {
            if (m_Settings.ContainsKey<string>(key, scope))
            {
                return m_Settings.Get<string>(key, scope, defaultValue);
            }

            return defaultValue;
        }

        public void SetBool(string key, bool value, SettingsScope scope = SettingsScope.Project)
        {
            m_Settings.Set<bool>(key, value, scope);
            m_Settings.Save();
        }

        public void SetInt(string key, int value, SettingsScope scope = SettingsScope.Project)
        {
            m_Settings.Set<int>(key, value, scope);
            m_Settings.Save();
        }

        public void SetStringForPaths(string key, string value, SettingsScope scope = SettingsScope.Project)
        {
            value = CoverageUtils.NormaliseFolderSeparators(value, false);
            value = value.Replace(CoverageUtils.GetProjectPath(), k_ProjectPathAlias);
            SetString(key, value, scope);
        }

        public void SetString(string key, string value, SettingsScope scope = SettingsScope.Project)
        {
            m_Settings.Set<string>(key, value, scope);
            m_Settings.Save();
        }

        public void DeleteBool(string key, SettingsScope scope = SettingsScope.Project)
        {
            m_Settings.DeleteKey<bool>(key, scope);
            m_Settings.Save();
        }

        public void DeleteInt(string key, SettingsScope scope = SettingsScope.Project)
        {
            m_Settings.DeleteKey<int>(key, scope);
            m_Settings.Save();
        }

        public void DeleteString(string key, SettingsScope scope = SettingsScope.Project)
        {
            m_Settings.DeleteKey<string>(key, scope);
            m_Settings.Save();
        }
    }
}
