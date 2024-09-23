using System;

namespace UnityEditor.Performance.ProfileAnalyzer
{
    /// <summary>
    /// Individual Thread details
    /// </summary>
    internal struct ThreadIdentifier
    {
        /// <summary>Thread name with index combined. A profiler analyzer specific unique thread representation</summary>
        public string threadNameWithIndex { get; private set; }
        /// <summary>Thread name (may not be unique)</summary>
        public string name { get; private set; }
        /// <summary>Thread index. A 1 based id for threads with matching names. -1 indicates all threads with the same name, 0 if there is only one thread with this name</summary>
        public int index { get; private set; }

        /// <summary>Thread index id which means all threads of the same name</summary>
        public static int kAll = -1;
        /// <summary>Thread index id use when there is only one thread with this name</summary>
        public static int kSingle = 0;

        /// <summary>Initialise ThreadIdentifier</summary>
        /// <param name="name">The thread name</param>
        /// <param name="index">The thread index</param>
        public ThreadIdentifier(string name, int index)
        {
            this.name = name;
            this.index = index;
            if (index == kAll)
                threadNameWithIndex = string.Format("All:{0}", name);
            else
                threadNameWithIndex = string.Format("{0}:{1}", index, name);
        }

        /// <summary>Initialise ThreadIdentifier from another ThreadIdentifier</summary>
        /// <param name="threadIdentifier">The other ThreadIdentifier</param>
        public ThreadIdentifier(ThreadIdentifier threadIdentifier)
        {
            name = threadIdentifier.name;
            index = threadIdentifier.index;
            threadNameWithIndex = threadIdentifier.threadNameWithIndex;
        }

        /// <summary>Initialise ThreadIdentifier from a unique name</summary>
        /// <param name="threadNameWithIndex">The unique name string (name with index)</param>
        public ThreadIdentifier(string threadNameWithIndex)
        {
            this.threadNameWithIndex = threadNameWithIndex;

            string[] tokens = threadNameWithIndex.Split(':');
            if (tokens.Length >= 2)
            {
                name = tokens[1];
                string indexString = tokens[0];
                if (indexString == "All")
                {
                    index = kAll;
                }
                else
                {
                    int intValue;
                    if (Int32.TryParse(tokens[0], out intValue))
                        index = intValue;
                    else
                        index = kSingle;
                }
            }
            else
            {
                index = kSingle;
                name = threadNameWithIndex;
            }
        }

        void UpdateThreadNameWithIndex()
        {
            if (index == kAll)
                threadNameWithIndex = string.Format("All:{0}", name);
            else
                threadNameWithIndex = string.Format("{0}:{1}", index, name);
        }

        /// <summary>Set the name of the thread</summary>
        /// <param name="newName">The name (without index)</param>
        public void SetName(string newName)
        {
            name = newName;
            UpdateThreadNameWithIndex();
        }

        /// <summary>Set the index of the thread name</summary>
        /// <param name="newIndex">The index of the thread with the same name</param>
        public void SetIndex(int newIndex)
        {
            index = newIndex;
            UpdateThreadNameWithIndex();
        }

        /// <summary>Sets the index to indicate we want all threads (used for filtering against other ThreadIdentifiers)</summary>
        public void SetAll()
        {
            SetIndex(kAll);
        }
    }
}
