.. _rust_docs:

Docs
====

Rust kernel code is not documented like C kernel code (i.e. via kernel-doc).
Instead, we use the usual system for documenting Rust code: the ``rustdoc``
tool, which uses Markdown (a *very* lightweight markup language).

This document describes how to make the most out of the kernel documentation
for Rust.


Reading the docs
----------------

An advantage of using Markdown is that it attempts to make text look almost as
you would have written it in plain text. This makes the documentation quite
pleasant to read even in its source form.

However, the generated HTML docs produced by ``rustdoc`` provide a *very* nice
experience, including integrated instant search, clickable items (types,
functions, constants, etc. -- including to all the standard Rust library ones
that we use in the kernel, e.g. ``core``), categorization, links to the source
code, etc.

Like for the rest of the kernel documentation, pregenerated HTML docs for
the libraries (crates) inside ``rust/`` that are used by the rest of the kernel
are available at `kernel.org`_ (TODO: link when in mainline and generated
alongside the rest of the documentation).

.. _kernel.org: http://kernel.org/

Otherwise, you can generate them locally. This is quite fast (same order as
compiling the code itself) and you do not need any special tools or environment.
This has the added advantage that they will be tailored to your particular
kernel configuration. To generate them, simply use the ``rustdoc`` target with
the same invocation you use for compilation, e.g.::

	make LLVM=1 rustdoc


Writing the docs
----------------

If you already know Markdown, learning how to write Rust documentation will be
a breeze. If not, understanding the basics is a matter of minutes reading other
code. There are also many guides available out there, a particularly nice one
is at `GitHub`_.

.. _GitHub: https://guides.github.com/features/mastering-markdown/#syntax

This is how a well-documented Rust function may look like (derived from the Rust
standard library)::

	/// Returns the contained [`Some`] value, consuming the `self` value,
	/// without checking that the value is not [`None`].
	///
	/// # Safety
	///
	/// Calling this method on [`None`] is *[undefined behavior]*.
	///
	/// [undefined behavior]: https://doc.rust-lang.org/reference/behavior-considered-undefined.html
	///
	/// # Examples
	///
	/// ```
	/// let x = Some("air");
	/// assert_eq!(unsafe { x.unwrap_unchecked() }, "air");
	/// ```
	pub unsafe fn unwrap_unchecked(self) -> T {
		match self {
			Some(val) => val,

			// SAFETY: the safety contract must be upheld by the caller.
			None => unsafe { hint::unreachable_unchecked() },
		}
	}

This example showcases a few ``rustdoc`` features and some common conventions
(that we also follow in the kernel):

* The first paragraph must be a single sentence briefly describing what
  the documented item does. Further explanations must go in extra paragraphs.

* ``unsafe`` functions must document the preconditions needed for a call to be
  safe under a ``Safety`` section.

* While not shown here, if a function may panic, the conditions under which
  that happens must be described under a ``Panics`` section. Please note that
  panicking should be very rare and used only with a good reason. In almost
  all cases, you should use a fallible approach, returning a `Result`.

* If providing examples of usage would help readers, they must be written in
  a section called ``Examples``.

* Rust items (functions, types, constants...) will be automatically linked
  (``rustdoc`` will find out the URL for you).

* Following the Rust standard library conventions, any ``unsafe`` block must be
  preceded by a ``SAFETY`` comment describing why the code inside is sound.

  While sometimes the reason might look trivial and therefore unneeded, writing
  these comments is not just a good way of documenting what has been taken into
  account, but also that there are no *extra* implicit constraints.

To learn more about how to write documentation for Rust and extra features,
please take a look at the ``rustdoc`` `book`_.

.. _book: https://doc.rust-lang.org/rustdoc/how-to-write-documentation.html
