use rig::{Arena, RigVec};

fn main() {
    let mut arena = Arena::new("request-lifetime arena");

    let mut users = RigVec::new(&mut arena, "users");
    let mut events = RigVec::new(&mut arena, "audit_events");

    for user_id in 1..=8 {
        users.push(user_id);
    }

    for event in ["login", "load_dashboard", "export_report", "logout"] {
        events.push(event.to_string());
    }

    println!("Rust is still safe, but memory growth is now visible.\n");
    println!("{}", arena.report());
}
