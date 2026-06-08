
```
██╗     ███████╗████████╗███████╗
██║     ██╔════╝╚══██╔══╝██╔════╝
██║     █████╗     ██║   ███████╗
██║     ██╔══╝     ██║   ╚════██║
███████╗███████╗   ██║   ███████║
╚══════╝╚══════╝   ╚═╝   ╚══════╝

██████╗ ██╗ ██████╗     ██╗████████╗
██╔══██╗██║██╔════╝     ██║╚══██╔══╝
██████╔╝██║██║  ███╗    ██║   ██║
██╔══██╗██║██║   ██║    ██║   ██║
██║  ██║██║╚██████╔╝    ██║   ██║
╚═╝  ╚═╝╚═╝ ╚═════╝     ╚═╝   ╚═╝
```

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
Visibility oveR mysteRy.

ExplIcitness over assumptIon.

UnderstandinG over guessinG.
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

RIG v0.7.0 has real machine-readable reports, evidence comparison, and optional evidence persistence through the Rust ecosystem rather than homemade serialization.

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

RIG v0.7.0 can explain change between two snapshots. Take report A, mutate tracked containers, take report B, and diff them.

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


## Growth history

RIG records real observed capacity growth events while tracked containers are mutated. A `GrowthEvent` captures the container name, container kind, old capacity, new capacity, and operation index after the push or append that caused the capacity change.

Growth history is not inferred later and fake events are not generated. It is observed live when `RigVec::push` or `RigString::push_str` sees capacity increase. Like snapshots, reports, JSON rendering, and diffs, this stays in memory unless the caller explicitly invokes `write_json` with a path.

## Optional evidence persistence

RIG does not write files automatically. Persistence is 100% opt-in: default reports stay in memory, and RIG does not create `.rig/`, logs, mystery directories, or background files.

`Arena::write_json(path)` writes the current pretty JSON report only when the programmer explicitly calls it. `Arena::load_report(path)` loads a persisted report back into an `ArenaReport`, so allocation/growth evidence can survive the process for later inspection.

## Report artifacts

RIG reports can be written as explicit JSON artifacts with `ArenaReport::write_artifact(path)`. The caller chooses the path, and RIG writes only that requested JSON file. Saved artifacts can be loaded later with `ReportArtifact::load(path)`, preserving the exact `ArenaReport` evidence that was written.

Loaded artifacts can be compared with `baseline_artifact.compare_to(&current_artifact)`. The resulting `ArtifactComparison` derives its `ArenaDiff` from the saved baseline and current reports, can print human evidence with `report()`, and can print compact JSON evidence with `report_json()`. Regression gates and memory budgets can also be run from saved evidence with `ArtifactComparison::regression_report(&RegressionBudget)` and `ArtifactComparison::budget_report(&MemoryBudget)`.

RIG does not automatically persist reports, does not create hidden files, and does not create hidden directories. Artifact persistence remains useful for CI pipelines, classrooms, reproducible memory audits, release validation, and before/after comparisons because every comparison and gate is based on explicit saved report evidence.

---

## Path to v1

RIG v0.7.0 is public API hardening for the path to a real v1. It does not add a CLI, macros, async work, background services, automatic persistence, or hidden project files. The point of this release is to make the API shape intentional, documented, and resistant to misuse.

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

## Real workload examples

RIG v0.8.0 adds deterministic workload examples that use the same tracked containers people inspect in the core API, but under larger and more realistic pressure:

- ECS simulation: tracks entity IDs, positions, velocities, active entity IDs, and a frame log while loading at least 100,000 entities and running at least 60 update frames.
- Log ingestion: tracks a raw log buffer plus parsed, warning, and error line containers while deterministically generating and ingesting at least 50,000 log lines.
- Pathfinding: tracks frontier, visited nodes, parent edges, reconstructed path nodes, and a search log while running deterministic breadth-first search on a grid graph.

Each example prints human reports, JSON reports, diffs, and growth history. These examples are intentionally deterministic, require no external files, and do not write reports unless a programmer explicitly adds persistence code.

## Allocation policy experiments

RIG v0.9.0 begins the path from allocation visibility toward allocation control. The new `GrowthPolicy` API lets developers run the same workload under explicit container-growth strategies and compare the consequences in reports.

These policies do **not** replace Rust's allocator and do not fake allocator metrics. They control when `RigVec` and `RigString` reserve capacity before a push or append, then RIG reports the actual observed capacity after Rust has performed the allocation.

Available policies:

- `RustDefault` keeps the previous behavior and lets `Vec` or `String` grow normally.
- `Double` reserves before growth so requested capacity is at least doubled, starting at at least 4 from zero capacity.
- `Exact` reserves exactly enough for the next needed length, while still reporting the actual capacity Rust provides.
- `ReserveAhead(n)` reserves enough for the needed length plus `n` spare capacity and records `ReserveAhead(n)` in reports.
- `Capped { max_capacity }` refuses growth beyond the cap through fallible APIs.

Use `RigVec::try_push` and `RigString::try_push_str` with capped containers to receive a typed `RigError::CapacityLimitExceeded` instead of relying on panic-only behavior. The ergonomic `push` and `push_str` methods remain available, but they panic clearly if a capped policy refuses growth.

This is a safe Rust bridge toward Zig-like explicit allocation thinking: choose a growth policy, run a real workload, inspect the observed growth events and current capacity, and make evidence-backed decisions without hidden files, automatic persistence, macros, async machinery, or a CLI.

## Readable evidence summaries

RIG v0.10.0 keeps raw allocation-growth evidence while making human reports readable for large real workloads. `ArenaReport` still contains the full `growth_history` vector for machines, JSON reports still include that raw history, and no evidence is deleted or replaced by a shortcut metric.

Human reports now summarize growth by default. `arena.report()` and `ArenaReport::report()` show the normal allocation report, a `GrowthSummary`, per-container growth summaries, the first few growth events, and the last few growth events. If a policy such as `Exact` creates 50,000 growth events, the default report explains the evidence without flooding the console.

Verbose reports expose the full raw history when a human needs every event:

```rust
let compact = arena.report();
let verbose = arena.report_verbose();
let summary_json = arena.growth_summary_json();
```

`GrowthSummary` and `ContainerGrowthSummary` derive every number from observed `GrowthEvent` data: total growth events, containers with growth, largest capacity delta, first and last growth events, per-container first and final capacities, and operation-index bounds. JSON remains machine-readable through `arena.report_json()` for full reports and `arena.growth_summary_json()` for compact summary data.

This solves the console flood problem found in real policy-comparison workloads: `Exact` can preserve 50,000 raw events for analysis while the default human report stays compact, and `report_verbose()` remains available for full evidence.

## Allocation attribution

RIG v0.11.0 adds causality for growth events. Reports now explain not only that a tracked container grew, but which operation caused the observed growth, how much capacity was added, which growth policy was active, and how much total capacity expansion has accumulated over that container's lifetime.

New attribution fields are derived from live `GrowthEvent` evidence:

- `ContainerReport::total_capacity_added`
- `ContainerReport::largest_growth_jump`
- `ContainerReport::average_growth_jump`
- `GrowthEvent::capacity_added`
- `GrowthEvent::growth_policy`
- `ArenaReport::growth_attributions`

`ArenaReport::top_growth_containers()` returns containers ordered by lifetime `total_capacity_added`, largest first. Human reports include a `Top growth contributors:` section so memory-heavy containers are visible without scanning every event. JSON reports include attribution data and still round-trip through `serde_json`.

Run the attribution example to see a large string buffer, a large vector, and a mixed workload produce ranked contributors, causal attribution events, and machine-readable report JSON:

```bash
cargo run --example allocation_attribution
```

All allocation numbers remain observed values from tracked container state and recorded growth events. RIG does not estimate capacity, invent memory totals, create hidden files, or persist reports unless the caller explicitly invokes a write method.

---

## Memory regression gates

RIG can compare a baseline `ArenaReport` with a current `ArenaReport` and return typed memory regression evidence. This helps students prove their program did not regress, teachers grade real resource behavior, and teams catch memory-growth regressions in CI before they become production problems.

The gate API is evidence-based: `ArenaReport::check_regressions_against(&baseline, &budget)` compares observed report totals and per-container fields, then returns a `RegressionReport` containing `MemoryRegression` entries for capacity or growth-event increases beyond the configured `RegressionBudget`.

The numbers come from observed RIG reports (`ArenaReport`, `ContainerReport`, and growth-event-derived counts), not estimates, synthetic benchmark scores, or guessed capacities. RIG does not automatically write files for regression gates; human output and JSON output are in-memory strings unless the caller explicitly writes them somewhere.

```rust
use rig::{Arena, RegressionBudget, RigVec};

let mut baseline_arena = Arena::new("baseline");
let mut baseline_values = RigVec::with_capacity(&mut baseline_arena, "values", 4);
baseline_values.push(1);
let baseline = baseline_arena.snapshot();

let mut current_arena = Arena::new("current");
let mut current_values = RigVec::with_capacity(&mut current_arena, "values", 4);
for value in 0..8 {
    current_values.push(value);
}
let current = current_arena.snapshot();

let gate = current.check_regressions_against(&baseline, &RegressionBudget::strict());
println!("{}", gate.report());
```

## Memory budgets

RIG can enforce explicit memory behavior budgets against observed RIG reports. A `MemoryBudget` defines limits for arena totals and per-container values such as current length, current capacity, growth-event count, and operation count.

Budget checks are performed from `ArenaReport::check_budget(&budget)`. They use the values already present in `ArenaReport`, `ContainerReport`, and observed growth evidence; RIG does not invent fake metrics, estimate capacities, or infer missing data.

Memory budgets are useful for schools, CI gates, benchmark discipline, production sanity checks, and memory-aware assignments because they answer whether a workload stayed inside its allowed memory behavior.

Budget checks are in-memory operations and do not write files automatically. `BudgetReport` and `BudgetViolation` provide typed results, and `BudgetReport::report_json()` supports JSON round-trip through `serde_json`.
## Evidence exports

RIG evidence can be exported to CSV or JSON Lines when a caller explicitly asks for it. Container summaries, growth history, growth attributions, budget violations, regression failures, and artifact comparison summaries are available as in-memory strings, and the same explicit export values can be written to caller-provided file paths.

These exports are useful for classrooms, CI artifacts, spreadsheets, grading scripts, report viewers, release validation, and reproducible memory audits outside Rust. Exported values come from observed RIG evidence already present in reports; RIG does not invent metrics, estimate capacities, automatically persist exports, or create hidden files.


## Workload contracts

RIG can validate explicit memory behavior contracts for named workloads. A `WorkloadContract` combines caller-provided memory budgets, regression gates, and evidence profile requirements into one typed check that answers whether the workload honored the memory behavior it promised.

Workload contracts are useful for CI gates, classroom grading, game-level validation, benchmark discipline, release certification, and reproducible audits because every `ContractReport` is derived from observed RIG evidence such as `ArenaReport`, `BudgetReport`, `RegressionReport`, `ProfileReport`, and `ArtifactComparison` data.

Contract reports are typed, can be rendered as human-readable text, and JSON round-trip through `serde_json`. Contracts use observed evidence only: RIG does not add fake metrics, estimate capacity, infer missing rules, write files automatically, create hidden files, or persist anything unless the caller explicitly chooses a persistence API elsewhere.

---

## v1 Readiness and Trust Hardening

RIG is being hardened toward v1 as an explicit, evidence-based library rather than a hidden runtime. The public API is intended to make allocation and growth behavior inspectable through caller-requested arenas, containers, reports, budgets, regressions, artifacts, exports, profiles, and workload contracts.

RIG does not create hidden project state, does not run background services, does not start daemons, and does not persist evidence automatically. Snapshots, reports, diffs, budgets, profiles, contracts, and exports are in-memory evidence until the caller explicitly chooses a write method and provides a path.

The v1 path is focused on API stability, clear errors, abuse resistance, and documentation clarity. RIG should remain boring under pressure: unusual names, empty reports, missing files, invalid JSON, repeated explicit writes, and empty evidence should produce deterministic behavior rather than magic side effects.

## Evidence certification

RIG can produce deterministic evidence certificates: typed, serializable proof objects that summarize a workload subject, the observed report evidence used, any applied contract or budget result, pass/fail status, violation counts, profile counts, and deterministic evidence fingerprints.

Certificates are useful for CI release gates, classroom grading, game levels, benchmarks, artifact review, and release audits because they turn observed RIG reports into durable pass/fail evidence without introducing hidden state.

Fingerprints use deterministic evidence serialization and the built-in `fnv1a64` identifier. They are stable identifiers for comparing evidence; they are not cryptographic hashes, signatures, tamper-proof seals, or blockchain records.

Evidence certification does not write files automatically, does not create hidden files, and does not persist anything unless the caller explicitly invokes an existing write API elsewhere. Certificate fields are derived from observed RIG evidence or explicit caller input.

```rust
let subject = rig::CertificationSubject::new("level-1");
let certificate = arena.snapshot().certify(subject);
assert!(certificate.passed);
println!("{}", certificate.report());
println!("{}", certificate.report_json());
```

---

## RIG v0.20.0 Memory Doctrine

RIG now includes a Memory Doctrine layer for allocation transparency, workload contracts, regression prevention, and reproducible memory engineering. Developers can express expected memory behavior with `WorkloadMemoryContract`, `AllocationBudget`, `ContainerBudget`, `RegressionExpectation`, and `GrowthProfileExpectation`, then validate those expectations against real `BenchmarkEvidence` generated from observed RIG workloads.

Doctrine reports provide pass/fail status, detailed violations, human explanations, machine-readable JSON, and evidence references for each decision. Evidence certification integrates with doctrine validation so successful and failed validations can be fingerprinted as durable release evidence.

Run the doctrine example:

```bash
cargo run --manifest-path rig/Cargo.toml --example memory_doctrine
```

Run release validation with a high-signal end-of-run forensic summary:

```bash
bash scripts/validate.sh
# then inspect the operational summary:
tail -100 logs/validation-*.log
```
