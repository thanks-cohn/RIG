# RIG

## The Explainable Systems Language

RIG is an experimental systems programming language focused on a simple idea:

> A program should not merely run.
>
> A program should explain itself.

For fifty years, systems programming has revolved around three competing virtues:

* C gave us power.
* Rust gave us safety.
* Zig gave us clarity.

RIG seeks a fourth virtue:

* Explainability.

The goal is not merely to build software that works.

The goal is to build software that can answer:

```text
What happened?
When did it happen?
Where did it happen?
Why did it happen?
What changed?
How do I reproduce it?
```

---

## Philosophy

Modern systems are often powerful but opaque.

When they fail, developers are left asking:

```text
Everything is correct on paper.

Then why did it break?
```

RIG exists to eliminate mystery.

Every build should leave evidence.

Every run should leave evidence.

Every crash should leave evidence.

Every successful result should be reproducible.

The machine should know its own history.

---

## Core Principles

### No Hidden Allocation

Memory should be visible.

The programmer should know:

* who allocated it
* who owns it
* who frees it
* how long it lives

---

### No Hidden Control Flow

Execution should be explicit.

No invisible magic.

No mysterious behavior.

No surprises.

---

### Explainability First

RIG treats explainability as a first-class feature.

The language should answer questions, not merely report errors.

---

### Reproducibility First

Successful runs should be replayable.

Failed runs should be investigable.

A build should be a receipt.

Not a ritual.

---

### Machine Visibility

The machine is not an enemy.

The machine is not magic.

The machine is understandable.

RIG exists to make that understanding practical.

---

## Initial Architecture

RIG v0 does not attempt to replace Rust.

Instead, RIG stands on Rust's shoulders.

```text
RIG Source
    ↓
RIG Frontend
    ↓
Rust Generation
    ↓
rustc
    ↓
LLVM
    ↓
Native Binary
```

This allows RIG to benefit from:

* Rust's safety model
* Rust's compiler infrastructure
* LLVM optimization
* Existing ecosystems

while pursuing a different programming experience.

---

## Long-Term Architecture

```text
RIG Source
    ↓
RIG Parser
    ↓
RIG Type System
    ↓
RIG Ownership System
    ↓
RIG Witness Engine
    ↓
RIG Intermediate Representation
    ↓
LLVM / Native Backends
```

---

## The Witness System

The defining feature of RIG.

RIG programs do not merely execute.

They leave evidence.

Examples:

```bash
rig build
rig run
rig inspect
rig replay
rig why
```

---

### Example

```bash
rig why crash
```

Output:

```text
Crash detected

Cause:
use-after-free

Allocation:
src/cache.rig:44

Freed:
src/cache.rig:91

Invalid Access:
src/router.rig:138

Triggered By:
Request #8842

Build:
ReleaseFast

Machine:
west-o

Last Successful Run:
2026-06-05 13:22 UTC
```

---

## Goals

### Short Term

* Zig-inspired syntax
* Rust backend
* Clear diagnostics
* Explicit memory model
* Minimal runtime

### Medium Term

* Ownership analysis
* Witness engine
* Build ledger
* Runtime ledger
* Crash forensics

### Long Term

* Self-hosting compiler
* Native backend
* Operating system tooling
* Reproducible infrastructure
* Explainable computing stack

---

## Non-Goals

RIG is not attempting to become:

* another garbage collected language
* another Java clone
* another JavaScript runtime
* another abstraction-heavy framework

RIG is for developers who want to understand the machine.

---

## Vision

Imagine a future where software can answer:

```text
Why did this work?

Why did this fail?

What changed?

Can I recreate it?
```

without guesswork.

That is the world RIG seeks to build.

A world where computers leave receipts.

A world where mystery is optional.
