#include <cstdint>
#include <stdio.h>
#include <stdlib.h>

#ifdef _WIN32
#include <intrin.h>
#else
#include <x86intrin.h>
#endif

extern "C" uint64_t __rdtscp() {
    unsigned dummy;
    return __rdtscp(&dummy);
}

struct Point {
    int32_t x;
};

extern "C" uint64_t read_int_ptr(int* const p) {
    #pragma clang diagnostic push
    #pragma clang diagnostic ignored "-Wunused-variable"

    uint64_t start = __rdtscp();
    volatile int x = *p;
    return __rdtscp() - start;

    #pragma clang diagnostic pop
}

extern "C" uint64_t write_int_ptr(int* const p) {
    #pragma clang diagnostic push
    #pragma clang diagnostic ignored "-Wunused-variable"

    uint64_t start = __rdtscp();
    *p = 5;
    return __rdtscp() - start;

    #pragma clang diagnostic pop
}


extern "C" uint64_t read_write_int_ptr(int* const p) {
    #pragma clang diagnostic push
    #pragma clang diagnostic ignored "-Wunused-variable"

    uint64_t start = __rdtscp();
    volatile int x = *p;
    *p = x + 5;
    return __rdtscp() - start;

    #pragma clang diagnostic pop
}