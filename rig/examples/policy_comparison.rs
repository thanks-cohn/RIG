use rig::{Arena, GrowthPolicy, RigVec};

const PUSHES: usize = 50_000;

fn run_policy(policy: GrowthPolicy, label: &str) -> Arena {
    let mut arena = Arena::new(format!("policy-comparison-{label}"));
    let mut values = RigVec::with_policy(&mut arena, "values", policy);

    for value in 0..PUSHES {
        values.push(value);
    }

    arena
}

fn print_summary_row(arena: &Arena) {
    let snapshot = arena.snapshot();
    let container = &snapshot.containers[0];
    println!(
        "{:<22} {:>8} {:>10} {:>13} {:>10}",
        container.growth_policy,
        container.len,
        container.current_capacity,
        container.growth_events,
        container.total_operations
    );
}

fn main() {
    let policies = [
        (GrowthPolicy::RustDefault, "rust-default"),
        (GrowthPolicy::Double, "double"),
        (GrowthPolicy::Exact, "exact"),
        (GrowthPolicy::ReserveAhead(1024), "reserve-ahead"),
    ];

    let arenas = policies
        .into_iter()
        .map(|(policy, label)| run_policy(policy, label))
        .collect::<Vec<_>>();

    println!(
        "{:<22} {:>8} {:>10} {:>13} {:>10}",
        "policy", "len", "capacity", "growth_events", "operations"
    );
    for arena in &arenas {
        print_summary_row(arena);
    }

    for arena in &arenas {
        println!("\n{}", arena.report());
    }

    println!("\nJSON report sample:");
    println!("{}", arenas[1].report_json());

    let mut capped_arena = Arena::new("policy-comparison-capped");
    let mut capped = RigVec::with_policy(
        &mut capped_arena,
        "capped_values",
        GrowthPolicy::Capped { max_capacity: 8 },
    );
    for value in 0..8 {
        capped.try_push(value).expect("value fits capped policy");
    }
    match capped.try_push(8) {
        Ok(()) => println!("capped workload unexpectedly succeeded"),
        Err(error) => println!("Capped failure: {error}"),
    }
    println!("\n{}", capped_arena.report());
}
