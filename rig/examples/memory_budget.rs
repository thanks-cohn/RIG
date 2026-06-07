use rig::{Arena, GrowthPolicy, MemoryBudget, RigString, RigVec};

fn passing_workload() -> rig::ArenaReport {
    let mut arena = Arena::new("budget-pass");
    let mut values = RigVec::with_capacity_and_policy(&mut arena, "values", 8, GrowthPolicy::Exact);
    for value in 0..4 {
        values.push(value);
    }
    arena.snapshot()
}

fn failing_workload() -> rig::ArenaReport {
    let mut arena = Arena::new("budget-fail");
    let mut raw_log_buffer =
        RigString::with_policy(&mut arena, "raw_log_buffer", GrowthPolicy::Exact);
    raw_log_buffer.push_str("login ok\n");
    raw_log_buffer.push_str("checkout started\n");
    raw_log_buffer.push_str("checkout completed\n");

    let mut parsed_lines = RigVec::with_policy(&mut arena, "parsed_lines", GrowthPolicy::Exact);
    for line_len in ["login ok", "checkout started", "checkout completed"].map(str::len) {
        parsed_lines.push(line_len);
    }

    arena.snapshot()
}

fn main() {
    let passed_report = passing_workload();
    let pass_budget = MemoryBudget::unlimited()
        .with_max_total_capacity(passed_report.totals.total_current_capacity)
        .with_max_container_capacity(8);
    let pass_budget_report = passed_report.check_budget(&pass_budget);

    println!("{}", pass_budget_report.report());
    println!();

    let failed_report = failing_workload();
    let strict_budget = MemoryBudget::unlimited()
        .with_max_total_capacity(16)
        .with_max_container_capacity(8)
        .with_max_total_growth_events(2);
    let failed_budget_report = failed_report.check_budget(&strict_budget);

    println!("{}", failed_budget_report.report());
    println!();
    println!("{}", failed_budget_report.report_json());
}
