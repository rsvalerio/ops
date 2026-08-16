//! Test-only scaffolding shared across the crate's test modules.
//!
//! TASK-1494 consolidated the `tracing` capture scaffold into a single
//! definition. TASK-1670 moved it here when the flat `tests.rs` was split:
//! its call sites now sit in two different modules (`parse/upgrade` and
//! `parse/deny`), and giving each one its own copy would silently revert
//! that consolidation.

use std::io::Write;
use std::sync::{Arc, Mutex};
use tracing::Level;
use tracing_subscriber::fmt::MakeWriter;

#[derive(Clone, Default)]
pub struct BufWriter(pub Arc<Mutex<Vec<u8>>>);

impl Write for BufWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0.lock().unwrap().extend_from_slice(buf);
        Ok(buf.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl<'a> MakeWriter<'a> for BufWriter {
    type Writer = BufWriter;
    fn make_writer(&'a self) -> Self::Writer {
        self.clone()
    }
}

pub fn with_captured_logs<F, R>(level: Level, ansi: bool, f: F) -> (R, String)
where
    F: FnOnce() -> R,
{
    let buf = BufWriter::default();
    let subscriber = tracing_subscriber::fmt()
        .with_max_level(level)
        .with_writer(buf.clone())
        .with_ansi(ansi)
        .without_time()
        .finish();
    let result = tracing::subscriber::with_default(subscriber, f);
    let logged = String::from_utf8(buf.0.lock().unwrap().clone()).unwrap();
    (result, logged)
}
