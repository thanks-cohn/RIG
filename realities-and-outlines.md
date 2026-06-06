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
