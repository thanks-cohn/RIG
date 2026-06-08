use rig::{Arena, EvidenceCapture, RigString, RigVec};
use std::fs;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let artifact_path = std::env::temp_dir().join(format!(
        "rig-workload-capture-{}-artifact.json",
        std::process::id()
    ));
    let _ = fs::remove_file(&artifact_path);

    let mut arena = Arena::new("workload-example-arena");
    let mut packets = RigVec::with_capacity(&mut arena, "packets", 1);
    let mut audit_log = RigString::with_capacity(&mut arena, "audit_log", 4);
    packets.push(0);
    audit_log.push_str("start");

    let mut capture = EvidenceCapture::new("workload_capture");
    let comparison = capture.capture_workload(&arena, "before-workload", "after-workload", || {
        for packet in 1..5 {
            packets.push(packet);
        }
        audit_log.push_str(";processed=4");
    });

    let artifact = capture.artifact();
    artifact.save_json(&artifact_path)?;

    println!("{}", artifact.report());
    println!();
    println!("{}", comparison.report());
    println!();
    println!("Machine-readable evidence report:");
    println!("{}", comparison.report_json());
    println!("Evidence artifact path: {}", artifact_path.display());
    println!(
        "Evidence captured checkpoints: {}",
        artifact.checkpoints.len()
    );
    println!(
        "Evidence captured growth events: {}",
        artifact
            .checkpoints
            .last()
            .map(|checkpoint| checkpoint.arena.totals.total_growth_events)
            .unwrap_or(0)
    );

    fs::remove_file(&artifact_path)?;
    Ok(())
}
