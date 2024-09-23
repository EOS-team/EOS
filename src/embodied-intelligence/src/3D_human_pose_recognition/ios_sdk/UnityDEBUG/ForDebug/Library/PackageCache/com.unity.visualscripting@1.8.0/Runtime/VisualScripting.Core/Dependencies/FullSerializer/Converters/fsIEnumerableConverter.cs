using System;
using System.Collections;
using System.Collections.Generic;
using System.Reflection;
using Unity.VisualScripting.FullSerializer.Internal;

namespace Unity.VisualScripting.FullSerializer
{
    /// <summary>
    /// Provides serialization support for anything which extends from
    /// `IEnumerable` and has an `Add` method.
    /// </summary>
    public class fsIEnumerableConverter : fsConverter
    {
        public override bool CanProcess(Type type)
        {
            if (typeof(IEnumerable).IsAssignableFrom(type) == false)
            {
                return false;
            }
            return GetAddMethod(type) != null;
        }

        public override object CreateInstance(fsData data, Type storageType)
        {
            return fsMetaType.Get(Serializer.Config, storageType).CreateInstance();
        }

        public override fsResult TrySerialize(object instance_, out fsData serialized, Type storageType)
        {
            var instance = (IEnumerable)instance_;
            var result = fsResult.Success;

            var elementType = GetElementType(storageType);

            serialized = fsData.CreateList(HintSize(instance));
            var serializedList = serialized.AsList;

            foreach (var item in instance)
            {
                fsData itemData;

                // note: We don't fail the entire deserialization even if the
                //       item failed
                var itemResult = Serializer.TrySerialize(elementType, item, out itemData);
                result.AddMessages(itemResult);
                if (itemResult.Failed)
                {
                    continue;
                }

                serializedList.Add(itemData);
            }

            // Stacks iterate from back to front, which means when we deserialize
            // we will deserialize the items in the wrong order, so the stack
            // will get reversed.
            if (IsStack(instance.GetType()))
            {
                serializedList.Reverse();
            }

            return result;
        }

        private bool IsStack(Type type)
        {
            return type.Resolve().IsGenericType &&
                type.Resolve().GetGenericTypeDefinition() == typeof(Stack<>);
        }

        public override fsResult TryDeserialize(fsData data, ref object instance_, Type storageType)
        {
            var instance = (IEnumerable)instance_;
            var result = fsResult.Success;

            if ((result += CheckType(data, fsDataType.Array)).Failed)
            {
                return result;
            }

            // LAZLO/LUDIQ: Changes to default behaviour.
            //  - Do not try to serialize into existing element; always clear and add
            //    (more reliable, compatible with custom indexers)
            //  - If the type is a list, add a default element on failure to prevent
            //    messing up the order.
            //  - Commented out that last change.

            var elementStorageType = GetElementType(storageType);
            var addMethod = GetAddMethod(storageType);
            TryClear(storageType, instance);

            var serializedList = data.AsList;

            for (var i = 0; i < serializedList.Count; ++i)
            {
                var itemData = serializedList[i];
                object itemInstance = null;

                var itemResult = Serializer.TryDeserialize(itemData, elementStorageType, ref itemInstance);

                result.AddMessages(itemResult);

                if (itemResult.Succeeded)
                {
                    addMethod.Invoke(instance, new[] { itemInstance });
                }
                // else
                // {
                //  if (typeof(IList).IsAssignableFrom(storageType))
                //  {
                //      addMethod.Invoke(instance, new[] { elementStorageType.Default() });
                //  }
                //  else
                //  {
                //      continue;
                //  }
                // }
            }

            return result;
        }

        private static int HintSize(IEnumerable collection)
        {
            if (collection is ICollection)
            {
                return ((ICollection)collection).Count;
            }
            return 0;
        }

        /// <summary>
        /// Fetches the element type for objects inside of the collection.
        /// </summary>
        private static Type GetElementType(Type objectType)
        {
            if (objectType.HasElementType)
            {
                return objectType.GetElementType();
            }

            var enumerableList = fsReflectionUtility.GetInterface(objectType, typeof(IEnumerable<>));
            if (enumerableList != null)
            {
                return enumerableList.GetGenericArguments()[0];
            }

            return typeof(object);
        }

        private static void TryClear(Type type, object instance)
        {
            var clear = type.GetFlattenedMethod("Clear");
            if (clear != null)
            {
                clear.Invoke(instance, null);
            }
        }

        private static int TryGetExistingSize(Type type, object instance)
        {
            var count = type.GetFlattenedProperty("Count");
            if (count != null)
            {
                return (int)count.GetGetMethod().Invoke(instance, null);
            }
            return 0;
        }

        private static MethodInfo GetAddMethod(Type type)
        {
            // There is a really good chance the type will extend ICollection{},
            // which will contain the add method we want. Just doing
            // type.GetFlattenedMethod() may return the incorrect one -- for
            // example, with dictionaries, it'll return Add(TKey, TValue), and we
            // want Add(KeyValuePair<TKey, TValue>).
            var collectionInterface = fsReflectionUtility.GetInterface(type, typeof(ICollection<>));
            if (collectionInterface != null)
            {
                var add = collectionInterface.GetDeclaredMethod("Add");
                if (add != null)
                {
                    return add;
                }
            }

            // Otherwise try and look up a general Add method.
            return
                type.GetFlattenedMethod("Add") ??
                type.GetFlattenedMethod("Push") ??
                type.GetFlattenedMethod("Enqueue");
        }
    }
}
