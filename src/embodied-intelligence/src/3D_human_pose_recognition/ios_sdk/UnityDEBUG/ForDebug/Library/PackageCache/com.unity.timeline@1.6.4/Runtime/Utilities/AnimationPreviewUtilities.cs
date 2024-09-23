using System;
#if UNITY_EDITOR
using System.Collections.Generic;
using UnityEditor;


namespace UnityEngine.Timeline
{
    static class AnimationPreviewUtilities
    {
        private const string k_PosX = "m_LocalPosition.x";
        private const string k_PosY = "m_LocalPosition.y";
        private const string k_PosZ = "m_LocalPosition.z";
        private const string k_RotX = "m_LocalRotation.x";
        private const string k_RotY = "m_LocalRotation.y";
        private const string k_RotZ = "m_LocalRotation.z";
        private const string k_RotW = "m_LocalRotation.w";
        private const string k_ScaleX = "m_LocalScale.x";
        private const string k_ScaleY = "m_LocalScale.y";
        private const string k_ScaleZ = "m_LocalScale.z";
        private const string k_EulerAnglesRaw = "localEulerAnglesRaw";
        private const string k_EulerHint = "m_LocalEulerAnglesHint";
        private const string k_Pos = "m_LocalPosition";
        private const string k_Rot = "m_LocalRotation";
        private const string k_MotionT = "MotionT";
        private const string k_MotionQ = "MotionQ";
        private const string k_RootT = "RootT";
        private const string k_RootQ = "RootQ";


        internal static Object s_PreviewDriver;


        internal class EditorCurveBindingComparer : IEqualityComparer<EditorCurveBinding>
        {
            public bool Equals(EditorCurveBinding x, EditorCurveBinding y) { return x.path.Equals(y.path) && x.type == y.type && x.propertyName == y.propertyName; }
            public int GetHashCode(EditorCurveBinding obj)
            {
                return obj.propertyName.GetHashCode() ^ obj.path.GetHashCode();
            }

            public static readonly EditorCurveBindingComparer Instance = new EditorCurveBindingComparer();
        }

        // a dictionary is faster than a hashset, because the capacity can be pre-set
        private static readonly Dictionary<EditorCurveBinding, int> s_CurveSet = new Dictionary<EditorCurveBinding, int>(10000, EditorCurveBindingComparer.Instance);
        private static readonly AnimatorBindingCache s_BindingCache = new AnimatorBindingCache();

        // string.StartsWith is slow (https://docs.unity3d.com/Manual/BestPracticeUnderstandingPerformanceInUnity5.html)
        // hand rolled version has best performance.
        private static bool FastStartsWith(string a, string toCompare)
        {
            int aLen = a.Length;
            int bLen = toCompare.Length;

            int ap = 0;
            int bp = 0;

            while (ap < aLen && bp < bLen && a[ap] == toCompare[bp])
            {
                ap++;
                bp++;
            }

            return (bp == bLen);
        }

        public static void ClearCaches()
        {
            s_BindingCache.Clear();
            s_CurveSet.Clear();
        }

        public static EditorCurveBinding[] GetBindings(GameObject animatorRoot, IEnumerable<AnimationClip> clips)
        {
            s_CurveSet.Clear();
            foreach (var clip in clips)
            {
                AddBindings(s_BindingCache.GetCurveBindings(clip));
            }

            // if we have a transform binding, bind the entire skeleton
            if (NeedsSkeletonBindings(s_CurveSet.Keys))
                AddBindings(s_BindingCache.GetAnimatorBindings(animatorRoot));

            var bindings = new EditorCurveBinding[s_CurveSet.Keys.Count];
            s_CurveSet.Keys.CopyTo(bindings, 0);
            return bindings;
        }

        public static int GetClipHash(List<AnimationClip> clips)
        {
            int hash = 0;

            foreach (var clip in clips)
            {
                var stats = AnimationUtility.GetAnimationClipStats(clip);
                hash = HashUtility.CombineHash(hash, clip.GetHashCode(), stats.clips, stats.size, stats.totalCurves);
            }
            return hash;
        }

        public static void PreviewFromCurves(GameObject animatorRoot, IEnumerable<EditorCurveBinding> keys)
        {
            if (!AnimationMode.InAnimationMode())
                return;

            var avatarRoot = GetAvatarRoot(animatorRoot);
            foreach (var binding in keys)
            {
                if (IsAvatarBinding(binding) || IsEuler(binding))
                    continue;

                bool isTransform = typeof(Transform).IsAssignableFrom(binding.type);
                if (isTransform && binding.propertyName.Equals(AnimatorBindingCache.TRPlaceHolder))
                    AddTRBinding(animatorRoot, binding);
                else if (isTransform && binding.propertyName.Equals(AnimatorBindingCache.ScalePlaceholder))
                    AddScaleBinding(animatorRoot, binding);
                else
                    AnimationMode.AddEditorCurveBinding(avatarRoot, binding);
            }
        }

        public static AnimationClip CreateDefaultClip(GameObject animatorRoot, IEnumerable<EditorCurveBinding> keys)
        {
            AnimationClip animClip = new AnimationClip() { name = "DefaultPose" };
            var keyFrames = new[] {new Keyframe(0, 0)};
            var curve = new AnimationCurve(keyFrames);
            bool rootMotion = false;
            var avatarRoot = GetAvatarRoot(animatorRoot);

            foreach (var binding in keys)
            {
                if (IsRootMotion(binding))
                {
                    rootMotion = true;
                    continue;
                }

                if (typeof(Transform).IsAssignableFrom(binding.type) && binding.propertyName.Equals(AnimatorBindingCache.TRPlaceHolder))
                {
                    if (string.IsNullOrEmpty(binding.path))
                        rootMotion = true;
                    else
                    {
                        var transform = animatorRoot.transform.Find(binding.path);
                        if (transform != null)
                        {
                            var pos = transform.localPosition;
                            var rot = transform.localRotation;
                            animClip.SetCurve(binding.path, typeof(Transform), k_PosX, SetZeroKey(curve, keyFrames, pos.x));
                            animClip.SetCurve(binding.path, typeof(Transform), k_PosY, SetZeroKey(curve, keyFrames, pos.y));
                            animClip.SetCurve(binding.path, typeof(Transform), k_PosZ, SetZeroKey(curve, keyFrames, pos.z));
                            animClip.SetCurve(binding.path, typeof(Transform), k_RotX, SetZeroKey(curve, keyFrames, rot.x));
                            animClip.SetCurve(binding.path, typeof(Transform), k_RotY, SetZeroKey(curve, keyFrames, rot.y));
                            animClip.SetCurve(binding.path, typeof(Transform), k_RotZ, SetZeroKey(curve, keyFrames, rot.z));
                            animClip.SetCurve(binding.path, typeof(Transform), k_RotW, SetZeroKey(curve, keyFrames, rot.w));
                        }
                    }

                    continue;
                }

                if (typeof(Transform).IsAssignableFrom(binding.type) && binding.propertyName == AnimatorBindingCache.ScalePlaceholder)
                {
                    var transform = animatorRoot.transform.Find(binding.path);
                    if (transform != null)
                    {
                        var scale = transform.localScale;
                        animClip.SetCurve(binding.path, typeof(Transform), k_ScaleX, SetZeroKey(curve, keyFrames, scale.x));
                        animClip.SetCurve(binding.path, typeof(Transform), k_ScaleY, SetZeroKey(curve, keyFrames, scale.y));
                        animClip.SetCurve(binding.path, typeof(Transform), k_ScaleZ, SetZeroKey(curve, keyFrames, scale.z));
                    }

                    continue;
                }

                // Not setting curves through AnimationUtility.SetEditorCurve to avoid reentrant
                // onCurveWasModified calls in timeline.  This means we don't get sprite curves
                // in the default clip right now.
                if (IsAvatarBinding(binding) || IsEulerHint(binding) || binding.isPPtrCurve)
                    continue;

                float floatValue;
                AnimationUtility.GetFloatValue(avatarRoot, binding, out floatValue);
                animClip.SetCurve(binding.path, binding.type, binding.propertyName, SetZeroKey(curve, keyFrames, floatValue));
            }

            // add root motion explicitly.
            if (rootMotion)
            {
                var pos = Vector3.zero;           // the appropriate root motion offsets are applied by timeline
                var rot = Quaternion.identity;
                animClip.SetCurve(string.Empty, typeof(Transform), k_PosX, SetZeroKey(curve, keyFrames, pos.x));
                animClip.SetCurve(string.Empty, typeof(Transform), k_PosY, SetZeroKey(curve, keyFrames, pos.y));
                animClip.SetCurve(string.Empty, typeof(Transform), k_PosZ, SetZeroKey(curve, keyFrames, pos.z));
                animClip.SetCurve(string.Empty, typeof(Transform), k_RotX, SetZeroKey(curve, keyFrames, rot.x));
                animClip.SetCurve(string.Empty, typeof(Transform), k_RotY, SetZeroKey(curve, keyFrames, rot.y));
                animClip.SetCurve(string.Empty, typeof(Transform), k_RotZ, SetZeroKey(curve, keyFrames, rot.z));
                animClip.SetCurve(string.Empty, typeof(Transform), k_RotW, SetZeroKey(curve, keyFrames, rot.w));
            }

            return animClip;
        }

        public static bool IsRootMotion(EditorCurveBinding binding)
        {
            // Root Transform TR.
            if (typeof(Transform).IsAssignableFrom(binding.type) && string.IsNullOrEmpty(binding.path))
            {
                return FastStartsWith(binding.propertyName, k_Pos)  || FastStartsWith(binding.propertyName, k_Rot);
            }

            // MotionCurves/RootCurves.
            if (binding.type == typeof(Animator))
            {
                return FastStartsWith(binding.propertyName, k_MotionT) ||
                    FastStartsWith(binding.propertyName, k_MotionQ) ||
                    FastStartsWith(binding.propertyName, k_RootT) ||
                    FastStartsWith(binding.propertyName, k_RootQ);
            }

            return false;
        }

        private static bool NeedsSkeletonBindings(IEnumerable<EditorCurveBinding> bindings)
        {
            foreach (var b in bindings)
            {
                if (IsSkeletalBinding(b))
                    return true;
            }

            return false;
        }

        private static void AddBindings(IEnumerable<EditorCurveBinding> bindings)
        {
            foreach (var b in bindings)
            {
                if (!s_CurveSet.ContainsKey(b))
                    s_CurveSet[b] = 1;
            }
        }

        private static void AddTRBinding(GameObject root, EditorCurveBinding binding)
        {
            var t = root.transform.Find(binding.path);
            if (t != null)
            {
                DrivenPropertyManager.RegisterProperty(s_PreviewDriver, t, "m_LocalPosition");
                DrivenPropertyManager.RegisterProperty(s_PreviewDriver, t, "m_LocalRotation");
            }
        }

        private static void AddScaleBinding(GameObject root, EditorCurveBinding binding)
        {
            var t = root.transform.Find(binding.path);
            if (t != null)
                DrivenPropertyManager.RegisterProperty(s_PreviewDriver, t, "m_LocalScale");
        }

        private static bool IsEuler(EditorCurveBinding binding)
        {
            return FastStartsWith(binding.propertyName, k_EulerAnglesRaw) &&
                typeof(Transform).IsAssignableFrom(binding.type);
        }

        private static bool IsAvatarBinding(EditorCurveBinding binding)
        {
            return string.IsNullOrEmpty(binding.path) && typeof(Animator) == binding.type;
        }

        private static bool IsSkeletalBinding(EditorCurveBinding binding)
        {
            // skin mesh incorporates blend shapes
            return typeof(Transform).IsAssignableFrom(binding.type) || typeof(SkinnedMeshRenderer).IsAssignableFrom(binding.type);
        }

        private static AnimationCurve SetZeroKey(AnimationCurve curve, Keyframe[] keys, float val)
        {
            keys[0].value = val;
            curve.keys = keys;
            return curve;
        }

        private static bool IsEulerHint(EditorCurveBinding binding)
        {
            return typeof(Transform).IsAssignableFrom(binding.type) && binding.propertyName.StartsWith(k_EulerHint);
        }

        private static GameObject GetAvatarRoot(GameObject animatorRoot)
        {
            var animator = animatorRoot.GetComponent<Animator>();
            if (animator != null && animator.avatarRoot != animatorRoot.transform)
                return animator.avatarRoot.gameObject;
            return animatorRoot;
        }
    }
}
#endif
