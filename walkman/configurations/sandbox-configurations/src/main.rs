use std::time::Duration;

use bytesize::ByteSize;
use humantime::*;

fn main() {
    println!("{} of {}", ByteSize::b(1024), ByteSize::b(3736442));
    println!("{}/s", ByteSize::b(511976.0753367505 as u64));
    println!("{}", 1024 as f64 / 3736442 as f64);

    let seconds = 0.18876147270202637_f64;
    let duration = Duration::from_secs_f64(seconds);
    println!("{}", format_duration(duration));
}
