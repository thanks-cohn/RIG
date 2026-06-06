use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn run_workload_example(example: &str) -> String {
    let output = Command::new(env!("CARGO"))
        .args(["run", "--quiet", "--example", example])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .unwrap_or_else(|error| panic!("failed to run example {example}: {error}"));

    assert!(
        output.status.success(),
        "example {example} failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    String::from_utf8(output.stdout)
        .unwrap_or_else(|error| panic!("example {example} stdout was not UTF-8: {error}"))
}

fn value_after_label(stdout: &str, label: &str) -> usize {
    stdout
        .lines()
        .find_map(|line| line.strip_prefix(label))
        .unwrap_or_else(|| panic!("missing label {label:?} in stdout"))
        .trim()
        .parse::<usize>()
        .unwrap_or_else(|error| panic!("could not parse numeric value after {label:?}: {error}"))
}

#[test]
fn workload_ecs_simulation_stdout_proves_real_tracked_entities() {
    let stdout = run_workload_example("ecs_simulation");

    assert!(stdout.contains("Arena: ecs-simulation"));
    assert!(stdout.contains("entities"));
    assert!(stdout.contains("positions"));
    assert!(stdout.contains("velocities"));
    assert!(stdout.contains("Growth history summary:"));
    assert!(stdout.contains("Growth history preview:"));
    assert!(stdout.contains("RIG allocation diff"));
    assert!(stdout.contains("\"growth_history\""));
    assert!(stdout.contains("ECS frames simulated: 60"));
    assert!(stdout.contains("Diff: empty to loaded"));
    assert!(stdout.contains("Diff: loaded to simulated"));

    let entity_count = value_after_label(&stdout, "ECS entities loaded:");
    assert!(
        entity_count >= 100_000,
        "expected at least 100,000 entities, saw {entity_count}"
    );
}

#[test]
fn workload_log_ingestion_stdout_proves_real_tracked_lines() {
    let stdout = run_workload_example("log_ingestion");

    assert!(stdout.contains("Arena: log-ingestion"));
    assert!(stdout.contains("raw_log_buffer"));
    assert!(stdout.contains("parsed_lines"));
    assert!(stdout.contains("error_lines"));
    assert!(stdout.contains("warning_lines"));
    assert!(stdout.contains("Growth history summary:"));
    assert!(stdout.contains("Growth history preview:"));
    assert!(stdout.contains("RIG allocation diff"));
    assert!(stdout.contains("\"growth_history\""));

    let line_count = value_after_label(&stdout, "Log lines processed:");
    assert!(
        line_count >= 50_000,
        "expected at least 50,000 parsed lines, saw {line_count}"
    );
    assert!(value_after_label(&stdout, "Log error lines:") > 0);
    assert!(value_after_label(&stdout, "Log warning lines:") > 0);
}

#[test]
fn workload_pathfinding_stdout_proves_real_reconstructed_path() {
    let stdout = run_workload_example("pathfinding");

    assert!(stdout.contains("Arena: pathfinding"));
    assert!(stdout.contains("frontier"));
    assert!(stdout.contains("visited"));
    assert!(stdout.contains("came_from"));
    assert!(stdout.contains("path"));
    assert!(stdout.contains("Growth history summary:"));
    assert!(stdout.contains("Growth history preview:"));
    assert!(stdout.contains("RIG allocation diff"));
    assert!(stdout.contains("\"growth_history\""));
    assert!(stdout.contains("Path found: true"));

    let path_length = value_after_label(&stdout, "Path length:");
    assert!(path_length > 0, "expected a nonzero reconstructed path");
}

#[test]
fn workload_examples_do_not_create_files_in_their_working_directory() {
    let mut temp_dir = std::env::temp_dir();
    temp_dir.push(format!("rig-workload-no-files-{}", std::process::id()));
    let _ = fs::remove_dir_all(&temp_dir);
    fs::create_dir(&temp_dir).expect("create isolated workload cwd");

    let manifest_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml");
    let target_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target");

    for example in ["ecs_simulation", "log_ingestion", "pathfinding"] {
        let output = Command::new(env!("CARGO"))
            .args([
                "run",
                "--quiet",
                "--manifest-path",
                manifest_path
                    .to_str()
                    .expect("manifest path is valid UTF-8"),
                "--example",
                example,
            ])
            .env("CARGO_TARGET_DIR", &target_dir)
            .current_dir(&temp_dir)
            .output()
            .unwrap_or_else(|error| panic!("failed to run example {example}: {error}"));

        assert!(
            output.status.success(),
            "example {example} failed\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }

    let created_entries = fs::read_dir(&temp_dir)
        .expect("read isolated workload cwd")
        .collect::<Result<Vec<_>, _>>()
        .expect("collect isolated workload cwd entries");
    assert!(
        created_entries.is_empty(),
        "workload examples created files in isolated cwd: {:?}",
        created_entries
            .iter()
            .map(|entry| entry.path())
            .collect::<Vec<_>>()
    );

    fs::remove_dir(&temp_dir).expect("remove isolated workload cwd");
}
