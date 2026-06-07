//! RIG makes allocation growth visible for a small set of Rust containers.
//!
//! The crate is intentionally explicit: creating an [`Arena`], pushing into
//! tracked containers, taking snapshots, rendering reports, and computing diffs
//! are all in-memory operations. Files are written only when the caller invokes
//! a `write_json` method with a path.
//!
//! # Arena and `RigVec`
//!
//! ```
//! use rig::{Arena, RigVec};
//!
//! let mut arena = Arena::new("request");
//! let mut users = RigVec::with_capacity(&mut arena, "users", 2);
//!
//! users.push(1);
//! users.push(2);
//! users.push(3);
//!
//! let snapshot = arena.snapshot();
//! assert_eq!(snapshot.arena_name, "request");
//! assert_eq!(snapshot.tracked_container_count, 1);
//! assert_eq!(snapshot.containers[0].name, "users");
//! assert_eq!(snapshot.containers[0].kind, "RigVec");
//! assert_eq!(snapshot.containers[0].len, 3);
//! assert_eq!(users.total_pushed(), 3);
//! ```
//!
//! # `RigString`
//!
//! ```
//! use rig::{Arena, RigString};
//!
//! let mut arena = Arena::new("audit");
//! let mut events = RigString::with_capacity(&mut arena, "events", 8);
//!
//! events.push_str("login");
//! events.push_str(";ok");
//!
//! assert_eq!(events.len(), "login;ok".len());
//! assert_eq!(events.append_operations(), 2);
//! assert_eq!(events.total_appended_bytes(), "login;ok".len());
//! assert_eq!(arena.snapshot().containers[0].kind, "RigString");
//! ```
//!
//! # Snapshots and JSON reports
//!
//! ```
//! use rig::{Arena, RigVec};
//!
//! let mut arena = Arena::new("json");
//! let mut jobs = RigVec::new(&mut arena, "jobs");
//! jobs.push(42);
//!
//! let snapshot = arena.snapshot();
//! let json = snapshot.report_json();
//! let decoded: rig::ArenaReport = serde_json::from_str(&json).unwrap();
//!
//! assert_eq!(decoded, snapshot);
//! assert!(json.contains("json"));
//! ```
//!
//! # Explicit persistence
//!
//! ```
//! use rig::{Arena, RigVec};
//! use std::fs;
//!
//! let mut arena = Arena::new("persist");
//! let mut jobs = RigVec::new(&mut arena, "jobs");
//! jobs.push(7);
//!
//! let mut path = std::env::temp_dir();
//! path.push(format!("rig-doctest-{}-report.json", std::process::id()));
//! let _ = fs::remove_file(&path);
//!
//! arena.write_json(&path).unwrap();
//! let loaded = Arena::load_report(&path).unwrap();
//!
//! assert_eq!(loaded, arena.snapshot());
//! assert!(path.exists());
//!
//! fs::remove_file(&path).unwrap();
//! ```
//!
//! # Diffs
//!
//! ```
//! use rig::{Arena, RigVec};
//!
//! let mut arena = Arena::new("diff");
//! let mut jobs = RigVec::with_capacity(&mut arena, "jobs", 2);
//! jobs.push(1);
//! let before = arena.snapshot();
//!
//! jobs.push(2);
//! jobs.push(3);
//! let after = arena.snapshot();
//!
//! let diff = before.diff(&after);
//! assert_eq!(diff.total_len_delta, 2);
//! assert_eq!(diff.containers_changed[0].name, "jobs");
//! assert_eq!(diff.containers_changed[0].operation_delta, 2);
//!
//! let decoded: rig::ArenaDiff = serde_json::from_str(&diff.diff_json()).unwrap();
//! assert_eq!(decoded, diff);
//! ```

#![warn(missing_docs)]

use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::error::Error;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};
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

/// Explicit export encoding for portable RIG evidence.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportFormat {
    /// Comma-separated values with a header row.
    Csv,
    /// Newline-delimited JSON with one JSON object per line.
    JsonLines,
}

/// Explicit caller-requested evidence export.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvidenceExport {
    /// Encoding used by this export.
    pub format: ExportFormat,
    /// Stable evidence category name for this export.
    pub kind: String,
    /// Exact exported bytes represented as UTF-8 text.
    pub contents: String,
}

/// Capacity reservation strategy used by tracked RIG containers before growth.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GrowthPolicy {
    /// Preserve standard Rust `Vec` and `String` growth behavior.
    RustDefault,
    /// Reserve before growth so the requested capacity is at least double the current capacity.
    Double,
    /// Reserve exactly enough additional capacity for the requested operation.
    Exact,
    /// Reserve enough capacity for the requested operation plus the configured spare capacity.
    ReserveAhead(usize),
    /// Refuse operations that would require capacity above `max_capacity`.
    Capped {
        /// Maximum capacity this tracked container may grow to through fallible operations.
        max_capacity: usize,
    },
}

impl GrowthPolicy {
    fn report_name(&self) -> String {
        match self {
            Self::RustDefault => "RustDefault".to_owned(),
            Self::Double => "Double".to_owned(),
            Self::Exact => "Exact".to_owned(),
            Self::ReserveAhead(amount) => format!("ReserveAhead({amount})"),
            Self::Capped { max_capacity } => format!("Capped(max_capacity={max_capacity})"),
        }
    }

    fn checked_target(
        &self,
        container_name: &str,
        current_capacity: usize,
        requested_len: usize,
    ) -> Result<Option<usize>, RigError> {
        if requested_len <= current_capacity {
            return Ok(None);
        }

        match self {
            Self::RustDefault => Ok(None),
            Self::Double => Ok(Some(
                current_capacity.saturating_mul(2).max(4).max(requested_len),
            )),
            Self::Exact => Ok(Some(requested_len)),
            Self::ReserveAhead(amount) => Ok(Some(requested_len.saturating_add(*amount))),
            Self::Capped { max_capacity } => {
                if requested_len > *max_capacity {
                    Err(RigError::CapacityLimitExceeded {
                        container_name: container_name.to_owned(),
                        requested_capacity: requested_len,
                        max_capacity: *max_capacity,
                    })
                } else {
                    Ok(Some(requested_len))
                }
            }
        }
    }
}

/// Typed RIG operation errors returned by fallible container APIs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RigError {
    /// A capped growth policy refused an operation that would exceed its limit.
    CapacityLimitExceeded {
        /// Human-readable container name supplied by the caller.
        container_name: String,
        /// Capacity needed by the refused operation.
        requested_capacity: usize,
        /// Maximum capacity allowed by the container policy.
        max_capacity: usize,
    },
}

impl fmt::Display for RigError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CapacityLimitExceeded {
                container_name,
                requested_capacity,
                max_capacity,
            } => write!(
                formatter,
                "CapacityLimitExceeded: container '{container_name}' requested capacity {requested_capacity}, but capped max_capacity is {max_capacity}"
            ),
        }
    }
}

impl Error for RigError {}

impl EvidenceExport {
    /// Write exactly this export's contents to an explicit caller-provided path.
    ///
    /// This method does not create missing parent directories and does not
    /// create hidden files unless the caller's path itself names a hidden file.
    pub fn write_to<P: AsRef<Path>>(&self, path: P) -> Result<(), RigIoError> {
        fs::write(path, &self.contents)?;
        Ok(())
    }
}

/// One observed live capacity-growth event for a tracked container.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GrowthEvent {
    /// Human-readable container name supplied by the caller.
    pub container_name: String,
    /// RIG container wrapper kind, such as `RigVec` or `RigString`.
    pub container_kind: String,
    /// Capacity observed immediately before the operation.
    pub old_capacity: usize,
    /// Capacity observed immediately after the operation.
    pub new_capacity: usize,
    /// Operation count after the operation that caused growth.
    pub operation_index: usize,
    /// Capacity added by this event (`new_capacity - old_capacity`).
    pub capacity_added: usize,
    /// Human-readable growth policy active when this event was observed.
    pub growth_policy: String,
}

/// Causal attribution for one observed container growth event.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GrowthAttribution {
    /// Human-readable container name supplied by the caller.
    pub container_name: String,
    /// Operation count after the operation that caused growth.
    pub operation_index: usize,
    /// Capacity observed immediately before the operation.
    pub old_capacity: usize,
    /// Capacity observed immediately after the operation.
    pub new_capacity: usize,
    /// Capacity added by the growth event.
    pub capacity_added: usize,
    /// Human-readable growth policy active when this event was observed.
    pub growth_policy: String,
}

impl From<&GrowthEvent> for GrowthAttribution {
    fn from(event: &GrowthEvent) -> Self {
        Self {
            container_name: event.container_name.clone(),
            operation_index: event.operation_index,
            old_capacity: event.old_capacity,
            new_capacity: event.new_capacity,
            capacity_added: event.capacity_added,
            growth_policy: event.growth_policy.clone(),
        }
    }
}

/// Compact machine-readable summary derived from raw [`GrowthEvent`] evidence.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GrowthSummary {
    /// Total number of raw growth events summarized.
    pub total_growth_events: usize,
    /// Number of containers that have at least one growth event.
    pub containers_with_growth: usize,
    /// Largest single-event capacity increase (`new_capacity - old_capacity`).
    pub largest_growth_delta: usize,
    /// Container name for the largest single-event capacity increase, if any.
    pub largest_growth_container: Option<String>,
    /// First growth event in the raw growth history, if any.
    pub first_growth_event: Option<GrowthEvent>,
    /// Last growth event in the raw growth history, if any.
    pub last_growth_event: Option<GrowthEvent>,
    /// Per-container summaries derived from the same raw growth history.
    pub per_container: Vec<ContainerGrowthSummary>,
}

/// Compact growth evidence summary for one container.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContainerGrowthSummary {
    /// Human-readable container name supplied by the caller.
    pub container_name: String,
    /// RIG container wrapper kind, such as `RigVec` or `RigString`.
    pub container_kind: String,
    /// Number of raw growth events for this container.
    pub growth_events: usize,
    /// Capacity before this container's first growth event.
    pub first_old_capacity: usize,
    /// Capacity after this container's final growth event.
    pub final_new_capacity: usize,
    /// Largest single-event capacity increase for this container.
    pub largest_growth_delta: usize,
    /// Operation index for this container's first growth event.
    pub first_operation_index: usize,
    /// Operation index for this container's last growth event.
    pub last_operation_index: usize,
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
    /// Human-readable growth policy used by this tracked container.
    pub growth_policy: String,
    /// Current underlying Rust container capacity.
    pub current_capacity: usize,
    /// Number of operations that caused capacity to increase.
    pub growth_events: usize,
    /// Total capacity added across this container's observed growth events.
    pub total_capacity_added: usize,
    /// Largest single observed capacity increase for this container.
    pub largest_growth_jump: usize,
    /// Integer average observed capacity increase for this container.
    pub average_growth_jump: usize,
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
    /// Observed live capacity-growth events for tracked containers.
    pub growth_history: Vec<GrowthEvent>,
    /// Causal attribution records derived from observed live growth events.
    pub growth_attributions: Vec<GrowthAttribution>,
}

/// Deterministic memory behavior profile names derived from observed RIG evidence.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MemoryProfileKind {
    /// No observed growth events were present in the profiled evidence.
    Stable,
    /// Capacity growth is concentrated in one container.
    BurstGrowth,
    /// Many observed growth events had a small average capacity jump.
    FrequentTinyGrowth,
    /// One observed growth event added a large amount of capacity.
    LargeSingleJump,
    /// Current capacity is much larger than current logical length.
    OverReserved,
    /// Growth-event count is high relative to logical length.
    UnderReserved,
    /// A memory budget gate failed.
    BudgetRisk,
    /// A regression gate or comparison risk signal failed.
    RegressionRisk,
}

impl MemoryProfileKind {
    fn as_str(self) -> &'static str {
        match self {
            Self::Stable => "Stable",
            Self::BurstGrowth => "BurstGrowth",
            Self::FrequentTinyGrowth => "FrequentTinyGrowth",
            Self::LargeSingleJump => "LargeSingleJump",
            Self::OverReserved => "OverReserved",
            Self::UnderReserved => "UnderReserved",
            Self::BudgetRisk => "BudgetRisk",
            Self::RegressionRisk => "RegressionRisk",
        }
    }
}

impl fmt::Display for MemoryProfileKind {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// One deterministic profile finding with the exact evidence metric that triggered it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemoryProfile {
    /// Profile category assigned from observed evidence.
    pub kind: MemoryProfileKind,
    /// Arena, container, gate, or comparison subject for this finding.
    pub subject: String,
    /// Deterministic explanation of the threshold comparison.
    pub reason: String,
    /// Name of the observed metric used as evidence.
    pub evidence_metric: String,
    /// Observed metric value.
    pub evidence_value: usize,
    /// Deterministic threshold used for this profile.
    pub threshold: usize,
}

/// In-memory profile report derived from observed RIG evidence.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProfileReport {
    /// Deterministic profile findings.
    pub profiles: Vec<MemoryProfile>,
}

impl ProfileReport {
    /// Return a human-readable evidence profile report.
    pub fn report(&self) -> String {
        self.to_string()
    }

    /// Serialize this profile report as pretty JSON.
    ///
    /// This is an in-memory operation and does not write files.
    pub fn report_json(&self) -> String {
        serde_json::to_string_pretty(self).expect("serializing a ProfileReport should not fail")
    }

    /// Return all profiles with the requested kind, preserving report order.
    pub fn profiles_by_kind(&self, kind: MemoryProfileKind) -> Vec<&MemoryProfile> {
        self.profiles
            .iter()
            .filter(|profile| profile.kind == kind)
            .collect()
    }
}

impl fmt::Display for ProfileReport {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(formatter, "RIG evidence profile report")?;
        writeln!(formatter, "Profiles: {}", self.profiles.len())?;
        if self.profiles.is_empty() {
            write!(formatter, "  (none)")?;
        } else {
            for (index, profile) in self.profiles.iter().enumerate() {
                writeln!(formatter, "{}. {}", index + 1, profile.kind)?;
                writeln!(formatter, "   subject: {}", profile.subject)?;
                writeln!(formatter, "   reason: {}", profile.reason)?;
                writeln!(formatter, "   evidence metric: {}", profile.evidence_metric)?;
                writeln!(formatter, "   evidence value: {}", profile.evidence_value)?;
                write!(formatter, "   threshold: {}", profile.threshold)?;
                if index + 1 < self.profiles.len() {
                    writeln!(formatter)?;
                    writeln!(formatter)?;
                }
            }
        }
        Ok(())
    }
}

const FREQUENT_TINY_GROWTH_MIN_EVENTS: usize = 8;
const FREQUENT_TINY_GROWTH_MAX_AVERAGE_JUMP: usize = 4;
const LARGE_SINGLE_JUMP_MIN_CAPACITY_ADDED: usize = 1024;
const BURST_GROWTH_MIN_TOTAL_CAPACITY_ADDED: usize = 16;
const BURST_GROWTH_MIN_TOP_CONTAINER_PERCENT: usize = 80;
const OVER_RESERVED_MIN_CAPACITY: usize = 16;
const OVER_RESERVED_CAPACITY_TO_LEN_RATIO: usize = 4;
const UNDER_RESERVED_MIN_GROWTH_EVENTS: usize = 8;
const UNDER_RESERVED_MAX_LEN_PER_GROWTH_EVENT: usize = 4;

/// A saved RIG report artifact loaded from, or written to, an explicit caller-provided path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReportArtifact {
    /// Caller-provided path for the JSON report artifact.
    pub path: PathBuf,
    /// Arena report evidence stored in the artifact.
    pub report: ArenaReport,
}

/// Evidence comparison between two explicitly saved report artifacts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArtifactComparison {
    /// Path to the baseline JSON report artifact.
    pub baseline_path: PathBuf,
    /// Path to the current JSON report artifact.
    pub current_path: PathBuf,
    /// Baseline arena report evidence loaded from disk.
    pub baseline: ArenaReport,
    /// Current arena report evidence loaded from disk.
    pub current: ArenaReport,
    /// Allocation diff derived from the baseline and current reports.
    pub diff: ArenaDiff,
}

#[derive(Debug, Serialize)]
struct ArtifactComparisonJson<'a> {
    baseline_path: &'a Path,
    current_path: &'a Path,
    baseline_arena_name: &'a str,
    current_arena_name: &'a str,
    diff: &'a ArenaDiff,
}

#[derive(Debug, Serialize)]
struct ArtifactComparisonSummary<'a> {
    baseline_path: String,
    current_path: String,
    baseline_arena_name: &'a str,
    current_arena_name: &'a str,
    total_len_delta: i64,
    total_capacity_delta: i64,
    total_growth_event_delta: i64,
    total_operation_delta: i64,
    containers_added: usize,
    containers_removed: usize,
    containers_changed: usize,
    growth_events_added: usize,
}

impl ArtifactComparisonSummary<'_> {
    fn csv_header() -> &'static str {
        "baseline_path,current_path,baseline_arena_name,current_arena_name,total_len_delta,total_capacity_delta,total_growth_event_delta,total_operation_delta,containers_added,containers_removed,containers_changed,growth_events_added\n"
    }

    fn csv_row(&self) -> String {
        csv_record([
            self.baseline_path.clone(),
            self.current_path.clone(),
            self.baseline_arena_name.to_owned(),
            self.current_arena_name.to_owned(),
            self.total_len_delta.to_string(),
            self.total_capacity_delta.to_string(),
            self.total_growth_event_delta.to_string(),
            self.total_operation_delta.to_string(),
            self.containers_added.to_string(),
            self.containers_removed.to_string(),
            self.containers_changed.to_string(),
            self.growth_events_added.to_string(),
        ])
    }
}

/// Named, typed declaration of expected memory behavior for one workload.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkloadContract {
    /// Human-readable contract name supplied by the caller.
    pub name: String,
    /// Human-readable contract description supplied by the caller.
    pub description: String,
    /// Optional memory budget checked against the current arena report.
    pub budget: Option<MemoryBudget>,
    /// Optional regression budget checked against baseline/current evidence.
    pub regression_budget: Option<RegressionBudget>,
    /// Profile kinds that must be absent from current profile evidence.
    pub required_profiles_absent: Vec<MemoryProfileKind>,
    /// Profile kinds that must be present in current profile evidence.
    pub required_profiles_present: Vec<MemoryProfileKind>,
}

/// Typed evidence for one workload-contract rule failure.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContractViolation {
    /// Contract name whose rule failed.
    pub contract_name: String,
    /// Rule category that failed, such as `budget`, `regression`, `profile_absent`, or `profile_present`.
    pub rule: String,
    /// Arena, container, profile subject, or comparison subject for the failed rule.
    pub subject: String,
    /// Deterministic explanation of the failed rule.
    pub reason: String,
    /// Exact observed evidence and configured threshold used for the failure.
    pub evidence: String,
}

/// In-memory result of validating one [`WorkloadContract`] against observed RIG evidence.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContractReport {
    /// Contract name that was evaluated.
    pub contract_name: String,
    /// Whether every explicit contract rule passed.
    pub passed: bool,
    /// Typed evidence for every failed contract rule.
    pub violations: Vec<ContractViolation>,
    /// Budget gate evidence, present only when the contract included a memory budget.
    pub budget_report: Option<BudgetReport>,
    /// Regression gate evidence, present only when artifact comparison evaluated a regression budget.
    pub regression_report: Option<RegressionReport>,
    /// Profile evidence, present only when the contract included profile requirements.
    pub profile_report: Option<ProfileReport>,
}

/// Explicit memory behavior budget checked against one observed [`ArenaReport`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemoryBudget {
    /// Maximum allowed aggregate current length, or `None` to skip this gate.
    pub max_total_len: Option<usize>,
    /// Maximum allowed aggregate current capacity, or `None` to skip this gate.
    pub max_total_capacity: Option<usize>,
    /// Maximum allowed aggregate growth-event count, or `None` to skip this gate.
    pub max_total_growth_events: Option<usize>,
    /// Maximum allowed aggregate push/append operation count, or `None` to skip this gate.
    pub max_total_operations: Option<usize>,
    /// Maximum allowed current length for each container, or `None` to skip this gate.
    pub max_container_len: Option<usize>,
    /// Maximum allowed current capacity for each container, or `None` to skip this gate.
    pub max_container_capacity: Option<usize>,
    /// Maximum allowed growth-event count for each container, or `None` to skip this gate.
    pub max_container_growth_events: Option<usize>,
    /// Maximum allowed push/append operation count for each container, or `None` to skip this gate.
    pub max_container_operations: Option<usize>,
}

impl WorkloadContract {
    /// Create a named workload contract with no rules.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: String::new(),
            budget: None,
            regression_budget: None,
            required_profiles_absent: Vec::new(),
            required_profiles_present: Vec::new(),
        }
    }

    /// Set a human-readable description for this workload contract.
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = description.into();
        self
    }

    /// Require the current report to satisfy the provided memory budget.
    pub fn with_budget(mut self, budget: MemoryBudget) -> Self {
        self.budget = Some(budget);
        self
    }

    /// Require a comparison to satisfy the provided regression budget.
    pub fn with_regression_budget(mut self, budget: RegressionBudget) -> Self {
        self.regression_budget = Some(budget);
        self
    }

    /// Require the current profile report not to contain `kind`.
    pub fn require_profile_absent(mut self, kind: MemoryProfileKind) -> Self {
        self.required_profiles_absent.push(kind);
        self
    }

    /// Require the current profile report to contain `kind`.
    pub fn require_profile_present(mut self, kind: MemoryProfileKind) -> Self {
        self.required_profiles_present.push(kind);
        self
    }

    fn has_profile_rules(&self) -> bool {
        !self.required_profiles_absent.is_empty() || !self.required_profiles_present.is_empty()
    }
}

impl ContractReport {
    /// Return a human-readable workload contract report.
    pub fn report(&self) -> String {
        self.to_string()
    }

    /// Serialize this contract report as pretty JSON.
    ///
    /// This is an in-memory operation and does not write files.
    pub fn report_json(&self) -> String {
        serde_json::to_string_pretty(self).expect("serializing a ContractReport should not fail")
    }
}

impl fmt::Display for ContractReport {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(formatter, "RIG workload contract report")?;
        writeln!(formatter, "Contract: {}", self.contract_name)?;
        writeln!(
            formatter,
            "Status: {}",
            if self.passed { "PASSED" } else { "FAILED" }
        )?;
        writeln!(formatter)?;
        writeln!(formatter, "Violations:")?;
        if self.violations.is_empty() {
            write!(formatter, "  (none)")?;
        } else {
            for (index, violation) in self.violations.iter().enumerate() {
                writeln!(formatter, "{}. {}", index + 1, violation.rule)?;
                writeln!(formatter, "   subject: {}", violation.subject)?;
                writeln!(formatter, "   reason: {}", violation.reason)?;
                write!(formatter, "   evidence: {}", violation.evidence)?;
                if index + 1 < self.violations.len() {
                    writeln!(formatter)?;
                    writeln!(formatter)?;
                }
            }
        }
        Ok(())
    }
}

impl MemoryBudget {
    /// Return a budget with no active limits.
    pub fn unlimited() -> Self {
        Self {
            max_total_len: None,
            max_total_capacity: None,
            max_total_growth_events: None,
            max_total_operations: None,
            max_container_len: None,
            max_container_capacity: None,
            max_container_growth_events: None,
            max_container_operations: None,
        }
    }

    /// Return a budget that allows no observed growth events at arena or container scope.
    pub fn strict_zero_growth() -> Self {
        Self::unlimited()
            .with_max_total_growth_events(0)
            .with_max_container_growth_events(0)
    }

    /// Return a budget with an aggregate current-capacity limit.
    pub fn max_total_capacity(value: usize) -> Self {
        Self::unlimited().with_max_total_capacity(value)
    }

    /// Return a budget with an aggregate growth-event limit.
    pub fn max_total_growth_events(value: usize) -> Self {
        Self::unlimited().with_max_total_growth_events(value)
    }

    /// Return a budget with a per-container current-capacity limit.
    pub fn max_container_capacity(value: usize) -> Self {
        Self::unlimited().with_max_container_capacity(value)
    }

    /// Return a budget with a per-container growth-event limit.
    pub fn max_container_growth_events(value: usize) -> Self {
        Self::unlimited().with_max_container_growth_events(value)
    }

    /// Set the aggregate current-length limit.
    pub fn with_max_total_len(mut self, value: usize) -> Self {
        self.max_total_len = Some(value);
        self
    }

    /// Set the aggregate current-capacity limit.
    pub fn with_max_total_capacity(mut self, value: usize) -> Self {
        self.max_total_capacity = Some(value);
        self
    }

    /// Set the aggregate growth-event limit.
    pub fn with_max_total_growth_events(mut self, value: usize) -> Self {
        self.max_total_growth_events = Some(value);
        self
    }

    /// Set the aggregate push/append operation limit.
    pub fn with_max_total_operations(mut self, value: usize) -> Self {
        self.max_total_operations = Some(value);
        self
    }

    /// Set the per-container current-length limit.
    pub fn with_max_container_len(mut self, value: usize) -> Self {
        self.max_container_len = Some(value);
        self
    }

    /// Set the per-container current-capacity limit.
    pub fn with_max_container_capacity(mut self, value: usize) -> Self {
        self.max_container_capacity = Some(value);
        self
    }

    /// Set the per-container growth-event limit.
    pub fn with_max_container_growth_events(mut self, value: usize) -> Self {
        self.max_container_growth_events = Some(value);
        self
    }

    /// Set the per-container push/append operation limit.
    pub fn with_max_container_operations(mut self, value: usize) -> Self {
        self.max_container_operations = Some(value);
        self
    }
}

/// Typed evidence for one memory-budget limit exceeded by observed report data.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BudgetViolation {
    /// Violation scope, currently `arena` or `container`.
    pub scope: String,
    /// Container name for container-scoped violations.
    pub container_name: Option<String>,
    /// Metric that exceeded its budget limit.
    pub metric: String,
    /// Observed value from [`ArenaReport`] or [`ContainerReport`] evidence.
    pub observed: usize,
    /// Configured budget limit.
    pub limit: usize,
    /// Positive amount by which `observed` exceeded `limit`.
    pub exceeded_by: usize,
}

/// Machine-readable result of checking one report against a [`MemoryBudget`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BudgetReport {
    /// Whether all configured budget gates passed.
    pub passed: bool,
    /// Typed evidence for every failed budget gate.
    pub violations: Vec<BudgetViolation>,
}

/// Typed evidence for one memory regression that exceeded a configured budget.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemoryRegression {
    /// Container name for per-container regressions, or `total` for aggregate regressions.
    pub container_name: String,
    /// Metric that regressed, such as `current_capacity` or `growth_events`.
    pub metric: String,
    /// Baseline value from the previous [`ArenaReport`].
    pub baseline: usize,
    /// Current value from the report being checked.
    pub current: usize,
    /// Positive observed increase from `baseline` to `current`.
    pub delta: usize,
    /// Increase allowed by the configured [`RegressionBudget`].
    pub allowed_delta: usize,
}

/// Configurable memory-regression budget used when comparing two reports.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RegressionBudget {
    /// Maximum allowed aggregate capacity increase, or `None` to skip that gate.
    pub max_total_capacity_delta: Option<usize>,
    /// Maximum allowed aggregate growth-event increase, or `None` to skip that gate.
    pub max_total_growth_events_delta: Option<usize>,
    /// Maximum allowed per-container capacity increase, or `None` to skip that gate.
    pub max_container_capacity_delta: Option<usize>,
    /// Maximum allowed per-container growth-event increase, or `None` to skip that gate.
    pub max_container_growth_events_delta: Option<usize>,
}

impl RegressionBudget {
    /// Return a budget that permits no capacity or growth-event increase.
    pub fn strict() -> Self {
        Self {
            max_total_capacity_delta: Some(0),
            max_total_growth_events_delta: Some(0),
            max_container_capacity_delta: Some(0),
            max_container_growth_events_delta: Some(0),
        }
    }

    /// Return a budget that permits `delta` capacity growth globally and per container.
    ///
    /// Growth-event gates remain strict.
    pub fn allow_capacity_delta(delta: usize) -> Self {
        Self {
            max_total_capacity_delta: Some(delta),
            max_total_growth_events_delta: Some(0),
            max_container_capacity_delta: Some(delta),
            max_container_growth_events_delta: Some(0),
        }
    }

    /// Return a budget that permits `delta` growth-event increase globally and per container.
    ///
    /// Capacity gates remain strict.
    pub fn allow_growth_events_delta(delta: usize) -> Self {
        Self {
            max_total_capacity_delta: Some(0),
            max_total_growth_events_delta: Some(delta),
            max_container_capacity_delta: Some(0),
            max_container_growth_events_delta: Some(delta),
        }
    }
}

/// Machine-readable result of checking one report against another.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RegressionReport {
    /// Whether all configured regression gates passed.
    pub passed: bool,
    /// Typed evidence for every failed gate.
    pub regressions: Vec<MemoryRegression>,
    /// Signed aggregate capacity delta from baseline to current.
    pub total_capacity_delta: isize,
    /// Signed aggregate growth-event delta from baseline to current.
    pub total_growth_event_delta: isize,
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
    /// Growth events present in the later report but not in the earlier report.
    pub growth_events_added: Vec<GrowthEvent>,
    /// Deltas for every container present in both reports.
    pub containers_changed: Vec<ContainerDiff>,
}

impl ReportArtifact {
    /// Load exactly one saved [`ArenaReport`] JSON artifact from a caller-provided path.
    pub fn load<P: AsRef<Path>>(path: P) -> Result<ReportArtifact, RigIoError> {
        let path = path.as_ref();
        let json = fs::read_to_string(path)?;
        let report = serde_json::from_str(&json)?;

        Ok(ReportArtifact {
            path: path.to_path_buf(),
            report,
        })
    }

    /// Compare this baseline artifact to a current artifact without writing files.
    pub fn compare_to(&self, current: &ReportArtifact) -> ArtifactComparison {
        ArtifactComparison {
            baseline_path: self.path.clone(),
            current_path: current.path.clone(),
            baseline: self.report.clone(),
            current: current.report.clone(),
            diff: self.report.diff(&current.report),
        }
    }
}

impl ArtifactComparison {
    /// Run memory regression gates using the current artifact report against the baseline artifact report.
    pub fn regression_report(&self, budget: &RegressionBudget) -> RegressionReport {
        self.current
            .check_regressions_against(&self.baseline, budget)
    }

    /// Run memory budget gates using only the current artifact report.
    pub fn budget_report(&self, budget: &MemoryBudget) -> BudgetReport {
        self.current.check_budget(budget)
    }

    /// Check this artifact comparison against an explicit workload contract.
    ///
    /// The current artifact report is used for budget and profile rules. The
    /// baseline/current artifact reports are used for regression rules. No
    /// files are written by this method.
    pub fn check_contract(&self, contract: &WorkloadContract) -> ContractReport {
        let budget_report = contract
            .budget
            .as_ref()
            .map(|budget| self.current.check_budget(budget));
        let regression_report = contract
            .regression_budget
            .as_ref()
            .map(|budget| self.regression_report(budget));
        let profile_report = contract.has_profile_rules().then(|| self.current.profile());

        build_contract_report(contract, budget_report, regression_report, profile_report)
    }

    /// Classify the current artifact report and positive comparison deltas as evidence profiles.
    pub fn profile(&self) -> ProfileReport {
        let mut profiles = self.current.profile().profiles;
        if self.diff.total_capacity_delta > 0 {
            let value = self.diff.total_capacity_delta as usize;
            profiles.push(MemoryProfile {
                kind: MemoryProfileKind::RegressionRisk,
                subject: format!(
                    "artifact comparison {} -> {}",
                    self.baseline_path.display(),
                    self.current_path.display()
                ),
                reason: format!(
                    "current artifact total capacity increased by {value}; threshold is 0 positive capacity delta"
                ),
                evidence_metric: "diff.total_capacity_delta".to_owned(),
                evidence_value: value,
                threshold: 0,
            });
        }
        if self.diff.total_growth_event_delta > 0 {
            let value = self.diff.total_growth_event_delta as usize;
            profiles.push(MemoryProfile {
                kind: MemoryProfileKind::RegressionRisk,
                subject: format!(
                    "artifact comparison {} -> {}",
                    self.baseline_path.display(),
                    self.current_path.display()
                ),
                reason: format!(
                    "current artifact growth events increased by {value}; threshold is 0 positive growth-event delta"
                ),
                evidence_metric: "diff.total_growth_event_delta".to_owned(),
                evidence_value: value,
                threshold: 0,
            });
        }
        ProfileReport { profiles }
    }

    /// Return a human-readable artifact comparison report.
    pub fn report(&self) -> String {
        format!(
            "RIG report artifact comparison\nBaseline: {}\nCurrent: {}\n\nDiff: {}\n\nRegression gate:\nNot evaluated by artifact comparison report.\n\nBudget gate:\nNot evaluated by artifact comparison report.",
            self.baseline_path.display(),
            self.current_path.display(),
            self.diff.report()
        )
    }

    /// Return compact JSON evidence for this artifact comparison.
    pub fn report_json(&self) -> String {
        let evidence = ArtifactComparisonJson {
            baseline_path: &self.baseline_path,
            current_path: &self.current_path,
            baseline_arena_name: &self.baseline.arena_name,
            current_arena_name: &self.current.arena_name,
            diff: &self.diff,
        };

        serde_json::to_string_pretty(&evidence)
            .expect("serializing an ArtifactComparison should not fail")
    }

    /// Export artifact comparison summary evidence as CSV.
    pub fn summary_csv(&self) -> String {
        let mut csv = String::from(ArtifactComparisonSummary::csv_header());
        csv.push_str(&self.summary_evidence().csv_row());
        csv.push('\n');
        csv
    }

    /// Export artifact comparison summary evidence as JSON Lines.
    pub fn summary_jsonl(&self) -> String {
        jsonl_lines(std::iter::once(self.summary_evidence()))
    }

    /// Export artifact comparison summary evidence in the requested format.
    pub fn export_summary(&self, format: ExportFormat) -> EvidenceExport {
        EvidenceExport {
            format,
            kind: "artifact_comparison_summary".to_owned(),
            contents: match format {
                ExportFormat::Csv => self.summary_csv(),
                ExportFormat::JsonLines => self.summary_jsonl(),
            },
        }
    }

    fn summary_evidence(&self) -> ArtifactComparisonSummary<'_> {
        ArtifactComparisonSummary {
            baseline_path: self.baseline_path.display().to_string(),
            current_path: self.current_path.display().to_string(),
            baseline_arena_name: &self.baseline.arena_name,
            current_arena_name: &self.current.arena_name,
            total_len_delta: self.diff.total_len_delta,
            total_capacity_delta: self.diff.total_capacity_delta,
            total_growth_event_delta: self.diff.total_growth_event_delta,
            total_operation_delta: self.diff.total_operation_delta,
            containers_added: self.diff.containers_added.len(),
            containers_removed: self.diff.containers_removed.len(),
            containers_changed: self.diff.containers_changed.len(),
            growth_events_added: self.diff.growth_events_added.len(),
        }
    }
}

fn profile_arena_report(report: &ArenaReport) -> ProfileReport {
    let mut profiles = Vec::new();

    if report.totals.total_growth_events == 0 {
        profiles.push(MemoryProfile {
            kind: MemoryProfileKind::Stable,
            subject: report.arena_name.clone(),
            reason: "total_growth_events is 0; Stable threshold is exactly 0 growth events"
                .to_owned(),
            evidence_metric: "totals.total_growth_events".to_owned(),
            evidence_value: report.totals.total_growth_events,
            threshold: 0,
        });
    }

    let total_capacity_added: usize = report
        .containers
        .iter()
        .map(|container| container.total_capacity_added)
        .sum();
    let average_growth_jump = if report.totals.total_growth_events == 0 {
        0
    } else {
        total_capacity_added / report.totals.total_growth_events
    };
    if report.totals.total_growth_events >= FREQUENT_TINY_GROWTH_MIN_EVENTS
        && average_growth_jump <= FREQUENT_TINY_GROWTH_MAX_AVERAGE_JUMP
    {
        profiles.push(MemoryProfile {
            kind: MemoryProfileKind::FrequentTinyGrowth,
            subject: report.arena_name.clone(),
            reason: format!(
                "{} growth events with average jump {average_growth_jump}; thresholds are at least {} events and average jump at most {}",
                report.totals.total_growth_events,
                FREQUENT_TINY_GROWTH_MIN_EVENTS,
                FREQUENT_TINY_GROWTH_MAX_AVERAGE_JUMP
            ),
            evidence_metric: "average_growth_jump".to_owned(),
            evidence_value: average_growth_jump,
            threshold: FREQUENT_TINY_GROWTH_MAX_AVERAGE_JUMP,
        });
    }

    if let Some(largest_event) = report
        .growth_history
        .iter()
        .max_by(|left, right| left.capacity_added.cmp(&right.capacity_added))
    {
        if largest_event.capacity_added >= LARGE_SINGLE_JUMP_MIN_CAPACITY_ADDED {
            profiles.push(MemoryProfile {
                kind: MemoryProfileKind::LargeSingleJump,
                subject: largest_event.container_name.clone(),
                reason: format!(
                    "largest observed growth jump added {}; threshold is at least {} capacity units",
                    largest_event.capacity_added, LARGE_SINGLE_JUMP_MIN_CAPACITY_ADDED
                ),
                evidence_metric: "growth_history.capacity_added".to_owned(),
                evidence_value: largest_event.capacity_added,
                threshold: LARGE_SINGLE_JUMP_MIN_CAPACITY_ADDED,
            });
        }
    }

    if total_capacity_added >= BURST_GROWTH_MIN_TOTAL_CAPACITY_ADDED {
        if let Some(top_container) = report
            .containers
            .iter()
            .max_by(|left, right| left.total_capacity_added.cmp(&right.total_capacity_added))
        {
            let top_percent =
                top_container.total_capacity_added.saturating_mul(100) / total_capacity_added;
            if top_percent >= BURST_GROWTH_MIN_TOP_CONTAINER_PERCENT {
                profiles.push(MemoryProfile {
                    kind: MemoryProfileKind::BurstGrowth,
                    subject: top_container.name.clone(),
                    reason: format!(
                        "container contributed {top_percent}% of total capacity added; threshold is at least {}% with total added at least {}",
                        BURST_GROWTH_MIN_TOP_CONTAINER_PERCENT,
                        BURST_GROWTH_MIN_TOTAL_CAPACITY_ADDED
                    ),
                    evidence_metric: "top_container_capacity_added_percent".to_owned(),
                    evidence_value: top_percent,
                    threshold: BURST_GROWTH_MIN_TOP_CONTAINER_PERCENT,
                });
            }
        }
    }

    for container in &report.containers {
        let over_reserved = if container.len == 0 {
            container.current_capacity >= OVER_RESERVED_MIN_CAPACITY
        } else {
            container.current_capacity >= OVER_RESERVED_MIN_CAPACITY
                && container.current_capacity
                    >= container
                        .len
                        .saturating_mul(OVER_RESERVED_CAPACITY_TO_LEN_RATIO)
        };
        if over_reserved {
            profiles.push(MemoryProfile {
                kind: MemoryProfileKind::OverReserved,
                subject: container.name.clone(),
                reason: format!(
                    "current capacity {} is at least {} and at least {}x len {}",
                    container.current_capacity,
                    OVER_RESERVED_MIN_CAPACITY,
                    OVER_RESERVED_CAPACITY_TO_LEN_RATIO,
                    container.len
                ),
                evidence_metric: "current_capacity".to_owned(),
                evidence_value: container.current_capacity,
                threshold: container
                    .len
                    .saturating_mul(OVER_RESERVED_CAPACITY_TO_LEN_RATIO)
                    .max(OVER_RESERVED_MIN_CAPACITY),
            });
        }

        if container.growth_events >= UNDER_RESERVED_MIN_GROWTH_EVENTS && container.len > 0 {
            let len_per_growth_event = container.len / container.growth_events;
            if len_per_growth_event <= UNDER_RESERVED_MAX_LEN_PER_GROWTH_EVENT {
                profiles.push(MemoryProfile {
                    kind: MemoryProfileKind::UnderReserved,
                    subject: container.name.clone(),
                    reason: format!(
                        "{} growth events for len {}; len per growth event {len_per_growth_event}; thresholds are at least {} growth events and at most {} len per event",
                        container.growth_events,
                        container.len,
                        UNDER_RESERVED_MIN_GROWTH_EVENTS,
                        UNDER_RESERVED_MAX_LEN_PER_GROWTH_EVENT
                    ),
                    evidence_metric: "len_per_growth_event".to_owned(),
                    evidence_value: len_per_growth_event,
                    threshold: UNDER_RESERVED_MAX_LEN_PER_GROWTH_EVENT,
                });
            }
        }
    }

    ProfileReport { profiles }
}

impl ArenaReport {
    /// Check this report against an explicit workload contract.
    ///
    /// Budget and profile rules use only values already present in this
    /// [`ArenaReport`] or derived by existing RIG profile logic. Regression
    /// rules require baseline evidence and are therefore ignored for a single
    /// report rather than inferred.
    pub fn check_contract(&self, contract: &WorkloadContract) -> ContractReport {
        let budget_report = contract
            .budget
            .as_ref()
            .map(|budget| self.check_budget(budget));
        let profile_report = contract.has_profile_rules().then(|| self.profile());

        build_contract_report(contract, budget_report, None, profile_report)
    }

    /// Check this report against explicit memory behavior limits.
    ///
    /// Budget checks use only values already present in this [`ArenaReport`]
    /// and its [`ContainerReport`] entries. Missing limits are skipped; missing
    /// data is never inferred or estimated.
    pub fn check_budget(&self, budget: &MemoryBudget) -> BudgetReport {
        let mut violations = Vec::new();

        push_budget_violation_if_over_limit(
            &mut violations,
            "arena",
            None,
            "total_len",
            self.totals.total_len,
            budget.max_total_len,
        );
        push_budget_violation_if_over_limit(
            &mut violations,
            "arena",
            None,
            "total_current_capacity",
            self.totals.total_current_capacity,
            budget.max_total_capacity,
        );
        push_budget_violation_if_over_limit(
            &mut violations,
            "arena",
            None,
            "total_growth_events",
            self.totals.total_growth_events,
            budget.max_total_growth_events,
        );
        push_budget_violation_if_over_limit(
            &mut violations,
            "arena",
            None,
            "total_pushed_appended_operations",
            self.totals.total_pushed_appended_operations,
            budget.max_total_operations,
        );

        for container in &self.containers {
            push_budget_violation_if_over_limit(
                &mut violations,
                "container",
                Some(&container.name),
                "len",
                container.len,
                budget.max_container_len,
            );
            push_budget_violation_if_over_limit(
                &mut violations,
                "container",
                Some(&container.name),
                "current_capacity",
                container.current_capacity,
                budget.max_container_capacity,
            );
            push_budget_violation_if_over_limit(
                &mut violations,
                "container",
                Some(&container.name),
                "growth_events",
                container.growth_events,
                budget.max_container_growth_events,
            );
            push_budget_violation_if_over_limit(
                &mut violations,
                "container",
                Some(&container.name),
                "total_operations",
                container.total_operations,
                budget.max_container_operations,
            );
        }

        BudgetReport {
            passed: violations.is_empty(),
            violations,
        }
    }

    /// Classify this arena report into deterministic evidence-derived memory profiles.
    ///
    /// Thresholds are fixed and explicit: `Stable` requires zero growth events;
    /// `FrequentTinyGrowth` requires at least 8 growth events and an integer
    /// average jump no greater than 4 capacity units; `LargeSingleJump`
    /// requires one jump of at least 1024 capacity units; `BurstGrowth`
    /// requires at least 16 total capacity units added and the largest
    /// container contributor to hold at least 80 percent of added capacity;
    /// `OverReserved` requires capacity of at least 16 and at least 4x len;
    /// `UnderReserved` requires at least 8 growth events and no more than 4
    /// logical length units per growth event.
    pub fn profile(&self) -> ProfileReport {
        profile_arena_report(self)
    }

    /// Return a compact summary derived from this report's raw growth history.
    pub fn growth_summary(&self) -> GrowthSummary {
        summarize_growth_events(&self.growth_history)
    }

    /// Serialize the compact growth summary as pretty JSON.
    pub fn growth_summary_json(&self) -> String {
        serde_json::to_string_pretty(&self.growth_summary())
            .expect("serializing a GrowthSummary should not fail")
    }

    /// Return containers ordered by lifetime capacity added, largest first.
    pub fn top_growth_containers(&self) -> Vec<ContainerReport> {
        let mut containers = self.containers.clone();
        containers.sort_by(|left, right| {
            right
                .total_capacity_added
                .cmp(&left.total_capacity_added)
                .then_with(|| left.name.cmp(&right.name))
        });
        containers
    }

    /// Return a compact human-readable allocation and growth report.
    pub fn report(&self) -> String {
        render_arena_report(self, GrowthHistoryMode::Compact)
    }

    /// Return a verbose human-readable report with the full raw growth history.
    pub fn report_verbose(&self) -> String {
        render_arena_report(self, GrowthHistoryMode::Verbose)
    }

    /// Serialize this report as pretty JSON.
    ///
    /// This is an in-memory operation. It does not create files and it does not
    /// enable persistence. Use [`ArenaReport::write_json`] or
    /// [`Arena::write_json`] when a file should be written explicitly.
    ///
    /// ```
    /// use rig::{Arena, RigVec};
    ///
    /// let mut arena = Arena::new("json-method");
    /// let mut values = RigVec::new(&mut arena, "values");
    /// values.push(1);
    ///
    /// let report = arena.snapshot();
    /// let json = report.report_json();
    /// let decoded: rig::ArenaReport = serde_json::from_str(&json).unwrap();
    ///
    /// assert_eq!(decoded, report);
    /// ```
    pub fn report_json(&self) -> String {
        serde_json::to_string_pretty(self).expect("serializing an ArenaReport should not fail")
    }

    /// Export per-container summary evidence as CSV.
    pub fn containers_csv(&self) -> String {
        let mut csv = String::from("name,kind,len,initial_capacity,growth_policy,current_capacity,growth_events,total_capacity_added,largest_growth_jump,average_growth_jump,operation_label,total_operations,extra_metric_label,extra_metric_value\n");
        for container in &self.containers {
            csv.push_str(&csv_record([
                container.name.clone(),
                container.kind.clone(),
                container.len.to_string(),
                container.initial_capacity.to_string(),
                container.growth_policy.clone(),
                container.current_capacity.to_string(),
                container.growth_events.to_string(),
                container.total_capacity_added.to_string(),
                container.largest_growth_jump.to_string(),
                container.average_growth_jump.to_string(),
                container.operation_label.clone(),
                container.total_operations.to_string(),
                container.extra_metric_label.clone().unwrap_or_default(),
                container
                    .extra_metric_value
                    .map(|value| value.to_string())
                    .unwrap_or_default(),
            ]));
            csv.push('\n');
        }
        csv
    }

    /// Export raw growth history evidence as CSV.
    pub fn growth_history_csv(&self) -> String {
        growth_events_csv(&self.growth_history)
    }

    /// Export growth attribution evidence as CSV.
    pub fn growth_attributions_csv(&self) -> String {
        let mut csv = String::from("container_name,operation_index,old_capacity,new_capacity,capacity_added,growth_policy\n");
        for attribution in &self.growth_attributions {
            csv.push_str(&csv_record([
                attribution.container_name.clone(),
                attribution.operation_index.to_string(),
                attribution.old_capacity.to_string(),
                attribution.new_capacity.to_string(),
                attribution.capacity_added.to_string(),
                attribution.growth_policy.clone(),
            ]));
            csv.push('\n');
        }
        csv
    }

    /// Export per-container summary evidence as JSON Lines.
    pub fn containers_jsonl(&self) -> String {
        jsonl_lines(self.containers.iter())
    }

    /// Export raw growth history evidence as JSON Lines.
    pub fn growth_history_jsonl(&self) -> String {
        jsonl_lines(self.growth_history.iter())
    }

    /// Export growth attribution evidence as JSON Lines.
    pub fn growth_attributions_jsonl(&self) -> String {
        jsonl_lines(self.growth_attributions.iter())
    }

    /// Export per-container summary evidence in the requested format.
    pub fn export_containers(&self, format: ExportFormat) -> EvidenceExport {
        EvidenceExport {
            format,
            kind: "containers".to_owned(),
            contents: match format {
                ExportFormat::Csv => self.containers_csv(),
                ExportFormat::JsonLines => self.containers_jsonl(),
            },
        }
    }

    /// Export raw growth history evidence in the requested format.
    pub fn export_growth_history(&self, format: ExportFormat) -> EvidenceExport {
        EvidenceExport {
            format,
            kind: "growth_history".to_owned(),
            contents: match format {
                ExportFormat::Csv => self.growth_history_csv(),
                ExportFormat::JsonLines => self.growth_history_jsonl(),
            },
        }
    }

    /// Export growth attribution evidence in the requested format.
    pub fn export_growth_attributions(&self, format: ExportFormat) -> EvidenceExport {
        EvidenceExport {
            format,
            kind: "growth_attributions".to_owned(),
            contents: match format {
                ExportFormat::Csv => self.growth_attributions_csv(),
                ExportFormat::JsonLines => self.growth_attributions_jsonl(),
            },
        }
    }

    /// Write this report as pretty JSON to a programmer-provided path.
    ///
    /// This is explicit opt-in persistence. RIG does not create parent
    /// directories, background files, hidden files, or hidden directories.
    pub fn write_json(&self, path: impl AsRef<Path>) -> std::io::Result<()> {
        fs::write(path, self.report_json())
    }

    /// Write this report as exactly one JSON artifact to a caller-provided path.
    pub fn write_artifact<P: AsRef<Path>>(&self, path: P) -> Result<ReportArtifact, RigIoError> {
        let path = path.as_ref();
        self.write_json(path)?;

        Ok(ReportArtifact {
            path: path.to_path_buf(),
            report: self.clone(),
        })
    }

    /// Check this current report against a baseline report using memory regression gates.
    ///
    /// `self` is the current report and `baseline` is the previous report.
    /// Increases beyond the configured [`RegressionBudget`] produce typed
    /// [`MemoryRegression`] evidence. Improvements and removed containers do
    /// not fail. Containers present only in `self` are compared against zero.
    pub fn check_regressions_against(
        &self,
        baseline: &ArenaReport,
        budget: &RegressionBudget,
    ) -> RegressionReport {
        let total_capacity_delta = signed_delta_isize(
            baseline.totals.total_current_capacity,
            self.totals.total_current_capacity,
        );
        let total_growth_event_delta = signed_delta_isize(
            baseline.totals.total_growth_events,
            self.totals.total_growth_events,
        );
        let mut regressions = Vec::new();

        push_regression_if_over_budget(
            &mut regressions,
            "total",
            "total_current_capacity",
            baseline.totals.total_current_capacity,
            self.totals.total_current_capacity,
            budget.max_total_capacity_delta,
        );
        push_regression_if_over_budget(
            &mut regressions,
            "total",
            "total_growth_events",
            baseline.totals.total_growth_events,
            self.totals.total_growth_events,
            budget.max_total_growth_events_delta,
        );

        let baseline_by_name: BTreeMap<&str, &ContainerReport> = baseline
            .containers
            .iter()
            .map(|container| (container.name.as_str(), container))
            .collect();

        for current in &self.containers {
            let (baseline_capacity, baseline_growth_events) = baseline_by_name
                .get(current.name.as_str())
                .map(|container| (container.current_capacity, container.growth_events))
                .unwrap_or((0, 0));

            push_regression_if_over_budget(
                &mut regressions,
                &current.name,
                "current_capacity",
                baseline_capacity,
                current.current_capacity,
                budget.max_container_capacity_delta,
            );
            push_regression_if_over_budget(
                &mut regressions,
                &current.name,
                "growth_events",
                baseline_growth_events,
                current.growth_events,
                budget.max_container_growth_events_delta,
            );
        }

        RegressionReport {
            passed: regressions.is_empty(),
            regressions,
            total_capacity_delta,
            total_growth_event_delta,
        }
    }

    /// Compare this earlier report with a later report.
    ///
    /// Containers are matched by name. Containers present only in the later
    /// report appear in [`ArenaDiff::containers_added`], containers present only
    /// in this report appear in [`ArenaDiff::containers_removed`], and containers
    /// present in both reports produce a [`ContainerDiff`].
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
        let growth_events_added = after
            .growth_history
            .iter()
            .filter(|event| !self.growth_history.contains(event))
            .cloned()
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
            growth_events_added,
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

impl BudgetReport {
    /// Serialize this budget report as pretty JSON.
    ///
    /// This is an in-memory operation and does not write files.
    pub fn report_json(&self) -> String {
        serde_json::to_string_pretty(self).expect("serializing a BudgetReport should not fail")
    }

    /// Return a human-readable memory budget report.
    pub fn report(&self) -> String {
        self.to_string()
    }

    /// Classify failed budget violations as deterministic budget-risk profiles.
    pub fn profile(&self) -> ProfileReport {
        let profiles = self
            .violations
            .iter()
            .map(|violation| MemoryProfile {
                kind: MemoryProfileKind::BudgetRisk,
                subject: match &violation.container_name {
                    Some(container_name) => format!("{} {container_name}", violation.scope),
                    None => violation.scope.clone(),
                },
                reason: format!(
                    "budget metric {} observed {} exceeded limit {} by {}",
                    violation.metric, violation.observed, violation.limit, violation.exceeded_by
                ),
                evidence_metric: violation.metric.clone(),
                evidence_value: violation.observed,
                threshold: violation.limit,
            })
            .collect();
        ProfileReport { profiles }
    }

    /// Export typed budget violation evidence as CSV.
    pub fn violations_csv(&self) -> String {
        let mut csv = String::from("scope,container_name,metric,observed,limit,exceeded_by\n");
        for violation in &self.violations {
            csv.push_str(&csv_record([
                violation.scope.clone(),
                violation.container_name.clone().unwrap_or_default(),
                violation.metric.clone(),
                violation.observed.to_string(),
                violation.limit.to_string(),
                violation.exceeded_by.to_string(),
            ]));
            csv.push('\n');
        }
        csv
    }

    /// Export typed budget violation evidence as JSON Lines.
    pub fn violations_jsonl(&self) -> String {
        jsonl_lines(self.violations.iter())
    }

    /// Export typed budget violation evidence in the requested format.
    pub fn export_violations(&self, format: ExportFormat) -> EvidenceExport {
        EvidenceExport {
            format,
            kind: "budget_violations".to_owned(),
            contents: match format {
                ExportFormat::Csv => self.violations_csv(),
                ExportFormat::JsonLines => self.violations_jsonl(),
            },
        }
    }
}

impl fmt::Display for BudgetReport {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(formatter, "RIG memory budget report")?;
        writeln!(
            formatter,
            "Status: {}",
            if self.passed { "PASSED" } else { "FAILED" }
        )?;
        writeln!(formatter)?;
        writeln!(formatter, "Violations:")?;
        if self.violations.is_empty() {
            write!(formatter, "(none)")?;
        } else {
            for (index, violation) in self.violations.iter().enumerate() {
                match &violation.container_name {
                    Some(container_name) => {
                        writeln!(
                            formatter,
                            "{}. {} {}",
                            index + 1,
                            violation.scope,
                            container_name
                        )?;
                    }
                    None => {
                        writeln!(formatter, "{}. {}", index + 1, violation.scope)?;
                    }
                }
                writeln!(formatter, "   metric: {}", violation.metric)?;
                writeln!(formatter, "   observed: {}", violation.observed)?;
                writeln!(formatter, "   limit: {}", violation.limit)?;
                write!(formatter, "   exceeded by: {}", violation.exceeded_by)?;
                if index + 1 < self.violations.len() {
                    writeln!(formatter)?;
                    writeln!(formatter)?;
                }
            }
        }
        Ok(())
    }
}

impl RegressionReport {
    /// Serialize this regression report as pretty JSON.
    ///
    /// This is an in-memory operation and does not write files.
    pub fn report_json(&self) -> String {
        serde_json::to_string_pretty(self).expect("serializing a RegressionReport should not fail")
    }

    /// Return a human-readable memory regression report.
    pub fn report(&self) -> String {
        self.to_string()
    }

    /// Classify failed regression evidence as deterministic regression-risk profiles.
    pub fn profile(&self) -> ProfileReport {
        let profiles = self
            .regressions
            .iter()
            .map(|regression| MemoryProfile {
                kind: MemoryProfileKind::RegressionRisk,
                subject: regression.container_name.clone(),
                reason: format!(
                    "regression metric {} delta {} exceeded allowed delta {}",
                    regression.metric, regression.delta, regression.allowed_delta
                ),
                evidence_metric: regression.metric.clone(),
                evidence_value: regression.delta,
                threshold: regression.allowed_delta,
            })
            .collect();
        ProfileReport { profiles }
    }

    /// Export typed regression failure evidence as CSV.
    pub fn regressions_csv(&self) -> String {
        let mut csv = String::from("container_name,metric,baseline,current,delta,allowed_delta\n");
        for regression in &self.regressions {
            csv.push_str(&csv_record([
                regression.container_name.clone(),
                regression.metric.clone(),
                regression.baseline.to_string(),
                regression.current.to_string(),
                regression.delta.to_string(),
                regression.allowed_delta.to_string(),
            ]));
            csv.push('\n');
        }
        csv
    }

    /// Export typed regression failure evidence as JSON Lines.
    pub fn regressions_jsonl(&self) -> String {
        jsonl_lines(self.regressions.iter())
    }

    /// Export typed regression failure evidence in the requested format.
    pub fn export_regressions(&self, format: ExportFormat) -> EvidenceExport {
        EvidenceExport {
            format,
            kind: "regression_failures".to_owned(),
            contents: match format {
                ExportFormat::Csv => self.regressions_csv(),
                ExportFormat::JsonLines => self.regressions_jsonl(),
            },
        }
    }
}

impl fmt::Display for RegressionReport {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(formatter, "RIG memory regression report")?;
        writeln!(
            formatter,
            "Status: {}",
            if self.passed { "PASSED" } else { "FAILED" }
        )?;
        writeln!(formatter)?;
        writeln!(
            formatter,
            "Total capacity delta: {}",
            format_delta_isize(self.total_capacity_delta)
        )?;
        writeln!(
            formatter,
            "Total growth event delta: {}",
            format_delta_isize(self.total_growth_event_delta)
        )?;
        writeln!(formatter)?;
        writeln!(formatter, "Regressions:")?;
        if self.regressions.is_empty() {
            write!(formatter, "  (none)")?;
        } else {
            for (index, regression) in self.regressions.iter().enumerate() {
                writeln!(formatter, "{}. {}", index + 1, regression.container_name)?;
                writeln!(formatter, "   metric: {}", regression.metric)?;
                writeln!(formatter, "   baseline: {}", regression.baseline)?;
                writeln!(formatter, "   current: {}", regression.current)?;
                writeln!(formatter, "   delta: {}", regression.delta)?;
                write!(formatter, "   allowed delta: {}", regression.allowed_delta)?;
                if index + 1 < self.regressions.len() {
                    writeln!(formatter)?;
                }
            }
        }
        Ok(())
    }
}

impl ArenaDiff {
    /// Return a pretty JSON diff report.
    ///
    /// This is an in-memory operation. It does not write files; use
    /// [`ArenaDiff::write_json`] for explicit file persistence.
    pub fn diff_json(&self) -> String {
        serde_json::to_string_pretty(self).expect("serializing an ArenaDiff should not fail")
    }

    /// Write this diff as pretty JSON to a programmer-provided path.
    ///
    /// This method does not create missing parent directories.
    pub fn write_json(&self, path: impl AsRef<Path>) -> std::io::Result<()> {
        fs::write(path, self.diff_json())
    }

    /// Return a human-readable diff report.
    ///
    /// The report is intended for direct inspection; use [`ArenaDiff::diff_json`]
    /// when stable machine-readable evidence is needed.
    pub fn report(&self) -> String {
        self.to_string()
    }

    /// Return a compact summary for growth events added by this diff.
    pub fn growth_summary(&self) -> GrowthSummary {
        summarize_growth_events(&self.growth_events_added)
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

        writeln!(formatter, "Growth events added:")?;
        if self.growth_events_added.is_empty() {
            writeln!(formatter, "  (none)")?;
        } else {
            for event in &self.growth_events_added {
                writeln!(
                    formatter,
                    "  {}: {} -> {} at operation {} (+{}) under {}",
                    event.container_name,
                    event.old_capacity,
                    event.new_capacity,
                    event.operation_index,
                    event.capacity_added,
                    event.growth_policy
                )?;
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

fn build_contract_report(
    contract: &WorkloadContract,
    budget_report: Option<BudgetReport>,
    regression_report: Option<RegressionReport>,
    profile_report: Option<ProfileReport>,
) -> ContractReport {
    let mut violations = Vec::new();

    if let Some(report) = &budget_report {
        violations.extend(report.violations.iter().map(|violation| {
            let subject = match &violation.container_name {
                Some(container_name) => container_name.clone(),
                None => violation.scope.clone(),
            };
            ContractViolation {
                contract_name: contract.name.clone(),
                rule: "budget".to_owned(),
                subject,
                reason: format!("{} exceeded limit", violation.metric),
                evidence: format!(
                    "observed {}, limit {}, exceeded by {}",
                    violation.observed, violation.limit, violation.exceeded_by
                ),
            }
        }));
    }

    if let Some(report) = &regression_report {
        violations.extend(
            report
                .regressions
                .iter()
                .map(|regression| ContractViolation {
                    contract_name: contract.name.clone(),
                    rule: "regression".to_owned(),
                    subject: regression.container_name.clone(),
                    reason: format!("{} delta exceeded allowed delta", regression.metric),
                    evidence: format!(
                        "baseline {}, current {}, delta {}, allowed delta {}",
                        regression.baseline,
                        regression.current,
                        regression.delta,
                        regression.allowed_delta
                    ),
                }),
        );
    }

    if let Some(report) = &profile_report {
        for required_absent in &contract.required_profiles_absent {
            for profile in report.profiles_by_kind(*required_absent) {
                violations.push(ContractViolation {
                    contract_name: contract.name.clone(),
                    rule: "profile_absent".to_owned(),
                    subject: profile.subject.clone(),
                    reason: format!("forbidden profile {} was present", profile.kind),
                    evidence: format!(
                        "{}={} threshold={}",
                        profile.evidence_metric, profile.evidence_value, profile.threshold
                    ),
                });
            }
        }

        for required_present in &contract.required_profiles_present {
            let matches = report.profiles_by_kind(*required_present);
            if matches.is_empty() {
                violations.push(ContractViolation {
                    contract_name: contract.name.clone(),
                    rule: "profile_present".to_owned(),
                    subject: contract.name.clone(),
                    reason: format!("required profile {required_present} was missing"),
                    evidence: format!(
                        "profile_count={} matching_kind_count=0",
                        report.profiles.len()
                    ),
                });
            }
        }
    }

    ContractReport {
        contract_name: contract.name.clone(),
        passed: violations.is_empty(),
        violations,
        budget_report,
        regression_report,
        profile_report,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GrowthHistoryMode {
    Compact,
    Verbose,
}

fn push_budget_violation_if_over_limit(
    violations: &mut Vec<BudgetViolation>,
    scope: &str,
    container_name: Option<&str>,
    metric: &str,
    observed: usize,
    limit: Option<usize>,
) {
    if let Some(limit) = limit {
        if observed > limit {
            violations.push(BudgetViolation {
                scope: scope.to_owned(),
                container_name: container_name.map(str::to_owned),
                metric: metric.to_owned(),
                observed,
                limit,
                exceeded_by: observed - limit,
            });
        }
    }
}

fn push_regression_if_over_budget(
    regressions: &mut Vec<MemoryRegression>,
    container_name: &str,
    metric: &str,
    baseline: usize,
    current: usize,
    allowed_delta: Option<usize>,
) {
    let Some(allowed_delta) = allowed_delta else {
        return;
    };
    let delta = current.saturating_sub(baseline);
    if delta > allowed_delta {
        regressions.push(MemoryRegression {
            container_name: container_name.to_owned(),
            metric: metric.to_owned(),
            baseline,
            current,
            delta,
            allowed_delta,
        });
    }
}

fn signed_delta_isize(before: usize, after: usize) -> isize {
    if after >= before {
        after.saturating_sub(before).min(isize::MAX as usize) as isize
    } else {
        -(before.saturating_sub(after).min(isize::MAX as usize) as isize)
    }
}

fn format_delta_isize(delta: isize) -> String {
    if delta >= 0 {
        format!("+{delta}")
    } else {
        delta.to_string()
    }
}

fn summarize_growth_events(events: &[GrowthEvent]) -> GrowthSummary {
    let mut per_container: BTreeMap<(String, String), ContainerGrowthSummary> = BTreeMap::new();
    let mut largest_growth_delta = 0;
    let mut largest_growth_container = None;

    for event in events {
        let delta = event.capacity_added;
        if delta > largest_growth_delta {
            largest_growth_delta = delta;
            largest_growth_container = Some(event.container_name.clone());
        }

        let key = (event.container_name.clone(), event.container_kind.clone());
        per_container
            .entry(key)
            .and_modify(|summary| {
                summary.growth_events += 1;
                summary.final_new_capacity = event.new_capacity;
                summary.largest_growth_delta = summary.largest_growth_delta.max(delta);
                summary.last_operation_index = event.operation_index;
            })
            .or_insert_with(|| ContainerGrowthSummary {
                container_name: event.container_name.clone(),
                container_kind: event.container_kind.clone(),
                growth_events: 1,
                first_old_capacity: event.old_capacity,
                final_new_capacity: event.new_capacity,
                largest_growth_delta: delta,
                first_operation_index: event.operation_index,
                last_operation_index: event.operation_index,
            });
    }

    GrowthSummary {
        total_growth_events: events.len(),
        containers_with_growth: per_container.len(),
        largest_growth_delta,
        largest_growth_container,
        first_growth_event: events.first().cloned(),
        last_growth_event: events.last().cloned(),
        per_container: per_container.into_values().collect(),
    }
}

fn render_arena_report(snapshot: &ArenaReport, mode: GrowthHistoryMode) -> String {
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
    }

    for record in &snapshot.containers {
        report.push_str(&format!(
            "\n  Container: {}\n  kind: {}\n  fields:\n    len: {}\n    initial capacity: {}\n    growth policy: {}\n    current capacity: {}\n    growth events: {}\n    total capacity added: {}\n    largest growth jump: {}\n    average growth jump: {}\n    {}: {}",
            record.name,
            record.kind,
            record.len,
            record.initial_capacity,
            record.growth_policy,
            record.current_capacity,
            record.growth_events,
            record.total_capacity_added,
            record.largest_growth_jump,
            record.average_growth_jump,
            record.operation_label,
            record.total_operations
        ));

        if let (Some(label), Some(value)) = (&record.extra_metric_label, record.extra_metric_value)
        {
            report.push_str(&format!("\n    {}: {}", label, value));
        }
    }

    render_top_growth_contributors(snapshot, &mut report);
    render_growth_summary(snapshot, &mut report);
    match mode {
        GrowthHistoryMode::Compact => render_compact_growth_history(snapshot, &mut report),
        GrowthHistoryMode::Verbose => render_verbose_growth_history(snapshot, &mut report),
    }
    report
}

fn render_top_growth_contributors(snapshot: &ArenaReport, report: &mut String) {
    report.push_str("\nTop growth contributors:");
    let ranked = snapshot.top_growth_containers();
    let contributors = ranked
        .iter()
        .filter(|container| container.total_capacity_added > 0)
        .collect::<Vec<_>>();

    if contributors.is_empty() {
        report.push_str("\n  (none)");
        return;
    }

    for (index, container) in contributors.iter().enumerate() {
        report.push_str(&format!(
            "\n  {}. {}\n     total capacity added: {}",
            index + 1,
            container.name,
            container.total_capacity_added
        ));
    }
}

fn render_growth_summary(snapshot: &ArenaReport, report: &mut String) {
    let summary = snapshot.growth_summary();
    report.push_str("\nGrowth history summary:");
    report.push_str(&format!(
        "\n  total_growth_events: {}\n  containers_with_growth: {}\n  largest_growth_delta: {}\n  largest_growth_container: {}",
        summary.total_growth_events,
        summary.containers_with_growth,
        summary.largest_growth_delta,
        summary.largest_growth_container.as_deref().unwrap_or("(none)")
    ));
    if let Some(event) = &summary.first_growth_event {
        report.push_str(&format!(
            "\n  first_growth_event: {} {} -> {} at operation {} (+{}) under {}",
            event.container_name,
            event.old_capacity,
            event.new_capacity,
            event.operation_index,
            event.capacity_added,
            event.growth_policy
        ));
    } else {
        report.push_str("\n  first_growth_event: (none)");
    }
    if let Some(event) = &summary.last_growth_event {
        report.push_str(&format!(
            "\n  last_growth_event: {} {} -> {} at operation {} (+{}) under {}",
            event.container_name,
            event.old_capacity,
            event.new_capacity,
            event.operation_index,
            event.capacity_added,
            event.growth_policy
        ));
    } else {
        report.push_str("\n  last_growth_event: (none)");
    }
    report.push_str("\n  per_container:");
    if summary.per_container.is_empty() {
        report.push_str("\n    (none)");
    } else {
        for container in &summary.per_container {
            report.push_str(&format!(
                "\n    {} ({}): growth_events={}, first_old_capacity={}, final_new_capacity={}, largest_growth_delta={}, first_operation_index={}, last_operation_index={}",
                container.container_name,
                container.container_kind,
                container.growth_events,
                container.first_old_capacity,
                container.final_new_capacity,
                container.largest_growth_delta,
                container.first_operation_index,
                container.last_operation_index
            ));
        }
    }
}

fn render_compact_growth_history(snapshot: &ArenaReport, report: &mut String) {
    const EDGE_EVENT_COUNT: usize = 3;
    report.push_str("\nGrowth history preview:");
    if snapshot.growth_history.is_empty() {
        report.push_str("\n  (none)");
        return;
    }

    let event_count = snapshot.growth_history.len();
    let head_count = event_count.min(EDGE_EVENT_COUNT);
    report.push_str("\n  first events:");
    for event in snapshot.growth_history.iter().take(head_count) {
        append_growth_event_line(report, event);
    }

    if event_count > EDGE_EVENT_COUNT {
        let tail_count = (event_count - head_count).min(EDGE_EVENT_COUNT);
        let omitted = event_count - head_count - tail_count;
        if omitted > 0 {
            report.push_str(&format!(
                "\n  ... {omitted} growth events omitted from compact report ..."
            ));
        }
        report.push_str("\n  last events:");
        for event in snapshot
            .growth_history
            .iter()
            .skip(event_count - tail_count)
        {
            append_growth_event_line(report, event);
        }
    }

    report.push_str(
        "\n  Full raw growth history is available through report_verbose() and report_json().",
    );
}

fn render_verbose_growth_history(snapshot: &ArenaReport, report: &mut String) {
    report.push_str("\nGrowth history (full raw evidence):");
    if snapshot.growth_history.is_empty() {
        report.push_str("\n  (none)");
    } else {
        for event in &snapshot.growth_history {
            append_growth_event_line(report, event);
        }
    }
}

fn append_growth_event_line(report: &mut String, event: &GrowthEvent) {
    report.push_str(&format!(
        "\n  {}: {} -> {} at operation {} (+{}) under {}",
        event.container_name,
        event.old_capacity,
        event.new_capacity,
        event.operation_index,
        event.capacity_added,
        event.growth_policy
    ));
}

fn csv_record<I, S>(fields: I) -> String
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    fields
        .into_iter()
        .map(|field| csv_field(field.as_ref()))
        .collect::<Vec<_>>()
        .join(",")
}

fn csv_field(field: &str) -> String {
    if field.contains([',', '"', '\n', '\r']) {
        format!("\"{}\"", field.replace('"', "\"\""))
    } else {
        field.to_owned()
    }
}

fn jsonl_lines<I, T>(items: I) -> String
where
    I: IntoIterator<Item = T>,
    T: Serialize,
{
    let mut lines = String::new();
    for item in items {
        lines.push_str(
            &serde_json::to_string(&item)
                .expect("serializing RIG evidence for JSON Lines should not fail"),
        );
        lines.push('\n');
    }
    lines
}

fn growth_events_csv(events: &[GrowthEvent]) -> String {
    let mut csv = String::from(
        "container_name,container_kind,old_capacity,new_capacity,operation_index,capacity_added,growth_policy\n",
    );
    for event in events {
        csv.push_str(&csv_record([
            event.container_name.clone(),
            event.container_kind.clone(),
            event.old_capacity.to_string(),
            event.new_capacity.to_string(),
            event.operation_index.to_string(),
            event.capacity_added.to_string(),
            event.growth_policy.clone(),
        ]));
        csv.push('\n');
    }
    csv
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

/// Errors that can occur while reading or writing explicit RIG JSON artifacts.
#[derive(Debug)]
pub enum RigIoError {
    /// The report file could not be read.
    Io(std::io::Error),
    /// The report file was read but did not contain valid `ArenaReport` JSON.
    Json(serde_json::Error),
}

impl fmt::Display for RigIoError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(formatter, "failed to read RIG report: {error}"),
            Self::Json(error) => write!(formatter, "failed to parse RIG report JSON: {error}"),
        }
    }
}

impl Error for RigIoError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Io(error) => Some(error),
            Self::Json(error) => Some(error),
        }
    }
}

impl From<std::io::Error> for RigIoError {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error)
    }
}

impl From<serde_json::Error> for RigIoError {
    fn from(error: serde_json::Error) -> Self {
        Self::Json(error)
    }
}

/// Backward-compatible name for report JSON I/O errors.
pub type LoadReportError = RigIoError;

#[derive(Debug, Clone)]
struct ContainerRecord {
    name: String,
    kind: ContainerKind,
    len: usize,
    initial_capacity: usize,
    growth_policy: GrowthPolicy,
    capacity: usize,
    growth_events: usize,
    operation_label: &'static str,
    total_operations: usize,
    extra_metric_label: Option<&'static str>,
    extra_metric_value: usize,
    growth_history: Vec<GrowthEvent>,
}

impl ContainerRecord {
    fn new(
        name: impl Into<String>,
        kind: ContainerKind,
        initial_capacity: usize,
        growth_policy: GrowthPolicy,
    ) -> Self {
        let (operation_label, extra_metric_label) = match kind {
            ContainerKind::RigVec => ("total pushed items", None),
            ContainerKind::RigString => ("total append operations", Some("total appended bytes")),
        };

        Self {
            name: name.into(),
            kind,
            len: 0,
            initial_capacity,
            growth_policy,
            capacity: initial_capacity,
            growth_events: 0,
            operation_label,
            total_operations: 0,
            extra_metric_label,
            extra_metric_value: 0,
            growth_history: Vec::new(),
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
        growth_policy: GrowthPolicy,
    ) -> usize {
        let mut inner = self.inner.borrow_mut();
        inner.records.push(ContainerRecord::new(
            container_name,
            kind,
            initial_capacity,
            growth_policy,
        ));
        inner.records.len() - 1
    }

    fn record_growth_event(
        &self,
        record_id: usize,
        old_capacity: usize,
        new_capacity: usize,
        operation_index: usize,
    ) {
        let mut inner = self.inner.borrow_mut();
        if let Some(record) = inner.records.get_mut(record_id) {
            record.growth_history.push(GrowthEvent {
                container_name: record.name.clone(),
                container_kind: record.kind.as_str().to_owned(),
                old_capacity,
                new_capacity,
                operation_index,
                capacity_added: new_capacity.saturating_sub(old_capacity),
                growth_policy: record.growth_policy.report_name(),
            });
        }
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
            .map(|record| {
                let total_capacity_added: usize = record
                    .growth_history
                    .iter()
                    .map(|event| event.capacity_added)
                    .sum();
                let largest_growth_jump = record
                    .growth_history
                    .iter()
                    .map(|event| event.capacity_added)
                    .max()
                    .unwrap_or(0);
                let average_growth_jump = if record.growth_history.is_empty() {
                    0
                } else {
                    total_capacity_added / record.growth_history.len()
                };

                ContainerReport {
                    name: record.name.clone(),
                    kind: record.kind.as_str().to_owned(),
                    len: record.len,
                    initial_capacity: record.initial_capacity,
                    growth_policy: record.growth_policy.report_name(),
                    current_capacity: record.capacity,
                    growth_events: record.growth_events,
                    total_capacity_added,
                    largest_growth_jump,
                    average_growth_jump,
                    operation_label: record.operation_label.to_owned(),
                    total_operations: record.total_operations,
                    extra_metric_label: record.extra_metric_label.map(str::to_owned),
                    extra_metric_value: record
                        .extra_metric_label
                        .map(|_| record.extra_metric_value),
                }
            })
            .collect();
        let growth_history: Vec<GrowthEvent> = inner
            .records
            .iter()
            .flat_map(|record| record.growth_history.iter().cloned())
            .collect();
        let growth_attributions = growth_history.iter().map(GrowthAttribution::from).collect();

        ArenaReport {
            arena_name: inner.name.clone(),
            tracked_container_count: inner.records.len(),
            totals,
            containers,
            growth_history,
            growth_attributions,
        }
    }

    /// Return a pretty JSON allocation and growth report for tracked containers.
    ///
    /// This is an in-memory operation equivalent to
    /// `self.snapshot().report_json()`. It does not create files.
    pub fn report_json(&self) -> String {
        self.snapshot().report_json()
    }

    /// Write the current pretty JSON report to a programmer-provided path.
    ///
    /// This method is explicit opt-in persistence: RIG never calls it automatically.
    /// It creates or overwrites the target file, but it does not create missing
    /// parent directories.
    pub fn write_json(&self, path: impl AsRef<Path>) -> std::io::Result<()> {
        fs::write(path, self.report_json())
    }

    /// Load a previously persisted arena report from JSON on disk.
    pub fn load_report(path: impl AsRef<Path>) -> Result<ArenaReport, RigIoError> {
        let json = fs::read_to_string(path)?;
        let report = serde_json::from_str(&json)?;
        Ok(report)
    }

    /// Return a compact human-readable allocation and growth report for tracked containers.
    pub fn report(&self) -> String {
        self.snapshot().report()
    }

    /// Return a verbose human-readable report with the full raw growth history.
    pub fn report_verbose(&self) -> String {
        self.snapshot().report_verbose()
    }

    /// Return a compact summary derived from the current raw growth history.
    pub fn growth_summary(&self) -> GrowthSummary {
        self.snapshot().growth_summary()
    }

    /// Serialize the compact growth summary as pretty JSON.
    pub fn growth_summary_json(&self) -> String {
        self.snapshot().growth_summary_json()
    }
}

/// A `Vec<T>` wrapper that keeps Rust safety while making growth visible.
#[derive(Debug)]
pub struct RigVec<T> {
    values: Vec<T>,
    arena: Arena,
    record_id: usize,
    container_name: String,
    growth_policy: GrowthPolicy,
    growth_events: usize,
    total_pushed: usize,
}

impl<T> RigVec<T> {
    /// Create a tracked vector record inside an arena.
    pub fn new(arena: &mut Arena, container_name: impl Into<String>) -> Self {
        Self::with_capacity(arena, container_name, 0)
    }

    /// Create a tracked vector record inside an arena using a growth policy.
    pub fn with_policy(
        arena: &mut Arena,
        container_name: impl Into<String>,
        policy: GrowthPolicy,
    ) -> Self {
        Self::with_capacity_and_policy(arena, container_name, 0, policy)
    }

    /// Create a tracked vector record inside an arena with preallocated capacity.
    pub fn with_capacity(
        arena: &mut Arena,
        container_name: impl Into<String>,
        capacity: usize,
    ) -> Self {
        Self::with_capacity_and_policy(arena, container_name, capacity, GrowthPolicy::RustDefault)
    }

    /// Create a tracked vector record with preallocated capacity and a growth policy.
    pub fn with_capacity_and_policy(
        arena: &mut Arena,
        container_name: impl Into<String>,
        capacity: usize,
        policy: GrowthPolicy,
    ) -> Self {
        let container_name = container_name.into();
        let record_id = arena.add_record(
            container_name.clone(),
            ContainerKind::RigVec,
            capacity,
            policy.clone(),
        );
        let vec = Self {
            values: Vec::with_capacity(capacity),
            arena: arena.clone(),
            record_id,
            container_name,
            growth_policy: policy,
            growth_events: 0,
            total_pushed: 0,
        };
        vec.sync_record();
        vec
    }

    /// Push an item into the underlying `Vec<T>` and record capacity growth.
    ///
    /// For capped containers, this panics with a clear message if the operation
    /// would exceed the configured capacity. Use [`RigVec::try_push`] to handle
    /// that failure without panicking.
    pub fn push(&mut self, value: T) {
        self.try_push(value)
            .expect("RigVec::push failed because growth policy refused capacity growth")
    }

    /// Fallibly push an item into the underlying `Vec<T>` and record capacity growth.
    pub fn try_push(&mut self, value: T) -> Result<(), RigError> {
        let old_capacity = self.values.capacity();
        let needed_len = self.values.len().saturating_add(1);
        self.reserve_for_needed_len(needed_len)?;
        self.values.push(value);
        self.total_pushed += 1;
        self.record_growth_if_needed(old_capacity);
        self.sync_record();
        Ok(())
    }

    fn reserve_for_needed_len(&mut self, needed_len: usize) -> Result<(), RigError> {
        if let Some(target_capacity) = self.growth_policy.checked_target(
            &self.container_name,
            self.values.capacity(),
            needed_len,
        )? {
            let additional = target_capacity.saturating_sub(self.values.len());
            match self.growth_policy {
                GrowthPolicy::Exact | GrowthPolicy::Capped { .. } => {
                    self.values.reserve_exact(additional);
                }
                GrowthPolicy::Double | GrowthPolicy::ReserveAhead(_) => {
                    self.values.reserve(additional);
                }
                GrowthPolicy::RustDefault => {}
            }
        }
        Ok(())
    }

    fn record_growth_if_needed(&mut self, old_capacity: usize) {
        let new_capacity = self.values.capacity();
        if new_capacity > old_capacity {
            self.growth_events += 1;
            self.arena.record_growth_event(
                self.record_id,
                old_capacity,
                new_capacity,
                self.total_pushed,
            );
        }
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
    container_name: String,
    growth_policy: GrowthPolicy,
    growth_events: usize,
    append_operations: usize,
    total_appended_bytes: usize,
}

impl RigString {
    /// Create a tracked string record inside an arena.
    pub fn new(arena: &mut Arena, container_name: impl Into<String>) -> Self {
        Self::with_capacity(arena, container_name, 0)
    }

    /// Create a tracked string record inside an arena using a growth policy.
    pub fn with_policy(
        arena: &mut Arena,
        container_name: impl Into<String>,
        policy: GrowthPolicy,
    ) -> Self {
        Self::with_capacity_and_policy(arena, container_name, 0, policy)
    }

    /// Create a tracked string record inside an arena with preallocated capacity.
    pub fn with_capacity(
        arena: &mut Arena,
        container_name: impl Into<String>,
        capacity: usize,
    ) -> Self {
        Self::with_capacity_and_policy(arena, container_name, capacity, GrowthPolicy::RustDefault)
    }

    /// Create a tracked string record with preallocated capacity and a growth policy.
    pub fn with_capacity_and_policy(
        arena: &mut Arena,
        container_name: impl Into<String>,
        capacity: usize,
        policy: GrowthPolicy,
    ) -> Self {
        let container_name = container_name.into();
        let record_id = arena.add_record(
            container_name.clone(),
            ContainerKind::RigString,
            capacity,
            policy.clone(),
        );
        let string = Self {
            value: String::with_capacity(capacity),
            arena: arena.clone(),
            record_id,
            container_name,
            growth_policy: policy,
            growth_events: 0,
            append_operations: 0,
            total_appended_bytes: 0,
        };
        string.sync_record();
        string
    }

    /// Append a string slice and record capacity growth.
    ///
    /// For capped strings, this panics with a clear message if the operation
    /// would exceed the configured capacity. Use [`RigString::try_push_str`] to
    /// handle that failure without panicking.
    pub fn push_str(&mut self, value: &str) {
        self.try_push_str(value)
            .expect("RigString::push_str failed because growth policy refused capacity growth")
    }

    /// Fallibly append a string slice and record capacity growth.
    pub fn try_push_str(&mut self, value: &str) -> Result<(), RigError> {
        let old_capacity = self.value.capacity();
        let needed_len = self.value.len().saturating_add(value.len());
        self.reserve_for_needed_len(needed_len)?;
        self.value.push_str(value);
        self.append_operations += 1;
        self.total_appended_bytes += value.len();
        self.record_growth_if_needed(old_capacity);
        self.sync_record();
        Ok(())
    }

    fn reserve_for_needed_len(&mut self, needed_len: usize) -> Result<(), RigError> {
        if let Some(target_capacity) = self.growth_policy.checked_target(
            &self.container_name,
            self.value.capacity(),
            needed_len,
        )? {
            let additional = target_capacity.saturating_sub(self.value.len());
            match self.growth_policy {
                GrowthPolicy::Exact | GrowthPolicy::Capped { .. } => {
                    self.value.reserve_exact(additional);
                }
                GrowthPolicy::Double | GrowthPolicy::ReserveAhead(_) => {
                    self.value.reserve(additional);
                }
                GrowthPolicy::RustDefault => {}
            }
        }
        Ok(())
    }

    fn record_growth_if_needed(&mut self, old_capacity: usize) {
        let new_capacity = self.value.capacity();
        if new_capacity > old_capacity {
            self.growth_events += 1;
            self.arena.record_growth_event(
                self.record_id,
                old_capacity,
                new_capacity,
                self.append_operations,
            );
        }
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
