use regex::Regex;

fn update_percentage(message: &str, new_percent: u8) -> String {
    // Matches optional spaces + 1-3 digits or ??, followed by "%  "
    static RE: once_cell::sync::Lazy<Regex> = once_cell::sync::Lazy::new(|| {
        Regex::new(r"^\s*(\d{1,3}|\?{2})%  ").unwrap()
    });

    RE.replace(message, format!("{:>3}%  ", new_percent)).into_owned()
}

fn main() {
    let msg = "100%  Cool Title";
    let updated = update_percentage(msg, 42);
    println!("{}", updated); // Output: " 42%  Cool Title"
}
