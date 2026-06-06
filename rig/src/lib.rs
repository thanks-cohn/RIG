use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::error::Error;
use std::fmt;
use std::fs;
use std::path::Path;
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

/// Machine-readable totals for all containers tracked by an [`Arena`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArenaTotals {
    /// Sum of current lengths for every tracked container.
    pub total_len: usize,
    /// Sum of current capacities for every tracked container.
    pub total_current_capacity: usize,
    /// Sum of capacity growth events for every tracked container.
    pub total_growth_events: usize,
    /// Sum of tracked push/append operations for every tracked container.
    pub total_pushed_appended_operations: usize,
}

/// Machine-readable state for one container tracked by an [`Arena`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContainerReport {
    /// Human-readable container name supplied by the caller.
    pub name: String,
    /// RIG container wrapper kind, such as `RigVec` or `RigString`.
    pub kind: String,
    /// Current logical length.
    pub len: usize,
    /// Capacity requested when the tracked container was created.
    pub initial_capacity: usize,
    /// Current underlying Rust container capacity.
    pub current_capacity: usize,
    /// Number of operations that caused capacity to increase.
    pub growth_events: usize,
    /// Human-readable operation metric label used by the existing text report.
    pub operation_label: String,
    /// Count for the operation metric.
    pub total_operations: usize,
    /// Optional extra metric label for container-specific data.
    pub extra_metric_label: Option<String>,
    /// Optional extra metric value for container-specific data.
    pub extra_metric_value: Option<usize>,
}

/// Machine-readable arena report returned by [`Arena::snapshot`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArenaReport {
    /// Human-readable arena name supplied by the caller.
    pub arena_name: String,
    /// Number of containers currently tracked by the arena.
    pub tracked_container_count: usize,
    /// Aggregated allocation and operation totals.
    pub totals: ArenaTotals,
    /// Per-container allocation and operation evidence.
    pub containers: Vec<ContainerReport>,
}

/// Machine-readable change evidence between two reports for one shared container.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContainerDiff {
    /// Human-readable container name present in both reports.
    pub name: String,
    /// RIG container wrapper kind from the later report.
    pub kind: String,
    /// Length in the earlier report.
    pub before_len: usize,
    /// Length in the later report.
    pub after_len: usize,
    /// Signed length change from earlier to later report.
    pub len_delta: i64,
    /// Capacity in the earlier report.
    pub before_capacity: usize,
    /// Capacity in the later report.
    pub after_capacity: usize,
    /// Signed capacity change from earlier to later report.
    pub capacity_delta: i64,
    /// Growth event count in the earlier report.
    pub before_growth_events: usize,
    /// Growth event count in the later report.
    pub after_growth_events: usize,
    /// Signed growth event change from earlier to later report.
    pub growth_event_delta: i64,
    /// Operation metric label from the later report.
    pub operation_label: String,
    /// Operation count in the earlier report.
    pub before_operations: usize,
    /// Operation count in the later report.
    pub after_operations: usize,
    /// Signed operation count change from earlier to later report.
    pub operation_delta: i64,
}

/// Machine-readable change evidence between two [`ArenaReport`] values.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArenaDiff {
    /// Arena name in the earlier report.
    pub before_arena_name: String,
    /// Arena name in the later report.
    pub after_arena_name: String,
    /// Containers present only in the later report.
    pub containers_added: Vec<ContainerReport>,
    /// Containers present only in the earlier report.
    pub containers_removed: Vec<ContainerReport>,
    /// Signed aggregate length change from earlier to later report.
    pub total_len_delta: i64,
    /// Signed aggregate capacity change from earlier to later report.
    pub total_capacity_delta: i64,
    /// Signed aggregate growth event change from earlier to later report.
    pub total_growth_event_delta: i64,
    /// Signed aggregate operation change from earlier to later report.
    pub total_operation_delta: i64,
    /// Deltas for every container present in both reports.
    pub containers_changed: Vec<ContainerDiff>,
}

impl ArenaReport {
    /// Compare this earlier report with a later report.
    pub fn diff(&self, after: &ArenaReport) -> ArenaDiff {
        let before_by_name: BTreeMap<&str, &ContainerReport> = self
            .containers
            .iter()
            .map(|container| (container.name.as_str(), container))
            .collect();
        let after_by_name: BTreeMap<&str, &ContainerReport> = after
            .containers
            .iter()
            .map(|container| (container.name.as_str(), container))
            .collect();

        let containers_added = after
            .containers
            .iter()
            .filter(|container| !before_by_name.contains_key(container.name.as_str()))
            .cloned()
            .collect();
        let containers_removed = self
            .containers
            .iter()
            .filter(|container| !after_by_name.contains_key(container.name.as_str()))
            .cloned()
            .collect();

        let containers_changed = self
            .containers
            .iter()
            .filter_map(|before| {
                after_by_name
                    .get(before.name.as_str())
                    .map(|after| ContainerDiff::between(before, after))
            })
            .collect();

        ArenaDiff {
            before_arena_name: self.arena_name.clone(),
            after_arena_name: after.arena_name.clone(),
            containers_added,
            containers_removed,
            total_len_delta: signed_delta(self.totals.total_len, after.totals.total_len),
            total_capacity_delta: signed_delta(
                self.totals.total_current_capacity,
                after.totals.total_current_capacity,
            ),
            total_growth_event_delta: signed_delta(
                self.totals.total_growth_events,
                after.totals.total_growth_events,
            ),
            total_operation_delta: signed_delta(
                self.totals.total_pushed_appended_operations,
                after.totals.total_pushed_appended_operations,
            ),
            containers_changed,
        }
    }
}

impl ContainerDiff {
    fn between(before: &ContainerReport, after: &ContainerReport) -> Self {
        Self {
            name: before.name.clone(),
            kind: after.kind.clone(),
            before_len: before.len,
            after_len: after.len,
            len_delta: signed_delta(before.len, after.len),
            before_capacity: before.current_capacity,
            after_capacity: after.current_capacity,
            capacity_delta: signed_delta(before.current_capacity, after.current_capacity),
            before_growth_events: before.growth_events,
            after_growth_events: after.growth_events,
            growth_event_delta: signed_delta(before.growth_events, after.growth_events),
            operation_label: after.operation_label.clone(),
            before_operations: before.total_operations,
            after_operations: after.total_operations,
            operation_delta: signed_delta(before.total_operations, after.total_operations),
        }
    }
}

impl ArenaDiff {
    /// Return a pretty JSON diff report.
    pub fn diff_json(&self) -> String {
        serde_json::to_string_pretty(self).expect("serializing an ArenaDiff should not fail")
    }

    /// Return a human-readable diff report.
    pub fn report(&self) -> String {
        self.to_string()
    }
}

impl fmt::Display for ArenaDiff {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(formatter, "RIG allocation diff")?;
        writeln!(formatter, "Before: {}", self.before_arena_name)?;
        writeln!(formatter, "After: {}", self.after_arena_name)?;
        writeln!(formatter, "Totals:")?;
        writeln!(formatter, "  len: {}", format_delta(self.total_len_delta))?;
        writeln!(
            formatter,
            "  capacity: {}",
            format_delta(self.total_capacity_delta)
        )?;
        writeln!(
            formatter,
            "  growth events: {}",
            format_delta(self.total_growth_event_delta)
        )?;
        writeln!(
            formatter,
            "  operations: {}",
            format_delta(self.total_operation_delta)
        )?;

        writeln!(formatter, "Added containers:")?;
        if self.containers_added.is_empty() {
            writeln!(formatter, "  (none)")?;
        } else {
            for container in &self.containers_added {
                writeln!(formatter, "  {} ({})", container.name, container.kind)?;
            }
        }

        writeln!(formatter, "Removed containers:")?;
        if self.containers_removed.is_empty() {
            writeln!(formatter, "  (none)")?;
        } else {
            for container in &self.containers_removed {
                writeln!(formatter, "  {} ({})", container.name, container.kind)?;
            }
        }

        writeln!(formatter, "Changed containers:")?;
        if self.containers_changed.is_empty() {
            write!(formatter, "  (none)")?;
        } else {
            for (index, container) in self.containers_changed.iter().enumerate() {
                writeln!(formatter, "  {}", container.name)?;
                writeln!(formatter, "    len: {}", format_delta(container.len_delta))?;
                writeln!(
                    formatter,
                    "    capacity: {}",
                    format_delta(container.capacity_delta)
                )?;
                writeln!(
                    formatter,
                    "    growth events: {}",
                    format_delta(container.growth_event_delta)
                )?;
                write!(
                    formatter,
                    "    operations: {}",
                    format_delta(container.operation_delta)
                )?;
                if index + 1 < self.containers_changed.len() {
                    writeln!(formatter)?;
                }
            }
        }

        Ok(())
    }
}

fn signed_delta(before: usize, after: usize) -> i64 {
    if after >= before {
        i64::try_from(after - before).expect("RIG delta should fit in i64")
    } else {
        -i64::try_from(before - after).expect("RIG delta should fit in i64")
    }
}

fn format_delta(delta: i64) -> String {
    if delta > 0 {
        format!("+{delta}")
    } else {
        delta.to_string()
    }
}

/// Errors that can occur while loading a persisted [`ArenaReport`].
#[derive(Debug)]
pub enum LoadReportError {
    /// The report file could not be read.
    Io(std::io::Error),
    /// The report file was read but did not contain valid `ArenaReport` JSON.
    Json(serde_json::Error),
}

impl fmt::Display for LoadReportError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(formatter, "failed to read RIG report: {error}"),
            Self::Json(error) => write!(formatter, "failed to parse RIG report JSON: {error}"),
        }
    }
}

impl Error for LoadReportError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Io(error) => Some(error),
            Self::Json(error) => Some(error),
        }
    }
}

impl From<std::io::Error> for LoadReportError {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error)
    }
}

impl From<serde_json::Error> for LoadReportError {
    fn from(error: serde_json::Error) -> Self {
        Self::Json(error)
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

    /// Return a machine-readable snapshot for tracked containers.
    pub fn snapshot(&self) -> ArenaReport {
        let inner = self.inner.borrow();
        let totals = ArenaTotals {
            total_len: inner.records.iter().map(|record| record.len).sum(),
            total_current_capacity: inner.records.iter().map(|record| record.capacity).sum(),
            total_growth_events: inner
                .records
                .iter()
                .map(|record| record.growth_events)
                .sum(),
            total_pushed_appended_operations: inner
                .records
                .iter()
                .map(|record| record.total_operations)
                .sum(),
        };

        let containers = inner
            .records
            .iter()
            .map(|record| ContainerReport {
                name: record.name.clone(),
                kind: record.kind.as_str().to_owned(),
                len: record.len,
                initial_capacity: record.initial_capacity,
                current_capacity: record.capacity,
                growth_events: record.growth_events,
                operation_label: record.operation_label.to_owned(),
                total_operations: record.total_operations,
                extra_metric_label: record.extra_metric_label.map(str::to_owned),
                extra_metric_value: record.extra_metric_label.map(|_| record.extra_metric_value),
            })
            .collect();

        ArenaReport {
            arena_name: inner.name.clone(),
            tracked_container_count: inner.records.len(),
            totals,
            containers,
        }
    }

    /// Return a pretty JSON allocation and growth report for tracked containers.
    pub fn report_json(&self) -> String {
        serde_json::to_string_pretty(&self.snapshot())
            .expect("serializing an ArenaReport should not fail")
    }

    /// Write the current pretty JSON report to a programmer-provided path.
    ///
    /// This method is explicit opt-in persistence: RIG never calls it automatically.
    /// It creates or overwrites the target file, but it does not create missing
    /// parent directories.
    pub fn write_json(&self, path: impl AsRef<Path>) -> std::io::Result<()> {
        fs::write(path, self.report_json())
    }

    /// Alias for [`Arena::write_json`].
    ///
    /// `write_json` already writes pretty JSON because RIG report JSON is intended
    /// for inspection. This alias exists only to make that behavior explicit.
    pub fn write_json_pretty(&self, path: impl AsRef<Path>) -> std::io::Result<()> {
        self.write_json(path)
    }

    /// Load a previously persisted arena report from JSON on disk.
    pub fn load_report(path: impl AsRef<Path>) -> Result<ArenaReport, LoadReportError> {
        let json = fs::read_to_string(path)?;
        let report = serde_json::from_str(&json)?;
        Ok(report)
    }

    /// Return a human-readable allocation and growth report for tracked containers.
    pub fn report(&self) -> String {
        let snapshot = self.snapshot();
        let mut report = format!(
            "RIG allocation report\nArena: {}\nTracked containers: {}\nTotals:\n  total len: {}\n  total current capacity: {}\n  total growth events: {}\n  total pushed/appended operations: {}\nContainers:",
            snapshot.arena_name,
            snapshot.tracked_container_count,
            snapshot.totals.total_len,
            snapshot.totals.total_current_capacity,
            snapshot.totals.total_growth_events,
            snapshot.totals.total_pushed_appended_operations
        );

        if snapshot.containers.is_empty() {
            report.push_str("\n  (none)");
            return report;
        }

        for record in &snapshot.containers {
            report.push_str(&format!(
                "\n  Container: {}\n  kind: {}\n  fields:\n    len: {}\n    initial capacity: {}\n    current capacity: {}\n    growth events: {}\n    {}: {}",
                record.name,
                record.kind,
                record.len,
                record.initial_capacity,
                record.current_capacity,
                record.growth_events,
                record.operation_label,
                record.total_operations
            ));

            if let (Some(label), Some(value)) =
                (&record.extra_metric_label, record.extra_metric_value)
            {
                report.push_str(&format!("\n    {}: {}", label, value));
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
