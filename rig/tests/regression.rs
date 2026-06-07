use rig::{Arena, ArenaReport, GrowthPolicy, RegressionBudget, RegressionReport, RigVec};
use std::process::Command;

fn exact_report(name: &str, container_name: &str, pushes: usize) -> ArenaReport {
    let mut arena = Arena::new(name);
    let mut values = RigVec::with_policy(&mut arena, container_name, GrowthPolicy::Exact);
    for value in 0..pushes {
        values.push(value);
    }
    arena.snapshot()
}

fn strict_without_total_capacity() -> RegressionBudget {
    let mut budget = RegressionBudget::strict();
    budget.max_total_capacity_delta = None;
    budget
}

fn strict_without_total_growth_events() -> RegressionBudget {
    let mut budget = RegressionBudget::strict();
    budget.max_total_growth_events_delta = None;
    budget
}

fn regression_for<'a>(
    report: &'a RegressionReport,
    container_name: &str,
    metric: &str,
) -> &'a rig::MemoryRegression {
    report
        .regressions
        .iter()
        .find(|regression| {
            regression.container_name == container_name && regression.metric == metric
        })
        .unwrap_or_else(|| panic!("missing regression for {container_name}.{metric}: {report:#?}"))
}

#[test]
fn regression_strict_budget_passes_identical_reports() {
    let baseline = exact_report("baseline", "values", 4);
    let current = baseline.clone();

    let report = current.check_regressions_against(&baseline, &RegressionBudget::strict());

    assert!(report.passed);
    assert!(report.regressions.is_empty());
    assert_eq!(report.total_capacity_delta, 0);
    assert_eq!(report.total_growth_event_delta, 0);
}

#[test]
fn regression_strict_budget_fails_capacity_increase() {
    let baseline = exact_report("baseline", "values", 2);
    let current = exact_report("current", "values", 5);

    let report =
        current.check_regressions_against(&baseline, &strict_without_total_growth_events());

    assert!(!report.passed);
    let regression = regression_for(&report, "values", "current_capacity");
    assert_eq!(regression.baseline, 2);
    assert_eq!(regression.current, 5);
    assert_eq!(regression.delta, 3);
    assert_eq!(regression.allowed_delta, 0);
    assert!(report.total_capacity_delta > 0);
}

#[test]
fn regression_strict_budget_fails_growth_event_increase() {
    let baseline = exact_report("baseline", "values", 2);
    let current = exact_report("current", "values", 5);

    let report = current.check_regressions_against(&baseline, &strict_without_total_capacity());

    assert!(!report.passed);
    let regression = regression_for(&report, "values", "growth_events");
    assert_eq!(regression.baseline, 2);
    assert_eq!(regression.current, 5);
    assert_eq!(regression.delta, 3);
    assert_eq!(regression.allowed_delta, 0);
    assert!(report.total_growth_event_delta > 0);
}

#[test]
fn regression_allowed_capacity_delta_can_pass_small_capacity_increase() {
    let baseline = exact_report("baseline", "values", 2);
    let current = exact_report("current", "values", 5);
    let mut budget = RegressionBudget::allow_capacity_delta(3);
    budget.max_total_growth_events_delta = None;
    budget.max_container_growth_events_delta = None;

    let report = current.check_regressions_against(&baseline, &budget);

    assert!(report.passed, "unexpected regressions: {report:#?}");
    assert_eq!(report.total_capacity_delta, 3);
    assert!(report.regressions.is_empty());
}

#[test]
fn regression_allowed_growth_events_delta_can_pass_small_growth_increase() {
    let baseline = exact_report("baseline", "values", 2);
    let current = exact_report("current", "values", 5);
    let mut budget = RegressionBudget::allow_growth_events_delta(3);
    budget.max_total_capacity_delta = None;
    budget.max_container_capacity_delta = None;

    let report = current.check_regressions_against(&baseline, &budget);

    assert!(report.passed, "unexpected regressions: {report:#?}");
    assert_eq!(report.total_growth_event_delta, 3);
    assert!(report.regressions.is_empty());
}

#[test]
fn regression_new_container_is_treated_as_growth_from_zero() {
    let mut baseline_arena = Arena::new("baseline");
    let mut existing = RigVec::with_policy(&mut baseline_arena, "existing", GrowthPolicy::Exact);
    existing.push(1);
    let baseline = baseline_arena.snapshot();

    let mut current_arena = Arena::new("current");
    let mut existing_current =
        RigVec::with_policy(&mut current_arena, "existing", GrowthPolicy::Exact);
    existing_current.push(1);
    let mut added = RigVec::with_policy(&mut current_arena, "added", GrowthPolicy::Exact);
    added.push(1);
    added.push(2);
    let current = current_arena.snapshot();

    let report = current.check_regressions_against(&baseline, &RegressionBudget::strict());

    assert!(!report.passed);
    let capacity = regression_for(&report, "added", "current_capacity");
    assert_eq!(capacity.baseline, 0);
    assert_eq!(capacity.current, 2);
    assert_eq!(capacity.delta, 2);
    let growth_events = regression_for(&report, "added", "growth_events");
    assert_eq!(growth_events.baseline, 0);
    assert_eq!(growth_events.current, 2);
}

#[test]
fn regression_removed_container_does_not_fail() {
    let mut baseline_arena = Arena::new("baseline");
    let mut removed = RigVec::with_policy(&mut baseline_arena, "removed", GrowthPolicy::Exact);
    removed.push(1);
    removed.push(2);
    let baseline = baseline_arena.snapshot();

    let current = Arena::new("current").snapshot();
    let report = current.check_regressions_against(&baseline, &RegressionBudget::strict());

    assert!(
        report.passed,
        "removed container falsely failed: {report:#?}"
    );
    assert!(report.regressions.is_empty());
    assert_eq!(report.total_capacity_delta, -2);
    assert_eq!(report.total_growth_event_delta, -2);
}

#[test]
fn regression_json_round_trip_works() {
    let baseline = exact_report("baseline", "values", 1);
    let current = exact_report("current", "values", 3);
    let report = current.check_regressions_against(&baseline, &RegressionBudget::strict());

    let decoded: RegressionReport =
        serde_json::from_str(&report.report_json()).expect("valid regression JSON");

    assert_eq!(decoded, report);
    assert!(!decoded.passed);
    assert!(!decoded.regressions.is_empty());
}

#[test]
fn regression_human_report_includes_passed_and_failed_statuses() {
    let baseline = exact_report("baseline", "values", 1);
    let identical = baseline.clone();
    let failed = exact_report("current", "values", 3);

    let passed_report = identical
        .check_regressions_against(&baseline, &RegressionBudget::strict())
        .report();
    let failed_report = failed
        .check_regressions_against(&baseline, &RegressionBudget::strict())
        .report();

    assert!(passed_report.contains("Status: PASSED"));
    assert!(passed_report.contains("Regressions:"));
    assert!(passed_report.contains("(none)"));
    assert!(failed_report.contains("Status: FAILED"));
    assert!(failed_report.contains("metric: current_capacity"));
    assert!(failed_report.contains("allowed delta: 0"));
}

#[test]
fn regression_gate_example_runs_and_outputs_real_regression_fields() {
    let output = Command::new(env!("CARGO"))
        .args(["run", "--quiet", "--example", "regression_gate"])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("run regression_gate example");

    assert!(
        output.status.success(),
        "regression_gate failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("UTF-8 stdout");

    assert!(stdout.contains("RIG memory regression report"));
    assert!(stdout.contains("Status: FAILED"));
    assert!(stdout.contains("Status: PASSED"));
    assert!(stdout.contains("parsed_lines"));
    assert!(stdout.contains("metric: current_capacity"));
    assert!(stdout.contains("baseline:"));
    assert!(stdout.contains("current:"));
    assert!(stdout.contains("delta:"));
    assert!(stdout.contains("allowed delta:"));
    assert!(stdout.contains("\"regressions\""));
    assert!(stdout.contains("\"total_capacity_delta\""));
    assert!(stdout.contains("\"total_growth_event_delta\""));
}
