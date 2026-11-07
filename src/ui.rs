use indicatif::{ProgressBar, ProgressStyle};
use std::time::Duration;

pub const GREEN_BOLD: &str = "\x1b[32;1m";
pub const YELLOW_BOLD: &str = "\x1b[33;1m";
pub const RED_BOLD: &str = "\x1b[31;1m";
pub const RESET: &str = "\x1b[0m";

#[inline]
pub fn spinner(msg: impl Into<std::borrow::Cow<'static, str>>) -> ProgressBar {
    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.green} {msg}")
            .unwrap_or_else(|_| ProgressStyle::default_spinner()),
    );
    pb.set_message(msg);
    pb.enable_steady_tick(Duration::from_millis(100));
    pb
}

#[inline]
pub fn success(msg: impl std::fmt::Display) {
    println!("{}[Success]{} {}", GREEN_BOLD, RESET, msg);
    log::info!("{}", msg);
}

#[inline]
pub fn warning(msg: impl std::fmt::Display) {
    eprintln!("{}[Warning]{} {}", YELLOW_BOLD, RESET, msg);
    log::warn!("{}", msg);
}

#[inline]
pub fn error(msg: impl std::fmt::Display) {
    eprintln!("{}[Error]{} {}", RED_BOLD, RESET, msg);
    log::error!("{}", msg);
}
#[inline]
pub fn info(msg: impl std::fmt::Display) {
    println!("[Info] {}", msg);
    log::info!("{}", msg);
}

#[inline]
pub fn suggestion(msg: impl std::fmt::Display) {
    println!("{}[Suggestion]{} {}", YELLOW_BOLD, RESET, msg);
    log::info!("Suggestion: {}", msg);
}

#[inline]
pub fn newline() {
    println!();
}

pub fn confirm(prompt_msg: &str) -> bool {
    use std::io::Write;
    print!("{}", prompt_msg);
    let _ = std::io::stdout().flush();
    let mut input = String::new();
    let _ = std::io::stdin().read_line(&mut input);
    let trimmed = input.trim();
    trimmed.eq_ignore_ascii_case("y") || trimmed.eq_ignore_ascii_case("yes")
}

#[cfg(windows)]
pub fn prompt_exit() {
    eprintln!("\nPress Enter to exit...");
    let _ = std::io::stdin().read_line(&mut String::new());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spinner_creation_and_finish() {
        let sp = spinner("Testing spinner...");
        sp.set_message("Updated message");
        sp.finish_and_clear();
        assert!(sp.is_finished());
    }
}
