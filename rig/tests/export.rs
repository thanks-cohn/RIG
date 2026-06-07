use rig::{
    Arena, ExportFormat, GrowthPolicy, MemoryBudget, RegressionBudget, ReportArtifact, RigString,
    RigVec,
};
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT_ID: AtomicUsize = AtomicUsize::new(0);

fn temp_export_dir(name: &str) -> PathBuf {
    let id = NEXT_ID.fetch_add(1, Ordering::SeqCst);
    let dir = std::env::temp_dir().join(format!(
        "rig-export-test-{name}-{}-{id}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir(&dir).expect("create export test directory");
    dir
}

fn direct_entries(dir: &Path) -> Vec<PathBuf> {
    let mut entries = fs::read_dir(dir)
        .expect("read export test directory")
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

fn report_with_growth(name: &str) -> rig::ArenaReport {
    let mut arena = Arena::new(name);
    let mut values = RigVec::with_policy(&mut arena, "values", GrowthPolicy::Exact);
    values.push(1);
    values.push(2);
    values.push(3);

    let mut log = RigString::with_capacity_and_policy(&mut arena, "log", 2, GrowthPolicy::Exact);
    log.push_str("abc");

    arena.snapshot()
}

fn jsonl_values(jsonl: &str) -> Vec<Value> {
    jsonl
        .lines()
        .map(|line| serde_json::from_str(line).expect("valid JSONL object"))
        .collect()
}

#[test]
fn export_containers_csv_includes_header_and_real_container_rows() {
    let report = report_with_growth("containers-csv");

    let csv = report.containers_csv();

    assert!(csv.starts_with("name,kind,len,initial_capacity,growth_policy,current_capacity"));
    assert!(csv.contains("values,RigVec,3,0,Exact,3,3,3,1,1,total pushed items,3,,"));
    assert!(csv.contains(
        "log,RigString,3,2,Exact,3,1,1,1,1,total append operations,1,total appended bytes,3"
    ));
}

#[test]
fn export_growth_history_csv_includes_observed_capacity_and_operation_index() {
    let report = report_with_growth("growth-history-csv");

    let csv = report.growth_history_csv();

    assert!(csv.starts_with("container_name,container_kind,old_capacity,new_capacity,operation_index,capacity_added,growth_policy\n"));
    assert!(csv.contains("values,RigVec,0,1,1,1,Exact"));
    assert!(csv.contains("values,RigVec,1,2,2,1,Exact"));
}

#[test]
fn export_growth_attributions_csv_includes_capacity_added_and_growth_policy() {
    let report = report_with_growth("growth-attribution-csv");

    let csv = report.growth_attributions_csv();

    assert!(csv.starts_with(
        "container_name,operation_index,old_capacity,new_capacity,capacity_added,growth_policy\n"
    ));
    assert!(csv.contains("values,1,0,1,1,Exact"));
    assert!(csv.contains(",1,Exact"));
}

#[test]
fn export_csv_escaping_handles_commas_quotes_and_newlines_in_container_names() {
    let mut arena = Arena::new("escaping");
    let mut tricky = RigVec::with_policy(&mut arena, "a,b \"quoted\"\nname", GrowthPolicy::Exact);
    tricky.push(1);
    let report = arena.snapshot();

    let csv = report.containers_csv();

    assert!(
        csv.contains("\"a,b \"\"quoted\"\"\nname\",RigVec"),
        "{csv:?}"
    );
}

#[test]
fn export_jsonl_deserializes_line_by_line() {
    let report = report_with_growth("jsonl");

    let containers = jsonl_values(&report.containers_jsonl());
    let history = jsonl_values(&report.growth_history_jsonl());
    let attributions = jsonl_values(&report.growth_attributions_jsonl());

    assert_eq!(containers[0]["name"], "values");
    assert_eq!(history[0]["old_capacity"], 0);
    assert_eq!(attributions[0]["capacity_added"], 1);
}

#[test]
fn export_empty_growth_history_jsonl_is_empty_string() {
    let report = Arena::new("empty-jsonl").snapshot();

    assert_eq!(report.growth_history_jsonl(), "");
}

#[test]
fn export_empty_growth_history_csv_still_has_header() {
    let report = Arena::new("empty-csv").snapshot();

    assert_eq!(
        report.growth_history_csv(),
        "container_name,container_kind,old_capacity,new_capacity,operation_index,capacity_added,growth_policy\n"
    );
}

#[test]
fn export_budget_violations_csv_and_jsonl_contain_typed_fields() {
    let report = report_with_growth("budget-export");
    let budget = MemoryBudget::unlimited()
        .with_max_total_capacity(1)
        .with_max_container_growth_events(0);
    let budget_report = report.check_budget(&budget);

    let csv = budget_report.violations_csv();
    let jsonl = budget_report.violations_jsonl();
    let values = jsonl_values(&jsonl);

    assert!(csv.starts_with("scope,container_name,metric,observed,limit,exceeded_by\n"));
    assert!(csv.contains("arena,,total_current_capacity,"));
    assert!(csv.contains("container,values,growth_events,3,0,3"));
    assert_eq!(values[0]["scope"], "arena");
    assert!(values
        .iter()
        .any(|value| value["metric"] == "growth_events"));
}

#[test]
fn export_regression_csv_and_jsonl_contain_typed_fields() {
    let baseline = Arena::new("baseline").snapshot();
    let current = report_with_growth("current");
    let regression_report =
        current.check_regressions_against(&baseline, &RegressionBudget::strict());

    let csv = regression_report.regressions_csv();
    let values = jsonl_values(&regression_report.regressions_jsonl());

    assert!(csv.starts_with("container_name,metric,baseline,current,delta,allowed_delta\n"));
    assert!(csv.contains("values,current_capacity,0,3,3,0"));
    assert!(csv.contains("total,total_current_capacity,0,"));
    assert_eq!(values[0]["metric"], "total_current_capacity");
    assert!(values
        .iter()
        .any(|value| value["container_name"] == "values"));
}

#[test]
fn export_artifact_comparison_summary_csv_and_jsonl_contain_paths_and_diff_totals() {
    let dir = temp_export_dir("artifact-summary");
    let baseline_path = dir.join("baseline.json");
    let current_path = dir.join("current.json");
    let baseline = Arena::new("baseline").snapshot();
    let current = report_with_growth("current");
    baseline
        .write_artifact(&baseline_path)
        .expect("write baseline");
    current
        .write_artifact(&current_path)
        .expect("write current");
    let comparison = ReportArtifact::load(&baseline_path)
        .expect("load baseline")
        .compare_to(&ReportArtifact::load(&current_path).expect("load current"));

    let csv = comparison.summary_csv();
    let values = jsonl_values(&comparison.summary_jsonl());

    assert!(csv.starts_with("baseline_path,current_path,baseline_arena_name,current_arena_name,total_len_delta,total_capacity_delta,total_growth_event_delta,total_operation_delta"));
    assert!(csv.contains(&baseline_path.display().to_string()));
    assert!(csv.contains(&current_path.display().to_string()));
    assert!(csv.contains(",baseline,current,6,"));
    assert_eq!(
        values[0]["baseline_path"],
        baseline_path.display().to_string()
    );
    assert_eq!(
        values[0]["current_path"],
        current_path.display().to_string()
    );
    assert_eq!(values[0]["total_len_delta"], 6);
    assert_no_hidden_entries(&dir);

    fs::remove_dir_all(&dir).expect("clean artifact summary dir");
}

#[test]
fn export_write_to_writes_exact_contents_and_no_hidden_files() {
    let dir = temp_export_dir("write-exact");
    let path = dir.join("containers.csv");
    let report = report_with_growth("write-exact");
    let export = report.export_containers(ExportFormat::Csv);

    export.write_to(&path).expect("write export");

    assert_eq!(
        fs::read_to_string(&path).expect("read export"),
        export.contents
    );
    assert_no_hidden_entries(&dir);

    fs::remove_dir_all(&dir).expect("clean write exact dir");
}

#[test]
fn export_write_to_fails_when_parent_directory_is_missing() {
    let dir = temp_export_dir("missing-parent");
    let missing_path = dir.join("missing").join("containers.csv");
    let export = report_with_growth("missing-parent").export_containers(ExportFormat::Csv);

    let error = export
        .write_to(&missing_path)
        .expect_err("missing parent should fail");

    assert!(matches!(error, rig::RigIoError::Io(_)));
    assert!(!missing_path.exists());
    assert_no_hidden_entries(&dir);

    fs::remove_dir_all(&dir).expect("clean missing parent dir");
}

#[test]
fn export_write_to_creates_hidden_file_only_when_path_explicitly_requests_it() {
    let dir = temp_export_dir("explicit-hidden");
    let hidden_path = dir.join(".explicit-export.csv");
    let export = report_with_growth("explicit-hidden").export_growth_history(ExportFormat::Csv);

    export
        .write_to(&hidden_path)
        .expect("write explicit hidden export");

    assert!(hidden_path.exists());
    assert_eq!(
        fs::read_to_string(&hidden_path).expect("read hidden export"),
        export.contents
    );
    let hidden_entries = direct_entries(&dir)
        .into_iter()
        .filter(|entry| {
            entry
                .file_name()
                .and_then(|name| name.to_str())
                .expect("UTF-8 file name")
                .starts_with('.')
        })
        .collect::<Vec<_>>();
    assert_eq!(hidden_entries, vec![hidden_path]);

    fs::remove_dir_all(&dir).expect("clean explicit hidden dir");
}

#[test]
fn export_evidence_exports_example_runs_and_prints_real_exports() {
    let output = Command::new(env!("CARGO"))
        .args(["run", "--example", "evidence_exports"])
        .output()
        .expect("run evidence_exports example");

    assert!(
        output.status.success(),
        "status: {:?}\nstderr: {}",
        output.status,
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("UTF-8 stdout");

    assert!(stdout.contains("containers CSV"));
    assert!(stdout.contains("name,kind,len,initial_capacity,growth_policy,current_capacity"));
    assert!(stdout.contains("growth history JSONL"));
    assert!(stdout.contains("\"old_capacity\""));
    assert!(stdout.contains("budget violations CSV"));
    assert!(stdout.contains("scope,container_name,metric,observed,limit,exceeded_by"));
    assert!(stdout.contains("regression JSONL"));
    assert!(stdout.contains("\"allowed_delta\""));
    assert!(stdout.contains("artifact summary CSV"));
    assert!(stdout.contains("baseline_path,current_path,baseline_arena_name,current_arena_name"));
    assert!(stdout.contains("export file round-trip: true"));
}
