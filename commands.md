# RIG commands

## Repository navigation

```bash
cd ~/dev/RIG
# Move to the repository root.
```

```bash
cd ~/dev/RIG/rig
# Move to the Rust crate directory.
```

## Git workflow

```bash
git status
# Show changed files and the current branch state.
```

```bash
git pull
# Pull the latest commits for the current branch from its configured remote.
```

```bash
git add README.md commands.md rig/Cargo.toml rig/Cargo.lock rig/README.md rig/src/lib.rs rig/tests/basic.rs rig/examples/demo.rs vendor/serde vendor/serde_derive vendor/serde_json
# Stage the current v0.3.0 report-data changes.
```

```bash
git commit -m "Build RIG v0.3.0 machine-readable reports"
# Commit the staged v0.3.0 report-data changes.
```

```bash
git push
# Push committed branch changes to the configured remote branch.
```

## Tags

```bash
git tag
# List local Git tags.
```

```bash
git ls-remote --tags origin
# Show tags currently available on the GitHub remote.
```

```bash
git show v0.2.0
# Inspect the annotated v0.2.0 tag message and commit.
```

```bash
git tag -a v0.3.0 -m "message here"
# Create an annotated release tag with a human-readable release note.
```

```bash
git push origin v0.3.0
# Push the v0.3.0 tag to GitHub.
```

## Crate-level Cargo commands

```bash
cargo fmt --check
# Check Rust formatting without rewriting files.
```

```bash
cargo test
# Run the crate test suite.
```

```bash
cargo test -- --nocapture
# Run tests while allowing printed output to appear.
```

```bash
cargo run --example demo
# Run the allocation-visibility demo example, including human and JSON reports.
```

```bash
cargo clippy -- -D warnings
# Run Clippy and fail on warnings.
```

```bash
cargo clean
# Remove Cargo build artifacts when a clean rebuild is needed.
```

## Root-level Cargo commands

```bash
cargo fmt --manifest-path rig/Cargo.toml --check
# From the repository root, check formatting for the rig crate by manifest path.
```

```bash
cargo test --manifest-path rig/Cargo.toml
# From the repository root, run tests for the rig crate by manifest path.
```

```bash
cargo test --manifest-path rig/Cargo.toml -- --nocapture
# From the repository root, run tests by manifest path while allowing printed output to appear.
```

```bash
cargo run --manifest-path rig/Cargo.toml --example demo
# From the repository root, run the demo example for the rig crate by manifest path.
```

```bash
cargo clippy --manifest-path rig/Cargo.toml -- -D warnings
# From the repository root, run Clippy for the rig crate by manifest path and fail on warnings.
```

## Full validation

```bash
cd ~/dev/RIG/rig && cargo fmt --check && cargo test && cargo run --example demo && cargo clippy -- -D warnings
# One-shot validation from the crate directory: format check, tests, demo, and Clippy.
```

```bash
cd ~/dev/RIG && cargo test --manifest-path rig/Cargo.toml
# One-shot root-level test validation for the rig crate by manifest path.
```
