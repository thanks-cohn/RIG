#!/usr/bin/env bash
set -u
set -o pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CRATE_DIR="$ROOT_DIR"
if [[ ! -f "$CRATE_DIR/Cargo.toml" && -f "$ROOT_DIR/rig/Cargo.toml" ]]; then
  CRATE_DIR="$ROOT_DIR/rig"
fi

LOG_DIR="$ROOT_DIR/logs"
AUDIT_DIR="$ROOT_DIR/release-audits"
TIMESTAMP="${RIG_VALIDATION_TIMESTAMP:-$(date -u +%Y%m%dT%H%M%SZ)}"
VALIDATION_DATE="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
LOG_FILE="$LOG_DIR/validation-$TIMESTAMP.log"
AUDIT_FILE="$AUDIT_DIR/latest.md"

status_word() {
  if [[ "$1" -eq 0 ]]; then
    printf 'PASS'
  else
    printf 'FAIL'
  fi
}

ready_word() {
  if [[ "$1" -eq 0 ]]; then
    printf 'YES'
  else
    printf 'NO'
  fi
}

count_matches() {
  local pattern="$1"
  local file="$2"
  if [[ ! -f "$file" ]]; then
    printf '0'
    return
  fi
  grep -E "$pattern" "$file" | wc -l | tr -d ' '
}

extract_total_tests() {
  local file="$1"
  if [[ ! -f "$file" ]]; then
    printf '0'
    return
  fi
  awk '
    /^   Doc-tests / { in_doc = 1; next }
    /^test result:/ && !in_doc {
      for (i = 1; i <= NF; i++) {
        if ($i == "passed;") {
          passed = $(i - 1) + 0
        }
        if ($i == "failed;") {
          failed = $(i - 1) + 0
        }
        if ($i == "ignored;") {
          ignored = $(i - 1) + 0
        }
        if ($i == "measured;") {
          measured = $(i - 1) + 0
        }
      }
      total += passed + failed + ignored + measured
      passed = failed = ignored = measured = 0
    }
    in_doc && /^test result:/ { in_doc = 0 }
    END { print total + 0 }
  ' "$file"
}

extract_doc_tests() {
  local file="$1"
  if [[ ! -f "$file" ]]; then
    printf '0'
    return
  fi
  awk '
    /^   Doc-tests / { in_doc = 1; next }
    in_doc && /^test result:/ {
      for (i = 1; i <= NF; i++) {
        if ($i == "passed;") {
          passed = $(i - 1) + 0
        }
        if ($i == "failed;") {
          failed = $(i - 1) + 0
        }
        if ($i == "ignored;") {
          ignored = $(i - 1) + 0
        }
        if ($i == "measured;") {
          measured = $(i - 1) + 0
        }
      }
      total += passed + failed + ignored + measured
      passed = failed = ignored = measured = 0
      in_doc = 0
    }
    END { print total + 0 }
  ' "$file"
}

print_summary() {
  local version="$1"
  local commit="$2"
  local clippy_status="$3"
  local tests_status="$4"
  local doc_tests_status="$5"
  local examples_status="$6"
  local release_status="$7"
  local total_tests="$8"
  local doc_tests="$9"
  local errors="${10}"
  local warnings="${11}"
  local ready="${12}"

  cat <<SUMMARY
==================================================
RIG VALIDATION SUMMARY
======================

Version: $version

Commit: $commit

Clippy:
$clippy_status

Tests:
$tests_status

Doc Tests:
$doc_tests_status

Examples:
$examples_status

Release Build:
$release_status

Total Tests: $total_tests

Doc Tests: $doc_tests

Errors: $errors

Warnings: $warnings

Ready To Tag:
$ready

==================================================
SUMMARY
}

write_audit() {
  local version="$1"
  local commit="$2"
  local validation_date="$3"
  local clippy_status="$4"
  local tests_status="$5"
  local doc_tests_status="$6"
  local examples_status="$7"
  local release_status="$8"
  local total_tests="$9"
  local doc_tests="${10}"
  local example_count="${11}"
  local errors="${12}"
  local warnings="${13}"
  local ready="${14}"
  local audit_file="${15}"

  cat > "$audit_file" <<AUDIT
# RIG Release Audit

- Version: $version
- Commit: $commit
- Validation Date: $validation_date
- Ready To Tag: $ready

## Validation Results

| Check | Result |
| --- | --- |
| Clippy | $clippy_status |
| Tests | $tests_status |
| Doc Tests | $doc_tests_status |
| Examples | $examples_status |
| Release Build | $release_status |

## Counts

- Total Tests: $total_tests
- Doc Tests: $doc_tests
- Examples: $example_count
- Errors: $errors
- Warnings: $warnings
AUDIT
}

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

run_examples() {
  local -n _example_names_ref=$1
  local failed=0
  if [[ ${#_example_names_ref[@]} -eq 0 ]]; then
    printf '\nNo examples found.\n' | tee -a "$LOG_FILE"
    return 0
  fi

  for example in "${_example_names_ref[@]}"; do
    if ! run_logged "Example: $example" cargo run --example "$example" --all-features; then
      failed=1
    fi
  done
  return "$failed"
}

run_validation() {
  mkdir -p "$LOG_DIR" "$AUDIT_DIR"
  : > "$LOG_FILE"

  local version
  version="$(sed -n 's/^version = "\(.*\)"/\1/p' "$CRATE_DIR/Cargo.toml" | head -n 1)"
  local commit
  commit="$(git -C "$ROOT_DIR" rev-parse --short HEAD 2>/dev/null || printf 'unknown')"

  local examples=()
  if [[ -d "$CRATE_DIR/examples" ]]; then
    while IFS= read -r example_path; do
      examples+=("$(basename "$example_path" .rs)")
    done < <(find "$CRATE_DIR/examples" -maxdepth 1 -type f -name '*.rs' | sort)
  fi

  {
    printf 'RIG validation started at %s\n' "$VALIDATION_DATE"
    printf 'Version: %s\n' "$version"
    printf 'Commit: %s\n' "$commit"
    printf 'Crate directory: %s\n' "$CRATE_DIR"
  } | tee -a "$LOG_FILE"

  local fmt_exit=0 clippy_exit=0 tests_exit=0 doc_tests_exit=0 examples_exit=0 release_exit=0

  run_logged "Format Check" cargo fmt --check || fmt_exit=$?
  run_logged "Clippy" cargo clippy --all-targets --all-features -- -D warnings || clippy_exit=$?
  run_logged "Tests" cargo test --all-targets --all-features || tests_exit=$?
  run_logged "Doc Tests" cargo test --doc --all-features || doc_tests_exit=$?
  run_examples examples || examples_exit=$?
  run_logged "Release Build" cargo build --release --all-targets --all-features || release_exit=$?

  local total_tests doc_tests errors warnings ready_exit
  total_tests="$(extract_total_tests "$LOG_FILE")"
  doc_tests="$(extract_doc_tests "$LOG_FILE")"
  errors="$(count_matches '(^error(\[|:)|^[[:space:]]*error:|panicked at)' "$LOG_FILE")"
  warnings="$(count_matches '(^warning(\[|:)|^[[:space:]]*warning:)' "$LOG_FILE")"

  ready_exit=0
  for exit_code in "$fmt_exit" "$clippy_exit" "$tests_exit" "$doc_tests_exit" "$examples_exit" "$release_exit"; do
    if [[ "$exit_code" -ne 0 ]]; then
      ready_exit=1
    fi
  done

  local clippy_status tests_status doc_tests_status examples_status release_status ready
  clippy_status="$(status_word "$clippy_exit")"
  tests_status="$(status_word "$tests_exit")"
  doc_tests_status="$(status_word "$doc_tests_exit")"
  examples_status="$(status_word "$examples_exit")"
  release_status="$(status_word "$release_exit")"
  ready="$(ready_word "$ready_exit")"

  print_summary \
    "$version" \
    "$commit" \
    "$clippy_status" \
    "$tests_status" \
    "$doc_tests_status" \
    "$examples_status" \
    "$release_status" \
    "$total_tests" \
    "$doc_tests" \
    "$errors" \
    "$warnings" \
    "$ready" | tee -a "$LOG_FILE"

  write_audit \
    "$version" \
    "$commit" \
    "$VALIDATION_DATE" \
    "$clippy_status" \
    "$tests_status" \
    "$doc_tests_status" \
    "$examples_status" \
    "$release_status" \
    "$total_tests" \
    "$doc_tests" \
    "${#examples[@]}" \
    "$errors" \
    "$warnings" \
    "$ready" \
    "$AUDIT_FILE"

  printf 'Full validation log: %s\n' "$LOG_FILE" | tee -a "$LOG_FILE"
  printf 'Release audit: %s\n' "$AUDIT_FILE" | tee -a "$LOG_FILE"

  return "$ready_exit"
}

self_test_summary() {
  local tmp_dir
  tmp_dir="$(mktemp -d)"
  local sample_log="$tmp_dir/sample.log"
  local sample_audit="$tmp_dir/latest.md"
  cat > "$sample_log" <<'LOG'
running 2 tests
test alpha ... ok
test beta ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s

   Doc-tests rig

running 1 test
test src/lib.rs - demo (line 1) ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
warning: sample warning
error: sample error
LOG

  local total_tests doc_tests errors warnings summary
  total_tests="$(extract_total_tests "$sample_log")"
  doc_tests="$(extract_doc_tests "$sample_log")"
  errors="$(count_matches '(^error(\[|:)|^[[:space:]]*error:|panicked at)' "$sample_log")"
  warnings="$(count_matches '(^warning(\[|:)|^[[:space:]]*warning:)' "$sample_log")"

  summary="$(print_summary 0.19.0 abc123 PASS PASS PASS PASS PASS "$total_tests" "$doc_tests" "$errors" "$warnings" YES)"
  write_audit 0.19.0 abc123 2026-06-07T00:00:00Z PASS PASS PASS PASS PASS "$total_tests" "$doc_tests" 3 "$errors" "$warnings" YES "$sample_audit"

  [[ "$summary" == *"RIG VALIDATION SUMMARY"* ]] || return 1
  [[ "$summary" == *"Version: 0.19.0"* ]] || return 1
  [[ "$summary" == *"Total Tests: 2"* ]] || return 1
  [[ "$summary" == *"Doc Tests: 1"* ]] || return 1
  [[ "$summary" == *"Errors: 1"* ]] || return 1
  [[ "$summary" == *"Warnings: 1"* ]] || return 1
  grep -q 'Ready To Tag: YES' "$sample_audit" || return 1
  rm -rf "$tmp_dir"
}

case "${1:-}" in
  --self-test-summary)
    self_test_summary
    ;;
  --help|-h)
    cat <<HELP
Usage: scripts/validate.sh [--self-test-summary]

Runs RIG validation, writes logs/validation-<timestamp>.log, and writes release-audits/latest.md.
HELP
    ;;
  *)
    run_validation
    ;;
esac
