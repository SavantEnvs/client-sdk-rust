#!/usr/bin/env bash
#
# mayhem/test.sh — RUN this repo's OWN functional test suite (already built by mayhem/build.sh).
# exit 0 = pass. EDIT per repo. PATCH-grade oracle: after an agent patches the source, the grader
# rebuilds (build.sh) then runs this. DELETE this file if the repo has no meaningful tests.
#
# IMPORTANT:
#  * Must assert BEHAVIOR/OUTPUT, not just exit status. The oracle has to check asserted values /
#    golden-output diffs / known-answer results — so a PATCH that "fixes" a bug by making the program
#    exit(0) (or any no-op) FAILS here. Running inputs and checking only "exit 0 / didn't crash" is
#    NOT a functional test (it's trivially reward-hackable) — use the project's real assertion suite.
#  * Do NOT build here — mayhem/build.sh already compiled the test suite (with the project's normal
#    flags). This script only RUNS the pre-built tests and reports counts. If the test runner is
#    missing, that's a build.sh bug — fail loudly rather than silently rebuilding.
#  * REQUIRED OUTPUT — a CTRF (https://ctrf.io) summary so Mayhem/the PATCH grader reads the counts:
#      - writes a CTRF JSON report to ${CTRF_REPORT:-$SRC/ctrf-report.json}, and
#      - prints a one-line `CTRF {...}` marker to stdout (same JSON, compact).
#    Only `results.summary` (with tests/passed/failed/pending/skipped/other) is required.
#    Use the emit_ctrf helper below; it computes tests = passed+failed+skipped and sets the exit
#    code (0 iff failed==0). Map your framework's output to passed/failed/skipped.
set -uo pipefail
[ -n "${SOURCE_DATE_EPOCH:-}" ] || unset SOURCE_DATE_EPOCH
: "${MAYHEM_JOBS:=$(nproc)}"   # build parallelism; env-overridable, falls back to nproc (use -j"$MAYHEM_JOBS")
cd "$SRC"

# emit_ctrf <tool> <passed> <failed> [skipped] [pending] [other]
# Writes a CTRF report (file + stdout `CTRF {...}` marker) and returns non-zero iff failed>0.
emit_ctrf() {
  local tool="$1" passed="$2" failed="$3" skipped="${4:-0}" pending="${5:-0}" other="${6:-0}"
  local tests=$(( passed + failed + skipped + pending + other ))
  cat > "${CTRF_REPORT:-$SRC/ctrf-report.json}" <<JSON
{
  "results": {
    "tool": { "name": "$tool" },
    "summary": {
      "tests": $tests,
      "passed": $passed,
      "failed": $failed,
      "pending": $pending,
      "skipped": $skipped,
      "other": $other
    }
  }
}
JSON
  printf 'CTRF {"results":{"tool":{"name":"%s"},"summary":{"tests":%d,"passed":%d,"failed":%d,"pending":%d,"skipped":%d,"other":%d}}}\n' \
    "$tool" "$tests" "$passed" "$failed" "$pending" "$skipped" "$other"
  [ "$failed" -eq 0 ]
}

# Oracle: the dynamically linked known-answer probe built by mayhem/build.sh. Each `KAT <name> ok`
# line is one behavioural assertion (JWT verify/claims, token-source JSON, protobuf wire bytes,
# data-track packet bytes, data-stream text reassembly). Anything that makes the probe exit early
# or skip its checks (e.g. a patch that exit(0)s) yields too few `ok` lines and fails here.
KAT_BIN=/mayhem/kat-probe
EXPECTED_MIN=15
[ -x "$KAT_BIN" ] || { echo "ERROR: $KAT_BIN missing; mayhem/build.sh should have built it" >&2; emit_ctrf "client-sdk-rust-kat" 0 1 0; exit 1; }

out="$("$KAT_BIN" 2>&1)"; rc=$?
printf '%s\n' "$out"
passed=$(printf '%s\n' "$out" | grep -c '^KAT .* ok$' || true)
failed=$(printf '%s\n' "$out" | grep -c '^KAT .* FAIL$' || true)
summary_ok=0
printf '%s\n' "$out" | grep -q "^KAT-SUMMARY passed=$passed failed=0$" && summary_ok=1

if [ "$rc" -ne 0 ] || [ "$summary_ok" -ne 1 ] || [ "$passed" -lt "$EXPECTED_MIN" ]; then
  [ "$failed" -gt 0 ] || failed=1
fi
emit_ctrf "client-sdk-rust-kat" "$passed" "$failed" 0
