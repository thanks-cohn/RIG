use rig::{
    Arena, ExportFormat, GrowthPolicy, MemoryBudget, RegressionBudget, ReportArtifact, RigString,
    RigVec,
};
use std::fs;

fn build_report(arena_name: &str, pushes: usize) -> rig::ArenaReport {
    let mut arena = Arena::new(arena_name);
    let mut values = RigVec::with_policy(&mut arena, "values", GrowthPolicy::Exact);
    for value in 0..pushes {
        values.push(value);
    }

    let mut notes = RigString::with_capacity(&mut arena, "notes", 4);
    notes.push_str("rig");
    notes.push_str(" evidence exports");

    arena.snapshot()
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let report = build_report("evidence-current", 5);

    println!("containers CSV");
    println!("{}", report.containers_csv());

    println!("growth history JSONL");
    println!("{}", report.growth_history_jsonl());

    let budget_report = report.check_budget(
        &MemoryBudget::unlimited()
            .with_max_total_capacity(4)
            .with_max_container_growth_events(1),
    );
    println!("budget violations CSV");
    println!("{}", budget_report.violations_csv());

    let baseline = build_report("evidence-baseline", 1);
    let regression_report =
        report.check_regressions_against(&baseline, &RegressionBudget::strict());
    println!("regression JSONL");
    println!("{}", regression_report.regressions_jsonl());

    let artifact_dir = std::env::temp_dir().join(format!(
        "rig-evidence-exports-example-{}",
        std::process::id()
    ));
    let baseline_path = artifact_dir.join("baseline.json");
    let current_path = artifact_dir.join("current.json");
    let export_path = artifact_dir.join("containers.csv");

    let _ = fs::remove_file(&baseline_path);
    let _ = fs::remove_file(&current_path);
    let _ = fs::remove_file(&export_path);
    let _ = fs::remove_dir(&artifact_dir);
    fs::create_dir(&artifact_dir)?;

    baseline.write_artifact(&baseline_path)?;
    report.write_artifact(&current_path)?;
    let baseline_artifact = ReportArtifact::load(&baseline_path)?;
    let current_artifact = ReportArtifact::load(&current_path)?;
    let comparison = baseline_artifact.compare_to(&current_artifact);

    println!("artifact summary CSV");
    println!("{}", comparison.summary_csv());

    let export = report.export_containers(ExportFormat::Csv);
    export.write_to(&export_path)?;
    let round_trip = fs::read_to_string(&export_path)? == export.contents;
    println!("export file round-trip: {round_trip}");

    fs::remove_file(&export_path)?;
    fs::remove_file(&baseline_path)?;
    fs::remove_file(&current_path)?;
    fs::remove_dir(&artifact_dir)?;

    Ok(())
}
