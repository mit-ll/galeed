This repository contains the source code behind the ACSAC '21 paper "Keeping Safe Rust Safe with Galeed"

## Firefox

We modified both libMPK and Firefox to separate the Rust heap from the rest of the MPK heap, and assign the Rust heap to an MPK key.  
When Rust code is executing, the heap has RW permissions, other wise RO.  We evaluated this on the parser module, which uses Rust to 
read in data which is only read in C++.  

See README in Firefox-patch for directions to obtain the firefox source code and
apply our patch.

## LLVM

The MPK scheme is too restrictive if C++ needs to write to Rust allocated memory.  We modified LLVM (adding llvm_fakeptr/llvm/lib/Transforms/Utils/FakePtrPass.cpp) to instantiate the fake pointer 
scheme described in the paper.  Our micro-benchmark showing how to use it is included in safe-rust-cpp-ffi.

## Disclaimer

Galeed is distributed under the terms of the MIT License DISTRIBUTION STATEMENT A. Approved for public release: distribution unlimited.

© 2021 MASSACHUSETTS INSTITUTE OF TECHNOLOGY


    Subject to FAR 52.227-11 – Patent Rights – Ownership by the Contractor (May 2014)
    SPDX-License-Identifier: MIT

This material is based upon work supported by the Under Secretary of Defense (USD) for Research & Engineering (R&E) under Air Force Contract No. FA8702-15-D-0001. Any opinions, findings, conclusions or recommendations expressed in this material are those of the author(s) and do not necessarily reflect the views of USD (R&E).

The software/firmware is provided to you on an As-Is basis
