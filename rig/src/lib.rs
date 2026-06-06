use std::cell::RefCell;
use std::rc::Rc;

#[derive(Debug, Clone)]
struct ContainerRecord {
    name: String,
    len: usize,
    initial_capacity: usize,
    capacity: usize,
    growth_events: usize,
    total_pushed: usize,
}

impl ContainerRecord {
    fn new(name: impl Into<String>, initial_capacity: usize) -> Self {
        Self {
            name: name.into(),
            len: 0,
            initial_capacity,
            capacity: initial_capacity,
            growth_events: 0,
            total_pushed: 0,
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

    fn add_record(&mut self, container_name: impl Into<String>, initial_capacity: usize) -> usize {
        let mut inner = self.inner.borrow_mut();
        inner
            .records
            .push(ContainerRecord::new(container_name, initial_capacity));
        inner.records.len() - 1
    }

    fn update_record(
        &self,
        record_id: usize,
        len: usize,
        capacity: usize,
        growth_events: usize,
        total_pushed: usize,
    ) {
        let mut inner = self.inner.borrow_mut();
        if let Some(record) = inner.records.get_mut(record_id) {
            record.len = len;
            record.capacity = capacity;
            record.growth_events = growth_events;
            record.total_pushed = total_pushed;
        }
    }

    /// Return a human-readable allocation and growth report for tracked containers.
    pub fn report(&self) -> String {
        let inner = self.inner.borrow();
        let mut report = format!(
            "RIG allocation report\nArena: {}\nTracked containers: {}",
            inner.name,
            inner.records.len()
        );

        if inner.records.is_empty() {
            report.push_str("\nNo tracked containers.");
            return report;
        }

        for record in &inner.records {
            report.push_str(&format!(
                "\n- Container: {}\n  fields:\n    len: {}\n    initial capacity: {}\n    current capacity: {}\n    growth events: {}\n    total pushed items: {}",
                record.name,
                record.len,
                record.initial_capacity,
                record.capacity,
                record.growth_events,
                record.total_pushed
            ));
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
        let record_id = arena.add_record(container_name, capacity);
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
        );
    }
}
