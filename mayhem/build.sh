#!/usr/bin/env bash
#
# mayhem/build.sh — build the client-sdk-rust cargo-fuzz targets as ASan libFuzzer binaries
# (OSS-Fuzz Rust path), plus a dynamically linked known-answer probe for mayhem/test.sh.
#
# Runs inside the commit image (mayhem/Dockerfile) as `mayhem` in /mayhem. The toolchain and the
# cargo registry live under $CARGO_HOME=/opt/toolchains/rust/cargo; the air-gapped PATCH re-run
# resolves crates from that cache (the runtime exports CARGO_NET_OFFLINE=true), so no `--offline`.
#
# Only the pure-Rust parser crates are fuzzed (livekit-protocol, -token, -token-source,
# -data-stream, -datatrack); libwebrtc / webrtc-sys / livekit-ffi are deliberately not built.
set -euo pipefail

# clang rejects SOURCE_DATE_EPOCH='' — must be unset or a valid integer.
[ -n "${SOURCE_DATE_EPOCH:-}" ] || unset SOURCE_DATE_EPOCH

: "${MAYHEM_JOBS:=$(nproc)}"
export CARGO_BUILD_JOBS="$MAYHEM_JOBS"

cd "$SRC"

# Upstream's rust-toolchain.toml pins stable, which would hijack a bare `cargo`; always name ours.
TC="${RUSTUP_TOOLCHAIN:-nightly-2026-10-20}"
export RUSTUP_TOOLCHAIN="$TC"

FUZZ_DIR="mayhem/fuzz"
TRIPLE="x86_64-unknown-linux-gnu"
FUZZ_ABS="$SRC/$FUZZ_DIR"

# DWARF <= 3 on every Rust CU we compile (SPEC §6.2 item 10).
: "${RUST_DEBUG_FLAGS:=-Cdebuginfo=2 -Zdwarf-version=3}"

# The precompiled std and libFuzzer objects carry DWARF 4/5, and the gate reads the FIRST compile
# unit. lsan_off.c (LeakSanitizer off, ASan stays on) is built at DWARF 3 and linked ahead of
# everything else through a linker wrapper, so it is both the LSan hook and the DWARF-3 anchor.
# $SANITIZER_FLAGS are clang flags that rustc ignores (the Rust build is sanitized via RUSTFLAGS
# below); the C object is deliberately built without them.
ANCHOR_DIR="$FUZZ_ABS/target/dwarf3-anchor"
mkdir -p "$ANCHOR_DIR"
clang -g -gdwarf-3 ${SANITIZER_FLAGS:+-fno-sanitize=all} -c mayhem/lsan_off.c -o "$ANCHOR_DIR/lsan_off.o"
cat > "$ANCHOR_DIR/cc-anchor.sh" <<EOF
#!/bin/sh
exec cc "$ANCHOR_DIR/lsan_off.o" "\$@"
EOF
chmod +x "$ANCHOR_DIR/cc-anchor.sh"

# libFuzzer + ASan flags as OSS-Fuzz sets them; cargo-fuzz appends its own sancov flags.
export RUSTFLAGS="${RUSTFLAGS:-} --cfg fuzzing -Zsanitizer=address -Cforce-frame-pointers $RUST_DEBUG_FLAGS -Clinker=$ANCHOR_DIR/cc-anchor.sh"

FUZZ_TARGETS=()
for f in "$FUZZ_DIR"/fuzz_targets/*.rs; do
  FUZZ_TARGETS+=("$(basename "${f%.*}")")
done
[ "${#FUZZ_TARGETS[@]}" -gt 0 ] || { echo "ERROR: no fuzz targets under $FUZZ_DIR/fuzz_targets/" >&2; exit 1; }

echo "=== cargo fuzz build ($TC, ASan) ==="
echo "RUSTFLAGS=$RUSTFLAGS"
echo "targets: ${FUZZ_TARGETS[*]}"

for t in "${FUZZ_TARGETS[@]}"; do
  echo "--- building fuzz target: $t ---"
  cargo fuzz build --fuzz-dir "$FUZZ_DIR" -O --debug-assertions "$t"
  bin="$FUZZ_ABS/target/$TRIPLE/release/$t"
  [ -x "$bin" ] || { echo "ERROR: expected fuzz binary not found at $bin" >&2; exit 1; }
  cp "$bin" "/mayhem/$t"
  echo "built /mayhem/$t"
done

# The known-answer probe for mayhem/test.sh: the project's NORMAL flags (no sanitizer, no fuzz cfg),
# a separate target dir so it never mixes with the instrumented build.
echo "=== known-answer probe (normal flags) ==="
RUSTFLAGS="" CARGO_ENCODED_RUSTFLAGS="" CARGO_TARGET_DIR="$FUZZ_ABS/target/kat" \
  cargo build --release --manifest-path "$FUZZ_DIR/Cargo.toml" --bin kat
cp "$FUZZ_ABS/target/kat/release/kat" /mayhem/kat-probe
if readelf -l /mayhem/kat-probe | grep -q INTERP; then
  echo "kat-probe is dynamically linked"
else
  echo "ERROR: /mayhem/kat-probe must be dynamically linked (the sabotage check LD_PRELOADs it)" >&2
  exit 1
fi

echo "build.sh complete"
