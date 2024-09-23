using System;
using UnityEditor.TestTools.CodeCoverage.Analytics;

namespace UnityEditor.TestTools.CodeCoverage
{
    /// <summary>
    /// Events invoked during a code coverage session. A code coverage session is the period between starting and finishing capturing code coverage data. 
    /// </summary>
    /// <example>
    /// In the following example we create event handler methods which subscribe to the <see cref="Events.onCoverageSessionStarted"/>, <see cref="Events.onCoverageSessionFinished"/>, <see cref="Events.onCoverageSessionPaused"/> and <see cref="Events.onCoverageSessionUnpaused"/> events.
    /// We use the <see href="https://docs.unity3d.com/ScriptReference/InitializeOnLoadAttribute.html">InitializeOnLoad</see> attribute to make sure that they will resubscribe on Domain Reload, when we enter Play Mode for example.
    /// <code>
    /// using UnityEngine;
    /// using UnityEditor;
    /// using UnityEditor.TestTools.CodeCoverage;
    /// 
    /// [InitializeOnLoad]
    /// public class CoverageSessionListener
    /// {
    ///     static CoverageSessionListener()
    ///     {
    ///         Events.onCoverageSessionStarted += OnSessionStarted;
    ///         Events.onCoverageSessionFinished += OnSessionFinished;
    ///         Events.onCoverageSessionPaused += OnSessionPaused;
    ///         Events.onCoverageSessionUnpaused += OnSessionUnpaused;
    ///     }
    ///
    ///     static void OnSessionStarted(SessionEventInfo args)
    ///     {
    ///         Debug.Log($"{args.SessionMode} Code Coverage Session Started");
    ///     }
    ///     static void OnSessionFinished(SessionEventInfo args)
    ///     {
    ///         Debug.Log($"{args.SessionMode} Code Coverage Session Finished");
    ///
    ///         string paths = string.Empty;
    ///         foreach (string path in args.SessionResultPaths)
    ///             paths = string.Concat(paths, "\n", path);
    ///
    ///         Debug.Log($"Code Coverage Results were saved in: {paths}");
    ///     }
    ///
    ///     static void OnSessionPaused(SessionEventInfo args)
    ///     {
    ///         Debug.Log($"{args.SessionMode} Code Coverage Session Paused");
    ///     }
    ///
    ///     static void OnSessionUnpaused(SessionEventInfo args)
    ///     {
    ///         Debug.Log($"{args.SessionMode} Code Coverage Session Unpaused");
    ///     }
    /// }
    /// </code>
    /// </example>
    public static class Events
    {
        /// <summary>
        /// This event is invoked when a code coverage session is started.
        /// </summary>
        public static event Action<SessionEventInfo> onCoverageSessionStarted;
        /// <summary>
        /// This event is invoked when a code coverage session is finished.
        /// </summary>
        public static event Action<SessionEventInfo> onCoverageSessionFinished;
        /// <summary>
        /// This event is invoked when a code coverage session is paused.
        /// </summary>
        public static event Action<SessionEventInfo> onCoverageSessionPaused;
        /// <summary>
        /// This event is invoked when a code coverage session is unpaused.
        /// </summary>
        public static event Action<SessionEventInfo> onCoverageSessionUnpaused;

        internal static void InvokeOnCoverageSessionStarted()
        {
            if (onCoverageSessionStarted != null)
            {
                CoverageAnalytics.instance.CurrentCoverageEvent.useEvent_onCoverageSessionStarted = true;
                onCoverageSessionStarted.Invoke(CoverageEventData.instance.GetCoverageSessionInfo());
            }    
        }

        internal static void InvokeOnCoverageSessionFinished()
        {
            if (onCoverageSessionFinished != null)
            {
                CoverageAnalytics.instance.CurrentCoverageEvent.useEvent_onCoverageSessionFinished = true;
                onCoverageSessionFinished.Invoke(CoverageEventData.instance.GetCoverageSessionInfo());
            }
        }

        internal static void InvokeOnCoverageSessionPaused()
        {
            if (onCoverageSessionPaused != null)
            {
                CoverageAnalytics.instance.CurrentCoverageEvent.useEvent_onCoverageSessionPaused = true;
                onCoverageSessionPaused.Invoke(CoverageEventData.instance.GetCoverageSessionInfo());
            }
        }

        internal static void InvokeOnCoverageSessionUnpaused()
        {
            if (onCoverageSessionUnpaused != null)
            {
                CoverageAnalytics.instance.CurrentCoverageEvent.useEvent_onCoverageSessionUnpaused = true;
                onCoverageSessionUnpaused.Invoke(CoverageEventData.instance.GetCoverageSessionInfo());
            }
        }
    }
}
