use rig::{
    Arena, ArtifactComparison, GrowthPolicy, MemoryBudget, MemoryProfileKind, RegressionBudget,
    RigString, RigVec, WorkloadContract,
};
use std::path::PathBuf;

fn passing_workload() -> rig::ArenaReport {
    let mut arena = Arena::new("passing-workload");
    let mut values = RigVec::with_capacity(&mut arena, "level_entities", 8);
    values.push(1);
    values.push(2);
    arena.snapshot()
}

fn budget_failure_workload() -> rig::ArenaReport {
    let mut arena = Arena::new("budget-failure-workload");
    let mut values = RigVec::with_policy(&mut arena, "level_entities", GrowthPolicy::Exact);
    for value in 0..16 {
        values.push(value);
    }
    arena.snapshot()
}

fn profile_failure_workload() -> rig::ArenaReport {
    let mut arena = Arena::new("profile-failure-workload");
    let mut log = RigString::with_policy(
        &mut arena,
        "raw_log_buffer",
        GrowthPolicy::ReserveAhead(65_536),
    );
    log.push_str("start");
    arena.snapshot()
}

fn in_memory_comparison(
    baseline: rig::ArenaReport,
    current: rig::ArenaReport,
) -> ArtifactComparison {
    let diff = baseline.diff(&current);
    ArtifactComparison {
        baseline_path: PathBuf::from("baseline-in-memory.json"),
        current_path: PathBuf::from("current-in-memory.json"),
        baseline,
        current,
        diff,
    }
}

fn main() {
    let passing = passing_workload();
    let passing_contract = WorkloadContract::new("passing-level-contract")
        .with_description("Small workload stays inside its capacity budget and is stable enough.")
        .with_budget(MemoryBudget::max_total_capacity(
            passing.totals.total_current_capacity,
        ))
        .require_profile_present(MemoryProfileKind::Stable);
    let passing_report = passing.check_contract(&passing_contract);
    println!("{}", passing_report.report());
    println!();

    let budget_failure = budget_failure_workload();
    let budget_contract = WorkloadContract::new("budget-failure-contract").with_budget(
        MemoryBudget::max_total_capacity(budget_failure.totals.total_current_capacity - 1),
    );
    let budget_report = budget_failure.check_contract(&budget_contract);
    println!("{}", budget_report.report());
    println!();

    let regression_baseline = passing_workload();
    let regression_current = budget_failure_workload();
    let comparison = in_memory_comparison(regression_baseline, regression_current);
    let regression_contract = WorkloadContract::new("regression-failure-contract")
        .with_regression_budget(RegressionBudget::strict());
    let regression_report = comparison.check_contract(&regression_contract);
    println!("{}", regression_report.report());
    println!();

    let profile_failure = profile_failure_workload();
    let profile_contract = WorkloadContract::new("profile-failure-contract")
        .require_profile_absent(MemoryProfileKind::LargeSingleJump);
    let profile_report = profile_failure.check_contract(&profile_contract);
    println!("{}", profile_report.report());
    println!();

    println!("Failed contract JSON:");
    println!("{}", budget_report.report_json());
}
