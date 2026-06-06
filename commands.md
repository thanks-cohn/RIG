# RIG commands

```bash
# Go to the repository root used by the main development checkout.
cd ~/dev/RIG
```

```bash
# Go directly to the Rust crate directory.
cd ~/dev/RIG/rig
```

```bash
# Show changed, staged, and untracked files.
git status
```

```bash
# Update the local main branch from the origin remote.
git pull origin main
```

```bash
# Stage all current file changes for commit.
git add .
```

```bash
# Commit staged changes with a release message.
git commit -m "release: RIG v0.3.0"
```

```bash
# Push the local main branch to origin.
git push origin main
```

```bash
# List known Git tags.
git tag --list
```

```bash
# Show the tagged source state for v0.2.0.
git show v0.2.0
```

```bash
# Create an annotated release tag.
git tag -a v0.3.0 -m "RIG v0.3.0"
```

```bash
# Check Rust formatting without rewriting files from the crate directory.
cargo fmt --check
```

```bash
# Run the crate test suite from the crate directory.
cargo test
```

```bash
# Run growth-history-related tests by name fragment.
cargo test growth
```

```bash
# Run rustdoc examples as tests.
cargo test --doc
```

```bash
# Build local API documentation without opening a browser.
cargo doc --no-deps
```

```bash
# Build and open local API documentation for the rig crate.
cargo doc --no-deps --open
```

```bash
# Run tests and show captured stdout/stderr from the crate directory.
cargo test -- --nocapture
```

```bash
# Run the allocation-visibility demo example from the crate directory.
cargo run --example demo
```

```bash
# Run Clippy from the crate directory and fail on warnings.
cargo clippy -- -D warnings
```

```bash
# Show the crate dependency tree.
# Useful for proving RIG uses real serde and serde_json from crates.io.
cargo tree
```

```bash
# One-shot full validation from the crate directory: format check, tests, demo, Clippy, and dependency tree.
cd ~/dev/RIG/rig && \
cargo fmt --check && \
cargo test && \
cargo test --doc && \
cargo run --example demo && \
cargo clippy -- -D warnings && \
cargo tree
```

```bash
# From the repository root, check formatting for the rig crate by manifest path.
cargo fmt --manifest-path rig/Cargo.toml --check
```

```bash
# From the repository root, run tests for the rig crate by manifest path.
cargo test --manifest-path rig/Cargo.toml
```

```bash
# Run rustdoc examples from the repo root.
cargo test --manifest-path rig/Cargo.toml --doc
```

```bash
# From the repository root, run tests with captured output for the rig crate by manifest path.
cargo test --manifest-path rig/Cargo.toml -- --nocapture
```

```bash
# From the repository root, run the demo example for the rig crate by manifest path.
cargo run --manifest-path rig/Cargo.toml --example demo
```

```bash
# From the repository root, run Clippy for the rig crate by manifest path and fail on warnings.
cargo clippy --manifest-path rig/Cargo.toml -- -D warnings
```

```bash
# Show the dependency tree from the repository root.
cargo tree --manifest-path rig/Cargo.toml
```

```bash
# Remove Cargo build artifacts when a clean rebuild is needed.
cargo clean
```

```bash
# Runs the demo, including explicit opt-in JSON report writing and loading.
cargo run --example demo
```

```bash
# Optional: inspect temporary RIG demo/test report files if any remain.
find /tmp -maxdepth 1 -name "rig-*.json" 2>/dev/null
```

```bash
# Useful when checking persistence tests and demo-like printed paths.
cargo test -- --nocapture
```

```bash
# One-shot full validation for RIG v0.7.0 evidence comparison from the repository root.
cargo fmt --manifest-path rig/Cargo.toml --check && \
cargo test --manifest-path rig/Cargo.toml && \
cargo test --manifest-path rig/Cargo.toml --doc && \
cargo run --manifest-path rig/Cargo.toml --example demo && \
cargo clippy --manifest-path rig/Cargo.toml -- -D warnings && \
cargo tree --manifest-path rig/Cargo.toml
```

```bash
# Run only evidence comparison tests by name fragment from the repository root.
cargo test --manifest-path rig/Cargo.toml diff
```

```bash
cargo run --example ecs_simulation
# Run the ECS workload example.
```

```bash
cargo run --example log_ingestion
# Run the log ingestion workload example.
```

```bash
cargo run --example pathfinding
# Run the pathfinding workload example.
```

```bash
cargo test workload
# Run workload example smoke tests by name fragment, if tests use that naming.
```
