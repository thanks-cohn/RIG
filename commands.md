# RIG command guide

Run commands from the repository root unless a section explicitly says otherwise. The Rust crate lives at `rig/Cargo.toml`; the repository root intentionally has no `Cargo.toml`, so release validation uses `--manifest-path rig/Cargo.toml` or changes into `rig/` before running crate-local Cargo commands.

## Golden validation command

Use this release-grade validation chain when you need one command that checks formatting, lints, tests, doctests, and a release build.

```bash
cd ~/dev/RIG && \
cargo fmt --manifest-path rig/Cargo.toml --check && \
cd rig && \
cargo clippy --all-targets --all-features -- -D warnings && \
cargo test --all-targets --all-features && \
cargo test --doc --all-features && \
cargo build --release --all-targets --all-features
```

## Tail-first validation

RIG validation logs can be long, so start by inspecting the final summary at the bottom before reading the entire log.

```bash
bash scripts/validate.sh
```

Use this quick tail when you want the bottom of every validation log currently in `logs/`.

```bash
tail -150 logs/validation-*.log
```

Use this focused tail when you want the final 500 lines from only the newest validation log.

```bash
tail -500 "$(ls -t logs/validation-*.log | head -1)"
```

## Do not trust green without evidence

A `PASS` label means nothing unless it is backed by command output, the release audit must match the current `HEAD`, examples must actually run, tests must inspect real behavior, and you must not tag if validation says `FAIL` or the audit commit is stale.

Verify the audit commit matches the current `HEAD` before trusting release readiness.

```bash
test "$(sed -n 's/^- Commit: //p' release-audits/latest.md | head -1)" = "$(git rev-parse --short HEAD)" && echo "audit matches HEAD" || { echo "audit commit is stale"; exit 1; }
```

Verify the newest validation log has an end-of-run evidence summary before trusting any pass/fail label.

```bash
grep -n "END-OF-RUN EVIDENCE SUMMARY\|Release readiness:" "$(ls -t logs/validation-*.log | head -1)"
```

Verify examples were executed in the newest validation log instead of assuming the examples compiled only indirectly.

```bash
grep -n "^Example:" "$(ls -t logs/validation-*.log | head -1)"
```

## Repository state

Show the current branch, staged files, unstaged files, and untracked files.

```bash
git status --short --branch
```

Show the current branch name only.

```bash
git branch --show-current
```

Fetch remote branch and tag metadata from `origin` without changing the working tree.

```bash
git fetch origin --tags
```

Verify that local `HEAD` is exactly the same commit as `origin/main`.

```bash
test "$(git rev-parse HEAD)" = "$(git rev-parse origin/main)" && echo "HEAD equals origin/main" || { echo "HEAD differs from origin/main"; git --no-pager log --oneline --decorate --left-right HEAD...origin/main; exit 1; }
```

Show the latest tags by creator date.

```bash
git tag --sort=-creatordate | head -10
```

Show the latest tag name only.

```bash
git describe --tags --abbrev=0
```

Show the latest annotated tag message.

```bash
git tag -n99 "$(git describe --tags --abbrev=0)"
```

Show commits made since the latest tag.

```bash
git --no-pager log --oneline "$(git describe --tags --abbrev=0)..HEAD"
```

Show files changed since the latest tag.

```bash
git --no-pager diff --name-status "$(git describe --tags --abbrev=0)..HEAD"
```

## Version checks

Show the crate version declared in `rig/Cargo.toml`.

```bash
sed -n 's/^version = "\(.*\)"/\1/p' rig/Cargo.toml | head -1
```

Show the lockfile package version for the local `rig` package.

```bash
awk '/^name = "rig"$/ { in_rig = 1; next } in_rig && /^version = / { gsub(/"/, "", $3); print $3; exit }' rig/Cargo.lock
```

Compare the current crate version with the latest tag.

```bash
printf 'crate=%s\nlatest_tag=%s\n' "$(sed -n 's/^version = "\(.*\)"/\1/p' rig/Cargo.toml | head -1)" "$(git describe --tags --abbrev=0 | sed 's/^v//')"
```

Verify the crate version and lockfile version agree.

```bash
test "$(sed -n 's/^version = "\(.*\)"/\1/p' rig/Cargo.toml | head -1)" = "$(awk '/^name = "rig"$/ { in_rig = 1; next } in_rig && /^version = / { gsub(/"/, "", $3); print $3; exit }' rig/Cargo.lock)" && echo "Cargo.toml and Cargo.lock versions match" || { echo "version mismatch"; exit 1; }
```

## Validation

Check Rust formatting for the crate from the repository root.

```bash
cargo fmt --manifest-path rig/Cargo.toml --check
```

Run Clippy on all targets and all features while denying warnings; this command must be run from `rig/`.

```bash
cd rig && cargo clippy --all-targets --all-features -- -D warnings
```

Run all target tests with all features; this command must be run from `rig/`.

```bash
cd rig && cargo test --all-targets --all-features
```

Run documentation tests with all features; this command must be run from `rig/`.

```bash
cd rig && cargo test --doc --all-features
```

Build all targets with all features in release mode; this command must be run from `rig/`.

```bash
cd rig && cargo build --release --all-targets --all-features
```

Run the full repository validation script, which writes a log and updates `release-audits/latest.md`.

```bash
bash scripts/validate.sh
```

Run the evidence-capture tail validation script, which focuses on evidence-capture tests and examples.

```bash
bash scripts/validate-tail.sh
```

Inspect the last 100 lines from the newest full validation log.

```bash
tail -100 "$(ls -t logs/validation-*.log | head -1)"
```

Inspect the last 200 lines from the newest full validation log.

```bash
tail -200 "$(ls -t logs/validation-*.log | head -1)"
```

Inspect the last 500 lines from the newest full validation log.

```bash
tail -500 "$(ls -t logs/validation-*.log | head -1)"
```

Run the validation script summary self-test without running the full crate validation.

```bash
bash scripts/validate.sh --self-test-summary
```

## Examples

Run the demo example.

```bash
cargo run --manifest-path rig/Cargo.toml --example demo --all-features
```

Run the ECS simulation example.

```bash
cargo run --manifest-path rig/Cargo.toml --example ecs_simulation --all-features
```

Run the log ingestion example.

```bash
cargo run --manifest-path rig/Cargo.toml --example log_ingestion --all-features
```

Run the pathfinding example.

```bash
cargo run --manifest-path rig/Cargo.toml --example pathfinding --all-features
```

Run the policy comparison example.

```bash
cargo run --manifest-path rig/Cargo.toml --example policy_comparison --all-features
```

Run the allocation attribution example.

```bash
cargo run --manifest-path rig/Cargo.toml --example allocation_attribution --all-features
```

Run the regression gate example.

```bash
cargo run --manifest-path rig/Cargo.toml --example regression_gate --all-features
```

Run the memory budget example.

```bash
cargo run --manifest-path rig/Cargo.toml --example memory_budget --all-features
```

Run the artifact comparison example.

```bash
cargo run --manifest-path rig/Cargo.toml --example artifact_compare --all-features
```

Run the evidence exports example.

```bash
cargo run --manifest-path rig/Cargo.toml --example evidence_exports --all-features
```

Run the evidence profiles example.

```bash
cargo run --manifest-path rig/Cargo.toml --example evidence_profiles --all-features
```

Run the workload contract example.

```bash
cargo run --manifest-path rig/Cargo.toml --example workload_contract --all-features
```

Run the evidence certificate example.

```bash
cargo run --manifest-path rig/Cargo.toml --example evidence_certificate --all-features
```

Run the memory doctrine example.

```bash
cargo run --manifest-path rig/Cargo.toml --example memory_doctrine --all-features
```

Run the checkpoint capture example.

```bash
cargo run --manifest-path rig/Cargo.toml --example checkpoint_capture --all-features
```

Run the workload capture example.

```bash
cargo run --manifest-path rig/Cargo.toml --example workload_capture --all-features
```

Run the artifact lifecycle example.

```bash
cargo run --manifest-path rig/Cargo.toml --example artifact_lifecycle --all-features
```

Run every checked-in example once with all features.

```bash
for example in rig/examples/*.rs; do cargo run --manifest-path rig/Cargo.toml --example "$(basename "$example" .rs)" --all-features || exit 1; done
```

## Focused tests

Run the basic integration test target.

```bash
cargo test --manifest-path rig/Cargo.toml --test basic --all-features
```

Run the policy integration test target.

```bash
cargo test --manifest-path rig/Cargo.toml --test policy --all-features
```

Run the summary integration test target.

```bash
cargo test --manifest-path rig/Cargo.toml --test summary --all-features
```

Run the allocation attribution integration test target.

```bash
cargo test --manifest-path rig/Cargo.toml --test allocation_attribution --all-features
```

Run the budget integration test target.

```bash
cargo test --manifest-path rig/Cargo.toml --test budget --all-features
```

Run the regression integration test target.

```bash
cargo test --manifest-path rig/Cargo.toml --test regression --all-features
```

Run the artifact integration test target.

```bash
cargo test --manifest-path rig/Cargo.toml --test artifact --all-features
```

Run the export integration test target.

```bash
cargo test --manifest-path rig/Cargo.toml --test export --all-features
```

Run the profile integration test target.

```bash
cargo test --manifest-path rig/Cargo.toml --test profile --all-features
```

Run the contract integration test target.

```bash
cargo test --manifest-path rig/Cargo.toml --test contract --all-features
```

Run the abuse integration test target.

```bash
cargo test --manifest-path rig/Cargo.toml --test abuse --all-features
```

Run the API integration test target.

```bash
cargo test --manifest-path rig/Cargo.toml --test api --all-features
```

Run the certificate integration test target.

```bash
cargo test --manifest-path rig/Cargo.toml --test certificate --all-features
```

Run the memory doctrine integration test target.

```bash
cargo test --manifest-path rig/Cargo.toml --test memory_doctrine --all-features
```

Run the evidence capture integration test target.

```bash
cargo test --manifest-path rig/Cargo.toml --test evidence_capture --all-features
```

Run the policy comparison integration test target.

```bash
cargo test --manifest-path rig/Cargo.toml --test policy_comparison --all-features
```

Run the validation runner integration test target.

```bash
cargo test --manifest-path rig/Cargo.toml --test validation_runner --all-features
```

Run the workload examples integration test target.

```bash
cargo test --manifest-path rig/Cargo.toml --test workload_examples --all-features
```

## Evidence Capture

Run the checkpoint capture example and tee output for artifact path inspection.

```bash
cargo run --manifest-path rig/Cargo.toml --example checkpoint_capture --all-features | tee /tmp/rig-checkpoint-capture.out
```

Run the workload capture example and tee output for artifact path inspection.

```bash
cargo run --manifest-path rig/Cargo.toml --example workload_capture --all-features | tee /tmp/rig-workload-capture.out
```

Run the artifact lifecycle example and tee output for artifact path inspection.

```bash
cargo run --manifest-path rig/Cargo.toml --example artifact_lifecycle --all-features | tee /tmp/rig-artifact-lifecycle.out
```

Run the evidence capture test target.

```bash
cargo test --manifest-path rig/Cargo.toml --test evidence_capture --all-features -- --nocapture
```

Inspect generated temporary artifacts when example output prints paths under `/tmp`.

```bash
awk '{ for (i = 1; i <= NF; i++) if ($i ~ /^\/tmp\//) print $i }' /tmp/rig-checkpoint-capture.out /tmp/rig-workload-capture.out /tmp/rig-artifact-lifecycle.out 2>/dev/null | sed 's/[.,;:]$//' | while read -r path; do test -e "$path" && find "$path" -maxdepth 2 -print; done
```

Verify evidence-capture examples do not create hidden files in the repository working tree.

```bash
before="$(mktemp)"; after="$(mktemp)"; find . -path ./.git -prune -o -name '.*' -print | sort > "$before"; cargo run --manifest-path rig/Cargo.toml --example checkpoint_capture --all-features >/tmp/rig-hidden-check.out && cargo run --manifest-path rig/Cargo.toml --example workload_capture --all-features >>/tmp/rig-hidden-check.out && cargo run --manifest-path rig/Cargo.toml --example artifact_lifecycle --all-features >>/tmp/rig-hidden-check.out; find . -path ./.git -prune -o -name '.*' -print | sort > "$after"; diff -u "$before" "$after"
```

Run the focused tail validation workflow for evidence capture.

```bash
bash scripts/validate-tail.sh
```

## Release audit

Read the current release audit.

```bash
cat release-audits/latest.md
```

Inspect the audit status fields.

```bash
sed -n '/^- Version:/p;/^- Commit:/p;/^- Validation Date:/p;/^- Ready To Tag:/p;/| Clippy |/p;/| Tests |/p;/| Doc Tests |/p;/| Examples |/p;/| Release Build |/p' release-audits/latest.md
```

Update the release audit through validation rather than by manual editing, unless you are only correcting metadata.

```bash
bash scripts/validate.sh
```

Compare the audit commit to the current `HEAD`.

```bash
printf 'audit=%s\nhead=%s\n' "$(sed -n 's/^- Commit: //p' release-audits/latest.md | head -1)" "$(git rev-parse --short HEAD)"
```

Fail if the audit commit is stale relative to the current `HEAD`.

```bash
test "$(sed -n 's/^- Commit: //p' release-audits/latest.md | head -1)" = "$(git rev-parse --short HEAD)"
```

## Tagging

Show the latest tag.

```bash
git describe --tags --abbrev=0
```

Compare `HEAD` to the latest tag before deciding whether tagging is appropriate.

```bash
git --no-pager diff --stat "$(git describe --tags --abbrev=0)..HEAD"
```

Create an annotated tag for the current crate version after validation and audit checks pass.

```bash
git tag -a "v$(sed -n 's/^version = "\(.*\)"/\1/p' rig/Cargo.toml | head -1)" -m "RIG v$(sed -n 's/^version = "\(.*\)"/\1/p' rig/Cargo.toml | head -1)"
```

Inspect the latest tag message.

```bash
git tag -n99 "$(git describe --tags --abbrev=0)"
```

Push a tag only after confirming it is correct locally.

```bash
git push origin "$(git describe --tags --abbrev=0)"
```

Delete a local tag only if the local tag was created in error and has not been used for release.

```bash
git tag -d "v$(sed -n 's/^version = "\(.*\)"/\1/p' rig/Cargo.toml | head -1)"
```

Delete a remote tag only if the pushed tag is known to be wrong and the release owner has explicitly approved deletion.

```bash
git push origin ":refs/tags/v$(sed -n 's/^version = "\(.*\)"/\1/p' rig/Cargo.toml | head -1)"
```

## Logs

List the newest validation logs first.

```bash
ls -lt logs/validation-*.log logs/validation-tail-*.log 2>/dev/null
```

Tail the newest validation log.

```bash
tail -150 "$(ls -t logs/validation-*.log logs/validation-tail-*.log 2>/dev/null | head -1)"
```

Search the newest validation log for failures, errors, and warnings.

```bash
grep -Ein "fail|failed|failure|error|warning|panicked" "$(ls -t logs/validation-*.log logs/validation-tail-*.log 2>/dev/null | head -1)"
```

Summarize Rust test result lines in the newest validation log.

```bash
grep -n "test result:" "$(ls -t logs/validation-*.log | head -1)"
```

Search the newest validation log for release readiness.

```bash
grep -n "Release readiness:" "$(ls -t logs/validation-*.log | head -1)"
```

Search the newest validation log for the final evidence summary.

```bash
grep -n "END-OF-RUN EVIDENCE SUMMARY\|Success Digest\|Failure Digest" "$(ls -t logs/validation-*.log | head -1)"
```

## Cleanup

Remove generated Cargo build artifacts for the RIG crate.

```bash
cargo clean --manifest-path rig/Cargo.toml
```

Remove untracked local validation logs only after checking that they are not needed.

```bash
git status --short logs && git ls-files --others --exclude-standard logs '*.log' -z | xargs -0 rm -f
```

Show what `git clean -fd` would delete without deleting anything.

```bash
git clean -fd --dry-run
```

Show what `git clean -fdx` would delete without deleting anything, including ignored files.

```bash
git clean -fdx --dry-run
```

Use `git clean -fdx` only with extreme care because it deletes untracked and ignored files such as build outputs, local logs, scratch artifacts, and any other non-committed files.

```bash
git clean -fdx
```

## External / integration

Check whether an `integration/` project exists before attempting external integration commands.

```bash
test -d integration && find integration -maxdepth 3 -type f -print || echo "no integration project present"
```

Run an integration field test script if one is present and executable.

```bash
test -x integration/field-test.sh && integration/field-test.sh || echo "no executable integration/field-test.sh present"
```

Check whether a present integration project depends on the local RIG path.

```bash
test -d integration && rg "path *= *[\"']\.\./rig[\"']|path *= *[\"']../rig[\"']" integration || echo "no local RIG path dependency found or no integration project present"
```

Run external integration Clippy if an integration Cargo project is present.

```bash
test -f integration/Cargo.toml && cargo clippy --manifest-path integration/Cargo.toml --all-targets --all-features -- -D warnings || echo "no integration Cargo.toml present"
```

Run external integration tests if an integration Cargo project is present.

```bash
test -f integration/Cargo.toml && cargo test --manifest-path integration/Cargo.toml --all-targets --all-features || echo "no integration Cargo.toml present"
```

Run an external integration binary if an integration Cargo project is present and has a default binary target.

```bash
test -f integration/Cargo.toml && cargo run --manifest-path integration/Cargo.toml --all-features || echo "no integration Cargo.toml present"
```

## Safety

Check for a dirty working tree before destructive operations.

```bash
git diff --quiet && git diff --cached --quiet && test -z "$(git ls-files --others --exclude-standard)" && echo "working tree clean" || { echo "working tree is dirty"; git status --short; exit 1; }
```

Dry-run cleanup before deleting untracked files.

```bash
git clean -fd --dry-run
```

Dry-run ignored-file cleanup before deleting ignored files.

```bash
git clean -fdx --dry-run
```

Restore one tracked file from `HEAD` after confirming the change should be discarded.

```bash
git restore -- commands.md
```

Restore one staged file back to the worktree without discarding its content.

```bash
git restore --staged -- commands.md
```

Reset the current branch to `origin/main` only when explicitly intended and after confirming all local work can be lost.

```bash
git fetch origin && git reset --hard origin/main
```

Show the exact commits that would be lost before any hard reset to `origin/main`.

```bash
git --no-pager log --oneline --decorate origin/main..HEAD
```
