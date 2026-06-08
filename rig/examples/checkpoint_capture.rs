use rig::{Arena, EvidenceCapture, RigVec};
use std::fs;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let artifact_path = std::env::temp_dir().join(format!(
        "rig-checkpoint-capture-{}-artifact.json",
        std::process::id()
    ));
    let _ = fs::remove_file(&artifact_path);

    let mut arena = Arena::new("checkpoint-example-arena");
    let mut ids = RigVec::with_capacity(&mut arena, "ids", 2);
    let mut capture = EvidenceCapture::new("checkpoint_capture");

    ids.push(1);
    capture.capture_checkpoint("one-id", &arena);
    ids.push(2);
    ids.push(3);
    capture.capture_checkpoint("three-ids", &arena);

    let artifact = capture.artifact();
    artifact.save_json(&artifact_path)?;
    let loaded = rig::EvidenceArtifact::load_json(&artifact_path)?;
    let comparison = loaded
        .compare_checkpoints("one-id", "three-ids")
        .expect("named checkpoints exist");

    println!("{}", loaded.report());
    println!();
    println!("{}", comparison.report());
    println!();
    println!("Evidence artifact path: {}", artifact_path.display());
    println!(
        "Evidence captured checkpoints: {}",
        loaded.checkpoints.len()
    );
    println!(
        "Evidence total len delta: {}",
        comparison.diff.total_len_delta
    );

    fs::remove_file(&artifact_path)?;
    Ok(())
}
