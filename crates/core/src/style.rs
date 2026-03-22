//! ANSI terminal styling helpers for CLI output.

macro_rules! ansi_style {
    ($(#[$meta:meta])* $name:ident, $code:expr) => {
        $(#[$meta])*
        pub fn $name(s: &str) -> String {
            format!("\x1b[{}m{}\x1b[0m", $code, s)
        }
    };
}

ansi_style!(cyan, 36);
ansi_style!(white, 37);
ansi_style!(grey, 90);
ansi_style!(dim, 2);
ansi_style!(green, 32);
ansi_style!(red, 31);
ansi_style!(yellow, 33);
ansi_style!(bold, 1);
