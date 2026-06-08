#!/usr/bin/env bash
set -u
set -o pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CRATE_DIR="$ROOT_DIR"
if [[ ! -f "$CRATE_DIR/Cargo.toml" && -f "$ROOT_DIR/rig/Cargo.toml" ]]; then
  CRATE_DIR="$ROOT_DIR/rig"
fi
LOG_DIR="$ROOT_DIR/logs"
TIMESTAMP="${RIG_VALIDATION_TIMESTAMP:-$(date -u +%Y%m%dT%H%M%SZ)}"
LOG_FILE="$LOG_DIR/validation-tail-$TIMESTAMP.log"
mkdir -p "$LOG_DIR"
: > "$LOG_FILE"

run_logged() {
  local label="$1"
  shift
  {
    printf '\n==================================================\n'
    printf '%s\n' "$label"
    printf 'Command:'
    printf ' %q' "$@"
    printf '\n==================================================\n'
  } | tee -a "$LOG_FILE"

  (cd "$CRATE_DIR" && "$@") 2>&1 | tee -a "$LOG_FILE"
  local command_status=${PIPESTATUS[0]}
  printf '%s exit status: %s\n' "$label" "$command_status" | tee -a "$LOG_FILE"
  return "$command_status"
}

status_word() {
  if [[ "$1" -eq 0 ]]; then
    printf 'PASS'
  else
    printf 'FAIL'
  fi
}

main() {
  local start_epoch end_epoch duration
  start_epoch="$(date +%s)"
  local examples=(checkpoint_capture workload_capture artifact_lifecycle)
  local tests_exit=0 examples_exit=0

  run_logged "Evidence Capture Tests" cargo test --test evidence_capture --all-features || tests_exit=$?
  for example in "${examples[@]}"; do
    run_logged "Evidence Capture Example: $example" cargo run --example "$example" --all-features || examples_exit=1
  done

  end_epoch="$(date +%s)"
  duration="$((end_epoch - start_epoch))"
  local ready_exit=0
  if [[ "$tests_exit" -ne 0 || "$examples_exit" -ne 0 ]]; then
    ready_exit=1
  fi

  {
    printf '\n==================================================\n'
    printf 'END-OF-RUN EVIDENCE CAPTURE TAIL SUMMARY\n'
    printf '========================================\n\n'
    printf 'Validation duration: %ss\n' "$duration"
    printf 'Evidence capture tests: %s\n' "$(status_word "$tests_exit")"
    printf 'Evidence capture examples: %s\n' "$(status_word "$examples_exit")"
    printf 'Examples executed:\n'
    for example in "${examples[@]}"; do
      printf -- '- %s\n' "$example"
    done
    printf 'Generated log artifact: %s\n' "$LOG_FILE"
    printf 'Final status: %s\n' "$(status_word "$ready_exit")"
    printf '==================================================\n'
  } | tee -a "$LOG_FILE"

  return "$ready_exit"
}

main "$@"
