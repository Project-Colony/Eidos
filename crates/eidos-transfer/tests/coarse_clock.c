// Give transfers identical subsecond timestamps without changing elapsed time.
#define _GNU_SOURCE
#include <dlfcn.h>
#include <stdlib.h>
#include <time.h>

int clock_gettime(clockid_t id, struct timespec *ts) {
    int (*real_clock)(clockid_t, struct timespec *) = dlsym(RTLD_NEXT, "clock_gettime");
    if (!real_clock)
        abort();
    int result = real_clock(id, ts);
    if (result == 0 && id == CLOCK_REALTIME)
        ts->tv_nsec = 0;
    return result;
}
