.. _rust_arch_support:

Arch Support
============

Currently, the Rust compiler (``rustc``) uses LLVM for code generation,
which limits the supported architectures we can target. In addition, support
for building the kernel with LLVM/Clang varies (see :ref:`kbuild_llvm`),
which ``bindgen`` relies on through ``libclang``.

Below is a general summary of architectures that currently work. Level of
support corresponds to ``S`` values in the ``MAINTAINERS`` file.

.. list-table::
   :widths: 10 10 10
   :header-rows: 1

   * - Architecture
     - Level of support
     - Constraints
   * - ``arm``
     - Maintained
     - ``armv6`` and compatible only, ``RUST_OPT_LEVEL >= 2``
   * - ``arm64``
     - Maintained
     - None
   * - ``powerpc``
     - Maintained
     - ``ppc64le`` only, ``RUST_OPT_LEVEL < 2`` requires ``CONFIG_THREAD_SHIFT=15``
   * - ``riscv``
     - Maintained
     - ``riscv64`` only
   * - ``x86``
     - Maintained
     - ``x86_64`` only
