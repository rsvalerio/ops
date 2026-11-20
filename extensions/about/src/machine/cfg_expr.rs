//! Evaluation of cargo's `cfg(..)` target-table keys against the host's
//! `rustc --print cfg` atoms.

/// Parse `rustc --print cfg` output into its atoms (`unix`,
/// `target_os="linux"`, ...), one per line, whitespace-trimmed.
#[must_use]
pub fn parse_rustc_cfg(stdout: &str) -> Vec<String> {
    stdout.lines().map(str::trim).filter(|l| !l.is_empty()).map(str::to_string).collect()
}

/// Deepest `all(..)` / `any(..)` / `not(..)` nesting the evaluator follows.
///
/// The parser recurses once per level and the key comes from a
/// repo-controlled `.cargo/config.toml`, so an unbounded
/// `cfg(not(not(not(...))))` would overflow the stack. Real cargo cfg
/// expressions nest a handful of levels; 32 leaves ample headroom while
/// keeping the recursion far below any thread's stack. Deeper keys are
/// treated as unparseable, so they do not match.
const MAX_CFG_DEPTH: usize = 32;

/// Whether a `target` table key like `cfg(all(unix, target_arch = "x86_64"))`
/// holds for a host whose `rustc --print cfg` atoms are `host_cfg`.
///
/// Supports cargo's grammar: bare names, `name = "value"`, `all(..)`,
/// `any(..)` and `not(..)`. A key that is not a `cfg(..)` expression, that
/// does not parse, or that nests deeper than `MAX_CFG_DEPTH`, does not match.
#[must_use]
pub fn cfg_matches(key: &str, host_cfg: &[String]) -> bool {
    let Some(inner) = key.trim().strip_prefix("cfg(").and_then(|rest| rest.strip_suffix(')'))
    else {
        return false;
    };
    let mut parser = CfgParser { rest: inner, depth: 0 };
    let Some(value) = parser.predicate(host_cfg) else {
        tracing::debug!(key, "about/machine: unparseable cfg target table");
        return false;
    };
    parser.rest.trim().is_empty() && value
}

/// Recursive-descent evaluator over a `cfg(..)` body.
struct CfgParser<'a> {
    rest: &'a str,
    /// Current `(..)` list nesting, bounded by `MAX_CFG_DEPTH`.
    depth: usize,
}

impl CfgParser<'_> {
    fn eat(&mut self, token: &str) -> bool {
        let trimmed = self.rest.trim_start();
        trimmed.strip_prefix(token).is_some_and(|after| {
            self.rest = after;
            true
        })
    }

    fn ident(&mut self) -> Option<&str> {
        let trimmed = self.rest.trim_start();
        let end = trimmed
            .find(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
            .unwrap_or(trimmed.len());
        if end == 0 {
            return None;
        }
        let (ident, after) = trimmed.split_at(end);
        self.rest = after;
        Some(ident)
    }

    fn string(&mut self) -> Option<&str> {
        let trimmed = self.rest.trim_start().strip_prefix('"')?;
        let end = trimmed.find('"')?;
        let (value, after) = trimmed.split_at(end);
        self.rest = after.strip_prefix('"')?;
        Some(value)
    }

    fn list(&mut self, host_cfg: &[String]) -> Option<Vec<bool>> {
        if self.depth >= MAX_CFG_DEPTH || !self.eat("(") {
            return None;
        }
        let outer = self.depth;
        self.depth = outer.checked_add(1)?;
        let values = self.list_items(host_cfg);
        self.depth = outer;
        values
    }

    fn list_items(&mut self, host_cfg: &[String]) -> Option<Vec<bool>> {
        let mut values = Vec::new();
        loop {
            if self.eat(")") {
                return Some(values);
            }
            values.push(self.predicate(host_cfg)?);
            if !self.eat(",") && !self.rest.trim_start().starts_with(')') {
                return None;
            }
        }
    }

    fn predicate(&mut self, host_cfg: &[String]) -> Option<bool> {
        let name = self.ident()?.to_string();
        match name.as_str() {
            "all" => Some(self.list(host_cfg)?.into_iter().all(|v| v)),
            "any" => Some(self.list(host_cfg)?.into_iter().any(|v| v)),
            "not" => {
                let values = self.list(host_cfg)?;
                match values.as_slice() {
                    [value] => Some(!value),
                    _ => None,
                }
            }
            _ if self.eat("=") => {
                let atom = format!("{name}=\"{}\"", self.string()?);
                Some(host_cfg.contains(&atom))
            }
            _ => Some(host_cfg.contains(&name)),
        }
    }
}

#[cfg(test)]
pub(super) mod tests {
    use super::*;

    pub(in crate::machine) fn linux_cfg() -> Vec<String> {
        parse_rustc_cfg(
            "debug_assertions\npanic=\"unwind\"\ntarget_arch=\"x86_64\"\ntarget_os=\"linux\"\nunix\n",
        )
    }

    #[test]
    fn cfg_matches_evaluates_cargo_cfg_grammar() {
        let cfg = linux_cfg();
        assert!(cfg_matches("cfg(unix)", &cfg));
        assert!(!cfg_matches("cfg(windows)", &cfg));
        assert!(cfg_matches("cfg(target_os = \"linux\")", &cfg));
        assert!(!cfg_matches("cfg(target_os = \"macos\")", &cfg));
        assert!(cfg_matches("cfg(all(unix, target_arch = \"x86_64\"))", &cfg));
        assert!(cfg_matches("cfg(any(windows, unix))", &cfg));
        assert!(cfg_matches("cfg(not(windows))", &cfg));
        assert!(!cfg_matches("cfg(all(unix, not(target_os = \"linux\")))", &cfg));
        assert!(!cfg_matches("x86_64-unknown-linux-gnu", &cfg), "a triple is not a cfg");
        assert!(!cfg_matches("cfg(all(unix", &cfg), "malformed never matches");
        assert!(!cfg_matches("cfg(not(unix, windows))", &cfg));
    }

    /// A key nested far past `MAX_CFG_DEPTH` is unparseable and does not
    /// match, instead of overflowing the stack; nesting at the limit still
    /// evaluates.
    #[test]
    fn deeply_nested_cfg_key_does_not_match_or_overflow() {
        let cfg = linux_cfg();
        let depth = 100_000;
        let key = format!("cfg({}unix{})", "not(".repeat(depth), ")".repeat(depth));
        assert!(!cfg_matches(&key, &cfg));

        let at_limit =
            format!("cfg({}unix{})", "not(".repeat(MAX_CFG_DEPTH), ")".repeat(MAX_CFG_DEPTH));
        assert!(cfg_matches(&at_limit, &cfg), "an even number of `not`s keeps `unix` true");
        let over = format!(
            "cfg({}unix{})",
            "not(".repeat(MAX_CFG_DEPTH + 1),
            ")".repeat(MAX_CFG_DEPTH + 1)
        );
        assert!(!cfg_matches(&over, &cfg));
    }
}
