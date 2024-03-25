#include <string.h>
#include <stdarg.h>
#include <stdio.h>
#include <helper.h>

static void (*global_buf_cb_t)(void *privdata, int level, const char *buf) = NULL;

void helper_format_vargs(void *privdata, int level, const char *fmt, ...)
{
	char buf[512];
	size_t len;
	va_list args;

	buf[0] = 0;
	va_start(args, fmt);
	vsnprintf(buf, sizeof(buf), fmt, args);
	va_end(args);

	len = strlen(buf);
	if (buf[len - 1] == '\n')
		buf[len - 1] = 0;

	if (global_buf_cb_t)
		global_buf_cb_t(privdata, level, buf);
}

void helper_set_global_progress_vfn(t_global_progress_vfn cb)
{
	global_buf_cb_t = cb;
}
