// Logging macros
#[macro_export]
macro_rules! redacted_info {
    ($($arg:tt)*) => {
        tracing::info!($($arg)*)
    };
}

#[macro_export]
macro_rules! redacted_error {
    ($($arg:tt)*) => {
        tracing::error!($($arg)*)
    };
}