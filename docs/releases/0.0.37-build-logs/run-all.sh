#!/usr/bin/env bash
# Run 0.0.37 Linux release-build targets and capture warning/error logs.
set -u
cd /workspace
LOGDIR=/workspace/docs/releases/0.0.37-build-logs
mkdir -p "$LOGDIR"
export CARGO_TERM_COLOR=never
export CARGO_INCREMENTAL=0
# Keep rustc diagnostics on stdout even when piped.
export RUSTFLAGS="${RUSTFLAGS:-} -D warnings" 2>/dev/null || true
# Do NOT fail closed on warnings for the first capture pass — we want the list.
unset RUSTFLAGS
export RUSTFLAGS=""

extract_diagnostics() {
  local full="$1"
  local diag="$2"
  python3 - "$full" "$diag" <<'PY'
import re, sys
from pathlib import Path
src, dst = Path(sys.argv[1]), Path(sys.argv[2])
text = src.read_text(encoding="utf-8", errors="replace")
# Cargo / rustc diagnostic lines plus wasm-pack / naga / linker failures.
pat = re.compile(
    r"^(error(\[[A-Z0-9]+\])?:|warning(\[[A-Z0-9]+\])?:|"
    r"error: |warning: |error\[|warning\[|"
    r".*error occurred.*|"
    r"Failed to|"
    r"Caused by:)",
    re.I | re.M,
)
# Also keep rustc rendered diagnostic headers of the form:
#   error[E0425]: ...
#   warning: unused ...
lines = []
for line in text.splitlines():
    s = line.strip()
    if not s:
        continue
    if re.match(r"^(error|warning)(\[|:\s)", s, re.I):
        lines.append(line)
        continue
    if s.startswith("error:") or s.startswith("warning:"):
        lines.append(line)
        continue
    if "aborting due to" in s.lower() or s.startswith("For more information about this error"):
        lines.append(line)

# Unique preserve-order
seen = set()
uniq = []
for line in lines:
    if line not in seen:
        seen.add(line)
        uniq.append(line)

err_n = sum(1 for l in uniq if re.match(r"^\s*error", l, re.I) or "aborting due to" in l.lower())
warn_n = sum(1 for l in uniq if re.match(r"^\s*warning", l, re.I))
body = "\n".join(uniq)
summary = f"errors={err_n} warnings={warn_n} unique_diagnostic_lines={len(uniq)}\n"
dst.write_text(summary + ("\n" + body + "\n" if body else "\n(no error/warning diagnostic lines captured)\n"), encoding="utf-8")
print(summary.strip())
PY
}

run_one() {
  local name="$1"
  shift
  local full="$LOGDIR/${name}.full.log"
  local diag="$LOGDIR/${name}.diagnostics.log"
  local status="$LOGDIR/${name}.status"
  echo "===== START ${name}  $(date -u +%FT%TZ) =====" | tee "$full"
  echo "CMD: $*" | tee -a "$full"
  local start=$SECONDS
  set +e
  "$@" >>"$full" 2>&1
  local rc=$?
  set -e
  local elapsed=$((SECONDS - start))
  echo "===== END ${name}  rc=${rc}  elapsed=${elapsed}s  $(date -u +%FT%TZ) =====" | tee -a "$full"
  echo "${rc}" > "$status"
  extract_diagnostics "$full" "$diag" | tee -a "$full"
  echo "${name} finished rc=${rc} elapsed=${elapsed}s"
  return 0
}

# 1) Native CLI release (release-cli.yml)
run_one "01-cli-linux" cargo build --release -p qualia-cli

# 2) WASM smoke-checks (release-wasm.yml check-wasm)
run_one "02-wasm-check-portal" cargo check --target wasm32-unknown-unknown -p qualia-core-db --no-default-features --features portal
run_one "03-wasm-check-lite" cargo check --target wasm32-unknown-unknown -p webizen-lite-wasm
run_one "04-wasm-check-logic" cargo check --target wasm32-unknown-unknown -p qualia-core-db --no-default-features --features wasm-logic
run_one "05-wasm-check-scientific" cargo check --target wasm32-unknown-unknown -p qualia-core-db --no-default-features --features wasm-scientific
run_one "06-wasm-check-llm" cargo check --target wasm32-unknown-unknown -p qualia-core-db --no-default-features --features wasm-llm

# 3) wasm-pack release artifacts
if command -v wasm-pack >/dev/null 2>&1; then
  run_one "07-wasm-pack-portal" env RUSTFLAGS="-C target-feature=+simd128" wasm-pack build crates/qualia-core-db --target web --release --out-dir pkg-qualia -- --no-default-features --features portal
  run_one "08-wasm-pack-lite" wasm-pack build crates/webizen-lite-wasm --target web --release --out-dir /workspace/docs/pkg/webizen-lite
  run_one "09-wasm-pack-playground" env RUSTFLAGS="-C target-feature=+simd128 -C link-arg=-zstack-size=8388608 -C link-arg=--max-memory=4294967296" wasm-pack build crates/qualia-core-db --target web --release --out-dir /workspace/docs/playground --out-name qualia_core_db --no-typescript -- --no-default-features --features wasm-full
else
  echo "wasm-pack missing" | tee "$LOGDIR/07-wasm-pack-portal.full.log"
  echo "1" > "$LOGDIR/07-wasm-pack-portal.status"
fi

# 4) P64 converter (release-p64-models.yml compile step)
run_one "10-p64-converter" cargo build --release -p qualia-core-db --example build_p64_release

# 5) Anatomy pack producer (compile only — CDN fetch is a separate release-time job)
run_one "11-anatomy-example" cargo build --release -p qualia-client-core --example build_anatomy_pack

# 6) Desktop native Linux binary (not the Windows NSIS / macOS Metal installer)
run_one "12-desktop-linux" cargo build --release -p webizen-desktop

# 7) Remaining workspace members that ship in some form
run_one "13-studio-lib" cargo build --release -p webizen-studio
run_one "14-poet-cli" cargo build --release -p poet-cli
run_one "15-vibe" cargo build --release -p vibe

python3 - "$LOGDIR" <<'PY'
from pathlib import Path
logdir = Path(__file__) if False else Path("/workspace/docs/releases/0.0.37-build-logs")
rows = []
for status in sorted(logdir.glob("*.status")):
    name = status.stem
    rc = status.read_text().strip()
    diag = logdir / f"{name}.diagnostics.log"
    summary = diag.read_text(encoding="utf-8", errors="replace").splitlines()[0] if diag.exists() else "missing diagnostics"
    rows.append(f"| `{name}` | {rc} | {summary} |")
(logdir / "SUMMARY.md").write_text(
    "# 0.0.37 release-build summary\n\n"
    "| Target | exit | diagnostics |\n| --- | ---: | --- |\n"
    + "\n".join(rows) + "\n",
    encoding="utf-8",
)
print("wrote SUMMARY.md")
for r in rows:
    print(r)
PY
