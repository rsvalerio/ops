//! Shared serde default functions.

pub const fn default_true() -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_true_returns_true() {
        assert!(default_true());
    }
}
