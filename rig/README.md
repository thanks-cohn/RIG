# RIG

RIG is a Rust crate that brings Zig-style allocator visibility to everyday Rust code.

Rust keeps doing the safety work: ownership, borrowing, lifetimes, and type checking are still handled by normal Rust. RIG does not replace the compiler, invent a programming language, or implement custom allocator internals. It makes allocation and growth behavior visible at the container level so developers can see what grows over time.

## What v0 proves

RIG v0 is intentionally small and real:

- `Arena` gives a human-readable name to a tracking scope.
- `RigVec<T>` wraps a real `Vec<T>`.
- `push`, `len`, `is_empty`, and `capacity` behave like normal Rust container operations.
- Capacity growth events and total pushed items are tracked.
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

Output includes the arena name, container name, current length, capacity, growth events, and total pushed items.

## Run the demo

```bash
cargo run --example demo
```

The demo creates two tracked vectors and prints a report that shows: Rust is still safe, but memory growth is now visible.

## What RIG is not

RIG is not:

- a new programming language
- a garbage collector
- a framework
- compiler work
- a macro system
- custom allocator internals

## Smoke tests that matter in v0

The v0 smoke tests prove real capability:

- arenas can be named and reported
- tracked vectors start empty and remain usable as normal Rust containers
- pushes update length and total pushed item counts
- capacity increases record growth events
- multiple vectors can report through one arena
- empty vector reports still contain valid allocation/growth fields
