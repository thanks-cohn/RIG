use rig::{Arena, RigString, RigVec};

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
    let expected = "RIG allocation report\nArena: format-check\nTracked containers: 2\nTotals:\n  total len: 0\n  total current capacity: 12\n  total growth events: 0\n  total pushed/appended operations: 0\nContainers:\n  Container: items\n  kind: RigVec\n  fields:\n    len: 0\n    initial capacity: 4\n    current capacity: 4\n    growth events: 0\n    total pushed items: 0\n  Container: audit_events\n  kind: RigString\n  fields:\n    len: 0\n    initial capacity: 8\n    current capacity: 8\n    growth events: 0\n    total append operations: 0\n    total appended bytes: 0";

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

    assert!(!repo_root.join("vendor").exists());
}
