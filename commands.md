# RIG commands



```bash
# Check Rust formatting without rewriting files.
cargo fmt --check
```

```bash
# Run the crate test suite.
cargo test
```

```bash
# Run the allocation-visibility demo example.
cargo run --example demo
```

```bash
# Run Clippy and fail on warnings.
cargo clippy -- -D warnings
```

```bash
# One-shot validation from the crate directory: format check, tests, demo, and Clippy.
cd ~/dev/RIG/rig && cargo fmt --check && cargo test && cargo run --example demo && cargo clippy -- -D warnings
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
# From the repository root, run the demo example for the rig crate by manifest path.
cargo run --manifest-path rig/Cargo.toml --example demo
```

```bash
# From the repository root, run Clippy for the rig crate by manifest path and fail on warnings.
cargo clippy --manifest-path rig/Cargo.toml -- -D warnings
```

```bash
# Remove Cargo build artifacts when a clean rebuild is needed.
cargo clean
```
