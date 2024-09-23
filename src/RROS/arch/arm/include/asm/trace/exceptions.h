/* SPDX-License-Identifier: GPL-2.0 */
#undef TRACE_SYSTEM
#define TRACE_SYSTEM exceptions

#if !defined(_TRACE_EXCEPTIONS_H) || defined(TRACE_HEADER_MULTI_READ)
#define _TRACE_EXCEPTIONS_H

#include <linux/tracepoint.h>
#include <asm/ptrace.h>
#include <asm/dovetail.h>

#define __trace_trap(__sym)	{ __sym, #__sym }

#define trace_trap_symbolic(__trapnr)				\
	__print_symbolic(__trapnr,				\
			__trace_trap(ARM_TRAP_ACCESS),		\
			__trace_trap(ARM_TRAP_SECTION),		\
			__trace_trap(ARM_TRAP_DABT),		\
			__trace_trap(ARM_TRAP_PABT),		\
			__trace_trap(ARM_TRAP_BREAK),		\
			__trace_trap(ARM_TRAP_FPU),		\
			__trace_trap(ARM_TRAP_VFP),		\
			__trace_trap(ARM_TRAP_UNDEFINSTR),	\
			__trace_trap(ARM_TRAP_ALIGNMENT))

DECLARE_EVENT_CLASS(ARM_trap_event,
	TP_PROTO(int trapnr, struct pt_regs *regs),
	TP_ARGS(trapnr, regs),

	TP_STRUCT__entry(
		__field(int, trapnr)
		__field(struct pt_regs *, regs)
		),

	TP_fast_assign(
		__entry->trapnr = trapnr;
		__entry->regs = regs;
		),

	TP_printk("%s mode trap: %s",
		user_mode(__entry->regs) ? "user" : "kernel",
		trace_trap_symbolic(__entry->trapnr))
);

DEFINE_EVENT(ARM_trap_event, ARM_trap_entry,
	TP_PROTO(int trapnr, struct pt_regs *regs),
	TP_ARGS(trapnr, regs)
);

DEFINE_EVENT(ARM_trap_event, ARM_trap_exit,
	TP_PROTO(int trapnr, struct pt_regs *regs),
	TP_ARGS(trapnr, regs)
);

#undef TRACE_INCLUDE_PATH
#undef TRACE_INCLUDE_FILE
#define TRACE_INCLUDE_PATH asm/trace
#define TRACE_INCLUDE_FILE exceptions
#endif /*  _TRACE_EXCEPTIONS_H */

/* This part must be outside protection */
#include <trace/define_trace.h>
