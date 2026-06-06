# RIG

RIG is a Rust crate that brings Zig-style allocator visibility to everyday Rust code.

Rust keeps doing the safety work: ownership, borrowing, lifetimes, and type checking are still handled by normal Rust. RIG does not replace the compiler, invent a programming language, or implement custom allocator internals. It makes allocation and growth behavior visible at the container level so developers can see what grows over time.

## What v1 proves

RIG v1 is intentionally small and real:

- `Arena` gives a human-readable name to a tracking scope.
- `RigVec<T>` wraps a real `Vec<T>`.
- `RigString` wraps a real `String`.
- `push`, `push_str`, `len`, `is_empty`, and `capacity` behave like normal Rust container operations.
- Capacity growth events, total pushed items, append operations, and appended bytes are tracked.
- `arena.report()` produces live allocation/growth data from tracked containers.

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

## Run the demo

```bash
cargo run --example demo
```

The v1 demo creates tracked vectors and strings, then prints the exact readable report below:

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

## Smoke tests that matter in v1

The v1 smoke tests prove real capability:

- arenas can be named and reported
- tracked vectors and strings start empty and remain usable as normal Rust containers
- pushes update length and total pushed item counts
- string appends update length, append operation counts, and appended byte counts
- capacity increases record growth events
- multiple containers can report through one arena
- empty container reports still contain valid allocation/growth fields
- reports preserve exact readable indentation for totals, containers, and fields
