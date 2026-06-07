use rig::{
    Arena, ArenaReport, CertificationSubject, MemoryBudget, RegressionBudget, RigVec,
    WorkloadContract,
};
use std::fs;
use std::path::PathBuf;

fn passing_report() -> ArenaReport {
    let mut arena = Arena::new("level-1-arena");
    let mut entities = RigVec::with_capacity(&mut arena, "entities", 4);
    entities.push(1);
    entities.push(2);
    arena.snapshot()
}

fn growing_report() -> ArenaReport {
    let mut arena = Arena::new("level-1-arena");
    let mut entities = RigVec::with_capacity(&mut arena, "entities", 1);
    for entity in 0..8 {
        entities.push(entity);
    }
    arena.snapshot()
}

fn temp_path(name: &str) -> PathBuf {
    let mut path = std::env::temp_dir();
    path.push(format!(
        "rig-evidence-certificate-{}-{name}.json",
        std::process::id()
    ));
    path
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let baseline = passing_report();
    let current = growing_report();

    let report_certificate = baseline
        .certify(CertificationSubject::new("level-1").with_description("Passing arena evidence"));

    let contract = WorkloadContract::new("level-1-memory-contract")
        .with_description("Require no growth regression and tiny current capacity")
        .with_budget(MemoryBudget::max_total_capacity(1))
        .with_regression_budget(RegressionBudget::strict());
    let contract_report = current.check_contract(&contract);
    let contract_certificate = contract_report.certify(
        CertificationSubject::new("level-1").with_description("Failing memory contract evidence"),
    );

    let baseline_path = temp_path("baseline");
    let current_path = temp_path("current");
    let _ = fs::remove_file(&baseline_path);
    let _ = fs::remove_file(&current_path);

    baseline.write_json(&baseline_path)?;
    current.write_json(&current_path)?;
    let baseline_artifact = rig::ReportArtifact::load(&baseline_path)?;
    let current_artifact = rig::ReportArtifact::load(&current_path)?;
    let comparison = baseline_artifact.compare_to(&current_artifact);
    let comparison_contract_report = comparison.check_contract(&contract);
    let comparison_certificate = comparison.certify(
        CertificationSubject::new("level-1-artifact")
            .with_description("Artifact comparison evidence"),
        Some(&comparison_contract_report),
    );

    println!("{}", report_certificate.report());
    println!();
    println!("{}", contract_certificate.report());
    println!();
    println!("{}", comparison_certificate.report());
    println!();
    println!("Failed contract certificate JSON:");
    println!("{}", contract_certificate.report_json());
    println!();
    println!(
        "Report certificate fingerprint: {}",
        report_certificate.fingerprint()
    );
    println!(
        "Contract certificate fingerprint: {}",
        contract_certificate.fingerprint()
    );
    println!(
        "Artifact certificate fingerprint: {}",
        comparison_certificate.fingerprint()
    );

    fs::remove_file(&baseline_path)?;
    fs::remove_file(&current_path)?;

    Ok(())
}
