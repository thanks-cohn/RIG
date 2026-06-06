use rig::{Arena, RigString, RigVec};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut arena = Arena::new("request-lifetime arena");

    let mut users = RigVec::new(&mut arena, "users");
    for user_id in 1..=8 {
        users.push(user_id);
    }

    let mut cached_users = RigVec::with_capacity(&mut arena, "cached_users", 4);
    for user_id in [101, 102, 103, 104] {
        cached_users.push(user_id);
    }

    let mut audit_events = RigString::new(&mut arena, "audit_events");
    audit_events.push_str("request started; ");
    audit_events.push_str("db loaded; ");
    audit_events.push_str("ok");

    let mut request_path = RigString::with_capacity(&mut arena, "request_path", 32);
    request_path.push_str("/v1/users/42");

    println!("Rust is still safe, but allocation and growth behavior is now visible.\n");
    println!("{}", arena.report());
    println!();
    println!("{}", arena.report_json());
    println!();

    let report_path = std::env::temp_dir().join(format!("rig-demo-{}.json", std::process::id()));
    println!("Writing report only because demo explicitly called write_json.");
    arena.write_json(&report_path)?;
    let loaded_report = Arena::load_report(&report_path)?;
    let loaded_matches_snapshot = loaded_report == arena.snapshot();

    println!("Report path: {}", report_path.display());
    println!(
        "Loaded report equals live snapshot: {}",
        loaded_matches_snapshot
    );

    Ok(())
}
