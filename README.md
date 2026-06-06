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
