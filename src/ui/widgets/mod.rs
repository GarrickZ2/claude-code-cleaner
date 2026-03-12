use bytesize::ByteSize;
use chrono::{DateTime, Local};

/// Format bytes into human-readable string
pub fn format_size(bytes: u64) -> String {
    ByteSize::b(bytes).to_string()
}

/// Format a datetime as relative age (e.g., "3d ago")
pub fn format_age(dt: &DateTime<Local>) -> String {
    let now = Local::now();
    let duration = now.signed_duration_since(*dt);

    if duration.num_days() > 365 {
        format!("{}y ago", duration.num_days() / 365)
    } else if duration.num_days() > 30 {
        format!("{}mo ago", duration.num_days() / 30)
    } else if duration.num_days() > 0 {
        format!("{}d ago", duration.num_days())
    } else if duration.num_hours() > 0 {
        format!("{}h ago", duration.num_hours())
    } else {
        "just now".to_string()
    }
}

/// Generate a simple bar chart string
pub fn bar_chart(ratio: f64, width: usize) -> String {
    let filled = (ratio * width as f64).round() as usize;
    let empty = width.saturating_sub(filled);
    format!("{}{}", "\u{2588}".repeat(filled), "\u{2591}".repeat(empty))
}
