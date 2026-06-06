use std::process::Command;

#[test]
fn policy_comparison_output_is_compact_but_preserves_policy_evidence() {
    let output = Command::new(env!("CARGO"))
        .args(["run", "--quiet", "--example", "policy_comparison"])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("run policy_comparison example");

    assert!(
        output.status.success(),
        "policy_comparison failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("stdout is UTF-8");
    for required in [
        "Policy comparison summary",
        "RustDefault",
        "Double",
        "Exact",
        "ReserveAhead(1024)",
        "Capped",
        "CapacityLimitExceeded",
        "growth_events",
        "Growth history summary:",
        "total_growth_events: 50000",
        "Full raw growth history is available through report_verbose() and report_json().",
        "\"growth_history\"",
    ] {
        assert!(stdout.contains(required), "stdout missing {required:?}");
    }

    let exact_growth_lines = stdout
        .lines()
        .filter(|line| line.trim_start().starts_with("values:"))
        .count();
    assert!(
        exact_growth_lines < 100,
        "policy_comparison flooded stdout with {exact_growth_lines} values growth lines"
    );
    assert!(
        !stdout.contains("at operation 25000"),
        "compact report leaked middle Exact growth event"
    );
}
