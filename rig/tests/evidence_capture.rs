use rig::{Arena, EvidenceArtifact, EvidenceCapture, RigString, RigVec};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT_ID: AtomicUsize = AtomicUsize::new(0);

fn temp_dir(name: &str) -> PathBuf {
    let id = NEXT_ID.fetch_add(1, Ordering::SeqCst);
    let dir = std::env::temp_dir().join(format!(
        "rig-evidence-capture-{name}-{}-{id}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir(&dir).expect("create evidence capture test directory");
    dir
}

fn direct_entries(dir: &Path) -> Vec<PathBuf> {
    let mut entries = fs::read_dir(dir)
        .expect("read evidence test directory")
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
fn artifact_round_trip_preserves_runtime_checkpoints() {
    let dir = temp_dir("round-trip");
    let path = dir.join("evidence.json");
    let mut arena = Arena::new("round-trip-arena");
    let mut values = RigVec::with_capacity(&mut arena, "values", 1);
    values.push(1);

    let mut capture = EvidenceCapture::new("round-trip-workload");
    capture.capture_checkpoint("after-one-push", &arena);
    values.push(2);
    capture.capture_checkpoint("after-two-pushes", &arena);
    let artifact = capture.artifact();

    artifact.save_json(&path).expect("save evidence artifact");
    let loaded = EvidenceArtifact::load_json(&path).expect("load evidence artifact");

    assert_eq!(loaded, artifact);
    assert_eq!(loaded.checkpoints.len(), 2);
    assert_eq!(loaded.checkpoints[0].arena.totals.total_len, 1);
    assert_eq!(loaded.checkpoints[1].arena.totals.total_len, 2);
    assert_no_hidden_entries(&dir);

    fs::remove_dir_all(&dir).expect("clean evidence capture test directory");
}

#[test]
fn evidence_integrity_uses_observed_arena_values() {
    let mut arena = Arena::new("integrity-arena");
    let mut values = RigVec::with_capacity(&mut arena, "values", 2);
    let mut log = RigString::with_capacity(&mut arena, "log", 4);

    values.push(10);
    values.push(20);
    values.push(30);
    log.push_str("ok");
    log.push_str(";done");

    let mut capture = EvidenceCapture::new("integrity-workload");
    let checkpoint = capture.capture_checkpoint("observed", &arena);
    let direct_snapshot = arena.snapshot();

    assert_eq!(checkpoint.arena, direct_snapshot);
    assert_eq!(checkpoint.arena.tracked_container_count, 2);
    assert_eq!(checkpoint.arena.totals.total_len, values.len() + log.len());
    assert_eq!(
        checkpoint.arena.totals.total_growth_events,
        direct_snapshot.growth_history.len()
    );
    assert!(
        checkpoint.captured_unix_epoch_seconds
            >= capture.session().metadata.started_unix_epoch_seconds
    );
}

#[test]
fn before_after_workload_comparison_reports_real_deltas() {
    let mut arena = Arena::new("before-after-arena");
    let mut values = RigVec::with_capacity(&mut arena, "values", 1);
    values.push(1);

    let mut capture = EvidenceCapture::new("before-after-workload");
    let comparison = capture.capture_workload(&arena, "before", "after", || {
        values.push(2);
        values.push(3);
    });

    assert_eq!(capture.session().checkpoints.len(), 2);
    assert_eq!(comparison.baseline_checkpoint, "before");
    assert_eq!(comparison.current_checkpoint, "after");
    assert_eq!(comparison.diff.total_len_delta, 2);
    assert_eq!(comparison.diff.containers_changed[0].operation_delta, 2);
    assert!(comparison.report().contains("RIG evidence comparison"));
    assert!(comparison.report_json().contains("total_len_delta"));
}

#[test]
fn artifact_comparison_uses_latest_saved_runtime_artifacts() {
    let mut baseline_arena = Arena::new("baseline-arena");
    let mut baseline_values = RigVec::with_capacity(&mut baseline_arena, "values", 2);
    baseline_values.push(1);

    let mut current_arena = Arena::new("current-arena");
    let mut current_values = RigVec::with_capacity(&mut current_arena, "values", 2);
    current_values.push(1);
    current_values.push(2);
    current_values.push(3);

    let mut baseline_capture = EvidenceCapture::new("baseline-workload");
    baseline_capture.capture_checkpoint("latest", &baseline_arena);
    let mut current_capture = EvidenceCapture::new("current-workload");
    current_capture.capture_checkpoint("latest", &current_arena);

    let comparison = baseline_capture
        .artifact()
        .compare_latest(&current_capture.artifact())
        .expect("latest checkpoint comparison");

    assert_eq!(comparison.diff.total_len_delta, 2);
    assert_eq!(comparison.diff.containers_changed[0].capacity_delta, 2);
}

#[test]
fn save_load_operations_create_no_hidden_files() {
    let dir = temp_dir("no-hidden");
    let path = dir.join("artifact.json");
    let mut arena = Arena::new("no-hidden-arena");
    let mut values = RigVec::new(&mut arena, "values");
    values.push(1);
    let mut capture = EvidenceCapture::new("no-hidden-workload");
    capture.capture_checkpoint("observed", &arena);

    capture.artifact().save_json(&path).expect("save artifact");
    let loaded = EvidenceArtifact::load_json(&path).expect("load artifact");
    let _human = loaded.report();
    let _machine = loaded.report_json();

    assert_no_hidden_entries(&dir);
    assert_eq!(direct_entries(&dir), vec![path]);

    fs::remove_dir_all(&dir).expect("clean evidence capture test directory");
}

#[test]
fn load_missing_and_invalid_artifacts_return_real_errors() {
    let dir = temp_dir("load-save-errors");
    let missing = dir.join("missing.json");
    assert!(matches!(
        EvidenceArtifact::load_json(&missing).expect_err("missing file should fail"),
        rig::RigIoError::Io(_)
    ));

    let invalid = dir.join("invalid.json");
    fs::write(&invalid, "not evidence json").expect("write invalid artifact contents");
    assert!(matches!(
        EvidenceArtifact::load_json(&invalid).expect_err("invalid JSON should fail"),
        rig::RigIoError::Json(_)
    ));
    assert_no_hidden_entries(&dir);

    fs::remove_dir_all(&dir).expect("clean evidence capture test directory");
}
