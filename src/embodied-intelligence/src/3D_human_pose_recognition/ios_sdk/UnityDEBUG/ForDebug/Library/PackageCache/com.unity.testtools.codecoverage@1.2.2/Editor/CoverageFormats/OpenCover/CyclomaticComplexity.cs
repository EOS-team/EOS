using System;
using System.Collections.Generic;
using System.Reflection;
using System.Reflection.Emit;
using Mono.Reflection;

namespace UnityEditor.TestTools.CodeCoverage.OpenCover
{
    internal static class CyclomaticComplexity
    {
        private static List<Instruction> targets = new List<Instruction>();

        public static int CalculateCyclomaticComplexity(this MethodBase method)
        {
            if (method == null || method.GetMethodBody() == null)
            {
                return 1;
            }

            bool hasSwitch = false;
            foreach (Instruction ins in method.GetInstructions())
            {
                if (ins.OpCode.OperandType == OperandType.InlineSwitch)
                {
                    hasSwitch = true;
                    break;
                }
            }

            if (hasSwitch)
            {
                return GetSwitchCyclomaticComplexity(method);
            }
            return GetFastCyclomaticComplexity(method);
        }

        private static int GetFastCyclomaticComplexity(MethodBase method)
        {
            int cc = 1;
            foreach (Instruction ins in method.GetInstructions())
            {
                switch (ins.OpCode.FlowControl)
                {
                    case FlowControl.Branch:
                        // detect ternary pattern
                        Instruction previous = ins.Previous;
                        if (previous != null && previous.OpCode.Name.StartsWith("ld"))
                        {
                            ++cc;
                        }
                        break;

                    case FlowControl.Cond_Branch:
                        ++cc;
                        break;
                }
            }

            return cc;
        }

        private static int GetSwitchCyclomaticComplexity(MethodBase method)
        {
            Instruction previous = null;
            Instruction branch = null;
            int cc = 1;

            foreach (Instruction ins in method.GetInstructions())
            {
                switch (ins.OpCode.FlowControl)
                {
                    case FlowControl.Branch:
                        if (previous == null)
                        {
                            continue;
                        }

                        // detect ternary pattern
                        previous = ins.Previous;
                        if (previous.OpCode.Name.StartsWith("ld"))
                        {
                            cc++;
                        }

                        // or 'default' (xmcs)
                        if (previous.OpCode.FlowControl == FlowControl.Cond_Branch)
                        {
                            branch = (previous.Operand as Instruction);
                            // branch can be null (e.g. switch -> Instruction[])
                            if ((branch != null) && targets.Contains(branch) && !targets.Contains(ins))
                            {
                                targets.Add(ins);
                            }
                        }
                        break;

                    case FlowControl.Cond_Branch:
                        // note: a single switch (C#) with sparse values can be broken into several swicth (IL)
                        // that will use the same 'targets' and must be counted only once
                        if (ins.OpCode.OperandType == OperandType.InlineSwitch)
                        {
                            AccumulateSwitchTargets(ins);
                        }
                        else
                        {
                            // some conditional branch can be related to the sparse switch
                            branch = (ins.Operand as Instruction);
                            previous = branch.Previous;
                            if ((previous != null) && previous.Previous.OpCode.OperandType != OperandType.InlineSwitch)
                            {
                                if (!targets.Contains(branch))
                                {
                                    cc++;
                                }
                            }
                        }
                        break;
                }
            }
            // count all unique targets (and default if more than one C# switch is used)
            cc += targets.Count;
            targets.Clear();

            return cc;
        }

        private static void AccumulateSwitchTargets(Instruction ins)
        {
            Instruction[] cases = (Instruction[])ins.Operand;
            foreach (Instruction target in cases)
            {
                // ignore targets that are the next instructions (xmcs)
                if (target != ins.Next && !targets.Contains(target))
                    targets.Add(target);
            }
            // add 'default' branch (if one exists)
            Instruction next = ins.Next;
            if (next.OpCode.FlowControl == FlowControl.Branch)
            {
                Instruction unc = FindFirstUnconditionalBranchTarget(cases[0]);
                if (unc != next.Operand && !targets.Contains(next.Operand as Instruction))
                    targets.Add(next.Operand as Instruction);
            }
        }

        private static Instruction FindFirstUnconditionalBranchTarget(Instruction ins)
        {
            while (ins != null)
            {
                if (FlowControl.Branch == ins.OpCode.FlowControl)
                    return ((Instruction)ins.Operand);

                ins = ins.Next;
            }
            return null;
        }
    }
}
