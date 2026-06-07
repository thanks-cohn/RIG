use rig::{Arena, MemoryBudget, RegressionBudget, RigVec};
use std::fs;

fn build_report(arena_name: &str, values: usize) -> rig::ArenaReport {
    let mut arena = Arena::new(arena_name);
    let mut ids = RigVec::with_capacity(&mut arena, "ids", 2);

    for value in 0..values {
        ids.push(value);
    }

    arena.snapshot()
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let artifact_dir =
        std::env::temp_dir().join(format!("rig-artifact-compare-{}", std::process::id()));
    let baseline_path = artifact_dir.join("baseline.json");
    let current_path = artifact_dir.join("current.json");

    let _ = fs::remove_file(&baseline_path);
    let _ = fs::remove_file(&current_path);
    let _ = fs::remove_dir(&artifact_dir);
    fs::create_dir(&artifact_dir)?;

    let baseline_report = build_report("artifact-baseline", 2);
    let current_report = build_report("artifact-current", 6);

    baseline_report.write_artifact(&baseline_path)?;
    current_report.write_artifact(&current_path)?;

    let baseline_artifact = rig::ReportArtifact::load(&baseline_path)?;
    let current_artifact = rig::ReportArtifact::load(&current_path)?;
    let comparison = baseline_artifact.compare_to(&current_artifact);

    println!("{}", comparison.report());
    println!();
    println!("{}", comparison.report_json());
    println!();

    let regression_report = comparison.regression_report(&RegressionBudget::strict());
    println!("{}", regression_report.report());
    println!();

    let budget_report = comparison.budget_report(&MemoryBudget::max_total_capacity(
        baseline_report.totals.total_current_capacity,
    ));
    println!("{}", budget_report.report());

    fs::remove_file(&baseline_path)?;
    fs::remove_file(&current_path)?;
    fs::remove_dir(&artifact_dir)?;

    Ok(())
}
