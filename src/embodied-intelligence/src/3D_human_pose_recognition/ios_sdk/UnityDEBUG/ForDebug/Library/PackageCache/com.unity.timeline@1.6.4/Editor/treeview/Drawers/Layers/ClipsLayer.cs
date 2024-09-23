using System.Collections.Generic;
using UnityEngine;
using UnityEngine.Timeline;

namespace UnityEditor.Timeline
{
    class ClipsLayer : ItemsLayer<TimelineClipGUI>
    {
        static readonly GUIStyle k_ConnectorIcon = DirectorStyles.Instance.connector;

        public ClipsLayer(Layer layerOrder, IRowGUI parent) : base(layerOrder)
        {
            var track = parent.asset;
            track.SortClips();
            TimelineClipGUI previousClipGUI = null;

            foreach (var clip in track.clips)
            {
                var oldClipGUI = ItemToItemGui.GetGuiForClip(clip);
                var isInvalid = oldClipGUI != null && oldClipGUI.isInvalid;  // HACK Make sure to carry invalidy state when refereshing the cache.

                var currentClipGUI = new TimelineClipGUI(clip, parent, this) { isInvalid = isInvalid };
                if (previousClipGUI != null) previousClipGUI.nextClip = currentClipGUI;
                currentClipGUI.previousClip = previousClipGUI;
                AddItem(currentClipGUI);
                previousClipGUI = currentClipGUI;
            }

            //adjust zOrder based on current clip selection
            foreach (var clipGUI in items)
            {
                if (clipGUI.IsSelected())
                    clipGUI.MoveToTop();
            }
        }

        public override void Draw(Rect rect, WindowState state)
        {
            base.Draw(rect, state); //draw clips
            DrawConnector(items);
        }

        static void DrawConnector(List<TimelineClipGUI> clips)
        {
            if (Event.current.type != EventType.Repaint)
                return;

            foreach (var clip in clips)
            {
                if (clip.previousClip != null && clip.visible && clip.treeViewRect.width > 14 &&
                    (DiscreteTime)clip.start == (DiscreteTime)clip.previousClip.end)
                {
                    // draw little connector widget
                    var localRect = clip.treeViewRect;
                    localRect.x -= Mathf.Floor(k_ConnectorIcon.fixedWidth / 2.0f);
                    localRect.width = k_ConnectorIcon.fixedWidth;
                    localRect.height = k_ConnectorIcon.fixedHeight;
                    GUI.Label(localRect, GUIContent.none, k_ConnectorIcon);
                }
            }
        }
    }
}
