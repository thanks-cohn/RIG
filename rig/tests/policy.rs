use rig::{Arena, GrowthPolicy, RigError, RigString, RigVec};
use std::process::Command;

fn push_many(values: &mut RigVec<usize>, count: usize) {
    for value in 0..count {
        values.push(value);
    }
}

#[test]
fn policy_api_records_default_and_explicit_policies() {
    let mut arena = Arena::new("policy-api");
    let default_vec: RigVec<u8> = RigVec::new(&mut arena, "default_vec");
    let _double_vec: RigVec<u8> =
        RigVec::with_policy(&mut arena, "double_vec", GrowthPolicy::Double);
    let capacity_vec: RigVec<u8> = RigVec::with_capacity_and_policy(
        &mut arena,
        "capacity_vec",
        32,
        GrowthPolicy::ReserveAhead(1024),
    );
    let default_string = RigString::new(&mut arena, "default_string");

    assert_eq!(default_vec.capacity(), 0);
    assert_eq!(capacity_vec.capacity(), 32);
    assert_eq!(default_string.capacity(), 0);

    let snapshot = arena.snapshot();
    let default_report = snapshot
        .containers
        .iter()
        .find(|container| container.name == "default_vec")
        .expect("default vec report exists");
    assert_eq!(default_report.growth_policy, "RustDefault");

    let double_report = snapshot
        .containers
        .iter()
        .find(|container| container.name == "double_vec")
        .expect("double vec report exists");
    assert_eq!(double_report.growth_policy, "Double");

    let capacity_report = snapshot
        .containers
        .iter()
        .find(|container| container.name == "capacity_vec")
        .expect("capacity vec report exists");
    assert_eq!(capacity_report.initial_capacity, 32);
    assert_eq!(capacity_report.current_capacity, 32);
    assert_eq!(capacity_report.growth_policy, "ReserveAhead(1024)");
}

#[test]
fn policy_double_vec_records_growth_and_round_trips_json() {
    let mut arena = Arena::new("double-policy");
    let mut values = RigVec::with_policy(&mut arena, "values", GrowthPolicy::Double);
    let mut last_capacity = values.capacity();

    for value in 0..2048 {
        values.push(value);
        assert!(
            values.capacity() >= last_capacity,
            "capacity shrank from {last_capacity} to {}",
            values.capacity()
        );
        last_capacity = values.capacity();
    }

    assert_eq!(values.len(), 2048);
    assert!(values.growth_events() > 0);

    let human = arena.report();
    assert!(human.contains("growth policy: Double"));
    let json = arena.report_json();
    assert!(json.contains("\"growth_policy\": \"Double\""));
    let decoded: rig::ArenaReport = serde_json::from_str(&json).expect("report JSON decodes");
    assert_eq!(decoded.containers[0].growth_policy, "Double");
    assert_eq!(decoded.growth_history.len(), values.growth_events());
}

#[test]
fn policy_exact_vec_reaches_target_and_round_trips_json() {
    let mut arena = Arena::new("exact-policy");
    let mut values = RigVec::with_policy(&mut arena, "values", GrowthPolicy::Exact);

    push_many(&mut values, 1500);

    assert_eq!(values.len(), 1500);
    assert!(values.capacity() >= values.len());
    let snapshot = arena.snapshot();
    assert_eq!(snapshot.containers[0].growth_policy, "Exact");
    let decoded: rig::ArenaReport =
        serde_json::from_str(&snapshot.report_json()).expect("exact policy JSON round trip");
    assert_eq!(decoded, snapshot);
}

#[test]
fn policy_reserve_ahead_vec_records_amount_and_real_capacity() {
    let mut arena = Arena::new("reserve-ahead-policy");
    let mut values = RigVec::with_policy(&mut arena, "values", GrowthPolicy::ReserveAhead(1024));

    push_many(&mut values, 4096);

    assert_eq!(values.len(), 4096);
    assert!(values.capacity() >= values.len());
    let snapshot = arena.snapshot();
    let report = &snapshot.containers[0];
    assert_eq!(report.growth_policy, "ReserveAhead(1024)");
    assert_eq!(report.current_capacity, values.capacity());
    assert!(snapshot.report_json().contains("ReserveAhead(1024)"));
}

#[test]
fn capped_vec_returns_typed_error_and_preserves_state() {
    let mut arena = Arena::new("capped-vec");
    let mut values = RigVec::with_capacity_and_policy(
        &mut arena,
        "limited_values",
        2,
        GrowthPolicy::Capped { max_capacity: 3 },
    );

    values.try_push(1).expect("first push fits");
    values.try_push(2).expect("second push fits");
    values.try_push(3).expect("third push fits cap");
    let len_before = values.len();
    let capacity_before = values.capacity();

    let error = values.try_push(4).expect_err("fourth push exceeds cap");

    assert_eq!(values.len(), len_before);
    assert!(values.capacity() <= 3);
    assert_eq!(values.capacity(), capacity_before);
    assert_eq!(
        error,
        RigError::CapacityLimitExceeded {
            container_name: "limited_values".to_owned(),
            requested_capacity: 4,
            max_capacity: 3,
        }
    );
    assert!(error.to_string().contains("CapacityLimitExceeded"));
    assert!(error.to_string().contains("limited_values"));
}

#[test]
fn capped_string_returns_typed_error_and_preserves_state() {
    let mut arena = Arena::new("capped-string");
    let mut text = RigString::with_capacity_and_policy(
        &mut arena,
        "limited_text",
        5,
        GrowthPolicy::Capped { max_capacity: 5 },
    );

    text.try_push_str("abc").expect("first append fits");
    let len_before = text.len();
    let capacity_before = text.capacity();

    let error = text
        .try_push_str("def")
        .expect_err("append would exceed cap");

    assert_eq!(text.len(), len_before);
    assert_eq!(text.capacity(), capacity_before);
    assert!(text.capacity() <= 5);
    assert_eq!(
        error,
        RigError::CapacityLimitExceeded {
            container_name: "limited_text".to_owned(),
            requested_capacity: 6,
            max_capacity: 5,
        }
    );
}

#[test]
fn policy_string_reserve_ahead_records_real_growth_history() {
    let mut arena = Arena::new("string-policy");
    let mut text = RigString::with_policy(&mut arena, "audit", GrowthPolicy::ReserveAhead(1024));

    for event_id in 0..250 {
        text.push_str(&format!("event-{event_id};"));
    }

    assert!(text.len() > 0);
    assert!(text.growth_events() > 0);
    let snapshot = arena.snapshot();
    assert_eq!(snapshot.containers[0].growth_policy, "ReserveAhead(1024)");
    assert!(!snapshot.growth_history.is_empty());
    for event in &snapshot.growth_history {
        assert!(event.new_capacity > event.old_capacity);
        assert!(event.new_capacity <= text.capacity());
    }
}

#[test]
fn policy_comparison_example_outputs_evidence() {
    let output = Command::new(env!("CARGO"))
        .args(["run", "--quiet", "--example", "policy_comparison"])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("run policy_comparison example");

    assert!(
        output.status.success(),
        "policy_comparison failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("stdout is UTF-8");
    for required in [
        "RustDefault",
        "Double",
        "Exact",
        "ReserveAhead(1024)",
        "Capped",
        "CapacityLimitExceeded",
        "growth_events",
        "\"growth_policy\"",
    ] {
        assert!(stdout.contains(required), "stdout missing {required:?}");
    }
}

#[test]
#[should_panic(expected = "RigVec::push failed because growth policy refused capacity growth")]
fn capped_vec_push_panics_clearly_when_cap_is_exceeded() {
    let mut arena = Arena::new("capped-vec-panic");
    let mut values = RigVec::with_policy(
        &mut arena,
        "panic_values",
        GrowthPolicy::Capped { max_capacity: 1 },
    );

    values.push(1);
    values.push(2);
}

#[test]
#[should_panic(
    expected = "RigString::push_str failed because growth policy refused capacity growth"
)]
fn capped_string_push_str_panics_clearly_when_cap_is_exceeded() {
    let mut arena = Arena::new("capped-string-panic");
    let mut text = RigString::with_policy(
        &mut arena,
        "panic_text",
        GrowthPolicy::Capped { max_capacity: 1 },
    );

    text.push_str("a");
    text.push_str("b");
}
