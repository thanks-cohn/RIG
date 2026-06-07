use std::process::Command;

#[test]
fn validation_summary_self_test_generates_summary_and_audit_from_inputs() {
    let script = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crate has repository parent")
        .join("scripts")
        .join("validate.sh");

    let output = Command::new("bash")
        .arg(script)
        .arg("--self-test-summary")
        .output()
        .expect("run validation summary self test");

    assert!(
        output.status.success(),
        "validation summary self test failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}
