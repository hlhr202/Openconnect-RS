#include <string.h>
#include <stdarg.h>
#include <stdio.h>
#include <helper.h>

#ifdef __MACH__
#include <Security/Security.h>
#include <mach-o/dyld.h>
#endif

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

#ifdef __MACH__
int helper_reluanch_as_root(void)
{
	char app_path[1024];
	uint32_t size = sizeof(app_path);
	AuthorizationRef auth_ref;
	OSStatus status;

	if (_NSGetExecutablePath(app_path, &size) != 0)
	{
		// TODO: handle error, can't get path
		return 0;
	}

	status = AuthorizationCreate(NULL, kAuthorizationEmptyEnvironment, kAuthorizationFlagDefaults, &auth_ref);

	if (status != errAuthorizationSuccess)
	{
		// TODO: handle error, can't create auth ref
		return 0;
	}

	status = AuthorizationExecuteWithPrivileges(auth_ref, app_path,
												kAuthorizationFlagDefaults, NULL, NULL);

	AuthorizationFree(auth_ref, kAuthorizationFlagDestroyRights);

	if (status == errAuthorizationSuccess)
	{
		/* We've successfully re-launched with root privs. */
		return 1;
	}

	return 0;
}
#endif