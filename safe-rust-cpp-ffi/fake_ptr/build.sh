#!/bin/bash
mkdir -p temp_cpp
clang++ src/uses_fakeptr_original.cpp -O0 -Werror -emit-llvm -S -o temp_cpp/uses_fakeptr_original.ll
clang++ src/uses_fakeptr_safe.cpp -O0 -Werror -emit-llvm -S -o temp_cpp/uses_fakeptr_safe.ll -mllvm -use-fakeptr
cargo build --release
