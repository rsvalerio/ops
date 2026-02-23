//! ANSI terminal styling helpers for CLI output.

pub fn cyan(s: &str) -> String {
    format!("\x1b[36m{}\x1b[0m", s)
}

pub fn dim(s: &str) -> String {
    format!("\x1b[2m{}\x1b[0m", s)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cyan_wraps_with_ansi_codes() {
        let result = cyan("test");
        assert!(result.starts_with("\x1b[36m"));
        assert!(result.ends_with("\x1b[0m"));
        assert!(result.contains("test"));
    }

    #[test]
    fn dim_wraps_with_ansi_codes() {
        let result = dim("test");
        assert!(result.starts_with("\x1b[2m"));
        assert!(result.ends_with("\x1b[0m"));
        assert!(result.contains("test"));
    }

    #[test]
    fn cyan_empty_string() {
        let result = cyan("");
        assert_eq!(result, "\x1b[36m\x1b[0m");
    }

    #[test]
    fn dim_empty_string() {
        let result = dim("");
        assert_eq!(result, "\x1b[2m\x1b[0m");
    }
}
