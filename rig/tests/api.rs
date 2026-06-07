use rig::{
    Arena, ArenaReport, ArtifactComparison, BudgetReport, ContainerReport, ContractReport,
    EvidenceExport, ExportFormat, GrowthAttribution, GrowthEvent, GrowthPolicy, GrowthSummary,
    MemoryBudget, ProfileReport, RegressionBudget, RegressionReport, ReportArtifact, RigError,
    RigString, RigVec, WorkloadContract,
};

fn assert_send_sync<T: Send + Sync>() {}
fn assert_clone_eq_debug<T: Clone + PartialEq + Eq + std::fmt::Debug>() {}

#[test]
fn api_public_data_types_keep_core_trait_contracts() {
    assert_clone_eq_debug::<GrowthPolicy>();
    assert_clone_eq_debug::<RigError>();
    assert_clone_eq_debug::<GrowthEvent>();
    assert_clone_eq_debug::<GrowthAttribution>();
    assert_clone_eq_debug::<GrowthSummary>();
    assert_clone_eq_debug::<ArenaReport>();
    assert_clone_eq_debug::<ContainerReport>();
    assert_clone_eq_debug::<EvidenceExport>();
    assert_clone_eq_debug::<ProfileReport>();
    assert_clone_eq_debug::<MemoryBudget>();
    assert_clone_eq_debug::<BudgetReport>();
    assert_clone_eq_debug::<RegressionBudget>();
    assert_clone_eq_debug::<RegressionReport>();
    assert_clone_eq_debug::<ReportArtifact>();
    assert_clone_eq_debug::<ArtifactComparison>();
    assert_clone_eq_debug::<WorkloadContract>();
    assert_clone_eq_debug::<ContractReport>();

    assert_send_sync::<GrowthPolicy>();
    assert_send_sync::<RigError>();
    assert_send_sync::<GrowthEvent>();
    assert_send_sync::<GrowthAttribution>();
    assert_send_sync::<GrowthSummary>();
    assert_send_sync::<ArenaReport>();
    assert_send_sync::<ContainerReport>();
    assert_send_sync::<EvidenceExport>();
    assert_send_sync::<ProfileReport>();
    assert_send_sync::<MemoryBudget>();
    assert_send_sync::<BudgetReport>();
    assert_send_sync::<RegressionBudget>();
    assert_send_sync::<RegressionReport>();
    assert_send_sync::<ReportArtifact>();
    assert_send_sync::<ArtifactComparison>();
    assert_send_sync::<WorkloadContract>();
    assert_send_sync::<ContractReport>();
}

#[test]
fn api_arena_vec_string_policy_error_and_report_methods_remain_callable() {
    let mut arena = Arena::new("api");
    assert_eq!(arena.name(), "api");

    let mut values = RigVec::with_capacity_and_policy(&mut arena, "values", 0, GrowthPolicy::Exact);
    assert!(values.is_empty());
    values.try_push(1).unwrap();
    values.push(2);
    assert_eq!(values.len(), 2);
    assert_eq!(values.total_pushed(), 2);
    assert!(values.capacity() >= values.len());
    assert!(values.growth_events() >= 1);

    let mut text = RigString::with_capacity_and_policy(&mut arena, "text", 0, GrowthPolicy::Double);
    assert!(text.is_empty());
    text.try_push_str("ab").unwrap();
    text.push_str("cd");
    assert_eq!(text.len(), 4);
    assert_eq!(text.append_operations(), 2);
    assert_eq!(text.total_appended_bytes(), 4);
    assert!(text.capacity() >= text.len());
    assert!(text.growth_events() >= 1);

    let mut capped = RigVec::with_policy(
        &mut arena,
        "capped",
        GrowthPolicy::Capped { max_capacity: 1 },
    );
    capped.try_push(1).unwrap();
    let error = capped.try_push(2).unwrap_err();
    assert_eq!(
        error,
        RigError::CapacityLimitExceeded {
            container_name: "capped".to_owned(),
            requested_capacity: 2,
            max_capacity: 1,
        }
    );
    assert!(error.to_string().contains("CapacityLimitExceeded"));

    let report = arena.snapshot();
    assert_eq!(
        serde_json::from_str::<ArenaReport>(&arena.report_json()).unwrap(),
        report
    );
    assert_eq!(arena.growth_summary(), report.growth_summary());
    assert_eq!(arena.growth_summary_json(), report.growth_summary_json());
    assert_eq!(arena.report(), report.report());
    assert_eq!(arena.report_verbose(), report.report_verbose());
    assert_eq!(
        report.top_growth_containers().len(),
        report.containers.len()
    );
    assert!(!report.containers_csv().is_empty());
    assert!(!report.growth_history_csv().is_empty());
    assert!(!report.growth_attributions_csv().is_empty());
    assert!(!report.containers_jsonl().is_empty());
    assert!(!report.growth_history_jsonl().is_empty());
    assert!(!report.growth_attributions_jsonl().is_empty());
    assert_eq!(
        report.export_containers(ExportFormat::Csv).kind,
        "containers"
    );
    assert_eq!(
        report.export_growth_history(ExportFormat::JsonLines).kind,
        "growth_history"
    );
    assert_eq!(
        report.export_growth_attributions(ExportFormat::Csv).kind,
        "growth_attributions"
    );
}

#[test]
fn api_budget_regression_artifact_profile_contract_and_export_methods_remain_callable() {
    let mut baseline_arena = Arena::new("baseline");
    let mut baseline_values = RigVec::with_capacity(&mut baseline_arena, "values", 4);
    baseline_values.push(1);
    let baseline = baseline_arena.snapshot();

    let mut current_arena = Arena::new("current");
    let mut current_values =
        RigVec::with_policy(&mut current_arena, "values", GrowthPolicy::ReserveAhead(2));
    current_values.push(1);
    current_values.push(2);
    let current = current_arena.snapshot();

    let budget = MemoryBudget::unlimited()
        .with_max_total_len(10)
        .with_max_total_capacity(100)
        .with_max_total_growth_events(10)
        .with_max_total_operations(10)
        .with_max_container_len(10)
        .with_max_container_capacity(100)
        .with_max_container_growth_events(10)
        .with_max_container_operations(10);
    assert!(current.check_budget(&budget).passed);
    assert!(
        current
            .check_budget(&MemoryBudget::max_total_capacity(usize::MAX))
            .passed
    );
    assert!(
        current
            .check_budget(&MemoryBudget::max_total_growth_events(usize::MAX))
            .passed
    );
    assert!(
        current
            .check_budget(&MemoryBudget::max_container_capacity(usize::MAX))
            .passed
    );
    assert!(
        current
            .check_budget(&MemoryBudget::max_container_growth_events(usize::MAX))
            .passed
    );

    let budget_report = current.check_budget(&MemoryBudget::strict_zero_growth());
    let _budget_json = budget_report.report_json();
    let _budget_human = budget_report.report();
    let _budget_profile = budget_report.profile();
    assert_eq!(
        budget_report.export_violations(ExportFormat::Csv).kind,
        "budget_violations"
    );
    assert_eq!(
        budget_report
            .export_violations(ExportFormat::JsonLines)
            .contents,
        budget_report.violations_jsonl()
    );
    assert!(budget_report
        .violations_csv()
        .contains("scope,container_name"));

    let regression_budget = RegressionBudget::strict();
    let regression_report = current.check_regressions_against(&baseline, &regression_budget);
    let _regression_json = regression_report.report_json();
    let _regression_human = regression_report.report();
    let _regression_profile = regression_report.profile();
    assert_eq!(
        regression_report.export_regressions(ExportFormat::Csv).kind,
        "regression_failures"
    );
    assert_eq!(
        regression_report
            .export_regressions(ExportFormat::JsonLines)
            .contents,
        regression_report.regressions_jsonl()
    );
    assert!(regression_report
        .regressions_csv()
        .contains("container_name,metric"));
    assert_eq!(
        RegressionBudget::allow_capacity_delta(usize::MAX).max_total_capacity_delta,
        Some(usize::MAX)
    );
    assert_eq!(
        RegressionBudget::allow_growth_events_delta(usize::MAX).max_total_growth_events_delta,
        Some(usize::MAX)
    );

    let profile = current.profile();
    assert_eq!(
        profile.report_json(),
        serde_json::to_string_pretty(&profile).unwrap()
    );
    assert_eq!(
        profile
            .profiles_by_kind(rig::MemoryProfileKind::Stable)
            .len(),
        0
    );
    assert!(profile.report().contains("RIG evidence profile report"));

    let contract = WorkloadContract::new("api-contract")
        .with_description("API smoke contract")
        .with_budget(budget)
        .with_regression_budget(regression_budget)
        .require_profile_absent(rig::MemoryProfileKind::BudgetRisk)
        .require_profile_present(rig::MemoryProfileKind::Stable);
    let contract_report = current.check_contract(&contract);
    assert_eq!(contract_report.contract_name, "api-contract");
    assert!(contract_report
        .report()
        .contains("RIG workload contract report"));
    assert_eq!(
        serde_json::from_str::<ContractReport>(&contract_report.report_json()).unwrap(),
        contract_report
    );

    let temp = std::env::temp_dir().join(format!("rig-api-{}-artifacts", std::process::id()));
    let _ = std::fs::remove_dir_all(&temp);
    std::fs::create_dir_all(&temp).unwrap();
    let baseline_artifact = baseline.write_artifact(temp.join("baseline.json")).unwrap();
    let current_artifact = current.write_artifact(temp.join("current.json")).unwrap();
    let loaded = ReportArtifact::load(temp.join("current.json")).unwrap();
    assert_eq!(loaded.report, current);

    let comparison = baseline_artifact.compare_to(&current_artifact);
    assert_eq!(
        comparison.regression_report(&RegressionBudget::strict()),
        regression_report
    );
    assert!(comparison.budget_report(&MemoryBudget::unlimited()).passed);
    assert!(
        comparison
            .check_contract(&WorkloadContract::new("empty"))
            .passed
    );
    assert!(!comparison.profile().profiles.is_empty());
    assert!(comparison
        .report()
        .contains("RIG report artifact comparison"));
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&comparison.report_json()).unwrap()["diff"],
        serde_json::to_value(&comparison.diff).unwrap()
    );
    assert!(comparison
        .summary_csv()
        .contains("baseline_path,current_path"));
    assert!(!comparison.summary_jsonl().is_empty());
    assert_eq!(
        comparison.export_summary(ExportFormat::Csv).kind,
        "artifact_comparison_summary"
    );

    let diff = baseline.diff(&current);
    assert_eq!(
        serde_json::from_str::<rig::ArenaDiff>(&diff.diff_json()).unwrap(),
        diff
    );
    assert_eq!(
        diff.growth_summary().total_growth_events,
        current.growth_summary().total_growth_events
    );
    assert!(diff.report().contains("RIG allocation diff"));

    std::fs::remove_dir_all(temp).unwrap();
}
