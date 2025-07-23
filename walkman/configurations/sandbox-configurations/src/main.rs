use regex::Regex;

fn update_percent(s: &str, new_percent: u8) -> String {
    static RE: once_cell::sync::Lazy<Regex> = once_cell::sync::Lazy::new(|| {
        Regex::new(r"^\s*\d+%\s{2}").unwrap()
    });

    RE.replace(s, format!("{:>3}%  ", new_percent)).into_owned()
}

fn main() {
    let original_percents = [0, 1, 10, 78, 100];
    let new_percents = [0, 1, 10, 78, 100];

    for &orig in &original_percents {
        let original_str = format!("{:>3}%  My Video Title", orig);

        for &new in &new_percents {
            let updated = update_percent(&original_str, new);
            println!("Original: {:<24} | New: {:>3} | Updated: {}", original_str, new, updated);
        }
    }
}
