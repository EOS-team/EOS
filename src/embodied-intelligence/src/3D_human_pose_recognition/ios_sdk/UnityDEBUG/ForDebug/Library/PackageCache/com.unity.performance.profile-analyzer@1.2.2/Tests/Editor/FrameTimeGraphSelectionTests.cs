using NUnit.Framework;
using System.Collections.Generic;
using UnityEditor.Performance.ProfileAnalyzer;
using UnityEngine;
using System.Text;

public class FrameTimeGraphSelectionTests
{
    static readonly MoveTestConfiguration[] k_MoveTestConfigurations =
    {
        new MoveTestConfiguration(new TestConfiguration.SelectionRange[]
        {
            new TestConfiguration.SelectionRange(0, 10)
        }, 10),    // range[0-9] + 10
        new MoveTestConfiguration(new TestConfiguration.SelectionRange[]
        {
            new TestConfiguration.SelectionRange(20, 10)
        }, -10),    // range[20-29] - 10
        new MoveTestConfiguration(new TestConfiguration.SelectionRange[]
        {
            new TestConfiguration.SelectionRange(50, 10),
            new TestConfiguration.SelectionRange(100, 10)
        }, 10),    // multi-select range[50-59][100-109] + 10
        new MoveTestConfiguration(new TestConfiguration.SelectionRange[]
        {
            new TestConfiguration.SelectionRange(50, 10),
            new TestConfiguration.SelectionRange(100, 10)
        }, -10),    // multi-select range[50-59][100-109] - 10
    };

    // Expects a 300 frame capture.
    static readonly MoveTestConfiguration[] k_MoveClampToBoundsTestConfigurations =
    {
        new MoveTestConfiguration(new TestConfiguration.SelectionRange[]
        {
            new TestConfiguration.SelectionRange(0, 10)
        }, -10),    // range[0-9] - 10
        new MoveTestConfiguration(new TestConfiguration.SelectionRange[]
        {
            new TestConfiguration.SelectionRange(290, 10)
        }, 10),    // range[290-299] + 10
        new MoveTestConfiguration(new TestConfiguration.SelectionRange[]
        {
            new TestConfiguration.SelectionRange(0, 10),
            new TestConfiguration.SelectionRange(50, 10)
        }, -10),    // multi-select range[0-9][50-59] - 10
        new MoveTestConfiguration(new TestConfiguration.SelectionRange[]
        {
            new TestConfiguration.SelectionRange(0, 10),
            new TestConfiguration.SelectionRange(290, 10)
        }, 10),    // multi-select range[0-9][290-299] + 10
    };

    static readonly ResizeTestConfiguration[] k_ResizeTestConfigurations =
    {
        new ResizeTestConfiguration(new TestConfiguration.SelectionRange[]
        {
            new TestConfiguration.SelectionRange(100, 100)
        }, -10, 10),    // range[100-199] grow [-10, 10]
        new ResizeTestConfiguration(new TestConfiguration.SelectionRange[]
        {
            new TestConfiguration.SelectionRange(100, 100)
        }, 10, -10),    // range[100-199] shrink [10, -10]
        new ResizeTestConfiguration(new TestConfiguration.SelectionRange[]
        {
            new TestConfiguration.SelectionRange(50, 50),
            new TestConfiguration.SelectionRange(200, 50)
        }, -10, 10),    // multi-select range[50-99][200-249] grow [-10, 10]
        new ResizeTestConfiguration(new TestConfiguration.SelectionRange[]
        {
            new TestConfiguration.SelectionRange(50, 50),
            new TestConfiguration.SelectionRange(200, 50)
        }, 10, -10),    // multi-select range[50-99][200-249] shrink [10, -10]
        new ResizeTestConfiguration(new TestConfiguration.SelectionRange[]
        {
            new TestConfiguration.SelectionRange(50, 50),
            new TestConfiguration.SelectionRange(200, 50)
        }, -10, 0),    // multi-select range[50-99][200-249] grow left [-10, 0]
        new ResizeTestConfiguration(new TestConfiguration.SelectionRange[]
        {
            new TestConfiguration.SelectionRange(50, 50),
            new TestConfiguration.SelectionRange(200, 50)
        }, 0, 10),    // multi-select range[50-99][200-249] grow right [0, 10]
        new ResizeTestConfiguration(new TestConfiguration.SelectionRange[]
        {
            new TestConfiguration.SelectionRange(50, 50),
            new TestConfiguration.SelectionRange(200, 50)
        }, 10, 0),    // multi-select range[50-99][200-249] shrink left [10, 0]
        new ResizeTestConfiguration(new TestConfiguration.SelectionRange[]
        {
            new TestConfiguration.SelectionRange(50, 50),
            new TestConfiguration.SelectionRange(200, 50)
        }, 0, -10),    // multi-select range[50-99][200-249] shrink right [0, -10]
    };

    // Expects a 300 frame capture.
    static readonly ResizeTestConfiguration[] k_ResizeClampToBoundsTestConfigurations =
    {
        new ResizeTestConfiguration(new TestConfiguration.SelectionRange[]
        {
            new TestConfiguration.SelectionRange(0, 10)
        }, -10, 0),    // range[0-9] grow left [-10, 0]
        new ResizeTestConfiguration(new TestConfiguration.SelectionRange[]
        {
            new TestConfiguration.SelectionRange(290, 10)
        }, 0, 10),    // range[290-299] grow right [0, 10]
        new ResizeTestConfiguration(new TestConfiguration.SelectionRange[]
        {
            new TestConfiguration.SelectionRange(0, 10),
            new TestConfiguration.SelectionRange(50, 10)
        }, -10, 0),    // range[0-9][50-59] grow left [-10, 0]
        new ResizeTestConfiguration(new TestConfiguration.SelectionRange[]
        {
            new TestConfiguration.SelectionRange(0, 10),
            new TestConfiguration.SelectionRange(290, 10)
        }, 0, 10),    // range[0-9][290-299] grow right [0, 10]
    };

    FrameTimeGraph m_FrameTimeGraph;
    List<int> m_ReportedSelection;

    [OneTimeSetUp]
    public void OneTimeSetUp()
    {
        var data = GenerateFrameTimeGraphData();
        m_FrameTimeGraph = NewFrameTimeGraph();
        m_FrameTimeGraph.SetData(data);
        m_FrameTimeGraph.SetRangeCallback(OnFrameTimeGraphDidSetRange);
    }

    [SetUp]
    public void SetUp()
    {
        m_ReportedSelection = new List<int>();
    }

    [Test]
    public void FrameTimeGraph_MoveSelectedRange([ValueSource("k_MoveTestConfigurations")] MoveTestConfiguration configuration)
    {
        List<int> expectedSelection = ExpectedSelectedFramesForMoveTestConfiguration(configuration);
        FrameTimeGraph_MoveSelectedRange(configuration, expectedSelection);
    }

    [Test]
    public void FrameTimeGraph_MoveSelectedRange_DoesNotMovePastGraphBounds([ValueSource("k_MoveClampToBoundsTestConfigurations")] MoveTestConfiguration configuration)
    {
        List<int> expectedSelection = InitialSelectedFramesForTestConfiguration(configuration);
        FrameTimeGraph_MoveSelectedRange(configuration, expectedSelection);
    }

    void FrameTimeGraph_MoveSelectedRange(MoveTestConfiguration configuration, List<int> expectedSelection)
    {
        var offset = configuration.offset;
        int clickCount = 1;
        bool singleClickAction = true;
        var currentSelectionState = SelectedRangeStateFromTestConfiguration(configuration);

        m_FrameTimeGraph.MoveSelectedRange(offset, clickCount, singleClickAction, FrameTimeGraph.State.None, currentSelectionState);

        CollectionAssert.AreEqual(expectedSelection, m_ReportedSelection);
    }

    [Test]
    public void FrameTimeGraph_ResizeSelectedRange([ValueSource("k_ResizeTestConfigurations")] ResizeTestConfiguration configuration)
    {
        List<int> expectedSelection = ExpectedSelectedFramesForResizeTestConfiguration(configuration);
        FrameTimeGraph_ResizeSelectedRange(configuration, expectedSelection);
    }

    [Test]
    public void FrameTimeGraph_ResizeSelectedRange_DoesNotMovePastGraphBounds([ValueSource("k_ResizeClampToBoundsTestConfigurations")] ResizeTestConfiguration configuration)
    {
        List<int> expectedSelection = InitialSelectedFramesForTestConfiguration(configuration);
        FrameTimeGraph_ResizeSelectedRange(configuration, expectedSelection);
    }

    void FrameTimeGraph_ResizeSelectedRange(ResizeTestConfiguration configuration, List<int> expectedSelection)
    {
        var leftOffset = configuration.leftOffset;
        var rightOffset = configuration.rightOffset;
        int clickCount = 1;
        bool singleClickAction = true;
        var currentSelectionState = SelectedRangeStateFromTestConfiguration(configuration);

        m_FrameTimeGraph.ResizeSelectedRange(leftOffset, rightOffset, clickCount, singleClickAction, FrameTimeGraph.State.None, currentSelectionState);

        CollectionAssert.AreEqual(expectedSelection, m_ReportedSelection);
    }

    FrameTimeGraph NewFrameTimeGraph()
    {
        var draw2D = new Draw2D("Unlit/ProfileAnalyzerShader");
        DisplayUnits displayUnits = new DisplayUnits(Units.Milliseconds);
        return new FrameTimeGraph(0, draw2D, displayUnits.Units, ProfileAnalyzerWindow.UIColor.barBackground, ProfileAnalyzerWindow.UIColor.barBackgroundSelected, ProfileAnalyzerWindow.UIColor.bar, ProfileAnalyzerWindow.UIColor.barSelected, ProfileAnalyzerWindow.UIColor.marker, ProfileAnalyzerWindow.UIColor.markerSelected, ProfileAnalyzerWindow.UIColor.thread, ProfileAnalyzerWindow.UIColor.threadSelected, ProfileAnalyzerWindow.UIColor.gridLines);
    }

    List<FrameTimeGraph.Data> GenerateFrameTimeGraphData()
    {
        const int k_DataLength = 300;
        var data = new List<FrameTimeGraph.Data>(k_DataLength);
        int i = 0;
        while (i < k_DataLength)
        {
            var frameData = new FrameTimeGraph.Data(Random.value * 16, i);
            data.Add(frameData);
            i++;
        }

        return data;
    }

    void OnFrameTimeGraphDidSetRange(List<int> selected, int clickCount, FrameTimeGraph.State inputStatus)
    {
        m_ReportedSelection = selected;
    }

    FrameTimeGraph.SelectedRangeState SelectedRangeStateFromTestConfiguration(TestConfiguration configuration)
    {
        List<int> selectedFrames = InitialSelectedFramesForTestConfiguration(configuration);
        int currentSelectionFirstDataOffset;
        int currentSelectionLastDataOffset;
        int firstFrameOffset;
        int lastFrameOffset;
        m_FrameTimeGraph.GetSelectedRange(selectedFrames, out currentSelectionFirstDataOffset, out currentSelectionLastDataOffset, out firstFrameOffset, out lastFrameOffset);

        return new FrameTimeGraph.SelectedRangeState()
        {
            currentSelectionFirstDataOffset = currentSelectionFirstDataOffset,
            currentSelectionLastDataOffset = currentSelectionLastDataOffset,
            lastSelectedFrameOffsets = selectedFrames,
        };
    }

    List<int> InitialSelectedFramesForTestConfiguration(TestConfiguration configuration)
    {
        List<int> selectedFrames = new List<int>();
        foreach (var selectionRange in configuration.selections)
        {
            var selectionFrames = GenerateListOfFrames(selectionRange.origin, selectionRange.length);
            selectedFrames.AddRange(selectionFrames);
        }

        return selectedFrames;
    }

    List<int> ExpectedSelectedFramesForMoveTestConfiguration(MoveTestConfiguration configuration)
    {
        List<int> selectedFrames = new List<int>();
        var offset = configuration.offset;
        foreach (var selectionRange in configuration.selections)
        {
            var selectionFrames = GenerateListOfFrames(selectionRange.origin + offset, selectionRange.length);
            selectedFrames.AddRange(selectionFrames);
        }

        return selectedFrames;
    }

    List<int> ExpectedSelectedFramesForResizeTestConfiguration(ResizeTestConfiguration configuration)
    {
        List<int> selectedFrames = new List<int>();
        var leftOffset = configuration.leftOffset;
        var rightOffset = configuration.rightOffset;
        foreach (var selectionRange in configuration.selections)
        {
            var leftIndex = selectionRange.origin + leftOffset;
            var rightIndex = selectionRange.LastIndex + rightOffset;
            var selectionLength = rightIndex - leftIndex + 1;
            var selectionFrames = GenerateListOfFrames(selectionRange.origin + leftOffset, selectionLength);
            selectedFrames.AddRange(selectionFrames);
        }

        return selectedFrames;
    }

    List<int> GenerateListOfFrames(int origin, int count)
    {
        var frames = new List<int>();

        int i = 0;
        while (i < count)
        {
            frames.Add(origin + i);
            ++i;
        }

        return frames;
    }

    public class TestConfiguration
    {
        public SelectionRange[] selections;

        public TestConfiguration(SelectionRange[] selections)
        {
            this.selections = selections;
        }

        protected string SelectionsToString()
        {
            var stringBuilder = new StringBuilder();
            for (int i = 0; i < selections.Length; ++i)
            {
                var selection = selections[i];
                stringBuilder.AppendFormat("[{0}-{1}]", selection.origin, selection.LastIndex);
            }

            return stringBuilder.ToString();
        }

        public struct SelectionRange
        {
            public int origin;
            public int length;

            public SelectionRange(int origin, int length)
            {
                this.origin = origin;
                this.length = length;
            }

            public int LastIndex
            {
                get
                {
                    return origin + length - 1;
                }
            }
        }
    }

    public class MoveTestConfiguration : TestConfiguration
    {
        public int offset;

        public MoveTestConfiguration(SelectionRange[] selections, int offset) : base(selections)
        {
            this.offset = offset;
        }

        public override string ToString()
        {
            var stringBuilder = new StringBuilder();
            if (selections.Length > 1)
            {
                stringBuilder.Append("multi-range");
            }
            else
            {
                stringBuilder.Append("range");
            }

            stringBuilder.Append(SelectionsToString());
            stringBuilder.Append(" | ");

            stringBuilder.AppendFormat("{0}[{1}]", (offset > 0) ? "right" : "left", offset);

            return stringBuilder.ToString();
        }
    }

    public class ResizeTestConfiguration : TestConfiguration
    {
        public int leftOffset;
        public int rightOffset;

        public ResizeTestConfiguration(SelectionRange[] selections, int leftOffset, int rightOffset) : base(selections)
        {
            this.leftOffset = leftOffset;
            this.rightOffset = rightOffset;
        }

        public override string ToString()
        {
            var stringBuilder = new StringBuilder();
            if (selections.Length > 1)
            {
                stringBuilder.Append("multi-range");
            }
            else
            {
                stringBuilder.Append("range");
            }

            stringBuilder.Append(SelectionsToString());
            stringBuilder.Append(" | ");

            bool hasLeftOffset = leftOffset != 0;
            if (hasLeftOffset)
            {
                var leftAction = (leftOffset < 0) ? "grow-left" : "shrink-left";
                stringBuilder.AppendFormat("{0}[{1}]", leftAction, leftOffset);
            }

            if (rightOffset != 0)
            {
                if (hasLeftOffset)
                {
                    stringBuilder.Append(" ");
                }

                var rightAction = (rightOffset > 0) ? "grow-right" : "shrink-right";
                stringBuilder.AppendFormat("{0}[{1}]", rightAction, rightOffset);
            }

            return stringBuilder.ToString();
        }
    }
}
