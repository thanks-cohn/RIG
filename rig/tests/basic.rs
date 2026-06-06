use rig::{Arena, RigVec};

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
    assert!(report.contains("len: 0"));
    assert!(report.contains("capacity: 0"));
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
    assert!(report.contains("len: 0"));
    assert!(report.contains("capacity: 0"));
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
    assert!(report.contains("len: 32"));
    assert!(report.contains("capacity:"));
    assert!(report.contains("growth events:"));
    assert!(report.contains("total pushed items: 32"));
    assert!(jobs.growth_events() > 0);
}
