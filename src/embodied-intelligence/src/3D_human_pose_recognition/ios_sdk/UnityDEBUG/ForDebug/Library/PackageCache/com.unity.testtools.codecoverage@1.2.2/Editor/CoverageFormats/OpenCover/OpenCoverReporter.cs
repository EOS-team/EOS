using System;
using System.Diagnostics.CodeAnalysis;
using System.Collections.Generic;
using System.Reflection;
using System.Text;
using System.Linq;
using UnityEngine;
using UnityEngine.TestTools;
using UnityEditor.TestTools.TestRunner.Api;
using OpenCover.Framework.Model;
using Module = OpenCover.Framework.Model.Module;
using ModelFile = OpenCover.Framework.Model.File;
using File = System.IO.File;
using UnityEditor.TestTools.CodeCoverage.Utils;

namespace UnityEditor.TestTools.CodeCoverage.OpenCover
{
    internal class OpenCoverReporter : ICoverageReporter
    {
        private readonly ICoverageReporterFilter m_ReporterFilter;
        private OpenCoverResultWriter m_Writer;

        private List<MethodBase> m_ExcludedMethods = null;
        private List<string> m_ExcludedTypes = null;

        private ITestAdaptor m_CurrentTestData;

        private static readonly Dictionary<string, string> m_Operators = new Dictionary<string, string>
        {
            { "op_Addition", "operator+" },
            { "op_UnaryPlus", "operator+" },
            { "op_Increment", "operator++" },
            { "op_Subtraction", "operator-" },
            { "op_UnaryNegation", "operator-" },
            { "op_Decrement", "operator--" },
            { "op_Multiply", "operator*" },
            { "op_Division", "operator/" },
            { "op_Modulus", "operator%" },
            { "op_ExclusiveOr", "operator^" },
            { "op_BitwiseAnd", "operator&" },
            { "op_BitwiseOr", "operator|" },
            { "op_LeftShift", "operator<<" },
            { "op_RightShift", "operator>>" },
            { "op_Equality", "operator==" },
            { "op_Inequality", "operator!=" },
            { "op_GreaterThan", "operator>" },
            { "op_LessThan", "operator<" },
            { "op_GreaterThanOrEqual", "operator>=" },
            { "op_LessThanOrEqual", "operator<=" },
            { "op_OnesComplement", "operator~" },
            { "op_LogicalNot", "operator!" },
            { "op_True", "operator true" },
            { "op_False", "operator false" }
        };

        public OpenCoverReporter() : this(new OpenCoverReporterFilter())
        {
        }

        internal OpenCoverReporter(ICoverageReporterFilter reporterFilter)
        {
            m_ReporterFilter = reporterFilter;
        }

        public ICoverageReporterFilter GetReporterFilter()
        {
            return m_ReporterFilter;
        }

        public void OnBeforeAssemblyReload()
        {
            if (m_CurrentTestData != null && m_ReporterFilter.ShouldGenerateTestReferences())
            {
                OutputVisitedCoverageReport(CoverageRunData.instance.isRecording, true, m_CurrentTestData);
            }
            else
            {
                OutputVisitedCoverageReport(true, !CoverageRunData.instance.isRecording);
            }   
        }

        public void OnCoverageRecordingPaused()
        {
            OutputVisitedCoverageReport();
        }

        public void OnInitialise(CoverageSettings settings)
        {
            if (!m_ReporterFilter.ShouldGenerateTestReferences() && settings.resetCoverageData)
            {
                Coverage.ResetAll();
            }

            m_ReporterFilter.SetupFiltering();

            if (m_Writer == null)
            {
                m_Writer = new OpenCoverResultWriter(settings);
            }
            m_Writer.SetupCoveragePaths();
        }

        public void OnRunStarted(ITestAdaptor testsToRun)
        {
            Events.InvokeOnCoverageSessionStarted();

            if (m_Writer != null)
            {
                if (!CommandLineManager.instance.dontClear)
                    m_Writer.ClearCoverageFolderIfExists();
                m_Writer.SetupCoveragePaths();

                OutputFullEmptyReport();
            }
        }

        public void OnRunFinished(ITestResultAdaptor testResults)
        {
            if (CoverageRunData.instance.isRecording || !m_ReporterFilter.ShouldGenerateTestReferences())
            {
                OutputVisitedCoverageReport(false);
            }

            Events.InvokeOnCoverageSessionFinished();
        }

        public void OnTestStarted(ITestAdaptor test)
        {
            if (m_ReporterFilter.ShouldGenerateTestReferences())
            {
                m_CurrentTestData = test;
                Coverage.ResetAll();
            }
        }

        public void OnTestFinished(ITestResultAdaptor result)
        {
            if (m_ReporterFilter.ShouldGenerateTestReferences())
            {
                OutputVisitedCoverageReport(CoverageRunData.instance.isRecording, true, result.Test);
            }
        }

        private void OutputFullEmptyReport()
        {
            if (m_Writer != null)
            {
                // Exit if generateRootEmptyReport arg is passed and a root fullEmpty report has already been generated 
                if (CommandLineManager.instance.generateRootEmptyReport)
                {
                    string rootFullEmptyPath = m_Writer.GetRootFullEmptyPath();
                    if (File.Exists(rootFullEmptyPath))
                        return;
                }
                
                if (!CommandLineManager.instance.batchmode)
                    EditorUtility.DisplayProgressBar(OpenCoverReporterStyles.ProgressTitle.text, OpenCoverReporterStyles.ProgressWritingFile.text, 0.85f);

                CoverageSession coverageSession = GenerateOpenCoverSession(CoverageReportType.FullEmpty);
                if (coverageSession != null)
                {
                    m_Writer.CoverageSession = coverageSession;
                    m_Writer.WriteCoverageSession(CoverageReportType.FullEmpty);
                }
                else
                {
                    ResultsLogger.Log(ResultID.Warning_NoCoverageResultsSaved, CoverageUtils.GetFilteringLogParams(m_ReporterFilter.GetAssemblyFiltering(), m_ReporterFilter.GetPathFiltering()) );
                }

                if (!CommandLineManager.instance.batchmode)
                    EditorUtility.ClearProgressBar();
            }
        }

        public void OutputVisitedCoverageReport(bool clearProgressBar = true, bool logNoCoverageResultsSavedWarning = true, ITestAdaptor testData = null)
        {
            if (!CommandLineManager.instance.batchmode)
                EditorUtility.DisplayProgressBar(OpenCoverReporterStyles.ProgressTitle.text, OpenCoverReporterStyles.ProgressWritingFile.text, 0.85f);

            MethodInfo testMethodInfo = testData != null ? testData.Method.MethodInfo : null;

            CoverageSession coverageSession = GenerateOpenCoverSession(CoverageReportType.CoveredMethodsOnly, testMethodInfo);
            if (coverageSession != null && m_Writer != null)
            {
                m_Writer.CoverageSession = coverageSession;
                m_Writer.WriteCoverageSession(CoverageReportType.CoveredMethodsOnly);
            }
            else
            {
                if (logNoCoverageResultsSavedWarning)
                {
                    if (CoverageRunData.instance.isRecordingPaused)
                    {
                        ResultsLogger.Log(ResultID.Warning_NoVisitedCoverageResultsSavedRecordingPaused, CoverageUtils.GetFilteringLogParams(m_ReporterFilter.GetAssemblyFiltering(), m_ReporterFilter.GetPathFiltering()));
                    }
                    else
                    {
                        ResultsLogger.Log(ResultID.Warning_NoVisitedCoverageResultsSaved, CoverageUtils.GetFilteringLogParams(m_ReporterFilter.GetAssemblyFiltering(), m_ReporterFilter.GetPathFiltering()));
                    }
                }
            }

            if (clearProgressBar)
                EditorUtility.ClearProgressBar();
        }

        private bool IsSpecialMethod(MethodBase methodBase)
        {
            return methodBase.IsSpecialName && ((methodBase.Attributes & MethodAttributes.HideBySig) != 0);
        }

        private bool IsConstructor(MethodBase methodBase)
        {
            return IsSpecialMethod(methodBase) && methodBase.MemberType == MemberTypes.Constructor && methodBase.Name == ".ctor";
        }

        private bool IsStaticConstructor(MethodBase methodBase)
        {
            return IsSpecialMethod(methodBase) && methodBase.IsStatic && methodBase.MemberType == MemberTypes.Constructor && methodBase.Name == ".cctor";
        }

        private bool IsPropertySetter(MethodBase methodBase)
        {
            return IsSpecialMethod(methodBase) && methodBase.Name.StartsWith("set_");
        }

        private bool IsPropertyGetter(MethodBase methodBase)
        {
            return IsSpecialMethod(methodBase) && methodBase.Name.StartsWith("get_");
        }

        private bool IsOperator(MethodBase methodBase)
        {
            return IsSpecialMethod(methodBase) && methodBase.Name.StartsWith("op_");
        }

        private bool IsAnonymousOrInnerMethod(MethodBase methodBase)
        {
            char[] invalidChars = { '<', '>' };
            return methodBase.Name.IndexOfAny(invalidChars) != -1;
        }

        private string GenerateOperatorName(MethodBase methodBase)
        {
            string operatorName = string.Empty;

            if (!m_Operators.TryGetValue(methodBase.Name, out operatorName))
            {
                switch (methodBase.Name)
                {
                    case "op_Implicit":
                        operatorName = $"implicit operator {GetReturnTypeName(methodBase)}";
                        break;

                    case "op_Explicit":
                        operatorName = $"explicit operator {GetReturnTypeName(methodBase)}";
                        break;

                    default:
                        operatorName = $"unknown operator {methodBase.Name}";
                        break;
                }
            }

            return operatorName;
        }

        private string GetReturnTypeName(MethodBase methodBase)
        {
            string returnTypeName = string.Empty;
            MethodInfo methodInfo = methodBase as MethodInfo;
            if (methodInfo != null)
            {
                returnTypeName = GenerateTypeName(methodInfo.ReturnType);
            }

            return returnTypeName;
        }

        internal string GenerateMethodName(MethodBase methodBase)
        {
            StringBuilder sb = new StringBuilder();

            if (methodBase.IsStatic)
            {
                sb.Append("static ");
            }

            string returnTypeName = GetReturnTypeName(methodBase);
            if (returnTypeName != string.Empty)
            {
                sb.Append(returnTypeName);
                sb.Append(' ');
            }

            StringBuilder methodStringBuilder = new StringBuilder();

            methodStringBuilder.Append(GenerateTypeName(methodBase.DeclaringType));
            methodStringBuilder.Append(".");
            bool lastDotSubstituted = false;

            if (IsConstructor(methodBase) || IsStaticConstructor(methodBase))
            {
                methodStringBuilder.Append(GenerateConstructorName(methodBase.DeclaringType));
            }
            else if (IsOperator(methodBase))
            {
                lastDotSubstituted = SubstituteLastDotWithDoubleColon(ref methodStringBuilder);
                methodStringBuilder.Append(GenerateOperatorName(methodBase));
            }
            else if (IsAnonymousOrInnerMethod(methodBase))
            {
                lastDotSubstituted = SubstituteLastDotWithDoubleColon(ref methodStringBuilder);
                methodStringBuilder.Append(methodBase.Name);
            }
            else
            {
                methodStringBuilder.Append(methodBase.Name);
            }

            if (!lastDotSubstituted)
            {
                SubstituteLastDotWithDoubleColon(ref methodStringBuilder);
            }

            sb.Append(methodStringBuilder.ToString());

            if (methodBase.IsGenericMethodDefinition)
            {
                Type[] types = methodBase.GetGenericArguments();
                sb.Append(GenerateGenericTypeList(types));
            }
            sb.Append('(');
            ParameterInfo[] parameterInfos = methodBase.GetParameters();
            for (int i=0; i<parameterInfos.Length; ++i)
            {
                sb.Append(GenerateTypeName(parameterInfos[i].ParameterType));

                if (i != parameterInfos.Length - 1)
                    sb.Append(", ");
            }
            sb.Append(')');

            return sb.ToString();
        }

        private bool SubstituteLastDotWithDoubleColon(ref StringBuilder sb)
        {
            bool substituted = false;

            if ( sb.Length > 0)
            {
                int lastDotPos = -1;
                for (int i = sb.Length - 1; i >= 0; i--)
                {
                    if (sb[i] == '.')
                    {
                        lastDotPos = i;
                        break;
                    }
                }

                if (lastDotPos != -1 )
                {
                    sb.Remove(lastDotPos, 1);
                    sb.Insert(lastDotPos, "::");
                    substituted = true;
                }
            }

            return substituted;
        }

        private string GenerateTypeName(Type type)
        {
            StringBuilder sb = new StringBuilder();
            if (type != null)
            {
                if (type.IsGenericTypeDefinition || type.IsGenericType)
                {
                    sb.Append(GenerateGenericTypeName(type));
                }
                else if (type.IsGenericParameter)
                {
                    sb.Append(type.Name);
                }
                else
                {
                    sb.Append(type.FullName);
                }

                // Replace + with / so as nested classes appear in the same file
                sb.Replace('+', '/');
            }

            return sb.ToString();
        }

        private string GenerateGenericTypeName(Type type)
        {
            StringBuilder sb = new StringBuilder();
            if (type != null)
            {
                if (type.IsGenericTypeDefinition || type.IsGenericType)
                {
                    // When IsGenericType the FullName includes unnecessary information and thus cannot be used.
                    // Therefore we use the Name instead and add the Namespace at the beginning
                    if (!type.IsGenericTypeDefinition && type.IsGenericType && type.Namespace != string.Empty)
                    {
                        sb.Append(type.Namespace);
                        sb.Append('.');
                    }

                    string[] splitTypes = type.IsGenericTypeDefinition ? type.FullName.Split('+') : type.Name.Split('+');
                    Type[] genericTypeArguments = type.GetGenericArguments();
                    int genericTypeArgumentIndex = 0;

                    int numOfTypes = splitTypes.Length;
                    for (int i = 0; i < numOfTypes; ++i)
                    {
                        string splitType = splitTypes[i];

                        int genericSeparatorIndex = splitType.LastIndexOf('`');
                        if (genericSeparatorIndex != -1)
                        {
                            sb.Append(splitType.Substring(0, genericSeparatorIndex));
                            string argumentIndexStr = splitType.Substring(genericSeparatorIndex+1);

                            int numOfArguments;
                            if (Int32.TryParse(argumentIndexStr, out numOfArguments))
                            {
                                sb.Append("[");
                                for (int j = 0; j < numOfArguments; ++j)
                                {
                                    if (genericTypeArgumentIndex < genericTypeArguments.Length)
                                    {
                                        sb.Append($"{genericTypeArguments[genericTypeArgumentIndex++].Name}");

                                        if (j < numOfArguments - 1)
                                            sb.Append(",");
                                    }
                                }
                                sb.Append("]");
                            }

                            if (i < numOfTypes - 1)
                                sb.Append("/");
                        }
                        else
                        {
                            sb.Append(splitType);
                        }
                    }
                }
            }

            return sb.ToString();
        }

        private string GenerateConstructorName(Type type)
        {
            StringBuilder sb = new StringBuilder();

            string typeName = type.Name;

            int genericSeparatorIndex = typeName.LastIndexOf('`');
            if (genericSeparatorIndex != -1)
            {
                sb.Append(typeName.Substring(0, genericSeparatorIndex));
            }
            else
            {
                sb.Append(typeName);
            }

            return sb.ToString();
        }

        private string GenerateGenericTypeList(Type[] genericTypes)
        {
            StringBuilder sb = new StringBuilder();

            sb.Append('[');
            for (int i = 0; i < genericTypes.Length; ++i)
            {
                sb.Append(genericTypes[i].Name);

                if (i != genericTypes.Length - 1)
                    sb.Append(", ");
            }
            sb.Append(']');

            return sb.ToString();
        }

        internal CoverageSession GenerateOpenCoverSession(CoverageReportType reportType, MethodInfo testMethodInfo = null)
        {
            ResultsLogger.LogSessionItem("Started OpenCover Session", LogVerbosityLevel.Info);
            CoverageSession coverageSession = null;

            UInt32 fileUID = 0;
            UInt32 trackedMethodUID = 0;
            List<Module> moduleList = new List<Module>();

            Assembly[] assemblies = AppDomain.CurrentDomain.GetAssemblies();

            float progressInterval = 0.9f / assemblies.Length;
            float currentProgress = 0.0f;

            bool shouldGenerateAdditionalMetrics = m_ReporterFilter.ShouldGenerateAdditionalMetrics();

            foreach (Assembly assembly in assemblies)
            {
                string assemblyName = assembly.GetName().Name.ToLower();

                try
                {
                    if (assembly.GetCustomAttribute<ExcludeFromCoverageAttribute>() != null ||
                        assembly.GetCustomAttribute<ExcludeFromCodeCoverageAttribute>() != null)
                    {
                        ResultsLogger.LogSessionItem($"Excluded assembly (ExcludeFromCoverage): {assemblyName}", LogVerbosityLevel.Info);
                        continue;
                    }
                }
                catch
                {
                    ResultsLogger.Log(ResultID.Warning_ExcludeAttributeAssembly, assemblyName);
                }

                currentProgress += progressInterval;

                if (!CommandLineManager.instance.batchmode)
                    EditorUtility.DisplayProgressBar(OpenCoverReporterStyles.ProgressTitle.text, OpenCoverReporterStyles.ProgressGatheringResults.text, currentProgress); 
        
                if (!m_ReporterFilter.ShouldProcessAssembly(assemblyName))
                {
                    ResultsLogger.LogSessionItem($"Excluded assembly (Assembly Filtering): {assemblyName}", LogVerbosityLevel.Verbose);
                    continue;
                }

                List<Class> coveredClasses = new List<Class>();
                List<string> filesNotFound = new List<string>();
                Dictionary<string, UInt32> fileList = new Dictionary<string, UInt32>();
                Type[] assemblyTypes = null;
                m_ExcludedMethods = null;
                m_ExcludedTypes = null;

                try
                {
                    assemblyTypes = assembly.GetTypes();
                }
                catch (ReflectionTypeLoadException ex)
                {
                    // This exception can be thrown if some of the types from this assembly can't be loaded. If this
                    // happens, the Types property array contains a Type for all loaded types and null for each
                    // type that couldn't be loaded.
                    assemblyTypes = ex.Types;
                    m_ReporterFilter.ShouldProcessAssembly(assemblyName);
                }

                if (assemblyTypes == null)
                {
                    ResultsLogger.Log(ResultID.Assert_NullAssemblyTypes);
                    continue;
                }

                ResultsLogger.LogSessionItem($"Processing assembly: {assemblyName}", LogVerbosityLevel.Verbose);

                foreach (Type type in assemblyTypes)
                {
                    // The type can be null if the ReflectionTypeLoadException has been thrown previously.
                    if (type == null)
                    {
                        continue;
                    }

                    string className = type.FullName;
                    
                    try
                    {
                        if (type.GetCustomAttribute<ExcludeFromCoverageAttribute>() != null ||
                            type.GetCustomAttribute<ExcludeFromCodeCoverageAttribute>() != null ||
                            CheckIfParentMemberIsExcluded(type))
                        {
                            ResultsLogger.LogSessionItem($"Excluded class (ExcludeFromCoverage): {className}, assembly: {assemblyName}", LogVerbosityLevel.Verbose);
                            if (m_ExcludedTypes == null)
                                m_ExcludedTypes = new List<string>();
                            m_ExcludedTypes.Add(type.FullName);
                            continue;
                        }
                    }
                    catch
                    {
                        ResultsLogger.Log(ResultID.Warning_ExcludeAttributeClass, className, assemblyName);
                    }
                    
                    ResultsLogger.LogSessionItem($"Processing class: {className}, assembly: {assemblyName}", LogVerbosityLevel.Verbose);

                    CoveredMethodStats[] classMethodStatsArray = Coverage.GetStatsFor(type);
                    if (classMethodStatsArray.Length > 0)
                    {
                        List<Method> coveredMethods = new List<Method>();

                        foreach (CoveredMethodStats classMethodStats in classMethodStatsArray)
                        {
                            MethodBase method = classMethodStats.method;

                            if (method == null)
                                continue;

                            string methodName = method.Name;

                            try
                            {
                                if (method.GetCustomAttribute<ExcludeFromCoverageAttribute>() != null ||
                                    method.GetCustomAttribute<ExcludeFromCodeCoverageAttribute>() != null ||
                                    CheckIfParentMemberIsExcluded(method))
                                {
                                    ResultsLogger.LogSessionItem($"Excluded method (ExcludeFromCoverage): {methodName}, class: {className}, assembly: {assemblyName}", LogVerbosityLevel.Info);
                                    if (m_ExcludedMethods == null)
                                        m_ExcludedMethods = new List<MethodBase>();
                                    m_ExcludedMethods.Add(method);
                                    continue;
                                }
                            }
                            catch
                            {
                                ResultsLogger.Log(ResultID.Warning_ExcludeAttributeMethod, methodName, className, assemblyName);
                            }

                            if (IsGetterSetterPropertyExcluded(method, type))
                            {
                                ResultsLogger.LogSessionItem($"Excluded method (ExcludeFromCoverage): {methodName}, class: {className}, assembly: {assemblyName}", LogVerbosityLevel.Info);
                                continue;
                            }

                            if (classMethodStats.totalSequencePoints > 0)
                            {
                                List<SequencePoint> coveredSequencePoints = new List<SequencePoint>();

                                uint fileId = 0;
                                int totalVisitCount = 0;
                                CoveredSequencePoint[] classMethodSequencePointsArray = Coverage.GetSequencePointsFor(method);
                                foreach (CoveredSequencePoint classMethodSequencePoint in classMethodSequencePointsArray)
                                {
                                    // Skip hidden sequence points
                                    if (classMethodSequencePoint.line == 0xfeefee)     // 16707566
                                    {
                                        ResultsLogger.LogSessionItem($"Ignored sequence point - Invalid line number: {classMethodSequencePoint.line} in method: {methodName} in class: {className}", LogVerbosityLevel.Info);
                                        continue;
                                    }

                                    string filename = classMethodSequencePoint.filename;
                                    if (filesNotFound.Contains(filename) || !m_ReporterFilter.ShouldProcessFile(filename))
                                    {
                                        ResultsLogger.LogSessionItem($"Excluded method (Path Filtering): {methodName}, class: {className}, assembly: {assemblyName}", LogVerbosityLevel.Verbose);
                                        continue;
                                    }
                                    
                                    if (!fileList.TryGetValue(filename, out fileId))
                                    {
                                        if (!File.Exists(filename))
                                        {
                                            filesNotFound.Add(filename);
                                            continue;
                                        }
                                        else
                                        {
                                            fileId = ++fileUID;
                                            fileList.Add(filename, fileId);
                                        }
                                    }

                                    SequencePoint coveredSequencePoint = new SequencePoint();
                                    coveredSequencePoint.FileId = fileId;
                                    coveredSequencePoint.StartLine = (int)classMethodSequencePoint.line;
                                    coveredSequencePoint.StartColumn = (int)classMethodSequencePoint.column;
                                    coveredSequencePoint.EndLine = (int)classMethodSequencePoint.line;
                                    coveredSequencePoint.EndColumn = (int)classMethodSequencePoint.column;
                                    coveredSequencePoint.VisitCount = reportType == CoverageReportType.FullEmpty ? 0 : (int)classMethodSequencePoint.hitCount;
                                    totalVisitCount += coveredSequencePoint.VisitCount;
                                    coveredSequencePoint.Offset = (int)classMethodSequencePoint.ilOffset;

                                    if (testMethodInfo != null)
                                    {
                                        SetupTrackedMethod(coveredSequencePoint, trackedMethodUID);
                                    }

                                    coveredSequencePoints.Add(coveredSequencePoint);
                                }

                                bool includeMethod = true;

                                if (reportType == CoverageReportType.CoveredMethodsOnly && totalVisitCount == 0)
                                {
                                    includeMethod = false;
                                }
                                else if (coveredSequencePoints.Count == 0)
                                {
                                    includeMethod = false;
                                }

                                if (includeMethod)
                                {
                                    Method coveredMethod = new Method();
                                    coveredMethod.MetadataToken = method.MetadataToken;
                                    coveredMethod.FullName = GenerateMethodName(method);
                                    coveredMethod.FileRef = new FileRef() { UniqueId = fileId };
                                    coveredMethod.IsConstructor = IsConstructor(method) || IsStaticConstructor(method);
                                    coveredMethod.IsStatic = method.IsStatic;
                                    coveredMethod.IsSetter = IsPropertySetter(method);
                                    coveredMethod.IsGetter = IsPropertyGetter(method);
                                    coveredMethod.SequencePoints = coveredSequencePoints.ToArray();
                                    if (shouldGenerateAdditionalMetrics)
                                    {
                                        coveredMethod.CyclomaticComplexity = method.CalculateCyclomaticComplexity();
                                    }

                                    ResultsLogger.LogSessionItem($"Processing method: {coveredMethod.FullName}, class: {className}, assembly: {assemblyName}", LogVerbosityLevel.Verbose);

                                    coveredMethods.Add(coveredMethod);
                                }
                            }
                        }

                        if (coveredMethods.Count > 0)
                        {
                            Class coveredClass = new Class();
                            coveredClass.FullName = GenerateTypeName(type);
                            coveredClass.Methods = coveredMethods.ToArray();
                            coveredClasses.Add(coveredClass);
                        }
                    }
                }

                if (coveredClasses.Count != 0)
                {
                    Module module = new Module();
                    module.ModuleName = assembly.GetName().Name;

                    if (testMethodInfo != null)
                    {
                        SetupTrackedMethod(module, testMethodInfo, trackedMethodUID);
                        trackedMethodUID++;
                    }

                    List<ModelFile> coveredFileList = new List<ModelFile>();
                    foreach (KeyValuePair<string, UInt32> fileEntry in fileList)
                    {
                        ModelFile coveredFile = new ModelFile();
                        coveredFile.FullPath = CoverageUtils.NormaliseFolderSeparators(fileEntry.Key);
                        if (CommandLineManager.instance.pathReplacingSpecified)
                            coveredFile.FullPath = CommandLineManager.instance.pathReplacing.ReplacePath(coveredFile.FullPath);
                        coveredFile.UniqueId = fileEntry.Value;

                        coveredFileList.Add(coveredFile);
                    }
                    module.Files = coveredFileList.ToArray();
                    module.Classes = coveredClasses.ToArray();
                    moduleList.Add(module);
                }
            }

            if (moduleList.Count > 0)
            {
                coverageSession = new CoverageSession();
                coverageSession.Modules = moduleList.ToArray();
                
                if (reportType != CoverageReportType.FullEmpty)
                    ProcessGenericMethods(coverageSession);

                foreach (Module coveredModule in moduleList)
                {
                    foreach (Class coveredClass in coveredModule.Classes)
                    {
                        foreach (Method coveredMethod in coveredClass.Methods)
                        {
                            UpdateMethodSummary(coveredMethod);
                        }
                        UpdateClassSummary(coveredClass);
                    }
                    UpdateModuleSummary(coveredModule);
                }

                UpdateSessionSummary(coverageSession);
            }

            ResultsLogger.LogSessionItem("Finished OpenCover Session", LogVerbosityLevel.Info);

            return coverageSession;
        }

        internal bool IsGetterSetterPropertyExcluded(MethodBase method, Type type)
        {
            if (IsPropertySetter(method) || IsPropertyGetter(method))
            {
                PropertyInfo[] properties = type.GetProperties(BindingFlags.Public | BindingFlags.NonPublic | BindingFlags.Instance | BindingFlags.Static | BindingFlags.DeclaredOnly);
                PropertyInfo property = properties.FirstOrDefault(pInfo => pInfo.GetMethod?.Name.Equals(method.Name) == true || pInfo.SetMethod?.Name.Equals(method.Name) == true);

                if (property?.GetCustomAttribute<ExcludeFromCoverageAttribute>() != null ||
                    property?.GetCustomAttribute<ExcludeFromCodeCoverageAttribute>() != null)
                {
                    return true;
                }
            }

            return false;
        }

        private void SetupTrackedMethod(SequencePoint sequencePoint, uint id)
        {
            TrackedMethodRef trackedMethodRef = new TrackedMethodRef();
            trackedMethodRef.UniqueId = id;
            trackedMethodRef.VisitCount = sequencePoint.VisitCount;
            TrackedMethodRef[] trackedMethodRefs = new TrackedMethodRef[1];
            trackedMethodRefs[0] = trackedMethodRef;
            sequencePoint.TrackedMethodRefs = trackedMethodRefs;
        }

        private void SetupTrackedMethod(Module module, MethodInfo testMethodInfo, uint id)
        {
            if (module.TrackedMethods == null)
            {
                TrackedMethod trackedMethod = new TrackedMethod();
                trackedMethod.UniqueId = id;
                trackedMethod.MetadataToken = testMethodInfo.MetadataToken;
                trackedMethod.FullName = GenerateMethodName(testMethodInfo);
                trackedMethod.Strategy = "Test";
                module.TrackedMethods = new TrackedMethod[1];
                module.TrackedMethods[0] = trackedMethod;
            }
        }

        private void UpdateMethodSummary(Method coveredMethod)
        {
            int totalSequencePoints = coveredMethod.SequencePoints.Length;
            int numCoveredSequencePoints = coveredMethod.SequencePoints.Count(sp => sp.VisitCount > 0);

            coveredMethod.Visited = numCoveredSequencePoints != 0;
            coveredMethod.Summary.NumClasses = 0;
            coveredMethod.Summary.VisitedClasses = 0;
            coveredMethod.Summary.NumMethods = 1;
            coveredMethod.Summary.VisitedMethods = (coveredMethod.Visited) ? 1 : 0;
            coveredMethod.Summary.NumSequencePoints = totalSequencePoints;
            coveredMethod.Summary.VisitedSequencePoints = numCoveredSequencePoints;
            CalculateSummarySequenceCoverage(coveredMethod.Summary);
            coveredMethod.SequenceCoverage = coveredMethod.Summary.SequenceCoverage;
            if (m_ReporterFilter.ShouldGenerateAdditionalMetrics())
            {
                coveredMethod.Summary.MaxCyclomaticComplexity = coveredMethod.CyclomaticComplexity;
                coveredMethod.Summary.MinCyclomaticComplexity = coveredMethod.CyclomaticComplexity;
                coveredMethod.CrapScore = CalculateCrapScore(coveredMethod.CyclomaticComplexity, coveredMethod.SequenceCoverage);
                coveredMethod.Summary.MaxCrapScore = coveredMethod.CrapScore;
                coveredMethod.Summary.MinCrapScore = coveredMethod.CrapScore;
            }
        }

        internal decimal CalculateCrapScore(int cyclomaticComplexity, decimal sequenceCoverage)
        {
            decimal crapScore = Math.Round((decimal)Math.Pow(cyclomaticComplexity, 2) *
              (decimal)Math.Pow(1.0 - (double)(sequenceCoverage / (decimal)100.0), 3.0) +
              cyclomaticComplexity,
              2);

            return crapScore;
        }

        private void UpdateClassSummary(Class coveredClass)
        {
            coveredClass.Summary.NumClasses = 1;
            UpdateSummary(coveredClass.Summary, coveredClass.Methods);
            coveredClass.Summary.VisitedClasses = (coveredClass.Summary.VisitedMethods > 0) ? 1 : 0;
        }

        private void UpdateModuleSummary(Module coveredModule)
        {
            UpdateSummary(coveredModule.Summary, coveredModule.Classes);
        }

        private void UpdateSessionSummary(CoverageSession coverageSession)
        {
            UpdateSummary(coverageSession.Summary, coverageSession.Modules);
        }

        private void UpdateSummary(Summary summary, SummarySkippedEntity[] entities)
        {
            if (entities.Length > 0)
            {
                foreach (Summary entitySummary in entities.Select(entity => entity.Summary))
                {
                    summary.NumSequencePoints += entitySummary.NumSequencePoints;
                    summary.VisitedSequencePoints += entitySummary.VisitedSequencePoints;
                    summary.NumBranchPoints += entitySummary.NumBranchPoints;
                    summary.VisitedBranchPoints += entitySummary.VisitedBranchPoints;
                    summary.NumMethods += entitySummary.NumMethods;
                    summary.VisitedMethods += entitySummary.VisitedMethods;
                    summary.NumClasses += entitySummary.NumClasses;
                    summary.VisitedClasses += entitySummary.VisitedClasses;
                }

                summary.MaxCyclomaticComplexity = entities.Max(entity => entity.Summary.MaxCyclomaticComplexity);
                summary.MinCyclomaticComplexity = entities.Min(entity => entity.Summary.MinCyclomaticComplexity);
                summary.MaxCrapScore = entities.Max(entity => entity.Summary.MaxCrapScore);
                summary.MinCrapScore = entities.Min(entity => entity.Summary.MinCrapScore);

                CalculateSummarySequenceCoverage(summary);
                CalculateSummaryBranchCoverage(summary);
            }
        }

        private void CalculateSummarySequenceCoverage(Summary summary)
        {
            if (summary.NumSequencePoints > 0)
            {
                summary.SequenceCoverage = decimal.Round(100.0m * (summary.VisitedSequencePoints / (decimal)summary.NumSequencePoints), 2);
            }
            else
            {
                summary.SequenceCoverage = 0.0m;
            }
        }

        private void CalculateSummaryBranchCoverage(Summary summary)
        {
            if (summary.NumBranchPoints > 0)
            {
                summary.BranchCoverage = decimal.Round(100.0m * (summary.VisitedBranchPoints / (decimal)summary.NumBranchPoints), 2);
            }
            else
            {
                summary.BranchCoverage = 0.0m;
            }
        }

        private void ProcessGenericMethods(CoverageSession coverageSession)
        {
            CoveredMethodStats[] coveredMethodStats = Coverage.GetStatsForAllCoveredMethods();
            foreach (MethodBase method in coveredMethodStats.Select(coveredMethodStat => coveredMethodStat.method))
            {
                Type declaringType = method.DeclaringType;

                if (!(declaringType.IsGenericType || method.IsGenericMethod))
                {
                    continue;
                }

                string assemblyName = declaringType.Assembly.GetName().Name.ToLower();
                if (!m_ReporterFilter.ShouldProcessAssembly(assemblyName))
                {
                    ResultsLogger.LogSessionItem($"Excluded generic method (Assembly Filtering): {method.Name}, assembly: {assemblyName}", LogVerbosityLevel.Verbose);
                    continue;
                }

                Module module = Array.Find(coverageSession.Modules, element => element.ModuleName.ToLower() == assemblyName);
                if (module != null)
                {
                    string className = string.Empty;
                    if (declaringType.IsGenericType)
                    {
                        Type genericTypeDefinition = declaringType.GetGenericTypeDefinition();
                        className = GenerateTypeName(genericTypeDefinition);
                    }
                    else if (method.IsGenericMethod)
                    {
                        className = GenerateTypeName(declaringType);
                    }

                    Class klass = Array.Find(module.Classes, element => element.FullName == className);
                    if (klass != null)
                    {
                        Method targetMethod = Array.Find(klass.Methods, element => element.MetadataToken == method.MetadataToken);
                        if (targetMethod != null)
                        {
                            ResultsLogger.LogSessionItem($"Processing generic method: {method.Name}, assembly: {assemblyName}", LogVerbosityLevel.Verbose);

                            CoveredSequencePoint[] coveredSequencePoints = Coverage.GetSequencePointsFor(method);
                            foreach (CoveredSequencePoint coveredSequencePoint in coveredSequencePoints)
                            {
                                SequencePoint targetSequencePoint = Array.Find(targetMethod.SequencePoints, element => (element.StartLine == coveredSequencePoint.line && element.Offset == coveredSequencePoint.ilOffset));
                                if (targetSequencePoint != null)
                                {
                                    targetSequencePoint.VisitCount += (int)coveredSequencePoint.hitCount;
                                    if (targetSequencePoint.TrackedMethodRefs != null && targetSequencePoint.TrackedMethodRefs.Length > 0)
                                    {
                                        targetSequencePoint.TrackedMethodRefs[0].VisitCount += (int)coveredSequencePoint.hitCount;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // Check if the parent member (type or method) is excluded
        private bool CheckIfParentMemberIsExcluded(MemberInfo member)
        {
            if (m_ExcludedMethods == null && m_ExcludedTypes == null)
            {
                return false;
            }

            Type declaringType = member.DeclaringType;

            while (declaringType != null)
            {
                // If parent type is excluded return true
                if (m_ExcludedTypes != null &&
                    m_ExcludedTypes.Any(typeName => typeName == declaringType.FullName))
                {
                    return true;
                }

                if (m_ExcludedMethods != null)
                {
                    // If parent method is excluded return true
                    foreach (var excludedMethod in m_ExcludedMethods)
                    {
                        if (declaringType.FullName == excludedMethod.DeclaringType.FullName &&
                            CheckIfParentMethodIsExcluded(member.Name, excludedMethod.Name))
                        {
                            return true;
                        }
                    }
                }
                
                declaringType = declaringType.DeclaringType;
            }

            return false;
        }

        private bool CheckIfParentMethodIsExcluded(string methodName, string excludedMethodName)
        {
            return
                // Lambda method
                methodName.IndexOf($"<{excludedMethodName}>b__") != -1 ||
                // yield method
                methodName.IndexOf($"<{excludedMethodName}>d__") != -1;
        }
    }
}
