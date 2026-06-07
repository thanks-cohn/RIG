use rig::{Arena, MemoryBudget, RegressionBudget, ReportArtifact, RigVec};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT_ID: AtomicUsize = AtomicUsize::new(0);

fn temp_artifact_dir(name: &str) -> PathBuf {
    let id = NEXT_ID.fetch_add(1, Ordering::SeqCst);
    let dir = std::env::temp_dir().join(format!(
        "rig-artifact-test-{name}-{}-{id}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir(&dir).expect("create artifact test directory");
    dir
}

fn exact_report(arena_name: &str, pushes: usize) -> rig::ArenaReport {
    let mut arena = Arena::new(arena_name);
    let mut values = RigVec::with_capacity(&mut arena, "values", 2);

    for value in 0..pushes {
        values.push(value);
    }

    arena.snapshot()
}

fn direct_entries(dir: &Path) -> Vec<PathBuf> {
    let mut entries = fs::read_dir(dir)
        .expect("read test directory")
        .map(|entry| entry.expect("directory entry").path())
        .collect::<Vec<_>>();
    entries.sort();
    entries
}

fn assert_no_hidden_entries(dir: &Path) {
    for entry in direct_entries(dir) {
        let name = entry
            .file_name()
            .and_then(|name| name.to_str())
            .expect("UTF-8 file name");
        assert!(!name.starts_with('.'), "hidden file was created: {entry:?}");
    }
}

#[test]
fn write_artifact_writes_json_file_at_requested_path() {
    let dir = temp_artifact_dir("write");
    let path = dir.join("baseline.json");
    let report = exact_report("artifact-write", 3);

    let artifact = report.write_artifact(&path).expect("write artifact");

    assert_eq!(artifact.path, path);
    assert_eq!(artifact.report, report);
    assert!(path.exists(), "artifact path should exist");
    let written = fs::read_to_string(&path).expect("read artifact JSON");
    let decoded: rig::ArenaReport = serde_json::from_str(&written).expect("ArenaReport JSON");
    assert_eq!(decoded, report);
    assert_eq!(written, report.report_json());
    assert_no_hidden_entries(&dir);

    fs::remove_dir_all(&dir).expect("clean artifact test directory");
}

#[test]
fn artifact_load_reads_same_arena_report_that_was_written() {
    let dir = temp_artifact_dir("load");
    let path = dir.join("saved.json");
    let report = exact_report("artifact-load", 4);
    report.write_artifact(&path).expect("write artifact");

    let loaded = ReportArtifact::load(&path).expect("load artifact");

    assert_eq!(loaded.path, path);
    assert_eq!(loaded.report, report);
    assert_no_hidden_entries(&dir);

    fs::remove_dir_all(&dir).expect("clean artifact test directory");
}

#[test]
fn artifact_loading_missing_artifact_returns_error() {
    let dir = temp_artifact_dir("missing");
    let path = dir.join("missing.json");

    let error = ReportArtifact::load(&path).expect_err("missing artifact should fail");

    assert!(matches!(error, rig::RigIoError::Io(_)));
    assert!(!path.exists());
    assert_no_hidden_entries(&dir);

    fs::remove_dir_all(&dir).expect("clean artifact test directory");
}

#[test]
fn artifact_loading_invalid_json_returns_error() {
    let dir = temp_artifact_dir("invalid-json");
    let path = dir.join("invalid.json");
    fs::write(&path, "not valid JSON").expect("write invalid JSON");

    let error = ReportArtifact::load(&path).expect_err("invalid JSON should fail");

    assert!(matches!(error, rig::RigIoError::Json(_)));
    let contents = fs::read_to_string(&path).expect("read invalid artifact");
    assert_eq!(contents, "not valid JSON");
    assert_no_hidden_entries(&dir);

    fs::remove_dir_all(&dir).expect("clean artifact test directory");
}

#[test]
fn artifact_compare_to_matches_arena_report_diff_and_writes_no_files() {
    let dir = temp_artifact_dir("compare-no-write");
    let baseline_path = dir.join("baseline.json");
    let current_path = dir.join("current.json");
    let baseline = exact_report("baseline", 2)
        .write_artifact(&baseline_path)
        .unwrap();
    let current = exact_report("current", 6)
        .write_artifact(&current_path)
        .unwrap();
    let before_entries = direct_entries(&dir);

    let comparison = baseline.compare_to(&current);

    assert_eq!(comparison.diff, baseline.report.diff(&current.report));
    assert_eq!(comparison.baseline, baseline.report);
    assert_eq!(comparison.current, current.report);
    assert_eq!(direct_entries(&dir), before_entries);
    assert_no_hidden_entries(&dir);

    fs::remove_dir_all(&dir).expect("clean artifact test directory");
}

#[test]
fn artifact_regression_report_uses_current_against_baseline() {
    let baseline = ReportArtifact {
        path: PathBuf::from("baseline.json"),
        report: exact_report("baseline", 2),
    };
    let current = ReportArtifact {
        path: PathBuf::from("current.json"),
        report: exact_report("current", 5),
    };
    let comparison = baseline.compare_to(&current);

    let report = comparison.regression_report(&RegressionBudget::strict());

    assert!(!report.passed);
    assert_eq!(
        report.total_capacity_delta,
        current.report.totals.total_current_capacity as isize
            - baseline.report.totals.total_current_capacity as isize
    );
    assert!(report
        .regressions
        .iter()
        .any(|regression| regression.metric == "current_capacity"));
}

#[test]
fn artifact_budget_report_checks_current_report_only() {
    let baseline = ReportArtifact {
        path: PathBuf::from("baseline.json"),
        report: exact_report("baseline", 20),
    };
    let current = ReportArtifact {
        path: PathBuf::from("current.json"),
        report: exact_report("current", 2),
    };
    let comparison = baseline.compare_to(&current);

    let report = comparison.budget_report(&MemoryBudget::max_total_capacity(
        current.report.totals.total_current_capacity,
    ));

    assert!(
        report.passed,
        "budget should check current only: {report:#?}"
    );
    assert!(report.violations.is_empty());
}

#[test]
fn artifact_report_json_decodes_to_compact_structured_evidence() {
    let baseline = ReportArtifact {
        path: PathBuf::from("/tmp/rig-baseline.json"),
        report: exact_report("baseline", 2),
    };
    let current = ReportArtifact {
        path: PathBuf::from("/tmp/rig-current.json"),
        report: exact_report("current", 4),
    };
    let comparison = baseline.compare_to(&current);

    let value: serde_json::Value =
        serde_json::from_str(&comparison.report_json()).expect("artifact comparison JSON");

    assert_eq!(value["baseline_path"], "/tmp/rig-baseline.json");
    assert_eq!(value["current_path"], "/tmp/rig-current.json");
    assert_eq!(value["baseline_arena_name"], "baseline");
    assert_eq!(value["current_arena_name"], "current");
    assert_eq!(value["diff"]["before_arena_name"], "baseline");
    assert_eq!(
        value["diff"],
        serde_json::to_value(&comparison.diff).unwrap()
    );
    assert!(value.get("baseline").is_none());
    assert!(value.get("current").is_none());
}

#[test]
fn artifact_human_report_includes_paths_and_allocation_diff() {
    let baseline = ReportArtifact {
        path: PathBuf::from("/tmp/rig-human-baseline.json"),
        report: exact_report("baseline", 2),
    };
    let current = ReportArtifact {
        path: PathBuf::from("/tmp/rig-human-current.json"),
        report: exact_report("current", 4),
    };
    let comparison = baseline.compare_to(&current);

    let report = comparison.report();

    assert!(report.contains("RIG report artifact comparison"));
    assert!(report.contains("Baseline: /tmp/rig-human-baseline.json"));
    assert!(report.contains("Current: /tmp/rig-human-current.json"));
    assert!(report.contains("RIG allocation diff"));
    assert!(report.contains("Regression gate:\nNot evaluated by artifact comparison report."));
    assert!(report.contains("Budget gate:\nNot evaluated by artifact comparison report."));
}

#[test]
fn artifact_compare_example_runs_and_outputs_saved_evidence() {
    let output = Command::new(env!("CARGO"))
        .args(["run", "--quiet", "--example", "artifact_compare"])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("run artifact_compare example");

    assert!(
        output.status.success(),
        "artifact_compare failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("UTF-8 stdout");

    assert!(stdout.contains("RIG report artifact comparison"));
    assert!(stdout.contains("Baseline:"));
    assert!(stdout.contains("Current:"));
    assert!(stdout.contains("RIG allocation diff"));
    assert!(stdout.contains("RIG memory regression report"));
    assert!(stdout.contains("RIG memory budget report"));
    assert!(stdout.contains("Status: FAILED") || stdout.contains("Status: PASSED"));
    assert!(stdout.contains("\"baseline_path\""));
    assert!(stdout.contains("\"current_path\""));
    assert!(stdout.contains("\"diff\""));
}

#[test]
fn artifact_operations_create_no_hidden_files() {
    let dir = temp_artifact_dir("no-hidden");
    let baseline_path = dir.join("baseline.json");
    let current_path = dir.join("current.json");
    let baseline = exact_report("baseline", 2)
        .write_artifact(&baseline_path)
        .unwrap();
    let current = exact_report("current", 3)
        .write_artifact(&current_path)
        .unwrap();

    let loaded_baseline = ReportArtifact::load(&baseline.path).unwrap();
    let loaded_current = ReportArtifact::load(&current.path).unwrap();
    let comparison = loaded_baseline.compare_to(&loaded_current);
    let _human = comparison.report();
    let json = comparison.report_json();
    let decoded: serde_json::Value = serde_json::from_str(&json).unwrap();

    assert_eq!(decoded["baseline_arena_name"], "baseline");
    assert_no_hidden_entries(&dir);
    assert_eq!(direct_entries(&dir), vec![baseline_path, current_path]);

    fs::remove_dir_all(&dir).expect("clean artifact test directory");
}
