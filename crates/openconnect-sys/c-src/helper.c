#include <stdarg.h>
#include <stdio.h>
#include <string.h>
#include "helper.h"

#if defined(_MSC_VER) && !defined(__clang__)
#include <windows.h>
static void *volatile global_buf_cb = NULL;

static t_global_progress_vfn load_global_cb(void)
{
    return (t_global_progress_vfn)InterlockedCompareExchangePointer(&global_buf_cb, NULL, NULL);
}

static void store_global_cb(t_global_progress_vfn cb)
{
    InterlockedExchangePointer(&global_buf_cb, (void *)cb);
}
#else
#include <stdatomic.h>
static _Atomic(t_global_progress_vfn) global_buf_cb = NULL;

static t_global_progress_vfn load_global_cb(void)
{
    return atomic_load_explicit(&global_buf_cb, memory_order_acquire);
}

static void store_global_cb(t_global_progress_vfn cb)
{
    atomic_store_explicit(&global_buf_cb, cb, memory_order_release);
}
#endif

void helper_format_vargs(void *privdata, int level, const char *fmt, ...)
{
    char buf[512];
    int written;
    size_t len;
    va_list args;
    t_global_progress_vfn cb;

    if (!fmt) {
        return;
    }

    buf[0] = '\0';
    va_start(args, fmt);
    written = vsnprintf(buf, sizeof(buf), fmt, args);
    va_end(args);

    if (written < 0) {
        return;
    }

    len = strlen(buf);
    if (len > 0 && buf[len - 1] == '\n') {
        buf[len - 1] = '\0';
    }

    cb = load_global_cb();
    if (cb) {
        cb(privdata, level, buf);
    }
}

void helper_set_global_progress_vfn(t_global_progress_vfn cb)
{
    store_global_cb(cb);
}
