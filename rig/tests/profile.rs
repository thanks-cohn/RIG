use rig::{
    Arena, ArenaReport, BudgetReport, GrowthPolicy, MemoryBudget, MemoryProfileKind, ProfileReport,
    RegressionBudget, RegressionReport, RigString, RigVec,
};
use std::process::Command;

fn has_kind(report: &ProfileReport, kind: MemoryProfileKind) -> bool {
    !report.profiles_by_kind(kind).is_empty()
}

fn profile_for(report: &ProfileReport, kind: MemoryProfileKind) -> &rig::MemoryProfile {
    report
        .profiles_by_kind(kind)
        .into_iter()
        .next()
        .unwrap_or_else(|| panic!("missing profile {kind:?}: {report:#?}"))
}

fn exact_tiny_report() -> ArenaReport {
    let mut arena = Arena::new("tiny-growth");
    let mut values = RigVec::with_policy(&mut arena, "exact_values", GrowthPolicy::Exact);
    for value in 0..12 {
        values.push(value);
    }
    arena.snapshot()
}

#[test]
fn profile_no_growth_report_produces_stable() {
    let mut arena = Arena::new("stable-workload");
    let mut values = RigVec::with_capacity(&mut arena, "preallocated", 8);
    values.push(1);
    values.push(2);

    let profile = arena.snapshot().profile();
    let stable = profile_for(&profile, MemoryProfileKind::Stable);

    assert_eq!(stable.subject, "stable-workload");
    assert_eq!(stable.evidence_metric, "totals.total_growth_events");
    assert_eq!(stable.evidence_value, 0);
    assert_eq!(stable.threshold, 0);
    assert!(stable.reason.contains("0 growth events"));
}

#[test]
fn profile_exact_policy_workload_produces_frequent_tiny_growth() {
    let observed = exact_tiny_report();
    assert_eq!(observed.totals.total_growth_events, 12);

    let profile = observed.profile();
    let frequent = profile_for(&profile, MemoryProfileKind::FrequentTinyGrowth);

    assert_eq!(frequent.subject, "tiny-growth");
    assert_eq!(frequent.evidence_metric, "average_growth_jump");
    assert!(frequent.evidence_value <= frequent.threshold);
    assert!(frequent.reason.contains("at least 8 events"));
}

#[test]
fn profile_large_reserve_ahead_string_produces_large_single_jump() {
    let mut arena = Arena::new("large-jump");
    let mut text = RigString::with_policy(
        &mut arena,
        "large_buffer",
        GrowthPolicy::ReserveAhead(2_048),
    );
    text.push_str("large");

    let profile = arena.snapshot().profile();
    let large = profile_for(&profile, MemoryProfileKind::LargeSingleJump);

    assert_eq!(large.subject, "large_buffer");
    assert_eq!(large.evidence_metric, "growth_history.capacity_added");
    assert!(large.evidence_value >= large.threshold);
    assert_eq!(large.threshold, 1_024);
}

#[test]
fn profile_high_capacity_low_len_produces_over_reserved() {
    let mut arena = Arena::new("over-reserved");
    let mut values = RigVec::with_capacity(&mut arena, "mostly_empty", 64);
    values.push(1);

    let profile = arena.snapshot().profile();
    let over = profile_for(&profile, MemoryProfileKind::OverReserved);

    assert_eq!(over.subject, "mostly_empty");
    assert_eq!(over.evidence_metric, "current_capacity");
    assert_eq!(over.evidence_value, 64);
    assert!(over.reason.contains("at least 4x len"));
}

#[test]
fn profile_many_growth_events_produce_under_reserved() {
    let observed = exact_tiny_report();

    let profile = observed.profile();
    let under = profile_for(&profile, MemoryProfileKind::UnderReserved);

    assert_eq!(under.subject, "exact_values");
    assert_eq!(under.evidence_metric, "len_per_growth_event");
    assert!(under.evidence_value <= under.threshold);
    assert!(under.reason.contains("growth events for len"));
}

#[test]
fn profile_concentrated_growth_produces_burst_growth() {
    let mut arena = Arena::new("burst");
    let mut hot = RigString::with_policy(&mut arena, "hot_buffer", GrowthPolicy::ReserveAhead(128));
    hot.push_str("hot");
    let mut cold = RigVec::with_capacity(&mut arena, "cold_values", 16);
    cold.push(1);

    let profile = arena.snapshot().profile();
    let burst = profile_for(&profile, MemoryProfileKind::BurstGrowth);

    assert_eq!(burst.subject, "hot_buffer");
    assert_eq!(
        burst.evidence_metric,
        "top_container_capacity_added_percent"
    );
    assert!(burst.evidence_value >= burst.threshold);
    assert_eq!(burst.threshold, 80);
}

#[test]
fn profile_failed_budget_report_produces_budget_risk() {
    let observed = exact_tiny_report();
    let budget = observed.check_budget(&MemoryBudget::max_total_growth_events(0));
    assert!(!budget.passed);

    let profile = budget.profile();
    let risk = profile_for(&profile, MemoryProfileKind::BudgetRisk);

    assert_eq!(risk.evidence_metric, "total_growth_events");
    assert_eq!(risk.evidence_value, observed.totals.total_growth_events);
    assert_eq!(risk.threshold, 0);
    assert!(risk.reason.contains("exceeded limit"));
}

#[test]
fn profile_failed_regression_report_produces_regression_risk() {
    let mut baseline_arena = Arena::new("regression");
    let mut baseline_values = RigVec::with_capacity(&mut baseline_arena, "values", 4);
    baseline_values.push(1);
    let baseline = baseline_arena.snapshot();

    let mut current_arena = Arena::new("regression");
    let mut current_values = RigVec::with_policy(&mut current_arena, "values", GrowthPolicy::Exact);
    for value in 0..10 {
        current_values.push(value);
    }
    let current = current_arena.snapshot();

    let regression = current.check_regressions_against(&baseline, &RegressionBudget::strict());
    assert!(!regression.passed);

    let profile = regression.profile();
    let risk = profile_for(&profile, MemoryProfileKind::RegressionRisk);

    assert!(
        risk.subject == "values" || risk.subject == "total",
        "unexpected regression profile subject: {}",
        risk.subject
    );
    assert!(risk.evidence_value > risk.threshold);
    assert!(risk.reason.contains("exceeded allowed delta"));
}

#[test]
fn profile_json_round_trip_works() {
    let profile = exact_tiny_report().profile();

    let decoded: ProfileReport =
        serde_json::from_str(&profile.report_json()).expect("profile JSON should decode");

    assert_eq!(decoded, profile);
    assert!(has_kind(&decoded, MemoryProfileKind::FrequentTinyGrowth));
    assert!(has_kind(&decoded, MemoryProfileKind::UnderReserved));
}

#[test]
fn profile_human_report_includes_profile_names_and_reasons() {
    let human = exact_tiny_report().profile().report();

    assert!(human.contains("RIG evidence profile report"));
    assert!(human.contains("FrequentTinyGrowth"));
    assert!(human.contains("UnderReserved"));
    assert!(human.contains("reason:"));
    assert!(human.contains("evidence metric:"));
}

#[test]
fn profile_artifact_comparison_combines_current_and_diff_risk_evidence() {
    let mut temp_dir = std::env::temp_dir();
    temp_dir.push(format!("rig-profile-artifact-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&temp_dir);
    std::fs::create_dir(&temp_dir).expect("create artifact temp dir");

    let baseline = stable_report_for_artifact();
    let current = exact_tiny_report();
    let baseline_path = temp_dir.join("baseline.json");
    let current_path = temp_dir.join("current.json");
    baseline
        .write_artifact(&baseline_path)
        .expect("write baseline artifact");
    current
        .write_artifact(&current_path)
        .expect("write current artifact");

    let baseline_artifact = rig::ReportArtifact::load(&baseline_path).expect("load baseline");
    let current_artifact = rig::ReportArtifact::load(&current_path).expect("load current");
    let comparison = baseline_artifact.compare_to(&current_artifact);

    let profile = comparison.profile();

    assert!(has_kind(&profile, MemoryProfileKind::FrequentTinyGrowth));
    assert!(has_kind(&profile, MemoryProfileKind::RegressionRisk));
    assert!(profile.profiles.iter().any(|profile| {
        profile.evidence_metric == "diff.total_capacity_delta" && profile.evidence_value > 0
    }));

    std::fs::remove_dir_all(&temp_dir).expect("remove artifact temp dir");
}

fn stable_report_for_artifact() -> ArenaReport {
    let mut arena = Arena::new("artifact-baseline");
    let mut values = RigVec::with_capacity(&mut arena, "exact_values", 8);
    values.push(1);
    arena.snapshot()
}

#[test]
fn profile_budget_and_regression_passed_reports_have_no_risk_profiles() {
    let budget = BudgetReport {
        passed: true,
        violations: Vec::new(),
    };
    let regression = RegressionReport {
        passed: true,
        regressions: Vec::new(),
        total_capacity_delta: 0,
        total_growth_event_delta: 0,
    };

    assert!(budget.profile().profiles.is_empty());
    assert!(regression.profile().profiles.is_empty());
}

#[test]
fn evidence_profiles_example_runs_and_stdout_contains_key_profile_names() {
    let output = Command::new(env!("CARGO"))
        .args(["run", "--quiet", "--example", "evidence_profiles"])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("run evidence_profiles example");

    assert!(
        output.status.success(),
        "evidence_profiles failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("UTF-8 stdout");

    for expected in [
        "Stable",
        "FrequentTinyGrowth",
        "LargeSingleJump",
        "BudgetRisk",
        "RegressionRisk",
        "RIG evidence profile report",
        "JSON profile report",
    ] {
        assert!(
            stdout.contains(expected),
            "stdout missing {expected:?}:\n{stdout}"
        );
    }
}
