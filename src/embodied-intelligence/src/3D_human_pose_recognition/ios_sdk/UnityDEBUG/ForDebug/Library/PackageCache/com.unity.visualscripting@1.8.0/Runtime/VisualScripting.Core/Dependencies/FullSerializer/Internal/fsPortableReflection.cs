#if !UNITY_EDITOR && UNITY_METRO && !ENABLE_IL2CPP
#define USE_TYPEINFO
#if !UNITY_WINRT_10_0
#define USE_TYPEINFO_EXTENSIONS
#endif
#endif

using System;
using System.Collections.Generic;
using System.Linq;
using System.Reflection;

#if USE_TYPEINFO
namespace System
{
    public static class AssemblyExtensions
    {
#if USE_TYPEINFO_EXTENSIONS
        public static Type[] GetTypes(this Assembly assembly)
        {
            TypeInfo[] infos = assembly.DefinedTypes.ToArray();
            Type[] types = new Type[infos.Length];
            for (int i = 0; i < infos.Length; ++i)
            {
                types[i] = infos[i].AsType();
            }
            return types;
        }

#endif

        public static Type GetType(this Assembly assembly, string name, bool throwOnError)
        {
            var types = assembly.GetTypes();
            for (int i = 0; i < types.Length; ++i)
            {
                if (types[i].Name == name)
                {
                    return types[i];
                }
            }

            if (throwOnError) throw new Exception("Type " + name + " was not found");
            return null;
        }
    }
}
#endif

namespace Unity.VisualScripting.FullSerializer.Internal
{
    /// <summary>
    /// This wraps reflection types so that it is portable across different Unity
    /// runtimes.
    /// </summary>
    public static class fsPortableReflection
    {
        public static Type[] EmptyTypes = { };

        #region Attribute Queries

#if USE_TYPEINFO
        public static TAttribute GetAttribute<TAttribute>(Type type)
            where TAttribute : Attribute
        {
            return GetAttribute<TAttribute>(type.GetTypeInfo());
        }

        public static Attribute GetAttribute(Type type, Type attributeType)
        {
            return GetAttribute(type.GetTypeInfo(), attributeType, /*shouldCache:*/ false);
        }

        public static bool HasAttribute(Type type, Type attributeType)
        {
            return GetAttribute(type, attributeType) != null;
        }

#endif

        /// <summary>
        /// Returns true if the given attribute is defined on the given element.
        /// </summary>
        public static bool HasAttribute<TAttribute>(MemberInfo element)
        {
            return HasAttribute(element, typeof(TAttribute));
        }

        /// <summary>
        /// Returns true if the given attribute is defined on the given element.
        /// </summary>
        public static bool HasAttribute<TAttribute>(MemberInfo element, bool shouldCache)
        {
            return HasAttribute(element, typeof(TAttribute), shouldCache);
        }

        /// <summary>
        /// Returns true if the given attribute is defined on the given element.
        /// </summary>
        public static bool HasAttribute(MemberInfo element, Type attributeType)
        {
            return HasAttribute(element, attributeType, true);
        }

        /// <summary>
        /// Returns true if the given attribute is defined on the given element.
        /// </summary>
        public static bool HasAttribute(MemberInfo element, Type attributeType, bool shouldCache)
        {
            // LAZLO / LUDIQ FIX
            // Inheritance on property overrides. MemberInfo.IsDefined ignores the inherited parameter.
            // https://stackoverflow.com/questions/2520035
            return Attribute.IsDefined(element, attributeType, true);
            //return element.IsDefined(attributeType, true);
        }

        /// <summary>
        /// Fetches the given attribute from the given MemberInfo. This method
        /// applies caching and is allocation free (after caching has been
        /// performed).
        /// </summary>
        /// <param name="element">
        /// The MemberInfo the get the attribute from.
        /// </param>
        /// <param name="attributeType">The type of attribute to fetch.</param>
        /// <returns>The attribute or null.</returns>
        public static Attribute GetAttribute(MemberInfo element, Type attributeType, bool shouldCache)
        {
            var query = new AttributeQuery
            {
                MemberInfo = element,
                AttributeType = attributeType
            };

            Attribute attribute;
            if (_cachedAttributeQueries.TryGetValue(query, out attribute) == false)
            {
                // LAZLO / LUDIQ FIX
                // Inheritance on property overrides. MemberInfo.IsDefined ignores the inherited parameter
                //var attributes = element.GetCustomAttributes(attributeType, /*inherit:*/ true).ToArray();
                var attributes = Attribute.GetCustomAttributes(element, attributeType, true).ToArray();

                if (attributes.Length > 0)
                {
                    attribute = (Attribute)attributes[0];
                }
                if (shouldCache)
                {
                    _cachedAttributeQueries[query] = attribute;
                }
            }

            return attribute;
        }

        /// <summary>
        /// Fetches the given attribute from the given MemberInfo.
        /// </summary>
        /// <typeparam name="TAttribute">
        /// The type of attribute to fetch.
        /// </typeparam>
        /// <param name="element">
        /// The MemberInfo to get the attribute from.
        /// </param>
        /// <param name="shouldCache">
        /// Should this computation be cached? If this is the only time it will
        /// ever be done, don't bother caching.
        /// </param>
        /// <returns>The attribute or null.</returns>
        public static TAttribute GetAttribute<TAttribute>(MemberInfo element, bool shouldCache)
            where TAttribute : Attribute
        {
            return (TAttribute)GetAttribute(element, typeof(TAttribute), shouldCache);
        }

        public static TAttribute GetAttribute<TAttribute>(MemberInfo element)
            where TAttribute : Attribute
        {
            return GetAttribute<TAttribute>(element, /*shouldCache:*/ true);
        }

        private struct AttributeQuery
        {
            public MemberInfo MemberInfo;
            public Type AttributeType;
        }

        private static IDictionary<AttributeQuery, Attribute> _cachedAttributeQueries =
            new Dictionary<AttributeQuery, Attribute>(new AttributeQueryComparator());

        private class AttributeQueryComparator : IEqualityComparer<AttributeQuery>
        {
            public bool Equals(AttributeQuery x, AttributeQuery y)
            {
                return
                    x.MemberInfo == y.MemberInfo &&
                    x.AttributeType == y.AttributeType;
            }

            public int GetHashCode(AttributeQuery obj)
            {
                return
                    obj.MemberInfo.GetHashCode() +
                    (17 * obj.AttributeType.GetHashCode());
            }
        }

        #endregion Attribute Queries

#if !USE_TYPEINFO
        private static BindingFlags DeclaredFlags =
            BindingFlags.NonPublic |
            BindingFlags.Public |
            BindingFlags.Instance |
            BindingFlags.Static |
            BindingFlags.DeclaredOnly;
#endif

        public static PropertyInfo GetDeclaredProperty(this Type type, string propertyName)
        {
            var props = GetDeclaredProperties(type);

            for (var i = 0; i < props.Length; ++i)
            {
                if (props[i].Name == propertyName)
                {
                    return props[i];
                }
            }

            return null;
        }

        public static MethodInfo GetDeclaredMethod(this Type type, string methodName)
        {
            var methods = GetDeclaredMethods(type);

            for (var i = 0; i < methods.Length; ++i)
            {
                if (methods[i].Name == methodName)
                {
                    return methods[i];
                }
            }

            return null;
        }

        public static ConstructorInfo GetDeclaredConstructor(this Type type, Type[] parameters)
        {
            var ctors = GetDeclaredConstructors(type);

            for (var i = 0; i < ctors.Length; ++i)
            {
                var ctor = ctors[i];
                var ctorParams = ctor.GetParameters();

                if (parameters.Length != ctorParams.Length)
                {
                    continue;
                }

                for (var j = 0; j < ctorParams.Length; ++j)
                {
                    // require an exact match
                    if (ctorParams[j].ParameterType != parameters[j])
                    {
                        continue;
                    }
                }

                return ctor;
            }

            return null;
        }

        public static ConstructorInfo[] GetDeclaredConstructors(this Type type)
        {
#if USE_TYPEINFO
            return type.GetTypeInfo().DeclaredConstructors.ToArray();
#else
            return type.GetConstructors(DeclaredFlags & ~BindingFlags.Static); // LUDIQ: Exclude static constructors
#endif
        }

        public static MemberInfo[] GetFlattenedMember(this Type type, string memberName)
        {
            var result = new List<MemberInfo>();

            while (type != null)
            {
                var members = GetDeclaredMembers(type);

                for (var i = 0; i < members.Length; ++i)
                {
                    if (members[i].Name == memberName)
                    {
                        result.Add(members[i]);
                    }
                }

                type = type.Resolve().BaseType;
            }

            return result.ToArray();
        }

        public static MethodInfo GetFlattenedMethod(this Type type, string methodName)
        {
            while (type != null)
            {
                var methods = GetDeclaredMethods(type);

                for (var i = 0; i < methods.Length; ++i)
                {
                    if (methods[i].Name == methodName)
                    {
                        return methods[i];
                    }
                }

                type = type.Resolve().BaseType;
            }

            return null;
        }

        public static IEnumerable<MethodInfo> GetFlattenedMethods(this Type type, string methodName)
        {
            while (type != null)
            {
                var methods = GetDeclaredMethods(type);

                for (var i = 0; i < methods.Length; ++i)
                {
                    if (methods[i].Name == methodName)
                    {
                        yield return methods[i];
                    }
                }

                type = type.Resolve().BaseType;
            }
        }

        public static PropertyInfo GetFlattenedProperty(this Type type, string propertyName)
        {
            while (type != null)
            {
                var properties = GetDeclaredProperties(type);

                for (var i = 0; i < properties.Length; ++i)
                {
                    if (properties[i].Name == propertyName)
                    {
                        return properties[i];
                    }
                }

                type = type.Resolve().BaseType;
            }

            return null;
        }

        public static MemberInfo GetDeclaredMember(this Type type, string memberName)
        {
            var members = GetDeclaredMembers(type);

            for (var i = 0; i < members.Length; ++i)
            {
                if (members[i].Name == memberName)
                {
                    return members[i];
                }
            }

            return null;
        }

        public static MethodInfo[] GetDeclaredMethods(this Type type)
        {
#if USE_TYPEINFO
            return type.GetTypeInfo().DeclaredMethods.ToArray();
#else
            return type.GetMethods(DeclaredFlags);
#endif
        }

        public static PropertyInfo[] GetDeclaredProperties(this Type type)
        {
#if USE_TYPEINFO
            return type.GetTypeInfo().DeclaredProperties.ToArray();
#else
            return type.GetProperties(DeclaredFlags);
#endif
        }

        public static FieldInfo[] GetDeclaredFields(this Type type)
        {
#if USE_TYPEINFO
            return type.GetTypeInfo().DeclaredFields.ToArray();
#else
            return type.GetFields(DeclaredFlags);
#endif
        }

        public static MemberInfo[] GetDeclaredMembers(this Type type)
        {
#if USE_TYPEINFO
            return type.GetTypeInfo().DeclaredMembers.ToArray();
#else
            return type.GetMembers(DeclaredFlags);
#endif
        }

        public static MemberInfo AsMemberInfo(Type type)
        {
#if USE_TYPEINFO
            return type.GetTypeInfo();
#else
            return type;
#endif
        }

        public static bool IsType(MemberInfo member)
        {
#if USE_TYPEINFO
            return member is TypeInfo;
#else
            return member is Type;
#endif
        }

        public static Type AsType(MemberInfo member)
        {
#if USE_TYPEINFO
            return ((TypeInfo)member).AsType();
#else
            return (Type)member;
#endif
        }

#if USE_TYPEINFO
        public static TypeInfo Resolve(this Type type)
        {
            return type.GetTypeInfo();
        }

#else
        public static Type Resolve(this Type type)
        {
            return type;
        }

#endif

        #region Extensions

#if USE_TYPEINFO_EXTENSIONS
        public static bool IsAssignableFrom(this Type parent, Type child)
        {
            return parent.GetTypeInfo().IsAssignableFrom(child.GetTypeInfo());
        }

        public static Type GetElementType(this Type type)
        {
            return type.GetTypeInfo().GetElementType();
        }

        public static MethodInfo GetSetMethod(this PropertyInfo member, bool nonPublic = false)
        {
            // only public requested but the set method is not public
            if (nonPublic == false && member.SetMethod != null && member.SetMethod.IsPublic == false) return null;

            return member.SetMethod;
        }

        public static MethodInfo GetGetMethod(this PropertyInfo member, bool nonPublic = false)
        {
            // only public requested but the set method is not public
            if (nonPublic == false && member.GetMethod != null && member.GetMethod.IsPublic == false) return null;

            return member.GetMethod;
        }

        public static MethodInfo GetBaseDefinition(this MethodInfo method)
        {
            return method.GetRuntimeBaseDefinition();
        }

        public static Type[] GetInterfaces(this Type type)
        {
            return type.GetTypeInfo().ImplementedInterfaces.ToArray();
        }

        public static Type[] GetGenericArguments(this Type type)
        {
            return type.GetTypeInfo().GenericTypeArguments.ToArray();
        }

#endif

        #endregion Extensions
    }
}
