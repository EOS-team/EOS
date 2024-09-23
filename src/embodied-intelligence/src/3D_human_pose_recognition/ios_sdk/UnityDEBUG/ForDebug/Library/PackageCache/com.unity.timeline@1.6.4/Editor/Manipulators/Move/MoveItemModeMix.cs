using System.Collections.Generic;
using System.Linq;
using UnityEngine;
using UnityEngine.Timeline;

namespace UnityEditor.Timeline
{
    class MoveItemModeMix : IMoveItemMode, IMoveItemDrawer
    {
        TimelineClip[] m_ClipsMoved;
        Dictionary<TimelineClip, double> m_OriginalEaseInDuration = new Dictionary<TimelineClip, double>();
        Dictionary<TimelineClip, double> m_OriginalEaseOutDuration = new Dictionary<TimelineClip, double>();

        public void OnTrackDetach(IEnumerable<ItemsPerTrack> itemsGroups)
        {
            // Nothing
        }

        public void HandleTrackSwitch(IEnumerable<ItemsPerTrack> itemsGroups)
        {
            foreach (var itemsGroup in itemsGroups)
            {
                var targetTrack = itemsGroup.targetTrack;
                if (targetTrack != null && itemsGroup.items.Any())
                {
                    var compatible = itemsGroup.items.First().IsCompatibleWithTrack(targetTrack) &&
                        !EditModeUtils.IsInfiniteTrack(targetTrack);
                    var track = compatible ? targetTrack : null;

                    UndoExtensions.RegisterTrack(track, L10n.Tr("Move Items"));
                    EditModeUtils.SetParentTrack(itemsGroup.items, track);
                }
                else
                {
                    EditModeUtils.SetParentTrack(itemsGroup.items, null);
                }
            }
        }

        public bool AllowTrackSwitch()
        {
            return true;
        }

        public double AdjustStartTime(WindowState state, ItemsPerTrack itemsGroup, double time)
        {
            return time;
        }

        public void OnModeClutchEnter(IEnumerable<ItemsPerTrack> itemsGroups)
        {
            // Nothing
        }

        public void OnModeClutchExit(IEnumerable<ItemsPerTrack> itemsGroups)
        {
            // Nothing
        }

        public void BeginMove(IEnumerable<ItemsPerTrack> itemsGroups)
        {
            m_ClipsMoved = itemsGroups.SelectMany(i => i.clips).ToArray();
            foreach (var clip in m_ClipsMoved)
            {
                m_OriginalEaseInDuration[clip] = clip.easeInDuration;
                m_OriginalEaseOutDuration[clip] = clip.easeOutDuration;
            }
        }

        public void UpdateMove(IEnumerable<ItemsPerTrack> itemsGroups)
        {
            //Compute Blends before updating ease values.
            foreach (var t in itemsGroups.Select(i => i.targetTrack).Where(t => t != null))
                t.ComputeBlendsFromOverlaps();
            //Reset to original ease values. The trim operation will calculate the proper blend values.
            foreach (var clip in m_ClipsMoved)
            {
                clip.easeInDuration = m_OriginalEaseInDuration[clip];
                clip.easeOutDuration = m_OriginalEaseOutDuration[clip];
                EditorUtility.SetDirty(clip.asset);
            }
        }

        public void FinishMove(IEnumerable<ItemsPerTrack> itemsGroups)
        {
            var allClips = itemsGroups.Select(i => i.targetTrack)
                .Where(t => t != null).SelectMany(t => t.clips);
            // update easeIn easeOut durations to apply any modifications caused by blends created or modified by clip move.
            foreach (var clip in allClips)
            {
                clip.easeInDuration = clip.easeInDuration;
                clip.easeOutDuration = clip.easeOutDuration;
            }
        }

        public bool ValidateMove(ItemsPerTrack itemsGroup)
        {
            var track = itemsGroup.targetTrack;
            var items = itemsGroup.items;

            if (EditModeUtils.IsInfiniteTrack(track))
            {
                double startTime;
                double stopTime;
                EditModeUtils.GetInfiniteClipBoundaries(track, out startTime, out stopTime);

                return items.All(item =>
                    !EditModeUtils.IsItemWithinRange(item, startTime, stopTime) &&
                    !EditModeUtils.IsRangeWithinItem(startTime, stopTime, item));
            }

            var siblings = ItemsUtils.GetItemsExcept(itemsGroup.targetTrack, items);
            return items.All(item => EditModeMixUtils.GetPlacementValidity(item, siblings) == PlacementValidity.Valid);
        }

        public void DrawGUI(WindowState state, IEnumerable<MovingItems> movingItems, Color color)
        {
            var selectionHasAnyBlendIn = false;
            var selectionHasAnyBlendOut = false;

            foreach (var grabbedItems in movingItems)
            {
                var bounds = grabbedItems.onTrackItemsBounds;

                var counter = 0;
                foreach (var item in grabbedItems.items.OfType<IBlendable>())
                {
                    if (item.hasLeftBlend)
                    {
                        EditModeGUIUtils.DrawBoundsEdge(bounds[counter], color, TrimEdge.Start);
                        selectionHasAnyBlendIn = true;
                    }

                    if (item.hasRightBlend)
                    {
                        EditModeGUIUtils.DrawBoundsEdge(bounds[counter], color, TrimEdge.End);
                        selectionHasAnyBlendOut = true;
                    }
                    counter++;
                }
            }

            if (selectionHasAnyBlendIn && selectionHasAnyBlendOut)
            {
                TimelineCursors.SetCursor(TimelineCursors.CursorType.MixBoth);
            }
            else if (selectionHasAnyBlendIn)
            {
                TimelineCursors.SetCursor(TimelineCursors.CursorType.MixLeft);
            }
            else if (selectionHasAnyBlendOut)
            {
                TimelineCursors.SetCursor(TimelineCursors.CursorType.MixRight);
            }
            else
            {
                TimelineCursors.ClearCursor();
            }
        }
    }
}
