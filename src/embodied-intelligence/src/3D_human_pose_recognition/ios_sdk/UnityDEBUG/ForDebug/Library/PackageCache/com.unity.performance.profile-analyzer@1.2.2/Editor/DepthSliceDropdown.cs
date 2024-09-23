using System;
using UnityEngine;
using System.Collections.Generic;
using System.Reflection;
using UnityEditor.IMGUI.Controls;

namespace UnityEditor.Performance.ProfileAnalyzer
{
    class DepthSliceDropdown : AdvancedDropdown
    {
        class DepthSliceDropdownItem : AdvancedDropdownItem
        {
            public int depthSlice;
            public int depthSliceLeft;
            public int depthSliceRight;
            public DepthSliceDropdownItem(int depthSlice)
                : base(DepthSliceUI.DepthFilterToString(depthSlice))
            {
                this.depthSlice = depthSlice;
                depthSliceLeft = depthSlice;
                depthSliceRight = depthSlice;
            }
            public DepthSliceDropdownItem(int depthSliceLeft, int depthSliceRight, bool leftIsMain)
                : base(DepthSliceUI.DepthFilterToString(depthSliceLeft, depthSliceRight, leftIsMain))
            {
                depthSlice = Math.Max(depthSliceLeft, depthSliceRight);
                this.depthSliceLeft = depthSliceLeft;
                this.depthSliceRight = depthSliceRight;
            }
        }

        Action<int, int, int> m_Callback = null;
        int m_DepthSliceCount;
        int m_DepthSliceCountRight;
        int m_CurrentDepthSliceA;
        int m_CurrentDepthSliceB;
        int m_DepthDiff;

        static FieldInfo m_DataSourceFieldInfo;
        static Type m_DataSourceTypeInfo;
        static PropertyInfo m_SelectedIdsFieldInfo;

        public DepthSliceDropdown(int depthSliceCount, int currentDepthSliceA, int currentDepthSliceB, Action<int, int, int> callback, int depthDiff, int depthSliceCountRight = ProfileAnalyzer.kDepthAll) : base(new AdvancedDropdownState())
        {
            m_DepthSliceCount = depthSliceCount;
            m_DepthSliceCountRight = depthSliceCountRight;
            m_CurrentDepthSliceA = currentDepthSliceA;
            m_CurrentDepthSliceB = currentDepthSliceB;
            m_Callback = callback;
            m_DepthDiff = depthDiff;
            if (m_DataSourceFieldInfo == null || m_DataSourceFieldInfo == null || m_SelectedIdsFieldInfo == null)
            {
                Assembly assem = typeof(AdvancedDropdown).Assembly;
                var advancedDropdownTypeInfo = typeof(AdvancedDropdown);
                m_DataSourceTypeInfo = assem.GetType("UnityEditor.IMGUI.Controls.CallbackDataSource");
                m_DataSourceFieldInfo = advancedDropdownTypeInfo.GetField("m_DataSource", BindingFlags.NonPublic | BindingFlags.Instance);
                m_SelectedIdsFieldInfo = m_DataSourceTypeInfo.GetProperty("selectedIDs", BindingFlags.Public | BindingFlags.Instance);
            }
        }

        protected override AdvancedDropdownItem BuildRoot()
        {
            var root = new AdvancedDropdownItem("Depth Slice");
            var allItem = new DepthSliceDropdownItem(ProfileAnalyzer.kDepthAll);
            root.AddChild(allItem);
            if (m_CurrentDepthSliceA == ProfileAnalyzer.kDepthAll && m_CurrentDepthSliceB == ProfileAnalyzer.kDepthAll)
                (m_SelectedIdsFieldInfo.GetValue(m_DataSourceFieldInfo.GetValue(this)) as List<int>).Add(allItem.id);
            var count = m_DepthSliceCountRight == ProfileAnalyzer.kDepthAll ? m_DepthSliceCount :
                Math.Max(m_DepthSliceCount + Math.Max(0, m_DepthDiff), m_DepthSliceCountRight - Math.Min(0, m_DepthDiff));

            var leftIsMain = m_DepthDiff < 0;
            var mainThreshold = leftIsMain ? m_DepthSliceCount : m_DepthSliceCountRight;
            var secondaryMinThreshold = Math.Abs(m_DepthDiff);
            var secondaryMaxThreshold = (leftIsMain ? m_DepthSliceCountRight : m_DepthSliceCount) + secondaryMinThreshold;

            var startIndex = 1;
            for (int i = startIndex; i <= count; i++)
            {
                var selected = false;
                AdvancedDropdownItem child;
                if (m_DepthSliceCountRight != ProfileAnalyzer.kDepthAll)
                {
                    var left = Mathf.Clamp(i - Math.Max(0, m_DepthDiff), 1, m_DepthSliceCount);
                    var right = Mathf.Clamp(i - Math.Max(0, -m_DepthDiff), 1, m_DepthSliceCountRight);

                    if (m_DepthSliceCount <= 0)
                        left = -1;
                    else if (m_DepthSliceCountRight <= 0)
                        right = -1;
                    else
                    {
                        // Separators only make sense if there is data on both sides

                        // did we pass the threshold of the main's max depth and started clamping it down?
                        if (i == mainThreshold + 1
                        // ... or the threshold of the secondary's negative depth when adjusted for the depth diff, and stoped clamping it up?
                            || (secondaryMinThreshold != 0 && i == secondaryMinThreshold + 1)
                        // ... or the threshold of the secondary's max depth when adjusted for the depth diff, and started clamping it down?
                            || (i == secondaryMaxThreshold + 1))
                            root.AddSeparator();
                    }

                    child = new DepthSliceDropdownItem(left, right, leftIsMain);
                    selected = m_CurrentDepthSliceA == left && m_CurrentDepthSliceB == right;
                }
                else
                {
                    child = new DepthSliceDropdownItem(i);
                    selected = m_CurrentDepthSliceA == i;
                }
                root.AddChild(child);
                if (selected)
                    (m_SelectedIdsFieldInfo.GetValue(m_DataSourceFieldInfo.GetValue(this)) as List<int>).Add(child.id);
            }
            return root;
        }

        protected override void ItemSelected(AdvancedDropdownItem item)
        {
            base.ItemSelected(item);
            if (m_Callback != null)
            {
                var sliceItem = (item as DepthSliceDropdownItem);
                m_Callback(sliceItem.depthSlice, sliceItem.depthSliceLeft, sliceItem.depthSliceRight);
            }
        }
    }
}
