# RIG commands

Run these commands from the repository root. The Rust crate lives at `rig/Cargo.toml`, so validation commands use `--manifest-path rig/Cargo.toml`.

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
git commit -m "release: RIG v0.9.0"
```

```bash
# Push the current branch to origin.
git push origin HEAD
```

```bash
# List known Git tags.
git tag --list
```

```bash
# Create an annotated release tag.
git tag -a v0.9.0 -m "RIG v0.9.0"
```

```bash
# Check Rust formatting without rewriting files.
cargo fmt --manifest-path rig/Cargo.toml --check
```

```bash
# Run the full crate test suite.
cargo test --manifest-path rig/Cargo.toml
```

```bash
# Run rustdoc examples as tests.
cargo test --manifest-path rig/Cargo.toml --doc
```

```bash
# Run growth-history-related tests by name fragment.
cargo test --manifest-path rig/Cargo.toml growth
```

```bash
# Run evidence comparison tests by name fragment.
cargo test --manifest-path rig/Cargo.toml diff
```

```bash
# Run workload example smoke tests by name fragment.
cargo test --manifest-path rig/Cargo.toml workload
```

```bash
# Run growth-policy tests by name fragment.
cargo test --manifest-path rig/Cargo.toml policy
```

```bash
# Run capped-capacity failure tests by name fragment.
cargo test --manifest-path rig/Cargo.toml capped
```

```bash
# Run tests and show captured stdout/stderr.
cargo test --manifest-path rig/Cargo.toml -- --nocapture
```

```bash
# Build local API documentation without opening a browser.
cargo doc --manifest-path rig/Cargo.toml --no-deps
```

```bash
# Build and open local API documentation for the rig crate.
cargo doc --manifest-path rig/Cargo.toml --no-deps --open
```

```bash
# Run the allocation-visibility demo example.
cargo run --manifest-path rig/Cargo.toml --example demo
```

```bash
# Run the ECS workload example.
cargo run --manifest-path rig/Cargo.toml --example ecs_simulation
```

```bash
# Run the log ingestion workload example.
cargo run --manifest-path rig/Cargo.toml --example log_ingestion
```

```bash
# Run the pathfinding workload example.
cargo run --manifest-path rig/Cargo.toml --example pathfinding
```

```bash
# Run growth-policy comparison workload.
cargo run --manifest-path rig/Cargo.toml --example policy_comparison
```

```bash
# Run growth-summary tests by name fragment.
cargo test --manifest-path rig/Cargo.toml summary
```

```bash
# Run verbose-report tests by name fragment.
cargo test --manifest-path rig/Cargo.toml verbose
```

```bash
# Run policy comparison tests by name fragment.
cargo test --manifest-path rig/Cargo.toml policy_comparison
```

```bash
# Run Clippy and fail on warnings.
cargo clippy --manifest-path rig/Cargo.toml -- -D warnings
```

```bash
# Show the crate dependency tree.
cargo tree --manifest-path rig/Cargo.toml
```

```bash
# One-shot full validation from the repository root.
cargo fmt --manifest-path rig/Cargo.toml --check && \
cargo test --manifest-path rig/Cargo.toml && \
cargo test --manifest-path rig/Cargo.toml --doc && \
cargo run --manifest-path rig/Cargo.toml --example demo && \
cargo run --manifest-path rig/Cargo.toml --example ecs_simulation && \
cargo run --manifest-path rig/Cargo.toml --example log_ingestion && \
cargo run --manifest-path rig/Cargo.toml --example pathfinding && \
cargo run --manifest-path rig/Cargo.toml --example policy_comparison && \
cargo test --manifest-path rig/Cargo.toml summary && \
cargo test --manifest-path rig/Cargo.toml verbose && \
cargo test --manifest-path rig/Cargo.toml policy_comparison && \
cargo clippy --manifest-path rig/Cargo.toml -- -D warnings && \
cargo tree --manifest-path rig/Cargo.toml
```

```bash
# Remove Cargo build artifacts when a clean rebuild is needed.
cargo clean --manifest-path rig/Cargo.toml
```

```bash
cargo run --example regression_gate
# Run memory regression gate example.
```

```bash
cargo test regression
# Run regression-gate tests.
```

```bash
cargo run --example memory_budget
```

# Run memory budget example.

```bash
cargo test budget
```

# Run memory-budget tests.

```bash
cargo run --example artifact_compare
```

# Run saved report artifact comparison example.

```bash
cargo test artifact
```

# Run report artifact tests.

