use rig::{
    Arena, ExportFormat, MemoryBudget, RegressionBudget, ReportArtifact, RigString, RigVec,
    WorkloadContract,
};
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

fn unique_temp_dir(test_name: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time should be after UNIX_EPOCH")
        .as_nanos();
    let path = std::env::temp_dir().join(format!(
        "rig-abuse-{}-{test_name}-{nanos}",
        std::process::id()
    ));
    fs::create_dir_all(&path).expect("temp directory should be created");
    path
}

fn directory_entries(path: &Path) -> BTreeSet<String> {
    fs::read_dir(path)
        .expect("directory should be readable")
        .map(|entry| {
            entry
                .expect("directory entry should be readable")
                .file_name()
                .to_string_lossy()
                .into_owned()
        })
        .collect()
}

fn hidden_entries(path: &Path) -> Vec<String> {
    directory_entries(path)
        .into_iter()
        .filter(|entry| entry.starts_with('.'))
        .collect()
}

#[test]
fn abuse_empty_arena_reports_are_real_zero_evidence() {
    let arena = Arena::new("empty");
    let report = arena.snapshot();

    assert_eq!(report.arena_name, "empty");
    assert_eq!(report.tracked_container_count, 0);
    assert_eq!(report.totals.total_len, 0);
    assert_eq!(report.totals.total_current_capacity, 0);
    assert_eq!(report.totals.total_growth_events, 0);
    assert_eq!(report.totals.total_pushed_appended_operations, 0);
    assert!(report.containers.is_empty());
    assert!(report.growth_history.is_empty());
    assert!(report.growth_attributions.is_empty());
    assert!(report.report().contains("Tracked containers: 0"));

    let decoded: rig::ArenaReport = serde_json::from_str(&report.report_json()).unwrap();
    assert_eq!(decoded, report);
}

#[test]
fn abuse_container_names_preserve_empty_special_and_huge_values() {
    let huge_name = "container-".repeat(8192);
    let special_name = "name, with comma \"quote\" and\nnewline";
    let mut arena = Arena::new("names");
    let mut empty = RigVec::new(&mut arena, "");
    let mut special = RigString::new(&mut arena, special_name);
    let mut huge = RigVec::new(&mut arena, huge_name.clone());

    empty.push(1);
    special.push_str("payload");
    huge.push(7);

    let report = arena.snapshot();
    assert_eq!(report.containers[0].name, "");
    assert_eq!(report.containers[1].name, special_name);
    assert_eq!(report.containers[2].name, huge_name);

    let csv = report.containers_csv();
    assert!(csv.contains("\"name, with comma \"\"quote\"\" and\nnewline\""));
    assert!(csv.contains(&huge_name));

    let decoded: rig::ArenaReport = serde_json::from_str(&report.report_json()).unwrap();
    assert_eq!(decoded.containers[0].name, "");
    assert_eq!(decoded.containers[1].name, special_name);
    assert_eq!(decoded.containers[2].name, huge_name);
}

#[test]
fn abuse_missing_invalid_and_missing_parent_file_errors_are_typed_or_io() {
    let temp = unique_temp_dir("file-errors");
    let missing = temp.join("missing.json");
    let invalid = temp.join("invalid.json");
    let missing_parent = temp.join("missing-parent").join("report.json");

    assert!(matches!(
        Arena::load_report(&missing),
        Err(rig::RigIoError::Io(_))
    ));
    assert!(matches!(
        ReportArtifact::load(&missing),
        Err(rig::RigIoError::Io(_))
    ));

    fs::write(&invalid, "{ this is not valid json").unwrap();
    assert!(matches!(
        Arena::load_report(&invalid),
        Err(rig::RigIoError::Json(_))
    ));
    assert!(matches!(
        ReportArtifact::load(&invalid),
        Err(rig::RigIoError::Json(_))
    ));

    let report = Arena::new("missing-parent").snapshot();
    assert!(report.write_json(&missing_parent).is_err());
    assert!(report.write_artifact(&missing_parent).is_err());
    assert!(report
        .export_containers(ExportFormat::Csv)
        .write_to(&missing_parent)
        .is_err());

    fs::remove_dir_all(temp).unwrap();
}

#[test]
fn abuse_repeated_artifact_and_export_writes_overwrite_explicit_paths_only() {
    let temp = unique_temp_dir("repeated-writes");
    let artifact_path = temp.join("report.json");
    let export_path = temp.join("containers.csv");

    let mut arena = Arena::new("first");
    let mut values = RigVec::new(&mut arena, "values");
    values.push(1);
    let first = arena.snapshot();
    let first_artifact = first.write_artifact(&artifact_path).unwrap();
    first
        .export_containers(ExportFormat::Csv)
        .write_to(&export_path)
        .unwrap();

    let mut arena = Arena::new("second");
    let mut values = RigVec::new(&mut arena, "values");
    values.push(1);
    values.push(2);
    let second = arena.snapshot();
    let second_artifact = second.write_artifact(&artifact_path).unwrap();
    second
        .export_growth_history(ExportFormat::JsonLines)
        .write_to(&export_path)
        .unwrap();

    assert_eq!(first_artifact.path, artifact_path);
    assert_eq!(second_artifact.path, artifact_path);
    assert_eq!(ReportArtifact::load(&artifact_path).unwrap().report, second);
    assert_eq!(
        fs::read_to_string(&export_path).unwrap(),
        second.growth_history_jsonl()
    );
    assert_eq!(
        directory_entries(&temp),
        BTreeSet::from(["containers.csv".into(), "report.json".into()])
    );
    assert!(hidden_entries(&temp).is_empty());

    fs::remove_dir_all(temp).unwrap();
}

#[test]
fn abuse_empty_artifact_comparison_budgets_regressions_profiles_contracts_and_exports_are_stable() {
    let temp = unique_temp_dir("empty-evidence");
    let baseline_path = temp.join("baseline.json");
    let current_path = temp.join("current.json");
    let baseline = Arena::new("baseline-empty").snapshot();
    let current = Arena::new("current-empty").snapshot();
    let baseline_artifact = baseline.write_artifact(&baseline_path).unwrap();
    let current_artifact = current.write_artifact(&current_path).unwrap();

    let comparison = baseline_artifact.compare_to(&current_artifact);
    assert!(comparison.diff.containers_added.is_empty());
    assert!(comparison.diff.containers_removed.is_empty());
    assert!(comparison.diff.containers_changed.is_empty());
    assert_eq!(comparison.diff.total_len_delta, 0);
    assert_eq!(comparison.diff.total_capacity_delta, 0);
    assert_eq!(comparison.diff.total_growth_event_delta, 0);

    let budget_report = current.check_budget(&MemoryBudget::strict_zero_growth());
    assert!(budget_report.passed);
    assert!(budget_report.violations.is_empty());

    let regression_report =
        current.check_regressions_against(&baseline, &RegressionBudget::strict());
    assert!(regression_report.passed);
    assert!(regression_report.regressions.is_empty());

    let profile_report = current.profile();
    assert_eq!(profile_report.profiles.len(), 1);
    assert_eq!(profile_report.profiles[0].subject, "current-empty");

    let empty_contract = WorkloadContract::new("no-rules");
    let contract_report = current.check_contract(&empty_contract);
    assert!(contract_report.passed);
    assert!(contract_report.violations.is_empty());
    assert!(contract_report.budget_report.is_none());
    assert!(contract_report.regression_report.is_none());
    assert!(contract_report.profile_report.is_none());

    assert_eq!(current.containers_csv(), "name,kind,len,initial_capacity,growth_policy,current_capacity,growth_events,total_capacity_added,largest_growth_jump,average_growth_jump,operation_label,total_operations,extra_metric_label,extra_metric_value\n");
    assert_eq!(current.growth_history_csv(), "container_name,container_kind,old_capacity,new_capacity,operation_index,capacity_added,growth_policy\n");
    assert_eq!(
        current.growth_attributions_csv(),
        "container_name,operation_index,old_capacity,new_capacity,capacity_added,growth_policy\n"
    );
    assert!(current.containers_jsonl().is_empty());
    assert!(current.growth_history_jsonl().is_empty());
    assert!(current.growth_attributions_jsonl().is_empty());

    fs::remove_dir_all(temp).unwrap();
}

#[test]
fn abuse_in_memory_operations_create_no_hidden_files_and_no_automatic_persistence() {
    let temp = unique_temp_dir("no-hidden-or-auto-persistence");
    let before = directory_entries(&temp);

    let mut arena = Arena::new("explicit-only");
    let mut values = RigVec::new(&mut arena, "values");
    values.push(1);
    values.push(2);
    let report = arena.snapshot();
    let _ = report.report();
    let _ = report.report_verbose();
    let _ = report.report_json();
    let _ = report.growth_summary();
    let _ = report.check_budget(&MemoryBudget::unlimited());
    let _ = report.profile();
    let _ = report.export_containers(ExportFormat::Csv);
    let _ = report.export_growth_history(ExportFormat::JsonLines);
    let _ = WorkloadContract::new("no-rules");

    let after = directory_entries(&temp);
    assert_eq!(after, before);
    assert!(hidden_entries(&temp).is_empty());

    fs::remove_dir_all(temp).unwrap();
}
