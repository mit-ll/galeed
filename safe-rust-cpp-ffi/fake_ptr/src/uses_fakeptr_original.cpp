#include "libfake_ptr.h"
#include <stdio.h>

#ifdef _WIN32
#include <intrin.h>
#else
#include <x86intrin.h>
#endif

inline 
uint64_t __rdtscp() {
    unsigned dummy;
    return __rdtscp(&dummy);
}

// User requirement - pointer inside must not be changed to another pointer
extern "C" uint64_t acton_mystruct_unsafe(MyStruct* const p) {
    #pragma clang diagnostic push
    #pragma clang diagnostic ignored "-Wunused-variable"

    // TODO - make sure this works, otherwise make declaration global
    // TODO - assemble RDTSC instruction (get cycles), steps of 10,000

    uint64_t start = __rdtscp();
    volatile int x = p->x;
    // p->x = 5;
    // p->x = x + 5;
    return __rdtscp() - start;

    // TODO - time here, return timing value to Rust

    #pragma clang diagnostic pop
}
