use rig::{
    Arena, ArenaReport, ArtifactComparison, ContractReport, GrowthPolicy, MemoryBudget,
    MemoryProfileKind, RegressionBudget, RigString, RigVec, WorkloadContract,
};
use std::path::PathBuf;
use std::process::Command;

fn small_report() -> ArenaReport {
    let mut arena = Arena::new("contract-small");
    let mut values = RigVec::with_capacity(&mut arena, "values", 4);
    values.push(1);
    arena.snapshot()
}

fn exact_growth_report() -> ArenaReport {
    let mut arena = Arena::new("contract-growth");
    let mut values = RigVec::with_policy(&mut arena, "values", GrowthPolicy::Exact);
    for value in 0..8 {
        values.push(value);
    }
    arena.snapshot()
}

fn large_jump_report() -> ArenaReport {
    let mut arena = Arena::new("contract-profile");
    let mut buffer = RigString::with_policy(
        &mut arena,
        "raw_log_buffer",
        GrowthPolicy::ReserveAhead(2_048),
    );
    buffer.push_str("log");
    arena.snapshot()
}

fn comparison(baseline: ArenaReport, current: ArenaReport) -> ArtifactComparison {
    let diff = baseline.diff(&current);
    ArtifactComparison {
        baseline_path: PathBuf::from("baseline.json"),
        current_path: PathBuf::from("current.json"),
        baseline,
        current,
        diff,
    }
}

fn violation_for<'a>(report: &'a ContractReport, rule: &str) -> &'a rig::ContractViolation {
    report
        .violations
        .iter()
        .find(|violation| violation.rule == rule)
        .unwrap_or_else(|| panic!("missing {rule} violation: {report:#?}"))
}

#[test]
fn empty_contract_passes() {
    let report = small_report().check_contract(&WorkloadContract::new("empty"));

    assert!(report.passed);
    assert!(report.violations.is_empty());
    assert!(report.budget_report.is_none());
    assert!(report.regression_report.is_none());
    assert!(report.profile_report.is_none());
}

#[test]
fn budget_contract_passes_when_report_is_inside_budget() {
    let observed = small_report();
    let contract = WorkloadContract::new("budget-pass").with_budget(
        MemoryBudget::max_total_capacity(observed.totals.total_current_capacity),
    );

    let report = observed.check_contract(&contract);

    assert!(report.passed, "unexpected contract violations: {report:#?}");
    let budget = report.budget_report.expect("budget evidence");
    assert!(budget.passed);
    assert!(budget.violations.is_empty());
}

#[test]
fn budget_contract_fails_with_typed_contract_violation() {
    let observed = exact_growth_report();
    let contract = WorkloadContract::new("budget-fail").with_budget(
        MemoryBudget::max_total_capacity(observed.totals.total_current_capacity - 1),
    );

    let report = observed.check_contract(&contract);

    assert!(!report.passed);
    let violation = violation_for(&report, "budget");
    assert_eq!(violation.contract_name, "budget-fail");
    assert_eq!(violation.subject, "arena");
    assert_eq!(violation.reason, "total_current_capacity exceeded limit");
    assert!(violation.evidence.contains("observed"));
    assert!(violation.evidence.contains("limit"));
}

#[test]
fn artifact_comparison_contract_runs_regression_budget() {
    let baseline = small_report();
    let current = exact_growth_report();
    let comparison = comparison(baseline, current);
    let contract =
        WorkloadContract::new("regression-fail").with_regression_budget(RegressionBudget::strict());

    let report = comparison.check_contract(&contract);

    assert!(!report.passed);
    let regression = report
        .regression_report
        .as_ref()
        .expect("regression evidence");
    assert!(!regression.passed);
    assert!(!regression.regressions.is_empty());
    let violation = violation_for(&report, "regression");
    assert!(violation.reason.contains("delta exceeded allowed delta"));
    assert!(violation.evidence.contains("baseline"));
    assert!(violation.evidence.contains("current"));
}

#[test]
fn profile_absent_fails_when_forbidden_profile_exists() {
    let contract = WorkloadContract::new("profile-absent")
        .require_profile_absent(MemoryProfileKind::LargeSingleJump);

    let report = large_jump_report().check_contract(&contract);

    assert!(!report.passed);
    let profile_report = report.profile_report.as_ref().expect("profile evidence");
    assert!(!profile_report
        .profiles_by_kind(MemoryProfileKind::LargeSingleJump)
        .is_empty());
    let violation = violation_for(&report, "profile_absent");
    assert_eq!(violation.subject, "raw_log_buffer");
    assert!(violation
        .reason
        .contains("forbidden profile LargeSingleJump"));
    assert!(violation.evidence.contains("threshold="));
}

#[test]
fn profile_present_passes_when_required_profile_exists() {
    let contract = WorkloadContract::new("profile-present")
        .require_profile_present(MemoryProfileKind::LargeSingleJump);

    let report = large_jump_report().check_contract(&contract);

    assert!(report.passed, "unexpected profile violation: {report:#?}");
    assert!(report.profile_report.is_some());
    assert!(report.violations.is_empty());
}

#[test]
fn profile_present_fails_when_required_profile_is_missing() {
    let contract = WorkloadContract::new("profile-missing")
        .require_profile_present(MemoryProfileKind::LargeSingleJump);

    let report = small_report().check_contract(&contract);

    assert!(!report.passed);
    let violation = violation_for(&report, "profile_present");
    assert_eq!(violation.subject, "profile-missing");
    assert!(violation
        .reason
        .contains("required profile LargeSingleJump was missing"));
    assert!(violation.evidence.contains("matching_kind_count=0"));
}

#[test]
fn no_rule_contract_does_not_invent_violations() {
    let observed = large_jump_report();
    assert!(!observed.profile().profiles.is_empty());

    let report = observed.check_contract(&WorkloadContract::new("no-rules"));

    assert!(report.passed);
    assert!(report.violations.is_empty());
    assert!(report.profile_report.is_none());
}

#[test]
fn contract_report_json_round_trip_works() {
    let observed = exact_growth_report();
    let contract = WorkloadContract::new("round-trip").with_budget(
        MemoryBudget::max_total_capacity(observed.totals.total_current_capacity - 1),
    );
    let report = observed.check_contract(&contract);

    let decoded: ContractReport =
        serde_json::from_str(&report.report_json()).expect("contract JSON decodes");

    assert_eq!(decoded, report);
    assert!(!decoded.passed);
    assert_eq!(decoded.violations[0].rule, "budget");
}

#[test]
fn human_report_includes_passed_and_failed_status() {
    let passed = small_report()
        .check_contract(&WorkloadContract::new("passed"))
        .report();
    let failed = exact_growth_report()
        .check_contract(
            &WorkloadContract::new("failed").with_budget(MemoryBudget::max_total_capacity(0)),
        )
        .report();

    assert!(passed.contains("RIG workload contract report"));
    assert!(passed.contains("Status: PASSED"));
    assert!(passed.contains("Violations:"));
    assert!(failed.contains("Status: FAILED"));
    assert!(failed.contains("budget"));
}

#[test]
fn workload_contract_example_runs_and_prints_contract_evidence() {
    let output = Command::new("cargo")
        .args(["run", "--quiet", "--example", "workload_contract"])
        .output()
        .expect("run workload_contract example");

    assert!(output.status.success(), "example failed: {output:#?}");
    let stdout = String::from_utf8(output.stdout).expect("stdout is UTF-8");

    for expected in [
        "RIG workload contract report",
        "Contract:",
        "Status: PASSED",
        "Status: FAILED",
        "budget",
        "regression",
        "profile_absent",
        "violations",
    ] {
        assert!(
            stdout.contains(expected),
            "stdout missing {expected:?}:\n{stdout}"
        );
    }
}
