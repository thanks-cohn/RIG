use rig::{Arena, RigString, RigVec};

const LINE_COUNT: usize = 50_000;

fn main() {
    let mut arena = Arena::new("log-ingestion");

    let mut raw_log_buffer = RigString::new(&mut arena, "raw_log_buffer");
    let mut parsed_lines = RigVec::new(&mut arena, "parsed_lines");
    let mut error_lines = RigVec::new(&mut arena, "error_lines");
    let mut warning_lines = RigVec::new(&mut arena, "warning_lines");

    let before = arena.snapshot();

    for index in 0..LINE_COUNT {
        let level = if index % 97 == 0 {
            "ERROR"
        } else if index % 13 == 0 {
            "WARN"
        } else {
            "INFO"
        };
        let line = format!(
            "ts=2026-06-06T00:{:02}:{:02}Z level={level} shard={} request={} bytes={}\n",
            (index / 60) % 60,
            index % 60,
            index % 16,
            10_000 + index,
            512 + (index % 4_096)
        );

        raw_log_buffer.push_str(&line);
        let parsed = line.trim_end().to_owned();
        if level == "ERROR" {
            error_lines.push(parsed.clone());
        } else if level == "WARN" {
            warning_lines.push(parsed.clone());
        }
        parsed_lines.push(parsed);
    }

    let after = arena.snapshot();
    let diff = before.diff(&after);

    println!("Log lines processed: {}", parsed_lines.len());
    println!("Log error lines: {}", error_lines.len());
    println!("Log warning lines: {}", warning_lines.len());
    println!(
        "Log bytes ingested: {}",
        raw_log_buffer.total_appended_bytes()
    );
    println!();
    println!("{}", arena.report());
    println!();
    println!("{}", diff.report());
    println!();
    println!("Growth history count: {}", after.growth_history.len());
    println!();
    println!("{}", after.report_json());
}
