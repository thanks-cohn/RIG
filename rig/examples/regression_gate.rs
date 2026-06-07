use rig::{Arena, GrowthPolicy, RegressionBudget, RigString, RigVec};

fn baseline_workload() -> rig::ArenaReport {
    let mut arena = Arena::new("log-parser-baseline");
    let mut raw_log_buffer =
        RigString::with_capacity_and_policy(&mut arena, "raw_log_buffer", 32, GrowthPolicy::Exact);
    let mut parsed_lines =
        RigVec::with_capacity_and_policy(&mut arena, "parsed_lines", 2, GrowthPolicy::Exact);

    raw_log_buffer.push_str("INFO boot\nWARN cache\n");
    parsed_lines.push("INFO boot");
    parsed_lines.push("WARN cache");

    arena.snapshot()
}

fn current_workload_with_regression() -> rig::ArenaReport {
    let mut arena = Arena::new("log-parser-current");
    let mut raw_log_buffer =
        RigString::with_capacity_and_policy(&mut arena, "raw_log_buffer", 32, GrowthPolicy::Exact);
    let mut parsed_lines =
        RigVec::with_capacity_and_policy(&mut arena, "parsed_lines", 2, GrowthPolicy::Exact);

    raw_log_buffer.push_str("INFO boot\nWARN cache\nERROR disk\nINFO done\n");
    for line in ["INFO boot", "WARN cache", "ERROR disk", "INFO done"] {
        parsed_lines.push(line);
    }

    arena.snapshot()
}

fn main() {
    let baseline = baseline_workload();
    let current = current_workload_with_regression();

    let failed = current.check_regressions_against(&baseline, &RegressionBudget::strict());
    println!("{}", failed.report());
    println!();
    println!("{}", failed.report_json());
    println!();

    let mut passing_budget = RegressionBudget::allow_capacity_delta(
        failed
            .regressions
            .iter()
            .filter(|regression| regression.metric.ends_with("capacity"))
            .map(|regression| regression.delta)
            .max()
            .unwrap_or(0),
    );
    passing_budget.max_total_growth_events_delta =
        Some(failed.total_growth_event_delta.max(0) as usize);
    passing_budget.max_container_growth_events_delta = failed
        .regressions
        .iter()
        .filter(|regression| regression.metric.ends_with("growth_events"))
        .map(|regression| regression.delta)
        .max();

    let passed = current.check_regressions_against(&baseline, &passing_budget);
    println!("{}", passed.report());
}
