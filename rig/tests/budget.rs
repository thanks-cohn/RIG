use rig::{Arena, ArenaReport, BudgetReport, GrowthPolicy, MemoryBudget, RigVec};
use std::process::Command;

fn exact_report() -> ArenaReport {
    let mut arena = Arena::new("budget");
    let mut small = RigVec::with_policy(&mut arena, "small", GrowthPolicy::Exact);
    small.push(1);

    let mut offender = RigVec::with_policy(&mut arena, "offender", GrowthPolicy::Exact);
    for value in 0..4 {
        offender.push(value);
    }

    arena.snapshot()
}

fn violation_for<'a>(
    report: &'a BudgetReport,
    scope: &str,
    container_name: Option<&str>,
    metric: &str,
) -> &'a rig::BudgetViolation {
    report
        .violations
        .iter()
        .find(|violation| {
            violation.scope == scope
                && violation.container_name.as_deref() == container_name
                && violation.metric == metric
        })
        .unwrap_or_else(|| {
            panic!("missing violation for {scope}.{container_name:?}.{metric}: {report:#?}")
        })
}

#[test]
fn budget_unlimited_passes_non_empty_report() {
    let observed = exact_report();
    assert!(observed.totals.total_current_capacity > 0);

    let report = observed.check_budget(&MemoryBudget::unlimited());

    assert!(report.passed);
    assert!(report.violations.is_empty());
}

#[test]
fn budget_strict_zero_growth_fails_when_growth_events_exist() {
    let observed = exact_report();
    assert!(observed.totals.total_growth_events > 0);

    let report = observed.check_budget(&MemoryBudget::strict_zero_growth());

    assert!(!report.passed);
    let arena = violation_for(&report, "arena", None, "total_growth_events");
    assert_eq!(arena.observed, observed.totals.total_growth_events);
    let container = violation_for(&report, "container", Some("offender"), "growth_events");
    assert_eq!(container.observed, 4);
}

#[test]
fn budget_max_total_capacity_catches_arena_capacity_violation() {
    let observed = exact_report();
    let limit = observed.totals.total_current_capacity - 1;

    let report = observed.check_budget(&MemoryBudget::max_total_capacity(limit));

    let violation = violation_for(&report, "arena", None, "total_current_capacity");
    assert_eq!(violation.observed, observed.totals.total_current_capacity);
    assert_eq!(violation.limit, limit);
    assert_eq!(violation.exceeded_by, 1);
}

#[test]
fn budget_max_total_growth_events_catches_arena_growth_violation() {
    let observed = exact_report();
    let limit = observed.totals.total_growth_events - 1;

    let report = observed.check_budget(&MemoryBudget::max_total_growth_events(limit));

    let violation = violation_for(&report, "arena", None, "total_growth_events");
    assert_eq!(violation.observed, observed.totals.total_growth_events);
    assert_eq!(violation.limit, limit);
    assert_eq!(violation.exceeded_by, 1);
}

#[test]
fn budget_max_container_capacity_catches_only_offending_container() {
    let observed = exact_report();
    let small_capacity = observed
        .containers
        .iter()
        .find(|container| container.name == "small")
        .expect("small container")
        .current_capacity;

    let report = observed.check_budget(&MemoryBudget::max_container_capacity(small_capacity));

    assert!(!report.passed);
    assert_eq!(report.violations.len(), 1);
    let violation = violation_for(&report, "container", Some("offender"), "current_capacity");
    assert_eq!(violation.observed, 4);
    assert_eq!(violation.limit, small_capacity);
}

#[test]
fn budget_max_container_growth_events_catches_offending_container() {
    let observed = exact_report();

    let report = observed.check_budget(&MemoryBudget::max_container_growth_events(1));

    assert!(!report.passed);
    assert_eq!(report.violations.len(), 1);
    let violation = violation_for(&report, "container", Some("offender"), "growth_events");
    assert_eq!(violation.observed, 4);
    assert_eq!(violation.limit, 1);
    assert_eq!(violation.exceeded_by, 3);
}

#[test]
fn budget_passed_report_has_no_violations() {
    let observed = exact_report();
    let budget = MemoryBudget::unlimited()
        .with_max_total_len(observed.totals.total_len)
        .with_max_total_capacity(observed.totals.total_current_capacity)
        .with_max_total_growth_events(observed.totals.total_growth_events)
        .with_max_total_operations(observed.totals.total_pushed_appended_operations)
        .with_max_container_len(4)
        .with_max_container_capacity(4)
        .with_max_container_growth_events(4)
        .with_max_container_operations(4);

    let report = observed.check_budget(&budget);

    assert!(report.passed, "unexpected violations: {report:#?}");
    assert!(report.violations.is_empty());
}

#[test]
fn budget_failed_report_has_typed_budget_violation_data() {
    let observed = exact_report();

    let report = observed.check_budget(&MemoryBudget::unlimited().with_max_total_operations(1));

    assert!(!report.passed);
    let violation = violation_for(&report, "arena", None, "total_pushed_appended_operations");
    assert_eq!(violation.container_name, None);
    assert_eq!(
        violation.observed,
        observed.totals.total_pushed_appended_operations
    );
    assert_eq!(violation.limit, 1);
}

#[test]
fn budget_exceeded_by_is_computed_correctly() {
    let observed = exact_report();

    let report = observed.check_budget(&MemoryBudget::max_container_capacity(1));

    let violation = violation_for(&report, "container", Some("offender"), "current_capacity");
    assert_eq!(violation.observed, 4);
    assert_eq!(violation.limit, 1);
    assert_eq!(violation.exceeded_by, 3);
}

#[test]
fn budget_report_json_round_trip_works() {
    let observed = exact_report();
    let report = observed.check_budget(&MemoryBudget::max_total_capacity(1));

    let decoded: BudgetReport =
        serde_json::from_str(&report.report_json()).expect("valid budget JSON");

    assert_eq!(decoded, report);
    assert!(!decoded.passed);
    assert!(!decoded.violations.is_empty());
}

#[test]
fn budget_human_report_includes_passed_and_failed_statuses() {
    let observed = exact_report();

    let passed = observed.check_budget(&MemoryBudget::unlimited()).report();
    let failed = observed
        .check_budget(&MemoryBudget::max_total_capacity(1))
        .report();

    assert!(passed.contains("RIG memory budget report"));
    assert!(passed.contains("Status: PASSED"));
    assert!(passed.contains("Violations:"));
    assert!(passed.contains("(none)"));
    assert!(failed.contains("Status: FAILED"));
    assert!(failed.contains("total_current_capacity"));
    assert!(failed.contains("exceeded by"));
}

#[test]
fn memory_budget_example_runs_and_outputs_real_budget_fields() {
    let output = Command::new(env!("CARGO"))
        .args(["run", "--quiet", "--example", "memory_budget"])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("run memory_budget example");

    assert!(
        output.status.success(),
        "memory_budget failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("UTF-8 stdout");

    assert!(stdout.contains("RIG memory budget report"));
    assert!(stdout.contains("Status: PASSED"));
    assert!(stdout.contains("Status: FAILED"));
    assert!(stdout.contains("total_current_capacity"));
    assert!(stdout.contains("current_capacity"));
    assert!(stdout.contains("exceeded by"));
    assert!(stdout.contains("\"violations\""));
}
