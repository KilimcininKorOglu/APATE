pub const CHROME_131: &str = "chrome_131";
pub const FIREFOX_130: &str = "firefox_130";
pub const SAFARI_18: &str = "safari_18";

pub fn is_builtin_profile(name: &str) -> bool {
    matches!(name, CHROME_131 | FIREFOX_130 | SAFARI_18)
}

#[cfg(test)]
mod tests {
    use crate::config::profiles::{CHROME_131, is_builtin_profile};

    #[test]
    fn chrome_profile_exists() {
        assert!(is_builtin_profile(CHROME_131));
    }
}
