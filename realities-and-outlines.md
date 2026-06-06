v0

```
rig/
├── Cargo.toml
├── README.md
├── src/
│   └── lib.rs
├── examples/
│   └── demo.rs
└── tests/
    └── basic.rs
```


v3
```
rig/
├── Cargo.toml
├── README.md
├── src/
│   ├── lib.rs
│   ├── arena.rs
│   ├── alloc.rs
│   ├── report.rs
│   ├── vec.rs
│   ├── string.rs
│   ├── box.rs
│   └── budget.rs
├── examples/
│   ├── demo.rs
│   ├── arena_pipeline.rs
│   └── memory_budget.rs
└── tests/
    ├── basic.rs
    ├── growth.rs
    └── budget.rs

```

v5

```
rig/
├── crates/
│   ├── rig-core/
│   ├── rig-alloc/
│   ├── rig-report/
│   ├── rig-macros/
│   └── rig-cli/
├── examples/
│   ├── arena_demo/
│   ├── parser_pipeline/
│   ├── server_memory_profile/
│   └── embedded_style/
├── docs/
│   ├── philosophy.md
│   ├── allocation-model.md
│   ├── reporting.md
│   └── budgets.md
├── tests/
│   ├── integration.rs
│   └── snapshots/
├── Cargo.toml
└── README.md
```

v10

```
rig/
├── crates/
│   ├── rig-core/
│   ├── rig-alloc/
│   ├── rig-arena/
│   ├── rig-pool/
│   ├── rig-containers/
│   ├── rig-report/
│   ├── rig-budget/
│   ├── rig-trace/
│   ├── rig-profiler/
│   ├── rig-macros/
│   ├── rig-cli/
│   ├── rig-cargo/
│   ├── rig-devtools/
│   └── rig-prelude/
├── src/
│   └── lib.rs
├── examples/
│   ├── hello_visible_memory/
│   ├── arena_first_app/
│   ├── parser_pipeline/
│   ├── web_server_budget/
│   ├── embedded_no_std/
│   ├── game_loop_allocator/
│   ├── database_cache_profile/
│   └── before_after_plain_rust/
├── benches/
│   ├── vec_growth.rs
│   ├── arena_alloc.rs
│   ├── pool_alloc.rs
│   └── report_overhead.rs
├── tests/
│   ├── integration.rs
│   ├── budgets.rs
│   ├── reports.rs
│   ├── no_std.rs
│   └── snapshots/
├── docs/
│   ├── philosophy.md
│   ├── getting-started.md
│   ├── allocation-model.md
│   ├── ownership-and-lifetimes.md
│   ├── allocator-visibility.md
│   ├── budgets.md
│   ├── reports.md
│   ├── cargo-rig.md
│   ├── no-std.md
│   ├── embedded.md
│   ├── server-patterns.md
│   └── migration-from-plain-rust.md
├── book/
│   ├── src/
│   │   ├── introduction.md
│   │   ├── rust-with-zig-clarity.md
│   │   ├── memory-should-be-visible.md
│   │   ├── arenas.md
│   │   ├── pools.md
│   │   ├── containers.md
│   │   ├── budgets.md
│   │   ├── reporting.md
│   │   └── production-patterns.md
│   └── book.toml
├── .github/
│   └── workflows/
│       ├── ci.yml
│       ├── benches.yml
│       └── release.yml
├── Cargo.toml
├── README.md
├── LICENSE-MIT
└── LICENSE-APACHE
```
