use rig::{Arena, RigString, RigVec};

fn main() {
    let mut arena = Arena::new("request-lifetime arena");

    let mut users = RigVec::new(&mut arena, "users");
    let mut cached_users = RigVec::with_capacity(&mut arena, "cached_users", 4);
    let mut audit_events = RigString::new(&mut arena, "audit_events");
    let mut request_path = RigString::with_capacity(&mut arena, "request_path", 32);

    for user_id in 1..=8 {
        users.push(user_id);
    }

    for user_id in [10, 20, 30, 40] {
        cached_users.push(user_id);
    }

    audit_events.push_str("login");
    audit_events.push_str(" -> export");
    audit_events.push_str(" -> logout");

    request_path.push_str("/v1/users/42");

    println!("Rust is still safe, but allocation and growth behavior is now visible.\n");
    println!("{}", arena.report());
    println!();
    println!("{}", arena.report_json());
}
