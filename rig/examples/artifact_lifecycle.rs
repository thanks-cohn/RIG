use rig::{Arena, EvidenceArtifact, EvidenceCapture, RigVec};
use std::fs;

fn capture_artifact(workload: &str, arena_name: &str, values: usize) -> rig::EvidenceArtifact {
    let mut arena = Arena::new(arena_name);
    let mut ids = RigVec::with_capacity(&mut arena, "ids", 2);
    for value in 0..values {
        ids.push(value);
    }
    let mut capture = EvidenceCapture::new(workload);
    capture.capture_checkpoint("final", &arena);
    capture.artifact()
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let dir = std::env::temp_dir().join(format!("rig-artifact-lifecycle-{}", std::process::id()));
    let baseline_path = dir.join("baseline-evidence.json");
    let current_path = dir.join("current-evidence.json");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir(&dir)?;

    let baseline = capture_artifact("artifact_lifecycle_baseline", "lifecycle-baseline", 2);
    let current = capture_artifact("artifact_lifecycle_current", "lifecycle-current", 5);
    baseline.save_json(&baseline_path)?;
    current.save_json(&current_path)?;

    let loaded_baseline = EvidenceArtifact::load_json(&baseline_path)?;
    let loaded_current = EvidenceArtifact::load_json(&current_path)?;
    let comparison = loaded_baseline
        .compare_latest(&loaded_current)
        .expect("both lifecycle artifacts have final checkpoints");

    println!("Baseline artifact path: {}", baseline_path.display());
    println!("Current artifact path: {}", current_path.display());
    println!("{}", comparison.report());
    println!();
    println!("Machine-readable artifact comparison:");
    println!("{}", comparison.report_json());
    println!("Evidence artifacts generated: 2");
    println!(
        "Evidence total len delta: {}",
        comparison.diff.total_len_delta
    );
    println!(
        "Evidence total capacity delta: {}",
        comparison.diff.total_capacity_delta
    );

    fs::remove_file(&baseline_path)?;
    fs::remove_file(&current_path)?;
    fs::remove_dir(&dir)?;
    Ok(())
}
