use crate::ffi::{c_str_to_option_string, ffi_catch_unwind};
use openconnect_sys::{PRG_DEBUG, PRG_ERR, PRG_INFO, PRG_TRACE};
use tracing::{
    Level, event,
    subscriber::{SetGlobalDefaultError, set_global_default},
};
use tracing_appender::rolling::{RollingFileAppender, Rotation};

pub struct Logger;

impl Logger {
    pub fn get_log_path() -> &'static str {
        #[cfg(target_os = "linux")]
        const LOG_PATH: &str = "/var/log/openconnect-rs";

        #[cfg(target_os = "macos")]
        const LOG_PATH: &str = "/Library/Logs/openconnect-rs";

        #[cfg(target_os = "windows")]
        const LOG_PATH: &str = "C:\\ProgramData\\openconnect-rs";

        LOG_PATH
    }

    pub fn init() -> Result<(), SetGlobalDefaultError> {
        let file_appender = RollingFileAppender::builder()
            .max_log_files(5)
            .rotation(Rotation::DAILY)
            .filename_prefix("openconnect-rs.log")
            .build(Self::get_log_path())
            .expect("failed to create file appender");

        let subscriber = tracing_subscriber::fmt()
            .compact()
            .with_level(true)
            .with_target(true)
            .with_max_level(Level::TRACE)
            .with_writer(file_appender)
            .finish();

        set_global_default(subscriber)
    }

    /// C progress log callback invoked by `helper_format_vargs`.
    ///
    /// # Safety
    /// If `buf` is non-null, it must point to a valid NUL-terminated C string for the
    /// duration of this call.
    pub(crate) unsafe extern "C" fn raw_handle_process_log(
        _privdata: *mut ::std::os::raw::c_void,
        level: ::std::os::raw::c_int,
        buf: *const ::std::os::raw::c_char,
    ) {
        ffi_catch_unwind((), || {
            // SAFETY: Caller guarantees `buf`, when non-null, is a valid NUL-terminated C string.
            let Some(message) = (unsafe { c_str_to_option_string(buf) }) else {
                return;
            };

            let level = match level as u32 {
                PRG_ERR => Level::ERROR,
                PRG_INFO => Level::INFO,
                PRG_DEBUG => Level::DEBUG,
                PRG_TRACE => Level::TRACE,
                _ => Level::TRACE,
            };

            Logger::log(level, &message);
        });
    }

    pub fn log(level: Level, message: &str) {
        match level {
            Level::ERROR => event!(Level::ERROR, "{}", message),
            Level::WARN => event!(Level::WARN, "{}", message),
            Level::INFO => event!(Level::INFO, "{}", message),
            Level::DEBUG => event!(Level::DEBUG, "{}", message),
            Level::TRACE => event!(Level::TRACE, "{}", message),
        }
    }
}
