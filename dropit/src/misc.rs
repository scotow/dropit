use std::time::Duration;

#[macro_export]
macro_rules! exit_error {
    ($($arg:tt)+) => {
        {
            log::error!($($arg)+);
            std::process::exit(1)
        }
    }
}

pub fn format_duration(duration: Duration) -> String {
    static UNITS: [(u64, &str); 5] = [
        (365 * 24 * 60 * 60, "year"),
        (24 * 60 * 60, "day"),
        (60 * 60, "hour"),
        (60, "min"),
        (1, "sec"),
    ];

    fn plural(n: u64, word: &str) -> String {
        if n >= 2 {
            format!("{} {}s", n, word)
        } else {
            format!("{} {}", n, word)
        }
    }

    let mut secs = duration.as_secs();
    if secs == 0 {
        return String::from("now");
    }

    let mut parts = Vec::with_capacity(2);
    for &(unit, word) in UNITS.iter() {
        if secs >= unit {
            let n = secs / unit;
            parts.push(plural(n, word));
            secs -= n * unit;
        }
        if parts.len() == 2 {
            break;
        }
    }
    if parts.len() == 1 {
        parts.into_iter().next().unwrap()
    } else {
        format!(
            "{} and {}",
            parts[..parts.len() - 1].join(", "),
            parts.last().unwrap()
        )
    }
}
