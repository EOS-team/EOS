========================
Introduction to Dovetail
========================

:Author: Philippe Gerum
:Date: 08.04.2020

Using Linux as a host for lightweight software cores specialized in
delivering very short and bounded response times has been a popular
way of supporting real-time applications in the embedded space over
the years.

In this so-called *dual kernel* design, the time-critical work is
immediately delegated to a small companion core running out-of-band
with respect to the regular, in-band kernel activities. Applications
run in user space, obtaining real-time services from the
core. Alternatively, when there is no real-time requirement, threads
can still use the rich GPOS feature set Linux provides such as
networking, data storage or GUIs.

*Dovetail* introduces a high-priority execution stage into the main
kernel logic reserved for such a companion core to run on.  At any
time, out-of-band activities from this stage can preempt the common,
in-band work. A companion core can be implemented as as a driver,
which connects to the main kernel via the Dovetail interface for
delivering ultra-low latency scheduling capabilities to applications.

Dovetail is fully described at https://evlproject.org/dovetail/.
The reference implementation of a Dovetail-based companion core is
maintained at https://evlproject.org/core/.
