using System;
using System.Collections.Generic;
using UnityEngine;
using UnityEditorInternal;
using System.IO;
using System.Runtime.Serialization.Formatters.Binary;
using System.Text.RegularExpressions;

namespace UnityEditor.Performance.ProfileAnalyzer
{
    [Serializable]
    internal class ProfileData
    {
        static int latestVersion = 7;
        /*
        Version 1 - Initial version. Thread names index:threadName (Some invalid thread names count:threadName index)
        Version 2 - Added frame start time.
        Version 3 - Saved out marker children times in the data (Never needed so rapidly skipped)
        Version 4 - Removed the child times again (at this point data was saved with 1 less frame at start and end)
        Version 5 - Updated the thread names to include the thread group as a prefix (index:threadGroup.threadName, index is 1 based, original is 0 based)
        Version 6 - fixed msStartTime (previously was 'seconds')
        Version 7 - Data now only skips the frame at the end
        */
        static Regex trailingDigit = new Regex(@"^(.*[^\s])[\s]+([\d]+)$", RegexOptions.Compiled);
        public int Version { get; private set; }
        public int FrameIndexOffset { get; private set; }
        public bool FirstFrameIncomplete;
        public bool LastFrameIncomplete;
        List<ProfileFrame> frames = new List<ProfileFrame>();
        List<string> markerNames = new List<string>();
        List<string> threadNames = new List<string>();
        Dictionary<string, int> markerNamesDict = new Dictionary<string, int>();
        Dictionary<string, int> threadNameDict = new Dictionary<string, int>();
        public string FilePath { get; private set; }
        public int MarkerNameCount => markerNames.Count;
        static float s_Progress = 0;

        public ProfileData()
        {
            FrameIndexOffset = 0;
            FilePath = string.Empty;
            Version = latestVersion;
        }

        public ProfileData(string filename)
        {
            FrameIndexOffset = 0;
            FilePath = filename;
            Version = latestVersion;
        }

        void Read()
        {
            if (string.IsNullOrEmpty(FilePath))
                throw new Exception("File path is invalid");

            using (var reader = new BinaryReader(File.Open(FilePath, FileMode.Open)))
            {
                s_Progress = 0;
                Version = reader.ReadInt32();
                if (Version < 0 || Version > latestVersion)
                {
                    throw new Exception(String.Format("File version unsupported: {0} != {1} expected, at path: {2}", Version, latestVersion, FilePath));
                }

                FrameIndexOffset = reader.ReadInt32();
                int frameCount = reader.ReadInt32();
                frames.Clear();
                for (int frame = 0; frame < frameCount; frame++)
                {
                    frames.Add(new ProfileFrame(reader, Version));
                    s_Progress = (float)frame / frameCount;
                }

                int markerCount = reader.ReadInt32();
                markerNames.Clear();
                for (int marker = 0; marker < markerCount; marker++)
                {
                    markerNames.Add(reader.ReadString());
                    s_Progress = (float)marker / markerCount;
                }

                int threadCount = reader.ReadInt32();
                threadNames.Clear();
                for (int thread = 0; thread < threadCount; thread++)
                {
                    var threadNameWithIndex = reader.ReadString();

                    threadNameWithIndex = CorrectThreadName(threadNameWithIndex);

                    threadNames.Add(threadNameWithIndex);
                    s_Progress = (float)thread / threadCount;
                }
            }
        }

        internal void DeleteTmpFiles()
        {
            if (ProfileAnalyzerWindow.FileInTempDir(FilePath))
                File.Delete(FilePath);
        }

        bool IsFrameSame(int frameIndex, ProfileData other)
        {
            ProfileFrame thisFrame = GetFrame(frameIndex);
            ProfileFrame otherFrame = other.GetFrame(frameIndex);
            return thisFrame.IsSame(otherFrame);
        }

        public bool IsSame(ProfileData other)
        {
            if (other == null)
                return false;

            int frameCount = GetFrameCount();
            if (frameCount != other.GetFrameCount())
            {
                // Frame counts differ
                return false;
            }

            if (frameCount == 0)
            {
                // Both empty
                return true;
            }

            if (!IsFrameSame(0, other))
                return false;
            if (!IsFrameSame(frameCount - 1, other))
                return false;

            // Close enough if same number of frames and first/last have exactly the same frame time and time offset.
            // If we see false matches we could add a full has of the data on load/pull
            return true;
        }

        static public string ThreadNameWithIndex(int index, string threadName)
        {
            return string.Format("{0}:{1}", index, threadName);
        }

        public void SetFrameIndexOffset(int offset)
        {
            FrameIndexOffset = offset;
        }

        public int GetFrameCount()
        {
            return frames.Count;
        }

        public ProfileFrame GetFrame(int offset)
        {
            if (offset < 0 || offset >= frames.Count)
                return null;

            return frames[offset];
        }

        public List<string> GetMarkerNames()
        {
            return markerNames;
        }

        public List<string> GetThreadNames()
        {
            return threadNames;
        }

        public int GetThreadCount()
        {
            return threadNames.Count;
        }

        public int OffsetToDisplayFrame(int offset)
        {
            return offset + (1 + FrameIndexOffset);
        }

        public int DisplayFrameToOffset(int displayFrame)
        {
            return displayFrame - (1 + FrameIndexOffset);
        }

        public void AddThreadName(string threadName, ProfileThread thread)
        {
            threadName = CorrectThreadName(threadName);

            int index = -1;

            if (!threadNameDict.TryGetValue(threadName, out index))
            {
                threadNames.Add(threadName);
                index = threadNames.Count - 1;

                threadNameDict.Add(threadName, index);
            }

            thread.threadIndex = index;
        }

        public void AddMarkerName(string markerName, ProfileMarker marker)
        {
            int index = -1;
            if (!markerNamesDict.TryGetValue(markerName, out index))
            {
                markerNames.Add(markerName);
                index = markerNames.Count - 1;

                markerNamesDict.Add(markerName, index);
            }

            marker.nameIndex = index;
        }

        public string GetThreadName(ProfileThread thread)
        {
            return threadNames[thread.threadIndex];
        }

        public string GetMarkerName(ProfileMarker marker)
        {
            return markerNames[marker.nameIndex];
        }

        public int GetMarkerIndex(string markerName)
        {
            for (int nameIndex = 0; nameIndex < markerNames.Count; ++nameIndex)
            {
                if (markerName == markerNames[nameIndex])
                    return nameIndex;
            }
            return -1;
        }

        public void Add(ProfileFrame frame)
        {
            frames.Add(frame);
        }

        void WriteInternal(string filepath)
        {
            using (var writer = new BinaryWriter(File.Open(filepath, FileMode.OpenOrCreate)))
            {
                Version = latestVersion;

                writer.Write(Version);
                writer.Write(FrameIndexOffset);

                writer.Write(frames.Count);
                foreach (var frame in frames)
                {
                    frame.Write(writer);
                }

                writer.Write(markerNames.Count);
                foreach (var markerName in markerNames)
                {
                    writer.Write(markerName);
                }

                writer.Write(threadNames.Count);
                foreach (var threadName in threadNames)
                {
                    writer.Write(threadName);
                }
            }
        }

        internal void Write()
        {
            //ensure that we can always write to the temp location at least
            if (string.IsNullOrEmpty(FilePath))
                FilePath = ProfileAnalyzerWindow.TmpPath;

            WriteInternal(FilePath);
        }

        internal void WriteTo(string path)
        {
            //no point in trying to save on top of ourselves
            if (path == FilePath)
                return;

            if (!string.IsNullOrEmpty(FilePath) && File.Exists(FilePath))
            {
                if (File.Exists(path))
                    File.Delete(path);

                File.Copy(FilePath, path);
            }
            else
            {
                WriteInternal(path);
            }
            FilePath = path;
        }

        public static string CorrectThreadName(string threadNameWithIndex)
        {
            var info = threadNameWithIndex.Split(':');
            if (info.Length >= 2)
            {
                string threadGroupIndexString = info[0];
                string threadName = info[1];
                if (threadName.Trim() == "")
                {
                    // Scan seen with no thread name
                    threadNameWithIndex = string.Format("{0}:[Unknown]", threadGroupIndexString);
                }
                else
                {
                    // Some scans have thread names such as
                    // "1:Worker Thread 0"
                    // "1:Worker Thread 1"
                    // rather than
                    // "1:Worker Thread"
                    // "2:Worker Thread"
                    // Update to the second format so the 'All' case is correctly determined
                    Match m = trailingDigit.Match(threadName);
                    if (m.Success)
                    {
                        string threadNamePrefix = m.Groups[1].Value;
                        int threadGroupIndex = 1 + int.Parse(m.Groups[2].Value);

                        threadNameWithIndex = string.Format("{0}:{1}", threadGroupIndex, threadNamePrefix);
                    }
                }
            }

            threadNameWithIndex = threadNameWithIndex.Trim();

            return threadNameWithIndex;
        }

        public static string GetThreadNameWithGroup(string threadName, string groupName)
        {
            if (string.IsNullOrEmpty(groupName))
                return threadName;

            return string.Format("{0}.{1}", groupName, threadName);
        }

        public static string GetThreadNameWithoutGroup(string threadNameWithGroup, out string groupName)
        {
            string[] tokens = threadNameWithGroup.Split('.');
            if (tokens.Length <= 1)
            {
                groupName = "";
                return tokens[0];
            }

            groupName = tokens[0];
            return tokens[1].TrimStart();
        }

        internal bool HasFrames
        {
            get
            {
                return frames != null && frames.Count > 0;
            }
        }

        internal bool HasThreads
        {
            get
            {
                return frames[0].threads != null && frames[0].threads.Count > 0;
            }
        }

        internal bool NeedsMarkerRebuild
        {
            get
            {
                if (frames.Count > 0 && frames[0].threads.Count > 0)
                    return frames[0].threads[0].markers.Count != frames[0].threads[0].markerCount;

                return false;
            }
        }

        public static bool Save(string filename, ProfileData data)
        {
            if (data == null)
                return false;

            if (string.IsNullOrEmpty(filename))
                return false;

            if (filename.EndsWith(".json"))
            {
                var json = JsonUtility.ToJson(data);
                File.WriteAllText(filename, json);
            }
            else if (filename.EndsWith(".padata"))
            {
                FileStream stream = File.Create(filename);
                var formatter = new BinaryFormatter();
                formatter.Serialize(stream, data);
                stream.Close();
            }
            else if (filename.EndsWith(".pdata"))
            {
                data.WriteTo(filename);
            }

            return true;
        }

        public static bool Load(string filename, out ProfileData data)
        {
            if (filename.EndsWith(".json"))
            {
                string json = File.ReadAllText(filename);
                data = JsonUtility.FromJson<ProfileData>(json);
            }
            else if (filename.EndsWith(".padata"))
            {
                FileStream stream = File.OpenRead(filename);
                var formatter = new BinaryFormatter();
                data = (ProfileData)formatter.Deserialize(stream);
                stream.Close();

                if (data.Version != latestVersion)
                {
                    Debug.Log(String.Format("Unable to load file. Incorrect file version in {0} : (file {1} != {2} expected", filename, data.Version, latestVersion));
                    data = null;
                    return false;
                }
            }
            else if (filename.EndsWith(".pdata"))
            {
                if (!File.Exists(filename))
                {
                    data = null;
                    return false;
                }

                try
                {
                    data = new ProfileData(filename);
                    data.Read();
                }
                catch (Exception e)
                {
                    var message = e.Message;
                    if (!string.IsNullOrEmpty(message))
                        Debug.Log(e.Message);
                    data = null;
                    return false;
                }
            }
            else
            {
                string errorMessage;
                if (filename.EndsWith(".data"))
                {
                    errorMessage = "Unable to load file. Profiler captures (.data) should be loaded in the Profiler Window and then pulled into the Analyzer via its Pull Data button.";
                }
                else
                {
                    errorMessage = string.Format("Unable to load file. Unsupported file format: '{0}'.", Path.GetExtension(filename));
                }

                Debug.Log(errorMessage);
                data = null;
                return false;
            }

            data.Finalise();
            return true;
        }

        void PushMarker(Stack<ProfileMarker> markerStack, ProfileMarker markerData)
        {
            Debug.Assert(markerData.depth == markerStack.Count + 1);
            markerStack.Push(markerData);
        }

        ProfileMarker PopMarkerAndRecordTimeInParent(Stack<ProfileMarker> markerStack)
        {
            ProfileMarker child = markerStack.Pop();

            ProfileMarker parentMarker = (markerStack.Count > 0) ? markerStack.Peek() : null;

            // Record the last markers time in its parent
            if (parentMarker != null)
                parentMarker.msChildren += child.msMarkerTotal;

            return parentMarker;
        }

        public void Finalise()
        {
            CalculateMarkerChildTimes();
            markerNamesDict.Clear();
        }

        void CalculateMarkerChildTimes()
        {
            var markerStack = new Stack<ProfileMarker>();

            for (int frameOffset = 0; frameOffset <= frames.Count; ++frameOffset)
            {
                var frameData = GetFrame(frameOffset);
                if (frameData == null)
                    continue;

                for (int threadIndex = 0; threadIndex < frameData.threads.Count; threadIndex++)
                {
                    var threadData = frameData.threads[threadIndex];

                    // The markers are in depth first order and the depth is known
                    // So we can infer a parent child relationship
                    // Zero them first
                    foreach (ProfileMarker markerData in threadData.markers)
                    {
                        markerData.msChildren = 0.0f;
                    }

                    // Update the child times
                    markerStack.Clear();
                    foreach (ProfileMarker markerData in threadData.markers)
                    {
                        int depth = markerData.depth;

                        // Update depth stack and record child times in the parent
                        if (depth >= markerStack.Count)
                        {
                            // If at same level then remove the last item at this level
                            if (depth == markerStack.Count)
                            {
                                PopMarkerAndRecordTimeInParent(markerStack);
                            }

                            // Assume we can't move down depth without markers between levels.
                        }
                        else if (depth < markerStack.Count)
                        {
                            // We can move up depth several layers so need to pop off all those markers
                            while (markerStack.Count >= depth)
                            {
                                PopMarkerAndRecordTimeInParent(markerStack);
                            }
                        }

                        PushMarker(markerStack, markerData);
                    }
                }
            }
        }

        public static float GetLoadingProgress()
        {
            return s_Progress;
        }
    }

    [Serializable]
    internal class ProfileFrame
    {
        public List<ProfileThread> threads = new List<ProfileThread>();
        public double msStartTime;
        public float msFrame;

        public ProfileFrame()
        {
            msStartTime = 0.0;
            msFrame = 0f;
        }

        public bool IsSame(ProfileFrame otherFrame)
        {
            if (msStartTime != otherFrame.msStartTime)
                return false;
            if (msFrame != otherFrame.msFrame)
                return false;
            if (threads.Count != otherFrame.threads.Count)
                return false;

            // Close enough.
            return true;
        }

        public void Add(ProfileThread thread)
        {
            threads.Add(thread);
        }

        public void Write(BinaryWriter writer)
        {
            writer.Write(msStartTime);
            writer.Write(msFrame);
            writer.Write(threads.Count);
            foreach (var thread in threads)
            {
                thread.Write(writer);
            }
            ;
        }

        public ProfileFrame(BinaryReader reader, int fileVersion)
        {
            if (fileVersion > 1)
            {
                if (fileVersion >= 6)
                {
                    msStartTime = reader.ReadDouble();
                }
                else
                {
                    double sStartTime = reader.ReadDouble();
                    msStartTime = sStartTime * 1000.0;
                }
            }

            msFrame = reader.ReadSingle();
            int threadCount = reader.ReadInt32();
            threads.Clear();
            for (int thread = 0; thread < threadCount; thread++)
            {
                threads.Add(new ProfileThread(reader, fileVersion));
            }
        }
    }

    [Serializable]
    internal class ProfileThread
    {
        [NonSerialized]
        public List<ProfileMarker> markers = new List<ProfileMarker>();
        public int threadIndex;
        public long streamPos;
        public int markerCount = 0;
        public int fileVersion;

        public ProfileThread()
        {
        }

        public void Write(BinaryWriter writer)
        {
            writer.Write(threadIndex);
            writer.Write(markers.Count);
            foreach (var marker in markers)
            {
                marker.Write(writer);
            }
            ;
        }

        public ProfileThread(BinaryReader reader, int fileversion)
        {
            streamPos = reader.BaseStream.Position;
            fileVersion = fileversion;
            threadIndex = reader.ReadInt32();
            markerCount = reader.ReadInt32();
            markers.Clear();
            for (int marker = 0; marker < markerCount; marker++)
            {
                markers.Add(new ProfileMarker(reader, fileVersion));
            }
        }

        public bool ReadMarkers(string path)
        {
            if (streamPos == 0)
                return false; // the stream positions havent been written yet.
            var stream = File.OpenRead(path);
            BinaryReader br = new BinaryReader(stream);

            br.BaseStream.Position = streamPos;
            threadIndex = br.ReadInt32();
            markerCount = br.ReadInt32();

            markers.Clear();
            for (int marker = 0; marker < markerCount; marker++)
            {
                markers.Add(new ProfileMarker(br, fileVersion));
            }

            br.Close();
            return true;
        }

        public void AddMarker(ProfileMarker markerData)
        {
            markers.Add(markerData);
            markerCount++;
        }

        public void RebuildMarkers(string path)
        {
            if (!File.Exists(path)) return;
            FileStream stream = File.OpenRead(path);
            using (var reader = new BinaryReader(stream))
            {
                reader.BaseStream.Position = streamPos;
                threadIndex = reader.ReadInt32();
                markerCount = reader.ReadInt32();
                markers.Clear();
                for (int marker = 0; marker < markerCount; marker++)
                {
                    markers.Add(new ProfileMarker(reader, fileVersion));
                }
            }
        }
    }

    [Serializable]
    internal class ProfileMarker
    {
        public int nameIndex;
        public float msMarkerTotal;
        public int depth;
        [NonSerialized]
        public float msChildren;        // Recalculated on load so not saved in file

        public ProfileMarker()
        {
        }

        public static ProfileMarker Create(float durationMS, int depth)
        {
            var item = new ProfileMarker
            {
                msMarkerTotal = durationMS,
                depth = depth,
                msChildren = 0.0f
            };

            return item;
        }

        public static ProfileMarker Create(ProfilerFrameDataIterator frameData)
        {
            return Create(frameData.durationMS, frameData.depth);
        }

        public void Write(BinaryWriter writer)
        {
            writer.Write(nameIndex);
            writer.Write(msMarkerTotal);
            writer.Write(depth);
        }

        public ProfileMarker(BinaryReader reader, int fileVersion)
        {
            nameIndex = reader.ReadInt32();
            msMarkerTotal = reader.ReadSingle();
            depth = reader.ReadInt32();
            if (fileVersion == 3)   // In this version we saved the msChildren value but we don't need to as we now recalculate on load
                msChildren = reader.ReadSingle();
            else
                msChildren = 0.0f;
        }
    }
}
