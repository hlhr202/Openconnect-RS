// #include <openconnect.h>

typedef void (*t_global_progress_vfn)(void *privdata, int level, const char *buf);
void helper_format_vargs(void *privdata, int level, const char *fmt, ...);
void helper_set_global_progress_vfn(t_global_progress_vfn cb);