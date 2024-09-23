namespace UnityEditor.TestTools.CodeCoverage.CommandLineParser
{
    interface ICommandLineOption
    {
        string ArgName { get; }
        void ApplyValue(string value);
    }
}
