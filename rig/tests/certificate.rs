use rig::{
    Arena, ArenaReport, ArtifactComparison, BudgetReport, CertificationSubject, ContractReport,
    EvidenceCertificate, MemoryBudget, ProfileReport, RegressionBudget, RegressionReport, RigVec,
    WorkloadContract,
};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn small_report(name: &str) -> ArenaReport {
    let mut arena = Arena::new(name);
    let mut values = RigVec::with_capacity(&mut arena, "values", 4);
    values.push(1);
    values.push(2);
    arena.snapshot()
}

fn grown_report(name: &str) -> ArenaReport {
    let mut arena = Arena::new(name);
    let mut values = RigVec::with_capacity(&mut arena, "values", 1);
    for value in 0..8 {
        values.push(value);
    }
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

fn failing_contract_report() -> ContractReport {
    let report = grown_report("contract-cert");
    let contract = WorkloadContract::new("level-1-memory-contract")
        .with_budget(MemoryBudget::max_total_capacity(1));
    report.check_contract(&contract)
}

fn hidden_entries(path: &Path) -> Vec<PathBuf> {
    fs::read_dir(path)
        .expect("read temp dir")
        .map(|entry| entry.expect("read temp entry").path())
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.starts_with('.'))
        })
        .collect()
}

#[test]
fn arena_report_fingerprint_is_deterministic_across_repeated_calls() {
    let report = small_report("deterministic");

    let first = report.fingerprint();
    let second = report.fingerprint();

    assert_eq!(first, second);
    assert_eq!(first.algorithm, "fnv1a64");
    assert!(!first.value.is_empty());
}

#[test]
fn different_reports_produce_different_fingerprints() {
    let first = small_report("different-a");
    let second = grown_report("different-b");

    assert_ne!(first.fingerprint(), second.fingerprint());
}

#[test]
fn same_report_serialized_and_deserialized_produces_same_fingerprint() {
    let report = grown_report("round-trip-report");
    let json = report.report_json();
    let decoded: ArenaReport = serde_json::from_str(&json).expect("decode arena report");

    assert_eq!(decoded, report);
    assert_eq!(decoded.fingerprint(), report.fingerprint());
}

#[test]
fn budget_report_fingerprint_is_deterministic() {
    let budget = MemoryBudget::max_total_capacity(1);
    let report: BudgetReport = grown_report("budget-fingerprint").check_budget(&budget);

    assert_eq!(report.fingerprint(), report.fingerprint());
    assert!(!report.violations.is_empty());
}

#[test]
fn regression_report_fingerprint_is_deterministic() {
    let baseline = small_report("regression-baseline");
    let current = grown_report("regression-current");
    let report: RegressionReport =
        current.check_regressions_against(&baseline, &RegressionBudget::strict());

    assert_eq!(report.fingerprint(), report.fingerprint());
    assert!(!report.regressions.is_empty());
}

#[test]
fn profile_report_fingerprint_is_deterministic() {
    let report: ProfileReport = small_report("profile-fingerprint").profile();

    assert_eq!(report.fingerprint(), report.fingerprint());
    assert_eq!(report.fingerprint().algorithm, "fnv1a64");
}

#[test]
fn contract_report_fingerprint_is_deterministic() {
    let report = failing_contract_report();

    assert_eq!(report.fingerprint(), report.fingerprint());
    assert_eq!(report.fingerprint().algorithm, "fnv1a64");
}

#[test]
fn artifact_comparison_fingerprint_is_deterministic() {
    let comparison = comparison(
        small_report("artifact-baseline"),
        grown_report("artifact-current"),
    );

    assert_eq!(comparison.fingerprint(), comparison.fingerprint());
    assert_eq!(comparison.fingerprint().algorithm, "fnv1a64");
}

#[test]
fn arena_report_certify_produces_passed_certificate() {
    let report = small_report("arena-cert");
    let certificate = report.certify(CertificationSubject::new("level-1"));

    assert!(certificate.passed);
    assert_eq!(certificate.subject.workload_name, "level-1");
    assert_eq!(certificate.report_fingerprint, report.fingerprint());
    assert_eq!(certificate.violation_count, 0);
    assert_eq!(certificate.budget_violation_count, 0);
    assert_eq!(certificate.regression_violation_count, 0);
    assert_eq!(certificate.profile_count, report.profile().profiles.len());
    assert!(certificate.summary.contains("Arena arena-cert observed"));
}

#[test]
fn contract_report_certify_preserves_failed_status_and_violation_counts() {
    let report = failing_contract_report();
    let certificate = report.certify(CertificationSubject::new("level-1"));

    assert!(!certificate.passed);
    assert_eq!(
        certificate.contract_name.as_deref(),
        Some("level-1-memory-contract")
    );
    assert_eq!(certificate.violation_count, report.violations.len());
    assert_eq!(
        certificate.budget_violation_count,
        report.budget_report.as_ref().unwrap().violations.len()
    );
    assert_eq!(certificate.regression_violation_count, 0);
    assert_eq!(certificate.report_fingerprint, report.fingerprint());
}

#[test]
fn artifact_comparison_certify_includes_contract_result_when_provided() {
    let comparison = comparison(
        small_report("artifact-contract-baseline"),
        grown_report("artifact-contract-current"),
    );
    let contract = WorkloadContract::new("artifact-contract")
        .with_regression_budget(RegressionBudget::strict());
    let contract_report = comparison.check_contract(&contract);
    let certificate =
        comparison.certify(CertificationSubject::new("level-1"), Some(&contract_report));

    assert_eq!(certificate.passed, contract_report.passed);
    assert_eq!(
        certificate.contract_name.as_deref(),
        Some("artifact-contract")
    );
    assert_eq!(
        certificate.violation_count,
        contract_report.violations.len()
    );
    assert_eq!(
        certificate.regression_violation_count,
        contract_report
            .regression_report
            .as_ref()
            .unwrap()
            .regressions
            .len()
    );
    assert!(certificate
        .summary
        .contains("contract artifact-contract was evaluated"));
}

#[test]
fn artifact_comparison_certify_without_contract_does_not_claim_gate() {
    let comparison = comparison(
        small_report("artifact-only-baseline"),
        grown_report("artifact-only-current"),
    );
    let certificate = comparison.certify(CertificationSubject::new("level-1"), None);

    assert!(certificate.passed);
    assert!(certificate.contract_name.is_none());
    assert_eq!(certificate.violation_count, 0);
    assert!(certificate.summary.contains("no contract was evaluated"));
}

#[test]
fn certificate_report_json_round_trips() {
    let certificate = failing_contract_report().certify(CertificationSubject::new("level-1"));
    let json = certificate.report_json();
    let decoded: EvidenceCertificate = serde_json::from_str(&json).expect("decode certificate");

    assert_eq!(decoded, certificate);
    assert_eq!(decoded.fingerprint(), certificate.fingerprint());
}

#[test]
fn certificate_human_report_includes_subject_status_fingerprints_counts_and_summary() {
    let certificate = failing_contract_report().certify(CertificationSubject::new("level-1"));
    let report = certificate.report();

    assert!(report.contains("RIG evidence certificate"));
    assert!(report.contains("Subject: level-1"));
    assert!(report.contains("Status: FAILED"));
    assert!(report.contains("Evidence fingerprint: fnv1a64:"));
    assert!(report.contains("Report fingerprint: fnv1a64:"));
    assert!(report.contains("Violations:"));
    assert!(report.contains("Budget violations:"));
    assert!(report.contains("Regression violations:"));
    assert!(report.contains("Profiles observed:"));
    assert!(report.contains("Summary:"));
    assert!(report.contains(&certificate.summary));
}

#[test]
fn evidence_certificate_example_runs_and_prints_certificate_data() {
    let output = Command::new(env!("CARGO"))
        .args(["run", "--quiet", "--example", "evidence_certificate"])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("run evidence_certificate example");

    assert!(
        output.status.success(),
        "example failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("example stdout utf8");

    assert!(stdout.contains("RIG evidence certificate"));
    assert!(stdout.contains("Evidence fingerprint"));
    assert!(stdout.contains("Report fingerprint"));
    assert!(stdout.contains("Status: PASSED"));
    assert!(stdout.contains("Status: FAILED"));
    assert!(stdout.contains("fnv1a64"));
}

#[test]
fn certification_apis_do_not_create_hidden_files_or_automatic_persistence() {
    let mut temp_dir = std::env::temp_dir();
    temp_dir.push(format!("rig-certificate-no-hidden-{}", std::process::id()));
    let _ = fs::remove_dir_all(&temp_dir);
    fs::create_dir(&temp_dir).expect("create isolated cwd");

    let before = hidden_entries(&temp_dir);
    let report = grown_report("no-persistence");
    let contract_report = failing_contract_report();
    let comparison = comparison(small_report("no-persistence-base"), report.clone());

    let arena_certificate = report.certify(CertificationSubject::new("arena"));
    let contract_certificate = contract_report.certify(CertificationSubject::new("contract"));
    let comparison_certificate = comparison.certify(
        CertificationSubject::new("comparison"),
        Some(&contract_report),
    );

    let after = hidden_entries(&temp_dir);
    let all_entries = fs::read_dir(&temp_dir)
        .expect("read isolated cwd")
        .collect::<Result<Vec<_>, _>>()
        .expect("collect isolated cwd entries");

    assert!(before.is_empty());
    assert!(after.is_empty());
    assert!(all_entries.is_empty());
    assert_eq!(arena_certificate.report_fingerprint, report.fingerprint());
    assert_eq!(
        contract_certificate.report_fingerprint,
        contract_report.fingerprint()
    );
    assert_eq!(
        comparison_certificate.report_fingerprint,
        comparison.fingerprint()
    );

    fs::remove_dir(&temp_dir).expect("remove isolated cwd");
}
