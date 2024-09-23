using System;
using System.Collections.Generic;
using System.Linq;
using System.Reflection;
using UnityEngine;

namespace UnityEditor.SettingsManagement
{
    /// <summary>
    /// A collection of utilities for working with settings.
    /// </summary>
    static class UserSettings
    {
        internal const string packageName = "com.unity.settings-manager";

        internal static string GetSettingsString(IEnumerable<Assembly> assemblies, params SettingsScope[] scopes)
        {
            var settings = FindUserSettings(assemblies, SettingVisibility.All);
            if (scopes != null && scopes.Length > 0)
                settings = settings.Where(x => scopes.Contains(x.scope));
            var sb = new System.Text.StringBuilder();
            Type t = null;

            foreach (var pref in settings.OrderBy(x => x.type.ToString()))
            {
                if (pref.type != t)
                {
                    if (t != null)
                        sb.AppendLine();
                    t = pref.type;
                    sb.AppendLine(pref.type.ToString());
                }

                var val = pref.GetValue();
                sb.AppendLine(string.Format("{0,-4}{1,-24}{2,-64}{3}", "", pref.scope, pref.key, val != null ? val.ToString() : "null"));
            }

            return sb.ToString();
        }

        /// <summary>
        /// Collect all registered UserSetting and HiddenSetting attributes.
        /// </summary>
        /// <returns></returns>
        public static IEnumerable<IUserSetting> FindUserSettings(IEnumerable<Assembly> assemblies, SettingVisibility visibility, BindingFlags flags = BindingFlags.Public | BindingFlags.NonPublic | BindingFlags.Instance | BindingFlags.Static)
        {
            var loadedTypes = assemblies.SelectMany(x => x.GetTypes());
            var loadedFields = loadedTypes.SelectMany(x => x.GetFields(flags));
            var settings = new List<IUserSetting>();

            if ((visibility & (SettingVisibility.Visible | SettingVisibility.Unlisted)) > 0)
            {
                var attributes = loadedFields.Where(prop => Attribute.IsDefined(prop, typeof(UserSettingAttribute)));

                foreach (var field in attributes)
                {
                    var userSetting = (UserSettingAttribute)Attribute.GetCustomAttribute(field, typeof(UserSettingAttribute));

                    if (!field.IsStatic || !typeof(IUserSetting).IsAssignableFrom(field.FieldType))
                    {
                        Debug.LogError("[UserSetting] is only valid on static fields of a type implementing `interface IUserSetting`. \"" + field.Name + "\" (" + field.FieldType + ")\n" + field.DeclaringType);
                        continue;
                    }

                    bool visible = userSetting.visibleInSettingsProvider;

                    if (visible && (visibility & SettingVisibility.Visible) == SettingVisibility.Visible)
                        settings.Add((IUserSetting)field.GetValue(null));
                    else if (!visible && (visibility & SettingVisibility.Hidden) == SettingVisibility.Hidden)
                        settings.Add((IUserSetting)field.GetValue(null));
                }
            }

            if ((visibility & SettingVisibility.Unlisted) == SettingVisibility.Unlisted)
            {
                var settingsKeys = loadedFields.Where(y => Attribute.IsDefined(y, typeof(SettingsKeyAttribute)));

                foreach (var field in settingsKeys)
                {
                    if (field.IsStatic)
                    {
                        settings.Add((IUserSetting)field.GetValue(null));
                    }
                    else
                    {
                        var settingAttribute = (SettingsKeyAttribute)Attribute.GetCustomAttribute(field, typeof(SettingsKeyAttribute));
                        var pref = CreateGenericPref(settingAttribute.key, settingAttribute.scope, field);
                        if (pref != null)
                            settings.Add(pref);
                        else
                            Debug.LogWarning("Failed adding [SettingsKey] " + field.FieldType + "\"" + settingAttribute.key + "\" in " + field.DeclaringType);
                    }
                }
            }

            if ((visibility & SettingVisibility.Unregistered) == SettingVisibility.Unregistered)
            {
                var unregisterd = loadedFields.Where(y => typeof(IUserSetting).IsAssignableFrom(y.FieldType)
                        && !Attribute.IsDefined(y, typeof(SettingsKeyAttribute))
                        && !Attribute.IsDefined(y, typeof(UserSettingAttribute)));

                foreach (var field in unregisterd)
                {
                    if (field.IsStatic)
                    {
                        settings.Add((IUserSetting)field.GetValue(null));
                    }
                    else
                    {
#if PB_DEBUG
                        Log.Warning("Found unregistered instance field: "
                            + field.FieldType
                            + " "
                            + field.Name
                            + " in " + field.DeclaringType);
#endif
                    }
                }
            }

            return settings;
        }

        static IUserSetting CreateGenericPref(string key, SettingsScope scope, FieldInfo field)
        {
            try
            {
                var type = field.FieldType;
                if (typeof(IUserSetting).IsAssignableFrom(type) && type.IsGenericType)
                    type = type.GetGenericArguments().FirstOrDefault();
                var genericPrefClass = typeof(UserSetting<>).MakeGenericType(type);
                var defaultValue = type.IsValueType ? Activator.CreateInstance(type) : null;
                return (IUserSetting)Activator.CreateInstance(genericPrefClass, new object[] { key, defaultValue, scope });
            }
            catch
            {
                return null;
            }
        }
    }
}
