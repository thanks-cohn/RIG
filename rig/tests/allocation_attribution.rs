use rig::{Arena, ArenaReport, GrowthAttribution, GrowthPolicy, RigString, RigVec};
use std::process::Command;

fn attribution_arena() -> ArenaReport {
    let mut arena = Arena::new("attribution-tests");
    let mut values = RigVec::with_policy(&mut arena, "values", GrowthPolicy::Double);
    let mut audit = RigString::with_policy(&mut arena, "audit", GrowthPolicy::Exact);
    let mut idle = RigVec::<usize>::with_capacity(&mut arena, "idle", 32);

    for value in 0..128 {
        values.push(value);
    }
    audit.push_str("abc");
    audit.push_str("defgh");
    audit.push_str("ijklmnop");
    idle.push(1);

    arena.snapshot()
}

#[test]
fn container_lifetime_growth_statistics_are_derived_from_growth_events() {
    let report = attribution_arena();

    for container in &report.containers {
        let evidence = report
            .growth_history
            .iter()
            .filter(|event| event.container_name == container.name)
            .collect::<Vec<_>>();
        let expected_total: usize = evidence.iter().map(|event| event.capacity_added).sum();
        let expected_largest = evidence
            .iter()
            .map(|event| event.capacity_added)
            .max()
            .unwrap_or(0);
        let expected_average = if evidence.is_empty() {
            0
        } else {
            expected_total / evidence.len()
        };

        assert_eq!(container.total_capacity_added, expected_total);
        assert_eq!(container.largest_growth_jump, expected_largest);
        assert_eq!(container.average_growth_jump, expected_average);
    }
}

#[test]
fn growth_attribution_records_causal_event_fields() {
    let report = attribution_arena();

    assert_eq!(
        report.growth_attributions.len(),
        report.growth_history.len()
    );
    for (event, attribution) in report
        .growth_history
        .iter()
        .zip(&report.growth_attributions)
    {
        let expected = GrowthAttribution::from(event);
        assert_eq!(attribution, &expected);
        assert_eq!(
            attribution.capacity_added,
            event.new_capacity - event.old_capacity
        );
        assert!(!attribution.growth_policy.is_empty());
    }
}

#[test]
fn top_growth_containers_orders_by_total_capacity_added() {
    let report = attribution_arena();
    let ranked = report.top_growth_containers();

    assert_eq!(ranked.len(), report.containers.len());
    assert!(ranked
        .windows(2)
        .all(|pair| pair[0].total_capacity_added >= pair[1].total_capacity_added));
    assert_eq!(ranked.last().unwrap().name, "idle");
    assert_eq!(ranked.last().unwrap().total_capacity_added, 0);
}

#[test]
fn allocation_attribution_json_round_trips() {
    let report = attribution_arena();
    let json = report.report_json();
    let decoded: ArenaReport = serde_json::from_str(&json).expect("ArenaReport JSON parses");

    assert_eq!(decoded, report);
    assert!(json.contains("\"total_capacity_added\""));
    assert!(json.contains("\"largest_growth_jump\""));
    assert!(json.contains("\"average_growth_jump\""));
    assert!(json.contains("\"growth_attributions\""));
    assert!(json.contains("\"growth_policy\""));
}

#[test]
fn human_report_includes_top_growth_contributors() {
    let report = attribution_arena();
    let human = report.report();

    assert!(human.contains("Top growth contributors:"));
    assert!(human.contains("total capacity added:"));
    assert!(human.contains("total capacity added: "));
    assert!(human.contains("largest growth jump:"));
    assert!(human.contains("average growth jump:"));
}

#[test]
fn allocation_attribution_example_prints_required_sections_and_valid_json() {
    let output = Command::new(env!("CARGO"))
        .args(["run", "--example", "allocation_attribution"])
        .output()
        .expect("example process starts");

    assert!(
        output.status.success(),
        "example failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("stdout is UTF-8");
    assert!(stdout.contains("Top growth contributors:"));
    assert!(stdout.contains("Attribution events:"));
    assert!(stdout.contains("Report JSON:"));
    assert!(stdout.contains("raw_log_buffer"));
    assert!(stdout.contains("values"));
    assert!(stdout.contains("growth_attributions"));

    let json_start = stdout
        .find("{\n")
        .expect("pretty report JSON begins with an object");
    let decoded: ArenaReport =
        serde_json::from_str(&stdout[json_start..]).expect("example JSON parses");
    assert!(!decoded.top_growth_containers().is_empty());
    assert!(!decoded.growth_attributions.is_empty());
}
