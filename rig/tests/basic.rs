use rig::{Arena, LoadReportError, RigString, RigVec};
use std::fs;
use std::path::PathBuf;

fn temp_report_path(test_name: &str) -> PathBuf {
    let mut path = std::env::temp_dir();
    path.push(format!(
        "rig-{test_name}-{}-{}.json",
        std::process::id(),
        std::thread::current().name().unwrap_or("unnamed")
    ));
    path
}

fn temp_observation_dir(test_name: &str) -> PathBuf {
    let mut path = std::env::temp_dir();
    path.push(format!(
        "rig-{test_name}-{}-{}",
        std::process::id(),
        std::thread::current().name().unwrap_or("unnamed")
    ));
    let _ = fs::remove_dir_all(&path);
    fs::create_dir(&path).expect("create observation directory");
    path
}

fn assert_dir_is_empty(path: &std::path::Path) {
    let entries = fs::read_dir(path)
        .expect("read observation directory")
        .collect::<Result<Vec<_>, _>>()
        .expect("collect observation directory entries");
    assert!(
        entries.is_empty(),
        "expected no filesystem entries in {}, found {:?}",
        path.display(),
        entries.iter().map(|entry| entry.path()).collect::<Vec<_>>()
    );
}

fn sample_arena(name: &str) -> Arena {
    let mut arena = Arena::new(name);
    let mut users = RigVec::with_capacity(&mut arena, "users", 2);
    let mut audit = RigString::with_capacity(&mut arena, "audit_events", 8);

    users.push(1);
    users.push(2);
    users.push(3);
    audit.push_str("login");
    audit.push_str(";ok");

    arena
}

#[test]
fn arena_creation_tracks_name() {
    let arena = Arena::new("main");

    assert_eq!(arena.name(), "main");
}

#[test]
fn rigvec_creation_starts_empty_and_is_reported() {
    let mut arena = Arena::new("main");
    let users: RigVec<i32> = RigVec::new(&mut arena, "users");

    assert!(users.is_empty());
    assert_eq!(users.len(), 0);
    assert_eq!(users.capacity(), 0);

    let report = arena.report();
    assert!(report.contains("Arena: main"));
    assert!(report.contains("Container: users"));
    assert!(report.contains("kind: RigVec"));
    assert!(report.contains("len: 0"));
    assert!(report.contains("current capacity: 0"));
    assert!(report.contains("growth events: 0"));
    assert!(report.contains("total pushed items: 0"));
}

#[test]
fn push_changes_length_and_total_pushed_count() {
    let mut arena = Arena::new("main");
    let mut users = RigVec::new(&mut arena, "users");

    users.push("Ada");
    users.push("Grace");
    users.push("Katherine");

    assert_eq!(users.len(), 3);
    assert_eq!(users.total_pushed(), 3);

    let report = arena.report();
    assert!(report.contains("len: 3"));
    assert!(report.contains("total pushed items: 3"));
}

#[test]
fn growth_events_are_recorded_when_capacity_increases() {
    let mut arena = Arena::new("main");
    let mut numbers = RigVec::new(&mut arena, "numbers");

    let starting_capacity = numbers.capacity();
    numbers.push(1);

    assert!(numbers.capacity() > starting_capacity);
    assert_eq!(numbers.growth_events(), 1);

    let report = arena.report();
    assert!(report.contains("growth events: 1"));
}

#[test]
fn multiple_rigvecs_can_be_tracked_in_one_arena() {
    let mut arena = Arena::new("request");
    let mut users = RigVec::new(&mut arena, "users");
    let mut events = RigVec::new(&mut arena, "events");

    users.push(1);
    users.push(2);
    events.push("created");

    let report = arena.report();
    assert!(report.contains("Tracked containers: 2"));
    assert!(report.contains("Container: users"));
    assert!(report.contains("Container: events"));
    assert!(report.contains("total pushed items: 2"));
    assert!(report.contains("total pushed items: 1"));
}

#[test]
fn empty_vector_report_is_valid() {
    let mut arena = Arena::new("empty-check");
    let _empty: RigVec<u8> = RigVec::new(&mut arena, "empty-buffer");

    let report = arena.report();
    assert!(report.contains("Arena: empty-check"));
    assert!(report.contains("Container: empty-buffer"));
    assert!(report.contains("kind: RigVec"));
    assert!(report.contains("len: 0"));
    assert!(report.contains("current capacity: 0"));
    assert!(report.contains("growth events: 0"));
    assert!(report.contains("total pushed items: 0"));
}

#[test]
fn report_contains_required_tracking_fields_after_growth() {
    let mut arena = Arena::new("batch");
    let mut jobs = RigVec::new(&mut arena, "jobs");

    for job_id in 0..32 {
        jobs.push(job_id);
    }

    let report = arena.report();
    assert!(report.contains("Arena: batch"));
    assert!(report.contains("Container: jobs"));
    assert!(report.contains("kind: RigVec"));
    assert!(report.contains("len: 32"));
    assert!(report.contains("current capacity:"));
    assert!(report.contains("growth events:"));
    assert!(report.contains("total pushed items: 32"));
    assert!(jobs.growth_events() > 0);
}

#[test]
fn with_capacity_records_capacity_and_reports_each_growth_phase() {
    let mut arena = Arena::new("capacity-check");
    let mut items = RigVec::with_capacity(&mut arena, "items", 2);

    assert_eq!(items.len(), 0);
    assert_eq!(items.capacity(), 2);
    assert_eq!(items.growth_events(), 0);

    let initial_report = arena.report();
    assert!(initial_report.contains("Arena: capacity-check"));
    assert!(initial_report.contains("Container: items"));
    assert!(initial_report.contains("  kind: RigVec"));
    assert!(initial_report.contains("    len: 0"));
    assert!(initial_report.contains("    initial capacity: 2"));
    assert!(initial_report.contains("    current capacity: 2"));
    assert!(initial_report.contains("    growth events: 0"));
    assert!(initial_report.contains("    total pushed items: 0"));

    items.push("first");
    items.push("second");

    assert_eq!(items.len(), 2);
    assert_eq!(items.capacity(), 2);
    assert_eq!(items.growth_events(), 0);

    let within_capacity_report = arena.report();
    assert!(within_capacity_report.contains("    len: 2"));
    assert!(within_capacity_report.contains("    initial capacity: 2"));
    assert!(within_capacity_report.contains("    current capacity: 2"));
    assert!(within_capacity_report.contains("    growth events: 0"));
    assert!(within_capacity_report.contains("    total pushed items: 2"));

    items.push("third");

    assert_eq!(items.len(), 3);
    assert!(items.capacity() > 2);
    assert_eq!(items.growth_events(), 1);

    let exceeded_capacity_report = arena.report();
    assert!(exceeded_capacity_report.contains("    len: 3"));
    assert!(exceeded_capacity_report.contains("    initial capacity: 2"));
    assert!(
        exceeded_capacity_report.contains(&format!("    current capacity: {}", items.capacity()))
    );
    assert!(exceeded_capacity_report.contains("    growth events: 1"));
    assert!(exceeded_capacity_report.contains("    total pushed items: 3"));
}

#[test]
fn rigstring_starts_empty_and_is_reported() {
    let mut arena = Arena::new("strings");
    let audit = RigString::new(&mut arena, "audit_events");

    assert!(audit.is_empty());
    assert_eq!(audit.len(), 0);
    assert_eq!(audit.capacity(), 0);
    assert_eq!(audit.growth_events(), 0);
    assert_eq!(audit.append_operations(), 0);
    assert_eq!(audit.total_appended_bytes(), 0);

    let report = arena.report();
    assert!(report.contains("Container: audit_events"));
    assert!(report.contains("kind: RigString"));
    assert!(report.contains("total append operations: 0"));
    assert!(report.contains("total appended bytes: 0"));
}

#[test]
fn rigstring_push_str_changes_len_and_appended_bytes() {
    let mut arena = Arena::new("strings");
    let mut audit = RigString::new(&mut arena, "audit_events");

    audit.push_str("log");
    audit.push_str("in");

    assert_eq!(audit.len(), 5);
    assert_eq!(audit.append_operations(), 2);
    assert_eq!(audit.total_appended_bytes(), 5);

    let report = arena.report();
    assert!(report.contains("    len: 5"));
    assert!(report.contains("    total append operations: 2"));
    assert!(report.contains("    total appended bytes: 5"));
}

#[test]
fn rigstring_growth_events_occur_when_capacity_grows() {
    let mut arena = Arena::new("strings");
    let mut audit = RigString::new(&mut arena, "audit_events");

    let starting_capacity = audit.capacity();
    audit.push_str("first event");

    assert!(audit.capacity() > starting_capacity);
    assert_eq!(audit.growth_events(), 1);

    let report = arena.report();
    assert!(report.contains("kind: RigString"));
    assert!(report.contains("    growth events: 1"));
}

#[test]
fn rigstring_with_capacity_records_initial_capacity() {
    let mut arena = Arena::new("strings");
    let audit = RigString::with_capacity(&mut arena, "audit_events", 16);

    assert_eq!(audit.len(), 0);
    assert_eq!(audit.capacity(), 16);
    assert_eq!(audit.growth_events(), 0);

    let report = arena.report();
    assert!(report.contains("    initial capacity: 16"));
    assert!(report.contains("    current capacity: 16"));
}

#[test]
fn pushing_and_appending_within_capacity_do_not_count_as_growth() {
    let mut arena = Arena::new("capacity-check");
    let mut items = RigVec::with_capacity(&mut arena, "items", 2);
    let mut audit = RigString::with_capacity(&mut arena, "audit_events", 8);

    items.push("first");
    items.push("second");
    audit.push_str("1234");
    audit.push_str("5678");

    assert_eq!(items.growth_events(), 0);
    assert_eq!(audit.growth_events(), 0);

    let report = arena.report();
    assert!(report.contains("  total growth events: 0"));
}

#[test]
fn exceeding_capacity_counts_as_growth_for_vec_and_string() {
    let mut arena = Arena::new("capacity-check");
    let mut items = RigVec::with_capacity(&mut arena, "items", 1);
    let mut audit = RigString::with_capacity(&mut arena, "audit_events", 4);

    items.push("first");
    audit.push_str("1234");

    assert_eq!(items.growth_events(), 0);
    assert_eq!(audit.growth_events(), 0);

    items.push("second");
    audit.push_str("5");

    assert_eq!(items.growth_events(), 1);
    assert_eq!(audit.growth_events(), 1);

    let report = arena.report();
    assert!(report.contains("  total growth events: 2"));
}

#[test]
fn arena_totals_update_across_multiple_containers() {
    let mut arena = Arena::new("request");
    let mut users = RigVec::with_capacity(&mut arena, "users", 4);
    let mut audit = RigString::with_capacity(&mut arena, "audit_events", 16);

    users.push(1);
    users.push(2);
    audit.push_str("login");
    audit.push_str(" ok");

    let report = arena.report();
    assert!(report.contains("Tracked containers: 2"));
    assert!(report.contains("Totals:\n  total len: 10"));
    assert!(report.contains("  total current capacity: 20"));
    assert!(report.contains("  total growth events: 0"));
    assert!(report.contains("  total pushed/appended operations: 4"));
}

#[test]
fn report_formats_nested_fields_with_clean_indentation() {
    let mut arena = Arena::new("format-check");
    let _items: RigVec<i32> = RigVec::with_capacity(&mut arena, "items", 4);
    let _audit = RigString::with_capacity(&mut arena, "audit_events", 8);

    let report = arena.report();
    let expected = "RIG allocation report\nArena: format-check\nTracked containers: 2\nTotals:\n  total len: 0\n  total current capacity: 12\n  total growth events: 0\n  total pushed/appended operations: 0\nContainers:\n  Container: items\n  kind: RigVec\n  fields:\n    len: 0\n    initial capacity: 4\n    current capacity: 4\n    growth events: 0\n    total pushed items: 0\n  Container: audit_events\n  kind: RigString\n  fields:\n    len: 0\n    initial capacity: 8\n    current capacity: 8\n    growth events: 0\n    total append operations: 0\n    total appended bytes: 0\nGrowth history:\n  (none)";

    assert_eq!(report, expected);
}

#[test]
fn snapshot_contains_arena_name_and_tracked_container_count() {
    let mut arena = Arena::new("machine-readable");
    let mut users = RigVec::with_capacity(&mut arena, "users", 2);
    let mut audit = RigString::with_capacity(&mut arena, "audit_events", 8);

    users.push(1);
    audit.push_str("login");

    let snapshot = arena.snapshot();

    assert_eq!(snapshot.arena_name, "machine-readable");
    assert_eq!(snapshot.tracked_container_count, 2);
    assert_eq!(snapshot.containers.len(), 2);
}

#[test]
fn snapshot_totals_match_human_report_totals() {
    let mut arena = Arena::new("totals-proof");
    let mut users = RigVec::with_capacity(&mut arena, "users", 2);
    let mut audit = RigString::with_capacity(&mut arena, "audit_events", 8);

    users.push(7);
    users.push(8);
    audit.push_str("abc");
    audit.push_str("def");

    let snapshot = arena.snapshot();
    let report = arena.report();

    assert_eq!(snapshot.totals.total_len, 8);
    assert!(report.contains(&format!("  total len: {}", snapshot.totals.total_len)));
    assert!(report.contains(&format!(
        "  total current capacity: {}",
        snapshot.totals.total_current_capacity
    )));
    assert!(report.contains(&format!(
        "  total growth events: {}",
        snapshot.totals.total_growth_events
    )));
    assert!(report.contains(&format!(
        "  total pushed/appended operations: {}",
        snapshot.totals.total_pushed_appended_operations
    )));
}

#[test]
fn snapshot_includes_vec_and_string_container_kinds() {
    let mut arena = Arena::new("kind-proof");
    let _users: RigVec<i32> = RigVec::new(&mut arena, "users");
    let _audit = RigString::new(&mut arena, "audit_events");

    let snapshot = arena.snapshot();
    let kinds: Vec<&str> = snapshot
        .containers
        .iter()
        .map(|container| container.kind.as_str())
        .collect();

    assert!(kinds.contains(&"RigVec"));
    assert!(kinds.contains(&"RigString"));
}

#[test]
fn snapshot_includes_capacity_and_growth_evidence() {
    let mut arena = Arena::new("growth-proof");
    let mut users = RigVec::with_capacity(&mut arena, "users", 1);
    let mut audit = RigString::with_capacity(&mut arena, "audit_events", 1);

    users.push(1);
    users.push(2);
    audit.push_str("a");
    audit.push_str("bc");

    let snapshot = arena.snapshot();
    let users_report = snapshot
        .containers
        .iter()
        .find(|container| container.name == "users")
        .expect("users report exists");
    let audit_report = snapshot
        .containers
        .iter()
        .find(|container| container.name == "audit_events")
        .expect("audit_events report exists");

    assert_eq!(users_report.initial_capacity, 1);
    assert_eq!(users_report.current_capacity, users.capacity());
    assert_eq!(users_report.growth_events, users.growth_events());
    assert!(users_report.growth_events >= 1);
    assert_eq!(audit_report.initial_capacity, 1);
    assert_eq!(audit_report.current_capacity, audit.capacity());
    assert_eq!(audit_report.growth_events, audit.growth_events());
    assert!(audit_report.growth_events >= 1);
}

#[test]
fn report_json_is_valid_json_and_contains_arena_name() {
    let mut arena = Arena::new("json-proof");
    let mut users = RigVec::new(&mut arena, "users");

    users.push(42);

    let json = arena.report_json();
    let parsed: serde_json::Value = serde_json::from_str(&json).expect("valid JSON report");

    assert_eq!(parsed["arena_name"], "json-proof");
    assert_eq!(parsed["tracked_container_count"], 1);
    assert_eq!(parsed["containers"][0]["kind"], "RigVec");
}

#[test]
fn report_json_round_trips_back_into_arena_report() {
    let mut arena = Arena::new("round-trip-proof");
    let mut audit = RigString::with_capacity(&mut arena, "audit_events", 4);

    audit.push_str("ok");

    let snapshot = arena.snapshot();
    let json = arena.report_json();
    let decoded: rig::ArenaReport = serde_json::from_str(&json).expect("ArenaReport JSON");

    assert_eq!(decoded, snapshot);
}

#[test]
fn repository_does_not_contain_fake_vendor_directory() {
    let crate_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let repo_root = crate_dir
        .parent()
        .expect("rig crate lives under the repository root");
    let vendor = repo_root.join("vendor");

    assert!(
        !vendor.exists(),
        "repo root must not contain {}",
        vendor.display()
    );
}

#[test]
fn repository_does_not_contain_placeholder_directories() {
    let crate_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let repo_root = crate_dir
        .parent()
        .expect("rig crate lives under the repository root");

    for directory_name in ["future", "todo", "placeholder", "stubs"] {
        let path = repo_root.join(directory_name);
        assert!(
            !path.exists(),
            "repo root must not contain placeholder directory {}",
            path.display()
        );
    }
}

#[test]
fn write_json_writes_a_file_with_valid_arena_report_json() {
    let path = temp_report_path("write-json-valid-report");
    let _ = fs::remove_file(&path);
    let arena = sample_arena("persist-proof");

    arena.write_json(&path).expect("write report JSON");

    assert!(path.exists(), "write_json should create the requested file");
    let written = fs::read_to_string(&path).expect("read written report");
    let parsed_value: serde_json::Value = serde_json::from_str(&written).expect("valid JSON");
    let decoded: rig::ArenaReport = serde_json::from_str(&written).expect("ArenaReport JSON");

    assert_eq!(parsed_value["arena_name"], "persist-proof");
    assert_eq!(decoded, arena.snapshot());

    let _ = fs::remove_file(&path);
}

#[test]
fn arena_report_write_json_writes_valid_report_json() {
    let path = temp_report_path("arena-report-write-json");
    let _ = fs::remove_file(&path);
    let report = sample_arena("report-write-proof").snapshot();

    report.write_json(&path).expect("write ArenaReport JSON");

    let written = fs::read_to_string(&path).expect("read ArenaReport JSON");
    let decoded: rig::ArenaReport = serde_json::from_str(&written).expect("valid ArenaReport JSON");

    assert_eq!(decoded, report);
    assert_eq!(written, report.report_json());

    let _ = fs::remove_file(&path);
}

#[test]
fn load_report_returns_the_same_snapshot_that_was_written() {
    let path = temp_report_path("load-report-round-trip");
    let _ = fs::remove_file(&path);
    let arena = sample_arena("load-proof");
    let snapshot = arena.snapshot();

    arena.write_json(&path).expect("write report JSON");
    let loaded = Arena::load_report(&path).expect("load report JSON");

    assert_eq!(loaded, snapshot);

    let _ = fs::remove_file(&path);
}

#[test]
fn write_json_overwrites_an_existing_file() {
    let path = temp_report_path("write-json-overwrites");
    let _ = fs::remove_file(&path);
    fs::write(&path, "not the final report").expect("seed existing file");
    let arena = sample_arena("overwrite-proof");

    arena.write_json(&path).expect("overwrite report JSON");

    let written = fs::read_to_string(&path).expect("read overwritten file");
    let decoded: rig::ArenaReport = serde_json::from_str(&written).expect("ArenaReport JSON");

    assert_ne!(written, "not the final report");
    assert_eq!(decoded, arena.snapshot());

    let _ = fs::remove_file(&path);
}

#[test]
fn write_json_returns_io_error_when_parent_directory_is_missing() {
    let mut missing_parent = std::env::temp_dir();
    missing_parent.push(format!(
        "rig-missing-parent-{}-{}",
        std::process::id(),
        std::thread::current().name().unwrap_or("unnamed")
    ));
    let path = missing_parent.join("report.json");
    let _ = fs::remove_file(&path);
    let _ = fs::remove_dir(&missing_parent);
    let arena = sample_arena("missing-parent-proof");

    let error = arena
        .write_json(&path)
        .expect_err("missing parent directory should be an IO error");

    assert_eq!(error.kind(), std::io::ErrorKind::NotFound);
    assert!(
        !missing_parent.exists(),
        "write_json must not create parent directories automatically"
    );
}

#[test]
fn load_report_returns_io_error_for_missing_file() {
    let path = temp_report_path("load-report-missing");
    let _ = fs::remove_file(&path);

    let error = Arena::load_report(&path).expect_err("missing file should be IO error");

    match error {
        LoadReportError::Io(io_error) => {
            assert_eq!(io_error.kind(), std::io::ErrorKind::NotFound);
        }
        LoadReportError::Json(json_error) => {
            panic!("expected IO error for missing file, got JSON error: {json_error}");
        }
    }
}

#[test]
fn load_report_returns_json_error_for_invalid_json_file() {
    let path = temp_report_path("load-report-invalid-json");
    let _ = fs::remove_file(&path);
    fs::write(&path, "{ this is not valid JSON").expect("write invalid JSON");

    let error = Arena::load_report(&path).expect_err("invalid JSON should be JSON error");

    match error {
        LoadReportError::Json(json_error) => {
            assert!(json_error.is_syntax() || json_error.is_data());
        }
        LoadReportError::Io(io_error) => {
            panic!("expected JSON error for invalid JSON, got IO error: {io_error}");
        }
    }

    let _ = fs::remove_file(&path);
}

#[test]
fn arena_new_does_not_create_files() {
    let dir = temp_observation_dir("arena-new-no-files");

    let arena = Arena::new("memory-only-new");

    assert_eq!(arena.name(), "memory-only-new");
    assert_dir_is_empty(&dir);

    fs::remove_dir(&dir).expect("remove observation directory");
}

#[test]
fn push_and_push_str_do_not_create_files() {
    let dir = temp_observation_dir("push-no-files");
    let mut arena = Arena::new("memory-only-push");
    let mut users = RigVec::new(&mut arena, "users");
    let mut audit = RigString::new(&mut arena, "audit_events");

    users.push(1);
    audit.push_str("ok");

    assert_eq!(users.len(), 1);
    assert_eq!(audit.len(), 2);
    assert_dir_is_empty(&dir);

    fs::remove_dir(&dir).expect("remove observation directory");
}

#[test]
fn diff_and_diff_json_do_not_create_files() {
    let dir = temp_observation_dir("diff-no-files");
    let mut arena = Arena::new("memory-only-diff");
    let mut users = RigVec::with_capacity(&mut arena, "users", 1);
    users.push(1);
    let before = arena.snapshot();
    users.push(2);
    let after = arena.snapshot();

    let diff = before.diff(&after);
    let json = diff.diff_json();

    assert_eq!(diff.total_len_delta, 1);
    assert!(json.contains("memory-only-diff"));
    assert_dir_is_empty(&dir);

    fs::remove_dir(&dir).expect("remove observation directory");
}

#[test]
fn arena_report_write_json_fails_when_parent_directory_is_missing() {
    let mut missing_parent = std::env::temp_dir();
    missing_parent.push(format!(
        "rig-report-missing-parent-{}-{}",
        std::process::id(),
        std::thread::current().name().unwrap_or("unnamed")
    ));
    let path = missing_parent.join("report.json");
    let _ = fs::remove_file(&path);
    let _ = fs::remove_dir_all(&missing_parent);
    let report = sample_arena("report-missing-parent-proof").snapshot();

    let error = report
        .write_json(&path)
        .expect_err("missing parent directory should be an IO error");

    assert_eq!(error.kind(), std::io::ErrorKind::NotFound);
    assert!(!missing_parent.exists());
}

#[test]
fn arena_diff_write_json_fails_when_parent_directory_is_missing() {
    let mut missing_parent = std::env::temp_dir();
    missing_parent.push(format!(
        "rig-diff-missing-parent-{}-{}",
        std::process::id(),
        std::thread::current().name().unwrap_or("unnamed")
    ));
    let path = missing_parent.join("diff.json");
    let _ = fs::remove_file(&path);
    let _ = fs::remove_dir_all(&missing_parent);
    let report = sample_arena("diff-missing-parent-proof").snapshot();
    let diff = report.diff(&report);

    let error = diff
        .write_json(&path)
        .expect_err("missing parent directory should be an IO error");

    assert_eq!(error.kind(), std::io::ErrorKind::NotFound);
    assert!(!missing_parent.exists());
}

#[test]
fn report_snapshot_and_report_json_do_not_create_files() {
    let path = temp_report_path("no-implicit-persistence");
    let _ = fs::remove_file(&path);
    let arena = sample_arena("memory-only-proof");

    let snapshot = arena.snapshot();
    let report = arena.report();
    let json = arena.report_json();
    let snapshot_json = snapshot.report_json();

    assert_eq!(snapshot.arena_name, "memory-only-proof");
    assert!(report.contains("Arena: memory-only-proof"));
    assert!(json.contains("memory-only-proof"));
    assert_eq!(snapshot_json, json);
    assert!(
        !path.exists(),
        "in-memory report APIs must not create the temp report path"
    );
}

#[test]
fn identical_reports_produce_zero_deltas() {
    let mut arena = Arena::new("same-proof");
    let mut users = RigVec::with_capacity(&mut arena, "users", 8);
    users.push(1);
    users.push(2);
    let before = arena.snapshot();
    let after = before.clone();

    let diff = before.diff(&after);
    let users_diff = diff
        .containers_changed
        .iter()
        .find(|container| container.name == "users")
        .expect("users diff evidence");

    assert!(diff.containers_added.is_empty());
    assert!(diff.containers_removed.is_empty());
    assert_eq!(diff.total_len_delta, 0);
    assert_eq!(diff.total_capacity_delta, 0);
    assert_eq!(diff.total_growth_event_delta, 0);
    assert_eq!(diff.total_operation_delta, 0);
    assert_eq!(users_diff.len_delta, 0);
    assert_eq!(users_diff.capacity_delta, 0);
    assert_eq!(users_diff.growth_event_delta, 0);
    assert_eq!(users_diff.operation_delta, 0);
}

#[test]
fn added_container_detected() {
    let mut before_arena = Arena::new("added-proof");
    let mut before_users = RigVec::with_capacity(&mut before_arena, "users", 2);
    before_users.push(1);
    let before = before_arena.snapshot();

    let mut after_arena = Arena::new("added-proof");
    let mut after_users = RigVec::with_capacity(&mut after_arena, "users", 2);
    after_users.push(1);
    let mut jobs = RigVec::with_capacity(&mut after_arena, "jobs", 4);
    jobs.push(9);
    let after = after_arena.snapshot();

    let diff = before.diff(&after);

    assert_eq!(diff.containers_added.len(), 1);
    assert_eq!(diff.containers_added[0].name, "jobs");
    assert_eq!(diff.containers_added[0].len, 1);
    assert!(diff.containers_removed.is_empty());
}

#[test]
fn removed_container_detected() {
    let mut before_arena = Arena::new("removed-proof");
    let mut users = RigVec::with_capacity(&mut before_arena, "users", 2);
    users.push(1);
    let mut stale = RigString::with_capacity(&mut before_arena, "stale_sessions", 16);
    stale.push_str("expired");
    let before = before_arena.snapshot();

    let mut after_arena = Arena::new("removed-proof");
    let mut after_users = RigVec::with_capacity(&mut after_arena, "users", 2);
    after_users.push(1);
    let after = after_arena.snapshot();

    let diff = before.diff(&after);

    assert_eq!(diff.containers_removed.len(), 1);
    assert_eq!(diff.containers_removed[0].name, "stale_sessions");
    assert_eq!(diff.containers_removed[0].len, "expired".len());
    assert!(diff.containers_added.is_empty());
}

#[test]
fn len_increase_detected() {
    let mut arena = Arena::new("len-proof");
    let mut users = RigVec::with_capacity(&mut arena, "users", 8);
    for user_id in 1..=8 {
        users.push(user_id);
    }
    let before = arena.snapshot();

    for user_id in 9..=12 {
        users.push(user_id);
    }
    let after = arena.snapshot();
    let diff = before.diff(&after);
    let users_diff = diff
        .containers_changed
        .iter()
        .find(|container| container.name == "users")
        .expect("users diff evidence");

    assert_eq!(users_diff.before_len, 8);
    assert_eq!(users_diff.after_len, 12);
    assert_eq!(users_diff.len_delta, 4);
}

#[test]
fn capacity_increase_detected() {
    let mut arena = Arena::new("capacity-diff-proof");
    let mut users = RigVec::with_capacity(&mut arena, "users", 8);
    for user_id in 1..=8 {
        users.push(user_id);
    }
    let before = arena.snapshot();

    users.push(9);
    let after = arena.snapshot();
    let diff = before.diff(&after);
    let users_diff = diff
        .containers_changed
        .iter()
        .find(|container| container.name == "users")
        .expect("users diff evidence");

    assert_eq!(users_diff.before_capacity, 8);
    assert_eq!(users_diff.after_capacity, 16);
    assert_eq!(users_diff.capacity_delta, 8);
}

#[test]
fn growth_event_increase_detected() {
    let mut arena = Arena::new("growth-diff-proof");
    let mut users = RigVec::with_capacity(&mut arena, "users", 8);
    for user_id in 1..=8 {
        users.push(user_id);
    }
    let before = arena.snapshot();

    users.push(9);
    let after = arena.snapshot();
    let diff = before.diff(&after);
    let users_diff = diff
        .containers_changed
        .iter()
        .find(|container| container.name == "users")
        .expect("users diff evidence");

    assert_eq!(users_diff.before_growth_events, 0);
    assert_eq!(users_diff.after_growth_events, 1);
    assert_eq!(users_diff.growth_event_delta, 1);
}

#[test]
fn operation_increase_detected() {
    let mut arena = Arena::new("operation-diff-proof");
    let mut audit = RigString::with_capacity(&mut arena, "audit_events", 32);
    audit.push_str("start");
    let before = arena.snapshot();

    audit.push_str(";load");
    audit.push_str(";ok");
    let after = arena.snapshot();
    let diff = before.diff(&after);
    let audit_diff = diff
        .containers_changed
        .iter()
        .find(|container| container.name == "audit_events")
        .expect("audit diff evidence");

    assert_eq!(audit_diff.operation_label, "total append operations");
    assert_eq!(audit_diff.before_operations, 1);
    assert_eq!(audit_diff.after_operations, 3);
    assert_eq!(audit_diff.operation_delta, 2);
}

#[test]
fn json_diff_valid() {
    let mut arena = Arena::new("json-diff-proof");
    let mut users = RigVec::with_capacity(&mut arena, "users", 8);
    for user_id in 1..=8 {
        users.push(user_id);
    }
    let before = arena.snapshot();

    users.push(9);
    let diff = before.diff(&arena.snapshot());
    let json = diff.diff_json();
    let value: serde_json::Value = serde_json::from_str(&json).expect("valid diff JSON value");
    let decoded: rig::ArenaDiff = serde_json::from_str(&json).expect("valid ArenaDiff JSON");

    assert_eq!(value["before_arena_name"], "json-diff-proof");
    assert_eq!(value["containers_changed"][0]["name"], "users");
    assert_eq!(decoded, diff);
}

#[test]
fn diff_write_json_round_trips_as_valid_diff_json() {
    let path = temp_report_path("diff-write-json-round-trip");
    let _ = fs::remove_file(&path);
    let mut arena = Arena::new("diff-write-proof");
    let mut users = RigVec::with_capacity(&mut arena, "users", 2);
    users.push(1);
    let before = arena.snapshot();
    users.push(2);
    users.push(3);
    let diff = before.diff(&arena.snapshot());

    diff.write_json(&path).expect("write diff JSON");

    let written = fs::read_to_string(&path).expect("read diff JSON");
    let value: serde_json::Value = serde_json::from_str(&written).expect("valid JSON value");
    let decoded: rig::ArenaDiff = serde_json::from_str(&written).expect("valid ArenaDiff JSON");

    assert_eq!(value["before_arena_name"], "diff-write-proof");
    assert_eq!(decoded, diff);
    assert_eq!(written, diff.diff_json());

    let _ = fs::remove_file(&path);
}

#[test]
fn human_diff_readable() {
    let mut arena = Arena::new("human-diff-proof");
    let mut users = RigVec::with_capacity(&mut arena, "users", 8);
    for user_id in 1..=8 {
        users.push(user_id);
    }
    let before = arena.snapshot();

    for user_id in 9..=12 {
        users.push(user_id);
    }
    let diff = before.diff(&arena.snapshot());
    let report = diff.report();

    assert!(report.contains("RIG allocation diff"));
    assert!(report.contains("Before: human-diff-proof"));
    assert!(report.contains("After: human-diff-proof"));
    assert!(report.contains("  users"));
    assert!(report.contains("    len: +4"));
    assert!(report.contains("    capacity: +8"));
    assert!(report.contains("    growth events: +1"));
    assert!(report.contains("    operations: +4"));
}

#[test]
fn growth_rigvec_records_growth_history_from_actual_capacity_changes() {
    let mut arena = Arena::new("growth-vec-history");
    let mut users = RigVec::new(&mut arena, "users");

    let old_capacity = users.capacity();
    users.push(1);
    let new_capacity = users.capacity();
    let snapshot = arena.snapshot();

    assert!(new_capacity > old_capacity);
    assert_eq!(snapshot.growth_history.len(), 1);
    assert_eq!(snapshot.growth_history[0].container_name, "users");
    assert_eq!(snapshot.growth_history[0].container_kind, "RigVec");
    assert_eq!(snapshot.growth_history[0].old_capacity, old_capacity);
    assert_eq!(snapshot.growth_history[0].new_capacity, new_capacity);
    assert_eq!(snapshot.growth_history[0].operation_index, 1);
}

#[test]
fn growth_rigstring_records_growth_history_from_actual_capacity_changes() {
    let mut arena = Arena::new("growth-string-history");
    let mut audit = RigString::new(&mut arena, "audit_events");

    let old_capacity = audit.capacity();
    audit.push_str("first event");
    let new_capacity = audit.capacity();
    let snapshot = arena.snapshot();

    assert!(new_capacity > old_capacity);
    assert_eq!(snapshot.growth_history.len(), 1);
    assert_eq!(snapshot.growth_history[0].container_name, "audit_events");
    assert_eq!(snapshot.growth_history[0].container_kind, "RigString");
    assert_eq!(snapshot.growth_history[0].old_capacity, old_capacity);
    assert_eq!(snapshot.growth_history[0].new_capacity, new_capacity);
    assert_eq!(snapshot.growth_history[0].operation_index, 1);
}

#[test]
fn growth_rigvec_operation_index_is_push_count_after_growth_operation() {
    let mut arena = Arena::new("growth-vec-index");
    let mut users = RigVec::with_capacity(&mut arena, "users", 2);

    users.push(1);
    users.push(2);
    let old_capacity = users.capacity();
    users.push(3);
    let new_capacity = users.capacity();
    let event = arena
        .snapshot()
        .growth_history
        .into_iter()
        .find(|event| event.container_name == "users")
        .expect("users growth event");

    assert!(new_capacity > old_capacity);
    assert_eq!(event.old_capacity, old_capacity);
    assert_eq!(event.new_capacity, new_capacity);
    assert_eq!(event.operation_index, 3);
}

#[test]
fn growth_rigstring_operation_index_is_append_count_after_growth_operation() {
    let mut arena = Arena::new("growth-string-index");
    let mut audit = RigString::with_capacity(&mut arena, "audit_events", 4);

    audit.push_str("1234");
    let old_capacity = audit.capacity();
    audit.push_str("5");
    let new_capacity = audit.capacity();
    let event = arena
        .snapshot()
        .growth_history
        .into_iter()
        .find(|event| event.container_name == "audit_events")
        .expect("audit_events growth event");

    assert!(new_capacity > old_capacity);
    assert_eq!(event.old_capacity, old_capacity);
    assert_eq!(event.new_capacity, new_capacity);
    assert_eq!(event.operation_index, 2);
}

#[test]
fn growth_history_appears_in_snapshot() {
    let mut arena = Arena::new("growth-snapshot");
    let mut users = RigVec::new(&mut arena, "users");

    users.push(1);
    let snapshot = arena.snapshot();

    assert_eq!(snapshot.growth_history.len(), users.growth_events());
    assert_eq!(snapshot.growth_history[0].container_name, "users");
}

#[test]
fn growth_history_appears_in_human_report() {
    let mut arena = Arena::new("growth-human");
    let mut users = RigVec::new(&mut arena, "users");

    let old_capacity = users.capacity();
    users.push(1);
    let new_capacity = users.capacity();
    let report = arena.report();

    assert!(report.contains("Growth history:"));
    assert!(report.contains(&format!(
        "  users: {old_capacity} -> {new_capacity} at operation 1"
    )));
}

#[test]
fn growth_history_human_report_says_none_without_growth_events() {
    let mut arena = Arena::new("growth-human-none");
    let _users: RigVec<i32> = RigVec::with_capacity(&mut arena, "users", 4);

    let report = arena.report();

    assert!(report.contains("Growth history:\n  (none)"));
}

#[test]
fn growth_history_appears_in_json_report() {
    let mut arena = Arena::new("growth-json");
    let mut users = RigVec::new(&mut arena, "users");

    users.push(1);
    let json = arena.report_json();
    let value: serde_json::Value = serde_json::from_str(&json).expect("valid JSON value");

    assert_eq!(value["growth_history"][0]["container_name"], "users");
    assert_eq!(value["growth_history"][0]["container_kind"], "RigVec");
    assert_eq!(value["growth_history"][0]["operation_index"], 1);
}

#[test]
fn growth_history_json_report_round_trips_intact() {
    let path = temp_report_path("growth-json-round-trip");
    let _ = fs::remove_file(&path);
    let mut arena = Arena::new("growth-json-round-trip");
    let mut users = RigVec::new(&mut arena, "users");

    users.push(1);
    let snapshot = arena.snapshot();
    arena.write_json(&path).expect("write report JSON");
    let loaded = Arena::load_report(&path).expect("load report JSON");
    let decoded: rig::ArenaReport =
        serde_json::from_str(&arena.report_json()).expect("ArenaReport JSON");

    assert_eq!(decoded.growth_history, snapshot.growth_history);
    assert_eq!(loaded.growth_history, snapshot.growth_history);
    assert_eq!(loaded, snapshot);

    let _ = fs::remove_file(&path);
}

#[test]
fn growth_diff_includes_growth_events_added_after_first_snapshot() {
    let mut arena = Arena::new("growth-diff-added");
    let mut users = RigVec::with_capacity(&mut arena, "users", 1);

    users.push(1);
    let before = arena.snapshot();
    let old_capacity = users.capacity();
    users.push(2);
    let new_capacity = users.capacity();
    let after = arena.snapshot();
    let diff = before.diff(&after);

    assert!(new_capacity > old_capacity);
    assert!(before.growth_history.is_empty());
    assert_eq!(after.growth_history.len(), 1);
    assert_eq!(diff.growth_events_added, after.growth_history);
    assert_eq!(diff.growth_events_added[0].old_capacity, old_capacity);
    assert_eq!(diff.growth_events_added[0].new_capacity, new_capacity);
    assert_eq!(diff.growth_events_added[0].operation_index, 2);
}

#[test]
fn growth_no_growth_event_is_recorded_within_capacity() {
    let mut arena = Arena::new("growth-within-capacity");
    let mut users = RigVec::with_capacity(&mut arena, "users", 2);
    let mut audit = RigString::with_capacity(&mut arena, "audit_events", 8);

    users.push(1);
    users.push(2);
    audit.push_str("1234");
    audit.push_str("5678");
    let snapshot = arena.snapshot();

    assert_eq!(users.growth_events(), 0);
    assert_eq!(audit.growth_events(), 0);
    assert!(snapshot.growth_history.is_empty());
}
