use chrono::{LocalResult, NaiveDate}; // Import necessary types from chrono
use chrono_tz::Tz; // Import the Tz type from chrono-tz for timezone handling

fn main() {
    // Example of an ambiguous datetime due to DST ending in New York
    // Assuming DST ends and clocks are set back at 2:00 AM to 1:00 AM, making 1:30 AM ambiguous
    let naive_date = NaiveDate::from_ymd_opt(2023, 11, 5)
        .unwrap()
        .and_hms_opt(1, 30, 0)
        .unwrap();
    let tz: Tz = "America/New_York".parse().expect("Invalid timezone"); // Parse the timezone

    // Attempt to resolve the ambiguous time in the New York timezone
    match naive_date.and_local_timezone(tz) {
        LocalResult::Single(datetime) => println!("Non-ambiguous datetime: {}", datetime),
        LocalResult::Ambiguous(early, late) => {
            println!("Ambiguous time could be either:");
            println!("Early: {}, {}", early, early.to_rfc3339());
            println!("Late: {}, {}", late, late.to_rfc3339());
        }
        LocalResult::None => println!("Time does not exist."),
    }
}
