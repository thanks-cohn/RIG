use rig::{
    AllocationBudget, Arena, BenchmarkEvidence, ContainerBudget, GrowthPolicy,
    GrowthProfileExpectation, MemoryProfileKind, RegressionBudget, RegressionExpectation,
    RigString, RigVec, WorkloadMemoryContract,
};

fn successful_workload() -> rig::ArenaReport {
    let mut arena = Arena::new("successful-certification");
    let mut ids = RigVec::with_capacity(&mut arena, "ids", 8);
    for id in 0..6 {
        ids.push(id);
    }
    arena.snapshot()
}

fn growth_workload(name: &str, count: usize) -> rig::ArenaReport {
    let mut arena = Arena::new(name);
    let mut ids = RigVec::with_policy(&mut arena, "ids", GrowthPolicy::Exact);
    for id in 0..count {
        ids.push(id);
    }
    arena.snapshot()
}

fn large_jump_workload() -> rig::ArenaReport {
    let mut arena = Arena::new("growth-profile-validation");
    let mut logs = RigString::with_policy(&mut arena, "logs", GrowthPolicy::ReserveAhead(2_048));
    logs.push_str("observed");
    arena.snapshot()
}

fn main() {
    let success_evidence = BenchmarkEvidence::from_current("success", successful_workload());
    let success_contract = WorkloadMemoryContract::new("successful workload certification")
        .with_allocation_budget(
            AllocationBudget::unlimited()
                .with_max_arena_capacity(success_evidence.current.totals.total_current_capacity)
                .with_max_growth_events(0),
        )
        .with_container_budget(ContainerBudget::any_container().with_max_capacity(8));
    let success = success_contract.validate(&success_evidence);
    let certificate = success.certify(rig::CertificationSubject::new("success"));
    println!(
        "1. Successful workload certification\n{}\n",
        success.report()
    );
    println!(
        "Evidence certificate fingerprint: {}\n",
        certificate.fingerprint()
    );

    let failure_contract = WorkloadMemoryContract::new("failed workload certification")
        .with_allocation_budget(AllocationBudget::unlimited().with_max_arena_capacity(1));
    let failure = failure_contract.validate(&success_evidence);
    println!("2. Failed workload certification\n{}\n", failure.report());

    let budget_evidence =
        BenchmarkEvidence::from_current("budget-enforcement", growth_workload("budget", 12));
    let budget_contract = WorkloadMemoryContract::new("budget enforcement")
        .with_container_budget(ContainerBudget::named("ids").with_max_growth_events(3));
    println!(
        "3. Budget enforcement\n{}\n",
        budget_contract.validate(&budget_evidence).report()
    );

    let baseline = growth_workload("regression-baseline", 4);
    let current = growth_workload("regression-current", 16);
    let comparison = BenchmarkEvidence::compare("benchmark comparison", baseline, current);
    let regression_contract =
        WorkloadMemoryContract::new("regression detection").with_regression_expectation(
            RegressionExpectation::new("strict growth regression", RegressionBudget::strict()),
        );
    println!(
        "4. Regression detection\n{}\n",
        regression_contract.validate(&comparison).report()
    );
    println!(
        "5. Benchmark comparison\n{}\n",
        comparison
            .strict_regression_report
            .as_ref()
            .expect("comparison has strict regression evidence")
            .report()
    );

    let profile_evidence = BenchmarkEvidence::from_current("profile", large_jump_workload());
    let profile_contract = WorkloadMemoryContract::new("growth profile validation")
        .with_growth_profile_expectation(
            GrowthProfileExpectation::empty()
                .require(MemoryProfileKind::LargeSingleJump)
                .forbid(MemoryProfileKind::Stable),
        );
    println!(
        "6. Growth profile validation\n{}\nJSON evidence:\n{}",
        profile_contract.validate(&profile_evidence).report(),
        profile_contract.validate(&profile_evidence).report_json()
    );
}
