using System;
using System.Linq;

namespace UnityEditor.TestTools.CodeCoverage.CommandLineParser
{
    internal class CommandLineOption : ICommandLineOption
    {
        readonly Action<string> m_ArgAction;

        public CommandLineOption(string argName, Action action)
        {
            ArgName = argName;
            m_ArgAction = s => action();
        }

        public CommandLineOption(string argName, Action<string> action)
        {
            ArgName = argName;
            m_ArgAction = action;
        }

        public CommandLineOption(string argName, Action<string[]> action)
        {
            ArgName = argName;
            m_ArgAction = s => action(SplitStringToArray(s));
        }

        public string ArgName { get; private set; }

        public void ApplyValue(string value)
        {
            m_ArgAction(value);
        }

        static string[] SplitStringToArray(string value)
        {
            if (string.IsNullOrEmpty(value))
            {
                return new string[] { };
            }

            return value.Split(';').ToArray();
        }
    }
}
