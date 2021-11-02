#include <sched.h>

extern "C" void pin_thread() {
    cpu_set_t  mask;
    CPU_ZERO(&mask);
    CPU_SET(0, &mask);
    sched_setaffinity(0, sizeof(mask), &mask);
    sched_param p;
    p.sched_priority = sched_get_priority_max(SCHED_FIFO);
    sched_setscheduler(0, SCHED_FIFO, &p);
}