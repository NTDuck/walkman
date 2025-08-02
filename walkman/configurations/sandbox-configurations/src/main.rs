fn extract_final_segment(s: &str) -> Option<&str> {
    s.rfind("] ").map(|i| &s[i + 2..])
}

fn main() {
    let input = "[abc @ 123] final_text";
    if let Some(result) = extract_final_segment(input) {
        println!("Extracted: {}", result); // → "final_text"
    }
}
