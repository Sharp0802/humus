
///
/// Log messages with specified severity levels.
///
/// This function allows you to log messages to stdout or stderr with
/// different severity levels. The log messages will include a
/// severity label and a timestamp.
///
/// ## Usage
///
/// ```rust
/// use humus_terra::log;
///
/// log!(<severity> <format> <arguments>...);
/// ```
///
/// ## Output Format
///
/// The generated log message will appear in the following format:
///
/// ```txt
/// [severity] [timestamp] formatted message...
/// ```
///
/// - **Severity**: This is the capitalised version of the severity argument.
/// - **Timestamp**: This follows the ISO 8601 / RFC 3339 date and time format.
///
/// ## Severity Levels
///
/// Only the following severity levels are permitted:
///
/// - `fail`
/// - `warn`
/// - `info`
///
/// ## Examples
///
/// Here are some examples of how to use the logging function:
///
/// ```rust
/// use humus_terra::log;
///
/// log!(info "Hello!");
/// log!(warn "Error with {}", "Such error");
/// ```
///
#[macro_export]
macro_rules! log {
    (fail $($args:expr),+) => {
        eprintln!(
            "[FAIL] [{}] {}",
            chrono::offset::Utc::now().format("%+"),
            format!($($args),+));
    };

    (warn $($args:expr),+) => {
        eprintln!(
            "[WARN] [{}] {}",
            chrono::offset::Utc::now().format("%+"),
            format!($($args),+));
    };

    (info $($args:expr),+) => {
        println!(
            "[INFO] [{}] {}",
            chrono::offset::Utc::now().format("%+"),
            format!($($args),+));
    };
}
