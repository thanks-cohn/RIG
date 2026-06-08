use rig::{
    AllocationBudget, Arena, BenchmarkEvidence, ContainerBudget, GrowthPolicy,
    GrowthProfileExpectation, MemoryProfileKind, RegressionBudget, RegressionExpectation,
    RigString, RigVec, WorkloadMemoryContract,
};
use std::process::Command;

fn exact_report(name: &str, count: usize) -> rig::ArenaReport {
    let mut arena = Arena::new(name);
    let mut values = RigVec::with_policy(&mut arena, "values", GrowthPolicy::Exact);
    for value in 0..count {
        values.push(value);
    }
    arena.snapshot()
}

fn large_jump_report() -> rig::ArenaReport {
    let mut arena = Arena::new("large-jump");
    let mut buffer =
        RigString::with_policy(&mut arena, "buffer", GrowthPolicy::ReserveAhead(2_048));
    buffer.push_str("payload");
    arena.snapshot()
}

#[test]
fn doctrine_passes_when_observed_evidence_stays_inside_allocation_and_container_budgets() {
    let evidence = BenchmarkEvidence::from_current("pass", exact_report("pass", 3));
    let contract = WorkloadMemoryContract::new("pass-contract")
        .with_allocation_budget(
            AllocationBudget::unlimited()
                .with_max_arena_capacity(evidence.current.totals.total_current_capacity)
                .with_max_growth_events(evidence.current.totals.total_growth_events),
        )
        .with_container_budget(
            ContainerBudget::named("values")
                .with_max_capacity(evidence.current.containers[0].current_capacity)
                .with_max_capacity_expansion(
                    evidence.current.growth_summary().largest_growth_delta,
                ),
        );

    let report = contract.validate(&evidence);

    assert!(report.passed, "unexpected violations: {report:#?}");
    assert!(report.report_json().contains("pass-contract"));
    assert!(
        report
            .certify(rig::CertificationSubject::new("pass"))
            .passed
    );
}

#[test]
fn doctrine_reports_budget_violations_with_evidence_references() {
    let evidence = BenchmarkEvidence::from_current("fail", exact_report("fail", 8));
    let contract = WorkloadMemoryContract::new("budget-fail")
        .with_allocation_budget(
            AllocationBudget::unlimited()
                .with_max_arena_capacity(1)
                .with_max_capacity_expansion(0),
        )
        .with_container_budget(ContainerBudget::named("values").with_max_growth_events(1));

    let report = contract.validate(&evidence);

    assert!(!report.passed);
    assert!(report
        .violations
        .iter()
        .any(|violation| violation.category == "allocation_budget"
            && violation.explanation.contains("total_current_capacity")));
    assert!(report.violations.iter().any(|violation| violation
        .evidence_references
        .iter()
        .any(|reference| reference.source == "growth_event")));
}

#[test]
fn doctrine_detects_regressions_from_real_baseline_and_current_measurements() {
    let evidence = BenchmarkEvidence::compare(
        "comparison",
        exact_report("base", 2),
        exact_report("cur", 12),
    );
    let contract = WorkloadMemoryContract::new("regression").with_regression_expectation(
        RegressionExpectation::new("strict", RegressionBudget::strict()),
    );

    let report = contract.validate(&evidence);

    assert!(!report.passed);
    assert_eq!(report.regression_reports.len(), 1);
    assert!(report.regression_reports[0]
        .regressions
        .iter()
        .any(|regression| regression.delta > regression.allowed_delta));
    assert!(evidence.diff.as_ref().expect("diff").total_capacity_delta > 0);
}

#[test]
fn doctrine_requires_baseline_for_regression_expectations() {
    let evidence = BenchmarkEvidence::from_current("single", exact_report("single", 2));
    let contract = WorkloadMemoryContract::new("needs-baseline").with_regression_expectation(
        RegressionExpectation::new("strict", RegressionBudget::strict()),
    );

    let report = contract.validate(&evidence);

    assert!(!report.passed);
    assert!(report
        .violations
        .iter()
        .any(|violation| violation.explanation.contains("had no baseline")));
}

#[test]
fn doctrine_validates_required_and_forbidden_growth_profiles() {
    let evidence = BenchmarkEvidence::from_current("profile", large_jump_report());
    let contract = WorkloadMemoryContract::new("profile-contract").with_growth_profile_expectation(
        GrowthProfileExpectation::empty()
            .require(MemoryProfileKind::LargeSingleJump)
            .forbid(MemoryProfileKind::Stable),
    );

    let report = contract.validate(&evidence);

    assert!(report.passed, "unexpected violations: {report:#?}");
    assert!(report
        .profile_report
        .profiles_by_kind(MemoryProfileKind::LargeSingleJump)
        .iter()
        .any(|profile| profile.evidence_value >= 2_048));
}

#[test]
fn doctrine_example_exercises_all_required_scenarios() {
    let output = Command::new("cargo")
        .args(["run", "--example", "memory_doctrine"])
        .output()
        .expect("run memory_doctrine example");

    assert!(output.status.success(), "example failed: {output:#?}");
    let stdout = String::from_utf8(output.stdout).expect("stdout UTF-8");
    for expected in [
        "Successful workload certification",
        "Failed workload certification",
        "Budget enforcement",
        "Regression detection",
        "Benchmark comparison",
        "Growth profile validation",
        "JSON evidence",
    ] {
        assert!(
            stdout.contains(expected),
            "stdout missing {expected}:\n{stdout}"
        );
    }
}
