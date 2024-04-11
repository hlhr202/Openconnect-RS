use openconnect_sys::{PRG_DEBUG, PRG_ERR, PRG_INFO, PRG_TRACE};
use tracing::{
    event,
    subscriber::{set_global_default, SetGlobalDefaultError},
    Level,
};

pub struct Logger;

impl Logger {
    pub fn init() -> Result<(), SetGlobalDefaultError> {
        // for file based logging, waiting https://github.com/tokio-rs/tracing/pull/2497 to be merged
        let subscriber = tracing_subscriber::fmt()
            .compact()
            .with_level(true)
            .with_target(true)
            .with_max_level(Level::TRACE)
            .finish();

        set_global_default(subscriber)
    }

    pub(crate) unsafe extern "C" fn raw_handle_process_log(
        _privdata: *mut ::std::os::raw::c_void,
        level: ::std::os::raw::c_int,
        buf: *const ::std::os::raw::c_char,
    ) {
        let buf = std::ffi::CStr::from_ptr(buf).to_str().ok();
        let level = level as u32;
        let level = match level {
            PRG_ERR => Level::ERROR,
            PRG_INFO => Level::INFO,
            PRG_DEBUG => Level::DEBUG,
            PRG_TRACE => Level::TRACE,
            _ => unreachable!("unknown log level: {}", level),
        };
        if buf.is_some() {
            Logger::log(level, buf.unwrap_or(""));
        }
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
