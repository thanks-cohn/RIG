# RIG

## Explicit Allocation for Rust

RIG is a Rust library inspired by Zig's allocation philosophy.

Rust gives us safety.

Zig gives us allocator visibility.

RIG exists to bring allocator visibility and memory awareness into everyday Rust development.

The goal is simple:

> Memory should not be mysterious.

When reading code, a developer should be able to quickly answer:

```text
Where did this memory come from?

Who owns it?

How long does it live?

Which allocator created it?

What grows over time?

What is consuming resources?
```

---

## Why RIG?

Rust is one of the safest systems programming languages ever created.

However, many allocations can become invisible as projects grow.

A simple:

```rust
let users = Vec::new();
```

does not immediately communicate:

* where memory comes from
* which allocator is responsible
* what lifetime strategy is intended
* whether growth is expected

Zig encourages developers to think about allocation explicitly.

RIG brings that mindset into Rust.

---

## Philosophy



RIG is a development philosophy expressed through a Rust library.

The philosophy is:

```text
Visibility over mystery.

Explicitness over assumption.

Understanding over guessing.
```

---

## Core Principles

### Allocators Matter

Allocation is one of the most important events in a system.

RIG makes allocation visible.

---

### Rust First

RIG embraces:

* rustc
* Cargo
* Rust ownership
* Rust borrowing
* Rust safety guarantees



---

### Zig-Inspired Design

RIG takes inspiration from Zig's explicit allocator culture.

Memory should be visible in code.

Allocation should feel intentional.

---

### Minimal Magic

No hidden runtime.

No garbage collector.

No framework lock-in.

No unnecessary abstraction.

---

## Example

Without RIG:

```rust
let users = Vec::new();
```

With RIG:

```rust
let mut arena = Arena::new("main");
let mut users = RigVec::new(&mut arena, "users");
```

Now the allocation strategy becomes visible.

The programmer can immediately see:

```text
Allocator: main
Container: users
Lifetime strategy: arena-owned
```

---

## Machine-readable reports and diffs

RIG v0.6.0 has real machine-readable reports, evidence comparison, and optional evidence persistence through the Rust ecosystem rather than homemade serialization.

```rust
let snapshot = arena.snapshot();
let json = arena.report_json();
let before = arena.snapshot();
// mutate tracked containers
let after = arena.snapshot();
let diff = before.diff(&after);
let diff_json = diff.diff_json();
```

`arena.snapshot()` returns an `ArenaReport` containing the arena name, tracked container count, aggregate totals, and a list of per-container reports. `arena.report_json()` pretty-prints that snapshot with real crates.io `serde` and `serde_json`. `ArenaReport::diff(&after)` returns an `ArenaDiff` that reports containers added, containers removed, aggregate deltas, and per-container `ContainerDiff` entries for every container present in both reports. `diff.diff_json()` pretty-prints the diff through `serde_json`.

Small JSON output example:

```json
{
  "arena_name": "main",
  "tracked_container_count": 1,
  "totals": {
    "total_len": 2,
    "total_current_capacity": 4,
    "total_growth_events": 0,
    "total_pushed_appended_operations": 2
  },
  "containers": [
    {
      "name": "users",
      "kind": "RigVec",
      "len": 2,
      "initial_capacity": 4,
      "current_capacity": 4,
      "growth_events": 0,
      "operation_label": "total pushed items",
      "total_operations": 2,
      "extra_metric_label": null,
      "extra_metric_value": null
    }
  ]
}
```


## Evidence comparison

RIG v0.6.0 can explain change between two snapshots. Take report A, mutate tracked containers, take report B, and diff them.

```rust
let before = arena.snapshot();
users.push(9);
let after = arena.snapshot();
let diff = before.diff(&after);

println!("{}", diff.report());
println!("{}", diff.diff_json());
```

Readable diff output highlights the evidence a developer needs:

```text
RIG allocation diff
Before: main
After: main
Changed containers:
  users
    len: +4
    capacity: +8
    growth events: +1
    operations: +4
```

---

## Optional evidence persistence

RIG does not write files automatically. Persistence is 100% opt-in: default reports stay in memory, and RIG does not create `.rig/`, logs, mystery directories, or background files.

`Arena::write_json(path)` writes the current pretty JSON report only when the programmer explicitly calls it. `Arena::load_report(path)` loads a persisted report back into an `ArenaReport`, so allocation/growth evidence can survive the process for later inspection.

---

## Path to v1

RIG v0.6.0 is public API hardening for the path to a real v1. It does not add a CLI, macros, async work, background services, automatic persistence, or hidden project files. The point of this release is to make the API shape intentional, documented, and resistant to misuse.

A real v1 requires stable public API shape, useful rustdoc for exported types and methods, compiling doc tests for normal workflows, and abuse tests that prove RIG stays explicit under pressure. RIG still avoids hidden behavior: reports, snapshots, JSON rendering, and diffs remain in memory unless the programmer explicitly chooses a `write_json` path.

---

## Initial Goals

### v0

* Arena
* RigVec
* RigString
* Allocation tracking
* Allocation reporting
* Examples
* Tests

### v1

* Multiple allocator strategies
* Allocation statistics
* Growth tracking
* Memory reports
* Better diagnostics

### v2

* Allocation auditing
* Leak detection helpers
* Resource visualization
* Project-wide memory reporting

---


RIG is not:

* a programming language
* a garbage collector
* a framework

---

## Vision

RIG exists because memory is too important to remain invisible.

The future of systems programming is not merely safety.

The future is safety with understanding.

Rust already protects memory.

RIG helps developers see it.
