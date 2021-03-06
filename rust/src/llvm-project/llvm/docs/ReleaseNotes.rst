=========================
LLVM 11.0.0 Release Notes
=========================

.. contents::
    :local:

.. warning::
   These are in-progress notes for the upcoming LLVM 11 release.
   Release notes for previous releases can be found on
   `the Download Page <https://releases.llvm.org/download.html>`_.


Introduction
============

This document contains the release notes for the LLVM Compiler Infrastructure,
release 11.0.0.  Here we describe the status of LLVM, including major improvements
from the previous release, improvements in various subprojects of LLVM, and
some of the current users of the code.  All LLVM releases may be downloaded
from the `LLVM releases web site <https://llvm.org/releases/>`_.

For more information about LLVM, including information about the latest
release, please check out the `main LLVM web site <https://llvm.org/>`_.  If you
have questions or comments, the `LLVM Developer's Mailing List
<https://lists.llvm.org/mailman/listinfo/llvm-dev>`_ is a good place to send
them.

Note that if you are reading this file from a Git checkout or the main
LLVM web page, this document applies to the *next* release, not the current
one.  To see the release notes for a specific release, please see the `releases
page <https://llvm.org/releases/>`_.

Deprecated and Removed Features/APIs
=================================================
* BG/Q support, including QPX, will be removed in the 12.0.0 release.

Non-comprehensive list of changes in this release
=================================================
.. NOTE
   For small 1-3 sentence descriptions, just add an entry at the end of
   this list. If your description won't fit comfortably in one bullet
   point (e.g. maybe you would like to give an example of the
   functionality, or simply have a lot to talk about), see the `NOTE` below
   for adding a new subsection.

* ...


.. NOTE
   If you would like to document a larger change, then you can add a
   subsection about it right here. You can copy the following boilerplate
   and un-indent it (the indentation causes it to be inside this comment).

   Special New Feature
   -------------------

   Makes programs 10x faster by doing Special New Thing.


Changes to the LLVM IR
----------------------

* The callsite attribute `vector-function-abi-variant
  <https://llvm.org/docs/LangRef.html#call-site-attributes>`_ has been
  added to describe the mapping between scalar functions and vector
  functions, to enable vectorization of call sites. The information
  provided by the attribute is interfaced via the API provided by the
  ``VFDatabase`` class. When scanning through the set of vector
  functions associated with a scalar call, the loop vectorizer now
  relies on ``VFDatabase``, instead of ``TargetLibraryInfo``.

* `dereferenceable` attributes and metadata on pointers no longer imply
  anything about the alignment of the pointer in question. Previously, some
  optimizations would make assumptions based on the type of the pointer. This
  behavior was undocumented. To preserve optimizations, frontends may need to
  be updated to generate appropriate `align` attributes and metadata.

* The DIModule metadata is extended to contain file and line number
  information. This information is used to represent Fortran modules debug
  info at IR level.

* LLVM IR now supports two distinct ``llvm::FixedVectorType`` and
  ``llvm::ScalableVectorType`` vector types, both derived from the
  base class ``llvm::VectorType``. A number of algorithms dealing with
  IR vector types have been updated to make sure they work for both
  scalable and fixed vector types. Where possible, the code has been
  made generic to cover both cases using the base class. Specifically,
  places that were using the type ``unsigned`` to count the number of
  lanes of a vector are now using ``llvm::ElementCount``. In places
  where ``uint64_t`` was used to denote the size in bits of a IR type
  we have partially migrated the codebase to using ``llvm::TypeSize``.

Changes to building LLVM
------------------------

Changes to the AArch64 Backend
------------------------------

* Back up and restore x18 in functions with windows calling convention on
  non-windows OSes.

* Clearly error out on unsupported relocations when targeting COFF, instead
  of silently accepting some (without being able to do what was requested).

* Clang adds support for the following macros that enable the
  C-intrinsics from the `Arm C language extensions for SVE
  <https://developer.arm.com/documentation/100987/>`_ (version
  ``00bet5``, see section 2.1 for the list of intrinsics associated to
  each macro):


      =================================  =================
      Preprocessor macro                 Target feature
      =================================  =================
      ``__ARM_FEATURE_SVE``              ``+sve``
      ``__ARM_FEATURE_SVE_BF16``         ``+sve+bf16``
      ``__ARM_FEATURE_SVE_MATMUL_FP32``  ``+sve+f32mm``
      ``__ARM_FEATURE_SVE_MATMUL_FP64``  ``+sve+f64mm``
      ``__ARM_FEATURE_SVE_MATMUL_INT8``  ``+sve+i8mm``
      ``__ARM_FEATURE_SVE2``             ``+sve2``
      ``__ARM_FEATURE_SVE2_AES``         ``+sve2-aes``
      ``__ARM_FEATURE_SVE2_BITPERM``     ``+sve2-bitperm``
      ``__ARM_FEATURE_SVE2_SHA3``        ``+sve2-sha3``
      ``__ARM_FEATURE_SVE2_SM4``         ``+sve2-sm4``
      =================================  =================

  The macros enable users to write C/C++ `Vector Length Agnostic
  (VLA)` loops, that can be executed on any CPU that implements the
  underlying instructions supported by the C intrinsics, independently
  of the hardware vector register size.

  For example, the ``__ARM_FEATURE_SVE`` macro is enabled when
  targeting AArch64 code generation by setting ``-march=armv8-a+sve``
  on the command line.

  .. code-block:: c
     :caption: Example of VLA addition of two arrays with SVE ACLE.

     // Compile with:
     // `clang++ -march=armv8a+sve ...` (for c++)
     // `clang -stc=c11 -march=armv8a+sve ...` (for c)
     #include <arm_sve.h>

     void VLA_add_arrays(double *x, double *y, double *out, unsigned N) {
       for (unsigned i = 0; i < N; i += svcntd()) {
         svbool_t Pg = svwhilelt_b64(i, N);
         svfloat64_t vx = svld1(Pg, &x[i]);
         svfloat64_t vy = svld1(Pg, &y[i]);
         svfloat64_t vout = svadd_x(Pg, vx, vy);
         svst1(Pg, &out[i], vout);
       }
     }

  Please note that support for lazy binding of SVE function calls is
  incomplete. When you interface user code with SVE functions that are
  provided through shared libraries, avoid using lazy binding. If you
  use lazy binding, the results could be corrupted.

Changes to the ARM Backend
--------------------------

During this release ...

* Implemented C-language intrinsics for the full Arm v8.1-M MVE instruction
  set. ``<arm_mve.h>`` now supports the complete API defined in the Arm C
  Language Extensions.

* Added support for assembly for the optional Custom Datapath Extension (CDE)
  for Arm M-profile targets.

* Implemented C-language intrinsics ``<arm_cde.h>`` for the CDE instruction set.

* Clang now defaults to ``-fomit-frame-pointer`` when targeting non-Android
  Linux for arm and thumb when optimizations are enabled. Users that were
  previously not specifying a value and relying on the implicit compiler
  default may wish to specify ``-fno-omit-frame-pointer`` to get the old
  behavior. This improves compatibility with GCC.

Changes to the MIPS Target
--------------------------

During this release ...


Changes to the PowerPC Target
-----------------------------

During this release ...

Changes to the X86 Target
-------------------------

During this release ...


* Functions with the probe-stack attribute set to "inline-asm" are now protected
  against stack clash without the need of a third-party probing function and
  with limited impact on performance.
* -x86-enable-old-knl-abi command line switch has been removed. v32i16/v64i8
  vectors are always passed in ZMM register when avx512f is enabled and avx512bw
  is disabled.
* Vectors larger than 512 bits with i16 or i8 elements will be passed in
  multiple ZMM registers when avx512f is enabled. Previously this required
  avx512bw otherwise they would split into multiple YMM registers. This means
  vXi16/vXi8 vectors are consistently treated the same as
  vXi32/vXi64/vXf64/vXf32 vectors of the same total width.

Changes to the AMDGPU Target
-----------------------------

* The backend default denormal handling mode has been switched to on
  for all targets for all compute function types. Frontends wishing to
  retain the old behavior should explicitly request f32 denormal
  flushing.

Changes to the AVR Target
-----------------------------

* Moved from an experimental backend to an official backend. AVR support is now
  included by default in all LLVM builds and releases and is available under
  the "avr-unknown-unknown" target triple.

Changes to the WebAssembly Target
---------------------------------

* Programs which don't have a "main" function, called "reactors" are now
  properly supported, with a new `-mexec-model=reactor` flag. Programs which
  previously used `-Wl,--no-entry` to avoid having a main function should
  switch to this new flag, so that static initialization is properly
  performed.

* `__attribute__((visibility("protected")))` now evokes a warning, as
  WebAssembly does not support "protected" visibility.

Changes to the Windows Target
-----------------------------

* Produce COFF weak external symbols for IR level weak symbols without a comdat
  (e.g. for `__attribute__((weak))` in C)

Changes to the OCaml bindings
-----------------------------



Changes to the C API
--------------------


Changes to the Go bindings
--------------------------


Changes to the DAG infrastructure
---------------------------------


Changes to the Debug Info
---------------------------------

* LLVM now supports the debug entry values (DW_OP_entry_value) production for
  the x86, ARM, and AArch64 targets by default. Other targets can use
  the utility by using the experimental option ("-debug-entry-values").
  This is a debug info feature that allows debuggers to recover the value of
  optimized-out parameters by going up a stack frame and interpreting the values
  passed to the callee. The feature improves the debugging user experience when
  debugging optimized code.

Changes to the LLVM tools
---------------------------------

* Added an option (--show-section-sizes) to llvm-dwarfdump to show the sizes
  of all debug sections within a file.

* llvm-nm now implements the flag ``--special-syms`` and will filter out special
  symbols, i.e. mapping symbols on ARM and AArch64, by default. This matches
  the GNU nm behavior.

* llvm-rc now tolerates -1 as menu item ID, supports the language id option
  and allows string table values to be split into multiple string literals

* llvm-lib supports adding import library objects in addition to regular
  object files

Changes to LLDB
===============

External Open Source Projects Using LLVM 11
===========================================

* A project...

Additional Information
======================

A wide variety of additional information is available on the `LLVM web page
<https://llvm.org/>`_, in particular in the `documentation
<https://llvm.org/docs/>`_ section.  The web page also contains versions of the
API documentation which is up-to-date with the Git version of the source
code.  You can access versions of these documents specific to this release by
going into the ``llvm/docs/`` directory in the LLVM tree.

If you have any questions or comments about LLVM, please feel free to contact
us via the `mailing lists <https://llvm.org/docs/#mailing-lists>`_.
