# RIG

RIG is a Rust crate that brings Zig-style allocator visibility to everyday Rust code.

Rust keeps doing the safety work: ownership, borrowing, lifetimes, and type checking are still handled by normal Rust. RIG does not replace the compiler, invent a programming language, or implement custom allocator internals. It makes allocation and growth behavior visible at the container level so developers can see what grows over time.

## What v0.6.0 proves

RIG v0.6.0 is intentionally small and real:

- `Arena` gives a human-readable name to a tracking scope.
- `RigVec<T>` wraps a real `Vec<T>`.
- `RigString` wraps a real `String`.
- `push`, `push_str`, `len`, `is_empty`, and `capacity` behave like normal Rust container operations.
- Capacity growth events, total pushed items, append operations, and appended bytes are tracked.
- `arena.report()` produces live allocation/growth data from tracked containers.
- `arena.snapshot()` returns typed machine-readable report data.
- `arena.report_json()` serializes that snapshot with real `serde_json`.
- `arena.write_json(path)` persists a report only when explicitly called.
- `Arena::load_report(path)` loads persisted report JSON back into an `ArenaReport`.
- `ArenaReport::diff(&after)` compares two snapshots and returns an `ArenaDiff`.
- `ArenaDiff::diff_json()` serializes comparison evidence with real `serde_json`.

## Example

```rust
use rig::{Arena, RigVec};

let mut arena = Arena::new("request-lifetime arena");
let mut users = RigVec::new(&mut arena, "users");

users.push(1);
users.push(2);

println!("{}", arena.report());
```

Output includes the arena name, container name, current length, capacity, growth events, and total pushed/appended operations.

## Machine-readable reports and diffs

RIG keeps the existing human report and adds structured report and diff data for tools.

```rust
let snapshot = arena.snapshot();
let json = arena.report_json();
let before = arena.snapshot();
// mutate tracked containers
let after = arena.snapshot();
let diff = before.diff(&after);
let diff_json = diff.diff_json();
```

`arena.snapshot()` returns an `ArenaReport` with the arena name, tracked container count, aggregate totals, and per-container evidence. `arena.report_json()` pretty-prints the same data through `serde_json::to_string_pretty(&self.snapshot())`, using the real crates.io `serde` and `serde_json` crates. `ArenaReport::diff(&after)` returns an `ArenaDiff` with containers added, containers removed, aggregate deltas, and a `ContainerDiff` for every container present in both reports. `diff.diff_json()` uses `serde_json` for machine-readable comparison evidence.

Small JSON example:

```json
{
  "arena_name": "request-lifetime arena",
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

RIG v0.6.0 explains change between two reports without adding a CLI, macros, async work, background services, automatic persistence, or hidden files.

```rust
let before = arena.snapshot();
users.push(9);
let after = arena.snapshot();
let diff = before.diff(&after);

println!("{}", diff.report());
println!("{}", diff.diff_json());
```

Human diff output is intentionally direct:

```text
RIG allocation diff
Before: request-lifetime arena
After: request-lifetime arena
Changed containers:
  users
    len: +4
    capacity: +8
    growth events: +1
    operations: +4
```

## Optional evidence persistence

RIG does not write files automatically. Default RIG behavior remains fully in-memory: `Arena::new()`, `RigVec` and `RigString` operations, `arena.report()`, `arena.snapshot()`, and `arena.report_json()` do not create files, logs, `.rig/`, or background output.

Persistence is opt-in. The only time RIG writes a report to disk is when the programmer explicitly calls `Arena::write_json(path)`. `Arena::write_json(path)` creates or overwrites the target report file with pretty JSON and returns real `std::io::Result<()>` filesystem errors.

`Arena::load_report(path)` reads a report back from disk into an `ArenaReport`. It returns a typed `LoadReportError` that distinguishes filesystem IO failures from JSON deserialization failures. This lets reports survive the process for later inspection without adding automatic file generation or hidden runtime behavior.

```rust
let path = std::env::temp_dir().join("rig-report.json");
arena.write_json(&path)?;
let loaded = Arena::load_report(&path)?;
assert_eq!(loaded, arena.snapshot());
```

## Path to v1

RIG v0.6.0 is public API hardening for the path to a real v1. It does not add a CLI, macros, async work, background services, automatic persistence, or hidden project files. The point of this release is to make the API shape intentional, documented, and resistant to misuse.

A real v1 requires stable public API shape, useful rustdoc for exported types and methods, compiling doc tests for normal workflows, and abuse tests that prove RIG stays explicit under pressure. RIG still avoids hidden behavior: reports, snapshots, JSON rendering, and diffs remain in memory unless the programmer explicitly chooses a `write_json` path.

---

## Run the demo

```bash
cargo run --example demo
```

The v0.6.0 demo creates report A, mutates tracked containers, creates report B, prints the readable report, prints the JSON report, prints the human diff, prints the JSON diff, explicitly writes the report to a temp file, loads it back, and verifies the loaded report equals the live snapshot.

```text
Rust is still safe, but allocation and growth behavior is now visible.

RIG allocation report
Arena: request-lifetime arena
Tracked containers: 4
Totals:
  total len: 49
  total current capacity: 76
  total growth events: 5
  total pushed/appended operations: 16
Containers:
  Container: users
  kind: RigVec
  fields:
    len: 8
    initial capacity: 0
    current capacity: 8
    growth events: 2
    total pushed items: 8
  Container: cached_users
  kind: RigVec
  fields:
    len: 4
    initial capacity: 4
    current capacity: 4
    growth events: 0
    total pushed items: 4
  Container: audit_events
  kind: RigString
  fields:
    len: 25
    initial capacity: 0
    current capacity: 32
    growth events: 3
    total append operations: 3
    total appended bytes: 25
  Container: request_path
  kind: RigString
  fields:
    len: 12
    initial capacity: 32
    current capacity: 32
    growth events: 0
    total append operations: 1
    total appended bytes: 12
```

## What RIG is not

RIG is not:

- a new programming language
- a garbage collector
- a framework
- compiler work
- a macro system
- custom allocator internals

## Smoke tests that matter in v0.6.0

The v0.6.0 smoke tests prove real capability:

- arenas can be named and reported
- tracked vectors and strings start empty and remain usable as normal Rust containers
- pushes update length and total pushed item counts
- string appends update length, append operation counts, and appended byte counts
- capacity increases record growth events
- multiple containers can report through one arena
- empty container reports still contain valid allocation/growth fields
- reports preserve exact readable indentation for totals, containers, and fields
- snapshots contain arena names, tracked container counts, totals, container kinds, capacity, and growth evidence
- JSON reports parse with real `serde_json` and round-trip into `ArenaReport`
- explicit `write_json` creates or overwrites a programmer-selected report file
- `Arena::load_report` loads persisted reports and distinguishes IO errors from JSON errors
- in-memory report APIs do not create files implicitly
- identical reports produce zero diff deltas
- added and removed containers are detected
- length, capacity, growth event, and operation increases are detected
- JSON diffs parse and round-trip as `ArenaDiff`
- human diffs are readable and inspectable
- the repository does not contain a fake `vendor/` dependency tree
