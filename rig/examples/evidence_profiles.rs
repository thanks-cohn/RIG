use rig::{Arena, GrowthPolicy, MemoryBudget, RegressionBudget, RigString, RigVec};

fn stable_report() -> rig::ArenaReport {
    let mut arena = Arena::new("stable-profile");
    let mut values = RigVec::with_capacity(&mut arena, "preallocated_values", 8);
    values.push(1);
    values.push(2);
    arena.snapshot()
}

fn frequent_tiny_growth_report() -> rig::ArenaReport {
    let mut arena = Arena::new("frequent-tiny-profile");
    let mut values = RigVec::with_policy(&mut arena, "exact_values", GrowthPolicy::Exact);
    for value in 0..12 {
        values.push(value);
    }
    arena.snapshot()
}

fn large_jump_report() -> rig::ArenaReport {
    let mut arena = Arena::new("large-jump-profile");
    let mut text = RigString::with_policy(
        &mut arena,
        "reserve_ahead_text",
        GrowthPolicy::ReserveAhead(2_048),
    );
    text.push_str("observed");
    arena.snapshot()
}

fn main() {
    let stable = stable_report();
    let frequent_tiny = frequent_tiny_growth_report();
    let large_jump = large_jump_report();

    let failed_budget = frequent_tiny.check_budget(&MemoryBudget::max_total_growth_events(0));

    let baseline = stable_report();
    let current = frequent_tiny_growth_report();
    let failed_regression =
        current.check_regressions_against(&baseline, &RegressionBudget::strict());

    println!("Stable workload profiles:\n{}", stable.profile().report());
    println!();
    println!(
        "Frequent tiny growth workload profiles:\n{}",
        frequent_tiny.profile().report()
    );
    println!();
    println!(
        "Large jump workload profiles:\n{}",
        large_jump.profile().report()
    );
    println!();
    println!(
        "Failed budget profiles:\n{}",
        failed_budget.profile().report()
    );
    println!();
    println!(
        "Failed regression profiles:\n{}",
        failed_regression.profile().report()
    );
    println!();
    println!(
        "JSON profile report:\n{}",
        frequent_tiny.profile().report_json()
    );
}
