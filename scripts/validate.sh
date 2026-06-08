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
  local validation_date="$3"
  local duration_seconds="$4"
  local clippy_status="$5"
  local tests_status="$6"
  local doc_tests_status="$7"
  local examples_status="$8"
  local release_status="$9"
  local total_tests="${10}"
  local doc_tests="${11}"
  local examples="${12}"
  local failures="${13}"
  local warnings="${14}"
  local memory_contract_violations="${15}"
  local regression_violations="${16}"
  local budget_violations="${17}"
  local benchmark_failures="${18}"
  local certification_results="${19}"
  local artifacts="${20}"
  local ready="${21}"
  local digest="${22}"
  local success_digest="${23}"

  cat <<SUMMARY
==================================================
END-OF-RUN EVIDENCE SUMMARY
===========================

Version: $version
Commit: $commit
Validation timestamp: $validation_date
Validation duration: ${duration_seconds}s

Check status:
- Clippy status: $clippy_status
- Test status: $tests_status
- Documentation test status: $doc_tests_status
- Example status: $examples_status
- Release build status: $release_status

Execution counts:
- Total tests executed: $total_tests
- Documentation tests executed: $doc_tests
- Total examples executed: $examples
- Total failures: $failures
- Total warnings: $warnings

Memory evidence:
- Memory contract violations: $memory_contract_violations
- Regression violations: $regression_violations
- Budget violations: $budget_violations
- Benchmark failures: $benchmark_failures
- Evidence certification results: $certification_results
- Newly generated artifacts: $artifacts

Release readiness: $ready

SUMMARY

  if [[ "$ready" == "YES" ]]; then
    cat <<SUMMARY
Success Digest
--------------
$success_digest
SUMMARY
  else
    cat <<SUMMARY
Failure Digest
--------------
$digest
SUMMARY
  fi

  cat <<SUMMARY

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

build_failure_digest() {
  local file="$1"
  local clippy_status="$2"
  local tests_status="$3"
  local doc_tests_status="$4"
  local examples_status="$5"
  local release_status="$6"
  local digest=""

  if [[ "$clippy_status" != "PASS" ]]; then
    digest+=$'- Component: Clippy\n  Failure category: lint failure\n  Explanation: cargo clippy returned a non-zero exit status.\n  Location: see the Clippy section above.\n  Suggested next inspection point: search this log for "Clippy exit status" and preceding error lines.\n'
  fi
  if [[ "$tests_status" != "PASS" ]]; then
    digest+=$'- Component: Tests\n  Failure category: test failure or panic\n  Explanation: cargo test returned a non-zero exit status.\n  Location: see test failure names and panic locations above.\n  Suggested next inspection point: search this log for "FAILED", "panicked at", or "failures:".\n'
  fi
  if [[ "$doc_tests_status" != "PASS" ]]; then
    digest+=$'- Component: Documentation tests\n  Failure category: doctest failure\n  Explanation: cargo test --doc returned a non-zero exit status.\n  Location: see Doc-tests section above.\n  Suggested next inspection point: search this log for "Doc-tests".\n'
  fi
  if [[ "$examples_status" != "PASS" ]]; then
    digest+=$'- Component: Examples\n  Failure category: example failure\n  Explanation: at least one example returned a non-zero exit status.\n  Location: see Example sections above.\n  Suggested next inspection point: search this log for "Example:".\n'
  fi
  if [[ "$release_status" != "PASS" ]]; then
    digest+=$'- Component: Release build\n  Failure category: build failure\n  Explanation: cargo build --release returned a non-zero exit status.\n  Location: see Release Build section above.\n  Suggested next inspection point: search this log for "Release Build exit status".\n'
  fi

  local extracted
  extracted="$(grep -E '(^error(\[|:)|^[[:space:]]*error:|panicked at|FAILED|exceeded .*limit|delta .*exceeded allowed delta|missing artifact)' "$file" | tail -20 || true)"
  if [[ -n "$extracted" ]]; then
    digest+=$'- Component: Extracted log evidence\n  Failure category: captured failure lines\n  Explanation: important failure lines reproduced from detailed logs.\n  Location: latest matching lines in validation log.\n  Suggested next inspection point: inspect the original command section around each line.\n'
    digest+="$extracted"
  fi
  if [[ -z "$digest" ]]; then
    digest='- No failures were detected by validation status or failure-pattern extraction.'
  fi
  printf '%s\n' "$digest"
}

build_success_digest() {
  local file="$1"
  local total_tests="$2"
  local examples="$3"
  local artifacts="$4"
  local largest_growth highest_container
  largest_growth="$(grep -E 'largest_growth|largest growth|Largest growth|capacity_added' "$file" | tail -1 || true)"
  highest_container="$(grep -E 'current_capacity|total capacity|Total capacity' "$file" | tail -1 || true)"
  cat <<DIGEST
- Largest allocation workload: inspect benchmark/example reports above; highest observed container line: ${highest_container:-not emitted by examples}.
- Largest growth event: ${largest_growth:-not emitted by examples}.
- Highest-capacity container: ${highest_container:-not emitted by examples}.
- Benchmark summary: examples executed=$examples; tests executed=$total_tests.
- Evidence generated: $artifacts.
- Contracts passed: no validation command failed.
- Regressions detected: none in release validation status.
DIGEST
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
  local start_epoch
  start_epoch="$(date +%s)"

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

  local total_tests doc_tests errors warnings ready_exit memory_contract_violations regression_violations budget_violations benchmark_failures
  total_tests="$(extract_total_tests "$LOG_FILE")"
  doc_tests="$(extract_doc_tests "$LOG_FILE")"
  errors="$(count_matches '(^error(\[|:)|^[[:space:]]*error:|panicked at)' "$LOG_FILE")"
  warnings="$(count_matches '(^warning(\[|:)|^[[:space:]]*warning:)' "$LOG_FILE")"
  memory_contract_violations="$(count_matches '(memory doctrine|workload contract|Contract:).*FAILED|growth_profile_forbidden|growth_profile_required' "$LOG_FILE")"
  regression_violations="$(count_matches '(regression|Regression).*FAILED|delta .*exceeded allowed delta' "$LOG_FILE")"
  budget_violations="$(count_matches '(budget|Budget).*FAILED|exceeded .*limit' "$LOG_FILE")"
  benchmark_failures="$(count_matches '(benchmark|Benchmark).*FAILED' "$LOG_FILE")"

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

  local end_epoch duration_seconds artifacts certification_results failure_digest success_digest
  end_epoch="$(date +%s)"
  duration_seconds="$((end_epoch - start_epoch))"
  artifacts="$LOG_FILE, $AUDIT_FILE"
  certification_results="$(count_matches 'Evidence certificate fingerprint|RIG evidence certificate' "$LOG_FILE") observed"
  failure_digest="$(build_failure_digest "$LOG_FILE" "$clippy_status" "$tests_status" "$doc_tests_status" "$examples_status" "$release_status")"
  success_digest="$(build_success_digest "$LOG_FILE" "$total_tests" "${#examples[@]}" "$artifacts")"

  print_summary \
    "$version" \
    "$commit" \
    "$VALIDATION_DATE" \
    "$duration_seconds" \
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
    "$memory_contract_violations" \
    "$regression_violations" \
    "$budget_violations" \
    "$benchmark_failures" \
    "$certification_results" \
    "$artifacts" \
    "$ready" \
    "$failure_digest" \
    "$success_digest" | tee -a "$LOG_FILE"

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

  summary="$(print_summary 0.20.0 abc123 2026-06-07T00:00:00Z 5 PASS PASS PASS PASS PASS "$total_tests" "$doc_tests" 3 "$errors" "$warnings" 0 0 0 0 "0 observed" "$sample_log, $sample_audit" YES "- No failures" "- Contracts passed")"
  write_audit 0.19.0 abc123 2026-06-07T00:00:00Z PASS PASS PASS PASS PASS "$total_tests" "$doc_tests" 3 "$errors" "$warnings" YES "$sample_audit"

  [[ "$summary" == *"END-OF-RUN EVIDENCE SUMMARY"* ]] || return 1
  [[ "$summary" == *"Version: 0.20.0"* ]] || return 1
  [[ "$summary" == *"Total tests executed: 2"* ]] || return 1
  [[ "$summary" == *"Documentation tests executed: 1"* ]] || return 1
  [[ "$summary" == *"Total failures: 1"* ]] || return 1
  [[ "$summary" == *"Total warnings: 1"* ]] || return 1
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
