#if !NO_UNITY && UNITY_5_3_OR_NEWER
using System;
using UnityEngine;
using UnityEngine.Events;

namespace Unity.VisualScripting.FullSerializer
{
    partial class fsConverterRegistrar
    {
        // Disable the converter for the time being. Unity's JsonUtility API
        // cannot be called from within a C# ISerializationCallbackReceiver
        // callback.

        // public static Internal.Converters.UnityEvent_Converter
        // Register_UnityEvent_Converter;
    }
}

namespace Unity.VisualScripting.FullSerializer.Internal.Converters
{
    // The standard FS reflection converter has started causing Unity to crash
    // when processing UnityEvent. We can send the serialization through
    // JsonUtility which appears to work correctly instead.
    //
    // We have to support legacy serialization formats so importing works as
    // expected.
    public class UnityEvent_Converter : fsConverter
    {
        public override bool CanProcess(Type type)
        {
            return typeof(UnityEvent).Resolve().IsAssignableFrom(type.Resolve()) && type.Resolve().IsGenericType == false;
        }

        public override bool RequestCycleSupport(Type storageType)
        {
            return false;
        }

        public override fsResult TryDeserialize(fsData data, ref object instance, Type storageType)
        {
            Type objectType = (Type)instance;

            fsResult result = fsResult.Success;
            instance = JsonUtility.FromJson(fsJsonPrinter.CompressedJson(data), objectType);
            return result;
        }

        public override fsResult TrySerialize(object instance, out fsData serialized, Type storageType)
        {
            fsResult result = fsResult.Success;
            serialized = fsJsonParser.Parse(JsonUtility.ToJson(instance));
            return result;
        }
    }
}
#endif
