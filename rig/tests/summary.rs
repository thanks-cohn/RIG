use rig::{Arena, GrowthPolicy, GrowthSummary, RigString, RigVec};

fn exact_growth_arena(pushes: usize) -> Arena {
    let mut arena = Arena::new("exact-growth");
    let mut values = RigVec::with_policy(&mut arena, "values", GrowthPolicy::Exact);
    for value in 0..pushes {
        values.push(value);
    }
    arena
}

#[test]
fn summary_empty_growth_history_returns_zero_summary() {
    let mut arena = Arena::new("empty-summary");
    let _values: RigVec<u8> = RigVec::with_capacity(&mut arena, "values", 16);

    let summary = arena.growth_summary();

    assert_eq!(summary.total_growth_events, 0);
    assert_eq!(summary.containers_with_growth, 0);
    assert_eq!(summary.largest_growth_delta, 0);
    assert_eq!(summary.largest_growth_container, None);
    assert_eq!(summary.first_growth_event, None);
    assert_eq!(summary.last_growth_event, None);
    assert!(summary.per_container.is_empty());
}

#[test]
fn summary_one_growth_event_uses_the_raw_event_for_first_last_and_largest() {
    let mut arena = Arena::new("one-summary");
    let mut values = RigVec::with_capacity(&mut arena, "values", 2);
    values.push(1);
    values.push(2);
    values.push(3);

    let snapshot = arena.snapshot();
    assert_eq!(snapshot.growth_history.len(), 1);
    let raw_event = snapshot.growth_history[0].clone();
    let summary = snapshot.growth_summary();

    assert_eq!(summary.total_growth_events, 1);
    assert_eq!(summary.containers_with_growth, 1);
    assert_eq!(
        summary.largest_growth_delta,
        raw_event.new_capacity - raw_event.old_capacity
    );
    assert_eq!(summary.largest_growth_container, Some("values".to_owned()));
    assert_eq!(summary.first_growth_event, Some(raw_event.clone()));
    assert_eq!(summary.last_growth_event, Some(raw_event.clone()));
    assert_eq!(summary.per_container.len(), 1);
    assert_eq!(
        summary.per_container[0].container_name,
        raw_event.container_name
    );
    assert_eq!(
        summary.per_container[0].container_kind,
        raw_event.container_kind
    );
    assert_eq!(summary.per_container[0].growth_events, 1);
    assert_eq!(
        summary.per_container[0].first_old_capacity,
        raw_event.old_capacity
    );
    assert_eq!(
        summary.per_container[0].final_new_capacity,
        raw_event.new_capacity
    );
    assert_eq!(
        summary.per_container[0].largest_growth_delta,
        raw_event.new_capacity - raw_event.old_capacity
    );
    assert_eq!(
        summary.per_container[0].first_operation_index,
        raw_event.operation_index
    );
    assert_eq!(
        summary.per_container[0].last_operation_index,
        raw_event.operation_index
    );
}

#[test]
fn summary_multiple_containers_keeps_per_container_evidence_and_operation_bounds() {
    let mut arena = Arena::new("multi-summary");
    let mut values = RigVec::with_policy(&mut arena, "values", GrowthPolicy::Exact);
    let mut text = RigString::with_policy(&mut arena, "audit", GrowthPolicy::Exact);

    for value in 0..5 {
        values.push(value);
    }
    text.push_str("a");
    text.push_str("bc");
    text.push_str("def");

    let snapshot = arena.snapshot();
    let summary = snapshot.growth_summary();

    assert_eq!(summary.total_growth_events, snapshot.growth_history.len());
    assert_eq!(summary.containers_with_growth, 2);
    assert_eq!(summary.per_container.len(), 2);

    for container in &summary.per_container {
        let raw_events = snapshot
            .growth_history
            .iter()
            .filter(|event| {
                event.container_name == container.container_name
                    && event.container_kind == container.container_kind
            })
            .collect::<Vec<_>>();
        assert!(!raw_events.is_empty());
        assert_eq!(container.growth_events, raw_events.len());
        assert_eq!(container.first_old_capacity, raw_events[0].old_capacity);
        assert_eq!(
            container.final_new_capacity,
            raw_events.last().unwrap().new_capacity
        );
        assert_eq!(
            container.first_operation_index,
            raw_events[0].operation_index
        );
        assert_eq!(
            container.last_operation_index,
            raw_events.last().unwrap().operation_index
        );
        assert_eq!(
            container.largest_growth_delta,
            raw_events
                .iter()
                .map(|event| event.new_capacity - event.old_capacity)
                .max()
                .unwrap()
        );
    }
}

#[test]
fn summary_largest_growth_delta_is_computed_from_real_old_and_new_capacities() {
    let mut arena = Arena::new("largest-delta");
    let mut values = RigVec::with_policy(&mut arena, "values", GrowthPolicy::Double);
    for value in 0..128 {
        values.push(value);
    }

    let snapshot = arena.snapshot();
    let expected = snapshot
        .growth_history
        .iter()
        .map(|event| {
            (
                event.container_name.clone(),
                event.new_capacity - event.old_capacity,
            )
        })
        .max_by_key(|(_, delta)| *delta)
        .expect("growth evidence exists");
    let summary = snapshot.growth_summary();

    assert_eq!(summary.largest_growth_delta, expected.1);
    assert_eq!(summary.largest_growth_container, Some(expected.0));
}

#[test]
fn compact_report_omits_middle_exact_events_but_verbose_report_keeps_full_history() {
    let arena = exact_growth_arena(1_000);
    let compact = arena.report();
    let verbose = arena.report_verbose();
    let snapshot = arena.snapshot();

    assert_eq!(snapshot.growth_history.len(), 1_000);
    assert!(compact.contains("Growth history summary:"));
    assert!(compact.contains("total_growth_events: 1000"));
    assert!(compact.contains(
        "Full raw growth history is available through report_verbose() and report_json()."
    ));
    assert!(compact.contains("values: 0 ->"));
    assert!(compact.contains("at operation 1"));
    assert!(compact.contains("at operation 1000"));
    assert!(!compact.contains("at operation 500"));

    let compact_growth_lines = compact
        .lines()
        .filter(|line| line.trim_start().starts_with("values:"))
        .count();
    assert!(
        compact_growth_lines < 20,
        "compact report printed {compact_growth_lines} growth lines"
    );

    assert!(verbose.contains("Growth history (full raw evidence):"));
    assert!(verbose.contains("total_growth_events: 1000"));
    assert!(verbose.contains("at operation 500"));
    let verbose_growth_lines = verbose
        .lines()
        .filter(|line| line.trim_start().starts_with("values:"))
        .count();
    assert_eq!(verbose_growth_lines, snapshot.growth_history.len());
}

#[test]
fn arena_report_methods_match_arena_methods() {
    let arena = exact_growth_arena(16);
    let snapshot = arena.snapshot();

    assert_eq!(arena.report(), snapshot.report());
    assert_eq!(arena.report_verbose(), snapshot.report_verbose());
    assert_eq!(arena.growth_summary(), snapshot.growth_summary());
}

#[test]
fn growth_summary_json_is_valid_and_round_trips_while_raw_report_keeps_growth_history() {
    let arena = exact_growth_arena(12);
    let snapshot = arena.snapshot();

    let summary_json = snapshot.growth_summary_json();
    let decoded: GrowthSummary = serde_json::from_str(&summary_json).expect("summary JSON parses");
    assert_eq!(decoded, snapshot.growth_summary());

    let report_json = snapshot.report_json();
    let decoded_report: rig::ArenaReport =
        serde_json::from_str(&report_json).expect("report JSON parses");
    assert_eq!(decoded_report.growth_history.len(), 12);
    assert!(report_json.contains("\"growth_history\""));
    assert!(report_json.contains("\"old_capacity\""));
    assert!(report_json.contains("\"new_capacity\""));
}
