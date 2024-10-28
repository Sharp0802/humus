
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

pub(crate) use log;
