#include <stdio.h>
#include <stdlib.h>
#include "../../crates/openconnect-sys/c-src/helper.h"

static void dummy_cb(void *privdata, int level, const char *buf) {
    (void)privdata;
    (void)level;
    printf("callback received: \"%s\"\n", buf);
}

int main(void) {
    helper_set_global_progress_vfn(dummy_cb);
    /*
     * When formatted string is empty (""), strlen(buf) == 0.
     * helper.c:20 executes `if (buf[len - 1] == '\n')` -> `buf[(size_t)-1]` = `buf[-1]`,
     * triggering a stack-buffer-underflow!
     */
    helper_format_vargs(NULL, 0, "%s", "");
    return 0;
}
