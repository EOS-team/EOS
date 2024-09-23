using System;
using System.Collections.Generic;
using System.Linq;
using UnityEditor.SceneManagement;
using UnityEditor.ShortcutManagement;
using UnityEditor.Timeline.Actions;
using UnityEngine;
using UnityEngine.Animations;
using UnityEngine.Playables;
using UnityEngine.SceneManagement;
using UnityEngine.Timeline;

namespace UnityEditor.Timeline
{
    partial class TimelineWindow
    {
        private int m_ComponentAddedFrame;

        void OnSelectionChangedInactive()
        {
            // Case  946942 -- when selection changes and the window is open but hidden, timeline
            // needs to update selection immediately so preview mode is correctly released
            // Case 1123119 -- except when recording
            if (!hasFocus)
            {
                RefreshSelection(!locked && state != null && !state.recording);
            }
        }

        void InitializeEditorCallbacks()
        {
            Undo.postprocessModifications += PostprocessAnimationRecordingModifications;
            Undo.postprocessModifications += ProcessAssetModifications;
            Undo.undoRedoPerformed += OnUndoRedo;
            EditorApplication.playModeStateChanged += OnPlayModeStateChanged;
            AnimationUtility.onCurveWasModified += OnCurveModified;
            EditorApplication.editorApplicationQuit += OnEditorQuit;
            Selection.selectionChanged += OnSelectionChangedInactive;
            EditorSceneManager.sceneSaved += OnSceneSaved;
            ObjectFactory.componentWasAdded += OnComponentWasAdded;
            PrefabUtility.prefabInstanceUpdated += OnPrefabApplied;
            EditorApplication.pauseStateChanged += OnPlayModePause;
            EditorApplication.globalEventHandler += GlobalEventHandler;
#if TIMELINE_FRAMEACCURATE
            TimelinePlayable.playableLooped += OnPlayableLooped;
#endif
        }

        // This callback is needed because the Animation window registers "Animation/Key Selected" as a global hotkey
        // and we want  to also react to the key.
        void GlobalEventHandler()
        {
            if (instance == null || !state.previewMode)
            {
                return;
            }

            var keyBinding = ShortcutManager.instance.GetShortcutBinding("Animation/Key Selected");
            if (keyBinding.Equals(ShortcutBinding.empty))
            {
                return;
            }

            var evtCombo = KeyCombination.FromKeyboardInput(Event.current);
            if (keyBinding.keyCombinationSequence.Contains(evtCombo))
            {
                Invoker.InvokeWithSelected<KeyAllAnimated>();
            }
        }

        void OnEditorQuit()
        {
            TimelineWindowViewPrefs.SaveAll();
        }

        void RemoveEditorCallbacks()
        {
            EditorApplication.playModeStateChanged -= OnPlayModeStateChanged;

            Undo.undoRedoPerformed -= OnUndoRedo;
            Undo.postprocessModifications -= PostprocessAnimationRecordingModifications;
            Undo.postprocessModifications -= ProcessAssetModifications;
            AnimationUtility.onCurveWasModified -= OnCurveModified;
            EditorApplication.editorApplicationQuit -= OnEditorQuit;
            Selection.selectionChanged -= OnSelectionChangedInactive;
            EditorSceneManager.sceneSaved -= OnSceneSaved;
            ObjectFactory.componentWasAdded -= OnComponentWasAdded;
            PrefabUtility.prefabInstanceUpdated -= OnPrefabApplied;
            EditorApplication.pauseStateChanged -= OnPlayModePause;
            EditorApplication.globalEventHandler -= GlobalEventHandler;
#if TIMELINE_FRAMEACCURATE
            TimelinePlayable.playableLooped -= OnPlayableLooped;
#endif
        }

        void OnPlayModePause(PauseState state)
        {
            // in PlayMode, if the timeline is playing, a constant repaint cycle occurs. Pausing the editor
            // breaks the cycle, so this will restart it
            Repaint();
        }

        // Called when a prefab change is applied to the scene.
        // Redraw so control tracks that use prefabs can show changes
        void OnPrefabApplied(GameObject go)
        {
            if (!state.previewMode)
                return;

            // if we added a component this frame, then rebuild, otherwise just let
            //  the individual playable handle the prefab application
            if (Time.frameCount == m_ComponentAddedFrame)
                TimelineEditor.Refresh(RefreshReason.ContentsModified);
            else
                TimelineEditor.Refresh(RefreshReason.SceneNeedsUpdate);
        }

        // When the scene is save the director time will get reset.
        void OnSceneSaved(Scene scene)
        {
            if (state != null)
                state.OnSceneSaved();
        }

        void OnCurveModified(AnimationClip clip, EditorCurveBinding binding, AnimationUtility.CurveModifiedType type)
        {
            InspectorWindow.RepaintAllInspectors();
            if (state == null || state.rebuildGraph)
                return;

            //Force refresh of curve when modified by another editor.
            Repaint();

            if (state.previewMode == false)
                return;

            bool hasPlayable = m_PlayableLookup.GetPlayableFromAnimClip(clip, out Playable playable);

            // mark the timeline clip as dirty
            TimelineClip timelineClip = m_PlayableLookup.GetTimelineClipFromCurves(clip);
            if (timelineClip != null)
                timelineClip.MarkDirty();

            if (type == AnimationUtility.CurveModifiedType.CurveModified)
            {
                if (hasPlayable)
                {
                    playable.SetAnimatedProperties(clip);
                }

                // updates the duration of the graph without rebuilding
                AnimationUtility.SyncEditorCurves(clip); // deleted keys are not synced when this is sent out, so duration could be incorrect
                state.UpdateRootPlayableDuration(state.editSequence.duration);

                bool isRecording = TimelineRecording.IsRecordingAnimationTrack;
                PlayableDirector masterDirector = TimelineEditor.masterDirector;
                bool isGraphValid = masterDirector != null && masterDirector.playableGraph.IsValid();

                // don't evaluate if this is caused by recording on an animation track, the extra evaluation can cause hiccups
                // Prevent graphs to be resurrected  by a changed clip.
                if (!isRecording && isGraphValid)
                    state.Evaluate();
            }
            else if (EditorUtility.IsDirty(clip)) // curve added/removed, or clip added/removed
            {
                state.rebuildGraph |= timelineClip != null || hasPlayable;
            }
        }

        void OnPlayModeStateChanged(PlayModeStateChange playModeState)
        {
            // case 923506 - make sure we save view data before switching modes
            if (playModeState == PlayModeStateChange.ExitingEditMode ||
                playModeState == PlayModeStateChange.ExitingPlayMode)
                TimelineWindowViewPrefs.SaveAll();

            bool isPlaymodeAboutToChange = playModeState == PlayModeStateChange.ExitingEditMode || playModeState == PlayModeStateChange.ExitingPlayMode;

            // Important to stop the graph on any director so temporary objects are properly cleaned up
            if (isPlaymodeAboutToChange && state != null)
                state.Stop();
        }

        UndoPropertyModification[] PostprocessAnimationRecordingModifications(UndoPropertyModification[] modifications)
        {
            DirtyModifiedObjects(modifications);

            var remaining = TimelineRecording.ProcessUndoModification(modifications, state);
            // if we've changed, we need to repaint the sequence window to show clip length changes
            if (remaining != modifications)
            {
                // only update if us or the sequencer window has focus
                // Prevents color pickers and other dialogs from being wrongly dismissed
                bool repaint = (focusedWindow == null) ||
                    (focusedWindow is InspectorWindow) ||
                    (focusedWindow is TimelineWindow);

                if (repaint)
                    Repaint();
            }


            return remaining;
        }

        void DirtyModifiedObjects(UndoPropertyModification[] modifications)
        {
            foreach (var m in modifications)
            {
                if (m.currentValue == null || m.currentValue.target == null)
                    continue;

                var track = m.currentValue.target as TrackAsset;
                var playableAsset = m.currentValue.target as PlayableAsset;
                var editorClip = m.currentValue.target as EditorClip;

                if (track != null)
                {
                    track.MarkDirty();
                }
                else if (playableAsset != null)
                {
                    var clip = TimelineRecording.FindClipWithAsset(state.editSequence.asset, playableAsset);
                    if (clip != null)
                    {
                        clip.MarkDirty();
                    }
                }
                else if (editorClip != null && editorClip.clip != null)
                {
                    editorClip.clip.MarkDirty();
                }
            }
        }

        UndoPropertyModification[] ProcessAssetModifications(UndoPropertyModification[] modifications)
        {
            bool rebuildGraph = false;

            for (int i = 0; i < modifications.Length && !rebuildGraph; i++)
            {
                var mod = modifications[i];

                if (mod.currentValue != null && mod.currentValue.target is IMarker currentMarker)
                {
                    if (currentMarker.parent != null && currentMarker.parent.timelineAsset == state.editSequence.asset)
                    {
                        if (mod.currentValue.target is INotification)
                            TimelineEditor.Refresh(RefreshReason.ContentsModified);
                        else
                            TimelineEditor.Refresh(RefreshReason.WindowNeedsRedraw);
                    }
                }
                else if (mod.previousValue != null && mod.previousValue.target is AvatarMask) // check if an Avatar Mask has been modified
                {
                    rebuildGraph = state.editSequence.asset != null &&
                        state.editSequence.asset.flattenedTracks
                            .OfType<UnityEngine.Timeline.AnimationTrack>()
                            .Any(x => mod.previousValue.target == x.avatarMask);
                }
            }

            if (rebuildGraph)
            {
                state.rebuildGraph = true;
                Repaint();
            }

            return modifications;
        }

        void OnUndoRedo()
        {
            var undos = new List<string>();
            var redos = new List<string>();
            Undo.GetRecords(undos, redos);

            var rebuildAll = redos.Any(x => x.StartsWith("Timeline ")) || undos.Any(x => x.StartsWith("Timeline"));
            var evalNow = redos.Any(x => x.Contains("Edit Curve")) || undos.Any(x => x.Contains("Edit Curve"));
            if (rebuildAll || evalNow)
            {
                ValidateSelection();
                if (state != null)
                {
                    if (evalNow) // when curves change, the new values need to be set in the transform before the inspector handles the undo
                        state.EvaluateImmediate();
                    if (rebuildAll)
                        state.Refresh();
                }
                Repaint();
            }
        }

        static void ValidateSelection()
        {
            //get all the clips in the selection
            var selectedClips = Selection.GetFiltered<EditorClip>(SelectionMode.Unfiltered).Select(x => x.clip);
            foreach (var selectedClip in selectedClips)
            {
                var parent = selectedClip.GetParentTrack();
                if (selectedClip.GetParentTrack() != null)
                {
                    if (!parent.clips.Contains(selectedClip))
                    {
                        SelectionManager.Remove(selectedClip);
                    }
                }
            }
        }

        void OnComponentWasAdded(Component c)
        {
            m_ComponentAddedFrame = Time.frameCount;
            var go = c.gameObject;
            foreach (var seq in state.GetAllSequences())
            {
                if (seq.director == null || seq.asset == null)
                {
                    return;
                }

                var rebind = seq.asset.GetOutputTracks().Any(track => seq.director.GetGenericBinding(track) == go);
                // Either the playable director has a binding for the GameObject or it is a sibling of the director.
                // The second case is needed since we have timeline top level markerTracks that do not have a binding, but
                // are still "targeting" the playable director
                if (rebind || seq.director.gameObject == go)
                {
                    seq.director.RebindPlayableGraphOutputs();
                }
            }
        }

#if TIMELINE_FRAMEACCURATE
        void OnPlayableLooped(Playable timelinePlayable)
        {
            if (state == null || !state.playing || state.masterSequence == null || state.masterSequence.director == null
                || !state.masterSequence.director.playableGraph.IsValid())
                return;
            var masterPlayable = state.masterSequence.director.playableGraph.GetRootPlayable(0);
            if (!masterPlayable.Equals(Playable.Null)
                && masterPlayable.Equals(timelinePlayable)
                && timelinePlayable.GetGraph().IsMatchFrameRateEnabled())
                timelinePlayable.SetTime(0);
        }

#endif
    }
}
