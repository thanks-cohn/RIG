use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::rc::Rc;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ContainerKind {
    RigVec,
    RigString,
}

impl ContainerKind {
    fn as_str(self) -> &'static str {
        match self {
            Self::RigVec => "RigVec",
            Self::RigString => "RigString",
        }
    }
}

#[derive(Debug, Clone)]
struct ContainerRecord {
    name: String,
    kind: ContainerKind,
    len: usize,
    initial_capacity: usize,
    capacity: usize,
    growth_events: usize,
    operation_label: &'static str,
    total_operations: usize,
    extra_metric_label: Option<&'static str>,
    extra_metric_value: usize,
}

impl ContainerRecord {
    fn new(name: impl Into<String>, kind: ContainerKind, initial_capacity: usize) -> Self {
        let (operation_label, extra_metric_label) = match kind {
            ContainerKind::RigVec => ("total pushed items", None),
            ContainerKind::RigString => ("total append operations", Some("total appended bytes")),
        };

        Self {
            name: name.into(),
            kind,
            len: 0,
            initial_capacity,
            capacity: initial_capacity,
            growth_events: 0,
            operation_label,
            total_operations: 0,
            extra_metric_label,
            extra_metric_value: 0,
        }
    }
}

/// Structured totals across all containers tracked by an [`Arena`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArenaTotals {
    /// Sum of all tracked container lengths.
    pub total_len: usize,
    /// Sum of all tracked container current capacities.
    pub total_current_capacity: usize,
    /// Sum of all tracked container growth events.
    pub total_growth_events: usize,
    /// Sum of all tracked push and append operations.
    pub total_pushed_appended_operations: usize,
}

/// Structured report for one tracked container.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContainerReport {
    /// Human-readable container name supplied by the caller.
    pub name: String,
    /// Container kind, such as `RigVec` or `RigString`.
    pub kind: String,
    /// Current container length.
    pub len: usize,
    /// Capacity requested when the tracked container was created.
    pub initial_capacity: usize,
    /// Current underlying container capacity.
    pub current_capacity: usize,
    /// Number of times an operation caused capacity to grow.
    pub growth_events: usize,
    /// Human-readable operation metric label used by the text report.
    pub operation_label: String,
    /// Total operations for the primary operation metric.
    pub total_operations: usize,
    /// Optional human-readable extra metric label used by the text report.
    pub extra_metric_label: Option<String>,
    /// Optional extra metric value, such as appended bytes for `RigString`.
    pub extra_metric_value: usize,
}

/// Structured snapshot of an arena suitable for JSON and tool consumption.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArenaReport {
    /// Human-readable arena name.
    pub arena_name: String,
    /// Number of tracked containers.
    pub tracked_container_count: usize,
    /// Totals across all tracked containers.
    pub totals: ArenaTotals,
    /// Per-container report data.
    pub containers: Vec<ContainerReport>,
}

#[derive(Debug)]
struct ArenaInner {
    name: String,
    records: Vec<ContainerRecord>,
}

/// A named tracking arena for visible allocation and container growth reports.
#[derive(Debug, Clone)]
pub struct Arena {
    inner: Rc<RefCell<ArenaInner>>,
}

impl Arena {
    /// Create a new named arena.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            inner: Rc::new(RefCell::new(ArenaInner {
                name: name.into(),
                records: Vec::new(),
            })),
        }
    }

    /// Return the human-readable arena name.
    pub fn name(&self) -> String {
        self.inner.borrow().name.clone()
    }

    fn add_record(
        &mut self,
        container_name: impl Into<String>,
        kind: ContainerKind,
        initial_capacity: usize,
    ) -> usize {
        let mut inner = self.inner.borrow_mut();
        inner
            .records
            .push(ContainerRecord::new(container_name, kind, initial_capacity));
        inner.records.len() - 1
    }

    fn update_record(
        &self,
        record_id: usize,
        len: usize,
        capacity: usize,
        growth_events: usize,
        total_operations: usize,
        extra_metric_value: usize,
    ) {
        let mut inner = self.inner.borrow_mut();
        if let Some(record) = inner.records.get_mut(record_id) {
            record.len = len;
            record.capacity = capacity;
            record.growth_events = growth_events;
            record.total_operations = total_operations;
            record.extra_metric_value = extra_metric_value;
        }
    }

    /// Return structured allocation and growth data for tracked containers.
    pub fn snapshot(&self) -> ArenaReport {
        let inner = self.inner.borrow();
        let total_len: usize = inner.records.iter().map(|record| record.len).sum();
        let total_current_capacity: usize =
            inner.records.iter().map(|record| record.capacity).sum();
        let total_growth_events: usize = inner
            .records
            .iter()
            .map(|record| record.growth_events)
            .sum();
        let total_pushed_appended_operations: usize = inner
            .records
            .iter()
            .map(|record| record.total_operations)
            .sum();
        let containers = inner
            .records
            .iter()
            .map(|record| ContainerReport {
                name: record.name.clone(),
                kind: record.kind.as_str().to_string(),
                len: record.len,
                initial_capacity: record.initial_capacity,
                current_capacity: record.capacity,
                growth_events: record.growth_events,
                operation_label: record.operation_label.to_string(),
                total_operations: record.total_operations,
                extra_metric_label: record.extra_metric_label.map(str::to_string),
                extra_metric_value: record.extra_metric_value,
            })
            .collect();

        ArenaReport {
            arena_name: inner.name.clone(),
            tracked_container_count: inner.records.len(),
            totals: ArenaTotals {
                total_len,
                total_current_capacity,
                total_growth_events,
                total_pushed_appended_operations,
            },
            containers,
        }
    }

    /// Return a pretty JSON allocation and growth report for tracked containers.
    pub fn report_json(&self) -> String {
        serde_json::to_string_pretty(&self.snapshot())
            .expect("serializing ArenaReport to JSON should not fail")
    }

    /// Return a human-readable allocation and growth report for tracked containers.
    pub fn report(&self) -> String {
        let snapshot = self.snapshot();
        let mut report = format!(
            "RIG allocation report
Arena: {}
Tracked containers: {}
Totals:
  total len: {}
  total current capacity: {}
  total growth events: {}
  total pushed/appended operations: {}
Containers:",
            snapshot.arena_name,
            snapshot.tracked_container_count,
            snapshot.totals.total_len,
            snapshot.totals.total_current_capacity,
            snapshot.totals.total_growth_events,
            snapshot.totals.total_pushed_appended_operations
        );

        if snapshot.containers.is_empty() {
            report.push_str(
                "
  (none)",
            );
            return report;
        }

        for container in &snapshot.containers {
            report.push_str(&format!(
                "
  Container: {}
  kind: {}
  fields:
    len: {}
    initial capacity: {}
    current capacity: {}
    growth events: {}
    {}: {}",
                container.name,
                container.kind,
                container.len,
                container.initial_capacity,
                container.current_capacity,
                container.growth_events,
                container.operation_label,
                container.total_operations
            ));

            if let Some(label) = &container.extra_metric_label {
                report.push_str(&format!(
                    "
    {}: {}",
                    label, container.extra_metric_value
                ));
            }
        }

        report
    }
}

/// A `Vec<T>` wrapper that keeps Rust safety while making growth visible.
#[derive(Debug)]
pub struct RigVec<T> {
    values: Vec<T>,
    arena: Arena,
    record_id: usize,
    growth_events: usize,
    total_pushed: usize,
}

impl<T> RigVec<T> {
    /// Create a tracked vector record inside an arena.
    pub fn new(arena: &mut Arena, container_name: impl Into<String>) -> Self {
        Self::with_capacity(arena, container_name, 0)
    }

    /// Create a tracked vector record inside an arena with preallocated capacity.
    pub fn with_capacity(
        arena: &mut Arena,
        container_name: impl Into<String>,
        capacity: usize,
    ) -> Self {
        let record_id = arena.add_record(container_name, ContainerKind::RigVec, capacity);
        let vec = Self {
            values: Vec::with_capacity(capacity),
            arena: arena.clone(),
            record_id,
            growth_events: 0,
            total_pushed: 0,
        };
        vec.sync_record();
        vec
    }

    /// Push an item into the underlying `Vec<T>` and record capacity growth.
    pub fn push(&mut self, value: T) {
        let old_capacity = self.values.capacity();
        self.values.push(value);
        self.total_pushed += 1;

        if self.values.capacity() > old_capacity {
            self.growth_events += 1;
        }

        self.sync_record();
    }

    /// Return the number of items currently stored.
    pub fn len(&self) -> usize {
        self.values.len()
    }

    /// Return whether the vector is empty.
    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }

    /// Return the current underlying `Vec<T>` capacity.
    pub fn capacity(&self) -> usize {
        self.values.capacity()
    }

    /// Return how many times pushing caused capacity to grow.
    pub fn growth_events(&self) -> usize {
        self.growth_events
    }

    /// Return the total number of successful `push` operations.
    pub fn total_pushed(&self) -> usize {
        self.total_pushed
    }

    fn sync_record(&self) {
        self.arena.update_record(
            self.record_id,
            self.len(),
            self.capacity(),
            self.growth_events,
            self.total_pushed,
            0,
        );
    }
}

/// A `String` wrapper that keeps Rust safety while making string growth visible.
#[derive(Debug)]
pub struct RigString {
    value: String,
    arena: Arena,
    record_id: usize,
    growth_events: usize,
    append_operations: usize,
    total_appended_bytes: usize,
}

impl RigString {
    /// Create a tracked string record inside an arena.
    pub fn new(arena: &mut Arena, container_name: impl Into<String>) -> Self {
        Self::with_capacity(arena, container_name, 0)
    }

    /// Create a tracked string record inside an arena with preallocated capacity.
    pub fn with_capacity(
        arena: &mut Arena,
        container_name: impl Into<String>,
        capacity: usize,
    ) -> Self {
        let record_id = arena.add_record(container_name, ContainerKind::RigString, capacity);
        let string = Self {
            value: String::with_capacity(capacity),
            arena: arena.clone(),
            record_id,
            growth_events: 0,
            append_operations: 0,
            total_appended_bytes: 0,
        };
        string.sync_record();
        string
    }

    /// Append a string slice and record capacity growth.
    pub fn push_str(&mut self, value: &str) {
        let old_capacity = self.value.capacity();
        self.value.push_str(value);
        self.append_operations += 1;
        self.total_appended_bytes += value.len();

        if self.value.capacity() > old_capacity {
            self.growth_events += 1;
        }

        self.sync_record();
    }

    /// Return the current string length in bytes.
    pub fn len(&self) -> usize {
        self.value.len()
    }

    /// Return whether the string is empty.
    pub fn is_empty(&self) -> bool {
        self.value.is_empty()
    }

    /// Return the current underlying `String` capacity in bytes.
    pub fn capacity(&self) -> usize {
        self.value.capacity()
    }

    /// Return how many times appending caused capacity to grow.
    pub fn growth_events(&self) -> usize {
        self.growth_events
    }

    /// Return the total number of successful `push_str` calls.
    pub fn append_operations(&self) -> usize {
        self.append_operations
    }

    /// Return the total number of bytes appended through `push_str`.
    pub fn total_appended_bytes(&self) -> usize {
        self.total_appended_bytes
    }

    fn sync_record(&self) {
        self.arena.update_record(
            self.record_id,
            self.len(),
            self.capacity(),
            self.growth_events,
            self.append_operations,
            self.total_appended_bytes,
        );
    }
}
