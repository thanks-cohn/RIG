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

    /// Return a human-readable allocation and growth report for tracked containers.
    pub fn report(&self) -> String {
        let inner = self.inner.borrow();
        let total_len: usize = inner.records.iter().map(|record| record.len).sum();
        let total_capacity: usize = inner.records.iter().map(|record| record.capacity).sum();
        let total_growth_events: usize = inner
            .records
            .iter()
            .map(|record| record.growth_events)
            .sum();
        let total_operations: usize = inner
            .records
            .iter()
            .map(|record| record.total_operations)
            .sum();

        let mut report = format!(
            "RIG allocation report\nArena: {}\nTracked containers: {}\nTotals:\n  total len: {}\n  total current capacity: {}\n  total growth events: {}\n  total pushed/appended operations: {}\nContainers:",
            inner.name,
            inner.records.len(),
            total_len,
            total_capacity,
            total_growth_events,
            total_operations
        );

        if inner.records.is_empty() {
            report.push_str("\n  (none)");
            return report;
        }

        for record in &inner.records {
            report.push_str(&format!(
                "\n  Container: {}\n  kind: {}\n  fields:\n    len: {}\n    initial capacity: {}\n    current capacity: {}\n    growth events: {}\n    {}: {}",
                record.name,
                record.kind.as_str(),
                record.len,
                record.initial_capacity,
                record.capacity,
                record.growth_events,
                record.operation_label,
                record.total_operations
            ));

            if let Some(label) = record.extra_metric_label {
                report.push_str(&format!("\n    {}: {}", label, record.extra_metric_value));
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
