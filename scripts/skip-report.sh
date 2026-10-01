#!/bin/sh
# TEST-26 / TASK-2252: print the `skip: …` lines that passing tests wrote into the
# JUnit report of the last `ops next` run (.config/nextest.toml stores their
# captured output there). Run by `ops skip-report`, part of `ops qa`.
#
# Skips warn, never fail: the guards that emit them exist so environments lacking a
# capability can still run the suite. A missing report fails, or the gate would show
# an unexplained green.
set -eu

# nextest's store dir is `target/nextest` under the workspace root whatever
# CARGO_TARGET_DIR says; `ops qa` runs this from the root.
report=target/nextest/default/junit.xml
if [ ! -f "$report" ]; then
  echo "skip-report: no JUnit report at $report; run ops next first" >&2
  exit 1
fi

count=$(grep -o 'skip: [^<]*' "$report" | wc -l)
if [ "$count" -gt 0 ]; then
  msg="$count skip_precondition line(s) were printed by passing tests; the assertions behind them did not run"
  if [ -n "${GITHUB_ACTIONS:-}" ]; then echo "::warning::$msg"; else echo "warning: $msg"; fi
  grep -o 'skip: [^<]*' "$report" | sort | uniq -c
else
  echo "No skip_precondition lines: every guard that tripped also ran its assertions."
fi
