using System;
using UnityEngine;

namespace UnityEditor.SettingsManagement
{
    [Serializable]
    sealed class ValueWrapper<T>
    {
#if PRETTY_PRINT_JSON
        const bool k_PrettyPrintJson = true;
#else
        const bool k_PrettyPrintJson = false;
#endif

        [SerializeField]
        T m_Value;

        public static string Serialize(T value)
        {
            var obj = new ValueWrapper<T>() { m_Value = value };
            return EditorJsonUtility.ToJson(obj, k_PrettyPrintJson);
        }

        public static T Deserialize(string json)
        {
            var value = (object)Activator.CreateInstance<ValueWrapper<T>>();
            EditorJsonUtility.FromJsonOverwrite(json, value);
            return ((ValueWrapper<T>)value).m_Value;
        }

        public static T DeepCopy(T value)
        {
            if (typeof(ValueType).IsAssignableFrom(typeof(T)))
                return value;
            var str = Serialize(value);
            return Deserialize(str);
        }
    }
}
